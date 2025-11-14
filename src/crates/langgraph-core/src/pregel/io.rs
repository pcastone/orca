//! I/O operations for Pregel execution.
//!
//! Functions for mapping inputs/outputs to channel writes/reads.

use crate::error::{GraphError, Result};
use langgraph_checkpoint::Channel;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Read a single channel value.
///
/// Returns `None` if the channel is empty or doesn't exist.
pub fn read_channel(
    channels: &HashMap<String, Box<dyn Channel>>,
    chan: &str,
) -> Option<Value> {
    channels.get(chan).and_then(|channel| channel.get().ok())
}

/// Read values from multiple channels.
///
/// If `keys` contains a single key, returns the value directly.
/// If `keys` contains multiple keys, returns a JSON object with key-value pairs.
/// Skips empty channels by default.
pub fn read_channels(
    channels: &HashMap<String, Box<dyn Channel>>,
    keys: &[String],
) -> Result<Value> {
    if keys.len() == 1 {
        Ok(read_channel(channels, &keys[0]).unwrap_or(Value::Null))
    } else {
        let mut values = Map::new();
        for key in keys {
            if let Some(value) = read_channel(channels, key) {
                values.insert(key.clone(), value);
            }
        }
        Ok(Value::Object(values))
    }
}

/// Map output values from channels after writes.
///
/// Checks if any pending writes affected the output channels, and if so,
/// reads and returns the current values of those channels.
///
/// # Arguments
///
/// * `output_keys` - Channel keys to include in output
/// * `pending_writes` - Either `true` to force output, or a list of (channel, value) tuples
/// * `channels` - Current channel state
///
/// # Returns
///
/// Returns `Some(value)` if output should be emitted, `None` otherwise.
/// - For single output key: returns the value directly
/// - For multiple output keys: returns a JSON object with key-value pairs
pub fn map_output_values(
    output_keys: &[String],
    pending_writes: Option<&[(String, Value)]>,
    channels: &HashMap<String, Box<dyn Channel>>,
) -> Result<Option<Value>> {
    // Check if we should emit output
    let should_emit = match pending_writes {
        None => true, // Force emit (e.g., at start or after interrupt)
        Some(writes) => {
            // Check if any write affected an output channel
            writes.iter().any(|(chan, _)| output_keys.contains(chan))
        }
    };

    if !should_emit {
        return Ok(None);
    }

    // Read current values from output channels
    if output_keys.len() == 1 {
        let value = read_channel(channels, &output_keys[0]).unwrap_or(Value::Null);
        Ok(Some(value))
    } else {
        let mut values = Map::new();
        for key in output_keys {
            if let Some(value) = read_channel(channels, key) {
                values.insert(key.clone(), value);
            }
        }
        Ok(Some(Value::Object(values)))
    }
}

/// Map output updates from task execution.
///
/// Takes completed tasks and their writes, and produces a node-by-node
/// summary of what was updated.
///
/// # Arguments
///
/// * `output_keys` - Channel keys to include in output
/// * `tasks_and_writes` - List of (task_name, writes) tuples
///
/// # Returns
///
/// Returns a JSON object mapping node names to their outputs.
/// - If a node wrote to multiple output channels once each: `{"node": {"key1": val1, "key2": val2}}`
/// - If a node wrote to a channel multiple times: `{"node": [{"key": val1}, {"key": val2}]}`
/// - If a node produced no output: `{"node": null}`
pub fn map_output_updates(
    output_keys: &[String],
    tasks_and_writes: &[(String, Vec<(String, Value)>)],
) -> Result<Option<Value>> {
    if tasks_and_writes.is_empty() {
        return Ok(None);
    }

    let mut grouped: Map<String, Value> = Map::new();

    for (task_name, writes) in tasks_and_writes {
        let mut node_updates: Vec<Value> = Vec::new();

        if output_keys.len() == 1 {
            // Single output key - collect all values for that key
            let key = &output_keys[0];
            for (chan, value) in writes {
                if chan == key {
                    node_updates.push(value.clone());
                }
            }
        } else {
            // Multiple output keys
            // Count how many times each key was written
            let mut counts: HashMap<&str, usize> = HashMap::new();
            for (chan, _) in writes {
                if output_keys.contains(chan) {
                    *counts.entry(chan.as_str()).or_insert(0) += 1;
                }
            }

            // If any key written multiple times, emit separate objects per write
            if counts.values().any(|&count| count > 1) {
                for (chan, value) in writes {
                    if output_keys.contains(chan) {
                        let mut obj = Map::new();
                        obj.insert(chan.clone(), value.clone());
                        node_updates.push(Value::Object(obj));
                    }
                }
            } else {
                // Each key written at most once - combine into single object
                let mut obj = Map::new();
                for (chan, value) in writes {
                    if output_keys.contains(chan) {
                        obj.insert(chan.clone(), value.clone());
                    }
                }
                if !obj.is_empty() {
                    node_updates.push(Value::Object(obj));
                }
            }
        }

        // Set the node's output
        let node_value = match node_updates.len() {
            0 => Value::Null,
            1 => node_updates.into_iter().next().unwrap(),
            _ => Value::Array(node_updates),
        };

        grouped.insert(task_name.clone(), node_value);
    }

    Ok(Some(Value::Object(grouped)))
}

