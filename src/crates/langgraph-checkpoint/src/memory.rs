//! In-memory checkpoint storage for development and testing
//!
//! This module provides **[`InMemoryCheckpointSaver`]** - a reference implementation of the
//! [`CheckpointSaver`] trait that stores all checkpoints in memory using a thread-safe HashMap.
//! This implementation is ideal for development, testing, and small-scale applications where
//! persistence across restarts is not required.
//!
//! # Overview
//!
//! The in-memory checkpoint saver:
//!
//! - **No External Dependencies** - Pure Rust, no database required
//! - **Thread-Safe** - Uses `Arc<RwLock<HashMap>>` for concurrent access
//! - **Full Feature Support** - Implements all `CheckpointSaver` methods
//! - **Fast** - All operations are in-memory (microsecond latency)
//! - **Ephemeral** - Data lost on application restart
//! - **Testing-Friendly** - Includes `clear()` method for test isolation
//! - **Development-Ready** - Works out of the box with no configuration
//!
//! # Core Type
//!
//! - [`InMemoryCheckpointSaver`] - Main struct implementing `CheckpointSaver` trait
//!
//! # When to Use
//!
//! **Use In-Memory Checkpoints For:**
//! - ✅ Development and prototyping
//! - ✅ Unit and integration tests
//! - ✅ Short-lived workflows (minutes, not hours)
//! - ✅ Single-process applications
//! - ✅ Demos and examples
//!
//! **Avoid In-Memory For:**
//! - ❌ Production deployments requiring persistence
//! - ❌ Long-running workflows (>1 hour)
//! - ❌ Multi-process or distributed systems
//! - ❌ Audit trails or compliance requirements
//! - ❌ Recovery from crashes
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  InMemoryCheckpointSaver                                     │
//! │                                                               │
//! │  ┌────────────────────────────────────────────────┐         │
//! │  │  Arc<RwLock<HashMap>>                          │         │
//! │  │  ┌──────────────────────────────────────────┐ │         │
//! │  │  │  thread_id: "session-1"                  │ │         │
//! │  │  │    ├─ [0] CheckpointEntry (step -1)      │ │         │
//! │  │  │    ├─ [1] CheckpointEntry (step 0)       │ │         │
//! │  │  │    ├─ [2] CheckpointEntry (step 1)       │ │         │
//! │  │  │    └─ [3] CheckpointEntry (step 2)       │ │         │
//! │  │  │                                          │ │         │
//! │  │  │  thread_id: "session-2"                  │ │         │
//! │  │  │    ├─ [0] CheckpointEntry (step -1)      │ │         │
//! │  │  │    └─ [1] CheckpointEntry (step 0)       │ │         │
//! │  │  └──────────────────────────────────────────┘ │         │
//! │  │  • Each thread_id has a Vec<CheckpointEntry> │         │
//! │  │  • Entries sorted by insertion order         │         │
//! │  │  • Read/Write lock for concurrency           │         │
//! │  └────────────────────────────────────────────────┘         │
//! │                                                               │
//! │  CheckpointEntry:                                            │
//! │    • checkpoint: Checkpoint (state snapshot)                 │
//! │    • metadata: CheckpointMetadata (step, source, etc.)       │
//! │    • config: CheckpointConfig (thread_id, checkpoint_id)     │
//! │    • parent_config: Option<CheckpointConfig>                 │
//! │    • writes: Vec<(channel, value, task_id)>                  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Basic Usage with StateGraph
//!
//! ```rust,ignore
//! use langgraph_checkpoint::InMemoryCheckpointSaver;
//! use langgraph_core::{StateGraph, CheckpointConfig};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. Create checkpointer
//!     let checkpointer = InMemoryCheckpointSaver::new();
//!
//!     // 2. Build graph
//!     let mut graph = StateGraph::new();
//!     graph.add_node("step1", |state| {
//!         Box::pin(async move {
//!             Ok(json!({"count": state["count"].as_i64().unwrap() + 1}))
//!         })
//!     });
//!     graph.add_edge("__start__", "step1");
//!     graph.add_edge("step1", "__end__");
//!
//!     // 3. Compile with checkpointer
//!     let compiled = graph.compile()?.with_checkpointer(checkpointer);
//!
//!     // 4. Execute (checkpoints saved automatically)
//!     let config = CheckpointConfig::new()
//!         .with_thread_id("my-session".to_string());
//!
//!     let result = compiled.invoke_with_config(
//!         json!({"count": 0}),
//!         Some(config)
//!     ).await?;
//!
//!     println!("Result: {}", result);
//!     // Checkpoints are now in memory, can be replayed
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Retrieving Checkpoints
//!
//! ```rust,ignore
//! use langgraph_checkpoint::{InMemoryCheckpointSaver, CheckpointSaver};
//! use langgraph_checkpoint::CheckpointConfig;
//!
//! let checkpointer = InMemoryCheckpointSaver::new();
//!
//! // ... after running graph ...
//!
//! // Get latest checkpoint for a thread
//! let config = CheckpointConfig::new()
//!     .with_thread_id("my-session".to_string());
//!
//! if let Some(tuple) = checkpointer.get_tuple(&config).await? {
//!     println!("Latest checkpoint step: {:?}", tuple.metadata.step);
//!     println!("Channel values: {:?}", tuple.checkpoint.channel_values);
//! }
//! ```
//!
//! ## Testing with Multiple Threads
//!
//! ```rust,ignore
//! #[tokio::test]
//! async fn test_checkpoint_isolation() {
//!     let checkpointer = InMemoryCheckpointSaver::new();
//!
//!     // Thread 1
//!     let config1 = CheckpointConfig::new()
//!         .with_thread_id("thread-1".to_string());
//!     checkpointer.put(&config1, checkpoint1, metadata1, versions1).await?;
//!
//!     // Thread 2
//!     let config2 = CheckpointConfig::new()
//!         .with_thread_id("thread-2".to_string());
//!     checkpointer.put(&config2, checkpoint2, metadata2, versions2).await?;
//!
//!     // Verify isolation
//!     assert_eq!(checkpointer.thread_count().await, 2);
//!     assert_eq!(checkpointer.checkpoint_count().await, 2);
//!
//!     // Clean up for next test
//!     checkpointer.clear().await;
//!     assert_eq!(checkpointer.checkpoint_count().await, 0);
//! }
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Time-Travel Debugging
//!
//! Load and inspect historical checkpoints:
//!
//! ```rust,ignore
//! use langgraph_checkpoint::{InMemoryCheckpointSaver, CheckpointSaver};
//! use futures::StreamExt;
//!
//! let checkpointer = InMemoryCheckpointSaver::new();
//!
//! // List all checkpoints for a thread
//! let config = CheckpointConfig::new()
//!     .with_thread_id("debug-session".to_string());
//!
//! let mut stream = checkpointer.list(Some(&config), None, None, None).await?;
//!
//! while let Some(Ok(tuple)) = stream.next().await {
//!     println!("Step {}: {:?}",
//!         tuple.metadata.step.unwrap_or(-1),
//!         tuple.checkpoint.channel_values
//!     );
//! }
//! ```
//!
//! ## Pattern 2: Snapshot and Restore
//!
//! Save current state and restore later:
//!
//! ```rust,ignore
//! // Execute up to a certain point
//! let result1 = compiled.invoke_with_config(
//!     initial_state,
//!     Some(config.clone())
//! ).await?;
//!
//! // Get the checkpoint
//! let snapshot = checkpointer.get_tuple(&config).await?.unwrap();
//!
//! // ... do other work ...
//!
//! // Restore from snapshot and continue
//! let config_with_checkpoint = CheckpointConfig::new()
//!     .with_thread_id(config.thread_id.unwrap())
//!     .with_checkpoint_id(snapshot.checkpoint.id);
//!
//! let result2 = compiled.invoke_with_config(
//!     json!({}), // State loaded from checkpoint
//!     Some(config_with_checkpoint)
//! ).await?;
//! ```
//!
//! ## Pattern 3: A/B Testing with Forks
//!
//! Fork execution from a checkpoint to try different paths:
//!
//! ```rust,ignore
//! // Execute to decision point
//! let config = CheckpointConfig::new()
//!     .with_thread_id("experiment".to_string());
//! compiled.invoke_with_config(initial_state, Some(config.clone())).await?;
//!
//! // Get checkpoint at decision point
//! let decision_checkpoint = checkpointer.get_tuple(&config).await?.unwrap();
//!
//! // Fork A: Try option 1
//! let config_a = CheckpointConfig::new()
//!     .with_thread_id("experiment-fork-a".to_string());
//! let mut state_a = decision_checkpoint.checkpoint.channel_values.clone();
//! state_a.insert("option".to_string(), json!("A"));
//! compiled.invoke_with_config(state_a, Some(config_a)).await?;
//!
//! // Fork B: Try option 2
//! let config_b = CheckpointConfig::new()
//!     .with_thread_id("experiment-fork-b".to_string());
//! let mut state_b = decision_checkpoint.checkpoint.channel_values.clone();
//! state_b.insert("option".to_string(), json!("B"));
//! compiled.invoke_with_config(state_b, Some(config_b)).await?;
//!
//! // Compare results
//! let result_a = checkpointer.get_tuple(&config_a).await?;
//! let result_b = checkpointer.get_tuple(&config_b).await?;
//! ```
//!
//! ## Pattern 4: Test Isolation
//!
//! Clean slate for each test:
//!
//! ```rust,ignore
//! #[tokio::test]
//! async fn test_workflow_step_1() {
//!     let checkpointer = InMemoryCheckpointSaver::new();
//!
//!     // ... run test ...
//!
//!     // Cleanup
//!     checkpointer.clear().await;
//! }
//!
//! #[tokio::test]
//! async fn test_workflow_step_2() {
//!     let checkpointer = InMemoryCheckpointSaver::new();
//!
//!     // ... run test ...
//!
//!     // Cleanup
//!     checkpointer.clear().await;
//! }
//! ```
//!
//! # Helper Methods
//!
//! Beyond the `CheckpointSaver` trait, `InMemoryCheckpointSaver` provides utility methods:
//!
//! ## thread_count()
//!
//! Get number of distinct threads being tracked:
//!
//! ```rust,ignore
//! let count = checkpointer.thread_count().await;
//! println!("Tracking {} threads", count);
//! ```
//!
//! ## checkpoint_count()
//!
//! Get total number of checkpoints across all threads:
//!
//! ```rust,ignore
//! let count = checkpointer.checkpoint_count().await;
//! println!("Stored {} checkpoints", count);
//! ```
//!
//! ## clear()
//!
//! Delete all checkpoints (useful for testing):
//!
//! ```rust,ignore
//! checkpointer.clear().await;
//! assert_eq!(checkpointer.checkpoint_count().await, 0);
//! ```
//!
//! # Performance Characteristics
//!
//! ## Time Complexity
//!
//! | Operation | Complexity | Notes |
//! |-----------|------------|-------|
//! | `put()` | O(1) amortized | Append to Vec |
//! | `get_tuple()` | O(n) | Linear scan through thread's checkpoints |
//! | `list()` | O(n*m) | n = threads, m = checkpoints per thread |
//! | `delete_thread()` | O(1) | Remove HashMap entry |
//!
//! ## Space Complexity
//!
//! - **Per Checkpoint**: Size of serialized state + metadata (~1KB to 1MB typical)
//! - **Per Thread**: Sum of all checkpoint sizes for that thread
//! - **Total**: All threads * avg checkpoints per thread * avg checkpoint size
//!
//! ## Concurrency
//!
//! - **Read-Heavy**: Multiple concurrent reads via `RwLock::read()`
//! - **Write-Heavy**: Serial writes via `RwLock::write()` (blocks readers)
//! - **Recommendation**: For high write concurrency, use database-backed checkpointer
//!
//! # Memory Management
//!
//! ## Growth Pattern
//!
//! Memory grows linearly with:
//! 1. Number of concurrent threads
//! 2. Number of steps per workflow
//! 3. Size of state at each step
//!
//! ## Example Memory Usage
//!
//! ```text
//! Assumptions:
//!   • 100 concurrent threads
//!   • 10 checkpoints per thread (10 steps)
//!   • 10KB per checkpoint (small state)
//!
//! Total: 100 * 10 * 10KB = 10MB
//! ```
//!
//! For large states (>1MB), consider database storage.
//!
//! # Limitations
//!
//! 1. **No Persistence** - All data lost on restart
//! 2. **Single Process** - Cannot share across processes
//! 3. **Memory Bound** - Limited by available RAM
//! 4. **No Cleanup** - Checkpoints never auto-deleted (manual `clear()` needed)
//! 5. **Linear Scan** - `get_tuple()` scans all checkpoints for thread
//!
//! # Migration to Production Backend
//!
//! When ready for production, swap implementation:
//!
//! ```rust,ignore
//! // Development
//! let checkpointer = InMemoryCheckpointSaver::new();
//!
//! // Production - PostgreSQL
//! let checkpointer = PostgresCheckpointSaver::new("postgres://...").await?;
//!
//! // Production - Redis
//! let checkpointer = RedisCheckpointSaver::new("redis://...").await?;
//!
//! // Application code stays the same!
//! let compiled = graph.compile()?.with_checkpointer(checkpointer);
//! ```
//!
//! # Best Practices
//!
//! 1. **Use for Testing** - Perfect for unit/integration tests with `clear()` between tests
//! 2. **Limit Workflow Length** - Keep workflows under 100 steps to manage memory
//! 3. **Monitor Memory** - Track `checkpoint_count()` in long-running apps
//! 4. **Thread Cleanup** - Call `delete_thread()` when workflows complete
//! 5. **Development Only** - Switch to persistent backend for production
//! 6. **Shallow Cloning** - Uses `Clone` so shared instances share same data
//!
//! # Comparison with Database Backends
//!
//! | Feature | In-Memory | PostgreSQL | Redis |
//! |---------|-----------|------------|-------|
//! | Persistence | ❌ | ✅ | ✅ |
//! | Multi-process | ❌ | ✅ | ✅ |
//! | Latency | ⚡ <1µs | ⚡⚡ 1-5ms | ⚡ <1ms |
//! | Scalability | ❌ Limited | ✅ Excellent | ✅ Good |
//! | Setup | ✅ Zero | ❌ Database required | ❌ Redis required |
//! | Testing | ✅ Perfect | ⚡ Good | ⚡ Good |
//! | Production | ❌ Not recommended | ✅ Recommended | ✅ For caching |
//!
//! # See Also
//!
//! - [`CheckpointSaver`](crate::traits::CheckpointSaver) - Trait this implements
//! - [`Checkpoint`](crate::checkpoint::Checkpoint) - Checkpoint data structure
//! - [`CheckpointConfig`](crate::checkpoint::CheckpointConfig) - Configuration type
//! - [Custom backends guide](crate::traits) - Implement your own storage backend

