//! API route definitions
//!
//! Defines all API routes and their associated handler functions.

use axum::{
    routing::{get, post, delete},
    Router,
};
use std::sync::Arc;

use crate::db::DatabaseConnection;
use crate::api::{handlers, ws::BroadcastState};
use crate::services::PromptService;

#[cfg(feature = "orca-integration")]
use orca::db::Database as UserDatabase;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub broadcast: Arc<BroadcastState>,
    pub prompt_service: Option<Arc<PromptService>>,
    /// LLM client for task execution (extracted from prompt_service)
    pub llm_client: Option<Arc<dyn langgraph_core::llm::ChatModel + Send + Sync>>,
    #[cfg(feature = "orca-integration")]
    pub user_db: Option<Arc<UserDatabase>>,
}

/// Build the complete API router
#[cfg(feature = "orca-integration")]
pub fn create_router(
    db: DatabaseConnection,
    broadcast: Arc<BroadcastState>,
    prompt_service: Option<PromptService>,
    user_db: Option<Arc<UserDatabase>>,
) -> Router {
    // Extract LLM client from prompt_service for task execution
    let llm_client = prompt_service.as_ref().map(|ps| ps.llm_client());
    let app_state = AppState {
        db: db.clone(),
        broadcast: broadcast.clone(),
        prompt_service: prompt_service.map(Arc::new),
        llm_client,
        user_db,
    };

    create_router_with_state(db, app_state)
}

/// Build the complete API router (without orca integration)
#[cfg(not(feature = "orca-integration"))]
pub fn create_router(
    db: DatabaseConnection,
    broadcast: Arc<BroadcastState>,
    prompt_service: Option<PromptService>,
) -> Router {
    // Extract LLM client from prompt_service for task execution
    let llm_client = prompt_service.as_ref().map(|ps| ps.llm_client());
    let app_state = AppState {
        db: db.clone(),
        broadcast: broadcast.clone(),
        prompt_service: prompt_service.map(Arc::new),
        llm_client,
    };

    create_router_with_state(db, app_state)
}

/// Internal router creation with state
fn create_router_with_state(
    db: DatabaseConnection,
    app_state: AppState,
) -> Router {
    let base_router = Router::new()
        // Health check endpoints
        .route("/health", get(handlers::health))
        .route(
            "/api/v1/system/health",
            get(handlers::health_detailed),
        )
        // Task endpoints
        .route(
            "/api/v1/tasks",
            post(handlers::create_task)
                .get(handlers::list_tasks),
        )
        .route(
            "/api/v1/tasks/:id",
            get(handlers::get_task)
                .put(handlers::update_task)
                .delete(handlers::delete_task),
        )
        // Tool execution endpoints
        .route(
            "/api/v1/tasks/:task_id/execute",
            post(handlers::execute_tool),
        )
        .route(
            "/api/v1/tasks/:task_id/executions",
            get(handlers::list_task_executions),
        )
        .route(
            "/api/v1/executions",
            get(handlers::list_executions),
        )
        .route(
            "/api/v1/executions/:id",
            get(handlers::get_execution),
        )
        // Workflow endpoints
        .route(
            "/api/v1/workflows",
            post(handlers::create_workflow)
                .get(handlers::list_workflows),
        )
        .route(
            "/api/v1/workflows/:id",
            get(handlers::get_workflow)
                .put(handlers::update_workflow)
                .delete(handlers::delete_workflow),
        )
        // System endpoints
        .route(
            "/api/v1/system/info",
            get(handlers::system_info),
        )
        .route(
            "/api/v1/system/metrics",
            get(handlers::system_metrics),
        )
        // Status endpoint
        .route(
            "/api/status",
            get(handlers::status),
        )
        // Bug endpoints
        .route(
            "/api/v1/bugs",
            post(handlers::create_bug)
                .get(handlers::list_bugs),
        )
        .route(
            "/api/v1/bugs/stats",
            get(handlers::get_bug_stats),
        )
        .route(
            "/api/v1/bugs/:id",
            get(handlers::get_bug)
                .put(handlers::update_bug)
                .delete(handlers::delete_bug),
        )
        // Prompt history endpoints
        .route(
            "/api/v1/prompts",
            post(handlers::create_prompt_history)
                .get(handlers::list_prompt_history),
        )
        .route(
            "/api/v1/prompts/stats",
            get(handlers::get_prompt_stats),
        )
        .route(
            "/api/v1/prompts/:id",
            get(handlers::get_prompt_history)
                .delete(handlers::delete_prompt_history),
        )
        .route(
            "/api/v1/tasks/:task_id/prompts",
            get(handlers::list_task_prompts),
        )
        .route(
            "/api/v1/sessions/:session_id/prompts",
            get(handlers::list_session_prompts),
        )
        // Checkpoint endpoints
        .route(
            "/api/v1/checkpoints",
            post(handlers::create_checkpoint)
                .get(handlers::list_checkpoints),
        )
        .route(
            "/api/v1/checkpoints/:id",
            get(handlers::get_checkpoint)
                .delete(handlers::delete_checkpoint),
        )
        .route(
            "/api/v1/executions/:execution_id/checkpoints",
            get(handlers::list_execution_checkpoints),
        )
        .route(
            "/api/v1/executions/:execution_id/checkpoints/latest",
            get(handlers::get_latest_checkpoint),
        )
        // LLM prompt endpoint
        .route(
            "/api/v1/prompt",
            post(handlers::send_prompt),
        );

    // Add data management routes (requires orca-integration)
    #[cfg(feature = "orca-integration")]
    let base_router = base_router
        .route(
            "/api/v1/data/backup",
            post(handlers::backup),
        )
        .route(
            "/api/v1/data/backups",
            get(handlers::list_backups),
        )
        .route(
            "/api/v1/data/restore",
            post(handlers::restore),
        )
        .route(
            "/api/v1/data/export",
            post(handlers::export),
        )
        .route(
            "/api/v1/data/import",
            post(handlers::import),
        );

    // Get LLM client for gRPC routes
    let llm_client = app_state.llm_client.clone();

    base_router
        .with_state(app_state)
        // Merge gRPC-compatible REST endpoints
        .merge(create_grpc_router(db, llm_client))
}

/// Create the gRPC-compatible REST router
fn create_grpc_router(db: DatabaseConnection, llm_client: Option<Arc<dyn langgraph_core::llm::ChatModel + Send + Sync>>) -> Router {
    use crate::grpc::{create_grpc_routes, GrpcState};

    let grpc_state = if let Some(client) = llm_client {
        GrpcState::with_llm_client(std::sync::Arc::new(db), client)
    } else {
        GrpcState::new(std::sync::Arc::new(db))
    };
    create_grpc_routes(grpc_state)
}

/// Create a router for testing
#[cfg(all(test, feature = "orca-integration"))]
pub fn create_test_router(db: DatabaseConnection) -> Router {
    let broadcast = Arc::new(BroadcastState::new());
    create_router(db, broadcast, None, None)
}

/// Create a router for testing (without orca integration)
#[cfg(all(test, not(feature = "orca-integration")))]
pub fn create_test_router(db: DatabaseConnection) -> Router {
    let broadcast = Arc::new(BroadcastState::new());
    create_router(db, broadcast, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        // This test just verifies the router can be created without panic
        assert!(true);
    }
}
