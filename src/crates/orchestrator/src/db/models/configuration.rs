//! Configuration model for database persistence
//!
//! Represents key-value configuration entries in the database.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a configuration entry in the database
///
/// Configurations are key-value pairs with optional metadata.
/// They can store strings, integers, floats, booleans, or JSON values.
/// Sensitive configuration values can be marked as secrets.
///
/// # Timestamps
/// All timestamp fields are ISO8601 strings due to SQLite type limitations.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Configuration {
    /// Configuration key (unique identifier)
    pub key: String,

    /// Configuration value
    pub value: String,

    /// Type of the value: string, integer, float, boolean, json
    pub value_type: String,

    /// Optional description of the configuration
    pub description: Option<String>,

    /// Whether this is a secret value (0 = no, 1 = yes)
    pub is_secret: i32,

    /// Configuration creation timestamp (ISO8601 string)
    pub created_at: String,

    /// Configuration last update timestamp (ISO8601 string)
    pub updated_at: String,
}

impl Configuration {
    /// Create a new string configuration
    ///
    /// # Arguments
    /// * `key` - Configuration key
    /// * `value` - Configuration value
    ///
    /// # Returns
    /// A new Configuration with string type and non-secret flag
    pub fn string(key: String, value: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            key,
            value,
            value_type: "string".to_string(),
            description: None,
            is_secret: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Create a new integer configuration
    pub fn integer(key: String, value: i32) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            key,
            value: value.to_string(),
            value_type: "integer".to_string(),
            description: None,
            is_secret: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Create a new float configuration
    pub fn float(key: String, value: f64) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            key,
            value: value.to_string(),
            value_type: "float".to_string(),
            description: None,
            is_secret: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Create a new boolean configuration
    pub fn boolean(key: String, value: bool) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            key,
            value: value.to_string(),
            value_type: "boolean".to_string(),
            description: None,
            is_secret: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Create a new JSON configuration
    pub fn json(key: String, value: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            key,
            value,
            value_type: "json".to_string(),
            description: None,
            is_secret: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Builder method to set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark this configuration as a secret
    pub fn as_secret(mut self) -> Self {
        self.is_secret = 1;
        self
    }

    /// Check if this is a secret configuration
    pub fn is_secret(&self) -> bool {
        self.is_secret == 1
    }

    /// Parse value as integer
    pub fn as_integer(&self) -> Option<i32> {
        self.value.parse().ok()
    }

    /// Parse value as float
    pub fn as_float(&self) -> Option<f64> {
        self.value.parse().ok()
    }

    /// Parse value as boolean
    pub fn as_boolean(&self) -> Option<bool> {
        match self.value.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        }
    }

    /// Parse value as JSON
    pub fn as_json(&self) -> Option<serde_json::Value> {
        serde_json::from_str(&self.value).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_string() {
        let config = Configuration::string(
            "app.name".to_string(),
            "Orchestrator".to_string(),
        );

        assert_eq!(config.key, "app.name");
        assert_eq!(config.value, "Orchestrator");
        assert_eq!(config.value_type, "string");
        assert!(!config.is_secret());
    }

    #[test]
    fn test_configuration_integer() {
        let config = Configuration::integer("app.port".to_string(), 8080);

        assert_eq!(config.key, "app.port");
        assert_eq!(config.as_integer(), Some(8080));
        assert_eq!(config.value_type, "integer");
    }

    #[test]
    fn test_configuration_float() {
        let config = Configuration::float("app.timeout".to_string(), 30.5);

        assert_eq!(config.key, "app.timeout");
        assert_eq!(config.as_float(), Some(30.5));
        assert_eq!(config.value_type, "float");
    }

    #[test]
    fn test_configuration_boolean() {
        let config = Configuration::boolean("app.debug".to_string(), true);

        assert_eq!(config.key, "app.debug");
        assert_eq!(config.as_boolean(), Some(true));
        assert_eq!(config.value_type, "boolean");
    }

    #[test]
    fn test_configuration_json() {
        let json_str = r#"{"nested": "value"}"#;
        let config = Configuration::json("app.settings".to_string(), json_str.to_string());

        assert_eq!(config.key, "app.settings");
        assert_eq!(config.value_type, "json");
        assert!(config.as_json().is_some());
    }

    #[test]
    fn test_configuration_secret() {
        let config = Configuration::string("db.password".to_string(), "secret123".to_string())
            .as_secret();

        assert!(config.is_secret());
    }

    #[test]
    fn test_configuration_with_description() {
        let config = Configuration::string("app.name".to_string(), "Orchestrator".to_string())
            .with_description("Application name");

        assert_eq!(config.description, Some("Application name".to_string()));
    }
}
