//! Configuration builder trait and helpers
//!
//! Provides a common trait for configuration structures to implement,
//! enabling consistent patterns for validation, environment variable loading,
//! and configuration merging across the workspace.

use crate::Result;

/// Trait for configuration structures that support building, validation, and merging
///
/// Implementing this trait provides a consistent API for:
/// - Default configuration creation
/// - Validation of configuration values
/// - Loading from environment variables
/// - Merging multiple configuration sources
///
/// # Example
///
/// ```rust,ignore
/// use tooling::config::ConfigBuilder;
///
/// #[derive(Clone, Default)]
/// struct MyConfig {
///     pub port: u16,
///     pub host: String,
/// }
///
/// impl ConfigBuilder for MyConfig {
///     fn validate(&self) -> tooling::Result<()> {
///         if self.port == 0 {
///             return Err(tooling::ToolingError::General(
///                 "Port must be non-zero".to_string()
///             ));
///         }
///         Ok(())
///     }
///
///     fn from_env(prefix: &str) -> tooling::Result<Self> {
///         // Load configuration from environment variables
///         Ok(Self::default())
///     }
///
///     fn merge(&mut self, other: Self) -> &mut Self {
///         // Merge other config into self
///         self
///     }
/// }
/// ```
pub trait ConfigBuilder: Default + Clone {
    /// Validate the configuration
    ///
    /// Returns an error if the configuration is invalid.
    /// Should check for:
    /// - Required fields being set
    /// - Values being within valid ranges
    /// - Dependencies between fields
    fn validate(&self) -> Result<()> {
        // Default implementation: always valid
        Ok(())
    }

    /// Load configuration from environment variables
    ///
    /// # Arguments
    ///
    /// * `prefix` - Prefix for environment variable names (e.g., "APP_" for APP_PORT)
    ///
    /// Environment variables should follow the pattern: `{PREFIX}{FIELD_NAME}`
    /// where FIELD_NAME is the uppercased field name.
    ///
    /// # Example
    ///
    /// For a config with field `port` and prefix "APP_":
    /// - Environment variable: `APP_PORT`
    /// - Value: "8080"
    fn from_env(prefix: &str) -> Result<Self>;

    /// Merge another configuration into this one
    ///
    /// Allows combining configurations from multiple sources.
    /// The general strategy is:
    /// - Option fields: `other` value overwrites if Some
    /// - Vec fields: `other` values are appended
    /// - Scalar fields: `other` value overwrites
    ///
    /// Returns self for chaining.
    fn merge(&mut self, other: Self) -> &mut Self;

    /// Create, validate, and return configuration
    ///
    /// Helper method that creates a default config and validates it.
    fn build() -> Result<Self> {
        let config = Self::default();
        config.validate()?;
        Ok(config)
    }

    /// Load from environment, merge defaults, and validate
    ///
    /// Helper method that combines:
    /// 1. Load from environment variables
    /// 2. Merge with defaults
    /// 3. Validate final result
    fn from_env_with_defaults(prefix: &str) -> Result<Self> {
        let mut config = Self::from_env(prefix)?;
        let defaults = Self::default();
        config.merge(defaults);
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolingError;

    #[derive(Debug, Clone, Default, PartialEq)]
    struct TestConfig {
        value: Option<i32>,
        items: Vec<String>,
    }

    impl ConfigBuilder for TestConfig {
        fn validate(&self) -> Result<()> {
            if let Some(v) = self.value {
                if v < 0 {
                    return Err(ToolingError::General("Value must be non-negative".into()));
                }
            }
            Ok(())
        }

        fn from_env(_prefix: &str) -> Result<Self> {
            // Simple implementation for testing
            Ok(Self {
                value: Some(42),
                items: vec!["from_env".to_string()],
            })
        }

        fn merge(&mut self, other: Self) -> &mut Self {
            if other.value.is_some() {
                self.value = other.value;
            }
            self.items.extend(other.items);
            self
        }
    }

    #[test]
    fn test_validate_success() {
        let config = TestConfig {
            value: Some(10),
            items: vec![],
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_failure() {
        let config = TestConfig {
            value: Some(-5),
            items: vec![],
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_from_env() {
        let config = TestConfig::from_env("TEST_").unwrap();
        assert_eq!(config.value, Some(42));
        assert_eq!(config.items, vec!["from_env"]);
    }

    #[test]
    fn test_merge() {
        let mut config1 = TestConfig {
            value: Some(1),
            items: vec!["a".to_string()],
        };

        let config2 = TestConfig {
            value: Some(2),
            items: vec!["b".to_string()],
        };

        config1.merge(config2);

        assert_eq!(config1.value, Some(2));
        assert_eq!(config1.items, vec!["a", "b"]);
    }

    #[test]
    fn test_build() {
        let config = TestConfig::build().unwrap();
        assert_eq!(config, TestConfig::default());
    }

    #[test]
    fn test_from_env_with_defaults() {
        let config = TestConfig::from_env_with_defaults("TEST_").unwrap();
        // Should have value from from_env
        assert_eq!(config.value, Some(42));
        // Should have items from both
        assert!(config.items.contains(&"from_env".to_string()));
    }
}
