# Task 002: Implement gRPC Authentication and Authorization

## Objective
Implement authentication and authorization layer for gRPC communication between aco client and orchestrator server using JWT tokens and interceptors.

## Priority
**HIGH** - Security foundation for client-server communication

## Dependencies
- Task 001 (Protocol Buffer definitions)

## Implementation Details

### Files to Create/Modify

1. **`src/crates/orchestrator/proto/auth.proto`**
```protobuf
syntax = "proto3";
package orchestrator.auth;

service AuthService {
  rpc Login(LoginRequest) returns (LoginResponse);
  rpc RefreshToken(RefreshTokenRequest) returns (RefreshTokenResponse);
  rpc Logout(LogoutRequest) returns (LogoutResponse);
}

message LoginRequest {
  string username = 1;
  string password = 2;
}

message LoginResponse {
  string access_token = 1;
  string refresh_token = 2;
  int64 expires_in = 3;  // seconds
}

message RefreshTokenRequest {
  string refresh_token = 1;
}

message RefreshTokenResponse {
  string access_token = 1;
  int64 expires_in = 2;
}

message LogoutRequest {
  string token = 1;
}

message LogoutResponse {
  bool success = 1;
}
```

2. **`src/crates/orchestrator/src/auth/jwt.rs`**
```rust
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // subject (user id)
    pub exp: usize,   // expiration time
    pub iat: usize,   // issued at
    pub roles: Vec<String>,
}

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    expiration_seconds: u64,
}

impl JwtManager {
    pub fn new(secret: &str, expiration_seconds: u64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            expiration_seconds,
        }
    }

    pub fn from_env() -> Result<Self> {
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "dev-secret-change-in-production".to_string());
        let expiration = std::env::var("JWT_EXPIRATION_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()?;
        Ok(Self::new(&secret, expiration))
    }

    pub fn generate_token(&self, user_id: &str, roles: Vec<String>) -> Result<String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize;
        let claims = Claims {
            sub: user_id.to_string(),
            exp: now + self.expiration_seconds as usize,
            iat: now,
            roles,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(Into::into)
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::default(),
        )?;
        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_token() {
        let manager = JwtManager::new("test-secret", 3600);
        let token = manager.generate_token("user123", vec!["admin".to_string()])
            .expect("Failed to generate token");

        let claims = manager.validate_token(&token)
            .expect("Failed to validate token");

        assert_eq!(claims.sub, "user123");
        assert!(claims.roles.contains(&"admin".to_string()));
    }

    #[test]
    fn test_expired_token() {
        let manager = JwtManager::new("test-secret", 0);  // Expires immediately
        let token = manager.generate_token("user123", vec![])
            .expect("Failed to generate token");

        std::thread::sleep(std::time::Duration::from_secs(1));

        let result = manager.validate_token(&token);
        assert!(result.is_err());  // Should fail due to expiration
    }

    #[test]
    fn test_invalid_signature() {
        let manager1 = JwtManager::new("secret1", 3600);
        let manager2 = JwtManager::new("secret2", 3600);

        let token = manager1.generate_token("user123", vec![])
            .expect("Failed to generate token");

        let result = manager2.validate_token(&token);
        assert!(result.is_err());  // Should fail due to different secret
    }
}
```

