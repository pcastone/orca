# Task 002: Implement Multi-Mode Authentication System

## Objective
Implement flexible authentication system for orchestrator server supporting 4 modes: no-auth, secret, user/pass, and LDAP. Client uses `--connect` flag with appropriate credentials.

## Priority
**HIGH** - Security foundation for client-server communication

## Dependencies
- Task 001 (Protocol Buffer definitions)

## Authentication Modes

### 1. No-Auth Mode
- No authentication required
- All requests allowed
- Use for local development only
- **ENV**: `AUTH_MODE=none`

### 2. Secret Mode
- Shared secret/API key authentication
- Client provides secret in metadata header
- Simple but requires secret distribution
- **ENV**: `AUTH_MODE=secret`, `AUTH_SECRET=<secret>`

### 3. User/Pass Mode
- Username and password authentication
- Credentials stored in config file or environment
- JWT tokens issued after validation
- **ENV**: `AUTH_MODE=userpass`, `AUTH_USERS=user1:hash1,user2:hash2`

### 4. LDAP Mode
- LDAP server authentication
- Validates against external LDAP directory
- JWT tokens issued after LDAP validation
- **ENV**: `AUTH_MODE=ldap`, `LDAP_URL=ldap://...`, `LDAP_BIND_DN=...`

## Implementation Details

### Files to Create

1. **`src/crates/orchestrator/src/auth/mod.rs`**
```rust
pub mod mode;
pub mod secret;
pub mod userpass;
pub mod ldap;
pub mod jwt;
pub mod interceptor;

pub use mode::AuthMode;
pub use interceptor::create_auth_interceptor;
```

2. **`src/crates/orchestrator/src/auth/mode.rs`**
```rust
use anyhow::{Result, Context};

#[derive(Debug, Clone, PartialEq)]
pub enum AuthMode {
    None,
    Secret,
    UserPass,
    Ldap,
}

impl AuthMode {
    pub fn from_env() -> Result<Self> {
        let mode = std::env::var("AUTH_MODE")
            .unwrap_or_else(|_| "none".to_string());

        match mode.to_lowercase().as_str() {
            "none" => Ok(AuthMode::None),
            "secret" => Ok(AuthMode::Secret),
            "userpass" | "user-pass" => Ok(AuthMode::UserPass),
            "ldap" => Ok(AuthMode::Ldap),
            _ => Err(anyhow::anyhow!("Invalid AUTH_MODE: {}", mode)),
        }
    }

    pub fn requires_jwt(&self) -> bool {
        matches!(self, AuthMode::UserPass | AuthMode::Ldap)
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, AuthMode::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_mode_from_string() {
        std::env::set_var("AUTH_MODE", "secret");
        assert_eq!(AuthMode::from_env().unwrap(), AuthMode::Secret);

        std::env::set_var("AUTH_MODE", "none");
        assert_eq!(AuthMode::from_env().unwrap(), AuthMode::None);
    }

    #[test]
    fn test_requires_jwt() {
        assert!(AuthMode::UserPass.requires_jwt());
        assert!(AuthMode::Ldap.requires_jwt());
        assert!(!AuthMode::Secret.requires_jwt());
        assert!(!AuthMode::None.requires_jwt());
    }
}
```

3. **`src/crates/orchestrator/src/auth/secret.rs`**
```rust
use tonic::{Request, Status};
use anyhow::Result;

pub struct SecretAuth {
    secret: String,
}

impl SecretAuth {
    pub fn from_env() -> Result<Self> {
        let secret = std::env::var("AUTH_SECRET")
            .map_err(|_| anyhow::anyhow!("AUTH_SECRET not set"))?;

        if secret.len() < 32 {
            tracing::warn!("AUTH_SECRET is too short (< 32 chars). Use a stronger secret.");
        }

        Ok(Self { secret })
    }

    pub fn validate<T>(&self, req: &Request<T>) -> Result<(), Status> {
        let provided_secret = req
            .metadata()
            .get("x-api-secret")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Status::unauthenticated("Missing x-api-secret header"))?;

        if provided_secret != self.secret {
            return Err(Status::unauthenticated("Invalid secret"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_validation() {
        let auth = SecretAuth {
            secret: "test-secret-key-must-be-at-least-32-chars".to_string(),
        };

        let mut request = Request::new(());
        request.metadata_mut().insert(
            "x-api-secret",
            "test-secret-key-must-be-at-least-32-chars".parse().unwrap(),
        );

        assert!(auth.validate(&request).is_ok());
    }

    #[test]
    fn test_secret_validation_fails() {
        let auth = SecretAuth {
            secret: "test-secret-key-must-be-at-least-32-chars".to_string(),
        };

        let mut request = Request::new(());
        request.metadata_mut().insert(
            "x-api-secret",
            "wrong-secret".parse().unwrap(),
        );

        assert!(auth.validate(&request).is_err());
    }
}
```

