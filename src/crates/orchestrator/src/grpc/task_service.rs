//! gRPC TaskService implementation
//!
//! Implements the TaskService trait for handling task-related gRPC requests.

use crate::db::repositories::TaskRepository;
use crate::db::DatabaseConnection;
use crate::executor::LlmTaskExecutor;
use crate::proto::tasks::{
    task_service_server::TaskService, CreateTaskRequest, DeleteTaskRequest, DeleteTaskResponse,
    ExecuteTaskRequest, ExecutionEvent, GetTaskRequest, ListTasksRequest, ListTasksResponse,
    Task as ProtoTask, UpdateTaskRequest,
};
use crate::{Task, TaskExecutor, TaskStatus};
use futures::Stream;
use langgraph_core::llm::ChatModel;
use std::pin::Pin;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{debug, info};
use uuid::Uuid;

/// TaskService implementation wrapping the database repository
pub struct TaskServiceImpl {
    db: Arc<DatabaseConnection>,
    /// Optional LLM client for real task execution
    llm_client: Option<Arc<dyn ChatModel>>,
}

impl TaskServiceImpl {
    /// Create a new TaskServiceImpl
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db, llm_client: None }
    }

    /// Create a new TaskServiceImpl with an LLM client for real execution
    pub fn with_llm_client(db: Arc<DatabaseConnection>, llm_client: Arc<dyn ChatModel>) -> Self {
        Self {
            db,
            llm_client: Some(llm_client),
        }
    }

    /// Set the LLM client after construction
    pub fn set_llm_client(&mut self, client: Arc<dyn ChatModel>) {
        self.llm_client = Some(client);
    }

    /// Convert database Task to proto Task
    fn db_to_proto(task: &crate::db::models::Task) -> ProtoTask {
        ProtoTask {
            id: task.id.clone(),
            title: task.title.clone(),
            description: task.description.clone().unwrap_or_default(),
            task_type: task.task_type.clone(),
            status: status_to_i32(&task.status),
            config: task.config.clone(),
            metadata: task.metadata.clone(),
            workspace_path: task.workspace_path.clone().unwrap_or_default(),
            created_at: task.created_at.clone(),
            updated_at: task.updated_at.clone(),
        }
    }
}

/// Convert status string to i32
fn status_to_i32(status: &str) -> i32 {
    match status.to_lowercase().as_str() {
        "pending" => 0,
        "running" => 1,
        "completed" => 2,
        "failed" => 3,
        "cancelled" => 4,
        _ => 0,
    }
}

/// Convert i32 to status string
fn i32_to_status(status: i32) -> &'static str {
    match status {
        0 => "pending",
        1 => "running",
        2 => "completed",
        3 => "failed",
        4 => "cancelled",
        _ => "pending",
    }
}

