//! aco gRPC client for communicating with orchestrator server
//!
//! Provides client-side infrastructure for connecting to the orchestrator,
//! managing authentication tokens, and handling gRPC channels.

use crate::error::{AcoError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Client configuration for connecting to orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Orchestrator server URL (e.g., "http://localhost:50051")
    pub server_url: String,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Path to store authentication tokens
    pub token_path: PathBuf,

    /// Enable TLS (true) or use insecure connection (false)
    pub use_tls: bool,

    /// Certificate path for TLS (optional)
    pub cert_path: Option<PathBuf>,

    /// Retry configuration
    pub retry_config: RetryConfig,
}

/// Retry configuration for connection attempts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,

    /// Initial backoff duration in milliseconds
    pub initial_backoff_ms: u64,

    /// Maximum backoff duration in milliseconds
    pub max_backoff_ms: u64,

    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let token_path = home_dir.join(".aco").join("token");

        Self {
            server_url: "http://localhost:50051".to_string(),
            timeout_seconds: 30,
            token_path,
            use_tls: false,
            cert_path: None,
            retry_config: RetryConfig::default(),
        }
    }
}

impl ClientConfig {
    /// Create a new client configuration
    pub fn new(server_url: String) -> Self {
        let mut config = Self::default();
        config.server_url = server_url;
        config
    }

    /// Set TLS configuration
    pub fn with_tls(mut self, cert_path: Option<PathBuf>) -> Self {
        self.use_tls = true;
        self.cert_path = cert_path;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set token path
    pub fn with_token_path(mut self, path: PathBuf) -> Self {
        self.token_path = path;
        self
    }

    /// Load configuration from file
    pub async fn from_file(path: &Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| AcoError::Config(format!("Failed to read config: {}", e)))?;

        let config: ClientConfig = toml::from_str(&content)
            .map_err(|e| AcoError::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    /// Save configuration to file
    pub async fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| AcoError::Config(format!("Failed to serialize config: {}", e)))?;

        tokio::fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))
            .await
            .map_err(|e| AcoError::Config(format!("Failed to create config dir: {}", e)))?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| AcoError::Config(format!("Failed to write config: {}", e)))?;

        Ok(())
    }
}

/// Authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// JWT access token
    pub access_token: String,

    /// Token expiration time
    pub expires_at: DateTime<Utc>,

    /// Username associated with token
    pub username: Option<String>,
}

impl AuthToken {
    /// Create a new auth token
    pub fn new(access_token: String, expires_in_seconds: u64, username: Option<String>) -> Self {
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_in_seconds as i64);

