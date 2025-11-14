//! REST API Layer for the orchestrator
//!
//! Provides HTTP/REST endpoints for orchestrator operations including:
//! - Task CRUD operations
//! - Workflow management
//! - Tool execution
//! - System health and metrics
//! - WebSocket real-time updates

pub mod error;
pub mod response;
pub mod middleware;
pub mod models;
pub mod handlers;
pub mod routes;
pub mod ws;

pub use error::{ApiError, ApiResult, ApiErrorResponse};
pub use response::{SuccessResponse, ErrorResponse, PaginatedResponse};
pub use routes::create_router;
pub use middleware::cors_layer;

/// Re-export commonly used items
pub mod prelude {
    pub use crate::api::error::{ApiError, ApiResult};
    pub use crate::api::models::*;
    pub use crate::api::response::*;
}