#[tonic::async_trait]
impl TaskService for TaskServiceImpl {
    async fn create_task(
        &self,
        request: Request<CreateTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();
        info!("gRPC: Creating task: {}", req.title);

        let task_id = Uuid::new_v4().to_string();
        let pool = self.db.pool();

        let task = TaskRepository::create(
            pool,
            task_id,
            req.title,
            req.task_type,
            req.workspace_path,
        )
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(Self::db_to_proto(&task)))
    }

    async fn get_task(
        &self,
        request: Request<GetTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();
        debug!("gRPC: Getting task: {}", req.id);

        let pool = self.db.pool();
        let task = TaskRepository::get_by_id(pool, &req.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Task not found: {}", req.id)))?;

        Ok(Response::new(Self::db_to_proto(&task)))
    }

    async fn list_tasks(
        &self,
        request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let req = request.into_inner();
        debug!("gRPC: Listing tasks with limit={}, offset={}", req.limit, req.offset);

        let pool = self.db.pool();

        // Get tasks based on status filter
        let tasks = if req.status >= 0 {
            let status = i32_to_status(req.status);
            TaskRepository::list_by_status(pool, status).await
        } else {
            TaskRepository::list(pool).await
        }
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Apply pagination
        let total = tasks.len() as i32;
        let offset = req.offset as usize;
        let limit = if req.limit > 0 { req.limit as usize } else { 100 };

        let paginated: Vec<ProtoTask> = tasks
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|t| Self::db_to_proto(&t))
            .collect();

        Ok(Response::new(ListTasksResponse {
            tasks: paginated,
            total,
        }))
    }

    async fn update_task(
        &self,
        request: Request<UpdateTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();
        info!("gRPC: Updating task: {}", req.id);

        let pool = self.db.pool();

        // Get existing task
        let mut task = TaskRepository::get_by_id(pool, &req.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Task not found: {}", req.id)))?;

        // Update status if changed
        if req.status >= 0 {
            let new_status = i32_to_status(req.status);
            TaskRepository::update_status(pool, &req.id, new_status)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
            task.status = new_status.to_string();
        }

        // Note: title and description updates would need additional repository methods

        Ok(Response::new(Self::db_to_proto(&task)))
    }

    async fn delete_task(
        &self,
        request: Request<DeleteTaskRequest>,
    ) -> Result<Response<DeleteTaskResponse>, Status> {
        let req = request.into_inner();
        info!("gRPC: Deleting task: {}", req.id);

        let pool = self.db.pool();

        // First verify task exists
        TaskRepository::get_by_id(pool, &req.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Task not found: {}", req.id)))?;

        // Delete task
        TaskRepository::delete(pool, &req.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(DeleteTaskResponse { success: true }))
    }

    type ExecuteTaskStream =
        Pin<Box<dyn Stream<Item = Result<ExecutionEvent, Status>> + Send + 'static>>;

    async fn execute_task(
        &self,
        request: Request<ExecuteTaskRequest>,
    ) -> Result<Response<Self::ExecuteTaskStream>, Status> {
        let req = request.into_inner();
        info!("gRPC: Executing task: {}", req.id);

        let pool = self.db.pool();

        // Verify task exists
        let db_task = TaskRepository::get_by_id(pool, &req.id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("Task not found: {}", req.id)))?;

        // Update status to running
        TaskRepository::update_status(pool, &req.id, "running")
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Create execution event stream
        let task_id = req.id.clone();
        let task_title = db_task.title.clone();
        let task_description = db_task.description.clone();
        let db = self.db.clone();
        let llm_client = self.llm_client.clone();

        let stream = async_stream::stream! {
            // Emit started event
            yield Ok(ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "started".to_string(),
                message: format!("Task {} started", task_id),
                status: "started".to_string(),
            });

            // Check if we have an LLM client for real execution
            if let Some(llm) = llm_client {
                yield Ok(ExecutionEvent {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    event_type: "progress".to_string(),
                    message: "Initializing LLM executor...".to_string(),
                    status: "in_progress".to_string(),
                });

                // Create the LLM task executor
                let executor = LlmTaskExecutor::new(llm);

                // Create a Task object for the executor
                let mut exec_task = Task::new(&task_title);
                if let Some(desc) = &task_description {
                    exec_task = exec_task.with_description(desc);
                }
                exec_task.status = TaskStatus::Running;

                yield Ok(ExecutionEvent {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    event_type: "progress".to_string(),
                    message: format!("Executing task via LLM: {}", task_title),
                    status: "in_progress".to_string(),
                });

                // Execute via LLM
                match executor.execute(&exec_task).await {
                    Ok(()) => {
                        // Update status to completed
                        let pool = db.pool();
                        if let Err(e) = TaskRepository::update_status(pool, &task_id, "completed").await {
                            tracing::error!("Failed to update task status: {}", e);
                        }

                        yield Ok(ExecutionEvent {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            event_type: "completed".to_string(),
                            message: format!("Task {} completed successfully via LLM", task_id),
                            status: "completed".to_string(),
                        });
                    }
                    Err(e) => {
                        tracing::error!("LLM task execution failed: {}", e);

                        // Update status to failed
                        let pool = db.pool();
                        if let Err(err) = TaskRepository::update_status(pool, &task_id, "failed").await {
                            tracing::error!("Failed to update task status: {}", err);
                        }

                        yield Ok(ExecutionEvent {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            event_type: "failed".to_string(),
                            message: format!("Task {} failed: {}", task_id, e),
                            status: "failed".to_string(),
                        });
                    }
                }
            } else {
                // No LLM client - fall back to simulated execution (for backward compatibility)
                tracing::warn!("No LLM client configured, using simulated execution for task {}", task_id);

                yield Ok(ExecutionEvent {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    event_type: "progress".to_string(),
                    message: "Initializing execution environment (simulated)...".to_string(),
                    status: "in_progress".to_string(),
                });

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                yield Ok(ExecutionEvent {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    event_type: "output".to_string(),
                    message: format!("Executing task (simulated): {}", task_title),
                    status: "in_progress".to_string(),
                });

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Update status to completed
                let pool = db.pool();
                if let Err(e) = TaskRepository::update_status(pool, &task_id, "completed").await {
                    tracing::error!("Failed to update task status: {}", e);
                }

                yield Ok(ExecutionEvent {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    event_type: "completed".to_string(),
                    message: format!("Task {} completed (simulated)", task_id),
                    status: "completed".to_string(),
                });
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_conversion() {
        assert_eq!(status_to_i32("pending"), 0);
        assert_eq!(status_to_i32("running"), 1);
        assert_eq!(status_to_i32("completed"), 2);
        assert_eq!(status_to_i32("failed"), 3);
        assert_eq!(status_to_i32("cancelled"), 4);

        assert_eq!(i32_to_status(0), "pending");
        assert_eq!(i32_to_status(1), "running");
        assert_eq!(i32_to_status(2), "completed");
        assert_eq!(i32_to_status(3), "failed");
        assert_eq!(i32_to_status(4), "cancelled");
    }
}
