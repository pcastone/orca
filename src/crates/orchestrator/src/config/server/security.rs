//! Security configuration and middleware
//!
//! Handles different security modes: open, secret-key, and user-login.

use crate::config::{SecurityConfig, SecurityMode};
use crate::services::auth::JwtManager;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;
use tracing::{debug, warn};

/// Security middleware state
#[derive(Clone)]
pub struct SecurityState {
    config: Arc<SecurityConfig>,
    /// JWT manager for UserLogin mode (lazily initialized)
    jwt_manager: Option<Arc<JwtManager>>,
}

impl SecurityState {
    pub fn new(config: SecurityConfig) -> Self {
        // Initialize JWT manager if in UserLogin mode
        let jwt_manager = if config.mode == SecurityMode::UserLogin {
            match JwtManager::from_env() {
                Ok(manager) => Some(Arc::new(manager)),
                Err(e) => {
                    warn!("JWT manager initialization failed: {}. UserLogin will reject all requests.", e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            config: Arc::new(config),
            jwt_manager,
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

    /// Get the JWT manager for token validation
    pub fn jwt_manager(&self) -> Option<&Arc<JwtManager>> {
        self.jwt_manager.as_ref()
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
            // Require valid JWT token for user login mode
            if let Some(jwt_manager) = state.jwt_manager() {
                if let Some(auth_header) = request.headers().get("Authorization") {
                    if let Ok(auth_str) = auth_header.to_str() {
                        if let Some(token) = auth_str.strip_prefix("Bearer ") {
                            match jwt_manager.validate_token(token) {
                                Ok(username) => {
                                    debug!("Authenticated request for user: {}", username);
                                    return next.run(request).await;
                                }
                                Err(e) => {
                                    warn!("JWT validation failed: {}", e);
                                }
                            }
                        }
                    }
                }

                // Return 401 Unauthorized - missing or invalid JWT
                warn!("Unauthorized request - missing or invalid JWT token");
                Response::builder()
                    .status(401)
                    .body("Unauthorized: Valid JWT token required".into())
                    .unwrap()
            } else {
                // JWT manager not configured - reject all requests
                warn!("UserLogin mode enabled but JWT_SECRET not configured");
                Response::builder()
                    .status(500)
                    .body("Server configuration error: JWT_SECRET not set".into())
                    .unwrap()
            }
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

    #[test]
    fn test_security_state_open_mode() {
        let config = SecurityConfig {
            mode: SecurityMode::Open,
            secret_key: None,
        };
        let state = SecurityState::new(config);
        assert_eq!(state.mode(), SecurityMode::Open);
        assert!(state.jwt_manager().is_none());
    }

    #[test]
    fn test_security_state_user_login_without_jwt_secret() {
        // Ensure JWT_SECRET is not set
        std::env::remove_var("JWT_SECRET");

        let config = SecurityConfig {
            mode: SecurityMode::UserLogin,
            secret_key: None,
        };
        let state = SecurityState::new(config);
        assert_eq!(state.mode(), SecurityMode::UserLogin);
        // JWT manager should be None because JWT_SECRET is not set
        assert!(state.jwt_manager().is_none());
    }

    #[test]
    fn test_security_state_user_login_with_jwt_secret() {
        // Set JWT_SECRET for this test
        std::env::set_var("JWT_SECRET", "test-secret-that-is-at-least-32-characters-long");

        let config = SecurityConfig {
            mode: SecurityMode::UserLogin,
            secret_key: None,
        };
        let state = SecurityState::new(config);
        assert_eq!(state.mode(), SecurityMode::UserLogin);
        // JWT manager should be Some because JWT_SECRET is set
        assert!(state.jwt_manager().is_some());

        // Clean up
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_jwt_token_validation_in_security_state() {
        // Set JWT_SECRET for this test
        std::env::set_var("JWT_SECRET", "test-secret-that-is-at-least-32-characters-long");

        let config = SecurityConfig {
            mode: SecurityMode::UserLogin,
            secret_key: None,
        };
        let state = SecurityState::new(config);

        // Get the JWT manager and generate a token
        let jwt_manager = state.jwt_manager().expect("JWT manager should be present");
        let token = jwt_manager.generate_token("testuser").expect("Token generation should succeed");

        // Validate the token
        let username = jwt_manager.validate_token(&token).expect("Token validation should succeed");
        assert_eq!(username, "testuser");

        // Clean up
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_invalid_jwt_token_rejected() {
        // Set JWT_SECRET for this test
        std::env::set_var("JWT_SECRET", "test-secret-that-is-at-least-32-characters-long");

        let config = SecurityConfig {
            mode: SecurityMode::UserLogin,
            secret_key: None,
        };
        let state = SecurityState::new(config);

        let jwt_manager = state.jwt_manager().expect("JWT manager should be present");

        // Invalid token should be rejected
        let result = jwt_manager.validate_token("invalid.token.here");
        assert!(result.is_err());

        // Clean up
        std::env::remove_var("JWT_SECRET");
    }
}

