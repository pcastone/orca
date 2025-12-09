//! Deep merge utility for JSON/YAML values
//!
//! Provides functions to recursively merge nested JSON structures.

use serde_json::Value;

/// Array merge strategy
#[derive(Debug, Clone, Default)]
pub enum ArrayMergeStrategy {
    /// New array replaces old (default)
    #[default]
    Replace,
    /// Append new items to old array
    Append,
    /// Merge arrays by matching on a key field
    MergeByKey(String),
}

/// How to handle null values in overlay
#[derive(Debug, Clone, Default)]
pub enum NullHandling {
    /// Null in overlay keeps original value (default)
    #[default]
    KeepOriginal,
    /// Null in overlay sets value to null
    SetNull,
}

/// Options for deep merge behavior
#[derive(Debug, Clone, Default)]
pub struct MergeOptions {
    /// How to merge arrays
    pub array_strategy: ArrayMergeStrategy,
    /// How to handle null values
    pub null_handling: NullHandling,
}

/// Deep merge two JSON values using default options
///
/// Rules:
/// - Objects: recursively merge, new keys added, existing keys merged
/// - Arrays: new array replaces old (configurable)
/// - Primitives: new value replaces old
/// - Null in overlay: keeps original value
///
/// # Example
/// ```
/// use serde_json::json;
/// use orca::config::deep_merge::deep_merge;
///
/// let mut base = json!({
///     "name": "test",
///     "config": { "timeout": 30, "retries": 3 }
/// });
///
/// let overlay = json!({
///     "config": { "timeout": 60, "new_field": "value" }
/// });
///
/// deep_merge(&mut base, &overlay);
///
/// assert_eq!(base["name"], "test");                  // preserved
/// assert_eq!(base["config"]["timeout"], 60);         // updated
/// assert_eq!(base["config"]["retries"], 3);          // preserved
/// assert_eq!(base["config"]["new_field"], "value");  // added
/// ```
pub fn deep_merge(base: &mut Value, overlay: &Value) {
    deep_merge_with_options(base, overlay, &MergeOptions::default())
}

/// Deep merge with custom options
pub fn deep_merge_with_options(base: &mut Value, overlay: &Value, options: &MergeOptions) {
    match (base, overlay) {
        // Both are objects: recursively merge
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                // Handle null values based on options
                match options.null_handling {
                    NullHandling::KeepOriginal if overlay_value.is_null() => continue,
                    _ => {}
                }

                if let Some(base_value) = base_map.get_mut(key) {
                    // Key exists in base: recursively merge
                    deep_merge_with_options(base_value, overlay_value, options);
                } else {
                    // Key doesn't exist in base: add it
                    base_map.insert(key.clone(), overlay_value.clone());
                }
            }
        }

        // Both are arrays: apply array strategy
        (Value::Array(base_arr), Value::Array(overlay_arr)) => {
            match &options.array_strategy {
                ArrayMergeStrategy::Replace => {
                    *base_arr = overlay_arr.clone();
                }
                ArrayMergeStrategy::Append => {
                    base_arr.extend(overlay_arr.clone());
                }
                ArrayMergeStrategy::MergeByKey(key) => {
                    // Merge arrays by matching on a key field
                    for overlay_item in overlay_arr {
                        if let Some(overlay_key) = overlay_item.get(key) {
                            // Try to find matching item in base
                            if let Some(base_item) = base_arr
                                .iter_mut()
                                .find(|b| b.get(key) == Some(overlay_key))
                            {
                                // Found: merge the items
                                deep_merge_with_options(base_item, overlay_item, options);
                            } else {
                                // Not found: add to base
                                base_arr.push(overlay_item.clone());
                            }
                        } else {
                            // No key field: just append
                            base_arr.push(overlay_item.clone());
                        }
                    }
                }
            }
        }

        // Different types or primitives: overlay replaces base
        (base, overlay) => {
            match options.null_handling {
                NullHandling::KeepOriginal if overlay.is_null() => {}
                _ => *base = overlay.clone(),
            }
        }
    }
}

