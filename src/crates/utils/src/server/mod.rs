//! HTTP server utilities and helpers.
//!
//! This module provides utilities for building HTTP servers including:
//! - Server configuration builders
//! - Middleware helpers
//! - Route builders
//! - Request/response helpers
//!
//! # Example
//!
//! ```rust,ignore
//! use utils::server::{ServerConfig, ServerBuilder};
//!
//! let config = ServerConfig::new("127.0.0.1", 8080)
//!     .with_timeout(Duration::from_secs(30))
//!     .with_max_connections(1000);
//!
//! println!("Server config: {:?}", config);
//! ```

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

/// Configuration for HTTP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host address to bind to.
    pub host: String,

    /// Port to bind to.
    pub port: u16,

    /// Request timeout duration.
    #[serde(default = "default_timeout")]
    pub timeout: Duration,

    /// Maximum concurrent connections.
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Enable request logging.
    #[serde(default)]
    pub enable_logging: bool,

    /// Enable CORS.
    #[serde(default)]
    pub enable_cors: bool,
}

impl ServerConfig {
    /// Create a new server configuration.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            timeout: default_timeout(),
            max_connections: default_max_connections(),
            enable_logging: false,
            enable_cors: false,
        }
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of connections.
    pub fn with_max_connections(mut self, max_connections: usize) -> Self {
        self.max_connections = max_connections;
        self
    }

    /// Enable request logging.
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }

    /// Enable CORS support.
    pub fn with_cors(mut self, enable: bool) -> Self {
        self.enable_cors = enable;
        self
    }

    /// Get the socket address.
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| crate::error::UtilsError::ServerError(format!("Invalid socket address: {}", e)))
    }

    /// Load configuration from environment variables.
    pub fn from_env(prefix: &str) -> Result<Self> {
        let host = std::env::var(format!("{}HOST", prefix))
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var(format!("{}PORT", prefix))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8080);

        Ok(Self::new(host, port))
    }
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_max_connections() -> usize {
    1000
}

/// Server builder for fluent server construction.
pub struct ServerBuilder {
    config: ServerConfig,
}

impl ServerBuilder {
    /// Create a new server builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: ServerConfig::new("127.0.0.1", 8080),
        }
    }

    /// Set the host and port.
    pub fn bind(mut self, host: impl Into<String>, port: u16) -> Self {
        self.config.host = host.into();
        self.config.port = port;
        self
    }

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the maximum connections.
    pub fn max_connections(mut self, max_connections: usize) -> Self {
        self.config.max_connections = max_connections;
        self
    }

    /// Enable logging.
    pub fn with_logging(mut self) -> Self {
        self.config.enable_logging = true;
        self
    }

    /// Enable CORS.
    pub fn with_cors(mut self) -> Self {
        self.config.enable_cors = true;
        self
    }

    /// Build the server configuration.
    pub fn build(self) -> ServerConfig {
        self.config
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config() {
        let config = ServerConfig::new("localhost", 3000)
            .with_timeout(Duration::from_secs(60))
            .with_max_connections(500)
            .with_logging(true)
            .with_cors(true);

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 3000);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_connections, 500);
        assert!(config.enable_logging);
        assert!(config.enable_cors);
    }

    #[test]
    fn test_server_builder() {
        let config = ServerBuilder::new()
            .bind("0.0.0.0", 8080)
            .timeout(Duration::from_secs(30))
            .max_connections(1000)
            .with_logging()
            .with_cors()
            .build();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert!(config.enable_logging);
        assert!(config.enable_cors);
    }
}

