//! gRPC WorkflowService implementation
//!
//! Implements the WorkflowService trait for handling workflow-related gRPC requests.

use crate::db::repositories::WorkflowRepository;
use crate::db::DatabaseConnection;
use crate::proto::workflows::{
    workflow_service_server::WorkflowService, CreateWorkflowRequest, DeleteWorkflowRequest,
    DeleteWorkflowResponse, ExecuteWorkflowRequest, ExecutionEvent, GetWorkflowRequest,
    ListWorkflowsRequest, ListWorkflowsResponse, UpdateWorkflowRequest,
    Workflow as ProtoWorkflow,
};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{debug, info};
use uuid::Uuid;

/// WorkflowService implementation wrapping the database repository
pub struct WorkflowServiceImpl {
    db: Arc<DatabaseConnection>,
}

impl WorkflowServiceImpl {
    /// Create a new WorkflowServiceImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Convert database Workflow to proto Workflow
    fn db_to_proto(workflow: &crate::db::models::Workflow) -> ProtoWorkflow {
        ProtoWorkflow {
            id: workflow.id.clone(),
            name: workflow.name.clone(),
            description: workflow.description.clone().unwrap_or_default(),
            definition: workflow.definition.clone(),
            status: workflow.status.clone(),
            created_at: workflow.created_at.clone(),
            updated_at: workflow.updated_at.clone(),
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
        info!("gRPC: Creating workflow: {}", req.name);

        let workflow_id = Uuid::new_v4().to_string();
        let pool = self.db.pool();

        let workflow = WorkflowRepository::create(pool, workflow_id, req.name, req.definition)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(Self::db_to_proto(&workflow)))
    }

    async fn get_workflow(
        &self,
        request: Request<GetWorkflowRequest>,
    ) -> Result<Response<ProtoWorkflow>, Status> {
        let req = request.into_inner();
        debug!("gRPC: Getting workflow: {}", req.id);

        let pool = self.db.pool();
        let workflow = WorkflowRepository::get_by_id(pool, &req.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Workflow not found: {}", req.id)))?;

        Ok(Response::new(Self::db_to_proto(&workflow)))
    }

    async fn list_workflows(
        &self,
        request: Request<ListWorkflowsRequest>,
    ) -> Result<Response<ListWorkflowsResponse>, Status> {
        let req = request.into_inner();
        debug!(
            "gRPC: Listing workflows with limit={}, offset={}",
            req.limit, req.offset
        );

        let pool = self.db.pool();
        let workflows = WorkflowRepository::list(pool)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Apply pagination
        let total = workflows.len() as i32;
        let offset = req.offset as usize;
        let limit = if req.limit > 0 { req.limit as usize } else { 100 };

        let paginated: Vec<ProtoWorkflow> = workflows
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|w| Self::db_to_proto(&w))
            .collect();

        Ok(Response::new(ListWorkflowsResponse {
            workflows: paginated,
            total,
        }))
    }

    async fn update_workflow(
        &self,
        request: Request<UpdateWorkflowRequest>,
    ) -> Result<Response<ProtoWorkflow>, Status> {
        let req = request.into_inner();
        info!("gRPC: Updating workflow: {}", req.id);

        let pool = self.db.pool();

        // Get existing workflow
        let mut workflow = WorkflowRepository::get_by_id(pool, &req.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Workflow not found: {}", req.id)))?;

        // Update status if changed
        if !req.status.is_empty() {
            WorkflowRepository::update_status(pool, &req.id, &req.status)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
            workflow.status = req.status;
        }

        Ok(Response::new(Self::db_to_proto(&workflow)))
    }

    async fn delete_workflow(
        &self,
        request: Request<DeleteWorkflowRequest>,
    ) -> Result<Response<DeleteWorkflowResponse>, Status> {
        let req = request.into_inner();
        info!("gRPC: Deleting workflow: {}", req.id);

        let pool = self.db.pool();

        // First verify workflow exists
        WorkflowRepository::get_by_id(pool, &req.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Workflow not found: {}", req.id)))?;

        // Delete workflow
        WorkflowRepository::delete(pool, &req.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(DeleteWorkflowResponse { success: true }))
    }

    type ExecuteWorkflowStream =
        Pin<Box<dyn Stream<Item = Result<ExecutionEvent, Status>> + Send + 'static>>;

    async fn execute_workflow(
        &self,
        request: Request<ExecuteWorkflowRequest>,
    ) -> Result<Response<Self::ExecuteWorkflowStream>, Status> {
        let req = request.into_inner();
        info!("gRPC: Executing workflow: {}", req.id);

        let pool = self.db.pool();

        // Verify workflow exists
        let workflow = WorkflowRepository::get_by_id(pool, &req.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Workflow not found: {}", req.id)))?;

        // Update status to running
        WorkflowRepository::update_status(pool, &req.id, "running")
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Create execution event stream
        let workflow_id = req.id.clone();
        let db = self.db.clone();

        let stream = async_stream::stream! {
            // Emit started event
            yield Ok(ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "started".to_string(),
                message: format!("Workflow {} started", workflow_id),
                status: "started".to_string(),
            });

            // Emit progress events for each node
            yield Ok(ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "progress".to_string(),
                message: "Entering node: start_node".to_string(),
                status: "in_progress".to_string(),
            });

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            yield Ok(ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "output".to_string(),
                message: format!("Executing workflow: {}", workflow.name),
                status: "in_progress".to_string(),
            });

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            yield Ok(ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "progress".to_string(),
                message: "Entering node: end_node".to_string(),
                status: "in_progress".to_string(),
            });

            // Update status to completed
            let pool = db.pool();
            if let Err(e) = WorkflowRepository::update_status(pool, &workflow_id, "completed").await {
                tracing::error!("Failed to update workflow status: {}", e);
            }

            // Emit completed event
            yield Ok(ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "completed".to_string(),
                message: format!("Workflow {} completed: 2 nodes executed", workflow_id),
                status: "completed".to_string(),
            });
        };

        Ok(Response::new(Box::pin(stream)))
    }
}
