//! API request handlers
//!
//! Provides handler functions for all API endpoints organized by resource.

pub mod health;
pub mod tasks;
pub mod workflows;
pub mod tool_executions;
pub mod system;
pub mod realtime;

pub use health::{health, health_detailed};
pub use tasks::{create_task, list_tasks, get_task, update_task, delete_task};
pub use workflows::{create_workflow, list_workflows, get_workflow, update_workflow, delete_workflow};
pub use tool_executions::{execute_tool, list_task_executions, list_executions, get_execution};
pub use system::{system_info, system_metrics, status};
pub use realtime::{get_realtime_stats, get_connection_status, get_performance_metrics};

/// Import WebSocket handler from ws module
pub fn ws_handler(
    state: axum::extract::State<crate::db::DatabaseConnection>,
    broadcast_state: axum::extract::State<std::sync::Arc<crate::api::ws::BroadcastState>>,
) -> impl std::future::Future<Output = impl axum::response::IntoResponse> {
    async move {
        crate::api::ws::ws_handler(state, broadcast_state).await
    }
}
