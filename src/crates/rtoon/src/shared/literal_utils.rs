//! Utilities for working with literal values

use crate::constants::{FALSE_LITERAL, NULL_LITERAL, TRUE_LITERAL};

/// Check if a token is a boolean or null literal
pub fn is_boolean_or_null_literal(token: &str) -> bool {
    token == TRUE_LITERAL || token == FALSE_LITERAL || token == NULL_LITERAL
}

/// Check if a token represents a valid numeric literal
///
/// Rejects numbers with leading zeros (except "0" itself or decimals like "0.5")
pub fn is_numeric_literal(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }

    // Must not have leading zeros (except for "0" itself or decimals like "0.5")
    let bytes = token.as_bytes();
    if bytes.len() > 1 && bytes[0] == b'0' && bytes[1] != b'.' {
        // Allow negative numbers with leading zero after the minus
        if !(bytes[0] == b'-' && bytes.len() > 2 && bytes[1] == b'0' && bytes[2] == b'.') {
            return false;
        }
    }

    // Handle negative numbers
    let token = if token.starts_with('-') {
        &token[1..]
    } else {
        token
    };

    // Must not have leading zeros after potential minus sign
    let bytes = token.as_bytes();
    if bytes.len() > 1 && bytes[0] == b'0' && bytes[1] != b'.' {
        return false;
    }

    // Try to parse as a number
    token.parse::<f64>().is_ok()
}
