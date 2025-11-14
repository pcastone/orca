# Task 015: Implement Authenticate RPC for UserPass/LDAP Modes

## Objective
Implement Authenticate RPC that validates credentials (userpass or LDAP) and returns JWT token for subsequent requests.

## Priority
**MODERATE** - Only needed for userpass and LDAP modes

## Dependencies
- Task 002 (Multi-mode auth implementation)

## Implementation Details

### Update Proto File

Add to `src/crates/orchestrator/proto/auth.proto`:
```protobuf
syntax = "proto3";
package orchestrator.auth;

service AuthService {
  rpc Authenticate(AuthenticateRequest) returns (AuthenticateResponse);
}

message AuthenticateRequest {
  string username = 1;
  string password = 2;
}

message AuthenticateResponse {
  string access_token = 1;
  int64 expires_in = 2;  // seconds
  string username = 3;
}
```

### Server Implementation

**`src/crates/orchestrator/src/services/auth.rs`**:
```rust
use crate::proto::auth::{
    auth_service_server::AuthService,
    AuthenticateRequest, AuthenticateResponse,
};
use crate::auth::{AuthMode, UserPassAuth, LdapAuth, JwtManager};
use tonic::{Request, Response, Status};
use std::sync::Arc;

pub struct AuthServiceImpl {
    mode: AuthMode,
    userpass: Option<Arc<UserPassAuth>>,
    ldap: Option<Arc<LdapAuth>>,
    jwt_manager: Arc<JwtManager>,
}

impl AuthServiceImpl {
    pub fn new(mode: AuthMode) -> anyhow::Result<Self> {
        let (userpass, ldap) = match &mode {
            AuthMode::UserPass => {
                let auth = UserPassAuth::from_env()?;
                (Some(Arc::new(auth)), None)
            }
            AuthMode::Ldap => {
                let auth = LdapAuth::from_env()?;
                (None, Some(Arc::new(auth)))
            }
            _ => (None, None),
        };

        let jwt_manager = if mode.requires_jwt() {
            Arc::new(JwtManager::from_env()?)
        } else {
            // Won't be used, but create with defaults
            Arc::new(JwtManager::from_env()?)
        };

        Ok(Self {
            mode,
            userpass,
            ldap,
            jwt_manager,
        })
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let req = request.into_inner();

        // Validate based on mode
        let authenticated = match (&self.mode, &self.userpass, &self.ldap) {
            (AuthMode::UserPass, Some(auth), _) => {
                auth.validate(&req.username, &req.password)?
            }
            (AuthMode::Ldap, _, Some(auth)) => {
                auth.validate(&req.username, &req.password).await?
            }
            _ => {
                return Err(Status::unimplemented(
                    "Authentication not configured for this server"
                ));
            }
        };

        if !authenticated {
            return Err(Status::unauthenticated("Invalid credentials"));
        }

        // Generate JWT token
        let token = self.jwt_manager
            .generate_token(&req.username)
            .map_err(|e| {
                tracing::error!("Failed to generate token: {}", e);
                Status::internal("Failed to generate token")
            })?;

        let expires_in = std::env::var("JWT_EXPIRATION_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .unwrap_or(3600);

        Ok(Response::new(AuthenticateResponse {
            access_token: token,
            expires_in,
            username: req.username,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::{Argon2, PasswordHasher};
    use argon2::password_hash::SaltString;
    use rand::rngs::OsRng;

    #[tokio::test]
    async fn test_userpass_authentication() {
        // Setup test user
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password("testpass".as_bytes(), &salt)
            .unwrap()
            .to_string();

        std::env::set_var("AUTH_USERS", format!("testuser:{}", hash));
        std::env::set_var("JWT_SECRET", "test-secret-min-32-characters-long");

        let service = AuthServiceImpl::new(AuthMode::UserPass).unwrap();

        let request = Request::new(AuthenticateRequest {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        });

        let response = service.authenticate(request).await.unwrap();
        let auth_response = response.into_inner();

        assert!(!auth_response.access_token.is_empty());
        assert_eq!(auth_response.username, "testuser");
        assert!(auth_response.expires_in > 0);
    }

    #[tokio::test]
    async fn test_invalid_credentials() {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password("correctpass".as_bytes(), &salt)
            .unwrap()
            .to_string();

        std::env::set_var("AUTH_USERS", format!("testuser:{}", hash));
        std::env::set_var("JWT_SECRET", "test-secret-min-32-characters-long");

        let service = AuthServiceImpl::new(AuthMode::UserPass).unwrap();

        let request = Request::new(AuthenticateRequest {
            username: "testuser".to_string(),
            password: "wrongpass".to_string(),
        });

        let result = service.authenticate(request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::Unauthenticated);
    }
}
```

### Update Server Setup

In **`src/crates/orchestrator/src/server.rs`**, add AuthService:

```rust
use crate::services::AuthServiceImpl;
use crate::proto::auth::auth_service_server::AuthServiceServer;

// In serve() method:
let auth_mode = AuthMode::from_env()?;

// Only register AuthService if auth mode requires it
if auth_mode.requires_jwt() {
    let auth_service = AuthServiceImpl::new(auth_mode.clone())?;

    Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        // ... other services
        .serve(addr)
        .await?;
}
```

## Client Usage

**`src/crates/aco/src/client.rs`** - Add authenticate method:

```rust
use crate::proto::auth::auth_service_client::AuthServiceClient;
use crate::proto::auth::AuthenticateRequest;

impl AcoClient {
    pub async fn authenticate(&mut self, username: &str, password: &str) -> Result<String> {
        let mut auth_client = AuthServiceClient::new(self.channel.clone());

        let request = Request::new(AuthenticateRequest {
            username: username.to_string(),
            password: password.to_string(),
        });

        let response = auth_client.authenticate(request).await?;
        let auth_response = response.into_inner();

        // Store token for future requests
        self.auth.set_token(auth_response.access_token.clone()).await;

        Ok(auth_response.access_token)
    }
}
```

## Acceptance Criteria

- [ ] Authenticate RPC validates userpass credentials
- [ ] Authenticate RPC validates LDAP credentials
- [ ] JWT token generated and returned
- [ ] Token includes username in claims
- [ ] Expiration time included in response
- [ ] Invalid credentials rejected
- [ ] Service only registered when mode requires JWT
- [ ] Client can call authenticate and store token
- [ ] All tests pass

## Complexity
**Moderate** - Integrates with auth modes from Task 002

## Estimated Effort
**4-5 hours**

## Notes
- AuthService is optional - only needed for userpass/LDAP modes
- Secret mode doesn't need this RPC (secret passed directly)
- No-auth mode doesn't need this RPC (no authentication)
- Token should be validated on subsequent requests via interceptor
- Consider adding token refresh endpoint later if needed
