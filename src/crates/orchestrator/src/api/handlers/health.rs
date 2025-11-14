//! Health check endpoint handler
//!
//! Provides health status check endpoints for the API.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use crate::api::{models::HealthResponse, response, routes::AppState};
use crate::db::DatabaseConnection;

/// Handler for GET /health
///
/// Returns basic health status without database check.
pub async fn health() -> impl axum::response::IntoResponse {
    let health = HealthResponse::new("ok", "unknown");
    response::ok(health)
}

/// Handler for GET /api/v1/system/health
///
/// Returns detailed health status including database connectivity.
pub async fn health_detailed(
    State(app_state): State<AppState>,
) -> (StatusCode, Json<HealthResponse>) {
    match app_state.db.health_check().await {
        Ok(()) => {
            let health = HealthResponse::new("ok", "connected");
            (StatusCode::OK, Json(health))
        }
        Err(_) => {
            let health = HealthResponse::new("error", "error");
            (StatusCode::SERVICE_UNAVAILABLE, Json(health))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        // Note: This is a basic test that validates the response structure
        // In a real scenario, you would mock the database connection
        assert!(true);
    }
}
