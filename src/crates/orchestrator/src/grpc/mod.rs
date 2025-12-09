//! gRPC service implementations for orchestrator
//!
//! This module provides gRPC-compatible JSON-over-HTTP endpoints that the
//! ACO client can call. Since we use hand-written proto definitions rather
//! than generated protobuf code, we implement a REST-like JSON API that
//! follows gRPC semantics.

mod task_service;
mod workflow_service;
pub mod dispatcher;
pub mod messages;
pub mod workers;

pub use task_service::TaskServiceImpl;
pub use workflow_service::WorkflowServiceImpl;
pub use workers::WorkerRegistry;
pub use dispatcher::ToolDispatcher;

use crate::db::DatabaseConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::Stream;
use std::sync::Arc;
use std::convert::Infallible;
use tracing::info;

use messages::{
    RegisterWorkerRequest, RegisterWorkerResponse, ToolResult, WorkerEvent, WorkerEventsParams,
};

/// Shared state for gRPC-style handlers
#[derive(Clone)]
pub struct GrpcState {
    pub task_service: Arc<TaskServiceImpl>,
    pub workflow_service: Arc<WorkflowServiceImpl>,
    pub worker_registry: Arc<WorkerRegistry>,
}

impl GrpcState {
    /// Create new gRPC state from database connection
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            task_service: Arc::new(TaskServiceImpl::new(db.clone())),
            workflow_service: Arc::new(WorkflowServiceImpl::new(db)),
            worker_registry: Arc::new(WorkerRegistry::new()),
        }
    }

    /// Create new gRPC state with an LLM client for real task execution
    pub fn with_llm_client(
        db: Arc<DatabaseConnection>,
        llm_client: Arc<dyn langgraph_core::llm::ChatModel + Send + Sync>,
    ) -> Self {
        Self {
            task_service: Arc::new(TaskServiceImpl::with_llm_client(db.clone(), llm_client)),
            workflow_service: Arc::new(WorkflowServiceImpl::new(db)),
            worker_registry: Arc::new(WorkerRegistry::new()),
        }
    }
}

/// Create gRPC-compatible REST router
///
/// This provides endpoints that match what the ACO TUI gRPC client expects,
/// but implemented as REST for simplicity since we don't have protoc-generated code.
pub fn create_grpc_routes(state: GrpcState) -> Router {
    Router::new()
        // Task endpoints
        .route("/grpc/tasks", get(list_tasks_handler))
        .route("/grpc/tasks", post(create_task_handler))
        .route("/grpc/tasks/:id", get(get_task_handler))
        .route("/grpc/tasks/:id/execute", post(execute_task_handler))
        // Workflow endpoints
        .route("/grpc/workflows", get(list_workflows_handler))
        .route("/grpc/workflows", post(create_workflow_handler))
        .route("/grpc/workflows/:id", get(get_workflow_handler))
        .route("/grpc/workflows/:id/execute", post(execute_workflow_handler))
        // Worker endpoints
        .route("/grpc/workers/register", post(register_worker_handler))
        .route("/grpc/workers/events", get(worker_events_handler))
        .route("/grpc/workers/results", post(tool_result_handler))
        .route("/grpc/workers", get(list_workers_handler))
        .with_state(state)
}

// Task handlers
use crate::proto::tasks::task_service_server::TaskService;
use crate::proto::workflows::workflow_service_server::WorkflowService;

async fn list_tasks_handler(
    State(state): State<GrpcState>,
) -> Result<Json<crate::proto::tasks::ListTasksResponse>, (StatusCode, String)> {
    use crate::proto::tasks::ListTasksRequest;
    use tonic::Request;

    let request = Request::new(ListTasksRequest {
        limit: 100,
        offset: 0,
        status: -1, // All statuses
    });

    TaskService::list_tasks(state.task_service.as_ref(), request)
        .await
        .map(|r| Json(r.into_inner()))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.message().to_string()))
}

async fn get_task_handler(
    State(state): State<GrpcState>,
    Path(id): Path<String>,
) -> Result<Json<crate::proto::tasks::Task>, (StatusCode, String)> {
    use crate::proto::tasks::GetTaskRequest;
    use tonic::Request;

    let request = Request::new(GetTaskRequest { id });

    TaskService::get_task(state.task_service.as_ref(), request)
        .await
        .map(|r| Json(r.into_inner()))
        .map_err(|e| {
            let status = if e.code() == tonic::Code::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, e.message().to_string())
        })
}

async fn create_task_handler(
    State(state): State<GrpcState>,
    Json(req): Json<crate::proto::tasks::CreateTaskRequest>,
) -> Result<Json<crate::proto::tasks::Task>, (StatusCode, String)> {
    use tonic::Request;

    TaskService::create_task(state.task_service.as_ref(), Request::new(req))
        .await
        .map(|r| Json(r.into_inner()))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.message().to_string()))
}

async fn execute_task_handler(
    State(state): State<GrpcState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<crate::proto::tasks::ExecutionEvent>>, (StatusCode, String)> {
    use crate::proto::tasks::ExecuteTaskRequest;
    use futures::StreamExt;
    use tonic::Request;

    let request = Request::new(ExecuteTaskRequest {
        id,
        parameters: None,
    });

    let stream = TaskService::execute_task(state.task_service.as_ref(), request)
        .await
        .map_err(|e| {
            let status = if e.code() == tonic::Code::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, e.message().to_string())
        })?;

    // Collect all events from the stream
    let events: Vec<_> = stream
        .into_inner()
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    Ok(Json(events))
}

