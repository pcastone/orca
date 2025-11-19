//! TOON value decoders

use crate::constants::{COLON, DEFAULT_DELIMITER, LIST_ITEM_PREFIX};
use crate::shared::find_closing_quote;
use crate::types::{ArrayHeaderInfo, DecodeOptions, Depth, ParsedLine, ToonError, ToonResult};
use serde_json::{Map, Value as JsonValue};

use super::parser::{
    is_array_header_after_hyphen, is_object_first_field_after_hyphen, map_row_values_to_primitives,
    parse_array_header_line, parse_delimited_values, parse_key_token, parse_primitive_token,
};
use super::scanner::LineCursor;
use super::validation::{
    assert_expected_count, validate_no_blank_lines_in_range, validate_no_extra_list_items,
    validate_no_extra_tabular_rows,
};

/// Decode a TOON value from parsed lines
pub fn decode_value_from_lines(
    cursor: &mut LineCursor,
    options: &DecodeOptions,
) -> ToonResult<JsonValue> {
    let first = cursor.peek().ok_or_else(|| {
        ToonError::ReferenceError("No content to decode".to_string())
    })?;

    // Check for root array
    if is_array_header_after_hyphen(&first.content) {
        if let Some((header, inline_values)) =
            parse_array_header_line(&first.content, DEFAULT_DELIMITER)
        {
            cursor.advance();
            return decode_array_from_header(&header, inline_values.as_deref(), cursor, 0, options);
        }
    }

    // Check for single primitive value
    if cursor.len() == 1 && !is_key_value_line(first) {
        return Ok(parse_primitive_token(first.content.trim()));
    }

    // Default to object
    decode_object(cursor, 0, options)
}

fn is_key_value_line(line: &ParsedLine) -> bool {
    let content = &line.content;
    // Look for unquoted colon or quoted key followed by colon
    if content.starts_with('"') {
        let closing_quote_index = match find_closing_quote(content, 0) {
            Some(i) => i,
            None => return false,
        };
        content[closing_quote_index + 1..].contains(COLON)
    } else {
        content.contains(COLON)
    }
}

/// Decode an object from parsed lines
fn decode_object(
    cursor: &mut LineCursor,
    base_depth: Depth,
    options: &DecodeOptions,
) -> ToonResult<JsonValue> {
    let mut obj: Map<String, JsonValue> = Map::new();
    let mut computed_depth: Option<Depth> = None;

    while !cursor.at_end() {
        let line = match cursor.peek() {
            Some(l) => l,
            None => break,
        };

        if line.depth < base_depth {
            break;
        }

        if computed_depth.is_none() && line.depth >= base_depth {
            computed_depth = Some(line.depth);
        }

        if Some(line.depth) == computed_depth {
            cursor.advance();
            let content = cursor.current().unwrap().content.clone();
            let (key, value, _is_quoted) =
                decode_key_value(&content, cursor, computed_depth.unwrap(), options)?;
            obj.insert(key, value);
        } else {
            break;
        }
    }

    Ok(JsonValue::Object(obj))
}

/// Decode a key-value pair
fn decode_key_value(
    content: &str,
    cursor: &mut LineCursor,
    base_depth: Depth,
    options: &DecodeOptions,
) -> ToonResult<(String, JsonValue, bool)> {
    // Check for array header first
    if let Some((header, inline_values)) = parse_array_header_line(content, DEFAULT_DELIMITER) {
        if let Some(key) = header.key.clone() {
            let decoded_value =
                decode_array_from_header(&header, inline_values.as_deref(), cursor, base_depth, options)?;
            return Ok((key, decoded_value, false));
        }
    }

    // Regular key-value pair
    let (key, end, is_quoted) = parse_key_token(content, 0)?;
    let rest = content[end..].trim();

    // No value after colon - expect nested object or empty
    if rest.is_empty() {
        if let Some(next_line) = cursor.peek() {
            if next_line.depth > base_depth {
                let nested = decode_object(cursor, base_depth + 1, options)?;
                return Ok((key, nested, is_quoted));
            }
        }
        // Empty object
        return Ok((key, JsonValue::Object(Map::new()), is_quoted));
    }

    // Inline primitive value
    let decoded_value = parse_primitive_token(rest);
    Ok((key, decoded_value, is_quoted))
}

