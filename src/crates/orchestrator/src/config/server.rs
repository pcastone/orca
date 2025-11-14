//! Server configuration for orchestrator-server
//!
//! Loads and parses orchestrator-server.toml configuration file
//! with SSL/TLS, security, database, and LDAP settings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

pub mod ldap;
pub mod security;
pub mod ssl;

#[derive(Debug, Error)]
pub enum ServerConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    ParseError(toml::de::Error),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// SSL/TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    /// SSL cipher suite
    pub cipher: String,
    /// TLS version ("1.2" or "1.3")
    pub version: String,
    /// MAC algorithm
    pub mac: String,
    /// SSL mode: "auto" (auto-generate PEM) or "pem" (use predefined PEM)
    pub mode: SslMode,
    /// X509 certificate configuration for auto-generation
    #[serde(default)]
    pub x509: X509Config,
}

/// SSL mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SslMode {
    /// Auto-generate PEM files
    Auto,
    /// Use predefined PEM files
    Pem,
}

/// X509 certificate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X509Config {
    /// Country code (ISO 3166-1 alpha-2)
    pub country: String,
    /// State or Province
    pub state: String,
    /// City or Locality
    pub locality: String,
    /// Organization name
    pub organization: String,
    /// Organizational unit
    pub organizational_unit: String,
    /// Common Name (CN)
    pub common_name: String,
    /// Certificate validity period in days
    pub validity_days: u32,
}

impl Default for X509Config {
    fn default() -> Self {
        Self {
            country: "US".to_string(),
            state: "CA".to_string(),
            locality: "San Francisco".to_string(),
            organization: "ACO".to_string(),
            organizational_unit: "Engineering".to_string(),
            common_name: "orchestrator-server".to_string(),
            validity_days: 365,
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// SQLite database file path
    pub path: String,
}

/// Security mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecurityMode {
    /// No authentication required
    Open,
    /// API key based authentication
    SecretKey,
    /// User login credentials
    UserLogin,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Security mode
    pub mode: SecurityMode,
    /// Secret key (can be overridden by SECRET_KEY environment variable)
    #[serde(default)]
    pub secret_key: Option<String>,
}

/// LDAP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapConfig {
    /// Enable LDAP authentication
    pub enabled: bool,
    /// LDAP server URL (e.g., "ldap://localhost:389" or "ldaps://ldap.example.com:636")
    pub server_url: String,
    /// Distinguished Name (DN) for LDAP bind
    pub dn: String,
    /// LDAP suffix (e.g., "dc=example,dc=com")
    pub suffix: String,
    /// LDAP group for authorization
    pub group: String,
    /// Read-only login credentials (username:password or DN:password)
    pub readonly_login: String,
}

/// Server identification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfoConfig {
    /// Server name for identification (displayed to clients)
    #[serde(default = "default_server_name")]
    pub name: String,
}

impl Default for ServerInfoConfig {
    fn default() -> Self {
        Self {
            name: default_server_name(),
        }
    }
}

fn default_server_name() -> String {
    "orchestrator-server".to_string()
}

/// Complete server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server identification
    #[serde(default)]
    pub server: ServerInfoConfig,
    /// SSL/TLS configuration
    pub ssl: SslConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// LDAP configuration
    pub ldap: LdapConfig,
}

impl ServerConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ServerConfigError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(ServerConfigError::ReadError)?;
        Self::from_str(&content)
    }

    /// Load configuration from TOML string
    pub fn from_str(content: &str) -> Result<Self, ServerConfigError> {
        toml::from_str(content).map_err(ServerConfigError::ParseError)
    }

    /// Load configuration from default location or environment
    ///
    /// Searches for config in:
    /// 1. CONFIG_PATH environment variable
    /// 2. ./config/orchestrator-server.toml
    /// 3. ../config/orchestrator-server.toml (for development)
    pub fn load() -> Result<Self, ServerConfigError> {
        // Try environment variable first
        if let Ok(config_path) = std::env::var("CONFIG_PATH") {
            return Self::from_file(config_path);
        }

        // Try common locations
        let paths = [
            PathBuf::from("config/orchestrator-server.toml"),
            PathBuf::from("../config/orchestrator-server.toml"),
            PathBuf::from("./orchestrator-server.toml"),
        ];

        for path in &paths {
            if path.exists() {
                return Self::from_file(path);
            }
        }

        Err(ServerConfigError::InvalidConfig(
            "Configuration file not found. Set CONFIG_PATH or place orchestrator-server.toml in config/".to_string(),
        ))
    }

    /// Get the secret key, checking environment variable first
    pub fn get_secret_key(&self) -> Option<String> {
        std::env::var("SECRET_KEY")
            .ok()
            .or_else(|| self.security.secret_key.clone())
    }

    /// Get database URL from configuration
    pub fn database_url(&self) -> String {
        format!("sqlite://{}", self.database.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let toml_content = r#"
[ssl]
cipher = "ECDHE-RSA-AES256-GCM-SHA384"
version = "1.3"
mac = "SHA256"
mode = "auto"

[ssl.x509]
country = "US"
state = "CA"
locality = "San Francisco"
organization = "ACO"
organizational_unit = "Engineering"
common_name = "orchestrator-server"
validity_days = 365

[database]
path = "orchestrator.db"

[security]
mode = "secret-key"
secret_key = ""

[ldap]
enabled = false
dn = ""
suffix = ""
group = ""
readonly_login = ""
"#;

        let config = ServerConfig::from_str(toml_content).unwrap();
        assert_eq!(config.ssl.cipher, "ECDHE-RSA-AES256-GCM-SHA384");
        assert_eq!(config.ssl.version, "1.3");
        assert_eq!(config.ssl.mode, SslMode::Auto);
        assert_eq!(config.database.path, "orchestrator.db");
        assert_eq!(config.security.mode, SecurityMode::SecretKey);
        assert!(!config.ldap.enabled);
    }
}

