use crate::proto::workflows::{
    workflow_service_server::WorkflowService,
    CreateWorkflowRequest, Workflow as ProtoWorkflow, GetWorkflowRequest,
    ListWorkflowsRequest, ListWorkflowsResponse, UpdateWorkflowRequest,
    DeleteWorkflowRequest, DeleteWorkflowResponse, ExecuteWorkflowRequest,
    ExecutionEvent,
};
use crate::db::DatabasePool;
use crate::execution::{ExecutionStreamHandler, ExecutionEventType};
use tonic::{Request, Response, Status};
use std::sync::Arc;
use chrono::Utc;
use uuid::Uuid;

pub struct WorkflowServiceImpl {
    pool: Arc<DatabasePool>,
    stream_buffer_size: usize,
}

impl WorkflowServiceImpl {
    pub fn new(pool: DatabasePool, stream_buffer_size: usize) -> Self {
        Self {
            pool: Arc::new(pool),
            stream_buffer_size,
        }
    }
}

#[tonic::async_trait]
impl WorkflowService for WorkflowServiceImpl {
    async fn create_workflow(
        &self,
        request: Request<CreateWorkflowRequest>,
    ) -> Result<Response<ProtoWorkflow>, Status> {
        let req = request.into_inner();

        // Validate input
        if req.name.is_empty() {
            return Err(Status::invalid_argument("Workflow name is required"));
        }

        if req.definition.is_empty() {
            return Err(Status::invalid_argument("Workflow definition is required"));
        }

        // Validate workflow definition JSON
        serde_json::from_str::<serde_json::Value>(&req.definition)
            .map_err(|e| Status::invalid_argument(
                format!("Invalid workflow definition JSON: {}", e)
            ))?;

        let workflow_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // In a real implementation, this would use the repository to create the workflow
        // For now, we'll create a response directly
        let proto_workflow = ProtoWorkflow {
            id: workflow_id,
            name: req.name,
            description: req.description,
            definition: req.definition,
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        tracing::info!("Created workflow: {}", proto_workflow.id);
        Ok(Response::new(proto_workflow))
    }

    async fn get_workflow(
        &self,
        request: Request<GetWorkflowRequest>,
    ) -> Result<Response<ProtoWorkflow>, Status> {
        let req = request.into_inner();

        // In a real implementation, this would query the database
        Err(Status::not_found(format!("Workflow not found: {}", req.id)))
    }

    async fn list_workflows(
        &self,
        request: Request<ListWorkflowsRequest>,
    ) -> Result<Response<ListWorkflowsResponse>, Status> {
        let _req = request.into_inner();

        // In a real implementation, this would query the database with pagination
        Ok(Response::new(ListWorkflowsResponse {
            workflows: vec![],
            total: 0,
        }))
    }

    async fn update_workflow(
        &self,
        request: Request<UpdateWorkflowRequest>,
    ) -> Result<Response<ProtoWorkflow>, Status> {
        let req = request.into_inner();

        // In a real implementation, this would update the workflow in the database
        Err(Status::not_found(format!("Workflow not found: {}", req.id)))
    }

    async fn delete_workflow(
        &self,
        request: Request<DeleteWorkflowRequest>,
    ) -> Result<Response<DeleteWorkflowResponse>, Status> {
        let req = request.into_inner();

        // In a real implementation, this would delete the workflow from the database
        Ok(Response::new(DeleteWorkflowResponse { success: false }))
    }

    type ExecuteWorkflowStream = tokio_stream::wrappers::ReceiverStream<Result<ExecutionEvent, Status>>;

    async fn execute_workflow(
        &self,
        request: Request<ExecuteWorkflowRequest>,
    ) -> Result<Response<Self::ExecuteWorkflowStream>, Status> {
        let req = request.into_inner();
        let workflow_id = req.id.clone();

        // Create streaming handler
        let (stream_handler, rx) = ExecutionStreamHandler::new(self.stream_buffer_size);
        let stream_handler = Arc::new(stream_handler);

        // Spawn workflow execution in background
        let workflow_id_clone = workflow_id.clone();
        let stream_handler_clone = stream_handler.clone();
        let pool = self.pool.clone();

        tokio::spawn(async move {
            // Send workflow started event
            if let Err(e) = stream_handler_clone.send_started(&workflow_id_clone).await {
                tracing::error!("Failed to send started event: {}", e);
                return;
            }

            // Load workflow definition from database
            let definition = match crate::db::repositories::WorkflowRepository::get_by_id(&pool, &workflow_id_clone).await {
                Ok(Some(workflow)) => workflow.definition,
                Ok(None) => {
                    tracing::error!("Workflow not found: {}", workflow_id_clone);
                    let _ = stream_handler_clone
                        .send_failed(&workflow_id_clone, "Workflow not found")
                        .await;
                    return;
                }
                Err(e) => {
                    tracing::error!("Failed to load workflow: {}", e);
                    let _ = stream_handler_clone
                        .send_failed(&workflow_id_clone, format!("Failed to load workflow: {}", e))
                        .await;
                    return;
                }
            };

            // Parse workflow definition to extract nodes and edges
            let (nodes, edges) = match crate::execution::workflow_engine::WorkflowExecutionEngine::parse_definition(&definition) {
                Ok((nodes, edges)) => (nodes, edges),
                Err(e) => {
                    tracing::error!("Failed to parse workflow definition: {}", e);
                    let _ = stream_handler_clone
                        .send_failed(&workflow_id_clone, format!("Invalid workflow definition: {}", e))
                        .await;
                    return;
                }
            };

            tracing::info!("Starting workflow execution: {} with {} nodes", workflow_id_clone, nodes.len());

            // Execute workflow nodes in topological order
            let mut executed_nodes = std::collections::HashSet::new();
            let mut current_nodes = crate::execution::workflow_engine::WorkflowExecutionEngine::find_next_nodes(None, &edges, &Default::default());
            let mut step_count = 0u32;
            let max_steps = 1000u32; // Safety limit to prevent infinite loops

            while !current_nodes.is_empty() && step_count < max_steps {
                step_count += 1;

                for node_id in current_nodes.iter() {
                    if executed_nodes.contains(node_id) {
                        continue; // Skip if already executed
                    }

                    // Find node definition
                    let node = match nodes.iter().find(|n| &n.id == node_id) {
                        Some(n) => n,
                        None => {
                            tracing::error!("Node not found: {}", node_id);
                            let _ = stream_handler_clone
                                .send_progress(format!("Node {} not found in definition", node_id))
                                .await;
                            continue;
                        }
                    };

                    // Send node entered event
                    if let Err(e) = stream_handler_clone
                        .send_progress(format!("Entering node: {}", node_id))
                        .await
                    {
                        tracing::error!("Failed to send node entered event: {}", e);
                        break;
                    }

                    // Execute node based on type
                    let result = match node.node_type.as_str() {
                        "task" => {
                            // Execute task node
                            let task_id = match node.config.get("task_id").and_then(|v| v.as_str()) {
                                Some(id) => id.to_string(),
                                None => {
                                    tracing::error!("Task node {} missing task_id", node_id);
                                    let _ = stream_handler_clone
                                        .send_progress(format!("Task node {} missing configuration", node_id))
                                        .await;
                                    return String::from("Missing task_id");
                                }
                            };

                            // Simulate task execution
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                            tracing::info!("Executed task node: {}", node_id);
                            Ok(())
                        }
                        "conditional" => {
                            // Execute conditional node
                            let condition = match node.config.get("condition").and_then(|v| v.as_str()) {
                                Some(cond) => cond.to_string(),
                                None => {
                                    tracing::error!("Conditional node {} missing condition", node_id);
                                    String::from("default")
                                }
                            };

                            tracing::info!("Evaluated condition in node {}: {}", node_id, condition);
                            Ok(())
                        }
                        _ => {
                            tracing::warn!("Unknown node type: {}", node.node_type);
                            Err(format!("Unknown node type: {}", node.node_type))
                        }
                    };

                    // Send node completed event
                    match result {
                        Ok(_) => {
                            let _ = stream_handler_clone
                                .send_output(format!("Node {} completed successfully", node_id))
                                .await;
                        }
                        Err(e) => {
                            tracing::error!("Node {} execution failed: {}", node_id, e);
                            let _ = stream_handler_clone
                                .send_progress(format!("Node {} failed: {}", node_id, e))
                                .await;
                        }
                    }

                    executed_nodes.insert(node_id.clone());
                }

                // Find next nodes to execute
                let mut state = crate::execution::workflow_engine::WorkflowExecutionState::default();
                state.step = step_count;
                let next_nodes: Vec<_> = current_nodes
                    .iter()
                    .flat_map(|node_id| {
                        crate::execution::workflow_engine::WorkflowExecutionEngine::find_next_nodes(
                            Some(node_id),
                            &edges,
                            &state,
                        )
                    })
                    .collect();

                current_nodes = next_nodes;

                // Send checkpoint event
                if let Err(e) = stream_handler_clone
                    .send_progress(format!("Workflow checkpoint: step {}", step_count))
                    .await
                {
                    tracing::error!("Failed to send checkpoint event: {}", e);
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }

            // Send completion event
            if stream_handler_clone.is_active() {
                tracing::info!("Workflow {} completed after {} steps with {} executed nodes",
                    workflow_id_clone, step_count, executed_nodes.len());

                if let Err(e) = stream_handler_clone
                    .send_completed(&workflow_id_clone, format!("Completed {} nodes in {} steps",
                        executed_nodes.len(), step_count))
                    .await
                {
                    tracing::error!("Failed to send completion event: {}", e);
                }
            }

            tracing::info!("Workflow {} execution streaming completed", workflow_id_clone);
        });

        tracing::info!("Started streaming execution for workflow: {}", workflow_id);

        Ok(Response::new(
            tokio_stream::wrappers::ReceiverStream::new(rx)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_service_creation() {
        // This is a placeholder test
        assert!(true);
    }
}
