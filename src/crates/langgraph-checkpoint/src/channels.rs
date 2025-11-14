//! Channel abstractions for state management

use crate::error::{CheckpointError, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Base trait for all channels
///
/// Channels are typed state containers that manage how state is stored,
/// updated, and checkpointed. Different channel types provide different
/// semantics for handling concurrent updates.
pub trait Channel: Send + Sync + Debug {
    /// Get the current value of the channel
    ///
    /// # Errors
    ///
    /// Returns `EmptyChannelError` if the channel has never been updated
    fn get(&self) -> Result<serde_json::Value>;

    /// Update the channel with a sequence of values
    ///
    /// The order of values is arbitrary. This is called at the end of each step.
    /// Returns `true` if the channel was updated, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns `InvalidUpdateError` if the sequence of updates is invalid
    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<bool>;

    /// Create a checkpoint of the current channel state
    ///
    /// Returns a serializable representation of the channel's state.
    fn checkpoint(&self) -> Result<serde_json::Value>;

    /// Restore the channel from a checkpoint
    fn from_checkpoint(&mut self, checkpoint: serde_json::Value) -> Result<()>;

    /// Check if the channel has a value (is not empty)
    fn is_available(&self) -> bool {
        self.get().is_ok()
    }

    /// Notify the channel that a subscribed task ran
    ///
    /// Returns `true` if the channel was updated, `false` otherwise.
    fn consume(&mut self) -> bool {
        false
    }

    /// Notify the channel that the Pregel run is finishing
    ///
    /// Returns `true` if the channel was updated, `false` otherwise.
    fn finish(&mut self) -> bool {
        false
    }

    /// Clone the channel into a Box
    fn clone_box(&self) -> Box<dyn Channel>;
}

/// LastValue channel - stores only the latest value
///
/// Can receive at most one value per step. If multiple values are
/// provided in a single update, it's an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastValueChannel {
    value: Option<serde_json::Value>,
}

impl LastValueChannel {
    /// Create a new LastValue channel
    pub fn new() -> Self {
        Self { value: None }
    }

    /// Create a new LastValue channel with an initial value
    pub fn with_value(value: serde_json::Value) -> Self {
        Self { value: Some(value) }
    }
}

impl Default for LastValueChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Channel for LastValueChannel {
    fn get(&self) -> Result<serde_json::Value> {
        self.value
            .clone()
            .ok_or_else(|| CheckpointError::Invalid("Channel is empty".to_string()))
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<bool> {
        if values.is_empty() {
            return Ok(false);
        }
        if values.len() > 1 {
            return Err(CheckpointError::Invalid(
                "LastValue channel can receive only one value per step".to_string(),
            ));
        }
        self.value = Some(values.into_iter().last().unwrap());
        Ok(true)
    }

    fn checkpoint(&self) -> Result<serde_json::Value> {
        self.value
            .clone()
            .ok_or_else(|| CheckpointError::Invalid("Channel is empty".to_string()))
    }

    fn from_checkpoint(&mut self, checkpoint: serde_json::Value) -> Result<()> {
        self.value = Some(checkpoint);
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.value.is_some()
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        Box::new(self.clone())
    }
}

/// Topic channel - append-only log of values
///
/// Accumulates all values received. Each update appends to the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicChannel {
    values: Vec<serde_json::Value>,
}

impl TopicChannel {
    /// Create a new Topic channel
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Get all accumulated values
    pub fn get_all(&self) -> &[serde_json::Value] {
        &self.values
    }
}

impl Default for TopicChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Channel for TopicChannel {
    fn get(&self) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Array(self.values.clone()))
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<bool> {
        if values.is_empty() {
            return Ok(false);
        }
        self.values.extend(values);
        Ok(true)
    }

    fn checkpoint(&self) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Array(self.values.clone()))
    }

    fn from_checkpoint(&mut self, checkpoint: serde_json::Value) -> Result<()> {
        if let serde_json::Value::Array(arr) = checkpoint {
            self.values = arr;
            Ok(())
        } else {
            Err(CheckpointError::Invalid(
                "Topic channel checkpoint must be an array".to_string(),
            ))
        }
    }

    fn is_available(&self) -> bool {
        !self.values.is_empty()
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        Box::new(self.clone())
    }
}

/// Reducer function type for BinaryOperator channel
pub type ReducerFn = Box<dyn Fn(serde_json::Value, serde_json::Value) -> serde_json::Value + Send + Sync>;

/// BinaryOperator channel - reduces multiple values with a custom operator
///
/// Uses a binary reduction function to combine multiple updates into a single value.
pub struct BinaryOperatorChannel {
    value: Option<serde_json::Value>,
    reducer: ReducerFn,
}

impl BinaryOperatorChannel {
    /// Create a new BinaryOperator channel with a custom reducer
    pub fn new<F>(reducer: F) -> Self
    where
        F: Fn(serde_json::Value, serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        Self {
            value: None,
            reducer: Box::new(reducer),
        }
    }

