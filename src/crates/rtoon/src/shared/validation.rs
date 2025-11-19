//! Validation utilities

use crate::constants::LIST_ITEM_MARKER;
use crate::shared::literal_utils::is_boolean_or_null_literal;
use regex::Regex;
use std::sync::LazyLock;

static UNQUOTED_KEY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_][\w.]*$").unwrap());

static IDENTIFIER_SEGMENT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_]\w*$").unwrap());

static NUMERIC_LIKE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?$").unwrap());

static LEADING_ZERO_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0\d+$").unwrap());

/// Checks if a key can be used without quotes
pub fn is_valid_unquoted_key(key: &str) -> bool {
    UNQUOTED_KEY_REGEX.is_match(key)
}

/// Checks if a key segment is a valid identifier for safe folding/expansion
pub fn is_identifier_segment(key: &str) -> bool {
    IDENTIFIER_SEGMENT_REGEX.is_match(key)
}

/// Determines if a string value can be safely encoded without quotes
pub fn is_safe_unquoted(value: &str, delimiter: char) -> bool {
    if value.is_empty() {
        return false;
    }

    if value != value.trim() {
        return false;
    }

    // Check if it looks like any literal value (boolean, null, or numeric)
    if is_boolean_or_null_literal(value) || is_numeric_like(value) {
        return false;
    }

    // Check for colon (always structural)
    if value.contains(':') {
        return false;
    }

    // Check for quotes and backslash (always need escaping)
    if value.contains('"') || value.contains('\\') {
        return false;
    }

    // Check for brackets and braces (always structural)
    if value.contains('[')
        || value.contains(']')
        || value.contains('{')
        || value.contains('}')
    {
        return false;
    }

    // Check for control characters (newline, carriage return, tab)
    if value.contains('\n') || value.contains('\r') || value.contains('\t') {
        return false;
    }

    // Check for the active delimiter
    if value.contains(delimiter) {
        return false;
    }

    // Check for hyphen at start (list marker)
    if value.starts_with(LIST_ITEM_MARKER) {
        return false;
    }

    true
}

/// Checks if a string looks like a number
fn is_numeric_like(value: &str) -> bool {
    NUMERIC_LIKE_REGEX.is_match(value) || LEADING_ZERO_REGEX.is_match(value)
}
