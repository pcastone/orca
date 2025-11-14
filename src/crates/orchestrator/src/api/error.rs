//! API error types and HTTP response conversion
//!
//! Provides custom error types for API operations with conversion to Axum HTTP responses.
//! Automatically converts database errors to appropriate HTTP status codes.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::db::DatabaseError;

/// API error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    /// Error type identifier
    pub error: String,
    /// Human-readable error message
    pub message: String,
    /// Error code for programmatic handling
    pub code: String,
}

impl ApiErrorResponse {
    /// Create a new API error response
    pub fn new(error: impl Into<String>, message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            code: code.into(),
        }
    }
}

/// API result type
pub type ApiResult<T> = Result<T, ApiError>;

/// Custom API error type
#[derive(Debug, Error)]
pub enum ApiError {
    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid request data
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Validation error
    #[error("Validation failed: {0}")]
    ValidationError(String),

    /// Conflict (e.g., duplicate resource)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Internal server error
    #[error("Internal server error: {0}")]
    InternalError(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),

    /// Unauthorized
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Forbidden
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// JSON parsing error
    #[error("JSON error: {0}")]
    JsonError(String),
}

impl ApiError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::DatabaseError(db_err) => {
                if db_err.is_not_found() {
                    StatusCode::NOT_FOUND
                } else if db_err.is_constraint_violation() {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            }
            ApiError::JsonError(_) => StatusCode::BAD_REQUEST,
        }
    }

    /// Get the error code identifier
    pub fn code(&self) -> &'static str {
        match self {
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::BadRequest(_) => "BAD_REQUEST",
            ApiError::ValidationError(_) => "VALIDATION_ERROR",
            ApiError::Conflict(_) => "CONFLICT",
            ApiError::Unauthorized(_) => "UNAUTHORIZED",
            ApiError::Forbidden(_) => "FORBIDDEN",
            ApiError::InternalError(_) => "INTERNAL_ERROR",
            ApiError::DatabaseError(db_err) => {
                if db_err.is_not_found() {
                    "DB_NOT_FOUND"
                } else if db_err.is_constraint_violation() {
                    "DB_CONSTRAINT_VIOLATION"
                } else {
                    "DB_ERROR"
                }
            }
            ApiError::JsonError(_) => "JSON_ERROR",
        }
    }

    /// Get the error type name
    pub fn error_type(&self) -> &'static str {
        match self {
            ApiError::NotFound(_) => "NotFound",
            ApiError::BadRequest(_) => "BadRequest",
            ApiError::ValidationError(_) => "ValidationError",
            ApiError::Conflict(_) => "Conflict",
            ApiError::Unauthorized(_) => "Unauthorized",
            ApiError::Forbidden(_) => "Forbidden",
            ApiError::InternalError(_) => "InternalError",
            ApiError::DatabaseError(_) => "DatabaseError",
            ApiError::JsonError(_) => "JsonError",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ApiErrorResponse::new(self.error_type(), self.to_string(), self.code());

        tracing::error!("API Error: {:?}", body);

        (status, Json(body)).into_response()
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::JsonError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = ApiError::NotFound("resource".to_string());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(err.code(), "NOT_FOUND");
        assert_eq!(err.error_type(), "NotFound");
    }

    #[test]
    fn test_validation_error() {
        let err = ApiError::ValidationError("invalid input".to_string());
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn test_bad_request_error() {
        let err = ApiError::BadRequest("malformed".to_string());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(err.code(), "BAD_REQUEST");
    }

    #[test]
    fn test_conflict_error() {
        let err = ApiError::Conflict("duplicate".to_string());
        assert_eq!(err.status_code(), StatusCode::CONFLICT);
        assert_eq!(err.code(), "CONFLICT");
    }

    #[test]
    fn test_internal_error() {
        let err = ApiError::InternalError("something went wrong".to_string());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.code(), "INTERNAL_ERROR");
    }

    #[test]
    fn test_unauthorized_error() {
        let err = ApiError::Unauthorized("no token".to_string());
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(err.code(), "UNAUTHORIZED");
    }

    #[test]
    fn test_forbidden_error() {
        let err = ApiError::Forbidden("access denied".to_string());
        assert_eq!(err.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(err.code(), "FORBIDDEN");
    }
}