use crate::{
    checkpoint::{
        ChannelVersions, Checkpoint, CheckpointConfig, CheckpointMetadata, CheckpointTuple,
    },
    error::{CheckpointError, Result},
    traits::{CheckpointSaver, CheckpointStream},
};
use async_trait::async_trait;
use futures::stream;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Storage entry for in-memory checkpoints
#[derive(Debug, Clone)]
struct CheckpointEntry {
    checkpoint: Checkpoint,
    metadata: CheckpointMetadata,
    config: CheckpointConfig,
    parent_config: Option<CheckpointConfig>,
    writes: Vec<(String, serde_json::Value, String)>, // (channel, value, task_id)
}

/// Thread-safe in-memory checkpoint storage
type CheckpointStorage = Arc<RwLock<HashMap<String, Vec<CheckpointEntry>>>>;

/// In-memory checkpoint saver implementation
///
/// This is a reference implementation that stores all checkpoints in memory.
/// It's suitable for development, testing, and small-scale applications.
///
/// For production use with persistence, implement the `CheckpointSaver` trait
/// with your preferred backend (PostgreSQL, SQLite, Redis, etc.).
///
/// # Example
///
/// ```rust
/// use langgraph_checkpoint::{InMemoryCheckpointSaver, CheckpointSaver};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let saver = InMemoryCheckpointSaver::new();
///
///     // Use the checkpoint saver...
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct InMemoryCheckpointSaver {
    storage: CheckpointStorage,
}

