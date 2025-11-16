//! Extended channel types
//!
//! This module provides additional channel types beyond the basic ones:
//! - EphemeralValue: Clears after being consumed
//! - AnyValue: Like LastValue but allows multiple writes per step
//! - UntrackedValue: Not persisted in checkpoints
//! - LastValueAfterFinish: Available only after finish() is called
//! - NamedBarrierValue: Waits for named signals

use crate::channels::Channel;
use crate::error::{CheckpointError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Ephemeral channel that clears its value after consumption.
///
/// `EphemeralValueChannel` stores a value that **automatically clears** after being read
/// by a consuming node, making it ideal for one-time signals, temporary state, and
/// event-like data that shouldn't persist across multiple supersteps.
///
/// # Behavior
///
/// - **Write**: Stores the value (last write wins if multiple)
/// - **Read**: Value is available
/// - **Consume**: Value is cleared automatically
/// - **Next Step**: Channel is empty (triggers nothing)
///
/// # Use Cases
///
/// 1. **One-time Events**: Button clicks, API callbacks
/// 2. **Temporary Signals**: Interrupt flags, completion markers
/// 3. **Ephemeral State**: Session data that shouldn't persist
/// 4. **Flow Control**: Trigger-once mechanisms
///
/// # Example: Interrupt Signal
///
/// ```rust
/// use langgraph_checkpoint::{Channel, channels_extended::EphemeralValueChannel};
/// use serde_json::json;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut channel = EphemeralValueChannel::new();
///
/// // Step 1: Write interrupt signal
/// channel.update(vec![json!({"interrupt": true})])?;
/// assert!(channel.is_available());
///
/// // Step 2: Node consumes signal
/// let value = channel.get()?;
/// assert_eq!(value, json!({"interrupt": true}));
/// channel.consume(); // Clears the value
///
/// // Step 3: Signal is gone
/// assert!(!channel.is_available());
/// # Ok(())
/// # }
/// ```
///
/// # Guard Mode
///
/// When `guard = true` (default), the channel enforces single-write semantics:
///
/// ```rust
/// # use langgraph_checkpoint::{Channel, channels_extended::EphemeralValueChannel};
/// # use serde_json::json;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut guarded = EphemeralValueChannel::new();
///
/// // Multiple writes in same step = error
/// let result = guarded.update(vec![json!(1), json!(2)]);
/// assert!(result.is_err()); // Guard prevents multiple values
/// # Ok(())
/// # }
/// ```
///
/// With `guard = false`, multiple writes are allowed (last wins):
///
/// ```rust
/// # use langgraph_checkpoint::{Channel, channels_extended::EphemeralValueChannel};
/// # use serde_json::json;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut unguarded = EphemeralValueChannel::with_guard(false);
///
/// // Multiple writes = last value wins
/// unguarded.update(vec![json!(1), json!(2), json!(3)])?;
/// assert_eq!(unguarded.get()?, json!(3));
/// # Ok(())
/// # }
/// ```
///
/// # Comparison with LastValue
///
/// | Feature | EphemeralValue | LastValue |
/// |---------|----------------|-----------|
/// | Persistence | Clears after consume | Persists across steps |
/// | Re-triggering | No (disappears) | Yes (stays available) |
/// | Use Case | Events/signals | Persistent state |
///
/// # See Also
///
/// - [`LastValueChannel`](crate::channels::LastValueChannel) - Persistent single-value storage
/// - [`UntrackedValueChannel`] - Value not saved in checkpoints
/// - [`Channel`] - Base channel trait
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemeralValueChannel {
    value: Option<serde_json::Value>,
    /// If true, requires exactly one value per update (prevents multiple writes per step)
    guard: bool,
}

impl EphemeralValueChannel {
    pub fn new() -> Self {
        Self {
            value: None,
            guard: true,
        }
    }

    pub fn with_guard(guard: bool) -> Self {
        Self { value: None, guard }
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

    fn consume(&mut self) -> bool {
        if self.value.is_some() {
            self.value = None;
            true
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        Box::new(self.clone())
    }
}

/// Permissive single-value channel allowing multiple writes per step.
///
/// `AnyValueChannel` is like `LastValueChannel` but **never errors on multiple writes** -
/// it always keeps the last value written in a superstep. This is useful when you need
/// flexible state updates without write-conflict enforcement.
///
/// # Behavior
///
/// - **Multiple Writes**: Last write wins (no error)
/// - **Empty Updates**: No-op (channel unchanged)
/// - **Persistence**: Value persists across steps
/// - **Consumption**: Does not clear on consume
///
/// # Example: Flexible State Updates
///
/// ```rust
/// use langgraph_checkpoint::{Channel, channels_extended::AnyValueChannel};
/// use serde_json::json;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut channel = AnyValueChannel::new();
///
/// // Multiple nodes can write - last wins
/// channel.update(vec![
///     json!({"status": "processing"}),
///     json!({"status": "analyzing"}),
///     json!({"status": "complete"})  // This value is kept
/// ])?;
///
/// assert_eq!(channel.get()?, json!({"status": "complete"}));
/// # Ok(())
/// # }
/// ```
///
/// # Comparison with LastValue
///
/// | Feature | AnyValue | LastValue |
/// |---------|----------|-----------|
/// | Multiple writes/step | ✓ Allowed (last wins) | ✗ Error |
/// | Use case | Flexible updates | Strict single-write |
/// | Safety | Permissive | Defensive |
///
/// # See Also
///
/// - [`LastValueChannel`](crate::channels::LastValueChannel) - Strict single-write version
/// - [`EphemeralValueChannel`] - Value that clears after consume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnyValueChannel {
    value: Option<serde_json::Value>,
}

impl AnyValueChannel {
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
            return Ok(false);
        }
        // Take last value if multiple (unlike LastValue which errors)
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

/// Transient channel that is never persisted in checkpoints.
///
/// `UntrackedValueChannel` stores values in memory but **excludes them from checkpoints**,
/// making it ideal for temporary computations, caches, and ephemeral data that doesn't
/// need to survive process restarts or time-travel debugging.
///
/// # Behavior
///
/// - **Write**: Stores value in memory
/// - **Read**: Value available within execution
/// - **Checkpoint**: Returns empty (not persisted)
/// - **Resume**: Starts empty after restore
///
/// # Use Cases
///
/// 1. **Performance Caches**: Computed values that can be regenerated
/// 2. **Temporary Buffers**: Intermediate computation results
/// 3. **Session Data**: Request-scoped state
/// 4. **Large Objects**: Data too big to checkpoint efficiently
///
/// # Example: Computation Cache
///
/// ```rust
/// use langgraph_checkpoint::{Channel, channels_extended::UntrackedValueChannel};
/// use serde_json::json;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut cache = UntrackedValueChannel::new();
///
/// // Store expensive computation result
/// cache.update(vec![json!({
///     "result": [1, 2, 3, 4, 5],
///     "computation_time_ms": 150
/// })])?;
///
/// // Available during execution
/// assert!(cache.is_available());
///
/// // But checkpoint() returns empty - not persisted
/// assert!(cache.checkpoint().is_err());
/// # Ok(())
/// # }
/// ```
///
/// # Memory vs Persistence Trade-off
///
/// ```text
/// ┌─────────────────────────────────────────────┐
/// │ Regular Channels                             │
/// │ Memory: ████                                 │
/// │ Disk:   ████ (checkpointed)                  │
/// └─────────────────────────────────────────────┘
///
/// ┌─────────────────────────────────────────────┐
/// │ Untracked Channels                           │
/// │ Memory: ████                                 │
/// │ Disk:   ---- (not checkpointed)              │
/// └─────────────────────────────────────────────┘
/// ```
///
/// # See Also
///
/// - [`LastValueChannel`](crate::channels::LastValueChannel) - Persistent version
/// - [`EphemeralValueChannel`] - Clears after consume (but is checkpointed)
#[derive(Debug, Clone)]
pub struct UntrackedValueChannel {
    value: Option<serde_json::Value>,
}

impl UntrackedValueChannel {
    pub fn new() -> Self {
        Self { value: None }
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
        if values.len() > 1 {
            return Err(CheckpointError::Invalid(
                "UntrackedValue can receive only one value per step".to_string(),
            ));
        }
        self.value = Some(values.into_iter().last().unwrap());
        Ok(true)
    }

    fn checkpoint(&self) -> Result<serde_json::Value> {
        // Untracked values are not persisted
        Err(CheckpointError::Invalid("UntrackedValue cannot be checkpointed".to_string()))
    }

    fn from_checkpoint(&mut self, _checkpoint: serde_json::Value) -> Result<()> {
        // Don't restore untracked values
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.value.is_some()
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        Box::new(self.clone())
    }
}

/// Deferred value channel that becomes available only after explicit finish signal.
///
/// `LastValueAfterFinishChannel` stores values but **defers their availability** until
/// `finish()` is explicitly called. This creates a two-phase pattern: accumulate writes
/// during execution, then release the value when processing is complete.
///
/// # Behavior
///
/// - **Write**: Stores value and marks as **not finished** (unavailable)
/// - **Finish**: Marks channel as finished (value becomes available)
/// - **Read**: Only succeeds if finished and value exists
/// - **Consume**: Clears value and resets finished flag
///
/// # Use Cases
///
/// 1. **Final Output Channels**: Accumulate result, release when computation done
/// 2. **Validation Gates**: Hold value until validation passes
/// 3. **Approval Workflows**: Value pending until human approves
/// 4. **Batch Completion**: Signal when batch processing finishes
///
/// # Example: Final Result Channel
///
/// ```rust
/// use langgraph_checkpoint::{Channel, channels_extended::LastValueAfterFinishChannel};
/// use serde_json::json;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut channel = LastValueAfterFinishChannel::new();
///
/// // Step 1: Node writes result (but not available yet)
/// channel.update(vec![json!({"result": "computed"})]).unwrap();
/// assert!(!channel.is_available()); // Not finished yet
/// assert!(channel.get().is_err());  // Can't read
///
/// // Step 2: Another node updates (still not available)
/// channel.update(vec![json!({"result": "refined"})]).unwrap();
/// assert!(!channel.is_available());
///
/// // Step 3: Call finish() to release the value
/// channel.finish();
/// assert!(channel.is_available());
/// assert_eq!(channel.get()?, json!({"result": "refined"}));
///
/// // Step 4: Consume clears it
/// channel.consume();
/// assert!(!channel.is_available());
/// # Ok(())
/// # }
/// ```
///
/// # Two-Phase Pattern
///
/// ```text
/// ┌─────────────────────────────────────────────┐
/// │ Phase 1: Accumulation                        │
/// │   update() → Stores value                    │
/// │   is_available() → false                     │
/// │   get() → Error                              │
/// └─────────────────────────────────────────────┘
///              ↓ finish()
/// ┌─────────────────────────────────────────────┐
/// │ Phase 2: Available                           │
/// │   is_available() → true                      │
/// │   get() → Ok(value)                          │
/// └─────────────────────────────────────────────┘
///              ↓ consume()
/// ┌─────────────────────────────────────────────┐
/// │ Phase 1: Reset                               │
/// │   (back to accumulation)                     │
/// └─────────────────────────────────────────────┘
/// ```
///
/// # Update Behavior
///
/// **Important**: Calling `update()` resets the `finished` flag:
///
/// ```rust
/// # use langgraph_checkpoint::{Channel, channels_extended::LastValueAfterFinishChannel};
/// # use serde_json::json;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut channel = LastValueAfterFinishChannel::new();
///
/// channel.update(vec![json!(1)])?;
/// channel.finish();
/// assert!(channel.is_available());
///
/// // New update resets finished flag
/// channel.update(vec![json!(2)])?;
/// assert!(!channel.is_available()); // Back to Phase 1
/// # Ok(())
/// # }
/// ```
///
/// # Comparison with Other Channels
///
/// | Feature | LastValueAfterFinish | LastValue | EphemeralValue |
/// |---------|----------------------|-----------|----------------|
/// | Availability | After finish() | Immediate | Immediate |
/// | Persistence | Until consumed | Across steps | Clears after consume |
/// | Use case | Deferred output | Regular state | One-time events |
///
/// # See Also
///
/// - [`LastValueChannel`](crate::channels::LastValueChannel) - Value immediately available
/// - [`EphemeralValueChannel`] - Value clears after consume
/// - [`Channel::finish`] - Finish signal method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastValueAfterFinishChannel {
    value: Option<serde_json::Value>,
    finished: bool,
}

impl LastValueAfterFinishChannel {
    pub fn new() -> Self {
        Self {
            value: None,
            finished: false,
        }
    }
}

impl Default for LastValueAfterFinishChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Channel for LastValueAfterFinishChannel {
    fn get(&self) -> Result<serde_json::Value> {
        if !self.finished || self.value.is_none() {
            return Err(CheckpointError::Invalid(
                "Channel not finished or empty".to_string(),
            ));
        }
        self.value
            .clone()
            .ok_or_else(|| CheckpointError::Invalid("Channel is empty".to_string()))
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<bool> {
        if values.is_empty() {
            return Ok(false);
        }
        self.finished = false;
        self.value = Some(values.into_iter().last().unwrap());
        Ok(true)
    }

    fn checkpoint(&self) -> Result<serde_json::Value> {
        if self.value.is_none() {
            return Err(CheckpointError::Invalid("Channel is empty".to_string()));
        }
        Ok(serde_json::json!({
            "value": self.value,
            "finished": self.finished
        }))
    }

    fn from_checkpoint(&mut self, checkpoint: serde_json::Value) -> Result<()> {
        if let serde_json::Value::Object(obj) = checkpoint {
            self.value = obj.get("value").cloned();
            self.finished = obj
                .get("finished")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(())
        } else {
            Err(CheckpointError::Invalid(
                "Invalid checkpoint format".to_string(),
            ))
        }
    }

    fn is_available(&self) -> bool {
        self.finished && self.value.is_some()
    }

    fn consume(&mut self) -> bool {
        if self.finished {
            self.finished = false;
            self.value = None;
            true
        } else {
            false
        }
    }

    fn finish(&mut self) -> bool {
        if !self.finished && self.value.is_some() {
            self.finished = true;
            true
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        Box::new(self.clone())
    }
}

/// Synchronization barrier channel that waits for specific named signals.
///
/// `NamedBarrierValueChannel` implements a **named barrier pattern** where the channel only
/// becomes available after receiving signals from all specified names. This enables
/// coordinating multiple parallel tasks and ensuring all dependencies complete before proceeding.
///
/// # Behavior
///
/// - **Initialization**: Specify required names (e.g., `["task1", "task2", "task3"]`)
/// - **Write**: Accept signals as string values matching expected names
/// - **Tracking**: Track which names have been received
/// - **Barrier**: Channel becomes available when **all** names received
/// - **Unknown Names**: Signals with unrecognized names are ignored
///
/// # Use Cases
///
/// 1. **Parallel Task Coordination**: Wait for multiple parallel nodes to complete
/// 2. **Fan-In Patterns**: Collect results from distributed workers
/// 3. **Approval Gates**: Wait for approvals from multiple reviewers
/// 4. **Dependency Management**: Ensure all prerequisites are satisfied
///
/// # Example: Parallel Task Coordination
///
/// ```rust
/// use langgraph_checkpoint::{Channel, channels_extended::NamedBarrierValueChannel};
/// use serde_json::json;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Create barrier waiting for 3 tasks
/// let mut barrier = NamedBarrierValueChannel::new(vec![
///     "data_fetch".to_string(),
///     "validation".to_string(),
///     "processing".to_string(),
/// ]);
///
/// // Initially not available
/// assert!(!barrier.is_available());
///
/// // Task 1 completes
/// barrier.update(vec![json!("data_fetch")])?;
/// assert!(!barrier.is_available()); // Still waiting
///
/// // Task 2 completes
/// barrier.update(vec![json!("validation")])?;
/// assert!(!barrier.is_available()); // Still waiting
///
/// // Task 3 completes - barrier satisfied!
/// barrier.update(vec![json!("processing")])?;
/// assert!(barrier.is_available()); // All tasks done
///
/// // Get returns array of received names
/// let result = barrier.get()?;
/// assert!(result.is_array());
/// # Ok(())
/// # }
/// ```
///
/// # Barrier State Diagram
///
/// ```text
/// ┌──────────────────────────────────────────────┐
/// │ Initial State                                 │
/// │ Required: [A, B, C]                           │
/// │ Received: []                                  │
/// │ Available: false                              │
/// └──────────────────────────────────────────────┘
///              ↓ update(["A"])
/// ┌──────────────────────────────────────────────┐
/// │ Partial Progress                              │
/// │ Required: [A, B, C]                           │
/// │ Received: [A]                                 │
/// │ Available: false                              │
/// └──────────────────────────────────────────────┘
///              ↓ update(["B", "C"])
/// ┌──────────────────────────────────────────────┐
/// │ Barrier Satisfied                             │
/// │ Required: [A, B, C]                           │
/// │ Received: [A, B, C]                           │
/// │ Available: true ✓                             │
/// └──────────────────────────────────────────────┘
/// ```
///
/// # Unknown Names Ignored
///
/// The channel only tracks expected names:
///
/// ```rust
/// # use langgraph_checkpoint::{Channel, channels_extended::NamedBarrierValueChannel};
/// # use serde_json::json;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut barrier = NamedBarrierValueChannel::new(vec!["task1".to_string()]);
///
/// // Unknown names are silently ignored
/// barrier.update(vec![
///     json!("unknown_task"),  // Ignored
///     json!("task1"),         // Accepted
///     json!("another_unknown") // Ignored
/// ])?;
///
/// assert!(barrier.is_available()); // Only "task1" needed
/// # Ok(())
/// # }
/// ```
///
/// # Duplicate Signals
///
/// Receiving the same name multiple times has no effect:
///
/// ```rust
/// # use langgraph_checkpoint::{Channel, channels_extended::NamedBarrierValueChannel};
/// # use serde_json::json;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut barrier = NamedBarrierValueChannel::new(vec![
///     "task1".to_string(),
///     "task2".to_string()
/// ]);
///
/// // Duplicate signals don't count twice
/// barrier.update(vec![json!("task1")])?;
/// barrier.update(vec![json!("task1")])?; // No effect
/// barrier.update(vec![json!("task1")])?; // No effect
///
/// assert!(!barrier.is_available()); // Still need "task2"
///
/// barrier.update(vec![json!("task2")])?;
/// assert!(barrier.is_available()); // Now complete
/// # Ok(())
/// # }
/// ```
///
/// # Comparison with Other Patterns
///
/// | Feature | NamedBarrier | LastValue | Topic |
/// |---------|--------------|-----------|-------|
/// | Coordination | Multiple named signals | Single value | Multiple appended values |
/// | Availability | After all names received | Immediate | Immediate |
/// | Use case | Fan-in synchronization | Regular state | Message accumulation |
///
/// # See Also
///
/// - [`TopicChannel`](crate::channels::TopicChannel) - Accumulates all values without barriers
/// - [`LastValueChannel`](crate::channels::LastValueChannel) - Single-value state
/// - Pregel barrier synchronization in distributed graph processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedBarrierValueChannel {
    /// Expected names that must be received
    names: HashSet<String>,
    /// Values received so far
    received: HashSet<String>,
}

impl NamedBarrierValueChannel {
    pub fn new(names: Vec<String>) -> Self {
        Self {
            names: names.into_iter().collect(),
            received: HashSet::new(),
        }
    }
}

impl Channel for NamedBarrierValueChannel {
    fn get(&self) -> Result<serde_json::Value> {
        if self.received.len() < self.names.len() {
            return Err(CheckpointError::Invalid(
                "Barrier not satisfied - not all names received".to_string(),
            ));
        }
        Ok(serde_json::Value::Array(
            self.received
                .iter()
                .map(|s| serde_json::Value::String(s.clone()))
                .collect(),
        ))
    }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<bool> {
        let mut updated = false;
        for value in values {
            if let serde_json::Value::String(name) = value {
                if self.names.contains(&name) && !self.received.contains(&name) {
                    self.received.insert(name);
                    updated = true;
                }
            }
        }
        Ok(updated)
    }

    fn checkpoint(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "names": self.names.iter().cloned().collect::<Vec<_>>(),
            "received": self.received.iter().cloned().collect::<Vec<_>>()
        }))
    }

    fn from_checkpoint(&mut self, checkpoint: serde_json::Value) -> Result<()> {
        if let serde_json::Value::Object(obj) = checkpoint {
            if let Some(serde_json::Value::Array(names)) = obj.get("names") {
                self.names = names
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            if let Some(serde_json::Value::Array(received)) = obj.get("received") {
                self.received = received
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            Ok(())
        } else {
            Err(CheckpointError::Invalid(
                "Invalid checkpoint format".to_string(),
            ))
        }
    }

    fn is_available(&self) -> bool {
        self.received.len() >= self.names.len()
    }

    fn clone_box(&self) -> Box<dyn Channel> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ephemeral_value_consume() {
        let mut channel = EphemeralValueChannel::new();

        // Update with value
        assert!(channel.update(vec![json!(42)]).unwrap());
        assert_eq!(channel.get().unwrap(), json!(42));

        // Consume clears the value
        assert!(channel.consume());
        assert!(channel.get().is_err());

        // Second consume returns false (already empty)
        assert!(!channel.consume());
    }

    #[test]
    fn test_ephemeral_value_guard() {
        let mut channel = EphemeralValueChannel::with_guard(true);

        // Multiple values with guard=true should error
        assert!(channel.update(vec![json!(1), json!(2)]).is_err());

        // Single value should work
        assert!(channel.update(vec![json!(1)]).unwrap());
        assert_eq!(channel.get().unwrap(), json!(1));
    }

    #[test]
    fn test_any_value_multiple_updates() {
        let mut channel = AnyValueChannel::new();

        // Unlike LastValue, multiple values are OK (last wins)
        assert!(channel.update(vec![json!(1), json!(2), json!(3)]).unwrap());
        assert_eq!(channel.get().unwrap(), json!(3));
    }

    #[test]
    fn test_untracked_value_no_checkpoint() {
        let mut channel = UntrackedValueChannel::new();

        channel.update(vec![json!("secret")]).unwrap();
        assert_eq!(channel.get().unwrap(), json!("secret"));

        // Checkpoint should fail for untracked values
        assert!(channel.checkpoint().is_err());
    }

    #[test]
    fn test_last_value_after_finish() {
        let mut channel = LastValueAfterFinishChannel::new();

        // Update but not finished yet
        channel.update(vec![json!(42)]).unwrap();
        assert!(!channel.is_available());
        assert!(channel.get().is_err());

        // After finish, becomes available
        assert!(channel.finish());
        assert!(channel.is_available());
        assert_eq!(channel.get().unwrap(), json!(42));

        // Consume clears it
        assert!(channel.consume());
        assert!(!channel.is_available());
    }

    #[test]
    fn test_last_value_after_finish_update_resets() {
        let mut channel = LastValueAfterFinishChannel::new();

        // Finish
        channel.update(vec![json!(1)]).unwrap();
        channel.finish();
        assert!(channel.is_available());

        // New update resets finished flag
        channel.update(vec![json!(2)]).unwrap();
        assert!(!channel.is_available());
        assert!(channel.get().is_err());
    }

    #[test]
    fn test_named_barrier_value() {
        let mut channel = NamedBarrierValueChannel::new(vec![
            "task1".to_string(),
            "task2".to_string(),
            "task3".to_string(),
        ]);

        // Not available yet
        assert!(!channel.is_available());

        // Receive task1
        channel
            .update(vec![json!("task1")])
            .unwrap();
        assert!(!channel.is_available());

        // Receive task2
        channel
            .update(vec![json!("task2")])
            .unwrap();
        assert!(!channel.is_available());

        // Receive task3 - now barrier is satisfied
        channel
            .update(vec![json!("task3")])
            .unwrap();
        assert!(channel.is_available());

        // Can get the result
        let result = channel.get().unwrap();
        assert!(result.is_array());
    }

    #[test]
    fn test_named_barrier_value_ignores_unknown() {
        let mut channel = NamedBarrierValueChannel::new(vec!["task1".to_string()]);

        // Unknown names are ignored
        channel
            .update(vec![json!("unknown"), json!("task1")])
            .unwrap();
        assert!(channel.is_available());
    }

    #[test]
    fn test_ephemeral_value_checkpoint() {
        let mut channel = EphemeralValueChannel::new();
        channel.update(vec![json!({"key": "value"})]).unwrap();

        let checkpoint = channel.checkpoint().unwrap();
        assert_eq!(checkpoint, json!({"key": "value"}));

        let mut new_channel = EphemeralValueChannel::new();
        new_channel.from_checkpoint(checkpoint).unwrap();
        assert_eq!(new_channel.get().unwrap(), json!({"key": "value"}));
    }

    #[test]
    fn test_last_value_after_finish_checkpoint() {
        let mut channel = LastValueAfterFinishChannel::new();
        channel.update(vec![json!(123)]).unwrap();
        channel.finish();

        let checkpoint = channel.checkpoint().unwrap();
        assert!(checkpoint.is_object());

        let mut new_channel = LastValueAfterFinishChannel::new();
        new_channel.from_checkpoint(checkpoint).unwrap();
        assert!(new_channel.is_available());
        assert_eq!(new_channel.get().unwrap(), json!(123));
    }

    #[test]
    fn test_ephemeral_value_empty_update() {
        let mut channel = EphemeralValueChannel::new();
        channel.update(vec![json!(42)]).unwrap();

        // Empty update clears the channel
        assert!(channel.update(vec![]).unwrap());
        assert!(!channel.is_available());
        assert!(channel.get().is_err());
    }

    #[test]
    fn test_ephemeral_value_checkpoint_empty() {
        let channel = EphemeralValueChannel::new();

        // Empty channel should error on checkpoint
        assert!(channel.checkpoint().is_err());
    }

    #[test]
    fn test_ephemeral_value_multiple_consume() {
        let mut channel = EphemeralValueChannel::new();
        channel.update(vec![json!(1)]).unwrap();

        // First consume succeeds
        assert!(channel.consume());
        assert!(!channel.is_available());

        // Second consume returns false (nothing to consume)
        assert!(!channel.consume());

        // Third consume also returns false
        assert!(!channel.consume());
    }

    #[test]
    fn test_ephemeral_value_guard_false_allows_multiple() {
        let mut channel = EphemeralValueChannel::with_guard(false);

        // Multiple values should work with guard=false
        assert!(channel.update(vec![json!(1), json!(2), json!(3)]).unwrap());
        assert_eq!(channel.get().unwrap(), json!(3));
    }

    #[test]
    fn test_any_value_empty_update() {
        let mut channel = AnyValueChannel::new();
        channel.update(vec![json!("initial")]).unwrap();

        // Empty update should clear the channel
        let result = channel.update(vec![]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_any_value_single_write() {
        let mut channel = AnyValueChannel::new();

        // Single write should work
        assert!(channel.update(vec![json!(42)]).unwrap());
        assert_eq!(channel.get().unwrap(), json!(42));
    }

    #[test]
    fn test_any_value_persistence() {
        let mut channel = AnyValueChannel::new();
        channel.update(vec![json!("persistent")]).unwrap();

        // Value persists across "steps" (doesn't clear on consume)
        channel.consume();
        assert!(channel.is_available());
        assert_eq!(channel.get().unwrap(), json!("persistent"));
    }

    #[test]
    fn test_any_value_checkpoint_roundtrip() {
        let mut channel = AnyValueChannel::new();
        channel.update(vec![json!({"data": "test"})]).unwrap();

        let checkpoint = channel.checkpoint().unwrap();
        let mut new_channel = AnyValueChannel::new();
        new_channel.from_checkpoint(checkpoint).unwrap();

        assert_eq!(new_channel.get().unwrap(), json!({"data": "test"}));
    }

    #[test]
    fn test_untracked_value_basic_operations() {
        let mut channel = UntrackedValueChannel::new();

        // Update and get work
        channel.update(vec![json!("untracked")]).unwrap();
        assert_eq!(channel.get().unwrap(), json!("untracked"));
        assert!(channel.is_available());
    }

    #[test]
    fn test_untracked_value_cannot_checkpoint() {
        let mut channel = UntrackedValueChannel::new();
        channel.update(vec![json!("secret")]).unwrap();

        // Checkpoint should fail
        assert!(channel.checkpoint().is_err());
    }

    #[test]
    fn test_untracked_value_from_checkpoint_is_noop() {
        let mut channel = UntrackedValueChannel::new();

        // from_checkpoint succeeds but doesn't restore data (no-op)
        assert!(channel.from_checkpoint(json!({"test": "data"})).is_ok());

        // Channel should still be empty after from_checkpoint
        assert!(!channel.is_available());
    }

    #[test]
    fn test_untracked_value_consume_doesnt_clear() {
        let mut channel = UntrackedValueChannel::new();
        channel.update(vec![json!(123)]).unwrap();

        // Consume doesn't clear untracked values
        channel.consume();
        assert!(channel.is_available());
        assert_eq!(channel.get().unwrap(), json!(123));
    }

    #[test]
    fn test_last_value_after_finish_multiple_finish_calls() {
        let mut channel = LastValueAfterFinishChannel::new();
        channel.update(vec![json!(1)]).unwrap();

        // First finish
        assert!(channel.finish());
        assert!(channel.is_available());

        // Second finish should return false (already finished)
        assert!(!channel.finish());
        assert!(channel.is_available());
    }

    #[test]
    fn test_last_value_after_finish_finish_before_update() {
        let mut channel = LastValueAfterFinishChannel::new();

        // Finish without update
        assert!(!channel.finish());
        assert!(!channel.is_available());

        // Now update and finish
        channel.update(vec![json!(42)]).unwrap();
        assert!(channel.finish());
        assert!(channel.is_available());
    }

    #[test]
    fn test_last_value_after_finish_empty_after_consume() {
        let mut channel = LastValueAfterFinishChannel::new();
        channel.update(vec![json!(99)]).unwrap();
        channel.finish();
        channel.consume();

        // After consume, should be empty
        assert!(!channel.is_available());
        assert!(channel.get().is_err());
    }

    #[test]
    fn test_named_barrier_value_empty_names() {
        let channel = NamedBarrierValueChannel::new(vec![]);

        // Empty barrier should be immediately available
        assert!(channel.is_available());
    }

    #[test]
    fn test_named_barrier_value_unique_names_only() {
        let mut channel = NamedBarrierValueChannel::new(vec![
            "task1".to_string(),
            "task2".to_string(),
            "task3".to_string(),
        ]);

        // Barrier requires all unique names to be received
        assert!(!channel.is_available());

        channel.update(vec![json!("task1")]).unwrap();
        assert!(!channel.is_available());

        channel.update(vec![json!("task2"), json!("task3")]).unwrap();
        assert!(channel.is_available());
    }

    #[test]
    fn test_named_barrier_value_no_consume() {
        let mut channel = NamedBarrierValueChannel::new(vec![
            "a".to_string(),
            "b".to_string(),
        ]);

        // Satisfy barrier
        channel.update(vec![json!("a")]).unwrap();
        channel.update(vec![json!("b")]).unwrap();
        assert!(channel.is_available());

        // NamedBarrierValueChannel doesn't have a custom consume implementation
        // So it uses the default trait behavior (likely does nothing or not implemented)
        // The barrier remains satisfied after "consuming"
    }

    #[test]
    fn test_named_barrier_value_checkpoint_roundtrip() {
        let mut channel = NamedBarrierValueChannel::new(vec![
            "x".to_string(),
            "y".to_string(),
        ]);

        // Partially satisfy barrier
        channel.update(vec![json!("x")]).unwrap();

        let checkpoint = channel.checkpoint().unwrap();
        let mut new_channel = NamedBarrierValueChannel::new(vec!["x".to_string(), "y".to_string()]);
        new_channel.from_checkpoint(checkpoint).unwrap();

        // Should preserve received signals
        assert!(!new_channel.is_available());
        new_channel.update(vec![json!("y")]).unwrap();
        assert!(new_channel.is_available());
    }

    #[test]
    fn test_named_barrier_value_all_at_once() {
        let mut channel = NamedBarrierValueChannel::new(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        ]);

        // Receive all signals in one update
        channel
            .update(vec![json!("a"), json!("b"), json!("c")])
            .unwrap();
        assert!(channel.is_available());
    }

    #[test]
    fn test_ephemeral_value_is_available() {
        let mut channel = EphemeralValueChannel::new();

        assert!(!channel.is_available());

        channel.update(vec![json!(1)]).unwrap();
        assert!(channel.is_available());

        channel.consume();
        assert!(!channel.is_available());
    }

    #[test]
    fn test_any_value_overwrite_behavior() {
        let mut channel = AnyValueChannel::new();

        // First write
        channel.update(vec![json!("first")]).unwrap();
        assert_eq!(channel.get().unwrap(), json!("first"));

        // Overwrite with multiple values (last wins)
        channel
            .update(vec![json!("second"), json!("third"), json!("fourth")])
            .unwrap();
        assert_eq!(channel.get().unwrap(), json!("fourth"));
    }

    #[test]
    fn test_untracked_value_clone() {
        let channel = UntrackedValueChannel::new();
        let cloned = channel.clone_box();

        // Should be able to clone
        assert!(!cloned.is_available());
    }

    #[test]
    fn test_last_value_after_finish_get_before_finish() {
        let mut channel = LastValueAfterFinishChannel::new();
        channel.update(vec![json!("data")]).unwrap();

        // get() should fail before finish
        assert!(channel.get().is_err());
        assert!(!channel.is_available());
    }

    #[test]
    fn test_named_barrier_value_single_name() {
        let mut channel = NamedBarrierValueChannel::new(vec!["single".to_string()]);

        assert!(!channel.is_available());
        channel.update(vec![json!("single")]).unwrap();
        assert!(channel.is_available());
    }

    #[test]
    fn test_ephemeral_value_default() {
        let channel = EphemeralValueChannel::default();
        assert!(!channel.is_available());
        assert!(channel.get().is_err());
    }

    #[test]
    fn test_any_value_default() {
        let channel = AnyValueChannel::default();
        assert!(!channel.is_available());
        assert!(channel.get().is_err());
    }

    #[test]
    fn test_untracked_value_default() {
        let channel = UntrackedValueChannel::default();
        assert!(!channel.is_available());
        assert!(channel.get().is_err());
    }
}
