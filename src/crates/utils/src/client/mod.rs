//! HTTP client utilities and helpers.
//!
//! This module provides utilities for building HTTP clients including:
//! - Client configuration builders
//! - Request builders with retry logic
//! - Authentication helpers
//! - Request/response helpers
//!
//! # Example
//!
//! ```rust,ignore
//! use utils::client::{ClientConfig, HttpClient};
//!
//! let config = ClientConfig::new()
//!     .with_timeout(Duration::from_secs(30))
//!     .with_max_retries(3);
//!
//! let client = HttpClient::new(config);
//! ```

use crate::error::{Result, UtilsError};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for HTTP client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Request timeout duration.
    #[serde(default = "default_timeout")]
    pub timeout: Duration,

    /// Maximum number of retries for failed requests.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Initial delay between retries.
    #[serde(default = "default_retry_delay")]
    pub retry_delay: Duration,

    /// Backoff multiplier for retry delays.
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f32,

    /// User agent string.
    pub user_agent: Option<String>,

    /// Default headers to include in all requests.
    #[serde(skip)]
    pub default_headers: Vec<(String, String)>,
}

impl ClientConfig {
    /// Create a new client configuration with defaults.
    pub fn new() -> Self {
        Self {
            timeout: default_timeout(),
            max_retries: default_max_retries(),
            retry_delay: default_retry_delay(),
            backoff_multiplier: default_backoff_multiplier(),
            user_agent: None,
            default_headers: Vec::new(),
        }
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the retry delay.
    pub fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Set the backoff multiplier.
    pub fn with_backoff_multiplier(mut self, multiplier: f32) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Set the user agent.
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Add a default header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.push((key.into(), value.into()));
        self
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new()
    }
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay() -> Duration {
    Duration::from_secs(1)
}

fn default_backoff_multiplier() -> f32 {
    2.0
}

/// HTTP client with retry and configuration support.
pub struct HttpClient {
    config: ClientConfig,
    client: Client,
}

impl HttpClient {
    /// Create a new HTTP client with the given configuration.
    pub fn new(config: ClientConfig) -> Result<Self> {
        let mut builder = Client::builder().timeout(config.timeout);

        if let Some(user_agent) = &config.user_agent {
            builder = builder.user_agent(user_agent);
        }

        let client = builder
            .build()
            .map_err(|e| UtilsError::ClientError(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// Get a reference to the underlying reqwest client.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Send a GET request.
    pub async fn get(&self, url: &str) -> Result<Response> {
        self.request(Method::GET, url).await
    }

    /// Send a POST request with JSON body.
    pub async fn post_json<T: Serialize>(&self, url: &str, body: &T) -> Result<Response> {
        let mut req = self.client.post(url);

        // Add default headers
        for (key, value) in &self.config.default_headers {
            req = req.header(key, value);
        }

        req = req.json(body);

        self.send_with_retry(req).await
    }

    /// Send a request with the given method.
    pub async fn request(&self, method: Method, url: &str) -> Result<Response> {
        let mut req = self.client.request(method, url);

        // Add default headers
        for (key, value) in &self.config.default_headers {
            req = req.header(key, value);
        }

        self.send_with_retry(req).await
    }

    /// Send a request builder with retry logic.
    async fn send_with_retry(&self, req: reqwest::RequestBuilder) -> Result<Response> {
        let mut attempts = 0;
        let mut delay = self.config.retry_delay;

        loop {
            let request = req
                .try_clone()
                .ok_or_else(|| UtilsError::ClientError("Failed to clone request".to_string()))?;

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() || attempts >= self.config.max_retries {
                        return Ok(response);
                    }

                    // Check if error is retryable (5xx errors)
                    if !response.status().is_server_error() {
                        return Ok(response);
                    }
                }
                Err(e) => {
                    if attempts >= self.config.max_retries {
                        return Err(UtilsError::HttpError(e));
                    }

                    // Only retry on network errors or timeouts
                    if !e.is_timeout() && !e.is_connect() {
                        return Err(UtilsError::HttpError(e));
                    }
                }
            }

            attempts += 1;
            tokio::time::sleep(delay).await;
            delay = Duration::from_secs_f32(delay.as_secs_f32() * self.config.backoff_multiplier);
        }
    }
}

/// Helper for building authenticated requests.
pub struct AuthHelper;

impl AuthHelper {
    /// Create a bearer token authorization header value.
    pub fn bearer_token(token: &str) -> String {
        format!("Bearer {}", token)
    }