    /// Create a sum reducer (for numbers)
    pub fn sum() -> Self {
        Self::new(|a, b| {
            let a_num = a.as_f64().unwrap_or(0.0);
            let b_num = b.as_f64().unwrap_or(0.0);
            serde_json::json!(a_num + b_num)
        })
    }

    /// Create an append reducer (for arrays)
    pub fn append() -> Self {
        Self::new(|a, b| {
            let mut result = if let serde_json::Value::Array(arr) = a {
                arr
            } else {
                vec![a]
            };

            if let serde_json::Value::Array(arr) = b {
                result.extend(arr);
            } else {
                result.push(b);
            }

            serde_json::Value::Array(result)
        })
    }
}

impl Debug for BinaryOperatorChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinaryOperatorChannel")
            .field("value", &self.value)
            .field("reducer", &"<function>")
            .finish()
    }
}

impl Channel for BinaryOperatorChannel {
    fn get(&self) -> Result<serde_json::Value> {
        self.value
            .clone()
            .ok_or_else(|| CheckpointError::Invalid("Channel is empty".to_string()))
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<bool> {
        if values.is_empty() {
            return Ok(false);
        }

        let reduced = values.into_iter().reduce(|acc, val| (self.reducer)(acc, val));

        if let Some(new_value) = reduced {
            self.value = if let Some(current) = &self.value {
                Some((self.reducer)(current.clone(), new_value))
            } else {
                Some(new_value)
            };
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn checkpoint(&self) -> Result<serde_json::Value> {
        self.value
            .clone()
            .ok_or_else(|| CheckpointError::Invalid("Channel is empty".to_string()))
    }

    fn from_checkpoint(&mut self, checkpoint: serde_json::Value) -> Result<()> {
        self.value = Some(checkpoint);
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.value.is_some()
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        // Note: We can't easily clone the reducer function, so this is a limitation
        // For now, we'll create a new channel with a default reducer
        // In practice, channels should be created fresh from checkpoints
        panic!("BinaryOperatorChannel cannot be cloned directly")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_last_value_channel() {
        let mut channel = LastValueChannel::new();
        assert!(!channel.is_available());

        // Update with single value
        let updated = channel.update(vec![serde_json::json!(42)]).unwrap();
        assert!(updated);
        assert!(channel.is_available());
        assert_eq!(channel.get().unwrap(), serde_json::json!(42));

        // Update with another value
        channel.update(vec![serde_json::json!(100)]).unwrap();
        assert_eq!(channel.get().unwrap(), serde_json::json!(100));
    }

    #[test]
    fn test_last_value_channel_rejects_multiple() {
        let mut channel = LastValueChannel::new();
        let result = channel.update(vec![serde_json::json!(1), serde_json::json!(2)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_topic_channel() {
        let mut channel = TopicChannel::new();
        assert!(!channel.is_available());

        // Add first batch
        channel.update(vec![serde_json::json!(1), serde_json::json!(2)]).unwrap();
        assert_eq!(channel.get_all().len(), 2);

        // Add second batch
        channel.update(vec![serde_json::json!(3)]).unwrap();
        assert_eq!(channel.get_all().len(), 3);

        let values = channel.get().unwrap();
        assert_eq!(
            values,
            serde_json::json!([1, 2, 3])
        );
    }

    #[test]
    fn test_binary_operator_sum() {
        let mut channel = BinaryOperatorChannel::sum();

        // Update with multiple values - should sum them
        channel.update(vec![serde_json::json!(1.0), serde_json::json!(2.0), serde_json::json!(3.0)]).unwrap();
        assert_eq!(channel.get().unwrap(), serde_json::json!(6.0));

        // Another update - should add to existing
        channel.update(vec![serde_json::json!(4.0)]).unwrap();
        assert_eq!(channel.get().unwrap(), serde_json::json!(10.0));
    }

    #[test]
    fn test_binary_operator_append() {
        let mut channel = BinaryOperatorChannel::append();

        // Update with multiple values - should append them
        channel.update(vec![serde_json::json!(1), serde_json::json!(2)]).unwrap();
        assert_eq!(channel.get().unwrap(), serde_json::json!([1, 2]));

        // Another update - should append to existing
        channel.update(vec![serde_json::json!(3)]).unwrap();
        assert_eq!(channel.get().unwrap(), serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_checkpoint_restore() {
        let mut channel = LastValueChannel::new();
        channel.update(vec![serde_json::json!(42)]).unwrap();

        let checkpoint = channel.checkpoint().unwrap();

        let mut channel2 = LastValueChannel::new();
        channel2.from_checkpoint(checkpoint).unwrap();

        assert_eq!(channel2.get().unwrap(), serde_json::json!(42));
    }
}
