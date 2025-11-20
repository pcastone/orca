//! String manipulation utilities

use crate::constants::{BACKSLASH, CARRIAGE_RETURN, DOUBLE_QUOTE, NEWLINE, TAB};

/// Escapes special characters in a string for encoding
pub fn escape_string(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(ch),
        }
    }
    result
}

/// Unescapes a string by processing escape sequences
pub fn unescape_string(value: &str) -> Result<String, String> {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == BACKSLASH {
            match chars.next() {
                Some('n') => result.push(NEWLINE),
                Some('t') => result.push(TAB),
                Some('r') => result.push(CARRIAGE_RETURN),
                Some('\\') => result.push(BACKSLASH),
                Some('"') => result.push(DOUBLE_QUOTE),
                Some(other) => {
                    return Err(format!("Invalid escape sequence: \\{}", other));
                }
                None => {
                    return Err("Invalid escape sequence: backslash at end of string".to_string());
                }
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

/// Finds the index of the closing double quote, accounting for escape sequences
pub fn find_closing_quote(content: &str, start: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut i = start + 1;

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            // Skip escaped character
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            return Some(i);
        }
        i += 1;
    }

    None
}

/// Finds the index of a character outside of quoted sections
pub fn find_unquoted_char(content: &str, ch: char, start: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let target = ch as u8;
    let mut in_quotes = false;
    let mut i = start;

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && in_quotes {
            // Skip escaped character
            i += 2;
            continue;
        }

        if bytes[i] == b'"' {
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }

        if bytes[i] == target && !in_quotes {
            return Some(i);
        }

        i += 1;
    }

    None
}
