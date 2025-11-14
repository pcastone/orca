//! API response helpers
//!
//! Provides convenient helper functions for creating consistent API responses
//! with proper HTTP status codes and JSON serialization.

use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

/// Generic success response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse<T> {
    /// Whether the request was successful
    pub success: bool,
    /// Response data
    pub data: T,
}

impl<T: Serialize> SuccessResponse<T> {
    /// Create a new success response
    pub fn new(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Create a 200 OK JSON response
pub fn ok<T: Serialize>(data: T) -> impl IntoResponse {
    (StatusCode::OK, Json(SuccessResponse::new(data)))
}

/// Create a 201 Created JSON response
pub fn created<T: Serialize>(data: T) -> impl IntoResponse {
    (StatusCode::CREATED, Json(SuccessResponse::new(data)))
}

/// Create a 204 No Content response
pub fn no_content() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

/// Create a 400 Bad Request error response
pub fn bad_request(message: impl Into<String>) -> impl IntoResponse {
    let err = ErrorResponse::new("BadRequest", message.into(), "BAD_REQUEST");
    (StatusCode::BAD_REQUEST, Json(err))
}

/// Create a 404 Not Found error response
pub fn not_found(message: impl Into<String>) -> impl IntoResponse {
    let err = ErrorResponse::new("NotFound", message.into(), "NOT_FOUND");
    (StatusCode::NOT_FOUND, Json(err))
}

/// Create a 409 Conflict error response
pub fn conflict(message: impl Into<String>) -> impl IntoResponse {
    let err = ErrorResponse::new("Conflict", message.into(), "CONFLICT");
    (StatusCode::CONFLICT, Json(err))
}

/// Create a 422 Unprocessable Entity error response
pub fn validation_error(message: impl Into<String>) -> impl IntoResponse {
    let err = ErrorResponse::new("ValidationError", message.into(), "VALIDATION_ERROR");
    (StatusCode::UNPROCESSABLE_ENTITY, Json(err))
}

/// Create a 500 Internal Server Error response
pub fn internal_error(message: impl Into<String>) -> impl IntoResponse {
    let err = ErrorResponse::new("InternalError", message.into(), "INTERNAL_ERROR");
    (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
}

/// Generic error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Whether the request was successful (always false for errors)
    pub success: bool,
    /// Error type
    pub error: String,
    /// Error message
    pub message: String,
    /// Error code for programmatic handling
    pub code: String,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(error: impl Into<String>, message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            success: false,
            error: error.into(),
            message: message.into(),
            code: code.into(),
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Response data items
    pub data: Vec<T>,
    /// Current page number (0-indexed)
    pub page: u32,
    /// Number of items per page
    pub per_page: u32,
    /// Total number of items
    pub total: u32,
    /// Total number of pages
    pub pages: u32,
}

impl<T: Serialize> PaginatedResponse<T> {
    /// Create a new paginated response
    pub fn new(data: Vec<T>, page: u32, per_page: u32, total: u32) -> Self {
        let pages = (total + per_page - 1) / per_page; // Ceiling division
        Self {
            data,
            page,
            per_page,
            total,
            pages,
        }
    }
}

/// Create a paginated response
pub fn paginated<T: Serialize>(data: Vec<T>, page: u32, per_page: u32, total: u32) -> impl IntoResponse {
    (StatusCode::OK, Json(PaginatedResponse::new(data, page, per_page, total)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestData {
        id: u32,
        name: String,
    }

    #[test]
    fn test_success_response() {
        let data = TestData {
            id: 1,
            name: "test".to_string(),
        };
        let resp = SuccessResponse::new(data);
        assert!(resp.success);
    }

    #[test]
    fn test_paginated_response() {
        let data = vec![
            TestData { id: 1, name: "test1".to_string() },
            TestData { id: 2, name: "test2".to_string() },
        ];
        let resp = PaginatedResponse::new(data, 0, 10, 2);
        assert_eq!(resp.page, 0);
        assert_eq!(resp.per_page, 10);
        assert_eq!(resp.total, 2);
        assert_eq!(resp.pages, 1);
    }

    #[test]
    fn test_paginated_response_multiple_pages() {
        let data: Vec<i32> = vec![];
        let resp = PaginatedResponse::new(data, 0, 10, 25);
        assert_eq!(resp.total, 25);
        assert_eq!(resp.pages, 3);
    }
}
