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
    use serde::Deserialize;
    use std::env;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // ========================================================================
    // Phase 9.2: Utils Config Loading Tests
    // ========================================================================

    // Test config structure
    #[derive(Debug, Deserialize, PartialEq)]
    struct TestConfig {
        name: String,
        port: u16,
        enabled: bool,
    }

    // ------------------------------------------------------------------------
    // Environment Variable Loading Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_get_env_or() {
        let value = get_env_or("NONEXISTENT_VAR_12345", "default");
        assert_eq!(value, "default");
    }

    #[test]
    fn test_get_env_existing() {
        env::set_var("TEST_EXISTING_VAR", "test_value");
        let value = get_env("TEST_EXISTING_VAR").unwrap();
        assert_eq!(value, "test_value");
        env::remove_var("TEST_EXISTING_VAR");
    }

    #[test]
    fn test_get_env_missing() {
        let result = get_env("DEFINITELY_NONEXISTENT_VAR_XYZ123");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_env_parse_or() {
        let value = get_env_parse_or("NONEXISTENT_VAR_12345", 42);
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_env_parse_integer() {
        env::set_var("TEST_INT_VAR", "12345");
        let value: i32 = get_env_parse("TEST_INT_VAR").unwrap();
        assert_eq!(value, 12345);
        env::remove_var("TEST_INT_VAR");
    }

    #[test]
    fn test_get_env_parse_float() {
        env::set_var("TEST_FLOAT_VAR", "3.14159");
        let value: f64 = get_env_parse("TEST_FLOAT_VAR").unwrap();
        assert!((value - 3.14159).abs() < 0.00001);
        env::remove_var("TEST_FLOAT_VAR");
    }

    #[test]
    fn test_get_env_parse_invalid() {
        env::set_var("TEST_INVALID_INT", "not_a_number");
        let result: Result<i32> = get_env_parse("TEST_INVALID_INT");
        assert!(result.is_err());
        env::remove_var("TEST_INVALID_INT");
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

        env::remove_var("TEST_BOOL_TRUE");
        env::remove_var("TEST_BOOL_FALSE");
        env::remove_var("TEST_BOOL_1");
        env::remove_var("TEST_BOOL_0");
    }

    #[test]
    fn test_get_env_bool_yes_no() {
        env::set_var("TEST_BOOL_YES", "yes");
        env::set_var("TEST_BOOL_NO", "no");

        assert!(get_env_bool("TEST_BOOL_YES").unwrap());
        assert!(!get_env_bool("TEST_BOOL_NO").unwrap());

        env::remove_var("TEST_BOOL_YES");
        env::remove_var("TEST_BOOL_NO");
    }

    #[test]
    fn test_get_env_bool_on_off() {
        env::set_var("TEST_BOOL_ON", "on");
        env::set_var("TEST_BOOL_OFF", "off");

        assert!(get_env_bool("TEST_BOOL_ON").unwrap());
        assert!(!get_env_bool("TEST_BOOL_OFF").unwrap());

        env::remove_var("TEST_BOOL_ON");
        env::remove_var("TEST_BOOL_OFF");
    }

    #[test]
    fn test_get_env_bool_case_insensitive() {
        env::set_var("TEST_BOOL_UPPER", "TRUE");
        env::set_var("TEST_BOOL_MIXED", "FaLsE");

        assert!(get_env_bool("TEST_BOOL_UPPER").unwrap());
        assert!(!get_env_bool("TEST_BOOL_MIXED").unwrap());

        env::remove_var("TEST_BOOL_UPPER");
        env::remove_var("TEST_BOOL_MIXED");
    }

    #[test]
    fn test_get_env_bool_invalid() {
        env::set_var("TEST_BOOL_INVALID", "maybe");
        let result = get_env_bool("TEST_BOOL_INVALID");
        assert!(result.is_err());
        env::remove_var("TEST_BOOL_INVALID");
    }

    #[test]
    fn test_get_env_bool_or() {
        let value = get_env_bool_or("NONEXISTENT_BOOL_VAR", true);
        assert!(value);

        let value = get_env_bool_or("NONEXISTENT_BOOL_VAR", false);
        assert!(!value);
    }

    // ------------------------------------------------------------------------
    // YAML Config Loading Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_load_yaml_config_valid() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
name: test_app
port: 8080
enabled: true
"#;

        fs::write(&config_path, yaml_content).unwrap();

        let config: TestConfig = load_yaml_config(&config_path).unwrap();
        assert_eq!(config.name, "test_app");
        assert_eq!(config.port, 8080);
        assert!(config.enabled);
    }

    #[test]
    fn test_load_yaml_config_malformed() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("bad.yaml");

        let bad_yaml = r#"
name: test_app
port: [invalid yaml structure
enabled: true
"#;

        fs::write(&config_path, bad_yaml).unwrap();

        let result: Result<TestConfig> = load_yaml_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_yaml_config_file_not_found() {
        let result: Result<TestConfig> = load_yaml_config("/nonexistent/path/config.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_yaml_config_wrong_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("wrong.yaml");

        // Valid YAML but wrong structure for TestConfig
        let yaml_content = r#"
different_field: value
another_field: 123
"#;

        fs::write(&config_path, yaml_content).unwrap();

        let result: Result<TestConfig> = load_yaml_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_yaml_config_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("empty.yaml");

        fs::write(&config_path, "").unwrap();

        let result: Result<TestConfig> = load_yaml_config(&config_path);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // JSON Config Loading Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_load_json_config_valid() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let json_content = r#"{
  "name": "test_app",
  "port": 8080,
  "enabled": true
}"#;

        fs::write(&config_path, json_content).unwrap();

        let config: TestConfig = load_json_config(&config_path).unwrap();
        assert_eq!(config.name, "test_app");
        assert_eq!(config.port, 8080);
        assert!(config.enabled);
    }

    #[test]
    fn test_load_json_config_malformed() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("bad.json");

        let bad_json = r#"{
  "name": "test_app",
  "port": 8080,
  "enabled": true
  // missing closing brace
"#;

        fs::write(&config_path, bad_json).unwrap();

        let result: Result<TestConfig> = load_json_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_json_config_file_not_found() {
        let result: Result<TestConfig> = load_json_config("/nonexistent/path/config.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_json_config_wrong_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("wrong.json");

        // Valid JSON but wrong structure
        let json_content = r#"{
  "wrong_field": "value",
  "another_field": 123
}"#;

        fs::write(&config_path, json_content).unwrap();

        let result: Result<TestConfig> = load_json_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_json_config_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("empty.json");

        fs::write(&config_path, "").unwrap();

        let result: Result<TestConfig> = load_json_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_json_config_trailing_comma() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("trailing.json");

        // JSON with trailing comma (invalid)
        let json_content = r#"{
  "name": "test",
  "port": 8080,
  "enabled": true,
}"#;

        fs::write(&config_path, json_content).unwrap();

        let result: Result<TestConfig> = load_json_config(&config_path);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Auto-detect Config File Format Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_load_config_file_yaml_extension() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
name: yaml_test
port: 9000
enabled: false
"#;

        fs::write(&config_path, yaml_content).unwrap();

        let config: TestConfig = load_config_file(&config_path).unwrap();
        assert_eq!(config.name, "yaml_test");
        assert_eq!(config.port, 9000);
        assert!(!config.enabled);
    }

    #[test]
    fn test_load_config_file_yml_extension() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yml");

        let yaml_content = r#"
name: yml_test
port: 9001
enabled: true
"#;

        fs::write(&config_path, yaml_content).unwrap();

        let config: TestConfig = load_config_file(&config_path).unwrap();
        assert_eq!(config.name, "yml_test");
    }

    #[test]
    fn test_load_config_file_json_extension() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let json_content = r#"{
  "name": "json_test",
  "port": 9002,
  "enabled": true
}"#;

        fs::write(&config_path, json_content).unwrap();

        let config: TestConfig = load_config_file(&config_path).unwrap();
        assert_eq!(config.name, "json_test");
    }

    #[test]
    fn test_load_config_file_unsupported_extension() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        fs::write(&config_path, "name = 'test'").unwrap();

        let result: Result<TestConfig> = load_config_file(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[test]
    fn test_load_config_file_no_extension() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config");

        fs::write(&config_path, "{}").unwrap();

        let result: Result<TestConfig> = load_config_file(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("extension"));
    }

    #[test]
    fn test_load_config_file_case_insensitive_extension() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.YAML");

        let yaml_content = r#"
