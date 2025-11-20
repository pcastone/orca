//! Key folding utilities for TOON encoding

use crate::constants::DOT;
use crate::shared::is_identifier_segment;
use crate::types::{EncodeOptions, KeyFolding};
use serde_json::Value as JsonValue;
use std::collections::HashSet;

use super::normalize::{is_empty_object, is_json_object};

/// Result of attempting to fold a key chain
pub struct FoldResult {
    /// The folded key with dot-separated segments
    pub folded_key: String,
    /// The remainder value after folding (None if fully folded to a leaf)
    pub remainder: Option<JsonValue>,
    /// The leaf value at the end of the folded chain
    pub leaf_value: JsonValue,
    /// The number of segments that were folded
    pub segment_count: usize,
}

/// Attempts to fold a single-key object chain into a dotted path
pub fn try_fold_key_chain(
    key: &str,
    value: &JsonValue,
    siblings: &[String],
    options: &EncodeOptions,
    root_literal_keys: Option<&HashSet<String>>,
    path_prefix: Option<&str>,
    flatten_depth: Option<usize>,
) -> Option<FoldResult> {
    // Only fold when safe mode is enabled
    if options.key_folding != KeyFolding::Safe {
        return None;
    }

    // Can only fold objects
    if !is_json_object(value) {
        return None;
    }

    let effective_flatten_depth = flatten_depth.unwrap_or(options.flatten_depth);

    // Collect the chain of single-key objects
    let (segments, tail, leaf_value) =
        collect_single_key_chain(key, value, effective_flatten_depth);

    // Need at least 2 segments for folding to be worthwhile
    if segments.len() < 2 {
        return None;
    }

    // Validate all segments are safe identifiers
    if !segments.iter().all(|seg| is_identifier_segment(seg)) {
        return None;
    }

    // Build the folded key
    let folded_key = segments.join(&DOT.to_string());

    // Build the absolute path from root
    let absolute_path = match path_prefix {
        Some(prefix) => format!("{}{}{}", prefix, DOT, folded_key),
        None => folded_key.clone(),
    };

    // Check for collision with existing literal sibling keys
    if siblings.contains(&folded_key) {
        return None;
    }

    // Check for collision with root-level literal dotted keys
    if let Some(keys) = root_literal_keys {
        if keys.contains(&absolute_path) {
            return None;
        }
    }

    Some(FoldResult {
        folded_key,
        remainder: tail,
        leaf_value,
        segment_count: segments.len(),
    })
}

/// Collects a chain of single-key objects into segments
fn collect_single_key_chain(
    start_key: &str,
    start_value: &JsonValue,
    max_depth: usize,
) -> (Vec<String>, Option<JsonValue>, JsonValue) {
    let mut segments = vec![start_key.to_string()];
    let mut current_value = start_value.clone();

    while segments.len() < max_depth {
        // Must be an object to continue
        let obj = match &current_value {
            JsonValue::Object(obj) => obj,
            _ => break,
        };

        // Must have exactly one key to continue the chain
        if obj.len() != 1 {
            break;
        }

        let (next_key, next_value) = obj.iter().next().unwrap();
        segments.push(next_key.clone());
        current_value = next_value.clone();
    }

    // Determine the tail
    if !is_json_object(&current_value) || is_empty_object(&current_value) {
        // Array, primitive, null, or empty object - this is a leaf value
        (segments, None, current_value)
    } else {
        // Has keys - return as tail (remainder)
        (segments, Some(current_value.clone()), current_value)
    }
}
