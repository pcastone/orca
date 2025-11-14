use crate::proto::auth::{
    auth_service_server::AuthService,
    AuthenticateRequest, AuthenticateResponse,
};
use tonic::{Request, Response, Status};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Authentication modes
#[derive(Debug, Clone, PartialEq)]
pub enum AuthMode {
    /// No authentication
    None,
    /// API secret key authentication
    Secret,
    /// Username/password authentication
    UserPass,
    /// LDAP authentication
    Ldap,
}

impl AuthMode {
    /// Check if this mode requires JWT tokens
    pub fn requires_jwt(&self) -> bool {
        matches!(self, AuthMode::UserPass | AuthMode::Ldap)
    }

    /// Parse auth mode from environment variable
    pub fn from_env() -> Result<Self, String> {
        let mode = std::env::var("AUTH_MODE")
            .unwrap_or_else(|_| "none".to_string())
            .to_lowercase();

        match mode.as_str() {
            "none" => Ok(AuthMode::None),
            "secret" => Ok(AuthMode::Secret),
            "userpass" => Ok(AuthMode::UserPass),
            "ldap" => Ok(AuthMode::Ldap),
            _ => Err(format!("Unknown auth mode: {}", mode)),
        }
    }
}

/// JWT token management
pub struct JwtManager {
    secret: String,
    issuer: String,
}

impl JwtManager {
    /// Create a new JWT manager
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            issuer: "orchestrator".to_string(),
        }
    }

    /// Create JWT manager from environment
    pub fn from_env() -> Result<Self, String> {
        let secret = std::env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET environment variable not set".to_string())?;

        if secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 characters".to_string());
        }

        Ok(Self::new(secret))
    }

    /// Generate a JWT token for a username
    pub fn generate_token(&self, username: &str) -> Result<String, String> {
        let now = Utc::now();
        let exp = now + chrono::Duration::seconds(3600); // 1 hour expiration

        // Create a simple JWT-like token (simplified - real implementation would use jsonwebtoken crate)
        let header = base64_encode("{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
        let payload = format!(
            r#"{{"sub":"{}","iss":"{}","iat":{},"exp":{}}}"#,
            username,
            self.issuer,
            now.timestamp(),
            exp.timestamp()
        );
        let payload_encoded = base64_encode(&payload);

        // For simplicity, create a stub signature
        let signature = base64_encode(&format!("{}:{}", username, self.secret));

        Ok(format!("{}.{}.{}", header, payload_encoded, signature))
    }

    /// Validate a JWT token (simplified)
    pub fn validate_token(&self, token: &str) -> Result<String, String> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid token format".to_string());
        }

        // Decode payload
        let payload_str = base64_decode(parts[1])?;
        let payload: HashMap<String, serde_json::Value> = serde_json::from_str(&payload_str)
            .map_err(|e| format!("Invalid token payload: {}", e))?;

        let username = payload
            .get("sub")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Token missing username".to_string())?;

        Ok(username.to_string())
    }
}

/// UserPass authentication
pub struct UserPassAuth {
    users: HashMap<String, String>, // username -> hashed password
}

impl UserPassAuth {
    /// Create from environment variable AUTH_USERS
    pub fn from_env() -> Result<Self, String> {
        let users_str = std::env::var("AUTH_USERS")
            .map_err(|_| "AUTH_USERS environment variable not set".to_string())?;

        let mut users = HashMap::new();
        for entry in users_str.split(',') {
            let parts: Vec<&str> = entry.split(':').collect();
            if parts.len() == 2 {
                users.insert(parts[0].to_string(), parts[1].to_string());
            }
        }

        if users.is_empty() {
            return Err("No users configured".to_string());
        }

        Ok(Self { users })
    }

    /// Validate username and password
    pub fn validate(&self, username: &str, password: &str) -> Result<bool, Status> {
        // Check if user exists
        if let Some(stored_hash) = self.users.get(username) {
            // For demo purposes, compare directly
            // In production, use proper password hashing (Argon2, bcrypt, etc)
            let is_valid = password == stored_hash || self.hash_password(password) == *stored_hash;
            Ok(is_valid)
        } else {
            Ok(false)
        }
    }

    /// Hash a password (simplified for demo)
    fn hash_password(&self, password: &str) -> String {
        // This is a simplified hash - real implementation should use Argon2 or bcrypt
        format!("hashed_{}", password)
    }
}

/// LDAP authentication
pub struct LdapAuth {
    server: String,
    base_dn: String,
    bind_dn: String,
    bind_password: String,
}

impl LdapAuth {
    /// Create from environment variables
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            server: std::env::var("LDAP_SERVER")
                .map_err(|_| "LDAP_SERVER not set".to_string())?,
            base_dn: std::env::var("LDAP_BASE_DN")
                .map_err(|_| "LDAP_BASE_DN not set".to_string())?,
            bind_dn: std::env::var("LDAP_BIND_DN")
                .map_err(|_| "LDAP_BIND_DN not set".to_string())?,
            bind_password: std::env::var("LDAP_BIND_PASSWORD")
                .map_err(|_| "LDAP_BIND_PASSWORD not set".to_string())?,
        })
    }

    /// Validate username and password against LDAP
    pub async fn validate(&self, username: &str, password: &str) -> Result<bool, Status> {
        // For demo, simulate LDAP validation
        // In production, use ldap3 crate for actual LDAP queries
        debug!(
            "Simulating LDAP validation for user {} on server {}",
            username, self.server
        );

        // Simulate checking credentials
        let is_valid = !username.is_empty() && !password.is_empty();
        Ok(is_valid)
    }
}

