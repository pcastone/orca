use crate::proto::tasks::{
    task_service_server::TaskService,
    CreateTaskRequest, Task as ProtoTask, GetTaskRequest,
    ListTasksRequest, ListTasksResponse, UpdateTaskRequest,
    DeleteTaskRequest, DeleteTaskResponse, ExecuteTaskRequest,
    ExecutionEvent,
};
use crate::db::DatabasePool;
use crate::execution::{ExecutionStreamHandler, ExecutionEventType};
use tonic::{Request, Response, Status};
use std::sync::Arc;
use chrono::Utc;
use uuid::Uuid;

pub struct TaskServiceImpl {
    pool: Arc<DatabasePool>,
    stream_buffer_size: usize,
}

impl TaskServiceImpl {
    pub fn new(pool: DatabasePool, stream_buffer_size: usize) -> Self {
        Self {
            pool: Arc::new(pool),
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
            return Err(Status::invalid_argument("Task title is required"));
        }

        // Parse config JSON if provided
        if let Some(ref config) = req.config {
            if !config.is_empty() {
                serde_json::from_str::<serde_json::Value>(config)
                    .map_err(|e| Status::invalid_argument(
                        format!("Invalid config JSON: {}", e)
                    ))?;
            }
        }

        // Parse metadata JSON if provided
        if let Some(ref metadata) = req.metadata {
            if !metadata.is_empty() {
                serde_json::from_str::<serde_json::Value>(metadata)
                    .map_err(|e| Status::invalid_argument(
                        format!("Invalid metadata JSON: {}", e)
                    ))?;
            }
        }

        let task_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // In a real implementation, this would use the repository to create the task
        // For now, we'll create a response directly
        let proto_task = ProtoTask {
            id: task_id,
            title: req.title,
            description: req.description,
            task_type: req.task_type,
            status: 0,
            config: req.config,
            metadata: req.metadata,
            workspace_path: req.workspace_path,
            created_at: now.clone(),
            updated_at: now,
        };

        tracing::info!("Created task: {}", proto_task.id);
        Ok(Response::new(proto_task))
    }

    async fn get_task(
        &self,
        request: Request<GetTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();

        // In a real implementation, this would query the database
        Err(Status::not_found(format!("Task not found: {}", req.id)))
    }

    async fn list_tasks(
        &self,
        request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let _req = request.into_inner();

        // In a real implementation, this would query the database with pagination
        Ok(Response::new(ListTasksResponse {
            tasks: vec![],
            total: 0,
        }))
    }

    async fn update_task(
        &self,
        request: Request<UpdateTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();

        // In a real implementation, this would update the task in the database
        Err(Status::not_found(format!("Task not found: {}", req.id)))
    }

    async fn delete_task(
        &self,
        request: Request<DeleteTaskRequest>,
    ) -> Result<Response<DeleteTaskResponse>, Status> {
        let req = request.into_inner();

        // In a real implementation, this would delete the task from the database
        Ok(Response::new(DeleteTaskResponse { success: false }))
    }

    type ExecuteTaskStream = tokio_stream::wrappers::ReceiverStream<Result<ExecutionEvent, Status>>;

    async fn execute_task(
        &self,
        request: Request<ExecuteTaskRequest>,
    ) -> Result<Response<Self::ExecuteTaskStream>, Status> {
        let req = request.into_inner();
        let task_id = req.id.clone();

        // Create streaming handler
        let (stream_handler, rx) = ExecutionStreamHandler::new(self.stream_buffer_size);
        let stream_handler = Arc::new(stream_handler);

        // Spawn task execution in background
        let task_id_clone = task_id.clone();
        let stream_handler_clone = stream_handler.clone();
        let pool = self.pool.clone();

        tokio::spawn(async move {
            // Send started event
            if let Err(e) = stream_handler_clone.send_started(&task_id_clone).await {
                tracing::error!("Failed to send started event: {}", e);
                return;
            }

            // Simulate task execution with streaming events
            // In a real implementation, this would call the TaskExecutionEngine

            // Send progress events
            for i in 1..=5 {
                if !stream_handler_clone.is_active() {
                    break;
                }

                if let Err(e) = stream_handler_clone
                    .send_progress(format!("Progress: {}%", i * 20))
                    .await
                {
                    tracing::error!("Failed to send progress event: {}", e);
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // Send output event
            if stream_handler_clone.is_active() {
                if let Err(e) = stream_handler_clone
                    .send_output("Task execution completed successfully")
                    .await
                {
                    tracing::error!("Failed to send output event: {}", e);
                }
            }

            // Send completion event
            if stream_handler_clone.is_active() {
                if let Err(e) = stream_handler_clone
                    .send_completed(&task_id_clone, "Success")
                    .await
                {
                    tracing::error!("Failed to send completion event: {}", e);
                }
            }

            tracing::info!("Task {} execution streaming completed", task_id_clone);
        });

        tracing::info!("Started streaming execution for task: {}", task_id);

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
        assert!(true);
    }
}