name: uppercase_ext
port: 9003
enabled: true
"#;

        fs::write(&config_path, yaml_content).unwrap();

        let config: TestConfig = load_config_file(&config_path).unwrap();
        assert_eq!(config.name, "uppercase_ext");
    }

    // ------------------------------------------------------------------------
    // Invalid File Path Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_load_config_invalid_path() {
        let result: Result<TestConfig> = load_config_file("/invalid/path/to/config.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_directory_path() {
        let temp_dir = TempDir::new().unwrap();

        // Try to load a directory as a config file
        let result: Result<TestConfig> = load_config_file(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    #[ignore] // Platform-specific, may not work in all environments (e.g., containers)
    #[cfg(unix)]
    fn test_load_config_permission_denied() {
        // This test is platform-specific and may not work on all systems
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("no_read.yaml");

        fs::write(&config_path, "name: test\nport: 8080\nenabled: true").unwrap();

        // Remove read permissions
        let mut perms = fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&config_path, perms).unwrap();

        let result: Result<TestConfig> = load_config_file(&config_path);
        assert!(result.is_err());

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&config_path, perms).unwrap();
    }

    // ------------------------------------------------------------------------
    // Edge Cases and Special Scenarios
    // ------------------------------------------------------------------------

    #[test]
    fn test_load_config_unicode_content() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("unicode.yaml");

        #[derive(Debug, Deserialize, PartialEq)]
        struct UnicodeConfig {
            message: String,
        }

        let yaml_content = r#"
