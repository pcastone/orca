//! Error types for checkpoint operations

use thiserror::Error;

/// Result type for checkpoint operations
pub type Result<T> = std::result::Result<T, CheckpointError>;

/// Errors that can occur during checkpoint operations
#[derive(Error, Debug)]
pub enum CheckpointError {
    /// Checkpoint not found
    #[error("Checkpoint not found: {0}")]
    NotFound(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Binary serialization error
    #[error("Binary serialization error: {0}")]
    BinarySerialization(#[from] bincode::Error),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Invalid checkpoint
    #[error("Invalid checkpoint: {0}")]
    Invalid(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Custom error
    #[error("{0}")]
    Custom(String),
}
