//! YAML configuration loader with include and environment variable support
//!
//! Provides functionality to load YAML configuration files with:
//! - `$include` directives for file composition
//! - `${ENV:default}` for environment variable expansion
//! - Deep merging of configurations
//! - Validation and error handling

use crate::OrchestratorError;
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::env;
use std::fs;
use std::path::Path;

/// Load and parse a YAML file with include support
///
/// # Arguments
///
/// * `path` - Path to the YAML file
///
/// # Returns
///
/// Parsed YAML value with includes resolved and variables expanded
pub fn load_yaml_file<P: AsRef<Path>>(path: P) -> Result<YamlValue, OrchestratorError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|e| {
        OrchestratorError::General(format!("Failed to read YAML file {:?}: {}", path, e))
    })?;

    let mut value: YamlValue = serde_yaml::from_str(&content).map_err(|e| {
        OrchestratorError::General(format!("Failed to parse YAML file {:?}: {}", path, e))
    })?;

    // Get the directory for resolving relative includes
    let base_dir = path
        .parent()
        .ok_or_else(|| OrchestratorError::General("Invalid file path".to_string()))?;

    // Process includes and expand variables
    process_includes(&mut value, base_dir)?;
    expand_variables(&mut value)?;

    Ok(value)
}

/// Load and deserialize a YAML file into a specific type
///
/// # Arguments
///
/// * `path` - Path to the YAML file
///
/// # Returns
///
/// Deserialized configuration object
pub fn load_yaml_config<T: DeserializeOwned, P: AsRef<Path>>(
    path: P,
) -> Result<T, OrchestratorError> {
    let yaml = load_yaml_file(path)?;

    // Convert YAML to JSON for easier deserialization
    let json = yaml_to_json(&yaml)?;

    serde_json::from_value(json).map_err(|e| {
        OrchestratorError::General(format!("Failed to deserialize configuration: {}", e))
    })
}

/// Process $include directives recursively
fn process_includes(value: &mut YamlValue, base_dir: &Path) -> Result<(), OrchestratorError> {
    match value {
        YamlValue::Mapping(map) => {
            // Check for $include directive
            if let Some(YamlValue::String(include_path)) = map.get(&YamlValue::String("$include".to_string())) {
                // Load the included file
                let include_full_path = base_dir.join(include_path);
                let included = load_yaml_file(&include_full_path)?;

                // Replace current value with included content
                *value = included;
                return Ok(());
            }

            // Recursively process all values in the mapping
            for (_, v) in map.iter_mut() {
                process_includes(v, base_dir)?;
            }
        }
        YamlValue::Sequence(seq) => {
            // Recursively process all items in sequence
            for item in seq.iter_mut() {
                process_includes(item, base_dir)?;
            }
        }
        _ => {}
    }

    Ok(())
}

/// Expand environment variables in the format ${ENV_VAR:default}
fn expand_variables(value: &mut YamlValue) -> Result<(), OrchestratorError> {
    match value {
        YamlValue::String(s) => {
            if let Some(expanded) = expand_env_in_string(s) {
                *s = expanded;
            }
        }
        YamlValue::Mapping(map) => {
            for (_, v) in map.iter_mut() {
                expand_variables(v)?;
            }
        }
        YamlValue::Sequence(seq) => {
            for item in seq.iter_mut() {
                expand_variables(item)?;
            }
        }
        _ => {}
    }

    Ok(())
}

/// Expand environment variables in a string
///
/// Supports syntax: ${ENV_VAR:default_value}
fn expand_env_in_string(s: &str) -> Option<String> {
    if !s.contains("${") {
        return None;
    }

    let mut result = s.to_string();
    let re = regex::Regex::new(r"\$\{([^:}]+)(?::([^}]*))?\}").ok()?;

    for cap in re.captures_iter(s) {
        let full_match = cap.get(0)?.as_str();
        let var_name = cap.get(1)?.as_str();
        let default_value = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        let value = env::var(var_name).unwrap_or_else(|_| default_value.to_string());
        result = result.replace(full_match, &value);
    }

    Some(result)
}

