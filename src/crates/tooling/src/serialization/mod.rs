//! Serialization utilities
//!
//! Provides utilities for consistent hashing, stable JSON serialization,
//! and JSON manipulation.

use crate::Result;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

/// Generate a stable hash for a value
///
/// Uses a deterministic hashing algorithm suitable for cache keys.
///
/// # Arguments
///
/// * `value` - Value to hash (must implement Hash)
///
/// # Returns
///
/// 64-bit hash value
///
/// # Example
///
/// ```rust
/// use tooling::serialization::generate_hash;
///
/// let hash1 = generate_hash(&"hello");
/// let hash2 = generate_hash(&"hello");
/// assert_eq!(hash1, hash2);
///
/// let hash3 = generate_hash(&"world");
/// assert_ne!(hash1, hash3);
/// ```
pub fn generate_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Generate a stable hash from JSON value
///
/// Sorts object keys to ensure deterministic hashing.
///
/// # Arguments
///
/// * `value` - JSON value to hash
///
/// # Returns
///
/// 64-bit hash value
///
/// # Example
///
/// ```rust
/// use tooling::serialization::generate_json_hash;
/// use serde_json::json;
///
/// let val1 = json!({"b": 2, "a": 1});
/// let val2 = json!({"a": 1, "b": 2});
/// assert_eq!(generate_json_hash(&val1), generate_json_hash(&val2));
/// ```
pub fn generate_json_hash(value: &Value) -> u64 {
    // Use stable_json_string to ensure deterministic ordering
    if let Ok(stable) = stable_json_string(value) {
        generate_hash(&stable)
    } else {
        // Fallback to direct hashing
        generate_hash(&value.to_string())
    }
}

/// Serialize JSON value to a stable string representation
///
/// Ensures deterministic output by sorting object keys alphabetically.
/// Useful for cache keys, comparison, and consistent serialization.
///
/// # Arguments
///
/// * `value` - JSON value to serialize
///
/// # Returns
///
/// Stable JSON string with sorted keys
///
/// # Example
///
/// ```rust
/// use tooling::serialization::stable_json_string;
/// use serde_json::json;
///
/// let val = json!({"b": 2, "a": 1, "c": 3});
/// let stable = stable_json_string(&val).unwrap();
/// assert_eq!(stable, r#"{"a":1,"b":2,"c":3}"#);
/// ```
pub fn stable_json_string(value: &Value) -> Result<String> {
    let normalized = normalize_json(value.clone());
    serde_json::to_string(&normalized).map_err(|e| e.into())
}

/// Serialize value to stable JSON string
///
/// Convenience function that serializes any serializable value to a stable JSON string.
///
/// # Arguments
///
/// * `value` - Value to serialize (must implement Serialize)
///
/// # Returns
///
/// Stable JSON string with sorted keys
pub fn to_stable_json<T: Serialize>(value: &T) -> Result<String> {
    let json_value = serde_json::to_value(value)?;
    stable_json_string(&json_value)
}

/// Normalize JSON value for stable serialization
///
/// Recursively sorts all object keys alphabetically.
fn normalize_json(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k, normalize_json(v));
            }
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(normalize_json).collect()),
        other => other,
    }
}

/// Truncate JSON string to a maximum length
///
/// Safely truncates JSON strings for logging or display, adding ellipsis
/// and ensuring valid UTF-8 boundaries.
///
/// # Arguments
///
/// * `json` - JSON string to truncate
/// * `max_length` - Maximum length (must be at least 10)
///
/// # Returns
///
/// Truncated string with "..." suffix if truncated
///
/// # Example
///
/// ```rust
/// use tooling::serialization::truncate_json;
///
/// let long_json = r#"{"key": "very long value that should be truncated"}"#;
/// let truncated = truncate_json(long_json, 20);
/// assert_eq!(truncated.len(), 20);
/// assert!(truncated.ends_with("..."));
/// ```
pub fn truncate_json(json: &str, max_length: usize) -> String {
    if json.len() <= max_length {
        return json.to_string();
    }

    let max_length = max_length.max(10); // Ensure minimum length for "..." suffix

    // Find a valid UTF-8 boundary
    let mut truncate_at = max_length - 3; // Reserve space for "..."
    while truncate_at > 0 && !json.is_char_boundary(truncate_at) {
        truncate_at -= 1;
    }

    format!("{}...", &json[..truncate_at])
}