impl InMemoryCheckpointSaver {
    /// Create a new in-memory checkpoint saver
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the number of threads being tracked
    pub async fn thread_count(&self) -> usize {
        self.storage.read().await.len()
    }

    /// Get the total number of checkpoints across all threads
    pub async fn checkpoint_count(&self) -> usize {
        self.storage
            .read()
            .await
            .values()
            .map(|entries| entries.len())
            .sum()
    }

    /// Clear all checkpoints (useful for testing)
    pub async fn clear(&self) {
        self.storage.write().await.clear();
    }
}

impl Default for InMemoryCheckpointSaver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CheckpointSaver for InMemoryCheckpointSaver {
    async fn get_tuple(&self, config: &CheckpointConfig) -> Result<Option<CheckpointTuple>> {
        let storage = self.storage.read().await;

        let thread_id = config
            .thread_id
            .as_ref()
            .ok_or_else(|| CheckpointError::Invalid("thread_id is required".to_string()))?;

        if let Some(entries) = storage.get(thread_id) {
            if let Some(checkpoint_id) = &config.checkpoint_id {
                // Find specific checkpoint by ID
                if let Some(entry) = entries.iter().find(|e| &e.checkpoint.id == checkpoint_id) {
                    return Ok(Some(CheckpointTuple {
                        config: entry.config.clone(),
                        checkpoint: entry.checkpoint.clone(),
                        metadata: entry.metadata.clone(),
                        parent_config: entry.parent_config.clone(),
                    }));
                }
            } else {
                // Return the latest checkpoint
                if let Some(entry) = entries.last() {
                    return Ok(Some(CheckpointTuple {
                        config: entry.config.clone(),
                        checkpoint: entry.checkpoint.clone(),
                        metadata: entry.metadata.clone(),
                        parent_config: entry.parent_config.clone(),
                    }));
                }
            }
        }

        Ok(None)
    }