/// Convert YAML value to JSON value for easier deserialization
fn yaml_to_json(yaml: &YamlValue) -> Result<JsonValue, OrchestratorError> {
    match yaml {
        YamlValue::Null => Ok(JsonValue::Null),
        YamlValue::Bool(b) => Ok(JsonValue::Bool(*b)),
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(JsonValue::Number(i.into()))
            } else if let Some(u) = n.as_u64() {
                Ok(JsonValue::Number(u.into()))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(JsonValue::Number)
                    .ok_or_else(|| {
                        OrchestratorError::General(format!("Invalid number: {}", f))
                    })
            } else {
                Err(OrchestratorError::General("Invalid number".to_string()))
            }
        }
        YamlValue::String(s) => Ok(JsonValue::String(s.clone())),
        YamlValue::Sequence(seq) => {
            let json_seq: Result<Vec<JsonValue>, _> = seq.iter().map(yaml_to_json).collect();
            Ok(JsonValue::Array(json_seq?))
        }
        YamlValue::Mapping(map) => {
            let mut json_map = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    YamlValue::String(s) => s.clone(),
                    _ => {
                        return Err(OrchestratorError::General(
                            "Map keys must be strings".to_string(),
                        ))
                    }
                };
                json_map.insert(key, yaml_to_json(v)?);
            }
            Ok(JsonValue::Object(json_map))
        }
        YamlValue::Tagged(tagged) => {
            // Handle tagged values by converting the inner value
            yaml_to_json(&tagged.value)
        }
    }
}

/// Merge two YAML values deeply
///
/// For objects, merges keys recursively. For other types, `other` overrides `base`.
pub fn deep_merge(base: &mut YamlValue, other: &YamlValue) {
    match (base, other) {
        (YamlValue::Mapping(base_map), YamlValue::Mapping(other_map)) => {
            for (key, other_value) in other_map {
                if let Some(base_value) = base_map.get_mut(key) {
                    deep_merge(base_value, other_value);
                } else {
                    base_map.insert(key.clone(), other_value.clone());
                }
            }
        }
        (base, other) => {
            *base = other.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_expand_env_in_string() {
        env::set_var("TEST_VAR", "test_value");

        let result = expand_env_in_string("prefix ${TEST_VAR} suffix");
        assert_eq!(result, Some("prefix test_value suffix".to_string()));

        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_expand_env_with_default() {
        let result = expand_env_in_string("value: ${MISSING_VAR:default_val}");
        assert_eq!(result, Some("value: default_val".to_string()));
    }

    #[test]
    fn test_yaml_to_json() {
        let yaml_str = r#"
            string: "hello"
            number: 42
            bool: true
            null_val: null
            array: [1, 2, 3]
            object:
              nested: "value"
        "#;

        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let json = yaml_to_json(&yaml).unwrap();

        assert!(json.is_object());
        assert_eq!(json["string"], "hello");
        assert_eq!(json["number"], 42);
        assert_eq!(json["bool"], true);
        assert!(json["null_val"].is_null());
        assert_eq!(json["array"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_deep_merge() {
        let mut base: YamlValue = serde_yaml::from_str(
            r#"
            a: 1
            b:
              c: 2
              d: 3
        "#,
        )
        .unwrap();

        let other: YamlValue = serde_yaml::from_str(
            r#"
            b:
              c: 20
              e: 4
            f: 5
        "#,
        )
        .unwrap();

        deep_merge(&mut base, &other);

        let json = yaml_to_json(&base).unwrap();
        assert_eq!(json["a"], 1);
        assert_eq!(json["b"]["c"], 20);
        assert_eq!(json["b"]["d"], 3);
        assert_eq!(json["b"]["e"], 4);
        assert_eq!(json["f"], 5);
    }

    #[test]
    fn test_load_yaml_file() -> Result<(), Box<dyn std::error::Error>> {
        let mut temp_file = NamedTempFile::new()?;
        write!(
            temp_file,
            r#"
test: "value"
number: 42
        "#
        )?;

        let yaml = load_yaml_file(temp_file.path())?;
        let json = yaml_to_json(&yaml)?;

        assert_eq!(json["test"], "value");
        assert_eq!(json["number"], 42);

        Ok(())
    }
}
