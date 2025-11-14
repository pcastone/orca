//! System information and metrics endpoint handlers

use axum::extract::State;
use std::sync::Arc;

use crate::api::models::{SystemInfoResponse, SystemMetricsResponse, StatusResponse};
use crate::api::response;
use crate::api::ws::BroadcastState;
use crate::db::DatabaseConnection;
use crate::db::repositories::ConfigurationRepository;

/// Get system information
///
/// GET /api/v1/system/info
pub async fn system_info() -> impl axum::response::IntoResponse {
    let info = SystemInfoResponse {
        version: crate::version::VERSION.to_string(),
        build_timestamp: env!("CARGO_PKG_VERSION").to_string(),
        git_commit: "unknown".to_string(),
        rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
    };
    response::ok(info)
}

/// Get system metrics
///
/// GET /api/v1/system/metrics
pub async fn system_metrics(
    State(app_state): State<crate::api::routes::AppState>,
) -> impl axum::response::IntoResponse {
    // Attempt to gather metrics from the database
    let metrics = match gather_system_metrics(&app_state.db).await {
        Ok(m) => m,
        Err(_) => {
            // Return default metrics if unable to query database
            SystemMetricsResponse {
                total_tasks: 0,
                active_tasks: 0,
                total_workflows: 0,
                total_executions: 0,
                avg_task_duration_ms: None,
                memory_bytes: None,
            }
        }
    };

    response::ok(metrics)
}

/// Get server status
///
/// GET /api/status
pub async fn status(
    State(app_state): State<crate::api::routes::AppState>,
) -> impl axum::response::IntoResponse {
    let pool = app_state.db.pool();
    
    // Get server name from config (stored in DB)
    let server_name = ConfigurationRepository::get(pool, "server.name")
        .await
        .ok()
        .flatten()
        .map(|c| c.value)
        .unwrap_or_else(|| "orchestrator-server".to_string());
    
    // Get or generate UUID
    let uuid = get_or_create_server_uuid(pool).await
        .unwrap_or_else(|_| "unknown".to_string());
    
    // Check database connection
    let db_status = match app_state.db.health_check().await {
        Ok(_) => "connected".to_string(),
        Err(_) => "disconnected".to_string(),
    };
    
    // Get connected clients count (number of active receivers)
    let connected_clients = app_state.broadcast.tx.receiver_count() as u32;
    
    let status = StatusResponse {
        name: server_name,
        uuid,
        version: crate::version::VERSION.to_string(),
        status: "running".to_string(),
        connected_clients,
        database: db_status,
    };
    
    response::ok(status)
}

/// Get or create server UUID, storing it in the database
async fn get_or_create_server_uuid(pool: &crate::db::DatabasePool) -> Result<String, sqlx::Error> {
    // Try to get existing UUID
    if let Some(config) = ConfigurationRepository::get(pool, "server.uuid").await? {
        return Ok(config.value);
    }
    
    // Generate new UUID
    let uuid = uuid::Uuid::new_v4().to_string();
    
    // Store in database
    ConfigurationRepository::set(
        pool,
        "server.uuid".to_string(),
        uuid.clone(),
        "string".to_string(),
    ).await?;
    
    Ok(uuid)
}

/// Helper function to gather metrics from database
async fn gather_system_metrics(db: &crate::db::DatabaseConnection) -> Result<SystemMetricsResponse, Box<dyn std::error::Error>> {
    use crate::db::repositories::{TaskRepository, WorkflowRepository, ToolExecutionRepository};

    let pool = db.pool();

    let total_tasks = TaskRepository::count(pool).await.unwrap_or(0);
    let active_tasks = TaskRepository::count_by_status(pool, "running").await.unwrap_or(0);
    let total_workflows = WorkflowRepository::count(pool).await.unwrap_or(0);
    let total_executions = ToolExecutionRepository::count(pool).await.unwrap_or(0);

    Ok(SystemMetricsResponse {
        total_tasks,
        active_tasks,
        total_workflows,
        total_executions,
        avg_task_duration_ms: None,
        memory_bytes: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_response() {
        let info = SystemInfoResponse {
            version: "1.0.0".to_string(),
            build_timestamp: "2025-01-01".to_string(),
            git_commit: "abc123".to_string(),
            rust_version: "1.75".to_string(),
        };
        assert_eq!(info.version, "1.0.0");
    }
}
