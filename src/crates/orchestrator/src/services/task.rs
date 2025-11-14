use crate::proto::tasks::{
    task_service_server::TaskService,
    CreateTaskRequest, Task as ProtoTask, GetTaskRequest,
    ListTasksRequest, ListTasksResponse, UpdateTaskRequest,
    DeleteTaskRequest, DeleteTaskResponse, ExecuteTaskRequest,
    ExecutionEvent,
};
use crate::database::TaskRepository;
use crate::proto_conv::{task_to_proto, proto_to_task};
use crate::error::OrchestratorError;
use domain::{Task, TaskStatus, TaskType};
use tonic::{Request, Response, Status};
use std::sync::Arc;
use sqlx::SqlitePool;
use chrono::Utc;

pub struct TaskServiceImpl {
    repository: TaskRepository,
    stream_buffer_size: usize,
}

impl TaskServiceImpl {
    pub fn new(pool: SqlitePool, stream_buffer_size: usize) -> Self {
        Self {
            repository: TaskRepository::new(pool),
            stream_buffer_size,
        }
    }
}

#[tonic::async_trait]
impl TaskService for TaskServiceImpl {
    async fn create_task(
        &self,
        request: Request<CreateTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();

        // Validate input
        if req.title.is_empty() {
            return Err(OrchestratorError::missing_field("title").into());
        }

        // Create domain task
        let task_type = match req.task_type.to_lowercase().as_str() {
            "code" => TaskType::Code,
            "research" => TaskType::Research,
            "review" => TaskType::Review,
            other => TaskType::Custom(other.to_string()),
        };

        let mut task = Task::new(req.title, req.description, task_type);

        if let Some(config) = req.config {
            if !config.is_empty() {
                task.config = Some(
                    serde_json::from_str(&config)
                        .map_err(|e| OrchestratorError::Validation(
                            format!("Invalid config JSON: {}", e)
                        ))?
                );
            }
        }

        if let Some(metadata) = req.metadata {
            if !metadata.is_empty() {
                task.metadata = Some(
                    serde_json::from_str(&metadata)
                        .map_err(|e| OrchestratorError::Validation(
                            format!("Invalid metadata JSON: {}", e)
                        ))?
                );
            }
        }

        if !req.workspace_path.is_empty() {
            task.workspace_path = Some(req.workspace_path);
        }

        // Save to database
        self.repository.create(&task).await
            .map_err(|e| {
                tracing::error!("Failed to create task: {}", e);
                Status::internal("Failed to create task")
            })?;

        // Convert to proto and return
        let proto_task = task_to_proto(&task);
        Ok(Response::new(proto_task))
    }

    async fn get_task(
        &self,
        request: Request<GetTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();

        let task = self.repository.get_by_id(&req.id).await
            .map_err(|e| {
                tracing::error!("Database error: {}", e);
                Status::internal("Database error")
            })?
            .ok_or_else(|| Status::not_found(
                format!("Task not found: {}", req.id)
            ))?;

        let proto_task = task_to_proto(&task);
        Ok(Response::new(proto_task))
    }

    async fn list_tasks(
        &self,
        request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let req = request.into_inner();

        let status = if req.status > 0 {
            TaskStatus::from_i32(req.status)
        } else {
            None
        };

        let tasks = self.repository.list(
            if req.limit > 0 { Some(req.limit) } else { None },
            if req.offset >= 0 { Some(req.offset) } else { None },
            status,
        ).await
            .map_err(|e| {
                tracing::error!("Database error: {}", e);
                Status::internal("Database error")
            })?;

        let total = tasks.len() as i32;
        let proto_tasks = tasks.iter()
            .map(task_to_proto)
            .collect();

        Ok(Response::new(ListTasksResponse {
            tasks: proto_tasks,
            total,
        }))
    }

    async fn update_task(
        &self,
        request: Request<UpdateTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();

        let mut task = self.repository.get_by_id(&req.id).await
            .map_err(|e| {
                tracing::error!("Database error: {}", e);
                Status::internal("Database error")
            })?
            .ok_or_else(|| Status::not_found(
                format!("Task not found: {}", req.id)
            ))?;

        // Update fields
        if !req.title.is_empty() {
            task.title = req.title;
        }
        if !req.description.is_empty() {
            task.description = req.description;
        }
        if req.status > 0 {
            task.status = TaskStatus::from_i32(req.status)
                .ok_or_else(|| Status::invalid_argument("Invalid task status"))?;
        }

        task.updated_at = Utc::now();

        self.repository.update(&task).await
            .map_err(|e| {
                tracing::error!("Failed to update task: {}", e);
                Status::internal("Failed to update task")
            })?;

        let proto_task = task_to_proto(&task);
        Ok(Response::new(proto_task))
    }

    async fn delete_task(
        &self,
        request: Request<DeleteTaskRequest>,
    ) -> Result<Response<DeleteTaskResponse>, Status> {
        let req = request.into_inner();

        let success = self.repository.delete(&req.id).await
            .map_err(|e| {
                tracing::error!("Failed to delete task: {}", e);
                Status::internal("Failed to delete task")
            })?;

        Ok(Response::new(DeleteTaskResponse { success }))
    }

    type ExecuteTaskStream = tokio_stream::wrappers::ReceiverStream<Result<ExecutionEvent, Status>>;

    async fn execute_task(
        &self,
        request: Request<ExecuteTaskRequest>,
    ) -> Result<Response<Self::ExecuteTaskStream>, Status> {
        let req = request.into_inner();

        // Verify task exists
        let _task = self.repository.get_by_id(&req.id).await
            .map_err(|e| {
                tracing::error!("Database error: {}", e);
                Status::internal("Database error")
            })?
            .ok_or_else(|| Status::not_found(
                format!("Task not found: {}", req.id)
            ))?;

        // This will be implemented in Task 012 (Task Execution Streaming)
        let (tx, rx) = tokio::sync::mpsc::channel(self.stream_buffer_size);

        // Send error for now - implementation in Task 012
        let _ = tx.send(Err(Status::unimplemented(
            "Task execution not yet implemented - see Task 012"
        ))).await;

        Ok(Response::new(
            tokio_stream::wrappers::ReceiverStream::new(rx)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_service_creation() {
        // This is a placeholder test
        // Full integration tests in tests/task_service.rs
        assert!(true);
    }
}
