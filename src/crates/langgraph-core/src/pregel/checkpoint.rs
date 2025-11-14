//! Checkpoint data structures for Pregel execution.
//!
//! This module defines the core data structures used for capturing and managing graph
//! execution state in the Pregel-inspired execution model. Checkpoints enable time-travel
//! debugging, deterministic replay, fault recovery, and human-in-the-loop workflows.
//!
//! # Overview
//!
//! A **checkpoint** is a complete snapshot of graph state at a specific point in time (typically
//! after each Pregel superstep). It contains:
//!
//! - **Channel values** - Current state of all channels (LastValue, Topic, BinaryOp, etc.)
//! - **Channel versions** - Version numbers for each channel (used for change detection)
//! - **Versions seen** - Which channel versions each node has processed (triggers execution)
//! - **Metadata** - Execution context (step number, source, custom data)
//!
//! # Core Types
//!
//! - [`Checkpoint`] - Complete state snapshot with all channel data
//! - [`ChannelVersion`] - Version identifier (int, float, or timestamp string)
//! - [`ChannelVersions`] - Map of channel names to their versions
//! - [`PendingWrite`] - Buffered write waiting to be applied
//! - [`CheckpointMetadata`] - Execution context and custom metadata
//!
//! # Versioning Strategy
//!
//! Channel versions track state changes and trigger node execution:
//!
//! ```text
//! Superstep 0:
//!   Channel "messages": version 0 → write → version 1
//!
//! Superstep 1:
//!   Node A sees "messages" v1 (> last seen v0) → triggers execution
//!   Node B sees "messages" v1 (= last seen v1) → skips execution
//! ```
//!
//! # Example: Checkpoint Lifecycle
//!
//! ```rust,ignore
//! use langgraph_core::pregel::checkpoint::{Checkpoint, ChannelVersion, PendingWrite};
//!
//! // Create initial checkpoint
//! let mut checkpoint = Checkpoint::new();
//!
//! // Add channel values
//! checkpoint.channel_values.insert(
//!     "messages".to_string(),
//!     json!(["Hello", "World"])
//! );
//! checkpoint.channel_versions.insert(
//!     "messages".to_string(),
//!     ChannelVersion::Int(1)
//! );
//!
//! // Track which versions nodes have seen
//! checkpoint.versions_seen.insert(
//!     "agent_node".to_string(),
//!     [(\"messages\".to_string(), ChannelVersion::Int(0))].into_iter().collect()
//! );
//!
//! // Serialize for persistence
//! let json = serde_json::to_string(&checkpoint)?;
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::cmp::Ordering;

/// Flexible version identifier for channel state tracking.
///
/// `ChannelVersion` supports multiple version formats to accommodate different
/// use cases and backends:
///
/// - **Int** - Sequential integer versions (most common)
/// - **Float** - Floating-point versions for fractional increments
/// - **String** - Timestamp or UUID-based versions
///
/// # Version Comparison
///
/// Versions are totally ordered within the same variant:
/// - `Int(5) < Int(10)`
/// - `Float(1.5) < Float(2.0)`
/// - `String("2024-01-01") < String("2024-01-02")`
///
/// Cross-variant comparisons return `None` for `partial_cmp()`.
///
/// # Use Cases
///
/// 1. **Sequential (Int)** - Most common, simple increment
/// 2. **Fractional (Float)** - Sub-versioning within a superstep
/// 3. **Timestamp (String)** - Wall-clock ordering for distributed systems
/// 4. **UUID (String)** - Globally unique identifiers
///
/// # Example: Version Tracking
///
/// ```rust
/// use langgraph_core::pregel::checkpoint::{ChannelVersion, increment};
///
/// // Sequential integer versions
/// let v1 = ChannelVersion::Int(5);
/// let v2 = increment(Some(&v1));
/// assert_eq!(v2, ChannelVersion::Int(6));
///
/// // Comparison
/// assert!(v1 < v2);
/// ```
///
/// # Serialization
///
/// Uses `#[serde(untagged)]` to serialize naturally:
/// - `Int(5)` → `5`
/// - `Float(1.5)` → `1.5`
/// - `String("abc")` → `"abc"`
///
/// # See Also
///
/// - [`ChannelVersions`] - Map of channel names to versions
/// - [`increment`] - Version increment function
/// - [`Checkpoint::channel_versions`] - Checkpoint version tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChannelVersion {
    /// Sequential integer version (most common)
    Int(i64),
    /// Floating-point version for fractional increments
    Float(f64),
    /// String-based version (timestamp, UUID, etc.)
    String(String),
}