3. **`src/crates/orchestrator/src/auth/interceptor.rs`**
```rust
use tonic::{Request, Status};
use crate::auth::jwt::{JwtManager, Claims};

pub struct AuthInterceptor {
    jwt_manager: JwtManager,
}

impl AuthInterceptor {
    pub fn new(jwt_manager: JwtManager) -> Self {
        Self { jwt_manager }
    }

    pub fn intercept(&self, mut req: Request<()>) -> Result<Request<()>, Status> {
        // Extract authorization header
        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("Missing authorization token"))?;

        // Validate token
        let claims = self.jwt_manager
            .validate_token(token)
            .map_err(|_| Status::unauthenticated("Invalid token"))?;

        // Add claims to request extensions for use in handlers
        req.extensions_mut().insert(claims);

        Ok(req)
    }
}

// Helper to extract claims from request
pub fn get_claims<T>(req: &Request<T>) -> Result<&Claims, Status> {
    req.extensions()
        .get::<Claims>()
        .ok_or_else(|| Status::internal("Claims not found in request"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::metadata::MetadataValue;

    #[test]
    fn test_missing_auth_header() {
        let jwt_manager = JwtManager::new("test-secret", 3600);
        let interceptor = AuthInterceptor::new(jwt_manager);

        let request = Request::new(());
        let result = interceptor.intercept(request);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn test_valid_token_interceptor() {
        let jwt_manager = JwtManager::new("test-secret", 3600);
        let token = jwt_manager.generate_token("user123", vec!["admin".to_string()])
            .expect("Failed to generate token");

        let interceptor = AuthInterceptor::new(JwtManager::new("test-secret", 3600));

        let mut request = Request::new(());
        let bearer_token = format!("Bearer {}", token);
        let metadata_value = MetadataValue::try_from(&bearer_token).unwrap();
        request.metadata_mut().insert("authorization", metadata_value);

        let result = interceptor.intercept(request);
        assert!(result.is_ok());

        let request = result.unwrap();
        let claims = request.extensions().get::<Claims>().unwrap();
        assert_eq!(claims.sub, "user123");
    }
}
```

4. **`src/crates/aco/src/auth/client.rs`**
```rust
use anyhow::Result;
use tonic::metadata::MetadataValue;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AuthClient {
    token: Arc<RwLock<Option<String>>>,
}

impl AuthClient {
    pub fn new() -> Self {
        Self {
            token: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_token(&self, token: String) {
        let mut t = self.token.write().await;
        *t = Some(token);
    }

    pub async fn get_token(&self) -> Option<String> {
        self.token.read().await.clone()
    }

    pub async fn clear_token(&self) {
        let mut t = self.token.write().await;
        *t = None;
    }

    pub async fn attach_token<T>(&self, mut request: tonic::Request<T>) -> Result<tonic::Request<T>> {
        if let Some(token) = self.get_token().await {
            let bearer_token = format!("Bearer {}", token);
            let metadata_value = MetadataValue::try_from(&bearer_token)?;
            request.metadata_mut().insert("authorization", metadata_value);
        }
        Ok(request)
    }
}

impl Default for AuthClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_storage() {
        let client = AuthClient::new();
        assert!(client.get_token().await.is_none());

        client.set_token("test-token".to_string()).await;
        assert_eq!(client.get_token().await.unwrap(), "test-token");

        client.clear_token().await;
        assert!(client.get_token().await.is_none());
    }

    #[tokio::test]
    async fn test_attach_token() {
        let client = AuthClient::new();
        client.set_token("test-token".to_string()).await;

        let request = tonic::Request::new(());
        let request = client.attach_token(request).await.unwrap();

        let auth_header = request.metadata().get("authorization").unwrap();
        assert_eq!(auth_header.to_str().unwrap(), "Bearer test-token");
    }
}
```

### Update Cargo.toml Files

5. **`src/crates/orchestrator/Cargo.toml`**:
```toml
[dependencies]
jsonwebtoken = "9.2"
serde = { workspace = true }
```

6. **`src/crates/aco/Cargo.toml`**:
```toml
[dependencies]
tonic = "0.10"
tokio = { workspace = true, features = ["sync"] }
anyhow = { workspace = true }
```

## Unit Tests

All unit tests are embedded in the implementation files above.

## Acceptance Criteria

- [ ] JWT token generation and validation works
- [ ] Tokens include expiration and role claims
- [ ] Server interceptor validates tokens and extracts claims
- [ ] Client can attach tokens to requests
- [ ] Invalid tokens are rejected with proper error codes
- [ ] Token expiration is enforced
- [ ] All tests pass
- [ ] Thread-safe token storage in client
- [ ] Environment variable configuration for JWT secret

## Complexity
**Moderate** - Standard JWT implementation with gRPC integration

## Estimated Effort
**6-8 hours**

## Notes
- Use strong secrets in production (not the dev default)
- Consider refresh token rotation for enhanced security
- Claims can be extended with custom fields as needed
- Token storage in client is in-memory; could add persistent storage later