message: "Hello 世界 🌍 مرحبا"
"#;

        fs::write(&config_path, yaml_content).unwrap();

        let config: UnicodeConfig = load_yaml_config(&config_path).unwrap();
        assert!(config.message.contains("世界"));
        assert!(config.message.contains("🌍"));
    }

    #[test]
    fn test_load_config_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("large.json");

        #[derive(Debug, Deserialize)]
        struct LargeConfig {
            items: Vec<String>,
        }

        // Create a large config with many items
        let mut items = Vec::new();
        for i in 0..1000 {
            items.push(format!("item_{}", i));
        }

        let json_content = serde_json::to_string(&serde_json::json!({
            "items": items
        })).unwrap();

        fs::write(&config_path, json_content).unwrap();

        let config: LargeConfig = load_json_config(&config_path).unwrap();
        assert_eq!(config.items.len(), 1000);
    }

    #[test]
    fn test_env_var_with_special_characters() {
        env::set_var("TEST_SPECIAL_CHARS", "value with spaces & symbols!");
        let value = get_env("TEST_SPECIAL_CHARS").unwrap();
        assert_eq!(value, "value with spaces & symbols!");
        env::remove_var("TEST_SPECIAL_CHARS");
    }

    #[test]
    fn test_env_var_empty_string() {
        env::set_var("TEST_EMPTY_VAR", "");
        let value = get_env("TEST_EMPTY_VAR").unwrap();
        assert_eq!(value, "");
        env::remove_var("TEST_EMPTY_VAR");
    }

    #[test]
    fn test_config_builder_default() {
        #[derive(Default)]
        struct BuilderTestConfig {
            value: i32,
        }

        let config = ConfigBuilder::<BuilderTestConfig>::new().build();
        assert_eq!(config.value, 0);
    }

    #[test]
    fn test_merge_configs_basic() {
        #[derive(Clone)]
        struct MergeConfig {
            field: String,
        }

        let base = MergeConfig {
            field: "base".to_string(),
        };
        let overlay = MergeConfig {
            field: "overlay".to_string(),
        };

        let merged = merge_configs(base, overlay);
        // Current implementation returns base
        assert_eq!(merged.field, "base");
    }
}

