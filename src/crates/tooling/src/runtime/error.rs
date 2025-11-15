//! Error types for Tool Runtime SDK

use thiserror::Error;

/// Errors that can occur in the tool runtime
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Tool not found
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Invalid tool arguments
    #[error("Invalid tool arguments for {tool}: {message}")]
    InvalidArguments { tool: String, message: String },

    /// Policy violation
    #[error("Policy violation: {0}")]
    PolicyViolation(String),

    /// Execution timeout
    #[error("Tool execution timeout after {0}ms")]
    Timeout(u64),

    /// Tool execution failed
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Invalid session
    #[error("Invalid session: {0}")]
    InvalidSession(String),

    /// Path security violation
    #[error("Path security violation: {0}")]
    PathSecurityViolation(String),

    /// Network policy violation
    #[error("Network policy violation: {0}")]
    NetworkPolicyViolation(String),

    /// Shell policy violation
    #[error("Shell policy violation: {0}")]
    ShellPolicyViolation(String),

    /// Git operation failed
    #[error("Git operation failed: {0}")]
    GitError(String),

    /// AST operation failed
    #[error("AST operation failed: {0}")]
    AstError(String),

    /// File I/O error
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl RuntimeError {
    /// Get the canonical error code for this error
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::ToolNotFound(_) => "E_TOOL_NOT_FOUND",
            Self::InvalidArguments { .. } => "E_INVALID_ARGS",
            Self::PolicyViolation(_) => "E_POLICY",
            Self::Timeout(_) => "E_TIMEOUT",
            Self::ExecutionFailed(_) => "E_EXECUTION",
            Self::SessionNotFound(_) => "E_SESSION_NOT_FOUND",
            Self::InvalidSession(_) => "E_INVALID_SESSION",
            Self::PathSecurityViolation(_) => "E_PATH_SECURITY",
            Self::NetworkPolicyViolation(_) => "E_NETWORK_POLICY",
            Self::ShellPolicyViolation(_) => "E_SHELL_POLICY",
            Self::GitError(_) => "E_GIT",
            Self::AstError(_) => "E_AST",
            Self::IoError(_) => "E_FILE_IO",
            Self::SerializationError(_) => "E_SERIALIZATION",
            Self::Internal(_) => "E_INTERNAL",
        }
    }
}

/// Result type for runtime operations
pub type Result<T> = std::result::Result<T, RuntimeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(
            RuntimeError::ToolNotFound("test".into()).error_code(),
            "E_TOOL_NOT_FOUND"
        );
        assert_eq!(
            RuntimeError::PolicyViolation("test".into()).error_code(),
            "E_POLICY"
        );
        assert_eq!(RuntimeError::Timeout(1000).error_code(), "E_TIMEOUT");
    }

    #[test]
    fn test_error_display() {
        let err = RuntimeError::ToolNotFound("file_read".to_string());
        assert_eq!(err.to_string(), "Tool not found: file_read");

        let err = RuntimeError::InvalidArguments {
            tool: "test".to_string(),
            message: "missing field".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid tool arguments for test: missing field"
        );
    }
}