4. **`src/crates/orchestrator/src/auth/userpass.rs`**
```rust
use tonic::{Request, Status};
use anyhow::Result;
use std::collections::HashMap;
use argon2::{Argon2, PasswordHash, PasswordVerifier};

pub struct UserPassAuth {
    users: HashMap<String, String>, // username -> password_hash
}

impl UserPassAuth {
    pub fn from_env() -> Result<Self> {
        let users_str = std::env::var("AUTH_USERS")
            .map_err(|_| anyhow::anyhow!("AUTH_USERS not set"))?;

        let mut users = HashMap::new();
        for entry in users_str.split(',') {
            let parts: Vec<&str> = entry.split(':').collect();
            if parts.len() != 2 {
                continue;
            }
            users.insert(parts[0].to_string(), parts[1].to_string());
        }

        Ok(Self { users })
    }

    pub fn validate(&self, username: &str, password: &str) -> Result<bool, Status> {
        let password_hash = self.users
            .get(username)
            .ok_or_else(|| Status::unauthenticated("Invalid credentials"))?;

        let parsed_hash = PasswordHash::new(password_hash)
            .map_err(|_| Status::internal("Invalid password hash format"))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::{Argon2, PasswordHasher};
    use argon2::password_hash::SaltString;
    use rand::rngs::OsRng;

    #[test]
    fn test_userpass_validation() {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password("test123".as_bytes(), &salt)
            .unwrap()
            .to_string();

        let mut users = HashMap::new();
        users.insert("testuser".to_string(), password_hash);

        let auth = UserPassAuth { users };

        assert!(auth.validate("testuser", "test123").unwrap());
        assert!(!auth.validate("testuser", "wrong").unwrap_or(false));
    }
}
```

5. **`src/crates/orchestrator/src/auth/ldap.rs`**
```rust
use tonic::Status;
use anyhow::Result;
use ldap3::{LdapConn, Scope, SearchEntry};

pub struct LdapAuth {
    ldap_url: String,
    bind_dn_template: String,  // e.g., "uid={},ou=users,dc=example,dc=com"
    search_base: Option<String>,
}

impl LdapAuth {
    pub fn from_env() -> Result<Self> {
        let ldap_url = std::env::var("LDAP_URL")
            .map_err(|_| anyhow::anyhow!("LDAP_URL not set"))?;

        let bind_dn_template = std::env::var("LDAP_BIND_DN_TEMPLATE")
            .unwrap_or_else(|_| "uid={},ou=users,dc=example,dc=com".to_string());

        let search_base = std::env::var("LDAP_SEARCH_BASE").ok();

        Ok(Self {
            ldap_url,
            bind_dn_template,
            search_base,
        })
    }

    pub async fn validate(&self, username: &str, password: &str) -> Result<bool, Status> {
        // Build bind DN
        let bind_dn = self.bind_dn_template.replace("{}", username);

        // Connect to LDAP
        let mut ldap = LdapConn::new(&self.ldap_url)
            .map_err(|e| {
                tracing::error!("LDAP connection failed: {}", e);
                Status::unavailable("LDAP server unavailable")
            })?;

        // Attempt bind with user credentials
        let bind_result = ldap.simple_bind(&bind_dn, password);

        match bind_result {
            Ok(_) => {
                // Bind successful - user authenticated
                ldap.unbind()
                    .map_err(|_| Status::internal("LDAP unbind failed"))?;
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("LDAP authentication failed for {}: {}", username, e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ldap_auth_creation() {
        std::env::set_var("LDAP_URL", "ldap://localhost:389");
        std::env::set_var("LDAP_BIND_DN_TEMPLATE", "cn={},dc=example,dc=com");

        let auth = LdapAuth::from_env().unwrap();
        assert_eq!(auth.ldap_url, "ldap://localhost:389");
    }
}
```

6. **`src/crates/orchestrator/src/auth/jwt.rs`**
```rust
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // subject (user id)
    pub exp: usize,   // expiration time
    pub iat: usize,   // issued at
}

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    expiration_seconds: u64,
}

impl JwtManager {
    pub fn from_env() -> Result<Self> {
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| {
                tracing::warn!("Using default JWT_SECRET - CHANGE IN PRODUCTION");
                "dev-jwt-secret-change-in-production-min-32-chars".to_string()
            });

        let expiration = std::env::var("JWT_EXPIRATION_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()?;

        Ok(Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            expiration_seconds: expiration,
        })
    }

    pub fn generate_token(&self, user_id: &str) -> Result<String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize;
        let claims = Claims {
            sub: user_id.to_string(),
            exp: now + self.expiration_seconds as usize,
            iat: now,
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
```

