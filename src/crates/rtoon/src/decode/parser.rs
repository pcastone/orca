//! TOON parsing utilities

use crate::constants::{
    BACKSLASH, CLOSE_BRACE, CLOSE_BRACKET, COLON, DOUBLE_QUOTE, FALSE_LITERAL, NULL_LITERAL,
    OPEN_BRACE, OPEN_BRACKET, PIPE, TAB, TRUE_LITERAL,
};
use crate::shared::{find_closing_quote, find_unquoted_char, is_boolean_or_null_literal, is_numeric_literal, unescape_string};
use crate::types::{ArrayHeaderInfo, Delimiter, ToonError, ToonResult};
use serde_json::Value as JsonValue;

/// Parse an array header line
pub fn parse_array_header_line(
    content: &str,
    default_delimiter: Delimiter,
) -> Option<(ArrayHeaderInfo, Option<String>)> {
    let trimmed = content.trim_start();

    // Find the bracket segment
    let bracket_start = if trimmed.starts_with(DOUBLE_QUOTE) {
        let closing_quote_index = find_closing_quote(trimmed, 0)?;
        let after_quote = &trimmed[closing_quote_index + 1..];
        if !after_quote.starts_with(OPEN_BRACKET) {
            return None;
        }
        let leading_whitespace = content.len() - trimmed.len();
        content[leading_whitespace + closing_quote_index + 1..]
            .find(OPEN_BRACKET)
            .map(|i| leading_whitespace + closing_quote_index + 1 + i)?
    } else {
        content.find(OPEN_BRACKET)?
    };

    let bracket_end = content[bracket_start..].find(CLOSE_BRACKET)? + bracket_start;

    // Find the colon after all brackets and braces
    let mut brace_end = bracket_end + 1;

    // Check for fields segment
    if let Some(brace_start) = content[bracket_end..].find(OPEN_BRACE) {
        let brace_start = brace_start + bracket_end;
        let colon_pos = content[bracket_end..].find(COLON).map(|i| i + bracket_end);
        if colon_pos.is_none() || brace_start < colon_pos.unwrap() {
            if let Some(found_brace_end) = content[brace_start..].find(CLOSE_BRACE) {
                brace_end = brace_start + found_brace_end + 1;
            }
        }
    }

    let colon_index = content[brace_end.max(bracket_end)..].find(COLON)? + brace_end.max(bracket_end);

    // Extract and parse the key
    let key = if bracket_start > 0 {
        let raw_key = content[..bracket_start].trim();
        if raw_key.starts_with(DOUBLE_QUOTE) {
            Some(parse_string_literal(raw_key).ok()?)
        } else {
            Some(raw_key.to_string())
        }
    } else {
        None
    };

    let after_colon = content[colon_index + 1..].trim();

    let bracket_content = &content[bracket_start + 1..bracket_end];

    // Parse bracket segment
    let (length, delimiter) = parse_bracket_segment(bracket_content, default_delimiter).ok()?;

    // Check for fields segment
    let fields = if let Some(brace_start) = content[bracket_end..colon_index].find(OPEN_BRACE) {
        let brace_start = brace_start + bracket_end;
        if let Some(found_brace_end) = content[brace_start..colon_index].find(CLOSE_BRACE) {
            let found_brace_end = found_brace_end + brace_start;
            let fields_content = &content[brace_start + 1..found_brace_end];
            let fields: Result<Vec<String>, _> = parse_delimited_values(fields_content, delimiter)
                .iter()
                .map(|field| parse_string_literal(field.trim()))
                .collect();
            Some(fields.ok()?)
        } else {
            None
        }
    } else {
        None
    };

    let inline_values = if after_colon.is_empty() {
        None
    } else {
        Some(after_colon.to_string())
    };

    Some((
        ArrayHeaderInfo {
            key,
            length,
            delimiter,
            fields,
        },
        inline_values,
    ))
}

/// Parse a bracket segment to extract length and delimiter
pub fn parse_bracket_segment(
    seg: &str,
    default_delimiter: Delimiter,
) -> ToonResult<(usize, Delimiter)> {
    let mut content = seg;

    // Check for delimiter suffix
    let delimiter = if content.ends_with(TAB) {
        content = &content[..content.len() - 1];
        Delimiter::Tab
    } else if content.ends_with(PIPE) {
        content = &content[..content.len() - 1];
        Delimiter::Pipe
    } else {
        default_delimiter
    };

    let length = content
        .parse::<usize>()
        .map_err(|_| ToonError::TypeError(format!("Invalid array length: {}", seg)))?;

    Ok((length, delimiter))
}

/// Parse delimited values from a string
pub fn parse_delimited_values(input: &str, delimiter: Delimiter) -> Vec<String> {
    let mut values = Vec::new();
    let mut value_buffer = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == BACKSLASH && chars.peek().is_some() && in_quotes {
            value_buffer.push(ch);
            if let Some(next) = chars.next() {
                value_buffer.push(next);
            }
            continue;
        }

        if ch == DOUBLE_QUOTE {
            in_quotes = !in_quotes;
            value_buffer.push(ch);
            continue;
        }

        if ch == delimiter.as_char() && !in_quotes {
            values.push(value_buffer.trim().to_string());
            value_buffer = String::new();
            continue;
        }

        value_buffer.push(ch);
    }

    // Add last value
    if !value_buffer.is_empty() || !values.is_empty() {
        values.push(value_buffer.trim().to_string());
    }

    values
}