    async fn list(
        &self,
        config: Option<&CheckpointConfig>,
        filter: Option<HashMap<String, serde_json::Value>>,
        before: Option<&CheckpointConfig>,
        limit: Option<usize>,
    ) -> Result<CheckpointStream> {
        let storage = self.storage.read().await;
        let mut results = Vec::new();

        // Determine which threads to search
        let thread_ids: Vec<String> = if let Some(cfg) = config {
            if let Some(thread_id) = &cfg.thread_id {
                vec![thread_id.clone()]
            } else {
                storage.keys().cloned().collect()
            }
        } else {
            storage.keys().cloned().collect()
        };

        // Collect matching checkpoints
        for thread_id in thread_ids {
            if let Some(entries) = storage.get(&thread_id) {
                for entry in entries.iter().rev() {
                    // Apply before filter
                    if let Some(before_cfg) = before {
                        if let Some(before_id) = &before_cfg.checkpoint_id {
                            if entry.checkpoint.id >= *before_id {
                                continue;
                            }
                        }
                    }

                    // Apply metadata filter
                    if let Some(filter_map) = &filter {
                        let mut matches = true;
                        for (key, value) in filter_map {
                            if entry.metadata.extra.get(key) != Some(value) {
                                matches = false;
                                break;
                            }
                        }
                        if !matches {
                            continue;
                        }
                    }

                    results.push(Ok(CheckpointTuple {
                        config: entry.config.clone(),
                        checkpoint: entry.checkpoint.clone(),
                        metadata: entry.metadata.clone(),
                        parent_config: entry.parent_config.clone(),
                    }));

                    if let Some(lim) = limit {
                        if results.len() >= lim {
                            break;
                        }
                    }
                }

                if let Some(lim) = limit {
                    if results.len() >= lim {
                        break;
                    }
                }
            }
        }

        Ok(Box::pin(stream::iter(results)))
    }

