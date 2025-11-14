//! Error types for LLM provider implementations.

use thiserror::Error;

/// Result type for LLM operations.
pub type Result<T> = std::result::Result<T, LlmError>;

/// Errors that can occur when working with LLM providers.
#[derive(Debug, Error)]
pub enum LlmError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Failed to serialize/deserialize data.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// API authentication failed.
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// API key not found in environment.
    #[error("API key not found: {0}")]
    ApiKeyNotFound(String),

    /// Model not found or unavailable.
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Provider service unavailable (e.g., Ollama not running).
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Invalid request parameters.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Invalid response from provider.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Request timeout.
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// General provider error.
    #[error("Provider error: {0}")]
    ProviderError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Generic error with context.
    #[error("{0}")]
    Other(String),
}

impl LlmError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            LlmError::HttpError(_)
                | LlmError::ServiceUnavailable(_)
                | LlmError::Timeout(_)
                | LlmError::RateLimitExceeded(_)
        )
    }

    /// Check if this error is due to authentication.
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            LlmError::AuthenticationError(_) | LlmError::ApiKeyNotFound(_)
        )
    }
}

impl From<serde_json::Error> for LlmError {
    fn from(err: serde_json::Error) -> Self {
        LlmError::SerializationError(err.to_string())
    }
}

/// Convert LlmError to langgraph_core::error::GraphError for trait implementation.
impl From<LlmError> for langgraph_core::error::GraphError {
    fn from(err: LlmError) -> Self {
        langgraph_core::error::GraphError::Validation(err.to_string())
    }
}

