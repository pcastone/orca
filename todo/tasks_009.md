# Task 009: Implement Task Service (gRPC)

## Objective
Implement the TaskService gRPC service with all RPC methods for task CRUD operations, integrating with TaskRepository and proper error handling.

## Priority
**CRITICAL** - Core server functionality

## Dependencies
- Task 007 (Server infrastructure)
- Task 008 (Database layer)

## Implementation Details

**File**: `src/crates/orchestrator/src/services/task.rs`

```rust
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
use domain::{Task, TaskStatus};
use tonic::{Request, Response, Status};
use std::sync::Arc;
use sqlx::SqlitePool;

pub struct TaskServiceImpl {
    repository: TaskRepository,
    stream_buffer_size: usize,
}

impl TaskServiceImpl {
    pub fn new(pool: Arc<Database>, stream_buffer_size: usize) -> Self {
        Self {
            repository: TaskRepository::new(pool.pool().clone()),
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
        let task_type = domain::TaskType::from_str(&req.task_type)
            .map_err(|_| OrchestratorError::Validation(
                format!("Invalid task type: {}", req.task_type)
            ))?;

        let mut task = Task::new(req.title, req.description, task_type);

        if let Some(config) = req.config {
            task.config = Some(serde_json::from_str(&config)
                .map_err(|e| OrchestratorError::Validation(
                    format!("Invalid config JSON: {}", e)
                ))?);
        }

        if let Some(metadata) = req.metadata {
            task.metadata = Some(serde_json::from_str(&metadata)
                .map_err(|e| OrchestratorError::Validation(
                    format!("Invalid metadata JSON: {}", e)
                ))?);
        }

        task.workspace_path = req.workspace_path;

        // Save to database
        self.repository.create(&task).await
            .map_err(OrchestratorError::from)?;

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
            .map_err(OrchestratorError::from)?
            .ok_or_else(|| OrchestratorError::task_not_found(&req.id))?;

        let proto_task = task_to_proto(&task);
        Ok(Response::new(proto_task))
    }

    async fn list_tasks(
        &self,
        request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let req = request.into_inner();

        let status = req.status
            .and_then(|s| TaskStatus::from_i32(s));

        let tasks = self.repository.list(
            req.limit,
            req.offset,
            status,
        ).await.map_err(OrchestratorError::from)?;

        let proto_tasks = tasks.iter()
            .map(task_to_proto)
            .collect();

        Ok(Response::new(ListTasksResponse {
            tasks: proto_tasks,
            total: tasks.len() as i32,
        }))
    }

    async fn update_task(
        &self,
        request: Request<UpdateTaskRequest>,
    ) -> Result<Response<ProtoTask>, Status> {
        let req = request.into_inner();

        let mut task = self.repository.get_by_id(&req.id).await
            .map_err(OrchestratorError::from)?
            .ok_or_else(|| OrchestratorError::task_not_found(&req.id))?;

        // Update fields
        if let Some(title) = req.title {
            task.title = title;
        }
        if let Some(description) = req.description {
            task.description = description;
        }
        if let Some(status) = req.status {
            task.status = TaskStatus::from_i32(status)
                .ok_or_else(|| OrchestratorError::Validation(
                    "Invalid task status".to_string()
                ))?;
        }

        task.updated_at = chrono::Utc::now();

        self.repository.update(&task).await
            .map_err(OrchestratorError::from)?;

        let proto_task = task_to_proto(&task);
        Ok(Response::new(proto_task))
    }

    async fn delete_task(
        &self,
        request: Request<DeleteTaskRequest>,
    ) -> Result<Response<DeleteTaskResponse>, Status> {
        let req = request.into_inner();

        let success = self.repository.delete(&req.id).await
            .map_err(OrchestratorError::from)?;

        Ok(Response::new(DeleteTaskResponse { success }))
    }

    type ExecuteTaskStream = ReceiverStream<Result<ExecutionEvent, Status>>;

    async fn execute_task(
        &self,
        request: Request<ExecuteTaskRequest>,
    ) -> Result<Response<Self::ExecuteTaskStream>, Status> {
        // This will be implemented in Task 012
        Err(Status::unimplemented("Task execution not yet implemented"))
    }
}
```

## Tests

Create `src/crates/orchestrator/tests/task_service.rs` with integration tests.

## Acceptance Criteria
- [ ] All CRUD operations implemented
- [ ] Input validation
- [ ] Error handling with proper status codes
- [ ] Integration with TaskRepository
- [ ] Proto-domain conversions
- [ ] Tests pass

## Complexity: Moderate
## Estimated Effort: 6-8 hours