    /// Create a basic auth authorization header value.
    pub fn basic_auth(username: &str, password: &str) -> String {
        use base64::Engine;
        let credentials = format!("{}:{}", username, password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    /// Create an API key header value.
    pub fn api_key(key: &str) -> String {
        key.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    // ========================================================================
    // Phase 7.3: Utils HTTP Client Retry Tests
    // ========================================================================

    // ------------------------------------------------------------------------
    // Client Configuration Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_client_config() {
        let config = ClientConfig::new()
            .with_timeout(Duration::from_secs(60))
            .with_max_retries(5)
            .with_user_agent("test-agent")
            .with_header("X-Custom", "value");

        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.user_agent, Some("test-agent".to_string()));
        assert_eq!(config.default_headers.len(), 1);
    }

    #[test]
    fn test_client_config_default_values() {
        let config = ClientConfig::default();

        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay, Duration::from_secs(1));
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!(config.user_agent.is_none());
        assert_eq!(config.default_headers.len(), 0);
    }

    #[test]
    fn test_client_config_builder_pattern() {
        let config = ClientConfig::new()
            .with_timeout(Duration::from_secs(10))
            .with_max_retries(2)
            .with_retry_delay(Duration::from_millis(500))
            .with_backoff_multiplier(1.5)
            .with_user_agent("custom-agent/1.0");

        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.retry_delay, Duration::from_millis(500));
        assert_eq!(config.backoff_multiplier, 1.5);
        assert_eq!(config.user_agent, Some("custom-agent/1.0".to_string()));
    }

    #[test]
    fn test_client_config_multiple_headers() {
        let config = ClientConfig::new()
            .with_header("Authorization", "Bearer token")
            .with_header("X-Request-ID", "req-123")
            .with_header("X-Custom-Header", "value");

        assert_eq!(config.default_headers.len(), 3);
        assert!(config.default_headers.contains(&("Authorization".to_string(), "Bearer token".to_string())));
        assert!(config.default_headers.contains(&("X-Request-ID".to_string(), "req-123".to_string())));
    }

    #[test]
    fn test_client_config_zero_retries() {
        let config = ClientConfig::new().with_max_retries(0);
        assert_eq!(config.max_retries, 0);
    }

    #[test]
    fn test_client_config_very_long_timeout() {
        let config = ClientConfig::new().with_timeout(Duration::from_secs(3600));
        assert_eq!(config.timeout, Duration::from_secs(3600));
    }

    #[test]
    fn test_client_config_custom_backoff() {
        let config = ClientConfig::new().with_backoff_multiplier(3.0);
        assert_eq!(config.backoff_multiplier, 3.0);
    }

    // ------------------------------------------------------------------------
    // Backoff Calculation Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_backoff_calculation_exponential() {
        let config = ClientConfig::new()
            .with_retry_delay(Duration::from_secs(1))
            .with_backoff_multiplier(2.0);

        // Initial delay: 1s
        let delay1 = config.retry_delay;
        assert_eq!(delay1, Duration::from_secs(1));

        // After 1st retry: 1s * 2.0 = 2s
        let delay2 = Duration::from_secs_f32(delay1.as_secs_f32() * config.backoff_multiplier);
        assert_eq!(delay2, Duration::from_secs(2));

        // After 2nd retry: 2s * 2.0 = 4s
        let delay3 = Duration::from_secs_f32(delay2.as_secs_f32() * config.backoff_multiplier);
        assert_eq!(delay3, Duration::from_secs(4));
    }

    #[test]
    fn test_backoff_calculation_linear() {
        let config = ClientConfig::new()
            .with_retry_delay(Duration::from_secs(1))
            .with_backoff_multiplier(1.0);

        let delay1 = config.retry_delay;
        let delay2 = Duration::from_secs_f32(delay1.as_secs_f32() * config.backoff_multiplier);

        // Linear backoff means delay stays the same
        assert_eq!(delay1, delay2);
    }

