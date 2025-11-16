use crate::proto::tasks::{
    task_service_server::TaskService,
    CreateTaskRequest, Task as ProtoTask, GetTaskRequest,
    ListTasksRequest, ListTasksResponse, UpdateTaskRequest,
    DeleteTaskRequest, DeleteTaskResponse, ExecuteTaskRequest,
    ExecutionEvent,
};
use crate::db::{DatabasePool, repositories::TaskRepository};
use crate::proto_conv::{task_to_proto, status_int_to_string};
use crate::execution::ExecutionStreamHandler;
use tonic::{Request, Response, Status};
use std::sync::Arc;
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

        if req.task_type.is_empty() {
            return Err(Status::invalid_argument("Task type is required"));
        }

        // Parse and validate config JSON if provided
        if let Some(ref config) = req.config {
            if !config.is_empty() {
                serde_json::from_str::<serde_json::Value>(config)
                    .map_err(|e| Status::invalid_argument(
                        format!("Invalid config JSON: {}", e)
                    ))?;
            }
        }

        // Parse and validate metadata JSON if provided
        if let Some(ref metadata) = req.metadata {
            if !metadata.is_empty() {
                serde_json::from_str::<serde_json::Value>(metadata)
                    .map_err(|e| Status::invalid_argument(
                        format!("Invalid metadata JSON: {}", e)
                    ))?;
            }
        }

        let task_id = Uuid::new_v4().to_string();
        let workspace_path = if req.workspace_path.is_empty() {
            format!("/tmp/workspace/{}", task_id)
        } else {
            req.workspace_path
        };

        // Create task in database using repository
        let task = TaskRepository::create(
            &self.pool,
            task_id,
            req.title,
            req.task_type,
            workspace_path,
        ).await.map_err(|e| {
            tracing::error!("Failed to create task in database: {}", e);
            Status::internal(format!("Failed to create task: {}", e))
        })?;

        tracing::info!("Created task: {}", task.id);
        Ok(Response::new(task_to_proto(&task)))
    }

    async fn get_task(
        &self,
        request: Request<GetTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();

        if req.id.is_empty() {
            return Err(Status::invalid_argument("Task ID is required"));
        }

        let task = TaskRepository::get_by_id(&self.pool, &req.id)
            .await
            .map_err(|e| {
                tracing::error!("Database error getting task {}: {}", req.id, e);
                Status::internal(format!("Failed to retrieve task: {}", e))
            })?
            .ok_or_else(|| {
                tracing::warn!("Task not found: {}", req.id);
                Status::not_found(format!("Task not found: {}", req.id))
            })?;

        tracing::debug!("Retrieved task: {}", task.id);
        Ok(Response::new(task_to_proto(&task)))
    }

    async fn list_tasks(
        &self,
        request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let req = request.into_inner();

        // Query tasks from database
        let tasks = if req.status > 0 {
            // Filter by status
            let status_str = status_int_to_string(req.status);
            TaskRepository::list_by_status(&self.pool, &status_str)
                .await
                .map_err(|e| {
                    tracing::error!("Database error listing tasks by status: {}", e);
                    Status::internal(format!("Failed to list tasks: {}", e))
                })?
        } else {
            // Get all tasks
            TaskRepository::list(&self.pool)
                .await
                .map_err(|e| {
                    tracing::error!("Database error listing tasks: {}", e);
                    Status::internal(format!("Failed to list tasks: {}", e))
                })?
        };

        // Apply pagination (simple offset/limit)
        let offset = req.offset.max(0) as usize;
        let limit = if req.limit > 0 {
            req.limit as usize
        } else {
            100 // Default limit
        };

        let total = tasks.len();
        let paginated_tasks: Vec<ProtoTask> = tasks
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|t| task_to_proto(&t))
            .collect();

        tracing::debug!("Listed {} tasks (total: {})", paginated_tasks.len(), total);
        Ok(Response::new(ListTasksResponse {
            tasks: paginated_tasks,
            total: total as i32,
        }))
    }

    async fn update_task(
        &self,
        request: Request<UpdateTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();

        if req.id.is_empty() {
            return Err(Status::invalid_argument("Task ID is required"));
        }

        // First check if task exists
        let _task = TaskRepository::get_by_id(&self.pool, &req.id)
            .await
            .map_err(|e| {
                tracing::error!("Database error getting task {}: {}", req.id, e);
                Status::internal(format!("Failed to retrieve task: {}", e))
            })?
            .ok_or_else(|| {
                tracing::warn!("Task not found for update: {}", req.id);
                Status::not_found(format!("Task not found: {}", req.id))
            })?;

        // Update status if provided
        if req.status > 0 {
            let status_str = status_int_to_string(req.status);
            TaskRepository::update_status(&self.pool, &req.id, &status_str)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update task status: {}", e);
                    Status::internal(format!("Failed to update task: {}", e))
                })?;
        }

        // Fetch updated task
        let updated_task = TaskRepository::get_by_id(&self.pool, &req.id)
            .await
            .map_err(|e| {
                tracing::error!("Database error getting updated task: {}", e);
                Status::internal(format!("Failed to retrieve updated task: {}", e))
            })?
            .ok_or_else(|| Status::internal("Task disappeared after update"))?;

        tracing::info!("Updated task: {}", updated_task.id);
        Ok(Response::new(task_to_proto(&updated_task)))
    }

    async fn delete_task(
        &self,
        request: Request<DeleteTaskRequest>,
    ) -> Result<Response<DeleteTaskResponse>, Status> {
        let req = request.into_inner();

        if req.id.is_empty() {
            return Err(Status::invalid_argument("Task ID is required"));
        }

        // Check if task exists before deletion
        let exists = TaskRepository::get_by_id(&self.pool, &req.id)
            .await
            .map_err(|e| {
                tracing::error!("Database error checking task {}: {}", req.id, e);
                Status::internal(format!("Failed to check task: {}", e))
            })?
            .is_some();

        if !exists {
            tracing::warn!("Attempted to delete non-existent task: {}", req.id);
            return Ok(Response::new(DeleteTaskResponse { success: false }));
        }

        // Delete the task
        TaskRepository::delete(&self.pool, &req.id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete task {}: {}", req.id, e);
                Status::internal(format!("Failed to delete task: {}", e))
            })?;

        tracing::info!("Deleted task: {}", req.id);
        Ok(Response::new(DeleteTaskResponse { success: true }))
    }

    type ExecuteTaskStream = tokio_stream::wrappers::ReceiverStream<Result<ExecutionEvent, Status>>;

    async fn execute_task(
        &self,
        request: Request<ExecuteTaskRequest>,
    ) -> Result<Response<Self::ExecuteTaskStream>, Status> {
        let req = request.into_inner();
        let task_id = req.id.clone();

        if task_id.is_empty() {
            return Err(Status::invalid_argument("Task ID is required"));
        }

        // Verify task exists
        let _task = TaskRepository::get_by_id(&self.pool, &task_id)
            .await
            .map_err(|e| {
                tracing::error!("Database error getting task {}: {}", task_id, e);
                Status::internal(format!("Failed to retrieve task: {}", e))
            })?
            .ok_or_else(|| {
                tracing::warn!("Task not found for execution: {}", task_id);
                Status::not_found(format!("Task not found: {}", task_id))
            })?;

        // Create streaming handler
        let (stream_handler, rx) = ExecutionStreamHandler::new(self.stream_buffer_size);
        let stream_handler = Arc::new(stream_handler);

        // Spawn task execution in background
        let task_id_clone = task_id.clone();
        let stream_handler_clone = stream_handler.clone();
        let pool = self.pool.clone();

        tokio::spawn(async move {
            // Mark task as started in database
            if let Err(e) = TaskRepository::mark_started(&pool, &task_id_clone).await {
                tracing::error!("Failed to mark task as started: {}", e);
                let _ = stream_handler_clone.send_failed(&task_id_clone, &format!("Failed to start task: {}", e)).await;
                return;
            }

            // Send started event
            if let Err(e) = stream_handler_clone.send_started(&task_id_clone).await {
                tracing::error!("Failed to send started event: {}", e);
                return;
            }

            // Simulate task execution with streaming events
            // TODO: In a real implementation, this would call the TaskExecutionEngine
            // and integrate with LLM/tool execution

            // Send progress events
            for i in 1..=5 {
                if !stream_handler_clone.is_active() {
                    break;
                }

                if let Err(e) = stream_handler_clone
                    .send_progress(format!("Processing step {} of 5...", i))
                    .await
                {
                    tracing::error!("Failed to send progress event: {}", e);
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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

            // Mark task as completed in database
            if stream_handler_clone.is_active() {
                if let Err(e) = TaskRepository::mark_completed(&pool, &task_id_clone).await {
                    tracing::error!("Failed to mark task as completed: {}", e);
                    let _ = stream_handler_clone.send_failed(&task_id_clone, &format!("Failed to complete task: {}", e)).await;
                    return;
                }

                // Send completion event
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