/// Map row values to primitives
pub fn map_row_values_to_primitives(values: &[String]) -> Vec<JsonValue> {
    values.iter().map(|v| parse_primitive_token(v)).collect()
}

/// Parse a primitive token to a JSON value
pub fn parse_primitive_token(token: &str) -> JsonValue {
    let trimmed = token.trim();

    // Empty token
    if trimmed.is_empty() {
        return JsonValue::String(String::new());
    }

    // Quoted string
    if trimmed.starts_with(DOUBLE_QUOTE) {
        return match parse_string_literal(trimmed) {
            Ok(s) => JsonValue::String(s),
            Err(_) => JsonValue::String(trimmed.to_string()),
        };
    }

    // Boolean or null literals
    if is_boolean_or_null_literal(trimmed) {
        if trimmed == TRUE_LITERAL {
            return JsonValue::Bool(true);
        }
        if trimmed == FALSE_LITERAL {
            return JsonValue::Bool(false);
        }
        if trimmed == NULL_LITERAL {
            return JsonValue::Null;
        }
    }

    // Numeric literal
    if is_numeric_literal(trimmed) {
        // Try to parse as integer first if it doesn't contain decimal or exponent
        if !trimmed.contains('.') && !trimmed.contains('e') && !trimmed.contains('E') {
            if let Ok(n) = trimmed.parse::<i64>() {
                return JsonValue::Number(n.into());
            }
        }
        // Fall back to float
        if let Ok(n) = trimmed.parse::<f64>() {
            // Normalize negative zero to positive zero
            let normalized = if n == 0.0 && n.is_sign_negative() {
                0.0
            } else {
                n
            };
            return serde_json::Number::from_f64(normalized)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::String(trimmed.to_string()));
        }
    }

    // Unquoted string
    JsonValue::String(trimmed.to_string())
}

/// Parse a string literal (potentially quoted)
pub fn parse_string_literal(token: &str) -> ToonResult<String> {
    let trimmed = token.trim();

    if trimmed.starts_with(DOUBLE_QUOTE) {
        let closing_quote_index = find_closing_quote(trimmed, 0).ok_or_else(|| {
            ToonError::syntax_no_line("Unterminated string: missing closing quote")
        })?;

        if closing_quote_index != trimmed.len() - 1 {
            return Err(ToonError::syntax_no_line(
                "Unexpected characters after closing quote",
            ));
        }

        let content = &trimmed[1..closing_quote_index];
        unescape_string(content).map_err(|e| ToonError::syntax_no_line(e))
    } else {
        Ok(trimmed.to_string())
    }
}

/// Parse an unquoted key
pub fn parse_unquoted_key(content: &str, start: usize) -> ToonResult<(String, usize)> {
    let mut parse_position = start;
    let bytes = content.as_bytes();

    while parse_position < bytes.len() && bytes[parse_position] != b':' {
        parse_position += 1;
    }

    if parse_position >= bytes.len() || bytes[parse_position] != b':' {
        return Err(ToonError::syntax_no_line("Missing colon after key"));
    }

    let key = content[start..parse_position].trim().to_string();
    parse_position += 1; // Skip the colon

    Ok((key, parse_position))
}

/// Parse a quoted key
pub fn parse_quoted_key(content: &str, start: usize) -> ToonResult<(String, usize)> {
    let closing_quote_index = find_closing_quote(content, start)
        .ok_or_else(|| ToonError::syntax_no_line("Unterminated quoted key"))?;

    let key_content = &content[start + 1..closing_quote_index];
    let key = unescape_string(key_content).map_err(|e| ToonError::syntax_no_line(e))?;
    let mut parse_position = closing_quote_index + 1;

    // Validate and skip colon after quoted key
    let bytes = content.as_bytes();
    if parse_position >= bytes.len() || bytes[parse_position] != b':' {
        return Err(ToonError::syntax_no_line("Missing colon after key"));
    }
    parse_position += 1;

    Ok((key, parse_position))
}

/// Parse a key token (quoted or unquoted)
pub fn parse_key_token(content: &str, start: usize) -> ToonResult<(String, usize, bool)> {
    let bytes = content.as_bytes();
    let is_quoted = bytes.get(start) == Some(&b'"');

    let (key, end) = if is_quoted {
        parse_quoted_key(content, start)?
    } else {
        parse_unquoted_key(content, start)?
    };

    Ok((key, end, is_quoted))
}

/// Check if content after hyphen is an array header
pub fn is_array_header_after_hyphen(content: &str) -> bool {
    content.trim().starts_with(OPEN_BRACKET) && find_unquoted_char(content, COLON, 0).is_some()
}

/// Check if content after hyphen is an object's first field
pub fn is_object_first_field_after_hyphen(content: &str) -> bool {
    find_unquoted_char(content, COLON, 0).is_some()
}
