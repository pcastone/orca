//! TOON value encoders

use crate::constants::{DOT, LIST_ITEM_MARKER};
use crate::types::{Depth, EncodeOptions, KeyFolding};
use serde_json::Value as JsonValue;
use std::collections::HashSet;

use super::folding::try_fold_key_chain;
use super::normalize::{
    is_array_of_arrays, is_array_of_objects, is_array_of_primitives, is_empty_object,
    is_json_array, is_json_object, is_json_primitive,
};
use super::primitives::{encode_and_join_primitives, encode_key, encode_primitive, format_header};
use super::writer::LineWriter;

/// Encode a JSON value to TOON format string
pub fn encode_value(value: &JsonValue, options: &EncodeOptions) -> String {
    if is_json_primitive(value) {
        return encode_primitive(value, options.delimiter.as_char());
    }

    let mut writer = LineWriter::new(options.indent);

    if is_json_array(value) {
        encode_array(None, value, &mut writer, 0, options);
    } else if is_json_object(value) {
        encode_object(value, &mut writer, 0, options, None, None, None);
    }

    writer.to_string()
}

/// Encode a JSON object
pub fn encode_object(
    value: &JsonValue,
    writer: &mut LineWriter,
    depth: Depth,
    options: &EncodeOptions,
    root_literal_keys: Option<&HashSet<String>>,
    path_prefix: Option<&str>,
    remaining_depth: Option<usize>,
) {
    let obj = match value {
        JsonValue::Object(obj) => obj,
        _ => return,
    };

    let keys: Vec<String> = obj.keys().cloned().collect();

    // At root level, collect all literal dotted keys for collision checking
    let owned_root_keys: HashSet<String>;
    let root_keys = if depth == 0 && root_literal_keys.is_none() {
        owned_root_keys = keys.iter().filter(|k| k.contains('.')).cloned().collect();
        Some(&owned_root_keys)
    } else {
        root_literal_keys
    };

    let effective_flatten_depth = remaining_depth.unwrap_or(options.flatten_depth);

    for (key, val) in obj {
        encode_key_value_pair(
            key,
            val,
            writer,
            depth,
            options,
            Some(&keys),
            root_keys,
            path_prefix,
            Some(effective_flatten_depth),
        );
    }
}

/// Encode a key-value pair
pub fn encode_key_value_pair(
    key: &str,
    value: &JsonValue,
    writer: &mut LineWriter,
    depth: Depth,
    options: &EncodeOptions,
    siblings: Option<&[String]>,
    root_literal_keys: Option<&HashSet<String>>,
    path_prefix: Option<&str>,
    flatten_depth: Option<usize>,
) {
    let current_path = match path_prefix {
        Some(prefix) => format!("{}{}{}", prefix, DOT, key),
        None => key.to_string(),
    };
    let effective_flatten_depth = flatten_depth.unwrap_or(options.flatten_depth);

    // Attempt key folding when enabled
    if options.key_folding == KeyFolding::Safe {
        if let Some(siblings) = siblings {
            if let Some(fold_result) = try_fold_key_chain(
                key,
                value,
                siblings,
                options,
                root_literal_keys,
                path_prefix,
                Some(effective_flatten_depth),
            ) {
                let encoded_folded_key = encode_key(&fold_result.folded_key);

                // Case 1: Fully folded to a leaf value
                if fold_result.remainder.is_none() {
                    if is_json_primitive(&fold_result.leaf_value) {
                        writer.push(
                            depth,
                            &format!(
                                "{}: {}",
                                encoded_folded_key,
                                encode_primitive(&fold_result.leaf_value, options.delimiter.as_char())
                            ),
                        );
                        return;
                    } else if is_json_array(&fold_result.leaf_value) {
                        encode_array(
                            Some(&fold_result.folded_key),
                            &fold_result.leaf_value,
                            writer,
                            depth,
                            options,
                        );
                        return;
                    } else if is_json_object(&fold_result.leaf_value)
                        && is_empty_object(&fold_result.leaf_value)
                    {
                        writer.push(depth, &format!("{}:", encoded_folded_key));
                        return;
                    }
                }

                // Case 2: Partially folded with a tail object
                if let Some(remainder) = &fold_result.remainder {
                    if is_json_object(remainder) {
                        writer.push(depth, &format!("{}:", encoded_folded_key));
                        let remaining_depth = effective_flatten_depth - fold_result.segment_count;
                        let folded_path = match path_prefix {
                            Some(prefix) => {
                                format!("{}{}{}", prefix, DOT, fold_result.folded_key)
                            }
                            None => fold_result.folded_key.clone(),
                        };
                        encode_object(
                            remainder,
                            writer,
                            depth + 1,
                            options,
                            root_literal_keys,
                            Some(&folded_path),
                            Some(remaining_depth),
                        );
                        return;
                    }
                }
            }
        }
    }

    // No folding applied - use standard encoding
    let encoded_key = encode_key(key);

    if is_json_primitive(value) {
        writer.push(
            depth,
            &format!(
                "{}: {}",
                encoded_key,
                encode_primitive(value, options.delimiter.as_char())
            ),
        );
    } else if is_json_array(value) {
        encode_array(Some(key), value, writer, depth, options);
    } else if is_json_object(value) {
        writer.push(depth, &format!("{}:", encoded_key));
        if !is_empty_object(value) {
            encode_object(
                value,
                writer,
                depth + 1,
                options,
                root_literal_keys,
                Some(&current_path),
                Some(effective_flatten_depth),
            );
        }
    }
}

