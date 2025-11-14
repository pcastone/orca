//! Core checkpoint data structures for state persistence and time-travel
//!
//! This module defines the fundamental data types for the checkpoint system: **[`Checkpoint`]**,
//! **[`CheckpointConfig`]**, **[`CheckpointMetadata`]**, and **[`CheckpointTuple`]**. These types
//! represent complete snapshots of graph execution state, enabling persistence, recovery, debugging,
//! and human-in-the-loop workflows.
//!
//! # Overview
//!
//! The checkpoint system provides:
//!
//! - **State Snapshots** - Complete point-in-time captures of all channel values
//! - **Version Tracking** - Channel version management for change detection
//! - **Metadata** - Step number, source, parent relationships, custom data
//! - **Thread Isolation** - Independent checkpoint histories per execution thread
//! - **Time-Travel** - Load and replay from any historical checkpoint
//! - **Serializable** - All types support JSON serialization via serde
//!
//! # Core Types
//!
//! - [`Checkpoint`] - Complete state snapshot with channel values and versions
//! - [`CheckpointConfig`] - Thread ID + checkpoint ID for identifying checkpoints
//! - [`CheckpointMetadata`] - Additional metadata (step, source, parents, custom)
//! - [`CheckpointTuple`] - Complete checkpoint with config, state, and metadata
//! - [`ChannelVersions`] - Map of channel names to version numbers
//! - [`CheckpointSource`] - Origin of checkpoint (Input, Loop, Update, Fork)
//! - [`PendingWrite`] - Uncommitted writes tracked in checkpoints
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Checkpoint Structure                                        │
//! │                                                               │
//! │  ┌──────────────────────────────────────────────┐           │
//! │  │  CheckpointTuple                             │           │
//! │  │  ┌────────────────────────────────────────┐ │           │
//! │  │  │  CheckpointConfig                      │ │           │
//! │  │  │  • thread_id: "user-123"               │ │           │
//! │  │  │  • checkpoint_id: "uuid-abc"           │ │           │
//! │  │  │  • checkpoint_ns: None                 │ │           │
//! │  │  └────────────────────────────────────────┘ │           │
//! │  │  ┌────────────────────────────────────────┐ │           │
//! │  │  │  Checkpoint                            │ │           │
//! │  │  │  • v: 1                                 │ │           │
//! │  │  │  • id: "uuid-abc"                       │ │           │
//! │  │  │  • ts: 2024-01-01T12:00:00Z             │ │           │
//! │  │  │  • channel_values: {                    │ │           │
//! │  │  │      "messages": [...],                 │ │           │
//! │  │  │      "context": {...}                   │ │           │
//! │  │  │    }                                    │ │           │
//! │  │  │  • channel_versions: {                  │ │           │
//! │  │  │      "messages": 5,                     │ │           │
//! │  │  │      "context": 2                       │ │           │
//! │  │  │    }                                    │ │           │
//! │  │  │  • versions_seen: {                     │ │           │
//! │  │  │      "node_a": {"messages": 4},         │ │           │
//! │  │  │      "node_b": {"context": 1}           │ │           │
//! │  │  │    }                                    │ │           │
//! │  │  └────────────────────────────────────────┘ │           │
//! │  │  ┌────────────────────────────────────────┐ │           │
//! │  │  │  CheckpointMetadata                    │ │           │
//! │  │  │  • source: Loop                         │ │           │
//! │  │  │  • step: 5                              │ │           │
//! │  │  │  • parents: {"": "parent-uuid"}         │ │           │
//! │  │  │  • extra: {custom data}                 │ │           │
//! │  │  └────────────────────────────────────────┘ │           │
//! │  └──────────────────────────────────────────────┘           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Creating a Checkpoint
//!
//! ```rust,ignore
//! use langgraph_checkpoint::{Checkpoint, ChannelVersions};
//! use std::collections::HashMap;
//! use serde_json::json;
//!
//! let mut channel_values = HashMap::new();
//! channel_values.insert("messages".to_string(), json!(["Hello", "World"]));
//! channel_values.insert("context".to_string(), json!({"user": "alice"}));
//!
//! let mut channel_versions = HashMap::new();
//! channel_versions.insert("messages".to_string(), ChannelVersion::Int(1));
//! channel_versions.insert("context".to_string(), ChannelVersion::Int(1));
//!
//! let versions_seen = HashMap::new(); // Initially empty
//!
//! let checkpoint = Checkpoint::new(
//!     "checkpoint-id-123".to_string(),
//!     channel_values,
//!     channel_versions,
//!     versions_seen,
//! );
//! ```
//!
//! ## Configuring Checkpoint Retrieval
//!
//! ```rust,ignore
//! use langgraph_checkpoint::CheckpointConfig;
//!
//! // Get latest checkpoint for a thread
//! let config = CheckpointConfig::new()
//!     .with_thread_id("user-session-123".to_string());
//!
//! // Get specific checkpoint
//! let config = CheckpointConfig::new()
//!     .with_thread_id("user-session-123".to_string())
//!     .with_checkpoint_id("checkpoint-uuid-abc".to_string());
//!
//! // Use checkpoint saver to retrieve
//! let tuple = checkpointer.get_tuple(&config).await?;
//! ```
//!
//! ## Adding Metadata
//!
//! ```rust,ignore
//! use langgraph_checkpoint::{CheckpointMetadata, CheckpointSource};
//! use serde_json::json;
//!
//! let metadata = CheckpointMetadata::new()
//!     .with_source(CheckpointSource::Loop)
//!     .with_step(5)
//!     .with_extra("user_action".to_string(), json!("approved"));
//! ```
//!
//! # Channel Versions
//!
//! Version tracking enables change detection and determines which nodes need to execute:
//!
//! ```rust,ignore
//! use langgraph_checkpoint::ChannelVersion;
//!
//! // Integer versions (most common)
//! let v1 = ChannelVersion::Int(1);
//! let v2 = v1.next(); // ChannelVersion::Int(2)
//!
//! // Float versions (for fractional increments)
//! let v1 = ChannelVersion::Float(1.0);
//! let v2 = v1.next(); // ChannelVersion::Float(2.0)
//!
//! // String versions (manual management required)
//! let v1 = ChannelVersion::String("v1.0.0".to_string());
//! // v1.next() would panic - must be managed explicitly
//! ```
//!
//! ## Versions Seen Tracking
//!
//! The `versions_seen` field tracks which channel versions each node has already processed:
//!
//! ```text
//! Example:
//!   Channel "messages" is at version 5
//!   Node "processor" has seen version 4
//!   → Node "processor" needs to run (new data available)
//!
//!   Node "formatter" has seen version 5
//!   → Node "formatter" is up-to-date (skip)
//! ```
//!
//! # Checkpoint Sources
//!
//! The `source` field in metadata indicates how the checkpoint was created:
//!
//! | Source | When Created | Use Case |
//! |--------|--------------|----------|
//! | `Input` | Initial invoke/stream call | Starting point for execution |
//! | `Loop` | After each superstep | Normal execution progress |
//! | `Update` | Manual state modification | User updates state externally |
//! | `Fork` | Copied from another checkpoint | Branching execution, A/B testing |
//!
//! ## Example: Checkpoint Source Lifecycle
//!
//! ```rust,ignore
//! // Step -1: Input checkpoint
//! let input_meta = CheckpointMetadata::new()
//!     .with_source(CheckpointSource::Input)
//!     .with_step(-1);
//!
//! // Step 0: First loop iteration
//! let loop0_meta = CheckpointMetadata::new()
//!     .with_source(CheckpointSource::Loop)
//!     .with_step(0);
//!
//! // Step 1, 2, 3...: Subsequent iterations
//! let loop1_meta = CheckpointMetadata::new()
//!     .with_source(CheckpointSource::Loop)
//!     .with_step(1);
//!
//! // Manual update during execution
//! let update_meta = CheckpointMetadata::new()
//!     .with_source(CheckpointSource::Update)
//!     .with_step(2); // Based on when update occurred
//!
//! // Fork for parallel execution
//! let fork_meta = CheckpointMetadata::new()
//!     .with_source(CheckpointSource::Fork)
//!     .with_step(2); // Forked from step 2
//! ```
//!
//! # Thread Isolation
//!
//! The `thread_id` in CheckpointConfig isolates checkpoint histories:
//!
//! ```rust,ignore
//! // User session 1
//! let config1 = CheckpointConfig::new()
//!     .with_thread_id("session-alice".to_string());
//!
//! // User session 2 (completely independent)
//! let config2 = CheckpointConfig::new()
//!     .with_thread_id("session-bob".to_string());
//!
//! // These create separate checkpoint histories
//! checkpointer.put(&config1, checkpoint1, metadata1, versions1).await?;
//! checkpointer.put(&config2, checkpoint2, metadata2, versions2).await?;
//! ```
//!
//! # Serialization
//!
//! All checkpoint types implement `Serialize` and `Deserialize`:
//!
//! ```rust,ignore
//! use langgraph_checkpoint::Checkpoint;
//!
//! // Serialize to JSON
//! let json = serde_json::to_string(&checkpoint)?;
//!
//! // Deserialize from JSON
//! let checkpoint: Checkpoint = serde_json::from_str(&json)?;
//!
//! // Serialize to MessagePack (smaller)
//! let bytes = rmp_serde::to_vec(&checkpoint)?;
//!
//! // Serialize to Bincode (faster)
//! let bytes = bincode::serialize(&checkpoint)?;
//! ```
//!
//! # Parent Relationships
//!
//! Checkpoints can track parent-child relationships via the `parents` field:
//!
//! ```rust,ignore
//! use std::collections::HashMap;
//!
//! let mut parents = HashMap::new();
//! parents.insert("".to_string(), "parent-checkpoint-uuid".to_string());
//! parents.insert("subgraph-1".to_string(), "subgraph-checkpoint-uuid".to_string());
//!
//! let metadata = CheckpointMetadata::new()
//!     .with_parents(parents);
//!
//! // Use case: Track checkpoint lineage for time-travel debugging
//! // "" (empty string) = immediate parent
//! // "subgraph-1" = parent from subgraph namespace
//! ```
//!
//! # Pending Writes
//!
//! The `PendingWrite` type tracks uncommitted channel updates:
//!
//! ```rust,ignore
//! use langgraph_checkpoint::PendingWrite;
//! use serde_json::json;
//!
//! // (task_id, channel_name, value)
//! let write: PendingWrite = (
//!     "task-123".to_string(),
//!     "messages".to_string(),
//!     json!({"role": "user", "content": "Hello"}),
//! );
//!
//! // Stored via put_writes() in CheckpointSaver
//! checkpointer.put_writes(&config, vec![write], "task-123".to_string()).await?;
//! ```
//!
//! **Use case:** In Send-based map-reduce, worker tasks write results that haven't been
//! committed to the main checkpoint yet. Pending writes track these interim results.
//!
//! # Best Practices
//!
//! 1. **Use thread_id consistently** - Same thread_id groups related checkpoints together
//! 2. **Increment step numbers** - Sequential steps enable time-travel navigation
//! 3. **Store channel_versions** - Required for determining which nodes need execution
//! 4. **Track versions_seen** - Enables efficient delta execution (only run when needed)
//! 5. **Add custom metadata** - Use `extra` field for audit trails, debugging context
//! 6. **Compress large states** - Channel values can be large; compress before storing
//! 7. **Index by thread_id + id** - These are the primary query keys
//!
//! # Comparison with Python LangGraph
//!
//! | Python LangGraph | rLangGraph | Notes |
//! |------------------|------------|-------|
//! | `Checkpoint` dict | `Checkpoint` struct | Rust has typed struct |
//! | `checkpoint_id` string | `id` field (String) | Same concept |
//! | `channel_values` dict | `HashMap<String, Value>` | JSON values in Rust |
//! | `channel_versions` dict | `ChannelVersions` type alias | Strongly typed |
//! | `metadata` dict | `CheckpointMetadata` struct | Structured metadata |
//! | `thread_ts` | `thread_id` (thread ID only) | Rust separates thread from timestamp |
//! | Python pickling | JSON (serde) | Cross-language compatible |
//!
//! # See Also
//!
//! - [`CheckpointSaver`](crate::traits::CheckpointSaver) - Trait for storage backends
//! - [`InMemoryCheckpointSaver`](crate::memory::InMemoryCheckpointSaver) - Reference implementation
//! - [`CompiledGraph::with_checkpointer()`](langgraph_core::compiled::CompiledGraph) - Attach checkpointer

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Checkpoint ID type
pub type CheckpointId = String;

