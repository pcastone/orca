//! Environment variable loading utilities
//!
//! Helper functions for loading and parsing environment variables with proper error handling.

use crate::{Result, ToolingError};
use std::env;
use std::str::FromStr;

/// Load an environment variable as a string
///
/// # Arguments
///
/// * `key` - Environment variable name
///
/// # Returns
///
/// * `Ok(Some(value))` if variable exists
/// * `Ok(None)` if variable doesn't exist
/// * `Err` if variable exists but has invalid UTF-8
pub fn get_env(key: &str) -> Result<Option<String>> {
    match env::var(key) {
        Ok(val) => Ok(Some(val)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(ToolingError::General(format!(
            "Environment variable {} contains invalid UTF-8",
            key
        ))),
    }
}

/// Load and parse an environment variable
///
/// # Arguments
///
/// * `key` - Environment variable name
///
/// # Type Parameters
///
/// * `T` - Type to parse into (must implement FromStr)
///
/// # Returns
///
/// * `Ok(Some(value))` if variable exists and parses successfully
/// * `Ok(None)` if variable doesn't exist
/// * `Err` if variable exists but fails to parse
///
/// # Example
///
/// ```rust,ignore
/// let port: Option<u16> = get_env_parse("PORT")?;
/// let timeout: Option<u64> = get_env_parse("TIMEOUT_MS")?;
/// ```
pub fn get_env_parse<T>(key: &str) -> Result<Option<T>>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match get_env(key)? {
        Some(val) => {
            let parsed = val.parse::<T>().map_err(|e| {
                ToolingError::General(format!(
                    "Failed to parse environment variable {}: {}",
                    key, e
                ))
            })?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

/// Load an environment variable with a default value
///
/// # Arguments
///
/// * `key` - Environment variable name
/// * `default` - Default value if variable doesn't exist
///
/// # Returns
///
/// * `Ok(value)` if variable exists
/// * `Ok(default)` if variable doesn't exist
/// * `Err` if variable exists but has invalid UTF-8
pub fn get_env_or(key: &str, default: impl Into<String>) -> Result<String> {
    Ok(get_env(key)?.unwrap_or_else(|| default.into()))
}

/// Load and parse an environment variable with a default value
///
/// # Arguments
///
/// * `key` - Environment variable name
/// * `default` - Default value if variable doesn't exist or fails to parse
///
/// # Returns
///
/// The parsed value or the default
pub fn get_env_parse_or<T>(key: &str, default: T) -> Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    Ok(get_env_parse(key)?.unwrap_or(default))
}

/// Load a boolean environment variable
///
/// Recognizes: "true", "1", "yes", "on" (case-insensitive) as true
/// Recognizes: "false", "0", "no", "off" (case-insensitive) as false
///
/// # Arguments
///
/// * `key` - Environment variable name
///
/// # Returns
///
/// * `Ok(Some(bool))` if variable exists and is a valid boolean
/// * `Ok(None)` if variable doesn't exist
/// * `Err` if variable exists but is not a valid boolean
pub fn get_env_bool(key: &str) -> Result<Option<bool>> {
    match get_env(key)? {
        Some(val) => {
            let lower = val.to_lowercase();
            let result = match lower.as_str() {
                "true" | "1" | "yes" | "on" => true,
                "false" | "0" | "no" | "off" => false,
                _ => {
                    return Err(ToolingError::General(format!(
                        "Invalid boolean value for {}: {}",
                        key, val
                    )))
                }
            };
            Ok(Some(result))
        }
        None => Ok(None),
    }
}

/// Build a prefixed environment variable name
///
/// # Arguments
///
/// * `prefix` - Prefix to add
/// * `name` - Variable name (will be uppercased)
///
/// # Example
///
/// ```rust,ignore
/// let key = build_env_key("APP_", "port"); // Returns "APP_PORT"
/// ```
pub fn build_env_key(prefix: &str, name: &str) -> String {
    format!("{}{}", prefix, name.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_env_missing() {
        let result = get_env("TOOLING_TEST_MISSING_VAR_12345");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_get_env_or() {
        let result = get_env_or("TOOLING_TEST_MISSING_VAR_12345", "default").unwrap();
        assert_eq!(result, "default");
    }

    #[test]
    fn test_get_env_parse() {
        env::set_var("TOOLING_TEST_NUMBER", "42");
        let result: Option<i32> = get_env_parse("TOOLING_TEST_NUMBER").unwrap();
        assert_eq!(result, Some(42));
        env::remove_var("TOOLING_TEST_NUMBER");
    }

    #[test]
    fn test_get_env_parse_invalid() {
        env::set_var("TOOLING_TEST_INVALID_NUMBER", "not_a_number");
        let result: Result<Option<i32>> = get_env_parse("TOOLING_TEST_INVALID_NUMBER");
        assert!(result.is_err());
        env::remove_var("TOOLING_TEST_INVALID_NUMBER");
    }

    #[test]
    fn test_get_env_parse_or() {
        let result: i32 = get_env_parse_or("TOOLING_TEST_MISSING_VAR_12345", 99).unwrap();
        assert_eq!(result, 99);

        env::set_var("TOOLING_TEST_NUMBER_OR", "42");
        let result: i32 = get_env_parse_or("TOOLING_TEST_NUMBER_OR", 99).unwrap();
        assert_eq!(result, 42);
        env::remove_var("TOOLING_TEST_NUMBER_OR");
    }

    #[test]
    fn test_get_env_bool() {
        let test_cases = vec![
            ("true", true),
            ("TRUE", true),
            ("1", true),
            ("yes", true),
            ("on", true),
            ("false", false),
            ("FALSE", false),
            ("0", false),
            ("no", false),
            ("off", false),
        ];

        for (value, expected) in test_cases {
            env::set_var("TOOLING_TEST_BOOL", value);
            let result = get_env_bool("TOOLING_TEST_BOOL").unwrap();
            assert_eq!(result, Some(expected), "Failed for value: {}", value);
        }

        env::remove_var("TOOLING_TEST_BOOL");
    }

    #[test]
    fn test_get_env_bool_invalid() {
        env::set_var("TOOLING_TEST_BOOL_INVALID", "maybe");
        let result = get_env_bool("TOOLING_TEST_BOOL_INVALID");
        assert!(result.is_err());
        env::remove_var("TOOLING_TEST_BOOL_INVALID");
    }

    #[test]
    fn test_build_env_key() {
        assert_eq!(build_env_key("APP_", "port"), "APP_PORT");
        assert_eq!(build_env_key("", "debug"), "DEBUG");
        assert_eq!(build_env_key("MY_", "some_value"), "MY_SOME_VALUE");
    }
}