/// Decode an array from its header
fn decode_array_from_header(
    header: &ArrayHeaderInfo,
    inline_values: Option<&str>,
    cursor: &mut LineCursor,
    base_depth: Depth,
    options: &DecodeOptions,
) -> ToonResult<JsonValue> {
    // Inline primitive array
    if let Some(values) = inline_values {
        return decode_inline_primitive_array(header, values, options);
    }

    // Tabular array
    if header.fields.is_some() && !header.fields.as_ref().unwrap().is_empty() {
        return decode_tabular_array(header, cursor, base_depth, options);
    }

    // List array
    decode_list_array(header, cursor, base_depth, options)
}

/// Decode an inline primitive array
fn decode_inline_primitive_array(
    header: &ArrayHeaderInfo,
    inline_values: &str,
    options: &DecodeOptions,
) -> ToonResult<JsonValue> {
    if inline_values.trim().is_empty() {
        assert_expected_count(0, header.length, "inline array items", options)?;
        return Ok(JsonValue::Array(Vec::new()));
    }

    let values = parse_delimited_values(inline_values, header.delimiter);
    let primitives = map_row_values_to_primitives(&values);

    assert_expected_count(primitives.len(), header.length, "inline array items", options)?;

    Ok(JsonValue::Array(primitives))
}

/// Decode a list array
fn decode_list_array(
    header: &ArrayHeaderInfo,
    cursor: &mut LineCursor,
    base_depth: Depth,
    options: &DecodeOptions,
) -> ToonResult<JsonValue> {
    let mut items: Vec<JsonValue> = Vec::new();
    let item_depth = base_depth + 1;

    let mut start_line: Option<usize> = None;
    let mut end_line: Option<usize> = None;

    while !cursor.at_end() && items.len() < header.length {
        let line = match cursor.peek() {
            Some(l) => l,
            None => break,
        };

        if line.depth < item_depth {
            break;
        }

        let is_list_item =
            line.content.starts_with(LIST_ITEM_PREFIX) || line.content == "-";

        if line.depth == item_depth && is_list_item {
            if start_line.is_none() {
                start_line = Some(line.line_number);
            }
            end_line = Some(line.line_number);

            let item = decode_list_item(cursor, item_depth, options)?;
            items.push(item);

            if let Some(current_line) = cursor.current() {
                end_line = Some(current_line.line_number);
            }
        } else {
            break;
        }
    }

    assert_expected_count(items.len(), header.length, "list array items", options)?;

    // In strict mode, check for blank lines inside the array
    if let (Some(start), Some(end)) = (start_line, end_line) {
        validate_no_blank_lines_in_range(
            start,
            end,
            cursor.get_blank_lines(),
            options.strict,
            "list array",
        )?;
    }

    // In strict mode, check for extra items
    if options.strict {
        validate_no_extra_list_items(cursor, item_depth, header.length)?;
    }

    Ok(JsonValue::Array(items))
}