/// Pending write tuple: (task_id, channel, value)
///
/// Represents a write operation that hasn't been committed to a channel yet.
/// Used to track writes that are pending completion.
pub type PendingWrite = (String, String, serde_json::Value);

/// Channel version type - can be int, float, or string
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ChannelVersion {
    Int(i64),
    Float(f64),
    String(String),
}

impl ChannelVersion {
    /// Get the next version (increments integers by 1)
    pub fn next(&self) -> Self {
        match self {
            ChannelVersion::Int(v) => ChannelVersion::Int(v + 1),
            ChannelVersion::Float(v) => ChannelVersion::Float(v + 1.0),
            ChannelVersion::String(_) => {
                panic!("String versions must be explicitly managed")
            }
        }
    }
}

/// Mapping from channel name to version
pub type ChannelVersions = HashMap<String, ChannelVersion>;

/// Metadata source type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CheckpointSource {
    /// Checkpoint created from an input to invoke/stream/batch
    Input,
    /// Checkpoint created from inside the pregel loop
    Loop,
    /// Checkpoint created from a manual state update
    Update,
    /// Checkpoint created as a copy of another checkpoint
    Fork,
}

/// Metadata associated with a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckpointMetadata {
    /// The source of the checkpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<CheckpointSource>,

    /// The step number of the checkpoint
    /// -1 for the first "input" checkpoint
    /// 0 for the first "loop" checkpoint
    /// n for the nth checkpoint afterwards
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<i32>,

    /// The IDs of the parent checkpoints
    /// Mapping from checkpoint namespace to checkpoint ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parents: Option<HashMap<String, String>>,

    /// Additional custom metadata
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl CheckpointMetadata {
    /// Create a new checkpoint metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source
    pub fn with_source(mut self, source: CheckpointSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Set the step number
    pub fn with_step(mut self, step: i32) -> Self {
        self.step = Some(step);
        self
    }

    /// Set parent checkpoints
    pub fn with_parents(mut self, parents: HashMap<String, String>) -> Self {
        self.parents = Some(parents);
        self
    }

    /// Add custom metadata
    pub fn with_extra(mut self, key: String, value: serde_json::Value) -> Self {
        self.extra.insert(key, value);
        self
    }
}

/// State snapshot at a given point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// The version of the checkpoint format (currently 1)
    pub v: i32,

    /// The ID of the checkpoint (unique and monotonically increasing)
    pub id: CheckpointId,

    /// The timestamp of the checkpoint
    pub ts: DateTime<Utc>,

    /// The values of the channels at the time of the checkpoint
    /// Mapping from channel name to serialized channel snapshot value
    pub channel_values: HashMap<String, serde_json::Value>,

    /// The versions of the channels at the time of the checkpoint
    pub channel_versions: ChannelVersions,

    /// Map from node ID to map from channel name to version seen
    /// This keeps track of the versions of the channels that each node has seen
    /// Used to determine which nodes to execute next
    pub versions_seen: HashMap<String, ChannelVersions>,

    /// The channels that were updated in this checkpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_channels: Option<Vec<String>>,
}