/// Encode an array
pub fn encode_array(
    key: Option<&str>,
    value: &JsonValue,
    writer: &mut LineWriter,
    depth: Depth,
    options: &EncodeOptions,
) {
    let arr = match value {
        JsonValue::Array(arr) => arr,
        _ => return,
    };

    if arr.is_empty() {
        let header = format_header(0, key, None, options.delimiter.as_char());
        writer.push(depth, &header);
        return;
    }

    // Primitive array
    if is_array_of_primitives(value) {
        let array_line = encode_inline_array_line(arr, options.delimiter.as_char(), key);
        writer.push(depth, &array_line);
        return;
    }

    // Array of arrays (all primitives)
    if is_array_of_arrays(value) {
        let all_primitive_arrays = arr
            .iter()
            .all(|item| matches!(item, JsonValue::Array(inner) if inner.iter().all(is_json_primitive)));
        if all_primitive_arrays {
            encode_array_of_arrays_as_list_items(key, arr, writer, depth, options);
            return;
        }
    }

    // Array of objects
    if is_array_of_objects(value) {
        if let Some(header) = extract_tabular_header(arr) {
            encode_array_of_objects_as_tabular(key, arr, &header, writer, depth, options);
        } else {
            encode_mixed_array_as_list_items(key, arr, writer, depth, options);
        }
        return;
    }

    // Mixed array: fallback to expanded format
    encode_mixed_array_as_list_items(key, arr, writer, depth, options);
}

/// Encode an inline array line
fn encode_inline_array_line(values: &[JsonValue], delimiter: char, prefix: Option<&str>) -> String {
    let header = format_header(values.len(), prefix, None, delimiter);
    let joined_value = encode_and_join_primitives(values, delimiter);
    if values.is_empty() {
        header
    } else {
        format!("{} {}", header, joined_value)
    }
}

/// Encode array of arrays as list items
fn encode_array_of_arrays_as_list_items(
    prefix: Option<&str>,
    values: &[JsonValue],
    writer: &mut LineWriter,
    depth: Depth,
    options: &EncodeOptions,
) {
    let header = format_header(values.len(), prefix, None, options.delimiter.as_char());
    writer.push(depth, &header);

    for arr in values {
        if let JsonValue::Array(inner) = arr {
            if inner.iter().all(is_json_primitive) {
                let array_line = encode_inline_array_line(inner, options.delimiter.as_char(), None);
                writer.push_list_item(depth + 1, &array_line);
            }
        }
    }
}

/// Extract tabular header from array of objects
fn extract_tabular_header(rows: &[JsonValue]) -> Option<Vec<String>> {
    if rows.is_empty() {
        return None;
    }

    let first_row = match &rows[0] {
        JsonValue::Object(obj) => obj,
        _ => return None,
    };

    let first_keys: Vec<String> = first_row.keys().cloned().collect();
    if first_keys.is_empty() {
        return None;
    }

    if is_tabular_array(rows, &first_keys) {
        Some(first_keys)
    } else {
        None
    }
}