    #[test]
    fn test_backoff_calculation_custom_multiplier() {
        let config = ClientConfig::new()
            .with_retry_delay(Duration::from_millis(100))
            .with_backoff_multiplier(1.5);

        let delay1 = config.retry_delay;
        assert_eq!(delay1, Duration::from_millis(100));

        // After 1st retry: 100ms * 1.5 = 150ms
        let delay2 = Duration::from_secs_f32(delay1.as_secs_f32() * config.backoff_multiplier);
        assert_eq!(delay2.as_millis(), 150);

        // After 2nd retry: 150ms * 1.5 = 225ms
        let delay3 = Duration::from_secs_f32(delay2.as_secs_f32() * config.backoff_multiplier);
        assert_eq!(delay3.as_millis(), 225);
    }

    #[test]
    fn test_backoff_with_zero_delay() {
        let config = ClientConfig::new()
            .with_retry_delay(Duration::from_secs(0))
            .with_backoff_multiplier(2.0);

        let delay = config.retry_delay;
        assert_eq!(delay, Duration::from_secs(0));

        // Even with backoff, zero stays zero
        let next_delay = Duration::from_secs_f32(delay.as_secs_f32() * config.backoff_multiplier);
        assert_eq!(next_delay, Duration::from_secs(0));
    }

    // ------------------------------------------------------------------------
    // HTTP Client Creation Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_http_client_creation_success() {
        let config = ClientConfig::new();
        let client = HttpClient::new(config);