impl Checkpoint {
    /// Current checkpoint format version
    pub const CURRENT_VERSION: i32 = 1;

    /// Create a new checkpoint
    pub fn new(
        id: CheckpointId,
        channel_values: HashMap<String, serde_json::Value>,
        channel_versions: ChannelVersions,
        versions_seen: HashMap<String, ChannelVersions>,
    ) -> Self {
        Self {
            v: Self::CURRENT_VERSION,
            id,
            ts: Utc::now(),
            channel_values,
            channel_versions,
            versions_seen,
            updated_channels: None,
        }
    }

    /// Create an empty checkpoint
    pub fn empty() -> Self {
        Self {
            v: Self::CURRENT_VERSION,
            id: Uuid::new_v4().to_string(),
            ts: Utc::now(),
            channel_values: HashMap::new(),
            channel_versions: HashMap::new(),
            versions_seen: HashMap::new(),
            updated_channels: None,
        }
    }

    /// Copy this checkpoint
    pub fn copy(&self) -> Self {
        Self {
            v: self.v,
            id: self.id.clone(),
            ts: self.ts,
            channel_values: self.channel_values.clone(),
            channel_versions: self.channel_versions.clone(),
            versions_seen: self.versions_seen.clone(),
            updated_channels: self.updated_channels.clone(),
        }
    }

