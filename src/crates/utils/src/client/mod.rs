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
    fn test_auth_helper() {
        let bearer = AuthHelper::bearer_token("my-token");
        assert_eq!(bearer, "Bearer my-token");

        let api_key = AuthHelper::api_key("key-123");
        assert_eq!(api_key, "key-123");
    }
}

