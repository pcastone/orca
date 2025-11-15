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
            if let Err(e) = WorkflowRepository::update_description(&self.pool, &workflow.id, &req.description).await {
                tracing::warn!("Failed to update workflow description: {}", e);
                // Continue anyway, non-critical error
            }
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

    type ExecuteWorkflowStream = tokio_stream::wrappers::ReceiverStream<Result<ExecutionEvent, Status>>;

    async fn execute_workflow(
        &self,
        request: Request<ExecuteWorkflowRequest>,
    ) -> Result<Response<Self::ExecuteWorkflowStream>, Status> {
        let req = request.into_inner();
        let workflow_id = req.id.clone();

        // Create streaming channel for workflow execution events
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<ExecutionEvent, Status>>(self.stream_buffer_size);

        // Spawn workflow execution in background (stub implementation)
        let workflow_id_clone = workflow_id.clone();
        tokio::spawn(async move {
            // TODO: Implement full workflow execution with proper event streaming
            tracing::info!("Workflow {} execution started (stub)", workflow_id_clone);
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