/// Decode a tabular array
fn decode_tabular_array(
    header: &ArrayHeaderInfo,
    cursor: &mut LineCursor,
    base_depth: Depth,
    options: &DecodeOptions,
) -> ToonResult<JsonValue> {
    let mut objects: Vec<JsonValue> = Vec::new();
    let row_depth = base_depth + 1;

    let mut start_line: Option<usize> = None;
    let mut end_line: Option<usize> = None;

    let fields = header.fields.as_ref().unwrap();

    while !cursor.at_end() && objects.len() < header.length {
        let line = match cursor.peek() {
            Some(l) => l,
            None => break,
        };

        if line.depth < row_depth {
            break;
        }

        if line.depth == row_depth {
            if start_line.is_none() {
                start_line = Some(line.line_number);
            }
            end_line = Some(line.line_number);

            cursor.advance();
            let line = cursor.current().unwrap();
            let values = parse_delimited_values(&line.content, header.delimiter);
            assert_expected_count(values.len(), fields.len(), "tabular row values", options)?;

            let primitives = map_row_values_to_primitives(&values);
            let mut obj: Map<String, JsonValue> = Map::new();

            for (i, field) in fields.iter().enumerate() {
                obj.insert(field.clone(), primitives[i].clone());
            }

            objects.push(JsonValue::Object(obj));
        } else {
            break;
        }
    }

    assert_expected_count(objects.len(), header.length, "tabular rows", options)?;

    // In strict mode, check for blank lines inside the array
    if let (Some(start), Some(end)) = (start_line, end_line) {
        validate_no_blank_lines_in_range(
            start,
            end,
            cursor.get_blank_lines(),
            options.strict,
            "tabular array",
        )?;
    }

    // In strict mode, check for extra rows
    if options.strict {
        validate_no_extra_tabular_rows(cursor, row_depth, header)?;
    }

    Ok(JsonValue::Array(objects))
}

/// Decode a list item
fn decode_list_item(
    cursor: &mut LineCursor,
    base_depth: Depth,
    options: &DecodeOptions,
) -> ToonResult<JsonValue> {
    let line = cursor.next().ok_or_else(|| {
        ToonError::ReferenceError("Expected list item".to_string())
    })?;

    // Empty list item should be an empty object
    if line.content == "-" {
        return Ok(JsonValue::Object(Map::new()));
    }

    let (after_hyphen, _line_number) = if line.content.starts_with(LIST_ITEM_PREFIX) {
        (line.content[LIST_ITEM_PREFIX.len()..].to_string(), line.line_number)
    } else {
        return Err(ToonError::syntax(
            line.line_number,
            format!("Expected list item to start with \"{}\"", LIST_ITEM_PREFIX),
        ));
    };

    // Empty content after list item should also be an empty object
    if after_hyphen.trim().is_empty() {
        return Ok(JsonValue::Object(Map::new()));
    }

    // Check for array header after hyphen
    if is_array_header_after_hyphen(&after_hyphen) {
        if let Some((header, inline_values)) =
            parse_array_header_line(&after_hyphen, DEFAULT_DELIMITER)
        {
            return decode_array_from_header(&header, inline_values.as_deref(), cursor, base_depth, options);
        }
    }

    // Check for object first field after hyphen
    if is_object_first_field_after_hyphen(&after_hyphen) {
        return decode_object_from_list_item(&after_hyphen, cursor, base_depth, options);
    }

    // Primitive value
    Ok(parse_primitive_token(&after_hyphen))
}

/// Decode an object from a list item's first field
fn decode_object_from_list_item(
    after_hyphen: &str,
    cursor: &mut LineCursor,
    base_depth: Depth,
    options: &DecodeOptions,
) -> ToonResult<JsonValue> {
    let (key, value, _is_quoted) = decode_key_value(after_hyphen, cursor, base_depth, options)?;

    let mut obj: Map<String, JsonValue> = Map::new();
    obj.insert(key, value);

    let follow_depth = base_depth + 1;

    // Read subsequent fields
    while !cursor.at_end() {
        let line = match cursor.peek() {
            Some(l) => l,
            None => break,
        };

        if line.depth < follow_depth {
            break;
        }

        if line.depth == follow_depth && !line.content.starts_with(LIST_ITEM_PREFIX) {
            cursor.advance();
            let content = cursor.current().unwrap().content.clone();
            let (k, v, _k_is_quoted) =
                decode_key_value(&content, cursor, follow_depth, options)?;
            obj.insert(k, v);
        } else {
            break;
        }
    }

    Ok(JsonValue::Object(obj))
}
