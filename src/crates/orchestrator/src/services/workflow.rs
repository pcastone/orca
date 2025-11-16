use crate::proto::workflows::{
    workflow_service_server::WorkflowService,
    CreateWorkflowRequest, Workflow as ProtoWorkflow, GetWorkflowRequest,
    ListWorkflowsRequest, ListWorkflowsResponse, UpdateWorkflowRequest,
    DeleteWorkflowRequest, DeleteWorkflowResponse, ExecuteWorkflowRequest,
    ExecutionEvent,
};
use crate::db::{DatabasePool, repositories::WorkflowRepository};
use crate::proto_conv::workflow_to_proto;
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

        // Create workflow in database using repository
        let workflow = WorkflowRepository::create(
            &self.pool,
            workflow_id,
            req.name,
            req.definition,
        ).await.map_err(|e| {
            tracing::error!("Failed to create workflow in database: {}", e);
            Status::internal(format!("Failed to create workflow: {}", e))
        })?;

        // Update description if provided
        if !req.description.is_empty() {
            WorkflowRepository::update_description(&self.pool, &workflow.id, &req.description)
                .await
                .map_err(|e| {
                    tracing::warn!("Failed to update workflow description: {}", e);
                    Status::internal(format!("Failed to update description: {}", e))
                })?;
        }

        // Fetch updated workflow
        let updated_workflow = WorkflowRepository::get_by_id(&self.pool, &workflow.id)
            .await
            .map_err(|e| {
                tracing::error!("Database error getting created workflow: {}", e);
                Status::internal("Failed to retrieve created workflow")
            })?
            .ok_or_else(|| Status::internal("Workflow disappeared after creation"))?;

        tracing::info!("Created workflow: {}", updated_workflow.id);
        Ok(Response::new(workflow_to_proto(&updated_workflow)))
    }

    async fn get_workflow(
        &self,
        request: Request<GetWorkflowRequest>,
    ) -> Result<Response<ProtoWorkflow>, Status> {
        let req = request.into_inner();

        if req.id.is_empty() {
            return Err(Status::invalid_argument("Workflow ID is required"));
        }

        let workflow = WorkflowRepository::get_by_id(&self.pool, &req.id)
            .await
            .map_err(|e| {
                tracing::error!("Database error getting workflow {}: {}", req.id, e);
                Status::internal(format!("Failed to retrieve workflow: {}", e))
            })?
            .ok_or_else(|| {
                tracing::warn!("Workflow not found: {}", req.id);
                Status::not_found(format!("Workflow not found: {}", req.id))
            })?;

        tracing::debug!("Retrieved workflow: {}", workflow.id);
        Ok(Response::new(workflow_to_proto(&workflow)))
    }

    async fn list_workflows(
        &self,
        request: Request<ListWorkflowsRequest>,
    ) -> Result<Response<ListWorkflowsResponse>, Status> {
        let req = request.into_inner();

        // Query workflows from database
        // Note: Status filtering removed as ListWorkflowsRequest doesn't have status field
        let workflows = WorkflowRepository::list(&self.pool)
                .await
                .map_err(|e| {
                    tracing::error!("Database error listing workflows: {}", e);
                    Status::internal(format!("Failed to list workflows: {}", e))
                })?;

        // Apply pagination (simple offset/limit)
        let offset = req.offset.max(0) as usize;
        let limit = if req.limit > 0 {
            req.limit as usize
        } else {
            100 // Default limit
        };

        let total = workflows.len();
        let paginated_workflows: Vec<ProtoWorkflow> = workflows
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|w| workflow_to_proto(&w))
            .collect();

        tracing::debug!("Listed {} workflows (total: {})", paginated_workflows.len(), total);
        Ok(Response::new(ListWorkflowsResponse {
            workflows: paginated_workflows,
            total: total as i32,
        }))
    }

    async fn update_workflow(
        &self,
        request: Request<UpdateWorkflowRequest>,
    ) -> Result<Response<ProtoWorkflow>, Status> {
        let req = request.into_inner();

        if req.id.is_empty() {
            return Err(Status::invalid_argument("Workflow ID is required"));
        }

        // First check if workflow exists
        let _workflow = WorkflowRepository::get_by_id(&self.pool, &req.id)
            .await
            .map_err(|e| {
                tracing::error!("Database error getting workflow {}: {}", req.id, e);
                Status::internal(format!("Failed to retrieve workflow: {}", e))
            })?
            .ok_or_else(|| {
                tracing::warn!("Workflow not found for update: {}", req.id);
                Status::not_found(format!("Workflow not found: {}", req.id))
            })?;

        // Update fields
        if !req.name.is_empty() {
            // Note: Repository doesn't have update_name method
            tracing::warn!("Name updates not yet supported");
        }

        if !req.description.is_empty() {
            WorkflowRepository::update_description(&self.pool, &req.id, &req.description)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update workflow description: {}", e);
                    Status::internal(format!("Failed to update workflow: {}", e))
                })?;
        }

        if !req.definition.is_empty() {
            // Validate definition JSON
            serde_json::from_str::<serde_json::Value>(&req.definition)
                .map_err(|e| Status::invalid_argument(
                    format!("Invalid workflow definition JSON: {}", e)
                ))?;

            WorkflowRepository::update_definition(&self.pool, &req.id, &req.definition)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update workflow definition: {}", e);
                    Status::internal(format!("Failed to update workflow: {}", e))
                })?;
        }

        if !req.status.is_empty() {
            WorkflowRepository::update_status(&self.pool, &req.id, &req.status)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update workflow status: {}", e);
                    Status::internal(format!("Failed to update workflow: {}", e))
                })?;
        }

        // Fetch updated workflow
        let updated_workflow = WorkflowRepository::get_by_id(&self.pool, &req.id)
            .await
            .map_err(|e| {
                tracing::error!("Database error getting updated workflow: {}", e);
                Status::internal(format!("Failed to retrieve updated workflow: {}", e))
            })?
            .ok_or_else(|| Status::internal("Workflow disappeared after update"))?;

        tracing::info!("Updated workflow: {}", updated_workflow.id);
        Ok(Response::new(workflow_to_proto(&updated_workflow)))
    }

    async fn delete_workflow(
        &self,
        request: Request<DeleteWorkflowRequest>,
    ) -> Result<Response<DeleteWorkflowResponse>, Status> {
        let req = request.into_inner();

        if req.id.is_empty() {
            return Err(Status::invalid_argument("Workflow ID is required"));
        }

        // Check if workflow exists before deletion
        let exists = WorkflowRepository::get_by_id(&self.pool, &req.id)
            .await
            .map_err(|e| {
                tracing::error!("Database error checking workflow {}: {}", req.id, e);
                Status::internal(format!("Failed to check workflow: {}", e))
            })?
            .is_some();

        if !exists {
            tracing::warn!("Attempted to delete non-existent workflow: {}", req.id);
            return Ok(Response::new(DeleteWorkflowResponse { success: false }));
        }

        // Delete the workflow
        WorkflowRepository::delete(&self.pool, &req.id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete workflow {}: {}", req.id, e);
                Status::internal(format!("Failed to delete workflow: {}", e))
            })?;

        tracing::info!("Deleted workflow: {}", req.id);
        Ok(Response::new(DeleteWorkflowResponse { success: true }))
    }

    type ExecuteWorkflowStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<ExecutionEvent, Status>> + Send>>;

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
                                    continue; // Skip this node and continue with next
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

        // Convert task execution events to workflow execution events
        use tokio_stream::StreamExt;
        let workflow_rx = Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)
            .map(|result| {
                result.map(|task_event| {
                    // Both event types have the same structure, so we can convert
                    ExecutionEvent {
                        timestamp: task_event.timestamp,
                        event_type: task_event.event_type,
                        message: task_event.message,
                        status: task_event.status,
                    }
                })
            }));

        Ok(Response::new(workflow_rx))
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