// Workflow handlers

async fn list_workflows_handler(
    State(state): State<GrpcState>,
) -> Result<Json<crate::proto::workflows::ListWorkflowsResponse>, (StatusCode, String)> {
    use crate::proto::workflows::ListWorkflowsRequest;
    use tonic::Request;

    let request = Request::new(ListWorkflowsRequest {
        limit: 100,
        offset: 0,
    });

    WorkflowService::list_workflows(state.workflow_service.as_ref(), request)
        .await
        .map(|r| Json(r.into_inner()))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.message().to_string()))
}

async fn get_workflow_handler(
    State(state): State<GrpcState>,
    Path(id): Path<String>,
) -> Result<Json<crate::proto::workflows::Workflow>, (StatusCode, String)> {
    use crate::proto::workflows::GetWorkflowRequest;
    use tonic::Request;

    let request = Request::new(GetWorkflowRequest { id });

    WorkflowService::get_workflow(state.workflow_service.as_ref(), request)
        .await
        .map(|r| Json(r.into_inner()))
        .map_err(|e| {
            let status = if e.code() == tonic::Code::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, e.message().to_string())
        })
}

async fn create_workflow_handler(
    State(state): State<GrpcState>,
    Json(req): Json<crate::proto::workflows::CreateWorkflowRequest>,
) -> Result<Json<crate::proto::workflows::Workflow>, (StatusCode, String)> {
    use tonic::Request;

    WorkflowService::create_workflow(state.workflow_service.as_ref(), Request::new(req))
        .await
        .map(|r| Json(r.into_inner()))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.message().to_string()))
}

async fn execute_workflow_handler(
    State(state): State<GrpcState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<crate::proto::workflows::ExecutionEvent>>, (StatusCode, String)> {
    use crate::proto::workflows::ExecuteWorkflowRequest;
    use futures::StreamExt;
    use tonic::Request;

    let request = Request::new(ExecuteWorkflowRequest {
        id,
        parameters: None,
    });

    let stream = WorkflowService::execute_workflow(state.workflow_service.as_ref(), request)
        .await
        .map_err(|e| {
            let status = if e.code() == tonic::Code::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, e.message().to_string())
        })?;

    // Collect all events from the stream
    let events: Vec<_> = stream
        .into_inner()
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    Ok(Json(events))
}

// Worker handlers

/// Register a new worker
async fn register_worker_handler(
    State(state): State<GrpcState>,
    Json(req): Json<RegisterWorkerRequest>,
) -> Result<Json<RegisterWorkerResponse>, (StatusCode, String)> {
    info!("Registering worker: {} with capabilities: {:?}", req.name, req.capabilities);

    let worker_id = state.worker_registry.register_worker(
        req.name,
        req.capabilities,
        req.workspace_path,
    );

    Ok(Json(RegisterWorkerResponse {
        worker_id,
        heartbeat_interval_ms: 30000, // 30 seconds
    }))
}

/// SSE event stream for workers to receive tool requests
async fn worker_events_handler(
    State(state): State<GrpcState>,
    Query(params): Query<WorkerEventsParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
    let worker_id = params.worker_id.clone();

    // Verify worker exists
    if state.worker_registry.get_worker(&worker_id).is_none() {
        return Err((StatusCode::NOT_FOUND, format!("Worker not found: {}", worker_id)));
    }

    // Subscribe to worker events
    let rx = state.worker_registry.subscribe(&worker_id)
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to subscribe".to_string()))?;

    info!("Worker {} connected to event stream", worker_id);

    // Convert broadcast receiver to SSE stream using async_stream
    let stream = async_stream::stream! {
        let mut rx = rx;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Ok(json) = serde_json::to_string(&event) {
                        let event_type = match &event {
                            WorkerEvent::ToolRequest(_) => "tool_request",
                            WorkerEvent::Heartbeat { .. } => "heartbeat",
                            WorkerEvent::Shutdown { .. } => "shutdown",
                        };
                        yield Ok(Event::default().event(event_type).data(json));
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    // Skip lagged messages, continue
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Channel closed, end stream
                    break;
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// Receive tool execution results from workers
async fn tool_result_handler(
    State(state): State<GrpcState>,
    Json(result): Json<ToolResult>,
) -> Result<Json<()>, (StatusCode, String)> {
    info!(
        "Received tool result for request {} from worker {}: success={}",
        result.request_id, result.worker_id, result.success
    );

    // Update worker heartbeat
    state.worker_registry.update_heartbeat(&result.worker_id);

    // Complete the pending request
    state.worker_registry.complete_request(result)
        .map_err(|e| (StatusCode::NOT_FOUND, e))?;

    Ok(Json(()))
}

/// List connected workers
async fn list_workers_handler(
    State(state): State<GrpcState>,
) -> Json<Vec<WorkerInfoResponse>> {
    let workers = state.worker_registry.list_workers();
    Json(workers.into_iter().map(|w| WorkerInfoResponse {
        id: w.id,
        name: w.name,
        capabilities: w.capabilities,
        workspace_path: w.workspace_path,
        connected_at: w.connected_at.to_rfc3339(),
        pending_count: w.pending_count,
    }).collect())
}

/// Worker info response for list endpoint
#[derive(serde::Serialize)]
struct WorkerInfoResponse {
    id: String,
    name: String,
    capabilities: Vec<String>,
    workspace_path: String,
    connected_at: String,
    pending_count: u32,
}
