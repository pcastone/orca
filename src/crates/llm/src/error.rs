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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_error_display() {
        let err = LlmError::SerializationError("invalid JSON".to_string());
        assert_eq!(err.to_string(), "Serialization error: invalid JSON");
    }

    #[test]
    fn test_authentication_error_display() {
        let err = LlmError::AuthenticationError("invalid token".to_string());
        assert_eq!(err.to_string(), "Authentication failed: invalid token");
    }

    #[test]
    fn test_api_key_not_found_display() {
        let err = LlmError::ApiKeyNotFound("OPENAI_API_KEY".to_string());
        assert_eq!(err.to_string(), "API key not found: OPENAI_API_KEY");
    }

    #[test]
    fn test_model_not_found_display() {
        let err = LlmError::ModelNotFound("gpt-5".to_string());
        assert_eq!(err.to_string(), "Model not found: gpt-5");
    }

    #[test]
    fn test_service_unavailable_display() {
        let err = LlmError::ServiceUnavailable("Ollama not running".to_string());
        assert_eq!(
            err.to_string(),
            "Service unavailable: Ollama not running"
        );
    }

    #[test]
    fn test_rate_limit_exceeded_display() {
        let err = LlmError::RateLimitExceeded("60 requests per minute".to_string());
        assert_eq!(
            err.to_string(),
            "Rate limit exceeded: 60 requests per minute"
        );
    }

    #[test]
    fn test_invalid_request_display() {
        let err = LlmError::InvalidRequest("missing required field".to_string());
        assert_eq!(err.to_string(), "Invalid request: missing required field");
    }

    #[test]
    fn test_invalid_response_display() {
        let err = LlmError::InvalidResponse("unexpected format".to_string());
        assert_eq!(err.to_string(), "Invalid response: unexpected format");
    }

    #[test]
    fn test_timeout_display() {
        let err = LlmError::Timeout("request took too long".to_string());
        assert_eq!(err.to_string(), "Request timeout: request took too long");
    }

    #[test]
    fn test_provider_error_display() {
        let err = LlmError::ProviderError("OpenAI API error".to_string());
        assert_eq!(err.to_string(), "Provider error: OpenAI API error");
    }

    #[test]
    fn test_config_error_display() {
        let err = LlmError::ConfigError("invalid base URL".to_string());
        assert_eq!(err.to_string(), "Configuration error: invalid base URL");
    }

    #[test]
    fn test_other_error_display() {
        let err = LlmError::Other("unknown error".to_string());
        assert_eq!(err.to_string(), "unknown error");
    }

    #[test]
    fn test_is_retryable_service_and_network_errors() {
        // HttpError would be retryable, but we can't easily construct it without reqwest internals
        // So we verify the pattern matches correctly with what we can test
        assert!(LlmError::ServiceUnavailable("test".to_string()).is_retryable());
        assert!(LlmError::Timeout("test".to_string()).is_retryable());
        assert!(LlmError::RateLimitExceeded("test".to_string()).is_retryable());
    }

    #[test]
    fn test_is_retryable_service_unavailable() {
        let err = LlmError::ServiceUnavailable("down".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_timeout() {
        let err = LlmError::Timeout("timeout".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_rate_limit() {
        let err = LlmError::RateLimitExceeded("limit".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_not_retryable_auth_error() {
        let err = LlmError::AuthenticationError("bad token".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_not_retryable_api_key_not_found() {
        let err = LlmError::ApiKeyNotFound("KEY".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_not_retryable_invalid_request() {
        let err = LlmError::InvalidRequest("bad params".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_not_retryable_model_not_found() {
        let err = LlmError::ModelNotFound("model".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_auth_error_authentication() {
        let err = LlmError::AuthenticationError("invalid".to_string());
        assert!(err.is_auth_error());
    }

    #[test]
    fn test_is_auth_error_api_key_not_found() {
        let err = LlmError::ApiKeyNotFound("KEY".to_string());
        assert!(err.is_auth_error());
    }

    #[test]
    fn test_is_not_auth_error_http() {
        // Test with other error types that are definitely not auth errors
        let err = LlmError::Timeout("test".to_string());
        assert!(!err.is_auth_error());

        let err2 = LlmError::ServiceUnavailable("test".to_string());
        assert!(!err2.is_auth_error());
    }

    #[test]
    fn test_is_not_auth_error_timeout() {
        let err = LlmError::Timeout("timeout".to_string());
        assert!(!err.is_auth_error());
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let llm_err: LlmError = json_err.into();

        match llm_err {
            LlmError::SerializationError(msg) => {
                assert!(msg.contains("expected"));
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_error_types_can_be_constructed() {
        // Test that all error types can be created
        let _http = "We can't easily construct HttpError without reqwest internals";
        let _serial = LlmError::SerializationError("test".to_string());
        let _auth = LlmError::AuthenticationError("test".to_string());
        let _key = LlmError::ApiKeyNotFound("test".to_string());
        let _model = LlmError::ModelNotFound("test".to_string());
        let _service = LlmError::ServiceUnavailable("test".to_string());
        let _rate = LlmError::RateLimitExceeded("test".to_string());
        let _request = LlmError::InvalidRequest("test".to_string());
        let _response = LlmError::InvalidResponse("test".to_string());
        let _timeout = LlmError::Timeout("test".to_string());
        let _provider = LlmError::ProviderError("test".to_string());
        let _config = LlmError::ConfigError("test".to_string());
        let _other = LlmError::Other("test".to_string());
    }

    #[test]
    fn test_into_graph_error() {
        let llm_err = LlmError::ModelNotFound("gpt-5".to_string());
        let graph_err: langgraph_core::error::GraphError = llm_err.into();

        match graph_err {
            langgraph_core::error::GraphError::Validation(msg) => {
                assert_eq!(msg, "Model not found: gpt-5");
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_error_debug_format() {
        let err = LlmError::Timeout("30s".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Timeout"));
        assert!(debug_str.contains("30s"));
    }

    #[test]
    fn test_multiple_error_conversions() {
        // Test chain: serde_json -> LlmError -> GraphError
        let json_err = serde_json::from_str::<serde_json::Value>("{bad}").unwrap_err();
        let llm_err: LlmError = json_err.into();
        let graph_err: langgraph_core::error::GraphError = llm_err.into();

        match graph_err {
            langgraph_core::error::GraphError::Validation(msg) => {
                assert!(msg.contains("Serialization error"));
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_retryable_classification_comprehensive() {
        let retryable_errors = vec![
            LlmError::ServiceUnavailable("test".to_string()),
            LlmError::Timeout("test".to_string()),
            LlmError::RateLimitExceeded("test".to_string()),
        ];

        for err in retryable_errors {
            assert!(err.is_retryable(), "Expected {:?} to be retryable", err);
        }

        let non_retryable_errors = vec![
            LlmError::AuthenticationError("test".to_string()),
            LlmError::ApiKeyNotFound("test".to_string()),
            LlmError::ModelNotFound("test".to_string()),
            LlmError::InvalidRequest("test".to_string()),
            LlmError::InvalidResponse("test".to_string()),
            LlmError::SerializationError("test".to_string()),
            LlmError::ConfigError("test".to_string()),
            LlmError::ProviderError("test".to_string()),
            LlmError::Other("test".to_string()),
        ];

        for err in non_retryable_errors {
            assert!(!err.is_retryable(), "Expected {:?} to not be retryable", err);
        }
    }

    #[test]
    fn test_auth_error_classification_comprehensive() {
        let auth_errors = vec![
            LlmError::AuthenticationError("test".to_string()),
            LlmError::ApiKeyNotFound("test".to_string()),
        ];

        for err in auth_errors {
            assert!(err.is_auth_error(), "Expected {:?} to be auth error", err);
        }

        let non_auth_errors = vec![
            LlmError::ServiceUnavailable("test".to_string()),
            LlmError::Timeout("test".to_string()),
            LlmError::RateLimitExceeded("test".to_string()),
            LlmError::ModelNotFound("test".to_string()),
            LlmError::InvalidRequest("test".to_string()),
            LlmError::InvalidResponse("test".to_string()),
            LlmError::SerializationError("test".to_string()),
            LlmError::ConfigError("test".to_string()),
            LlmError::ProviderError("test".to_string()),
            LlmError::Other("test".to_string()),
        ];

        for err in non_auth_errors {
            assert!(!err.is_auth_error(), "Expected {:?} to not be auth error", err);
        }
    }

    #[test]
    fn test_result_type_usage() {
        fn returns_ok() -> Result<String> {
            Ok("success".to_string())
        }

        fn returns_err() -> Result<String> {
            Err(LlmError::Timeout("timeout".to_string()))
        }

        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_error_context_preservation() {
        let original_msg = "connection refused on port 11434";
        let err = LlmError::ServiceUnavailable(original_msg.to_string());
        let err_string = err.to_string();

        assert!(err_string.contains(original_msg));
        assert!(err_string.contains("Service unavailable"));
    }

    #[test]
    fn test_empty_error_messages() {
        // Test that errors work with empty messages
        let err = LlmError::Other("".to_string());
        assert_eq!(err.to_string(), "");

        let err2 = LlmError::ProviderError("".to_string());
        assert_eq!(err2.to_string(), "Provider error: ");
    }
}

