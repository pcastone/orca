//! Primitive encoding utilities

use crate::constants::{DEFAULT_DELIMITER, DOUBLE_QUOTE, NULL_LITERAL};
use crate::shared::{escape_string, is_safe_unquoted, is_valid_unquoted_key};
use serde_json::Value as JsonValue;

/// Encode a primitive value to TOON format
pub fn encode_primitive(value: &JsonValue, delimiter: char) -> String {
    match value {
        JsonValue::Null => NULL_LITERAL.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => encode_string_literal(s, delimiter),
        _ => panic!("encode_primitive called with non-primitive value"),
    }
}

/// Encode a string literal, quoting if necessary
pub fn encode_string_literal(value: &str, delimiter: char) -> String {
    if is_safe_unquoted(value, delimiter) {
        value.to_string()
    } else {
        format!("{}{}{}", DOUBLE_QUOTE, escape_string(value), DOUBLE_QUOTE)
    }
}

/// Encode a key, quoting if necessary
pub fn encode_key(key: &str) -> String {
    if is_valid_unquoted_key(key) {
        key.to_string()
    } else {
        format!("{}{}{}", DOUBLE_QUOTE, escape_string(key), DOUBLE_QUOTE)
    }
}

/// Encode and join primitives with a delimiter
pub fn encode_and_join_primitives(values: &[JsonValue], delimiter: char) -> String {
    values
        .iter()
        .map(|v| encode_primitive(v, delimiter))
        .collect::<Vec<_>>()
        .join(&delimiter.to_string())
}

/// Format an array header
pub fn format_header(
    length: usize,
    key: Option<&str>,
    fields: Option<&[String]>,
    delimiter: char,
) -> String {
    let mut header = String::new();

    if let Some(k) = key {
        header.push_str(&encode_key(k));
    }

    // Only include delimiter if it's not the default (comma)
    let delimiter_suffix = if delimiter != DEFAULT_DELIMITER.as_char() {
        delimiter.to_string()
    } else {
        String::new()
    };

    header.push_str(&format!("[{}{}]", length, delimiter_suffix));

    if let Some(f) = fields {
        let quoted_fields: Vec<String> = f.iter().map(|field| encode_key(field)).collect();
        header.push_str(&format!("{{{}}}", quoted_fields.join(&delimiter.to_string())));
    }

    header.push(':');
    header
}