        assert!(client.is_ok());
    }

    #[test]
    fn test_http_client_creation_with_user_agent() {
        let config = ClientConfig::new().with_user_agent("test-agent/1.0");
        let client = HttpClient::new(config);

        assert!(client.is_ok());
        // User agent is set in the underlying reqwest client
    }

    #[test]
    fn test_http_client_creation_with_timeout() {
        let config = ClientConfig::new().with_timeout(Duration::from_secs(5));
        let client = HttpClient::new(config);

        assert!(client.is_ok());
        // Timeout is configured in the underlying reqwest client
    }

    #[test]
    fn test_http_client_with_headers() {
        let config = ClientConfig::new()
            .with_header("X-API-Key", "key-123")
            .with_header("X-Client-Version", "1.0.0");

        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }

    // ------------------------------------------------------------------------
    // Retry Logic Tests (requires mock server - marked #[ignore])
    // ------------------------------------------------------------------------

    #[tokio::test]
    #[ignore] // Requires mock HTTP server
    async fn test_retry_on_timeout() {
        let config = ClientConfig::new()
            .with_max_retries(3)
            .with_retry_delay(Duration::from_millis(100))
            .with_timeout(Duration::from_millis(50));

        let client = HttpClient::new(config).unwrap();

        // Would require mock server that delays response > 50ms
        // Should retry up to 3 times before failing
        // let result = client.get("http://localhost:9999/slow").await;
        // assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires mock HTTP server
    async fn test_retry_on_5xx_error() {
        let config = ClientConfig::new()
            .with_max_retries(3)
            .with_retry_delay(Duration::from_millis(100));

        let client = HttpClient::new(config).unwrap();

        // Mock server would return 500 error
        // Should retry up to 3 times
        // let result = client.get("http://localhost:9999/error500").await;
        // Eventually returns 500 response or succeeds if server recovers
    }

    #[tokio::test]
    #[ignore] // Requires mock HTTP server
    async fn test_no_retry_on_4xx_error() {
        let config = ClientConfig::new()
            .with_max_retries(3)
            .with_retry_delay(Duration::from_millis(100));

        let client = HttpClient::new(config).unwrap();

        // Mock server returns 404
        // Should NOT retry (client errors are not retryable)
        // let result = client.get("http://localhost:9999/notfound").await;
        // Should immediately return 404 without retries
    }

    #[tokio::test]
    #[ignore] // Requires mock HTTP server
    async fn test_retry_on_connection_error() {
        let config = ClientConfig::new()
            .with_max_retries(3)
            .with_retry_delay(Duration::from_millis(100))
            .with_timeout(Duration::from_secs(1));

        let client = HttpClient::new(config).unwrap();

        // Connect to unreachable host
        // Should retry connection errors
        // let result = client.get("http://192.0.2.1:9999/test").await;
        // Should retry 3 times then fail
    }

    #[tokio::test]
    #[ignore] // Requires mock HTTP server
    async fn test_max_retries_respected() {
        let config = ClientConfig::new()
            .with_max_retries(2)
            .with_retry_delay(Duration::from_millis(50));

        let client = HttpClient::new(config).unwrap();

        // Mock server that always returns 503
        // Should retry exactly 2 times (3 attempts total)
        // let result = client.get("http://localhost:9999/always-fail").await;
        // Verify exactly 3 requests were made (1 initial + 2 retries)
    }

    #[tokio::test]
    #[ignore] // Requires mock HTTP server
    async fn test_exponential_backoff_timing() {
        use std::time::Instant;

        let config = ClientConfig::new()
            .with_max_retries(3)
            .with_retry_delay(Duration::from_millis(100))
            .with_backoff_multiplier(2.0);

        let client = HttpClient::new(config).unwrap();

        let start = Instant::now();

        // Mock server that always fails
        // Should wait: 100ms, 200ms, 400ms between retries
        // Total wait time should be ~700ms
        // let _result = client.get("http://localhost:9999/always-fail").await;

        let elapsed = start.elapsed();

        // Verify backoff timing (allow some tolerance)
        // assert!(elapsed >= Duration::from_millis(650));
        // assert!(elapsed <= Duration::from_millis(850));
    }

    #[tokio::test]
    #[ignore] // Requires mock HTTP server
    async fn test_success_on_retry() {
        let config = ClientConfig::new()
            .with_max_retries(3)
            .with_retry_delay(Duration::from_millis(100));

        let client = HttpClient::new(config).unwrap();

        // Mock server that fails twice, then succeeds
        // Should retry and eventually succeed
        // let result = client.get("http://localhost:9999/fail-twice").await;
        // assert!(result.is_ok());
    }

    // ------------------------------------------------------------------------
    // Connection Timeout Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    #[ignore] // Network-dependent test
    async fn test_connection_timeout_unreachable() {
        let config = ClientConfig::new()
            .with_timeout(Duration::from_secs(2))
            .with_max_retries(0);

        let client = HttpClient::new(config).unwrap();

        // Unreachable IP address (TEST-NET-1)
        let result = client.get("http://192.0.2.1:9999/test").await;

        // Should timeout or fail to connect
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_error_invalid_url() {
        let config = ClientConfig::new()
            .with_timeout(Duration::from_secs(1))
            .with_max_retries(0);

        let client = HttpClient::new(config).unwrap();

        // Invalid URL
        let result = client.get("not-a-valid-url").await;

        // Should fail with connection error
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires mock server
    async fn test_timeout_during_slow_response() {
        let config = ClientConfig::new()
            .with_timeout(Duration::from_millis(100))
            .with_max_retries(0);

        let client = HttpClient::new(config).unwrap();

        // Mock server that delays response for > 100ms
        // Should timeout
        // let result = client.get("http://localhost:9999/slow-response").await;
        // assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // HTTP Methods Tests (GET, POST)
    // ------------------------------------------------------------------------

    #[tokio::test]
    #[ignore] // Requires mock server
    async fn test_get_request_success() {
        let config = ClientConfig::new();
        let client = HttpClient::new(config).unwrap();

        // Mock server returns 200 OK
        // let result = client.get("http://localhost:9999/test").await;
        // assert!(result.is_ok());
        // let response = result.unwrap();
        // assert!(response.status().is_success());
    }

    #[tokio::test]
    #[ignore] // Requires mock server
    async fn test_get_request_with_headers() {
        let config = ClientConfig::new()
            .with_header("X-API-Key", "test-key")
            .with_header("X-Request-ID", "req-123");

        let client = HttpClient::new(config).unwrap();

        // Mock server would verify headers are present
        // let result = client.get("http://localhost:9999/headers").await;
        // assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires mock server
    async fn test_post_json_success() {
        use serde_json::json;

        let config = ClientConfig::new();
        let client = HttpClient::new(config).unwrap();

        let body = json!({
            "name": "test",
            "value": 123
        });

        // Mock server accepts JSON
        // let result = client.post_json("http://localhost:9999/api/data", &body).await;
        // assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires mock server
    async fn test_post_json_with_custom_headers() {
        use serde_json::json;

        let config = ClientConfig::new()
            .with_header("Authorization", "Bearer token123")
            .with_header("X-Custom", "value");

        let client = HttpClient::new(config).unwrap();

        let body = json!({"data": "test"});

        // Mock server would verify headers are included
        // let result = client.post_json("http://localhost:9999/api/post", &body).await;
        // assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires mock server
    async fn test_request_method_custom() {
        let config = ClientConfig::new();
        let client = HttpClient::new(config).unwrap();

        // Test custom HTTP methods (PUT, DELETE, PATCH, etc.)
        // let result = client.request(Method::PUT, "http://localhost:9999/resource").await;
        // let result = client.request(Method::DELETE, "http://localhost:9999/resource").await;
        // let result = client.request(Method::PATCH, "http://localhost:9999/resource").await;
    }

    // ------------------------------------------------------------------------
    // Authentication Helper Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_auth_helper() {
        let bearer = AuthHelper::bearer_token("my-token");
        assert_eq!(bearer, "Bearer my-token");

        let api_key = AuthHelper::api_key("key-123");
        assert_eq!(api_key, "key-123");
    }

    #[test]
    fn test_bearer_token_formatting() {
        let token = AuthHelper::bearer_token("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
        assert_eq!(token, "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
        assert!(token.starts_with("Bearer "));
    }

    #[test]
    fn test_basic_auth_encoding() {
        let basic = AuthHelper::basic_auth("user", "pass");
        assert!(basic.starts_with("Basic "));

        // Decode and verify
        let encoded_part = basic.strip_prefix("Basic ").unwrap();
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded_part).unwrap();
        let credentials = String::from_utf8(decoded).unwrap();
        assert_eq!(credentials, "user:pass");
    }

    #[test]
    fn test_basic_auth_special_characters() {
        let basic = AuthHelper::basic_auth("user@example.com", "p@ss:w0rd!");
        assert!(basic.starts_with("Basic "));

        // Verify special characters are properly encoded
        let encoded_part = basic.strip_prefix("Basic ").unwrap();
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded_part).unwrap();
        let credentials = String::from_utf8(decoded).unwrap();
        assert_eq!(credentials, "user@example.com:p@ss:w0rd!");
    }

    #[test]
    fn test_basic_auth_empty_password() {
        let basic = AuthHelper::basic_auth("user", "");
        let encoded_part = basic.strip_prefix("Basic ").unwrap();
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded_part).unwrap();
        let credentials = String::from_utf8(decoded).unwrap();
        assert_eq!(credentials, "user:");
    }

    #[test]
    fn test_api_key_passthrough() {
        let key = "sk-1234567890abcdef";
        let api_key = AuthHelper::api_key(key);
        assert_eq!(api_key, key);
    }

    // ------------------------------------------------------------------------
    // Edge Cases and Error Scenarios
    // ------------------------------------------------------------------------

    #[tokio::test]
    #[ignore] // Requires mock server
    async fn test_concurrent_requests() {
        let config = ClientConfig::new();
        let client = HttpClient::new(config).unwrap();
        let client = std::sync::Arc::new(client);

        // Spawn multiple concurrent requests
        let mut handles = vec![];
        for i in 0..10 {
            let client_clone = client.clone();
            let handle = tokio::spawn(async move {
                client_clone.get(&format!("http://localhost:9999/test/{}", i)).await
            });
            handles.push(handle);
        }

        // All requests should complete successfully
        for handle in handles {
            let result = handle.await;
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_config_serialization() {
        let config = ClientConfig::new()
            .with_timeout(Duration::from_secs(45))
            .with_max_retries(5)
            .with_retry_delay(Duration::from_millis(500))
            .with_backoff_multiplier(1.8);

        // Test that config can be serialized/deserialized
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ClientConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.timeout, config.timeout);
        assert_eq!(deserialized.max_retries, config.max_retries);
        assert_eq!(deserialized.retry_delay, config.retry_delay);
        assert_eq!(deserialized.backoff_multiplier, config.backoff_multiplier);
    }

    #[tokio::test]
    #[ignore] // Requires mock server
    async fn test_request_clone_for_retry() {
        // Verify that requests can be cloned for retry attempts
        let config = ClientConfig::new().with_max_retries(2);
        let client = HttpClient::new(config).unwrap();

        // Mock server that fails first request, succeeds second
        // Verifies that request can be cloned and retried
        // let result = client.get("http://localhost:9999/fail-once").await;
        // assert!(result.is_ok());
    }
}

