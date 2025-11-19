//! rToon - TOON format serialization library
//!
//! TOON is a human-readable data format optimized for configuration and data exchange.
//! This crate provides encoding and decoding functionality for the TOON format.
//!
//! # Example
//!
//! ```rust
//! use rtoon::{encode, decode, EncodeOptions, DecodeOptions};
//! use serde_json::json;
//!
//! // Encode a JSON value to TOON format
//! let value = json!({
//!     "name": "Alice",
//!     "age": 30
//! });
//!
//! let toon_string = encode(&value, None);
//! println!("{}", toon_string);
//! // name: Alice
//! // age: 30
//!
//! // Decode a TOON string back to JSON
//! let decoded = decode(&toon_string, None).unwrap();
//! assert_eq!(decoded, value);
//! ```

pub mod constants;
pub mod decode;
pub mod encode;
pub mod shared;
pub mod types;

pub use constants::{Delimiter, DEFAULT_DELIMITER};
pub use types::{
    DecodeOptions, EncodeOptions, KeyFolding, PathExpansion, ToonError, ToonResult,
};

use decode::{expand_paths_safe, to_parsed_lines, LineCursor};
use encode::{encode_value, normalize_value};
use serde_json::Value as JsonValue;

/// Encode a JSON value to TOON format string
///
/// # Arguments
///
/// * `input` - A JSON value to encode
/// * `options` - Optional encoding configuration
///
/// # Returns
///
/// A TOON formatted string
///
/// # Example
///
/// ```rust
/// use rtoon::encode;
/// use serde_json::json;
///
/// let value = json!({"name": "Alice", "age": 30});
/// let toon = encode(&value, None);
/// ```
pub fn encode(input: &JsonValue, options: Option<EncodeOptions>) -> String {
    let normalized_value = normalize_value(input.clone());
    let resolved_options = options.unwrap_or_default();
    encode_value(&normalized_value, &resolved_options)
}

/// Decode a TOON format string to a JSON value
///
/// # Arguments
///
/// * `input` - A TOON formatted string
/// * `options` - Optional decoding configuration
///
/// # Returns
///
/// A parsed JSON value, or an error if decoding fails
///
/// # Example
///
/// ```rust
/// use rtoon::decode;
///
/// let toon = "name: Alice\nage: 30";
/// let value = decode(toon, None).unwrap();
/// ```
pub fn decode(input: &str, options: Option<DecodeOptions>) -> ToonResult<JsonValue> {
    let resolved_options = options.unwrap_or_default();
    let scan_result = to_parsed_lines(input, resolved_options.indent, resolved_options.strict)?;

    if scan_result.lines.is_empty() {
        return Ok(JsonValue::Object(serde_json::Map::new()));
    }

    let mut cursor = LineCursor::new(scan_result.lines, scan_result.blank_lines);
    let decoded_value = decode::decode_value_from_lines(&mut cursor, &resolved_options)?;

    // Apply path expansion if enabled
    if resolved_options.expand_paths == PathExpansion::Safe {
        return expand_paths_safe(decoded_value, resolved_options.strict);
    }

    Ok(decoded_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encode_simple_object() {
        let value = json!({
            "name": "Alice",
            "age": 30
        });
        let result = encode(&value, None);
        assert!(result.contains("name: Alice"));
        assert!(result.contains("age: 30"));
    }

    #[test]
    fn test_decode_simple_object() {
        let input = "name: Alice\nage: 30";
        let result = decode(input, None).unwrap();
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 30);
    }

    #[test]
    fn test_roundtrip() {
        let original = json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ]
        });
        let encoded = encode(&original, None);
        let decoded = decode(&encoded, None).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_empty_input() {
        let result = decode("", None).unwrap();
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_primitive_values() {
        let value = json!(42);
        let encoded = encode(&value, None);
        assert_eq!(encoded, "42");

        let value = json!("hello");
        let encoded = encode(&value, None);
        assert_eq!(encoded, "hello");

        let value = json!(true);
        let encoded = encode(&value, None);
        assert_eq!(encoded, "true");

        let value = json!(null);
        let encoded = encode(&value, None);
        assert_eq!(encoded, "null");
    }
}