        Self {
            access_token,
            expires_at,
            username,
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Get time until expiration in seconds
    pub fn time_to_expiration(&self) -> i64 {
        (self.expires_at - Utc::now()).num_seconds()
    }
}

/// Token manager for loading and storing authentication tokens
pub struct TokenManager {
    token_path: PathBuf,
    current_token: Arc<RwLock<Option<AuthToken>>>,
}

impl TokenManager {
    /// Create a new token manager
    pub fn new(token_path: PathBuf) -> Self {
        Self {
            token_path,
            current_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Load token from file
    pub async fn load_token(&self) -> Result<Option<AuthToken>> {
        if !self.token_path.exists() {
            debug!("Token file not found at {:?}", self.token_path);
            return Ok(None);
        }

        match tokio::fs::read_to_string(&self.token_path).await {
            Ok(content) => {
                let token: AuthToken = serde_json::from_str(&content)
                    .map_err(|e| AcoError::Auth(format!("Failed to parse token: {}", e)))?;

                if token.is_expired() {
                    warn!("Loaded token is expired");
                    return Ok(None);
                }

                let mut current = self.current_token.write().await;
                *current = Some(token.clone());

                Ok(Some(token))
            }
            Err(e) => {
                debug!("Failed to read token file: {}", e);
                Ok(None)
            }
        }
    }

    /// Save token to file
    pub async fn save_token(&self, token: AuthToken) -> Result<()> {
        let token_dir = self.token_path.parent().unwrap_or_else(|| Path::new("."));

        tokio::fs::create_dir_all(token_dir)
            .await
            .map_err(|e| AcoError::Config(format!("Failed to create token dir: {}", e)))?;

        let content = serde_json::to_string(&token)
            .map_err(|e| AcoError::Auth(format!("Failed to serialize token: {}", e)))?;

        tokio::fs::write(&self.token_path, content)
            .await
            .map_err(|e| AcoError::Auth(format!("Failed to write token: {}", e)))?;

        let mut current = self.current_token.write().await;
        *current = Some(token);

        info!("Token saved to {:?}", self.token_path);
        Ok(())
    }

    /// Get current token
    pub async fn get_token(&self) -> Option<AuthToken> {
        let token = self.current_token.read().await;
        token.clone()
    }

    /// Clear current token
    pub async fn clear_token(&self) -> Result<()> {
        let mut token = self.current_token.write().await;
        *token = None;

        if self.token_path.exists() {
            tokio::fs::remove_file(&self.token_path)
                .await
                .map_err(|e| AcoError::Auth(format!("Failed to delete token: {}", e)))?;
        }

        info!("Token cleared");
        Ok(())
    }

    /// Set token directly
    pub async fn set_token(&self, token: AuthToken) {
        let mut current = self.current_token.write().await;
        *current = Some(token);
    }
}

/// aco gRPC client for orchestrator communication
pub struct AcoClient {
    config: ClientConfig,
    token_manager: Arc<TokenManager>,
    channel: Option<Arc<tonic::transport::Channel>>,
}

impl AcoClient {
    /// Create a new aco client
    pub fn new(config: ClientConfig) -> Self {
        let token_manager = TokenManager::new(config.token_path.clone());

        Self {
            config,
            token_manager: Arc::new(token_manager),
            channel: None,
        }
    }

    /// Create a client from server URL
    pub fn from_url(server_url: String) -> Self {
        Self::new(ClientConfig::new(server_url))
    }

    /// Connect to orchestrator server with retry logic
    pub async fn connect(&mut self) -> Result<()> {
        let mut retries = 0;
        let mut backoff = self.config.retry_config.initial_backoff_ms;

        loop {
            match self.create_channel().await {
                Ok(channel) => {
                    self.channel = Some(Arc::new(channel));
                    info!("Successfully connected to orchestrator at {}", self.config.server_url);
                    return Ok(());
                }
                Err(e) => {
                    retries += 1;
                    if retries > self.config.retry_config.max_retries {
                        return Err(AcoError::Connection(format!(
                            "Failed to connect after {} retries: {}",
                            retries, e
                        )));
                    }

                    warn!(
                        "Connection attempt {} failed: {}. Retrying in {}ms",
                        retries, e, backoff
                    );

                    tokio::time::sleep(Duration::from_millis(backoff)).await;

                    backoff = std::cmp::min(
                        (backoff as f64 * self.config.retry_config.backoff_multiplier) as u64,
                        self.config.retry_config.max_backoff_ms,
                    );
                }
            }
        }
    }

    /// Create a tonic channel
    async fn create_channel(&self) -> Result<tonic::transport::Channel> {
        let uri = self.config.server_url.parse()
            .map_err(|e| AcoError::Connection(format!("Invalid server URL: {}", e)))?;

        let channel = if self.config.use_tls {
            let tls_config = if let Some(cert_path) = &self.config.cert_path {
                let cert = tokio::fs::read(cert_path)
                    .await
                    .map_err(|e| AcoError::Connection(format!("Failed to read cert: {}", e)))?;

                let pem = tonic::transport::Certificate::from_pem(cert);
                tonic::transport::ClientTlsConfig::new().ca_certificate(pem)
            } else {
                tonic::transport::ClientTlsConfig::new()
            };

            tonic::transport::Channel::builder(uri)
                .tls_config(tls_config)
                .map_err(|e| AcoError::Connection(format!("TLS config error: {}", e)))?
                .timeout(Duration::from_secs(self.config.timeout_seconds))
                .connect()
                .await
                .map_err(|e| AcoError::Connection(format!("Connection failed: {}", e)))?
        } else {
            tonic::transport::Channel::builder(uri)
                .timeout(Duration::from_secs(self.config.timeout_seconds))
                .connect()
                .await
                .map_err(|e| AcoError::Connection(format!("Connection failed: {}", e)))?
        };

        Ok(channel)
    }

    /// Get the gRPC channel
    pub fn channel(&self) -> Result<Arc<tonic::transport::Channel>> {
        self.channel
            .clone()
            .ok_or_else(|| AcoError::Connection("Not connected".to_string()))
    }

    /// Load token from disk
    pub async fn load_token(&self) -> Result<()> {
        self.token_manager.load_token().await?;
        Ok(())
    }

    /// Set authentication token
    pub async fn set_token(&self, token: AuthToken) {
        self.token_manager.set_token(token).await;
    }

    /// Get current authentication token
    pub async fn get_token(&self) -> Option<AuthToken> {
        self.token_manager.get_token().await
    }

    /// Clear authentication token
    pub async fn clear_token(&self) -> Result<()> {
        self.token_manager.clear_token().await
    }

    /// Check if authenticated
    pub async fn is_authenticated(&self) -> bool {
        if let Some(token) = self.get_token().await {
            !token.is_expired()
        } else {
            false
        }
    }

    /// Get server configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Check connection status
    pub fn is_connected(&self) -> bool {
        self.channel.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.server_url, "http://localhost:50051");
        assert!(!config.use_tls);
    }

    #[test]
    fn test_client_config_new() {
        let config = ClientConfig::new("http://example.com:50051".to_string());
        assert_eq!(config.server_url, "http://example.com:50051");
    }

    #[test]
    fn test_client_config_with_tls() {
        let config = ClientConfig::default().with_tls(None);
        assert!(config.use_tls);
    }

    #[test]
    fn test_client_config_with_timeout() {
        let config = ClientConfig::default().with_timeout(60);
        assert_eq!(config.timeout_seconds, 60);
    }

    #[test]
    fn test_auth_token_creation() {
        let token = AuthToken::new("test-token".to_string(), 3600, Some("testuser".to_string()));
        assert_eq!(token.access_token, "test-token");
        assert_eq!(token.username, Some("testuser".to_string()));
        assert!(!token.is_expired());
    }

    #[test]
    fn test_auth_token_expired() {
        let token = AuthToken::new("test-token".to_string(), 0, None);
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(token.is_expired());
    }

    #[test]
    fn test_auth_token_time_to_expiration() {
        let token = AuthToken::new("test-token".to_string(), 100, None);
        let time_left = token.time_to_expiration();
        assert!(time_left > 0 && time_left <= 100);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_ms, 100);
    }

    #[tokio::test]
    async fn test_token_manager_creation() {
        let temp_dir = std::env::temp_dir().join("aco_test_token");
        let manager = TokenManager::new(temp_dir);
        assert!(manager.get_token().await.is_none());
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = ClientConfig::default();
        let client = AcoClient::new(config);
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_client_from_url() {
        let client = AcoClient::from_url("http://localhost:50051".to_string());
        assert_eq!(client.config().server_url, "http://localhost:50051");
    }

    #[tokio::test]
    async fn test_client_authentication_status() {
        let config = ClientConfig::default();
        let client = AcoClient::new(config);
        assert!(!client.is_authenticated().await);
    }
}