7. **`src/crates/orchestrator/src/auth/interceptor.rs`**
```rust
use tonic::{Request, Status};
use crate::auth::{AuthMode, SecretAuth, UserPassAuth, LdapAuth, JwtManager, Claims};
use std::sync::Arc;

pub enum AuthInterceptor {
    None,
    Secret(Arc<SecretAuth>),
    Jwt(Arc<JwtManager>),
}

impl AuthInterceptor {
    pub fn intercept<T>(&self, mut req: Request<T>) -> Result<Request<T>, Status> {
        match self {
            AuthInterceptor::None => {
                // No authentication - allow all requests
                Ok(req)
            }
            AuthInterceptor::Secret(auth) => {
                // Validate secret from header
                auth.validate(&req)?;
                Ok(req)
            }
            AuthInterceptor::Jwt(jwt_manager) => {
                // Validate JWT token
                let token = req
                    .metadata()
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer "))
                    .ok_or_else(|| Status::unauthenticated("Missing authorization token"))?;

                let claims = jwt_manager
                    .validate_token(token)
                    .map_err(|_| Status::unauthenticated("Invalid token"))?;

                req.extensions_mut().insert(claims);
                Ok(req)
            }
        }
    }
}

pub fn create_auth_interceptor(mode: AuthMode) -> anyhow::Result<AuthInterceptor> {
    match mode {
        AuthMode::None => Ok(AuthInterceptor::None),
        AuthMode::Secret => {
            let auth = SecretAuth::from_env()?;
            Ok(AuthInterceptor::Secret(Arc::new(auth)))
        }
        AuthMode::UserPass | AuthMode::Ldap => {
            let jwt = JwtManager::from_env()?;
            Ok(AuthInterceptor::Jwt(Arc::new(jwt)))
        }
    }
}
```

8. **`src/crates/aco/src/auth/connect.rs`**
```rust
use anyhow::Result;
use tonic::metadata::MetadataValue;

#[derive(Debug, Clone)]
pub enum ConnectAuth {
    None,
    Secret(String),
    UserPass { username: String, password: String },
    Token(String),  // For pre-obtained JWT
}

impl ConnectAuth {
    pub fn from_connect_string(connect: &str) -> Result<Self> {
        if connect.is_empty() || connect == "none" {
            return Ok(ConnectAuth::None);
        }

        // Format: "secret:<secret>" or "user:pass" or "token:<jwt>"
        if let Some(secret) = connect.strip_prefix("secret:") {
            return Ok(ConnectAuth::Secret(secret.to_string()));
        }

        if let Some(token) = connect.strip_prefix("token:") {
            return Ok(ConnectAuth::Token(token.to_string()));
        }

        // Try parsing as user:pass
        if let Some((user, pass)) = connect.split_once(':') {
            return Ok(ConnectAuth::UserPass {
                username: user.to_string(),
                password: pass.to_string(),
            });
        }

        Err(anyhow::anyhow!("Invalid --connect format"))
    }

    pub fn attach_to_request<T>(&self, mut request: tonic::Request<T>) -> Result<tonic::Request<T>> {
        match self {
            ConnectAuth::None => Ok(request),
            ConnectAuth::Secret(secret) => {
                let value = MetadataValue::try_from(secret.as_str())?;
                request.metadata_mut().insert("x-api-secret", value);
                Ok(request)
            }
            ConnectAuth::Token(token) => {
                let bearer = format!("Bearer {}", token);
                let value = MetadataValue::try_from(&bearer)?;
                request.metadata_mut().insert("authorization", value);
                Ok(request)
            }
            ConnectAuth::UserPass { .. } => {
                // User/pass requires obtaining JWT first (handled in Task 015)
                Ok(request)
            }
        }
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
        assert!(matches!(auth, ConnectAuth::Secret(_)));
    }

    #[test]
    fn test_parse_connect_userpass() {
        let auth = ConnectAuth::from_connect_string("admin:password123").unwrap();
        assert!(matches!(auth, ConnectAuth::UserPass { .. }));
    }

    #[test]
    fn test_parse_connect_token() {
        let auth = ConnectAuth::from_connect_string("token:eyJhbGc...").unwrap();
        assert!(matches!(auth, ConnectAuth::Token(_)));
    }
}
```

## Update Cargo.toml

**`src/crates/orchestrator/Cargo.toml`**:
```toml
[dependencies]
jsonwebtoken = "9.2"
argon2 = "0.5"
ldap3 = "0.11"
rand = "0.8"
```

## Acceptance Criteria

- [ ] Four auth modes configurable via AUTH_MODE env var
- [ ] No-auth mode bypasses all authentication
- [ ] Secret mode validates x-api-secret header
- [ ] UserPass mode validates credentials and issues JWT
- [ ] LDAP mode authenticates against LDAP server and issues JWT
- [ ] Client supports --connect flag with all formats
- [ ] JWT tokens validated properly
- [ ] All tests pass
- [ ] Secure password hashing (Argon2)
- [ ] LDAP connection handling

## Complexity
**High** - Multiple authentication mechanisms

## Estimated Effort
**10-12 hours**

## Notes
- Use AUTH_MODE=none only for local development
- Secret should be at least 32 characters
- UserPass hashes use Argon2 (industry standard)
- LDAP timeout should be configurable
- JWT secret must be strong in production