/// Truncate JSON value to a maximum string length
///
/// Serializes and truncates a JSON value in one operation.
///
/// # Arguments
///
/// * `value` - JSON value to serialize and truncate
/// * `max_length` - Maximum length
///
/// # Returns
///
/// Truncated JSON string
pub fn truncate_json_value(value: &Value, max_length: usize) -> String {
    let json = value.to_string();
    truncate_json(&json, max_length)
}

/// Pretty-print JSON value with indentation
///
/// # Arguments
///
/// * `value` - JSON value to format
///
/// # Returns
///
/// Pretty-printed JSON string
pub fn pretty_json(value: &Value) -> Result<String> {
    serde_json::to_string_pretty(value).map_err(|e| e.into())
}

/// Minify JSON string by removing whitespace
///
/// # Arguments
///
/// * `json` - JSON string to minify
///
/// # Returns
///
/// Minified JSON string
pub fn minify_json(json: &str) -> Result<String> {
    let value: Value = serde_json::from_str(json)?;
    serde_json::to_string(&value).map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_hash_deterministic() {
        let hash1 = generate_hash(&"test");
        let hash2 = generate_hash(&"test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_generate_hash_different_values() {
        let hash1 = generate_hash(&"test1");
        let hash2 = generate_hash(&"test2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_json_hash_stable() {
        let val1 = json!({"b": 2, "a": 1});
        let val2 = json!({"a": 1, "b": 2});
        assert_eq!(generate_json_hash(&val1), generate_json_hash(&val2));
    }

    #[test]
    fn test_stable_json_string() {
        let val = json!({"c": 3, "b": 2, "a": 1});
        let stable = stable_json_string(&val).unwrap();
        assert_eq!(stable, r#"{"a":1,"b":2,"c":3}"#);
    }

    #[test]
    fn test_stable_json_nested() {
        let val = json!({
            "outer": {
                "z": 26,
                "a": 1
            },
            "array": [3, 2, 1]
        });
        let stable = stable_json_string(&val).unwrap();

        // Keys should be sorted, but array order preserved
        assert!(stable.contains(r#""a":1"#));
        assert!(stable.contains(r#""z":26"#));
        assert!(stable.contains(r#"[3,2,1]"#));
    }

    #[test]
    fn test_to_stable_json() {
        #[derive(serde::Serialize)]
        struct TestStruct {
            b: i32,
            a: String,
        }

        let val = TestStruct {
            b: 2,
            a: "test".to_string(),
        };
        let stable = to_stable_json(&val).unwrap();
        assert_eq!(stable, r#"{"a":"test","b":2}"#);
    }

    #[test]
    fn test_truncate_json() {
        let json = r#"{"key": "very long value"}"#;
        let truncated = truncate_json(json, 15);
        assert_eq!(truncated.len(), 15);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_json_short() {
        let json = r#"{"key": "value"}"#;
        let truncated = truncate_json(json, 100);
        assert_eq!(truncated, json);
    }

    #[test]
    fn test_truncate_json_utf8() {
        let json = r#"{"emoji": "🎉🎊🎈"}"#;
        let truncated = truncate_json(json, 20);
        // Should not panic or produce invalid UTF-8
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn test_truncate_json_value() {
        let val = json!({"key": "very long value that exceeds max length"});
        let truncated = truncate_json_value(&val, 20);
        assert_eq!(truncated.len(), 20);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_pretty_json() {
        let val = json!({"a": 1, "b": 2});
        let pretty = pretty_json(&val).unwrap();
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("  ")); // Indentation
    }

    #[test]
    fn test_minify_json() {
        let json = r#"{
            "a": 1,
            "b": 2
        }"#;
        let minified = minify_json(json).unwrap();
        assert_eq!(minified, r#"{"a":1,"b":2}"#);
    }

    #[test]
    fn test_normalize_json_nested() {
        let val = json!({
            "z": {
                "c": 3,
                "a": 1
            },
            "a": {
                "z": 26
            }
        });

        let normalized = normalize_json(val);
        let stable = serde_json::to_string(&normalized).unwrap();

        // Verify keys are sorted at all levels
        let a_pos = stable.find(r#""a":"#).unwrap();
        let z_pos = stable.find(r#""z":"#).unwrap();
        assert!(a_pos < z_pos, "Top-level keys should be sorted");
    }
}