    /// Set the updated channels
    pub fn with_updated_channels(mut self, channels: Vec<String>) -> Self {
        self.updated_channels = Some(channels);
        self
    }
}

/// Configuration for checkpoint operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckpointConfig {
    /// Thread ID for grouping related checkpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,

    /// Specific checkpoint ID to retrieve
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<CheckpointId>,

    /// Checkpoint namespace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_ns: Option<String>,

    /// Additional configuration
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl CheckpointConfig {
    /// Create a new checkpoint configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the thread ID
    pub fn with_thread_id(mut self, thread_id: String) -> Self {
        self.thread_id = Some(thread_id);
        self
    }

    /// Set the checkpoint ID
    pub fn with_checkpoint_id(mut self, checkpoint_id: CheckpointId) -> Self {
        self.checkpoint_id = Some(checkpoint_id);
        self
    }

    /// Set the checkpoint namespace
    pub fn with_checkpoint_ns(mut self, checkpoint_ns: String) -> Self {
        self.checkpoint_ns = Some(checkpoint_ns);
        self
    }
}

/// A tuple containing a checkpoint and its associated data
#[derive(Debug, Clone)]
pub struct CheckpointTuple {
    /// Configuration for this checkpoint
    pub config: CheckpointConfig,

    /// The checkpoint itself
    pub checkpoint: Checkpoint,