/// Merge a YAML-derived Value into a base Value
///
/// Convenience function that parses YAML string and merges into base.
pub fn merge_yaml_into(base: &mut Value, yaml_content: &str) -> Result<(), String> {
    let overlay: Value = serde_yaml::from_str(yaml_content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    deep_merge(base, &overlay);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deep_merge_objects() {
        let mut base = json!({
            "name": "test",
            "config": {
                "timeout": 30,
                "retries": 3
            }
        });

        let overlay = json!({
            "config": {
                "timeout": 60,
                "new_field": "value"
            }
        });

        deep_merge(&mut base, &overlay);

        assert_eq!(base["name"], "test");
        assert_eq!(base["config"]["timeout"], 60);
        assert_eq!(base["config"]["retries"], 3);
        assert_eq!(base["config"]["new_field"], "value");
    }

    #[test]
    fn test_deep_merge_nested_objects() {
        let mut base = json!({
            "level1": {
                "level2": {
                    "a": 1,
                    "b": 2
                }
            }
        });

        let overlay = json!({
            "level1": {
                "level2": {
                    "b": 20,
                    "c": 3
                }
            }
        });

        deep_merge(&mut base, &overlay);

        assert_eq!(base["level1"]["level2"]["a"], 1);
        assert_eq!(base["level1"]["level2"]["b"], 20);
        assert_eq!(base["level1"]["level2"]["c"], 3);
    }

    #[test]
    fn test_deep_merge_arrays_replace() {
        let mut base = json!({
            "items": [1, 2, 3]
        });

        let overlay = json!({
            "items": [4, 5]
        });

        deep_merge(&mut base, &overlay);

        assert_eq!(base["items"], json!([4, 5]));
    }

    #[test]
    fn test_deep_merge_arrays_append() {
        let mut base = json!({
            "items": [1, 2, 3]
        });

        let overlay = json!({
            "items": [4, 5]
        });

        let options = MergeOptions {
            array_strategy: ArrayMergeStrategy::Append,
            ..Default::default()
        };

        deep_merge_with_options(&mut base, &overlay, &options);

        assert_eq!(base["items"], json!([1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_deep_merge_arrays_by_key() {
        let mut base = json!({
            "tools": [
                { "name": "read_file", "enabled": true },
                { "name": "write_file", "enabled": false }
            ]
        });

        let overlay = json!({
            "tools": [
                { "name": "write_file", "enabled": true },
                { "name": "search", "enabled": true }
            ]
        });

        let options = MergeOptions {
            array_strategy: ArrayMergeStrategy::MergeByKey("name".to_string()),
            ..Default::default()
        };

        deep_merge_with_options(&mut base, &overlay, &options);

        let tools = base["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 3);

        // read_file unchanged
        assert_eq!(tools[0]["name"], "read_file");
        assert_eq!(tools[0]["enabled"], true);

        // write_file updated
        assert_eq!(tools[1]["name"], "write_file");
        assert_eq!(tools[1]["enabled"], true);

        // search added
        assert_eq!(tools[2]["name"], "search");
        assert_eq!(tools[2]["enabled"], true);
    }

    #[test]
    fn test_deep_merge_null_keep_original() {
        let mut base = json!({
            "name": "test",
            "value": 42
        });

        let overlay = json!({
            "name": null,
            "value": 100
        });

        deep_merge(&mut base, &overlay);

        assert_eq!(base["name"], "test"); // null keeps original
        assert_eq!(base["value"], 100);
    }

    #[test]
    fn test_deep_merge_null_set_null() {
        let mut base = json!({
            "name": "test",
            "value": 42
        });

        let overlay = json!({
            "name": null,
            "value": 100
        });

        let options = MergeOptions {
            null_handling: NullHandling::SetNull,
            ..Default::default()
        };

        deep_merge_with_options(&mut base, &overlay, &options);

        assert!(base["name"].is_null());
        assert_eq!(base["value"], 100);
    }

    #[test]
    fn test_deep_merge_add_new_keys() {
        let mut base = json!({
            "existing": "value"
        });

        let overlay = json!({
            "new_key": "new_value",
            "nested": {
                "inner": 123
            }
        });

        deep_merge(&mut base, &overlay);

        assert_eq!(base["existing"], "value");
        assert_eq!(base["new_key"], "new_value");
        assert_eq!(base["nested"]["inner"], 123);
    }

    #[test]
    fn test_deep_merge_type_change() {
        let mut base = json!({
            "value": "string"
        });

        let overlay = json!({
            "value": 123
        });

        deep_merge(&mut base, &overlay);

        assert_eq!(base["value"], 123);
    }
}
