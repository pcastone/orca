//! Error types for utility functions.

use thiserror::Error;

/// Result type for utility operations.
pub type Result<T> = std::result::Result<T, UtilsError>;

/// Errors that can occur in utility operations.
#[derive(Debug, Error)]
pub enum UtilsError {
    /// HTTP error.
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Environment variable error.
    #[error("Environment variable error: {0}")]
    EnvError(#[from] std::env::VarError),

    /// Invalid input.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Server error.
    #[error("Server error: {0}")]
    ServerError(String),

    /// Client error.
    #[error("Client error: {0}")]
    ClientError(String),

    /// Generic error.
    #[error("{0}")]
    Other(String),
}

impl From<serde_json::Error> for UtilsError {
    fn from(err: serde_json::Error) -> Self {
        UtilsError::SerializationError(err.to_string())
    }
}

impl From<serde_yaml::Error> for UtilsError {
    fn from(err: serde_yaml::Error) -> Self {
        UtilsError::SerializationError(err.to_string())
    }
}

