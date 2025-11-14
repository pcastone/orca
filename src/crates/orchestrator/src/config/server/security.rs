//! Security configuration and middleware
//!
//! Handles different security modes: open, secret-key, and user-login.

use crate::config::{SecurityConfig, SecurityMode};
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;
use tracing::warn;

/// Security middleware state
#[derive(Clone)]
pub struct SecurityState {
    config: Arc<SecurityConfig>,
}

impl SecurityState {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub fn mode(&self) -> SecurityMode {
        self.config.mode
    }

    pub fn get_secret_key(&self) -> Option<String> {
        std::env::var("SECRET_KEY")
            .ok()
            .or_else(|| self.config.secret_key.clone())
    }
}

/// Security middleware for Axum
pub async fn security_middleware(
    state: Arc<SecurityState>,
    request: Request,
    next: Next,
) -> Response {
    match state.mode() {
        SecurityMode::Open => {
            // No authentication required
            next.run(request).await
        }
        SecurityMode::SecretKey => {
            // Check for API key in headers
            if let Some(auth_header) = request.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if let Some(key) = auth_str.strip_prefix("Bearer ") {
                        if let Some(expected_key) = state.get_secret_key() {
                            if key == expected_key {
                                return next.run(request).await;
                            }
                        }
                    }
                }
            }
            
            // Return 401 Unauthorized
            warn!("Unauthorized request - missing or invalid API key");
            Response::builder()
                .status(401)
                .body("Unauthorized".into())
                .unwrap()
        }
        SecurityMode::UserLogin => {
            // TODO: Implement user login authentication
            // For now, allow all requests
            next.run(request).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_state() {
        let config = SecurityConfig {
            mode: SecurityMode::SecretKey,
            secret_key: Some("test-key".to_string()),
        };
        let state = SecurityState::new(config);
        assert_eq!(state.mode(), SecurityMode::SecretKey);
        assert_eq!(state.get_secret_key(), Some("test-key".to_string()));
    }
}

