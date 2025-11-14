//! Configuration management utilities.
//!
//! This module provides utilities for loading and managing configuration including:
//! - Environment variable loading with type parsing
//! - Configuration validation
//! - Configuration merging
//! - YAML/JSON config file loading
//!
//! # Example
//!
//! ```rust,ignore
//! use utils::config::{get_env, get_env_parse, load_config_file};
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct AppConfig {
//!     database_url: String,
//!     port: u16,
//! }
//!
//! // Load from environment
//! let db_url = get_env("DATABASE_URL")?;
//! let port = get_env_parse::<u16>("PORT")?;
//!
//! // Load from file
//! let config: AppConfig = load_config_file("config.yaml")?;
//! ```

use crate::error::{Result, UtilsError};
use serde::de::DeserializeOwned;
use std::path::Path;

/// Get an environment variable as a string.
pub fn get_env(key: &str) -> Result<String> {
    std::env::var(key).map_err(|e| {
        UtilsError::ConfigError(format!("Environment variable '{}' not found: {}", key, e))
    })
}

/// Get an environment variable and parse it to the specified type.
pub fn get_env_parse<T: std::str::FromStr>(key: &str) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    let value = get_env(key)?;
    value.parse::<T>().map_err(|e| {
        UtilsError::ConfigError(format!(
            "Failed to parse environment variable '{}': {}",
            key, e
        ))
    })
}

/// Get an environment variable with a default value.
pub fn get_env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Get an environment variable and parse it, or use a default value.
pub fn get_env_parse_or<T: std::str::FromStr>(key: &str, default: T) -> T
where
    T::Err: std::fmt::Display,
{
    get_env_parse(key).unwrap_or(default)
}

/// Get a boolean environment variable.
pub fn get_env_bool(key: &str) -> Result<bool> {
    let value = get_env(key)?;
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(UtilsError::ConfigError(format!(
            "Invalid boolean value for '{}': {}",
            key, value
        ))),
    }
}

/// Get a boolean environment variable with a default.
pub fn get_env_bool_or(key: &str, default: bool) -> bool {
    get_env_bool(key).unwrap_or(default)
}

/// Load configuration from a YAML file.
pub fn load_yaml_config<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let content = std::fs::read_to_string(path.as_ref())?;
    serde_yaml::from_str(&content).map_err(|e| {
        UtilsError::ConfigError(format!(
            "Failed to parse YAML config from {:?}: {}",
            path.as_ref(),
            e
        ))
    })
}

/// Load configuration from a JSON file.
pub fn load_json_config<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let content = std::fs::read_to_string(path.as_ref())?;
    serde_json::from_str(&content).map_err(|e| {
        UtilsError::ConfigError(format!(
            "Failed to parse JSON config from {:?}: {}",
            path.as_ref(),
            e
        ))
    })
}

/// Load configuration from a file (auto-detect format from extension).
pub fn load_config_file<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| {
            UtilsError::ConfigError(format!("Unable to determine file extension for {:?}", path))
        })?;

    match extension.to_lowercase().as_str() {
        "yaml" | "yml" => load_yaml_config(path),
        "json" => load_json_config(path),
        _ => Err(UtilsError::ConfigError(format!(
            "Unsupported config file extension: {}",
            extension
        ))),
    }
}

/// Trait for types that can be loaded from environment variables.
pub trait FromEnv: Sized {
    /// Load configuration from environment variables with the given prefix.
    fn from_env(prefix: &str) -> Result<Self>;
}

/// Trait for validating configuration.
pub trait ValidateConfig {
    /// Validate the configuration, returning an error if invalid.
    fn validate(&self) -> Result<()>;
}

/// Configuration builder for fluent configuration construction.
pub struct ConfigBuilder<T> {
    config: T,
}

impl<T: Default> ConfigBuilder<T> {
    /// Create a new configuration builder with default values.
    pub fn new() -> Self {
        Self {
            config: T::default(),
        }
    }

    /// Build the configuration.
    pub fn build(self) -> T {
        self.config
    }
}

impl<T: Default> Default for ConfigBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to merge two configuration objects.
///
/// This is a placeholder for more sophisticated merging logic.
/// In practice, you might want to use a library like `merge` or implement
/// custom merge logic for your specific config types.
pub fn merge_configs<T: Clone>(base: T, _overlay: T) -> T {
    // Simplified merge - in a real implementation, you'd merge fields
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_env_or() {
        let value = get_env_or("NONEXISTENT_VAR_12345", "default");
        assert_eq!(value, "default");
    }

    #[test]
    fn test_get_env_parse_or() {
        let value = get_env_parse_or("NONEXISTENT_VAR_12345", 42);
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_env_bool() {
        env::set_var("TEST_BOOL_TRUE", "true");
        env::set_var("TEST_BOOL_FALSE", "false");
        env::set_var("TEST_BOOL_1", "1");
        env::set_var("TEST_BOOL_0", "0");

        assert!(get_env_bool("TEST_BOOL_TRUE").unwrap());
        assert!(!get_env_bool("TEST_BOOL_FALSE").unwrap());
        assert!(get_env_bool("TEST_BOOL_1").unwrap());
        assert!(!get_env_bool("TEST_BOOL_0").unwrap());
    }

    #[test]
    fn test_get_env_bool_or() {
        let value = get_env_bool_or("NONEXISTENT_BOOL_VAR", true);
        assert!(value);

        let value = get_env_bool_or("NONEXISTENT_BOOL_VAR", false);
        assert!(!value);
    }
}

