//! Value normalization for encoding

use serde_json::{Map, Value as JsonValue};

/// Normalize a JSON value for encoding
pub fn normalize_value(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::Null => JsonValue::Null,
        JsonValue::Bool(b) => JsonValue::Bool(b),
        JsonValue::Number(n) => {
            // Handle special number cases
            if let Some(f) = n.as_f64() {
                if !f.is_finite() {
                    return JsonValue::Null;
                }
                // Normalize -0 to 0
                if f == 0.0 && f.is_sign_negative() {
                    return JsonValue::Number(serde_json::Number::from(0));
                }
            }
            JsonValue::Number(n)
        }
        JsonValue::String(s) => JsonValue::String(s),
        JsonValue::Array(arr) => {
            JsonValue::Array(arr.into_iter().map(normalize_value).collect())
        }
        JsonValue::Object(obj) => {
            let normalized: Map<String, JsonValue> = obj
                .into_iter()
                .map(|(k, v)| (k, normalize_value(v)))
                .collect();
            JsonValue::Object(normalized)
        }
    }
}

/// Check if a value is a JSON primitive
pub fn is_json_primitive(value: &JsonValue) -> bool {
    matches!(
        value,
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_)
    )
}

/// Check if a value is a JSON array
pub fn is_json_array(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Array(_))
}

/// Check if a value is a JSON object
pub fn is_json_object(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Object(_))
}

/// Check if an object is empty
pub fn is_empty_object(value: &JsonValue) -> bool {
    match value {
        JsonValue::Object(obj) => obj.is_empty(),
        _ => false,
    }
}

/// Check if an array contains only primitives
pub fn is_array_of_primitives(value: &JsonValue) -> bool {
    match value {
        JsonValue::Array(arr) => arr.iter().all(is_json_primitive),
        _ => false,
    }
}

/// Check if an array contains only arrays
pub fn is_array_of_arrays(value: &JsonValue) -> bool {
    match value {
        JsonValue::Array(arr) => arr.iter().all(is_json_array),
        _ => false,
    }
}

/// Check if an array contains only objects
pub fn is_array_of_objects(value: &JsonValue) -> bool {
    match value {
        JsonValue::Array(arr) => arr.iter().all(is_json_object),
        _ => false,
    }
}
