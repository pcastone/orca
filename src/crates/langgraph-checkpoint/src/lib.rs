//! # langgraph-checkpoint - State Persistence for Graph Execution
//!
//! **Trait-based checkpoint abstractions and implementations** for persisting and restoring
//! graph execution state. This crate enables time-travel debugging, deterministic replay,
//! human-in-the-loop workflows, and distributed execution.
//!
//! ## Overview
//!
//! Checkpoints are **snapshots of graph execution state** captured after each Pregel superstep.
//! They enable:
//!
//! - **Time-Travel Debugging** - Inspect state at any execution point
//! - **Deterministic Replay** - Reproduce exact execution sequences
//! - **Human-in-the-Loop** - Pause, inspect, modify, and resume execution
//! - **Fault Recovery** - Resume from failures without restarting
//! - **Branching Timelines** - Create alternate execution paths from checkpoints
//! - **Audit Trails** - Track complete state evolution history
//!
//! ## Core Concepts
//!
//! ### 1. CheckpointSaver Trait
//!
//! The [`CheckpointSaver`] trait defines the interface for checkpoint persistence backends.
//! Implementors provide:
//!
//! - **`put()`** - Save checkpoint with config and metadata
//! - **`get_tuple()`** - Retrieve checkpoint by config
//! - **`list()`** - Query checkpoint history
//! - **`put_writes()`** - Store intermediate writes (optional)
//!
//! ### 2. Channel System
//!
//! Channels are **typed state containers** that manage individual state values:
//!
//! - [`LastValueChannel`] - Single value (last write wins)
//! - [`TopicChannel`] - Append-only list of values
//! - [`BinaryOperatorChannel`] - Accumulator with custom reducer (sum, product, etc.)
//! - [`EphemeralValueChannel`] - Clears after consumption
//! - [`AnyValueChannel`] - Permissive single value (no write conflicts)
//! - [`UntrackedValueChannel`] - Not persisted in checkpoints
//! - [`LastValueAfterFinishChannel`] - Available only after finish signal
//! - [`NamedBarrierValueChannel`] - Waits for multiple named signals
//!
//! ### 3. Checkpoint Structure
//!
//! A [`Checkpoint`] contains:
//! - **Version info** - Checkpoint ID and version numbers
//! - **Channel values** - Current state of all channels
//! - **Pending writes** - Buffered writes not yet applied
//! - **Channel versions** - Per-channel version tracking
//!
//! ### 4. Implementation Strategy
//!
//! This crate provides [`InMemoryCheckpointSaver`] as a reference implementation.
//! For production use, implement [`CheckpointSaver`] with your preferred backend:
//!
//! - **PostgreSQL** - Production persistence with ACID guarantees
//! - **SQLite** - Local development and single-node deployments
//! - **Redis** - High-speed distributed caching
//! - **S3/Object Storage** - Long-term archival
//! - **Custom** - Domain-specific storage solutions
//!
//! ## Quick Start
//!
//! ### Basic Checkpoint Usage
//!
//! ```rust,no_run
//! use langgraph_checkpoint::{
//!     InMemoryCheckpointSaver, CheckpointSaver, CheckpointConfig,
//!     Checkpoint, CheckpointMetadata
//! };
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let saver = InMemoryCheckpointSaver::new();
//!
//!     // Save a checkpoint
//!     let config = CheckpointConfig::new("thread-123");
//!     let checkpoint = Checkpoint::new(
//!         HashMap::new(), // channel_values
//!         HashMap::new(), // channel_versions
//!     );
//!     let metadata = CheckpointMetadata::default();
//!
//!     let saved_config = saver.put(&config, checkpoint, metadata, HashMap::new()).await?;
//!     println!("Checkpoint saved with ID: {:?}", saved_config.checkpoint_id);
//!
//!     // Retrieve checkpoint
//!     if let Some(tuple) = saver.get_tuple(&saved_config).await? {
//!         println!("Retrieved checkpoint: {:?}", tuple.checkpoint.id);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Channel Types
//!
//! ```rust,ignore
//! use langgraph_checkpoint::{
//!     Channel, LastValueChannel, TopicChannel, BinaryOperatorChannel,
//!     EphemeralValueChannel, NamedBarrierValueChannel
//! };
//! use serde_json::json;
//!
//! // Single value channel (overwrite semantics)
//! let mut last_value = LastValueChannel::new();
//! last_value.update(vec![json!({"status": "active"})]).unwrap();
//!
//! // Append-only channel (message history)
//! let mut topic = TopicChannel::new();
//! topic.update(vec![json!("msg1"), json!("msg2")]).unwrap();
//!
//! // Accumulator channel (sum)
//! let mut counter = BinaryOperatorChannel::new(|a: i32, b: i32| a + b, 0);
//! counter.update(vec![json!(5), json!(10)]).unwrap();
//! assert_eq!(counter.get().unwrap(), json!(15));
//!
//! // Ephemeral channel (one-time signals)
//! let mut signal = EphemeralValueChannel::new();
//! signal.update(vec![json!({"interrupt": true})]).unwrap();
//! signal.consume(); // Clears after use
//!
//! // Named barrier (parallel coordination)
//! let mut barrier = NamedBarrierValueChannel::new(vec!["task1".into(), "task2".into()]);
//! barrier.update(vec![json!("task1")]).unwrap();
//! barrier.update(vec![json!("task2")]).unwrap();
//! assert!(barrier.is_available()); // All tasks complete
//! ```
//!
//! ### Implementing Custom Backend
//!
//! ```rust,ignore
//! use langgraph_checkpoint::{
//!     CheckpointSaver, CheckpointConfig, Checkpoint, CheckpointMetadata,
//!     CheckpointTuple, CheckpointStream, Result, ChannelVersions
//! };
//! use async_trait::async_trait;
//!
//! struct PostgresCheckpointSaver {
//!     pool: sqlx::PgPool,
//! }
//!
//! #[async_trait]
//! impl CheckpointSaver for PostgresCheckpointSaver {
//!     async fn put(
//!         &self,
//!         config: &CheckpointConfig,
//!         checkpoint: Checkpoint,
//!         metadata: CheckpointMetadata,
//!         new_versions: ChannelVersions,
//!     ) -> Result<CheckpointConfig> {
//!         // Serialize checkpoint to JSON
//!         let data = serde_json::to_value(&checkpoint)?;
//!
//!         // Insert into PostgreSQL with UPSERT semantics
//!         sqlx::query!(
//!             "INSERT INTO checkpoints (thread_id, checkpoint_id, data, metadata)
//!              VALUES ($1, $2, $3, $4)
//!              ON CONFLICT (thread_id, checkpoint_id) DO UPDATE SET data = $3",
//!             config.thread_id,
//!             checkpoint.id,
//!             data,
//!             serde_json::to_value(&metadata)?,
//!         )
//!         .execute(&self.pool)
//!         .await?;
//!
//!         Ok(config.clone())
//!     }
//!
//!     async fn get_tuple(&self, config: &CheckpointConfig) -> Result<Option<CheckpointTuple>> {
//!         // Query PostgreSQL for checkpoint
//!         let row = sqlx::query!(
//!             "SELECT data, metadata FROM checkpoints
//!              WHERE thread_id = $1 AND checkpoint_id = $2",
//!             config.thread_id,
//!             config.checkpoint_id,
//!         )
//!         .fetch_optional(&self.pool)
//!         .await?;
//!
//!         Ok(row.map(|r| CheckpointTuple {
//!             config: config.clone(),
//!             checkpoint: serde_json::from_value(r.data).unwrap(),
//!             metadata: serde_json::from_value(r.metadata).unwrap(),
//!             parent_config: None,
//!             pending_writes: None,
//!         }))
//!     }
//!
//!     // Implement list() and put_writes() ...
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │              langgraph-core (Graph Execution)           │
//! │  • Pregel algorithm                                      │
//! │  • Node scheduling                                       │
//! │  • State management                                      │
//! └────────────────────┬────────────────────────────────────┘
//!                      │ Calls put() after each superstep
//!                      ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │           CheckpointSaver Trait (This Crate)            │
//! │  • put() - Save checkpoint                              │
//! │  • get_tuple() - Load checkpoint                        │
//! │  • list() - Query history                               │
//! └────────────────────┬────────────────────────────────────┘
//!                      │ Implemented by
//!         ┌────────────┴────────────┬──────────────┬─────────┐
//!         ▼                         ▼              ▼         ▼
//!  ┌──────────────┐    ┌─────────────────┐  ┌─────────┐  ┌────────┐
//!  │  In-Memory   │    │  PostgreSQL     │  │  SQLite │  │ Custom │
//!  │ (Reference)  │    │ (Production)    │  │  (Dev)  │  │        │
//!  └──────────────┘    └─────────────────┘  └─────────┘  └────────┘
//! ```
//!
//! ## Channel Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │           Graph State (Checkpoint)               │
//! │  ┌────────────┐  ┌────────────┐  ┌────────────┐ │
//! │  │  Channel   │  │  Channel   │  │  Channel   │ │
//! │  │  "status"  │  │ "messages" │  │  "count"   │ │
//! │  │            │  │            │  │            │ │
//! │  │ LastValue  │  │   Topic    │  │  BinaryOp  │ │
//! │  │  Channel   │  │  Channel   │  │  Channel   │ │
//! │  └────────────┘  └────────────┘  └────────────┘ │
//! └──────────────────────────────────────────────────┘
//!                  │
//!                  │ Serialized by CheckpointSaver
//!                  ▼
//!      ┌──────────────────────────┐
//!      │   Persistent Storage     │
//!      │  (DB, File, Memory)      │
//!      └──────────────────────────┘
//! ```
//!
//! ## Module Organization
//!
//! ### Core Types
//! - [`checkpoint`] - [`Checkpoint`], [`CheckpointConfig`], [`CheckpointMetadata`]
//! - [`traits`] - [`CheckpointSaver`] trait and [`CheckpointStream`]
//! - [`memory`] - [`InMemoryCheckpointSaver`] reference implementation
//! - [`error`] - [`CheckpointError`] types
//!
//! ### Channel Types
//! - [`channels`] - Core channels ([`LastValueChannel`], [`TopicChannel`], [`BinaryOperatorChannel`])
//! - [`channels_extended`] - Extended channels ([`EphemeralValueChannel`], [`AnyValueChannel`], [`UntrackedValueChannel`], [`LastValueAfterFinishChannel`], [`NamedBarrierValueChannel`])
//!
//! ### Utilities
//! - [`serializer`] - Serialization protocols for checkpoints
//! - [`Channel`] - Base trait for all channel types
//!
//! ## Use Cases
//!
//! ### 1. Development & Debugging
//!
//! Use [`InMemoryCheckpointSaver`] for:
//! - Local development
//! - Unit tests
//! - Debugging with state inspection
//! - Prototype workflows
//!
//! ### 2. Production Persistence
//!
//! Implement [`CheckpointSaver`] with:
//! - **PostgreSQL** - Multi-node deployments with strong consistency
//! - **SQLite** - Single-node apps, edge deployments
//! - **Redis** - High-throughput with optional persistence
//!
//! ### 3. Audit & Compliance
//!
//! Use checkpoints for:
//! - Complete execution audit trails
//! - Regulatory compliance (track all decisions)
//! - Post-incident analysis
//! - A/B testing with branched timelines
//!
//! ### 4. Human-in-the-Loop
//!
//! Enable interactive workflows:
//! - Pause execution for approval
//! - Inspect and modify state
//! - Resume from last checkpoint
//! - Branch to alternate paths
//!
//! ## Performance Considerations
//!
//! ### Checkpoint Frequency
//!
//! - **Every superstep** (default) - Maximum safety, moderate overhead
//! - **Configurable intervals** - Reduce overhead for long-running graphs
//! - **Conditional** - Only checkpoint on significant state changes
//!
//! ### Serialization
//!
//! - Checkpoints are JSON-serialized by default
//! - Consider MessagePack or binary formats for large states
//! - Implement compression for archival
//!
//! ### Channel Selection
//!
//! - Use [`UntrackedValueChannel`] for data too large to checkpoint
//! - Use [`EphemeralValueChannel`] for transient signals
//! - Batch writes with `put_writes()` for efficiency
//!
//! ## See Also
//!
//! - [`langgraph_core`] - Graph execution engine that uses this crate
//! - [`langgraph_prebuilt`] - High-level agent patterns with checkpoint support
//! - Python LangGraph Checkpoint Docs: <https://langchain-ai.github.io/langgraph/checkpointing/>
//! - Pregel Paper: <https://research.google/pubs/pub37252/>

pub mod checkpoint;
pub mod channels;
pub mod channels_ext;
pub mod channels_extended;
pub mod error;
pub mod memory;
pub mod serializer;
pub mod traits;

// Re-export main types
pub use checkpoint::{Checkpoint, CheckpointConfig, CheckpointId, CheckpointMetadata, CheckpointTuple, PendingWrite};
pub use channels::{BinaryOperatorChannel, Channel, LastValueChannel, TopicChannel};
pub use channels_ext::{
    AnyValueChannel, EphemeralValueChannel, NamedBarrierValueChannel, UntrackedValueChannel,
};
pub use channels_extended::{
    EphemeralValueChannel as EphemeralValue,
    AnyValueChannel as AnyValue,
    UntrackedValueChannel as UntrackedValue,
    LastValueAfterFinishChannel,
    NamedBarrierValueChannel as NamedBarrierValue,
};
pub use error::{CheckpointError, Result};
pub use memory::InMemoryCheckpointSaver;
pub use serializer::SerializerProtocol;
pub use traits::{CheckpointSaver, CheckpointStream};