/// Check if an array is tabular (all objects have same keys and primitive values)
fn is_tabular_array(rows: &[JsonValue], header: &[String]) -> bool {
    for row in rows {
        let obj = match row {
            JsonValue::Object(obj) => obj,
            _ => return false,
        };

        if obj.len() != header.len() {
            return false;
        }

        for key in header {
            match obj.get(key) {
                Some(val) if is_json_primitive(val) => {}
                _ => return false,
            }
        }
    }
    true
}

/// Encode array of objects as tabular format
fn encode_array_of_objects_as_tabular(
    prefix: Option<&str>,
    rows: &[JsonValue],
    header: &[String],
    writer: &mut LineWriter,
    depth: Depth,
    options: &EncodeOptions,
) {
    let formatted_header = format_header(
        rows.len(),
        prefix,
        Some(header),
        options.delimiter.as_char(),
    );
    writer.push(depth, &formatted_header);

    for row in rows {
        if let JsonValue::Object(obj) = row {
            let values: Vec<JsonValue> = header.iter().map(|k| obj.get(k).cloned().unwrap_or(JsonValue::Null)).collect();
            let joined_value = encode_and_join_primitives(&values, options.delimiter.as_char());
            writer.push(depth + 1, &joined_value);
        }
    }
}

/// Encode mixed array as list items
fn encode_mixed_array_as_list_items(
    prefix: Option<&str>,
    items: &[JsonValue],
    writer: &mut LineWriter,
    depth: Depth,
    options: &EncodeOptions,
) {
    let header = format_header(items.len(), prefix, None, options.delimiter.as_char());
    writer.push(depth, &header);

    for item in items {
        encode_list_item_value(item, writer, depth + 1, options);
    }
}

/// Encode an object as a list item
fn encode_object_as_list_item(
    obj: &serde_json::Map<String, JsonValue>,
    writer: &mut LineWriter,
    depth: Depth,
    options: &EncodeOptions,
) {
    if obj.is_empty() {
        writer.push(depth, &LIST_ITEM_MARKER.to_string());
        return;
    }

    let entries: Vec<_> = obj.iter().collect();
    let (first_key, first_value) = entries[0];
    let encoded_key = encode_key(first_key);

    if is_json_primitive(first_value) {
        writer.push_list_item(
            depth,
            &format!(
                "{}: {}",
                encoded_key,
                encode_primitive(first_value, options.delimiter.as_char())
            ),
        );
    } else if let JsonValue::Array(arr) = first_value {
        if arr.iter().all(is_json_primitive) {
            let array_line = encode_inline_array_line(arr, options.delimiter.as_char(), Some(first_key));
            writer.push_list_item(depth, &array_line);
        } else {
            writer.push_list_item(depth, &format!("{}[{}]:", encoded_key, arr.len()));
            for item in arr {
                encode_list_item_value(item, writer, depth + 1, options);
            }
        }
    } else if is_json_object(first_value) {
        writer.push_list_item(depth, &format!("{}:", encoded_key));
        if !is_empty_object(first_value) {
            encode_object(first_value, writer, depth + 2, options, None, None, None);
        }
    }

    // Remaining entries on indented lines
    for (key, value) in entries.iter().skip(1) {
        encode_key_value_pair(key, value, writer, depth + 1, options, None, None, None, None);
    }
}

/// Encode a list item value
fn encode_list_item_value(
    value: &JsonValue,
    writer: &mut LineWriter,
    depth: Depth,
    options: &EncodeOptions,
) {
    if is_json_primitive(value) {
        writer.push_list_item(depth, &encode_primitive(value, options.delimiter.as_char()));
    } else if let JsonValue::Array(arr) = value {
        if arr.iter().all(is_json_primitive) {
            let array_line = encode_inline_array_line(arr, options.delimiter.as_char(), None);
            writer.push_list_item(depth, &array_line);
        }
    } else if let JsonValue::Object(obj) = value {
        encode_object_as_list_item(obj, writer, depth, options);
    }
}
