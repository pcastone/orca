//! Path expansion for decoded TOON values

use crate::constants::DOT;
use crate::shared::is_identifier_segment;
use crate::types::{ToonError, ToonResult};
use serde_json::{Map, Value as JsonValue};

/// Expand dotted keys into nested objects in safe mode
pub fn expand_paths_safe(value: JsonValue, strict: bool) -> ToonResult<JsonValue> {
    match value {
        JsonValue::Array(arr) => {
            let expanded: Result<Vec<JsonValue>, _> = arr
                .into_iter()
                .map(|item| expand_paths_safe(item, strict))
                .collect();
            Ok(JsonValue::Array(expanded?))
        }
        JsonValue::Object(obj) => {
            let mut expanded_object: Map<String, JsonValue> = Map::new();

            for (key, key_value) in obj {
                // Check if key contains dots and should be expanded
                if key.contains(DOT) {
                    let segments: Vec<&str> = key.split(DOT).collect();

                    // Validate all segments are identifiers
                    if segments.iter().all(|seg| is_identifier_segment(seg)) {
                        // Expand this dotted key
                        let expanded_value = expand_paths_safe(key_value, strict)?;
                        insert_path_safe(&mut expanded_object, &segments, expanded_value, strict)?;
                        continue;
                    }
                }

                // Not expandable - keep as literal key, but still recursively expand the value
                let expanded_value = expand_paths_safe(key_value, strict)?;

                // Check for conflicts with already-expanded keys
                if let Some(conflicting_value) = expanded_object.get(&key) {
                    if can_merge(conflicting_value, &expanded_value) {
                        // Merge objects
                        if let (JsonValue::Object(target), JsonValue::Object(source)) =
                            (expanded_object.get_mut(&key).unwrap(), expanded_value)
                        {
                            merge_objects(target, source, strict)?;
                        }
                    } else {
                        // Conflict: incompatible types
                        if strict {
                            return Err(ToonError::TypeError(format!(
                                "Path expansion conflict at key \"{}\": cannot merge types",
                                key
                            )));
                        }
                        // Non-strict: overwrite (LWW)
                        expanded_object.insert(key, expanded_value);
                    }
                } else {
                    // No conflict - insert directly
                    expanded_object.insert(key, expanded_value);
                }
            }

            Ok(JsonValue::Object(expanded_object))
        }
        // Primitive value - return as-is
        _ => Ok(value),
    }
}

/// Insert a value at a nested path, creating intermediate objects as needed
fn insert_path_safe(
    target: &mut Map<String, JsonValue>,
    segments: &[&str],
    value: JsonValue,
    strict: bool,
) -> ToonResult<()> {
    if segments.is_empty() {
        return Ok(());
    }

    if segments.len() == 1 {
        // Final segment - insert value
        let last_seg = segments[0];
        insert_final_segment(target, last_seg, value, strict)?;
        return Ok(());
    }

    // Walk to the penultimate segment, creating objects as needed
    let segment = segments[0];
    let segment_string = segment.to_string();

    // Ensure the intermediate object exists
    if !target.contains_key(segment) {
        target.insert(segment_string.clone(), JsonValue::Object(Map::new()));
    }

    // Check if existing value is an object
    let needs_replace = match target.get(segment) {
        Some(JsonValue::Object(_)) => false,
        Some(_) => {
            if strict {
                return Err(ToonError::TypeError(format!(
                    "Path expansion conflict at segment \"{}\": expected object",
                    segment
                )));
            }
            true
        }
        None => false,
    };

    if needs_replace {
        target.insert(segment_string, JsonValue::Object(Map::new()));
    }

    // Recurse into the nested object
    if let Some(JsonValue::Object(nested)) = target.get_mut(segment) {
        insert_path_safe(nested, &segments[1..], value, strict)?;
    }

    Ok(())
}

/// Insert value at the final segment
fn insert_final_segment(
    target: &mut Map<String, JsonValue>,
    last_seg: &str,
    value: JsonValue,
    strict: bool,
) -> ToonResult<()> {
    let should_merge = match target.get(last_seg) {
        Some(dest) => can_merge(dest, &value),
        None => false,
    };

    if should_merge {
        // Both are objects - deep merge
        if let (Some(JsonValue::Object(target_obj)), JsonValue::Object(source_obj)) =
            (target.get_mut(last_seg), value)
        {
            merge_objects(target_obj, source_obj, strict)?;
        }
    } else if target.contains_key(last_seg) {
        // Conflict: incompatible types
        if strict {
            return Err(ToonError::TypeError(format!(
                "Path expansion conflict at key \"{}\": cannot merge types",
                last_seg
            )));
        }
        // Non-strict: overwrite (LWW)
        target.insert(last_seg.to_string(), value);
    } else {
        // No conflict - insert directly
        target.insert(last_seg.to_string(), value);
    }

    Ok(())
}

/// Deep merge properties from source into target
fn merge_objects(
    target: &mut Map<String, JsonValue>,
    source: Map<String, JsonValue>,
    strict: bool,
) -> ToonResult<()> {
    for (key, source_value) in source {
        if let Some(target_value) = target.get(&key) {
            if can_merge(target_value, &source_value) {
                // Both are objects - recursively merge
                if let (Some(JsonValue::Object(target_obj)), JsonValue::Object(source_obj)) =
                    (target.get_mut(&key), source_value)
                {
                    merge_objects(target_obj, source_obj, strict)?;
                }
            } else {
                // Conflict: incompatible types
                if strict {
                    return Err(ToonError::TypeError(format!(
                        "Path expansion conflict at key \"{}\": cannot merge types",
                        key
                    )));
                }
                // Non-strict: overwrite (LWW)
                target.insert(key, source_value);
            }
        } else {
            // Key doesn't exist in target - copy it
            target.insert(key, source_value);
        }
    }

    Ok(())
}

fn can_merge(a: &JsonValue, b: &JsonValue) -> bool {
    matches!((a, b), (JsonValue::Object(_), JsonValue::Object(_)))
}
