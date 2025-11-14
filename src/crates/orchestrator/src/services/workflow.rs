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
            // Send started event
            if let Err(e) = stream_handler_clone.send_started(&workflow_id_clone).await {
                tracing::error!("Failed to send started event: {}", e);
                return;
            }

            // Simulate workflow execution with streaming events
            // In a real implementation, this would call the WorkflowExecutionEngine

            // Send progress events for workflow steps
            let steps = vec!["Initialize", "Execute Tasks", "Aggregate Results", "Cleanup"];
            for (idx, step) in steps.iter().enumerate() {
                if !stream_handler_clone.is_active() {
                    break;
                }

                if let Err(e) = stream_handler_clone
                    .send_progress(format!("Step {}: {}", idx + 1, step))
                    .await
                {
                    tracing::error!("Failed to send progress event: {}", e);
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            }

            // Send output event
            if stream_handler_clone.is_active() {
                if let Err(e) = stream_handler_clone
                    .send_output("Workflow execution completed successfully")
                    .await
                {
                    tracing::error!("Failed to send output event: {}", e);
                }
            }

            // Send completion event
            if stream_handler_clone.is_active() {
                if let Err(e) = stream_handler_clone
                    .send_completed(&workflow_id_clone, "Success")
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