    async fn put(
        &self,
        config: &CheckpointConfig,
        checkpoint: Checkpoint,
        metadata: CheckpointMetadata,
        _new_versions: ChannelVersions,
    ) -> Result<CheckpointConfig> {
        let thread_id = config
            .thread_id
            .as_ref()
            .ok_or_else(|| CheckpointError::Invalid("thread_id is required".to_string()))?;

        let mut storage = self.storage.write().await;
        let entries = storage.entry(thread_id.clone()).or_insert_with(Vec::new);

        // Create the config for this checkpoint
        let checkpoint_config = CheckpointConfig {
            thread_id: Some(thread_id.clone()),
            checkpoint_id: Some(checkpoint.id.clone()),
            checkpoint_ns: config.checkpoint_ns.clone(),
            extra: config.extra.clone(),
        };

        let entry = CheckpointEntry {
            checkpoint,
            metadata,
            config: checkpoint_config.clone(),
            parent_config: config.checkpoint_id.as_ref().map(|_| config.clone()),
            writes: Vec::new(),
        };

        entries.push(entry);

        Ok(checkpoint_config)
    }

    async fn put_writes(
        &self,
        config: &CheckpointConfig,
        writes: Vec<(String, serde_json::Value)>,
        task_id: String,
    ) -> Result<()> {
        let thread_id = config
            .thread_id
            .as_ref()
            .ok_or_else(|| CheckpointError::Invalid("thread_id is required".to_string()))?;

        let checkpoint_id = config
            .checkpoint_id
            .as_ref()
            .ok_or_else(|| CheckpointError::Invalid("checkpoint_id is required".to_string()))?;

        let mut storage = self.storage.write().await;

        if let Some(entries) = storage.get_mut(thread_id) {
            if let Some(entry) = entries
                .iter_mut()
                .find(|e| &e.checkpoint.id == checkpoint_id)
            {
                for (channel, value) in writes {
                    entry.writes.push((channel, value, task_id.clone()));
                }
                return Ok(());
            }
        }

        Err(CheckpointError::NotFound(format!(
            "Checkpoint not found: {}",
            checkpoint_id
        )))
    }

