//! Authentication handling for aco client
//!
//! Supports multiple authentication modes:
//! - none: No authentication
//! - secret:<key>: API secret key
//! - <user>:<pass>: Username and password (obtains JWT)
//! - token:<jwt>: Pre-obtained JWT token

use crate::error::{AcoError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::{debug, info};

/// Authentication connection modes
#[derive(Debug, Clone)]
pub enum ConnectAuth {
    /// No authentication required
    None,

    /// API secret key authentication
    Secret(String),

    /// Username and password (will be exchanged for JWT)
    UserPass { username: String, password: String },

    /// Pre-obtained JWT token
    Token(String),
}

impl fmt::Display for ConnectAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Secret(_) => write!(f, "secret"),
            Self::UserPass { username, .. } => write!(f, "userpass ({})", username),
            Self::Token(_) => write!(f, "jwt token"),
        }
    }
}

impl ConnectAuth {
    /// Parse authentication mode from connection string
    ///
    /// Formats:
    /// - "none" -> ConnectAuth::None
    /// - "secret:my-key" -> ConnectAuth::Secret("my-key")
    /// - "user:pass" -> ConnectAuth::UserPass { username: "user", password: "pass" }
    /// - "token:jwt-content" -> ConnectAuth::Token("jwt-content")
    pub fn from_connect_string(s: &str) -> Result<Self> {
        if s == "none" {
            return Ok(ConnectAuth::None);
        }

        if let Some(secret) = s.strip_prefix("secret:") {
            return Ok(ConnectAuth::Secret(secret.to_string()));
        }

        if let Some(token) = s.strip_prefix("token:") {
            return Ok(ConnectAuth::Token(token.to_string()));
        }

        // Try to parse as user:pass
        if let Some(colon_pos) = s.find(':') {
            let username = &s[..colon_pos];
            let password = &s[colon_pos + 1..];

            if !username.is_empty() && !password.is_empty() {
                debug!("Parsed userpass authentication for user: {}", username);
                return Ok(ConnectAuth::UserPass {
                    username: username.to_string(),
                    password: password.to_string(),
                });
            }
        }

        Err(AcoError::Auth(format!(
            "Invalid connection string: '{}'. Expected format: none|secret:<key>|<user>:<pass>|token:<jwt>",
            s
        )))
    }

    /// Attach authentication to a gRPC request
    pub fn attach_to_request<T>(&self, mut req: tonic::Request<T>) -> Result<tonic::Request<T>> {
        match self {
            ConnectAuth::None => {
                // No auth needed
                Ok(req)
            }

            ConnectAuth::Secret(key) => {
                // Add x-api-secret header
                let metadata = req.metadata_mut();
                let key_value = tonic::metadata::MetadataValue::try_from(key)
                    .map_err(|e| AcoError::Auth(format!("Invalid secret header: {}", e)))?;
                metadata.insert("x-api-secret", key_value);
                Ok(req)
            }

            ConnectAuth::UserPass { .. } => {
                // This shouldn't be used directly - should be converted to Token
                Err(AcoError::Auth(
                    "UserPass auth must be exchanged for token before use".to_string(),
                ))
            }

            ConnectAuth::Token(token) => {
                // Add Authorization header with Bearer token
                let auth_header = format!("Bearer {}", token);
                let metadata = req.metadata_mut();
                let auth_value = tonic::metadata::MetadataValue::try_from(&auth_header)
                    .map_err(|e| AcoError::Auth(format!("Invalid token header: {}", e)))?;
                metadata.insert("authorization", auth_value);
                Ok(req)
            }
        }
    }

    /// Get a human-readable description of the auth mode
    pub fn mode_description(&self) -> String {
        format!("Auth mode: {}", self)
    }

    /// Check if this auth mode requires token exchange (UserPass -> Token)
    pub fn requires_token_exchange(&self) -> bool {
        matches!(self, ConnectAuth::UserPass { .. })
    }

    /// Convert UserPass to Token
    pub fn with_token(self, token: String) -> Self {
        ConnectAuth::Token(token)
    }
}

/// Token storage for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedToken {
    /// The JWT token
    pub token: String,

    /// When the token was cached
    pub cached_at: chrono::DateTime<chrono::Utc>,

    /// Optional expiration time
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl CachedToken {
    /// Create a new cached token
    pub fn new(token: String) -> Self {
        Self {
            token,
            cached_at: chrono::Utc::now(),
            expires_at: None,
        }
    }

    /// Create a cached token with expiration
    pub fn with_expiration(token: String, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            token,
            cached_at: chrono::Utc::now(),
            expires_at: Some(expires_at),
        }
    }

    /// Check if token is still valid
    pub fn is_valid(&self) -> bool {
        match self.expires_at {
            Some(exp) => chrono::Utc::now() < exp,
            None => true, // No expiration set, assume valid
        }
    }

    /// Check if token has been cached for longer than a threshold
    pub fn is_stale(&self, hours: i64) -> bool {
        let now = chrono::Utc::now();
        let threshold = self.cached_at + chrono::Duration::hours(hours);
        now > threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_connect_none() {
        let auth = ConnectAuth::from_connect_string("none").unwrap();
        assert!(matches!(auth, ConnectAuth::None));
    }

    #[test]
    fn test_parse_connect_secret() {
        let auth = ConnectAuth::from_connect_string("secret:my-secret-key").unwrap();
        match auth {
            ConnectAuth::Secret(key) => {
                assert_eq!(key, "my-secret-key");
            }
            _ => panic!("Expected Secret variant"),
        }
    }

    #[test]
    fn test_parse_connect_userpass() {
        let auth = ConnectAuth::from_connect_string("alice:password123").unwrap();
        match auth {
            ConnectAuth::UserPass { username, password } => {
                assert_eq!(username, "alice");
                assert_eq!(password, "password123");
            }
            _ => panic!("Expected UserPass variant"),
        }
    }

    #[test]
    fn test_parse_connect_token() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let auth = ConnectAuth::from_connect_string(&format!("token:{}", token)).unwrap();
        match auth {
            ConnectAuth::Token(t) => {
                assert_eq!(t, token);
            }
            _ => panic!("Expected Token variant"),
        }
    }

    #[test]
    fn test_parse_connect_invalid_no_password() {
        let result = ConnectAuth::from_connect_string("username:");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_connect_invalid_no_username() {
        let result = ConnectAuth::from_connect_string(":password");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_is_valid() {
        let token = CachedToken::new("test-token".to_string());
        assert!(token.is_valid());
    }

    #[test]
    fn test_token_is_expired() {
        let expiration = chrono::Utc::now() - chrono::Duration::hours(1);
        let token = CachedToken::with_expiration("test-token".to_string(), expiration);
        assert!(!token.is_valid());
    }

    #[test]
    fn test_token_is_not_stale() {
        let token = CachedToken::new("test-token".to_string());
        assert!(!token.is_stale(24)); // Not stale within 24 hours
    }

    #[test]
    fn test_auth_display() {
        assert_eq!(ConnectAuth::None.to_string(), "none");
        assert_eq!(
            ConnectAuth::Secret("key".to_string()).to_string(),
            "secret"
        );
        assert_eq!(
            ConnectAuth::Token("jwt".to_string()).to_string(),
            "jwt token"
        );
    }
}
