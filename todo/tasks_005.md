# Task 005: Implement gRPC Error Handling and Status Codes

## Objective
Create comprehensive error handling layer for gRPC services with proper status codes, error mapping, and client-friendly error messages following gRPC best practices.

## Priority
**HIGH** - Essential for production-quality error reporting

## Dependencies
- Task 001 (Protocol Buffer definitions)
- Task 003 (Domain models with error types)

## Implementation Details

### Files to Create

1. **`src/crates/orchestrator/src/error.rs`**:
```rust
use tonic::{Status, Code};
use domain::DomainError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {resource} with id {id}")]
    NotFound { resource: String, id: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Service unavailable: {0}")]
    Unavailable(String),
}

impl From<OrchestratorError> for Status {
    fn from(err: OrchestratorError) -> Self {
        match err {
            OrchestratorError::NotFound { resource, id } => {
                Status::not_found(format!("{} not found: {}", resource, id))
            }
            OrchestratorError::Validation(msg) => {
                Status::invalid_argument(msg)
            }
            OrchestratorError::Authentication(msg) => {
                Status::unauthenticated(msg)
            }
            OrchestratorError::Authorization(msg) => {
                Status::permission_denied(msg)
            }
            OrchestratorError::BadRequest(msg) => {
                Status::invalid_argument(msg)
            }
            OrchestratorError::Unavailable(msg) => {
                Status::unavailable(msg)
            }
            OrchestratorError::Database(e) => {
                tracing::error!("Database error: {}", e);
                Status::internal("Database error")
            }
            OrchestratorError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                Status::internal("Internal server error")
            }
            OrchestratorError::Domain(e) => {
                match e {
                    DomainError::TaskNotFound(id) => {
                        Status::not_found(format!("Task not found: {}", id))
                    }
                    DomainError::WorkflowNotFound(id) => {
                        Status::not_found(format!("Workflow not found: {}", id))
                    }
                    DomainError::InvalidTaskStatus(s) => {
                        Status::invalid_argument(format!("Invalid task status: {}", s))
                    }
                    DomainError::InvalidWorkflowStatus(s) => {
                        Status::invalid_argument(format!("Invalid workflow status: {}", s))
                    }
                    DomainError::InvalidStateTransition { from, to } => {
                        Status::failed_precondition(
                            format!("Invalid state transition from {} to {}", from, to)
                        )
                    }
                    _ => Status::internal("Domain error"),
                }
            }
            OrchestratorError::Execution(msg) => {
                tracing::error!("Execution error: {}", msg);
                Status::internal(format!("Execution error: {}", msg))
            }
            OrchestratorError::Llm(msg) => {
                tracing::error!("LLM error: {}", msg);
                Status::unavailable(format!("LLM service error: {}", msg))
            }
        }
    }
}

// Convenience result type
pub type Result<T> = std::result::Result<T, OrchestratorError>;

// Helper functions for common errors
impl OrchestratorError {
    pub fn task_not_found(id: impl Into<String>) -> Self {
        OrchestratorError::NotFound {
            resource: "Task".to_string(),
            id: id.into(),
        }
    }

    pub fn workflow_not_found(id: impl Into<String>) -> Self {
        OrchestratorError::NotFound {
            resource: "Workflow".to_string(),
            id: id.into(),
        }
    }

    pub fn invalid_id(id: impl Into<String>) -> Self {
        OrchestratorError::Validation(format!("Invalid ID: {}", id.into()))
    }

    pub fn missing_field(field: impl Into<String>) -> Self {
        OrchestratorError::Validation(format!("Missing required field: {}", field.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_to_status() {
        let err = OrchestratorError::task_not_found("task-123");
        let status: Status = err.into();

        assert_eq!(status.code(), Code::NotFound);
        assert!(status.message().contains("task-123"));
    }

    #[test]
    fn test_validation_to_status() {
        let err = OrchestratorError::Validation("Invalid input".to_string());
        let status: Status = err.into();

        assert_eq!(status.code(), Code::InvalidArgument);
    }

    #[test]
    fn test_authentication_to_status() {
        let err = OrchestratorError::Authentication("Invalid token".to_string());
        let status: Status = err.into();

        assert_eq!(status.code(), Code::Unauthenticated);
    }

    #[test]
    fn test_domain_error_mapping() {
        let err = OrchestratorError::Domain(
            DomainError::TaskNotFound("task-456".to_string())
        );
        let status: Status = err.into();

        assert_eq!(status.code(), Code::NotFound);
        assert!(status.message().contains("task-456"));
    }

    #[test]
    fn test_helper_functions() {
        let err = OrchestratorError::task_not_found("t1");
        assert!(matches!(err, OrchestratorError::NotFound { .. }));

        let err = OrchestratorError::missing_field("title");
        assert!(matches!(err, OrchestratorError::Validation(_)));
    }
}
```

