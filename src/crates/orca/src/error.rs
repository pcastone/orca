//! Error types for Orca
//!
//! Provides a unified error type for all Orca operations.

use std::fmt;

/// Result type alias for Orca operations
pub type Result<T> = std::result::Result<T, OrcaError>;

/// Main error type for Orca operations
#[derive(Debug)]
pub enum OrcaError {
    /// Configuration error
    Config(String),

    /// Database error
    Database(String),

    /// Tool execution error
    ToolExecution(String),

    /// Workflow execution error
    Workflow(String),

    /// Pattern execution error
    Pattern(String),

    /// Execution error (agent/task execution)
    Execution(String),

    /// Task timeout error
    Timeout { task_id: String, duration_secs: u64 },

    /// Feature not yet implemented
    NotImplemented(String),

    /// Not found error
    NotFound(String),

    /// Budget exceeded error
    BudgetExceeded(String),

    /// LLM error
    LlmError(String),

    /// IO error
    Io(std::io::Error),

    /// Serialization/deserialization error
    Serde(serde_json::Error),

    /// SQL error
    Sqlx(sqlx::Error),

    /// Generic error with message
    Other(String),
}

impl fmt::Display for OrcaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Database(msg) => write!(f, "Database error: {}", msg),
            Self::ToolExecution(msg) => write!(f, "Tool execution error: {}", msg),
            Self::Workflow(msg) => write!(f, "Workflow error: {}", msg),
            Self::Pattern(msg) => write!(f, "Pattern error: {}", msg),
            Self::Execution(msg) => write!(f, "Execution error: {}", msg),
            Self::Timeout { task_id, duration_secs } => {
                write!(f, "Task {} timed out after {} seconds", task_id, duration_secs)
            }
            Self::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::BudgetExceeded(msg) => write!(f, "Budget exceeded: {}", msg),
            Self::LlmError(msg) => write!(f, "LLM error: {}", msg),
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Serde(err) => write!(f, "Serialization error: {}", err),
            Self::Sqlx(err) => write!(f, "SQL error: {}", err),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for OrcaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Serde(err) => Some(err),
            Self::Sqlx(err) => Some(err),
            _ => None,
        }
    }
}

// Conversions from common error types
impl From<std::io::Error> for OrcaError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for OrcaError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}

impl From<sqlx::Error> for OrcaError {
    fn from(err: sqlx::Error) -> Self {
        Self::Sqlx(err)
    }
}

impl From<anyhow::Error> for OrcaError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<String> for OrcaError {
    fn from(msg: String) -> Self {
        Self::Other(msg)
    }
}

impl From<&str> for OrcaError {
    fn from(msg: &str) -> Self {
        Self::Other(msg.to_string())
    }
}
