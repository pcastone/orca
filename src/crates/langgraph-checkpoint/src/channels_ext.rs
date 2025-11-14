//! Additional channel types for Pregel execution.
//!
//! This module contains the extended channel types:
//! - EphemeralValueChannel
//! - AnyValueChannel
//! - UntrackedValueChannel
//! - NamedBarrierValueChannel

use crate::error::{CheckpointError, Result};
use crate::channels::Channel;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// EphemeralValue channel - stores value temporarily, clears each superstep.
///
/// This channel stores a value received in the step immediately preceding,
/// then clears it. Useful for temporary state that shouldn't persist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemeralValueChannel {
    value: Option<serde_json::Value>,
    guard: bool,
}

impl EphemeralValueChannel {
    /// Create a new EphemeralValue channel.
    pub fn new() -> Self {
        Self {
            value: None,
            guard: true,
        }
    }

    /// Create a new EphemeralValue channel without guard.
    pub fn new_unguarded() -> Self {
        Self {
            value: None,
            guard: false,
        }
    }
}

impl Default for EphemeralValueChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Channel for EphemeralValueChannel {
    fn get(&self) -> Result<serde_json::Value> {
        self.value
            .clone()
            .ok_or_else(|| CheckpointError::Invalid("Channel is empty".to_string()))
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<bool> {
        if values.is_empty() {
            // Clear the value
            if self.value.is_some() {
                self.value = None;
                return Ok(true);
            }
            return Ok(false);
        }

        if values.len() > 1 && self.guard {
            return Err(CheckpointError::Invalid(
                "EphemeralValue(guard=true) can receive only one value per step".to_string(),
            ));
        }

        self.value = values.last().cloned();
        Ok(true)
    }

    fn checkpoint(&self) -> Result<serde_json::Value> {
        Ok(self.value.clone().unwrap_or(serde_json::Value::Null))
    }

    fn from_checkpoint(&mut self, checkpoint: serde_json::Value) -> Result<()> {
        if !checkpoint.is_null() {
            self.value = Some(checkpoint);
        }
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.value.is_some()
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        Box::new(self.clone())
    }
}

/// AnyValue channel - stores last value, assumes all values are equal.
///
/// If multiple values are received, takes the last one.
/// Useful when you expect only one write but want to be permissive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnyValueChannel {
    value: Option<serde_json::Value>,
}

impl AnyValueChannel {
    /// Create a new AnyValue channel.
    pub fn new() -> Self {
        Self { value: None }
    }
}

impl Default for AnyValueChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Channel for AnyValueChannel {
    fn get(&self) -> Result<serde_json::Value> {
        self.value
            .clone()
            .ok_or_else(|| CheckpointError::Invalid("Channel is empty".to_string()))
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<bool> {
        if values.is_empty() {
            if self.value.is_some() {
                self.value = None;
                return Ok(true);
            }
            return Ok(false);
        }

        self.value = values.last().cloned();
        Ok(true)
    }

    fn checkpoint(&self) -> Result<serde_json::Value> {
        Ok(self.value.clone().unwrap_or(serde_json::Value::Null))
    }

    fn from_checkpoint(&mut self, checkpoint: serde_json::Value) -> Result<()> {
        if !checkpoint.is_null() {
            self.value = Some(checkpoint);
        }
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.value.is_some()
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        Box::new(self.clone())
    }
}

/// UntrackedValue channel - stores value but never checkpoints.
///
/// Useful for transient state that shouldn't be persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntrackedValueChannel {
    #[serde(skip)]
    value: Option<serde_json::Value>,
    guard: bool,
}

impl UntrackedValueChannel {
    /// Create a new UntrackedValue channel.
    pub fn new() -> Self {
        Self {
            value: None,
            guard: true,
        }
    }

    /// Create a new UntrackedValue channel without guard.
    pub fn new_unguarded() -> Self {
        Self {
            value: None,
            guard: false,
        }
    }
}