2. **`src/crates/aco/src/error.rs`**:
```rust
use tonic::{Status, Code};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcoError {
    #[error("gRPC error: {0}")]
    Grpc(#[from] Status),

    #[error("Transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AcoError>;

impl AcoError {
    /// Convert gRPC Status to user-friendly error message
    pub fn from_status(status: Status) -> Self {
        match status.code() {
            Code::NotFound => {
                AcoError::Grpc(Status::not_found(
                    format!("Resource not found: {}", status.message())
                ))
            }
            Code::Unauthenticated => AcoError::AuthenticationRequired,
            Code::PermissionDenied => {
                AcoError::Grpc(Status::permission_denied(
                    format!("Permission denied: {}", status.message())
                ))
            }
            Code::InvalidArgument => {
                AcoError::Grpc(Status::invalid_argument(
                    format!("Invalid input: {}", status.message())
                ))
            }
            Code::Unavailable => {
                AcoError::Connection(
                    format!("Service unavailable: {}", status.message())
                )
            }
            Code::DeadlineExceeded => {
                AcoError::Timeout(status.message().to_string())
            }
            _ => AcoError::Grpc(status),
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            AcoError::Grpc(status) => {
                matches!(
                    status.code(),
                    Code::Unavailable | Code::DeadlineExceeded | Code::ResourceExhausted
                )
            }
            AcoError::Connection(_) | AcoError::Timeout(_) | AcoError::Transport(_) => true,
            _ => false,
        }
    }

    /// Get user-friendly error message for display
    pub fn user_message(&self) -> String {
        match self {
            AcoError::Grpc(status) => {
                match status.code() {
                    Code::NotFound => "Resource not found".to_string(),
                    Code::Unauthenticated => "Authentication required. Please log in.".to_string(),
                    Code::PermissionDenied => "You don't have permission for this action.".to_string(),
                    Code::InvalidArgument => "Invalid input provided.".to_string(),
                    Code::Unavailable => "Service is currently unavailable. Please try again later.".to_string(),
                    _ => format!("Error: {}", status.message()),
                }
            }
            AcoError::Connection(_) => "Cannot connect to server. Check your network connection.".to_string(),
            AcoError::Timeout(_) => "Request timed out. Please try again.".to_string(),
            _ => self.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_status_not_found() {
        let status = Status::not_found("task not found");
        let err = AcoError::from_status(status);

        assert!(matches!(err, AcoError::Grpc(_)));
    }

    #[test]
    fn test_from_status_unauthenticated() {
        let status = Status::unauthenticated("invalid token");
        let err = AcoError::from_status(status);

        assert!(matches!(err, AcoError::AuthenticationRequired));
    }

    #[test]
    fn test_is_retryable() {
        let err = AcoError::Connection("timeout".to_string());
        assert!(err.is_retryable());

        let err = AcoError::AuthenticationRequired;
        assert!(!err.is_retryable());

        let err = AcoError::Grpc(Status::unavailable("server down"));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_user_message() {
        let err = AcoError::AuthenticationRequired;
        assert!(err.user_message().contains("log in"));

        let err = AcoError::Connection("failed".to_string());
        assert!(err.user_message().contains("network"));
    }
}
```

3. **`src/crates/orchestrator/src/middleware/error_logging.rs`**:
```rust
use tonic::{Request, Status};
use tracing::{error, warn, info};

/// Log errors based on severity
pub fn log_error(err: &Status, request_path: &str) {
    match err.code() {
        tonic::Code::Internal | tonic::Code::DataLoss | tonic::Code::Unknown => {
            error!(
                path = request_path,
                code = ?err.code(),
                message = err.message(),
                "Server error occurred"
            );
        }
        tonic::Code::InvalidArgument | tonic::Code::NotFound => {
            warn!(
                path = request_path,
                code = ?err.code(),
                message = err.message(),
                "Client error"
            );
        }
        _ => {
            info!(
                path = request_path,
                code = ?err.code(),
                message = err.message(),
                "Request failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_error_runs_without_panic() {
        // Just ensure logging doesn't panic
        log_error(&Status::internal("test"), "/test.path");
        log_error(&Status::not_found("test"), "/test.path");
        log_error(&Status::unauthenticated("test"), "/test.path");
    }
}
```

## Update Cargo.toml

**`src/crates/orchestrator/Cargo.toml`**:
```toml
[dependencies]
sqlx = { workspace = true, features = ["sqlite", "runtime-tokio-rustls"] }
tracing = { workspace = true }
```

## Unit Tests

All tests embedded in implementation files.

## Acceptance Criteria

- [ ] OrchestratorError with all error variants
- [ ] Conversion from OrchestratorError to tonic::Status
- [ ] Proper gRPC status codes for each error type
- [ ] AcoError with user-friendly messages
- [ ] is_retryable() logic for client retries
- [ ] user_message() for UI display
- [ ] Error logging middleware
- [ ] Helper functions for common errors
- [ ] All tests pass
- [ ] Sensitive information not leaked in errors

## Complexity
**Moderate** - Standard error handling patterns

## Estimated Effort
**4-5 hours**

## Notes
- Don't expose internal errors to clients (use generic messages)
- Log detailed errors server-side for debugging
- Map database errors to Internal (don't expose schema)
- Authentication errors should not reveal whether user exists
- Use proper gRPC status codes for REST API compatibility