    /// Metadata associated with the checkpoint
    pub metadata: CheckpointMetadata,

    /// Parent configuration (if any)
    pub parent_config: Option<CheckpointConfig>,
}

impl CheckpointTuple {
    /// Create a new checkpoint tuple
    pub fn new(
        config: CheckpointConfig,
        checkpoint: Checkpoint,
        metadata: CheckpointMetadata,
    ) -> Self {
        Self {
            config,
            checkpoint,
            metadata,
            parent_config: None,
        }
    }

    /// Set the parent configuration
    pub fn with_parent_config(mut self, parent_config: CheckpointConfig) -> Self {
        self.parent_config = Some(parent_config);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let checkpoint = Checkpoint::empty();
        assert_eq!(checkpoint.v, Checkpoint::CURRENT_VERSION);
        assert!(checkpoint.channel_values.is_empty());
        assert!(checkpoint.channel_versions.is_empty());
        assert!(checkpoint.versions_seen.is_empty());
    }

    #[test]
    fn test_channel_version_increment() {
        let v1 = ChannelVersion::Int(1);
        assert_eq!(v1.next(), ChannelVersion::Int(2));

        let v2 = ChannelVersion::Float(1.0);
        assert_eq!(v2.next(), ChannelVersion::Float(2.0));
    }

    #[test]
    fn test_checkpoint_metadata() {
        let metadata = CheckpointMetadata::new()
            .with_source(CheckpointSource::Input)
            .with_step(-1)
            .with_extra("key".to_string(), serde_json::json!("value"));

        assert_eq!(metadata.source, Some(CheckpointSource::Input));
        assert_eq!(metadata.step, Some(-1));
        assert_eq!(metadata.extra.get("key"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn test_checkpoint_config() {
        let config = CheckpointConfig::new()
            .with_thread_id("thread-1".to_string())
            .with_checkpoint_id("checkpoint-1".to_string());

        assert_eq!(config.thread_id, Some("thread-1".to_string()));
        assert_eq!(config.checkpoint_id, Some("checkpoint-1".to_string()));
    }
}
