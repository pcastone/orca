//! Error types for aco application

use thiserror::Error;

/// Errors that can occur in the aco application
#[derive(Debug, Error)]
pub enum AcoError {
    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Tool execution error
    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    /// Session error
    #[error("Session error: {0}")]
    Session(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP request error
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// TOML parsing error
    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    /// General error
    #[error("aco error: {0}")]
    General(String),
}

/// Result type for aco operations
pub type Result<T> = std::result::Result<T, AcoError>;