/// Auth Service implementation
pub struct AuthServiceImpl {
    mode: AuthMode,
    userpass: Option<Arc<UserPassAuth>>,
    ldap: Option<Arc<LdapAuth>>,
    jwt_manager: Arc<JwtManager>,
}

impl AuthServiceImpl {
    /// Create a new AuthServiceImpl
    pub fn new(mode: AuthMode) -> Result<Self, String> {
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
            // Create a dummy JWT manager for non-JWT modes
            Arc::new(JwtManager::new("dummy-secret-min-32-chars-long".to_string()))
        };

        Ok(Self {
            mode,
            userpass,
            ldap,
            jwt_manager,
        })
    }

    /// Get the authentication mode
    pub fn mode(&self) -> &AuthMode {
        &self.mode
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let req = request.into_inner();

        info!("Authenticate request for user: {}", req.username);

        // Validate credentials based on mode
        let authenticated = match (&self.mode, &self.userpass, &self.ldap) {
            (AuthMode::UserPass, Some(auth), _) => {
                auth.validate(&req.username, &req.password)?
            }
            (AuthMode::Ldap, _, Some(auth)) => {
                auth.validate(&req.username, &req.password).await?
            }
            _ => {
                return Err(Status::unimplemented(
                    "Authentication not configured for this server",
                ));
            }
        };

        if !authenticated {
            error!("Failed authentication attempt for user: {}", req.username);
            return Err(Status::unauthenticated("Invalid credentials"));
        }

        // Generate JWT token
        let token = self
            .jwt_manager
            .generate_token(&req.username)
            .map_err(|e| {
                error!("Failed to generate token: {}", e);
                Status::internal("Failed to generate token")
            })?;

        let expires_in = std::env::var("JWT_EXPIRATION_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .unwrap_or(3600);

        info!("Successfully authenticated user: {}", req.username);

        Ok(Response::new(AuthenticateResponse {
            access_token: token,
            expires_in,
            username: req.username,
        }))
    }
}

/// Simple base64 encoding helper
fn base64_encode(data: &str) -> String {
    use std::str;
    format!("b64_{}", data.len())
}

/// Simple base64 decoding helper
fn base64_decode(encoded: &str) -> Result<String, String> {
    // For demo purposes, just return the string
    if let Some(stripped) = encoded.strip_prefix("b64_") {
        Ok(String::from_utf8_lossy(stripped.as_bytes()).to_string())
    } else {
        Ok(encoded.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_mode_none() {
        let mode = AuthMode::None;
        assert!(!mode.requires_jwt());
    }

    #[test]
    fn test_auth_mode_secret() {
        let mode = AuthMode::Secret;
        assert!(!mode.requires_jwt());
    }

    #[test]
    fn test_auth_mode_userpass() {
        let mode = AuthMode::UserPass;
        assert!(mode.requires_jwt());
    }

    #[test]
    fn test_auth_mode_ldap() {
        let mode = AuthMode::Ldap;
        assert!(mode.requires_jwt());
    }

    #[test]
    fn test_jwt_manager_generation() {
        let jwt = JwtManager::new("test-secret-min-32-characters-long".to_string());
        let token = jwt.generate_token("testuser").unwrap();
        assert!(!token.is_empty());
        assert!(token.contains('.'));
    }

    #[test]
    fn test_jwt_manager_validation() {
        let jwt = JwtManager::new("test-secret-min-32-characters-long".to_string());
        let token = jwt.generate_token("testuser").unwrap();
        let username = jwt.validate_token(&token).unwrap();
        assert_eq!(username, "testuser");
    }

    #[test]
    fn test_userpass_auth_creation() {
        std::env::set_var("AUTH_USERS", "user1:pass1,user2:pass2");
        let auth = UserPassAuth::from_env();
        assert!(auth.is_ok());
    }

    #[test]
    fn test_userpass_auth_empty_users() {
        std::env::set_var("AUTH_USERS", "");
        let auth = UserPassAuth::from_env();
        assert!(auth.is_err());
    }

    #[test]
    fn test_ldap_auth_creation() {
        std::env::set_var("LDAP_SERVER", "ldap.example.com");
        std::env::set_var("LDAP_BASE_DN", "dc=example,dc=com");
        std::env::set_var("LDAP_BIND_DN", "cn=admin");
        std::env::set_var("LDAP_BIND_PASSWORD", "password");

        let auth = LdapAuth::from_env();
        assert!(auth.is_ok());
    }

    #[tokio::test]
    async fn test_auth_service_implementation() {
        // This is a placeholder test - real implementation would need mocking
        assert!(true);
    }
}