/// Map user input to channel writes.
///
/// Converts user input into a list of pending writes to input channels.
pub fn map_input(
    input: &Value,
    input_keys: &[String],
) -> Result<Vec<(String, Value)>> {
    if input.is_null() {
        return Ok(vec![]);
    }

    if input_keys.len() == 1 {
        // Single input channel - accept input directly
        Ok(vec![(input_keys[0].clone(), input.clone())])
    } else {
        // Multiple input channels - input must be an object
        match input {
            Value::Object(map) => {
                let mut writes = Vec::new();
                for key in input_keys {
                    if let Some(value) = map.get(key) {
                        writes.push((key.clone(), value.clone()));
                    }
                }
                Ok(writes)
            }
            _ => Err(GraphError::Execution(
                format!("Expected input to be an object for multiple input channels, got {}",
                    serde_json::to_string(input).unwrap_or_else(|_| "?".to_string()))
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use langgraph_checkpoint::channels::LastValueChannel;

    #[test]
    fn test_read_channel_empty() {
        let channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let result = read_channel(&channels, "foo");
        assert!(result.is_none());
    }

    #[test]
    fn test_read_channel_with_value() {
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut chan = LastValueChannel::new();
        chan.update(vec![serde_json::json!({"data": "test"})]).unwrap();
        channels.insert("foo".to_string(), Box::new(chan));

        let result = read_channel(&channels, "foo");
        assert_eq!(result, Some(serde_json::json!({"data": "test"})));
    }

    #[test]
    fn test_read_channels_single_key() {
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut chan = LastValueChannel::new();
        chan.update(vec![serde_json::json!(42)]).unwrap();
        channels.insert("value".to_string(), Box::new(chan));

        let result = read_channels(&channels, &["value".to_string()]).unwrap();
        assert_eq!(result, serde_json::json!(42));
    }

    #[test]
    fn test_read_channels_multiple_keys() {
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();

        let mut chan1 = LastValueChannel::new();
        chan1.update(vec![serde_json::json!(10)]).unwrap();
        channels.insert("a".to_string(), Box::new(chan1));

        let mut chan2 = LastValueChannel::new();
        chan2.update(vec![serde_json::json!(20)]).unwrap();
        channels.insert("b".to_string(), Box::new(chan2));

        let result = read_channels(&channels, &["a".to_string(), "b".to_string()]).unwrap();
        assert_eq!(result, serde_json::json!({"a": 10, "b": 20}));
    }

    #[test]
    fn test_map_output_values_no_writes() {
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut chan = LastValueChannel::new();
        chan.update(vec![serde_json::json!({"value": 42})]).unwrap();
        channels.insert("state".to_string(), Box::new(chan));

        // Empty writes - should not emit
        let result = map_output_values(
            &["state".to_string()],
            Some(&[]),
            &channels
        ).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_map_output_values_force_emit() {
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut chan = LastValueChannel::new();
        chan.update(vec![serde_json::json!({"value": 42})]).unwrap();
        channels.insert("state".to_string(), Box::new(chan));

        // None = force emit
        let result = map_output_values(
            &["state".to_string()],
            None,
            &channels
        ).unwrap();
        assert_eq!(result, Some(serde_json::json!({"value": 42})));
    }

    #[test]
    fn test_map_output_values_with_writes() {
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut chan = LastValueChannel::new();
        chan.update(vec![serde_json::json!({"count": 1})]).unwrap();
        channels.insert("counter".to_string(), Box::new(chan));

        // Writes to counter - should emit
        let writes = vec![("counter".to_string(), serde_json::json!({"count": 1}))];
        let result = map_output_values(
            &["counter".to_string()],
            Some(&writes),
            &channels
        ).unwrap();
        assert_eq!(result, Some(serde_json::json!({"count": 1})));
    }

    #[test]
    fn test_map_output_values_writes_to_other_channel() {
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut chan1 = LastValueChannel::new();
        chan1.update(vec![serde_json::json!(1)]).unwrap();
        channels.insert("a".to_string(), Box::new(chan1));

        let mut chan2 = LastValueChannel::new();
        chan2.update(vec![serde_json::json!(2)]).unwrap();
        channels.insert("b".to_string(), Box::new(chan2));

        // Writes to 'b', but output_keys only includes 'a' - should not emit
        let writes = vec![("b".to_string(), serde_json::json!(2))];
        let result = map_output_values(
            &["a".to_string()],
            Some(&writes),
            &channels
        ).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_map_output_updates_single_task() {
        let tasks = vec![
            ("node1".to_string(), vec![
                ("output".to_string(), serde_json::json!({"result": 42}))
            ])
        ];

        let result = map_output_updates(&["output".to_string()], &tasks).unwrap();
        let expected = serde_json::json!({
            "node1": {"result": 42}
        });
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_map_output_updates_multiple_tasks() {
        let tasks = vec![
            ("node1".to_string(), vec![
                ("value".to_string(), serde_json::json!(10))
            ]),
            ("node2".to_string(), vec![
                ("value".to_string(), serde_json::json!(20))
            ])
        ];

        let result = map_output_updates(&["value".to_string()], &tasks).unwrap();
        let expected = serde_json::json!({
            "node1": 10,
            "node2": 20
        });
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_map_input_single_key() {
        let input = serde_json::json!({"value": 42});
        let keys = vec!["state".to_string()];

        let result = map_input(&input, &keys).unwrap();
        assert_eq!(result, vec![("state".to_string(), serde_json::json!({"value": 42}))]);
    }

    #[test]
    fn test_map_input_multiple_keys() {
        let input = serde_json::json!({"a": 1, "b": 2});
        let keys = vec!["a".to_string(), "b".to_string()];

        let result = map_input(&input, &keys).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&("a".to_string(), serde_json::json!(1))));
        assert!(result.contains(&("b".to_string(), serde_json::json!(2))));
    }

    #[test]
    fn test_map_input_null() {
        let input = serde_json::json!(null);
        let keys = vec!["state".to_string()];

        let result = map_input(&input, &keys).unwrap();
        assert_eq!(result, vec![]);
    }
}