impl PartialEq for ChannelVersion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ChannelVersion::Int(a), ChannelVersion::Int(b)) => a == b,
            (ChannelVersion::Float(a), ChannelVersion::Float(b)) => {
                (a - b).abs() < f64::EPSILON
            }
            (ChannelVersion::String(a), ChannelVersion::String(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for ChannelVersion {}

impl PartialOrd for ChannelVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ChannelVersion::Int(a), ChannelVersion::Int(b)) => a.partial_cmp(b),
            (ChannelVersion::Float(a), ChannelVersion::Float(b)) => a.partial_cmp(b),
            (ChannelVersion::String(a), ChannelVersion::String(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl Ord for ChannelVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl Default for ChannelVersion {
    fn default() -> Self {
        ChannelVersion::Int(0)
    }
}

/// Mapping of channel names to their current versions.
///
/// This type alias represents the version state of all channels at a specific point
/// in execution. It's used throughout the checkpoint system for:
///
/// - Tracking current channel versions in [`Checkpoint::channel_versions`]
/// - Recording which versions each node has seen in [`Checkpoint::versions_seen`]
/// - Detecting channel changes to trigger node execution
///
/// # Example
///
/// ```rust
/// use langgraph_core::pregel::checkpoint::{ChannelVersions, ChannelVersion};
/// use std::collections::HashMap;
///
/// let mut versions: ChannelVersions = HashMap::new();
/// versions.insert("messages".to_string(), ChannelVersion::Int(5));
/// versions.insert("status".to_string(), ChannelVersion::Int(2));
/// ```
pub type ChannelVersions = HashMap<String, ChannelVersion>;

/// Complete snapshot of graph execution state at a specific point in time.
///
/// A `Checkpoint` captures **everything needed to resume execution** from a specific
/// superstep, including channel values, version numbers, and metadata. This enables:
///
/// - **Time-Travel Debugging** - Inspect state at any execution point
/// - **Deterministic Replay** - Re-execute from any checkpoint with identical results
/// - **Fault Recovery** - Resume after crashes or interrupts
/// - **Human-in-the-Loop** - Pause, inspect, modify, and continue execution
/// - **Branch Timelines** - Create alternate execution paths from a checkpoint
///
/// # Structure
///
/// ```text
/// Checkpoint {
///   id: "uuid-1234",           // Unique identifier
///   ts: 2024-01-15T10:30:00Z,  // Creation timestamp
///   v: 1,                      // Format version
///
///   channel_values: {          // Current state
///     "messages": [...],
///     "status": "active"
///   },
///
///   channel_versions: {        // Version tracking
///     "messages": 5,
///     "status": 2
///   },
///
///   versions_seen: {           // Node tracking
///     "agent": {
///       "messages": 4,         // Last version node saw
///       "status": 2
///     }
///   }
/// }
/// ```
///
/// # Triggering Logic
///
/// Nodes execute when they see channel versions **newer than previously seen**:
///
/// ```text
/// Current version: channel_versions["messages"] = 5
/// Node last saw:   versions_seen["agent"]["messages"] = 4
///
/// → 5 > 4 → Node "agent" will execute this superstep
/// ```
///
/// # Example: Creating and Using Checkpoints
///
/// ```rust
/// use langgraph_core::pregel::checkpoint::{Checkpoint, ChannelVersion};
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// // Create new checkpoint
/// let mut checkpoint = Checkpoint::new();
///
/// // Add channel state
/// checkpoint.channel_values.insert(
///     "messages".to_string(),
///     json!(["Hello", "World"])
/// );
/// checkpoint.channel_versions.insert(
///     "messages".to_string(),
///     ChannelVersion::Int(1)
/// );
///
/// // Serialize for storage
/// let serialized = serde_json::to_string(&checkpoint).unwrap();
///
/// // Deserialize later
/// let restored: Checkpoint = serde_json::from_str(&serialized).unwrap();
/// assert_eq!(checkpoint.id, restored.id);
/// ```
///
/// # Checkpoint Lifecycle
///
/// ```text
/// ┌─────────────────────────────────────────────────┐
/// │ Superstep N                                      │
/// │  1. Load checkpoint N-1                          │
/// │  2. Execute nodes                                │
/// │  3. Collect writes to channels                   │
/// │  4. Apply writes → new channel values            │
/// │  5. Increment channel versions                   │
/// │  6. Create checkpoint N with new state           │
/// │  7. Persist checkpoint N                         │
/// └─────────────────────────────────────────────────┘
/// ```
///
/// # See Also
///
/// - [`ChannelVersion`] - Version tracking for channels
/// - [`PendingWrite`] - Writes waiting to be applied
/// - [`CheckpointMetadata`] - Execution context metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Version number of the checkpoint format (currently 1)
    pub v: i32,
    /// Unique checkpoint identifier (UUID)
    pub id: String,
    /// Creation timestamp (UTC)
    #[serde(with = "chrono::serde::ts_seconds")]
    pub ts: DateTime<Utc>,
    /// Current values of all channels (channel_name → JSON value)
    pub channel_values: HashMap<String, serde_json::Value>,
    /// Current version of each channel (channel_name → version)
    pub channel_versions: ChannelVersions,
    /// Versions each node has processed (node_name → channel_name → version)
    ///
    /// Used to determine if a node should execute: node executes if any channel
    /// version is newer than the version in `versions_seen` for that node.
    pub versions_seen: HashMap<String, ChannelVersions>,
    /// Channels that were updated in this superstep (optional optimization)
    pub updated_channels: Option<Vec<String>>,
}

impl Checkpoint {
    /// Create a new empty checkpoint.
    pub fn new() -> Self {
        Self {
            v: 1,
            id: uuid::Uuid::new_v4().to_string(),
            ts: Utc::now(),
            channel_values: HashMap::new(),
            channel_versions: HashMap::new(),
            versions_seen: HashMap::new(),
            updated_channels: None,
        }
    }

    /// Create a checkpoint with specific ID (for testing).
    pub fn with_id(id: String) -> Self {
        let mut cp = Self::new();
        cp.id = id;
        cp
    }

    /// Get the null version for this checkpoint (used for comparisons).
    pub fn null_version(&self) -> ChannelVersion {
        if let Some(version) = self.channel_versions.values().next() {
            match version {
                ChannelVersion::Int(_) => ChannelVersion::Int(0),
                ChannelVersion::Float(_) => ChannelVersion::Float(0.0),
                ChannelVersion::String(_) => ChannelVersion::String(String::new()),
            }
        } else {
            ChannelVersion::Int(0)
        }
    }
}

impl Default for Checkpoint {
    fn default() -> Self {
        Self::new()
    }
}

/// Increment a channel version by one unit.
///
/// This function handles version incrementing for all [`ChannelVersion`] variants:
///
/// - **Int**: Increments by 1 (`5 → 6`)
/// - **Float**: Increments by 1.0 (`1.5 → 2.5`)
/// - **String**: Attempts to parse as int and increment, otherwise appends ".1"
/// - **None**: Returns `Int(1)` for initial version
///
/// # Example
///
/// ```rust
/// use langgraph_core::pregel::checkpoint::{ChannelVersion, increment};
///
/// // Int versions
/// let v = ChannelVersion::Int(5);
/// assert_eq!(increment(Some(&v)), ChannelVersion::Int(6));
///
/// // Float versions
/// let v = ChannelVersion::Float(1.5);
/// assert_eq!(increment(Some(&v)), ChannelVersion::Float(2.5));
///
/// // String versions (numeric)
/// let v = ChannelVersion::String("10".to_string());
/// assert_eq!(increment(Some(&v)), ChannelVersion::String("11".to_string()));
///
/// // String versions (non-numeric)
/// let v = ChannelVersion::String("abc".to_string());
/// assert_eq!(increment(Some(&v)), ChannelVersion::String("abc.1".to_string()));
///
/// // No previous version
/// assert_eq!(increment(None), ChannelVersion::Int(1));
/// ```
///
/// # Use in Pregel Loop
///
/// Called after each superstep when a channel is updated:
///
/// ```text
/// Superstep N:
///   channel_versions["messages"] = Int(5)
///
/// → Node writes to "messages" channel
///
/// Superstep N+1:
///   channel_versions["messages"] = increment(Int(5)) = Int(6)
/// ```
pub fn increment(current: Option<&ChannelVersion>) -> ChannelVersion {
    match current {
        Some(ChannelVersion::Int(v)) => ChannelVersion::Int(v + 1),
        Some(ChannelVersion::Float(v)) => ChannelVersion::Float(v + 1.0),
        Some(ChannelVersion::String(v)) => {
            // For string versions, try to parse as int and increment
            if let Ok(num) = v.parse::<i64>() {
                ChannelVersion::String((num + 1).to_string())
            } else {
                // Otherwise append ".1"
                ChannelVersion::String(format!("{}.1", v))
            }
        }
        None => ChannelVersion::Int(1),
    }
}

/// Buffered write waiting to be applied to a checkpoint.
///
/// `PendingWrite` represents a **write operation** from a node execution that hasn't yet
/// been committed to the checkpoint. Pending writes enable:
///
/// - **Write Buffering** - Collect writes during node execution
/// - **Transactional Semantics** - Apply all writes atomically after barrier
/// - **Audit Trails** - Track which task produced each write
/// - **Conflict Detection** - Identify multiple writes to same channel
///
/// # Lifecycle
///
/// ```text
/// 1. Node executes → produces writes
/// 2. Writes buffered as PendingWrite
/// 3. All nodes complete (barrier)
/// 4. PendingWrites applied to channels
/// 5. Channel versions incremented
/// 6. New checkpoint created
/// ```
///
/// # Example: Write Buffering
///
/// ```rust
/// use langgraph_core::pregel::checkpoint::PendingWrite;
/// use serde_json::json;
///
/// // Node execution produces a write
/// let write = PendingWrite::new(
///     "agent_node:step5".to_string(),
///     "messages".to_string(),
///     json!({"role": "assistant", "content": "Hello!"})
/// );
///
/// // Writes are buffered until barrier
/// let mut pending = vec![write];
///
/// // After barrier, apply to checkpoint
/// // checkpoint.apply_writes(pending);
/// ```
///
/// # Multiple Writes to Same Channel
///
/// When multiple nodes write to the same channel, the channel's reducer
/// determines how writes are combined:
///
/// - **LastValue**: Error (unless guard disabled)
/// - **Topic**: Append all values
/// - **BinaryOp**: Apply reducer function (sum, product, etc.)
///
/// ```text
/// Task A → write("messages", "Hello") ──┐
/// Task B → write("messages", "World") ──┤→ Channel reducer → ["Hello", "World"]
/// ```
///
/// # See Also
///
/// - [`Checkpoint`] - State snapshot that receives applied writes
/// - [`ChannelVersion`] - Incremented after writes applied
/// - CheckpointSaver::put_writes() - Persists pending writes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingWrite {
    /// Task ID that created this write (for tracing and conflict detection)
    pub task_id: String,
    /// Channel name to write to
    pub channel: String,
    /// Value to write (will be passed to channel's reducer)
    pub value: serde_json::Value,
}

impl PendingWrite {
    pub fn new(task_id: String, channel: String, value: serde_json::Value) -> Self {
        Self {
            task_id,
            channel,
            value,
        }
    }
}

/// Execution context and custom metadata attached to a checkpoint.
///
/// `CheckpointMetadata` provides **contextual information** about how and why a checkpoint
/// was created. This metadata enables:
///
/// - **Execution Tracking** - Know which step/source created the checkpoint
/// - **Custom Annotations** - Add domain-specific metadata for filtering/analysis
/// - **Audit Trails** - Track execution provenance
/// - **Debugging** - Understand checkpoint creation context
///
/// # Standard Fields
///
/// - **source** - How checkpoint was created:
///   - `"input"` - Initial state from user input
///   - `"loop"` - Created during Pregel superstep
///   - `"update"` - Created from state update operation
///   - `"interrupt"` - Created at interrupt point
///
/// - **step** - Superstep number (e.g., step 0, step 1, ...)
///
/// # Custom Metadata
///
/// The `extra` field accepts arbitrary JSON data via `#[serde(flatten)]`:
///
/// ```rust
/// use langgraph_core::pregel::checkpoint::CheckpointMetadata;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let mut metadata = CheckpointMetadata::default();
/// metadata.source = Some("loop".to_string());
/// metadata.step = Some(5);
///
/// // Add custom metadata
/// metadata.extra.insert("user_id".to_string(), json!("user-123"));
/// metadata.extra.insert("session_id".to_string(), json!("session-abc"));
/// metadata.extra.insert("cost_usd".to_string(), json!(0.042));
///
/// // Serialize includes extra fields at top level
/// let json = serde_json::to_string(&metadata).unwrap();
/// // {"source":"loop","step":5,"user_id":"user-123","session_id":"session-abc",...}
/// ```
///
/// # Use Cases
///
/// ## 1. Cost Tracking
///
/// ```rust,ignore
/// metadata.extra.insert("tokens_used".to_string(), json!(1500));
/// metadata.extra.insert("cost_usd".to_string(), json!(0.03));
/// ```
///
/// ## 2. User Tracking
///
/// ```rust,ignore
/// metadata.extra.insert("user_id".to_string(), json!("user-456"));
/// metadata.extra.insert("org_id".to_string(), json!("org-789"));
/// ```
///
/// ## 3. Performance Metrics
///
/// ```rust,ignore
/// metadata.extra.insert("duration_ms".to_string(), json!(250));
/// metadata.extra.insert("nodes_executed".to_string(), json!(3));
/// ```
///
/// ## 4. Filtering Checkpoints
///
/// Use metadata to filter checkpoints via CheckpointSaver::list():
///
/// ```rust,ignore
/// let filter = HashMap::from([
///     ("user_id".to_string(), json!("user-123")),
///     ("step".to_string(), json!(5)),
/// ]);
/// let checkpoints = saver.list(Some(&config), Some(filter), None, None).await?;
/// ```
///
/// # See Also
///
/// - [`Checkpoint`] - State snapshot that includes this metadata
/// - CheckpointSaver::list() - Query checkpoints by metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckpointMetadata {
    /// Source of the checkpoint (e.g., "input", "loop", "update", "interrupt")
    pub source: Option<String>,
    /// Superstep number (0-indexed)
    pub step: Option<i32>,
    /// Custom metadata (flattened during serialization)
    ///
    /// Use this field to add domain-specific metadata for tracking, filtering,
    /// and analysis. All fields in `extra` are serialized at the top level.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_version_comparison() {
        let v1 = ChannelVersion::Int(5);
        let v2 = ChannelVersion::Int(10);
        assert!(v1 < v2);
        assert!(v2 > v1);
        assert_eq!(v1, ChannelVersion::Int(5));
    }

    #[test]
    fn test_channel_version_float() {
        let v1 = ChannelVersion::Float(1.5);
        let v2 = ChannelVersion::Float(2.5);
        assert!(v1 < v2);
    }

    #[test]
    fn test_increment_int() {
        let v = ChannelVersion::Int(5);
        let next = increment(Some(&v));
        assert_eq!(next, ChannelVersion::Int(6));
    }

    #[test]
    fn test_increment_none() {
        let next = increment(None);
        assert_eq!(next, ChannelVersion::Int(1));
    }

    #[test]
    fn test_checkpoint_creation() {
        let cp = Checkpoint::new();
        assert_eq!(cp.v, 1);
        assert!(cp.id.len() > 0);
        assert_eq!(cp.channel_values.len(), 0);
    }

    #[test]
    fn test_checkpoint_null_version() {
        let mut cp = Checkpoint::new();
        cp.channel_versions
            .insert("test".into(), ChannelVersion::Int(5));

        let null = cp.null_version();
        assert_eq!(null, ChannelVersion::Int(0));
    }

    #[test]
    fn test_pending_write() {
        let write = PendingWrite::new(
            "task1".into(),
            "channel1".into(),
            serde_json::json!({"value": 42}),
        );
        assert_eq!(write.task_id, "task1");
        assert_eq!(write.channel, "channel1");
    }
}
