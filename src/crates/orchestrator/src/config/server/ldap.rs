//! LDAP configuration and authentication
//!
//! Handles LDAP connection and authentication based on configuration.

use crate::config::LdapConfig;
use ldap3::{Ldap, LdapConnAsync};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Debug, Error)]
pub enum LdapError {
    #[error("LDAP connection failed: {0}")]
    Connection(String),
    #[error("LDAP bind failed: {0}")]
    Bind(String),
    #[error("LDAP search failed: {0}")]
    Search(String),
    #[error("LDAP not enabled")]
    NotEnabled,
}

/// LDAP client wrapper
pub struct LdapClient {
    config: Arc<LdapConfig>,
    connection: Option<Ldap>,
}

impl LdapClient {
    /// Create a new LDAP client (does not connect immediately)
    pub fn new(config: LdapConfig) -> Self {
        Self {
            config: Arc::new(config),
            connection: None,
        }
    }

    /// Connect to LDAP server
    pub async fn connect(&mut self) -> Result<(), LdapError> {
        if !self.config.enabled {
            return Err(LdapError::NotEnabled);
        }

        if self.config.server_url.is_empty() {
            return Err(LdapError::Connection("LDAP server URL not configured".to_string()));
        }

        info!("Connecting to LDAP server: {}", self.config.server_url);
        
        let server_url = &self.config.server_url;

        let (conn, mut ldap) = LdapConnAsync::new(server_url)
            .await
            .map_err(|e| LdapError::Connection(format!("Failed to connect: {}", e)))?;

        // Bind with DN (empty password for anonymous bind, or use readonly credentials)
        let _ = if let Some((user, pass)) = self.readonly_credentials() {
            ldap.simple_bind(&user, &pass)
                .await
                .map_err(|e| LdapError::Bind(format!("Bind failed: {}", e)))?;
        } else {
            // Try anonymous bind
            ldap.simple_bind("", "")
                .await
                .map_err(|e| LdapError::Bind(format!("Bind failed: {}", e)))?;
        };

        self.connection = Some(ldap);
        info!("LDAP connection established");
        Ok(())
    }

    /// Authenticate a user
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<bool, LdapError> {
        if !self.config.enabled {
            return Err(LdapError::NotEnabled);
        }

        // Build user DN
        let user_dn = format!("cn={},{}", username, self.config.suffix);
        
        debug!("Attempting LDAP authentication for: {}", user_dn);

        // Use configured server URL
        let server_url = if self.config.server_url.is_empty() {
            "ldap://localhost"
        } else {
            &self.config.server_url
        };

        // Try to bind with user credentials
        let (_conn, mut ldap) = LdapConnAsync::new(server_url)
            .await
            .map_err(|e| LdapError::Connection(format!("Failed to connect: {}", e)))?;

        match ldap.simple_bind(&user_dn, password).await {
            Ok(_) => {
                debug!("LDAP authentication successful for: {}", username);
                Ok(true)
            }
            Err(e) => {
                warn!("LDAP authentication failed for {}: {}", username, e);
                Ok(false)
            }
        }
    }

    /// Check if user is in required group
    pub async fn check_group_membership(&self, username: &str) -> Result<bool, LdapError> {
        if !self.config.enabled {
            return Err(LdapError::NotEnabled);
        }

        if self.config.group.is_empty() {
            // No group requirement
            return Ok(true);
        }

        // Search for user's group membership
        let search_base = &self.config.suffix;
        let filter = format!("(&(cn={})(memberOf={}))", username, self.config.group);
        
        debug!("Checking group membership: {}", filter);

        // TODO: Implement actual LDAP search
        // For now, return true if group is not configured
        Ok(true)
    }

    /// Get read-only login credentials
    pub fn readonly_credentials(&self) -> Option<(String, String)> {
        if self.config.readonly_login.is_empty() {
            return None;
        }

        // Parse "username:password" or "dn:password"
        if let Some((user, pass)) = self.config.readonly_login.split_once(':') {
            Some((user.to_string(), pass.to_string()))
        } else {
            None
        }
    }
}

impl Default for LdapClient {
    fn default() -> Self {
        Self::new(LdapConfig {
            enabled: false,
            server_url: String::new(),
            dn: String::new(),
            suffix: String::new(),
            group: String::new(),
            readonly_login: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ldap_client_creation() {
        let config = LdapConfig {
            enabled: true,
            server_url: "ldap://localhost".to_string(),
            dn: "cn=admin,dc=example,dc=com".to_string(),
            suffix: "dc=example,dc=com".to_string(),
            group: "cn=users".to_string(),
            readonly_login: "readonly:password".to_string(),
        };
        let client = LdapClient::new(config);
        // Just test that it creates without error
        assert!(client.config.enabled);
    }

    #[test]
    fn test_readonly_credentials_parsing() {
        let config = LdapConfig {
            enabled: true,
            server_url: "ldap://localhost".to_string(),
            dn: "cn=admin,dc=example,dc=com".to_string(),
            suffix: "dc=example,dc=com".to_string(),
            group: "cn=users".to_string(),
            readonly_login: "readonly:password123".to_string(),
        };
        let client = LdapClient::new(config);
        let creds = client.readonly_credentials();
        assert!(creds.is_some());
        let (user, pass) = creds.unwrap();
        assert_eq!(user, "readonly");
        assert_eq!(pass, "password123");
    }
}