    async fn delete_thread(&self, thread_id: &str) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.remove(thread_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::CheckpointSource;

    #[tokio::test]
    async fn test_save_and_load_checkpoint() {
        let saver = InMemoryCheckpointSaver::new();
        let checkpoint = Checkpoint::empty();
        let metadata = CheckpointMetadata::new().with_source(CheckpointSource::Input);
        let config = CheckpointConfig::new().with_thread_id("thread-1".to_string());

        // Save checkpoint
        let saved_config = saver
            .put(&config, checkpoint.clone(), metadata, HashMap::new())
            .await
            .unwrap();

        assert!(saved_config.checkpoint_id.is_some());

        // Load checkpoint
        let loaded = saver.get_tuple(&saved_config).await.unwrap();
        assert!(loaded.is_some());

        let tuple = loaded.unwrap();
        assert_eq!(tuple.checkpoint.id, checkpoint.id);
    }

    #[tokio::test]
    async fn test_list_checkpoints() {
        let saver = InMemoryCheckpointSaver::new();
        let config = CheckpointConfig::new().with_thread_id("thread-1".to_string());

        // Save multiple checkpoints
        for i in 0..3 {
            let checkpoint = Checkpoint::empty();
            let metadata = CheckpointMetadata::new().with_step(i);
            saver
                .put(&config, checkpoint, metadata, HashMap::new())
                .await
                .unwrap();
        }

        // List all checkpoints
        let stream = saver.list(Some(&config), None, None, None).await.unwrap();
        use futures::StreamExt;
        let results: Vec<_> = stream.collect().await;

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_thread() {
        let saver = InMemoryCheckpointSaver::new();
        let checkpoint = Checkpoint::empty();
        let metadata = CheckpointMetadata::new();
        let config = CheckpointConfig::new().with_thread_id("thread-1".to_string());

        // Save checkpoint
        saver
            .put(&config, checkpoint, metadata, HashMap::new())
            .await
            .unwrap();

        assert_eq!(saver.thread_count().await, 1);

        // Delete thread
        saver.delete_thread("thread-1").await.unwrap();

        assert_eq!(saver.thread_count().await, 0);
    }

    #[tokio::test]
    async fn test_put_writes() {
        let saver = InMemoryCheckpointSaver::new();
        let checkpoint = Checkpoint::empty();
        let metadata = CheckpointMetadata::new();
        let config = CheckpointConfig::new().with_thread_id("thread-1".to_string());

        // Save checkpoint
        let saved_config = saver
            .put(&config, checkpoint, metadata, HashMap::new())
            .await
            .unwrap();

        // Add writes
        let writes = vec![
            ("channel1".to_string(), serde_json::json!(42)),
            ("channel2".to_string(), serde_json::json!("hello")),
        ];

        saver
            .put_writes(&saved_config, writes, "task-1".to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_clear() {
        let saver = InMemoryCheckpointSaver::new();
        let checkpoint = Checkpoint::empty();
        let metadata = CheckpointMetadata::new();
        let config = CheckpointConfig::new().with_thread_id("thread-1".to_string());

        saver
            .put(&config, checkpoint, metadata, HashMap::new())
            .await
            .unwrap();

        assert_eq!(saver.checkpoint_count().await, 1);

        saver.clear().await;

        assert_eq!(saver.checkpoint_count().await, 0);
    }
}