impl Default for UntrackedValueChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Channel for UntrackedValueChannel {
    fn get(&self) -> Result<serde_json::Value> {
        self.value
            .clone()
            .ok_or_else(|| CheckpointError::Invalid("Channel is empty".to_string()))
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<bool> {
        if values.is_empty() {
            return Ok(false);
        }

        if values.len() > 1 && self.guard {
            return Err(CheckpointError::Invalid(
                "UntrackedValue(guard=true) can receive only one value per step".to_string(),
            ));
        }

        self.value = values.last().cloned();
        Ok(true)
    }

    fn checkpoint(&self) -> Result<serde_json::Value> {
        // Never checkpoint - return null
        Ok(serde_json::Value::Null)
    }

    fn from_checkpoint(&mut self, _checkpoint: serde_json::Value) -> Result<()> {
        // Never restore from checkpoint
        self.value = None;
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.value.is_some()
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        Box::new(self.clone())
    }
}

/// NamedBarrierValue channel - waits for all named values before becoming available.
///
/// This channel collects values from named sources and only becomes available
/// when all expected names have written.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedBarrierValueChannel {
    names: HashSet<String>,
    seen: HashSet<String>,
}

impl NamedBarrierValueChannel {
    /// Create a new NamedBarrierValue channel with expected names.
    pub fn new(names: HashSet<String>) -> Self {
        Self {
            names,
            seen: HashSet::new(),
        }
    }
}

impl Channel for NamedBarrierValueChannel {
    fn get(&self) -> Result<serde_json::Value> {
        if self.seen != self.names {
            return Err(CheckpointError::Invalid(
                "Not all barrier values received yet".to_string(),
            ));
        }
        Ok(serde_json::Value::Null)
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<bool> {
        let mut updated = false;

        for value in values {
            if let Some(name) = value.as_str() {
                if self.names.contains(name) {
                    if !self.seen.contains(name) {
                        self.seen.insert(name.to_string());
                        updated = true;
                    }
                } else {
                    return Err(CheckpointError::Invalid(format!(
                        "Value '{}' not in expected names",
                        name
                    )));
                }
            } else {
                return Err(CheckpointError::Invalid(
                    "NamedBarrierValue expects string values".to_string(),
                ));
            }
        }

        Ok(updated)
    }

    fn checkpoint(&self) -> Result<serde_json::Value> {
        let seen_vec: Vec<_> = self.seen.iter().cloned().collect();
        Ok(serde_json::json!(seen_vec))
    }

    fn from_checkpoint(&mut self, checkpoint: serde_json::Value) -> Result<()> {
        if let Some(arr) = checkpoint.as_array() {
            self.seen = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.seen == self.names
    }

    fn consume(&mut self) -> bool {
        if self.seen == self.names {
            self.seen.clear();
            true
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ephemeral_channel() {
        let mut channel = EphemeralValueChannel::new();
        assert!(!channel.is_available());

        // Set value
        channel.update(vec![serde_json::json!(42)]).unwrap();
        assert_eq!(channel.get().unwrap(), serde_json::json!(42));

        // Clear on empty update
        channel.update(vec![]).unwrap();
        assert!(!channel.is_available());
    }

    #[test]
    fn test_any_value_channel() {
        let mut channel = AnyValueChannel::new();

        // Can receive multiple values (takes last)
        channel
            .update(vec![serde_json::json!(1), serde_json::json!(2)])
            .unwrap();
        assert_eq!(channel.get().unwrap(), serde_json::json!(2));
    }

    #[test]
    fn test_untracked_channel() {
        let mut channel = UntrackedValueChannel::new();
        channel.update(vec![serde_json::json!(42)]).unwrap();

        // Checkpoint returns null
        let checkpoint = channel.checkpoint().unwrap();
        assert!(checkpoint.is_null());

        // Restore doesn't set value
        let mut channel2 = UntrackedValueChannel::new();
        channel2.from_checkpoint(checkpoint).unwrap();
        assert!(!channel2.is_available());
    }

    #[test]
    fn test_named_barrier_channel() {
        let mut names = HashSet::new();
        names.insert("task_a".to_string());
        names.insert("task_b".to_string());

        let mut channel = NamedBarrierValueChannel::new(names);
        assert!(!channel.is_available());

        // Add first name
        channel.update(vec![serde_json::json!("task_a")]).unwrap();
        assert!(!channel.is_available()); // Still waiting for task_b

        // Add second name
        channel.update(vec![serde_json::json!("task_b")]).unwrap();
        assert!(channel.is_available()); // Now all barriers met

        // Consume should clear
        assert!(channel.consume());
        assert!(!channel.is_available());
    }

    #[test]
    fn test_named_barrier_invalid_name() {
        let mut names = HashSet::new();
        names.insert("task_a".to_string());

        let mut channel = NamedBarrierValueChannel::new(names);

        // Try to add unknown name
        let result = channel.update(vec![serde_json::json!("task_b")]);
        assert!(result.is_err());
    }
}
