//! Extensible checkpoint storage trait for custom backend implementations
//!
//! This module defines the **[`CheckpointSaver`]** trait - the core abstraction for implementing
//! checkpoint persistence backends. The trait enables downstream projects to integrate rLangGraph
//! with any storage system (PostgreSQL, SQLite, Redis, MongoDB, S3, etc.) while maintaining
//! compatibility with the checkpointing system.
//!
//! # Overview
//!
//! The checkpoint system provides:
//!
//! - **State Persistence** - Save and restore complete graph execution state
//! - **Time-Travel Debugging** - Inspect and replay from historical states
//! - **Fault Recovery** - Resume execution after crashes or failures
//! - **Human-in-the-Loop** - Pause, inspect, modify, and resume workflows
//! - **Audit Trails** - Track all state transitions with metadata
//! - **Concurrent Execution** - Thread-isolated checkpoints per execution thread
//! - **Version Management** - Track channel versions across checkpoints
//!
//! # Core Types
//!
//! - [`CheckpointSaver`] - Main trait for storage backend implementation
//! - [`CheckpointTuple`] - Complete checkpoint with config, metadata, and state
//! - [`Checkpoint`] - Serialized graph state at a point in time
//! - [`CheckpointConfig`] - Thread ID and timestamp identifying a checkpoint
//! - [`CheckpointMetadata`] - Additional metadata (step, source, writes, etc.)
//! - [`CheckpointStream`] - Async stream of checkpoint query results
//! - [`ChannelVersions`] - Version tracking for state channels
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  rLangGraph Core                                             │
//! │  ┌────────────────────────────────────────────────┐         │
//! │  │  CompiledGraph Execution                       │         │
//! │  │  • Execute superstep                            │         │
//! │  │  • Collect channel updates                      │         │
//! │  │  • Call checkpointer.put()                      │         │
//! │  └────────────┬───────────────────────────────────┘         │
//! └───────────────┼──────────────────────────────────────────────┘
//!                 │ CheckpointSaver trait
//!                 ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Storage Backend (Your Implementation)                       │
//! │  ┌────────────────────────────────────────────────┐         │
//! │  │  impl CheckpointSaver for PostgresBackend {    │         │
//! │  │    async fn put(...) -> Result<Config> {       │         │
//! │  │      // Serialize checkpoint                    │         │
//! │  │      // INSERT INTO checkpoints ...             │         │
//! │  │      // Return updated config                   │         │
//! │  │    }                                            │         │
//! │  │                                                 │         │
//! │  │    async fn get_tuple(...) -> CheckpointTuple { │         │
//! │  │      // SELECT FROM checkpoints WHERE ...       │         │
//! │  │      // Deserialize and return                  │         │
//! │  │    }                                            │         │
//! │  │                                                 │         │
//! │  │    async fn list(...) -> CheckpointStream {     │         │
//! │  │      // Stream results from database            │         │
//! │  │    }                                            │         │
//! │  │  }                                              │         │
//! │  └────────────────────────────────────────────────┘         │
//! │           │                                                  │
//! │           ↓                                                  │
//! │  ┌────────────────────────────────────────────────┐         │
//! │  │  Database / Storage System                     │         │
//! │  │  • PostgreSQL                                   │         │
//! │  │  • SQLite                                       │         │
//! │  │  • Redis                                        │         │
//! │  │  • MongoDB                                      │         │
//! │  │  • S3 / Object Storage                          │         │
//! │  │  • Custom solution                              │         │
//! │  └────────────────────────────────────────────────┘         │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Using the In-Memory Backend
//!
//! ```rust,ignore
//! use langgraph_checkpoint::InMemoryCheckpointSaver;
//! use langgraph_core::{StateGraph, CheckpointConfig};
//!
//! let checkpointer = InMemoryCheckpointSaver::new();
//!
//! let mut graph = StateGraph::new();
//! // ... build graph ...
//!
//! let compiled = graph.compile()?.with_checkpointer(checkpointer);
//!
//! // Checkpoints automatically saved after each superstep
//! let result = compiled.invoke(initial_state).await?;
//! ```
//!
//! ## Implementing Custom Backend
//!
//! ```rust,ignore
//! use langgraph_checkpoint::{
//!     CheckpointSaver, Checkpoint, CheckpointConfig, CheckpointMetadata,
//!     CheckpointTuple, CheckpointStream, ChannelVersions,
//! };
//! use async_trait::async_trait;
//! use std::sync::Arc;
//!
//! pub struct MyDatabaseCheckpointer {
//!     connection_pool: Arc<DatabasePool>,
//! }
//!
//! #[async_trait]
//! impl CheckpointSaver for MyDatabaseCheckpointer {
//!     async fn put(
//!         &self,
//!         config: &CheckpointConfig,
//!         checkpoint: Checkpoint,
//!         metadata: CheckpointMetadata,
//!         new_versions: ChannelVersions,
//!     ) -> langgraph_checkpoint::Result<CheckpointConfig> {
//!         // 1. Serialize checkpoint to bytes
//!         let data = serde_json::to_vec(&checkpoint)?;
//!
//!         // 2. Store in your database
//!         self.connection_pool.execute(
//!             "INSERT INTO checkpoints (thread_id, ts, data, metadata) VALUES ($1, $2, $3, $4)",
//!             &[&config.thread_id, &config.checkpoint_ts, &data, &metadata]
//!         ).await?;
//!
//!         // 3. Return the config
//!         Ok(config.clone())
//!     }
//!
//!     async fn get_tuple(
//!         &self,
//!         config: &CheckpointConfig,
//!     ) -> langgraph_checkpoint::Result<Option<CheckpointTuple>> {
//!         // Query database and deserialize
//!         let row = self.connection_pool.query_one(
//!             "SELECT data, metadata FROM checkpoints WHERE thread_id = $1 AND ts = $2",
//!             &[&config.thread_id, &config.checkpoint_ts]
//!         ).await?;
//!
//!         let checkpoint: Checkpoint = serde_json::from_slice(&row.get::<_, Vec<u8>>("data"))?;
//!         let metadata: CheckpointMetadata = row.get("metadata");
//!
//!         Ok(Some(CheckpointTuple {
//!             config: config.clone(),
//!             checkpoint,
//!             metadata,
//!             parent_config: None,
//!         }))
//!     }
//!
//!     async fn list(
//!         &self,
//!         config: Option<&CheckpointConfig>,
//!         filter: Option<HashMap<String, Value>>,
//!         before: Option<&CheckpointConfig>,
//!         limit: Option<usize>,
//!     ) -> langgraph_checkpoint::Result<CheckpointStream> {
//!         // Return async stream of results
//!         // (implementation depends on your database driver)
//!         todo!("Implement based on your database")
//!     }
//!
//!     async fn put_writes(
//!         &self,
//!         config: &CheckpointConfig,
//!         writes: Vec<(String, Value)>,
//!         task_id: String,
//!     ) -> langgraph_checkpoint::Result<()> {
//!         // Store pending writes for this checkpoint
//!         for (channel, value) in writes {
//!             self.connection_pool.execute(
//!                 "INSERT INTO checkpoint_writes VALUES ($1, $2, $3, $4)",
//!                 &[&config.thread_id, &config.checkpoint_ts, &channel, &value]
//!             ).await?;
//!         }
//!         Ok(())
//!     }
//! }
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Database-Backed Checkpoint (PostgreSQL)
//!
//! ```rust,ignore
//! use sqlx::{PgPool, postgres::PgRow, Row};
//! use langgraph_checkpoint::*;
//! use async_trait::async_trait;
//! use std::sync::Arc;
//!
//! pub struct PostgresCheckpointer {
//!     pool: Arc<PgPool>,
//! }
//!
//! impl PostgresCheckpointer {
//!     pub async fn new(database_url: &str) -> Result<Self> {
//!         let pool = PgPool::connect(database_url).await?;
//!
//!         // Create tables if they don't exist
//!         sqlx::query(r#"
//!             CREATE TABLE IF NOT EXISTS checkpoints (
//!                 thread_id TEXT NOT NULL,
//!                 ts TIMESTAMP NOT NULL,
//!                 checkpoint JSONB NOT NULL,
//!                 metadata JSONB NOT NULL,
//!                 PRIMARY KEY (thread_id, ts)
//!             )
//!         "#).execute(&pool).await?;
//!
//!         Ok(Self {
//!             pool: Arc::new(pool),
//!         })
//!     }
//! }
//!
//! #[async_trait]
//! impl CheckpointSaver for PostgresCheckpointer {
//!     async fn put(
//!         &self,
//!         config: &CheckpointConfig,
//!         checkpoint: Checkpoint,
//!         metadata: CheckpointMetadata,
//!         _versions: ChannelVersions,
//!     ) -> langgraph_checkpoint::Result<CheckpointConfig> {
//!         let checkpoint_json = serde_json::to_value(&checkpoint)?;
//!         let metadata_json = serde_json::to_value(&metadata)?;
//!
//!         sqlx::query(
//!             "INSERT INTO checkpoints (thread_id, ts, checkpoint, metadata)
//!              VALUES ($1, $2, $3, $4)
//!              ON CONFLICT (thread_id, ts) DO UPDATE
//!              SET checkpoint = $3, metadata = $4"
//!         )
//!         .bind(&config.thread_id)
//!         .bind(&config.checkpoint_ts)
//!         .bind(&checkpoint_json)
//!         .bind(&metadata_json)
//!         .execute(&*self.pool)
//!         .await?;
//!
//!         Ok(config.clone())
//!     }
//!
//!     // ... implement other methods ...
//! }
//! ```
//!
//! ## Pattern 2: Redis-Backed Checkpoint (Cache Layer)
//!
//! ```rust,ignore
//! use redis::{Client, AsyncCommands};
//! use langgraph_checkpoint::*;
//! use async_trait::async_trait;
//!
//! pub struct RedisCheckpointer {
//!     client: Client,
//!     ttl_seconds: usize,
//! }
//!
//! #[async_trait]
//! impl CheckpointSaver for RedisCheckpointer {
//!     async fn put(
//!         &self,
//!         config: &CheckpointConfig,
//!         checkpoint: Checkpoint,
//!         metadata: CheckpointMetadata,
//!         _versions: ChannelVersions,
//!     ) -> langgraph_checkpoint::Result<CheckpointConfig> {
//!         let mut conn = self.client.get_async_connection().await?;
//!
//!         let key = format!("checkpoint:{}:{}", config.thread_id, config.checkpoint_ts);
//!         let value = serde_json::to_string(&(checkpoint, metadata))?;
//!
//!         // Store with TTL for automatic expiration
//!         conn.set_ex(key, value, self.ttl_seconds).await?;
//!
//!         Ok(config.clone())
//!     }
//!
//!     async fn get_tuple(
//!         &self,
//!         config: &CheckpointConfig,
//!     ) -> langgraph_checkpoint::Result<Option<CheckpointTuple>> {
//!         let mut conn = self.client.get_async_connection().await?;
//!         let key = format!("checkpoint:{}:{}", config.thread_id, config.checkpoint_ts);
//!
//!         let value: Option<String> = conn.get(key).await?;
//!         if let Some(json) = value {
//!             let (checkpoint, metadata) = serde_json::from_str(&json)?;
//!             Ok(Some(CheckpointTuple {
//!                 config: config.clone(),
//!                 checkpoint,
//!                 metadata,
//!                 parent_config: None,
//!             }))
//!         } else {
//!             Ok(None)
//!         }
//!     }
//!
//!     // ... implement other methods ...
//! }
//! ```
//!
//! ## Pattern 3: Hybrid Storage (Hot + Cold)
//!
//! Recent checkpoints in Redis, older ones in PostgreSQL:
//!
//! ```rust,ignore
//! pub struct HybridCheckpointer {
//!     redis: Arc<RedisCheckpointer>,
//!     postgres: Arc<PostgresCheckpointer>,
//!     hot_threshold_hours: i64,
//! }
//!
//! #[async_trait]
//! impl CheckpointSaver for HybridCheckpointer {
//!     async fn put(
//!         &self,
//!         config: &CheckpointConfig,
//!         checkpoint: Checkpoint,
//!         metadata: CheckpointMetadata,
//!         versions: ChannelVersions,
//!     ) -> langgraph_checkpoint::Result<CheckpointConfig> {
//!         // Always write to both (Redis for speed, Postgres for durability)
//!         let redis_fut = self.redis.put(config, checkpoint.clone(), metadata.clone(), versions.clone());
//!         let pg_fut = self.postgres.put(config, checkpoint, metadata, versions);
//!
//!         tokio::try_join!(redis_fut, pg_fut)?;
//!         Ok(config.clone())
//!     }
//!
//!     async fn get_tuple(
//!         &self,
//!         config: &CheckpointConfig,
//!     ) -> langgraph_checkpoint::Result<Option<CheckpointTuple>> {
//!         // Try Redis first (hot path)
//!         if let Some(tuple) = self.redis.get_tuple(config).await? {
//!             return Ok(Some(tuple));
//!         }
//!
//!         // Fall back to Postgres (cold storage)
//!         self.postgres.get_tuple(config).await
//!     }
//!
//!     // ... other methods ...
//! }
//! ```
//!
//! # Method Reference
//!
//! ## put() - Save Checkpoint
//!
//! Called after each superstep to persist graph state.
//!
//! **Parameters:**
//! - `config`: Thread ID + timestamp identifying this checkpoint
//! - `checkpoint`: Complete graph state (channels, pending writes, versions)
//! - `metadata`: Step number, source, parent checkpoint, etc.
//! - `new_versions`: Updated channel version numbers
//!
//! **Returns:** Updated `CheckpointConfig` (may include new timestamp)
//!
//! ## get_tuple() - Retrieve Checkpoint
//!
//! Fetch a specific checkpoint by thread ID and timestamp.
//!
//! **Parameters:**
//! - `config`: Identifies which checkpoint to retrieve
//!
//! **Returns:** Complete `CheckpointTuple` with config, state, and metadata, or `None`
//!
//! ## list() - Query Checkpoints
//!
//! Stream checkpoints matching filter criteria.
//!
//! **Parameters:**
//! - `config`: Base filter (thread ID, optionally timestamp)
//! - `filter`: Additional metadata filters (e.g., `{"step": 5}`)
//! - `before`: Only return checkpoints before this config
//! - `limit`: Maximum results to return
//!
//! **Returns:** Async stream of matching checkpoints
//!
//! ## put_writes() - Store Pending Writes
//!
//! Save intermediate writes that haven't been committed to a checkpoint yet.
//!
//! **Parameters:**
//! - `config`: Associated checkpoint
//! - `writes`: List of (channel_name, value) pairs
//! - `task_id`: Task that created these writes
//!
//! **Use case:** Send-based map-reduce where worker results arrive asynchronously
//!
//! # Thread Safety
//!
//! All `CheckpointSaver` implementations must be:
//!
//! - **`Send + Sync`**: Safe to share across threads
//! - **Concurrent-safe**: Handle simultaneous reads/writes
//! - **Isolation**: Each thread_id creates independent checkpoint history
//!
//! # Serialization
//!
//! Checkpoints are serialized as JSON by default (via `serde_json`). Custom serialization:
//!
//! ```rust,ignore
//! // Use MessagePack for smaller size
//! let bytes = rmp_serde::to_vec(&checkpoint)?;
//!
//! // Use Bincode for speed
//! let bytes = bincode::serialize(&checkpoint)?;
//!
//! // Custom compression
//! let json = serde_json::to_vec(&checkpoint)?;
//! let compressed = zstd::encode_all(&json[..], 3)?;
//! ```
//!
//! # Error Handling
//!
//! All methods return `langgraph_checkpoint::Result<T>` which wraps storage errors:
//!
//! ```rust,ignore
//! match checkpointer.get_tuple(&config).await {
//!     Ok(Some(tuple)) => { /* found */ }
//!     Ok(None) => { /* not found */ }
//!     Err(e) => {
//!         eprintln!("Storage error: {}", e);
//!         // Handle database connection errors, serialization errors, etc.
//!     }
//! }
//! ```
//!
//! # Best Practices
//!
//! 1. **Index thread_id and ts** - These are the primary query keys
//! 2. **Store metadata separately** - Enables filtering without deserializing full checkpoints
//! 3. **Use connection pooling** - Reuse database connections across checkpoints
//! 4. **Implement TTL** - Auto-delete old checkpoints to manage storage
//! 5. **Compress large states** - Use zstd/gzip for states >1MB
//! 6. **Batch writes** - Combine multiple put_writes() calls when possible
//! 7. **Test replay** - Verify checkpoints can be loaded and executed from
//!
//! # Comparison with Python LangGraph
//!
//! | Python LangGraph | rLangGraph | Notes |
//! |------------------|------------|-------|
//! | `BaseSaver` class | `CheckpointSaver` trait | Rust uses traits for polymorphism |
//! | `aget()`, `aput()` methods | `async fn get/put` | Rust async/await |
//! | `MemorySaver` | `InMemoryCheckpointSaver` | Equivalent in-memory backend |
//! | Tuple serialization | `CheckpointTuple` struct | Strongly typed in Rust |
//! | `checkpoint_id` str | `checkpoint_ts` as Option<String> | Rust uses typed timestamps |
//! | Python pickling | JSON (serde) serialization | Cross-language compatible |
//!
//! # See Also
//!
//! - [`InMemoryCheckpointSaver`](crate::memory::InMemoryCheckpointSaver) - Reference implementation
//! - [`Checkpoint`](crate::checkpoint::Checkpoint) - Checkpoint data structure
//! - [`CheckpointConfig`](crate::checkpoint::CheckpointConfig) - Configuration and identification
//! - [`CompiledGraph::with_checkpointer()`](langgraph_core::compiled::CompiledGraph) - Attach checkpointer to graph

use crate::{
    checkpoint::{
        ChannelVersions, Checkpoint, CheckpointConfig, CheckpointMetadata, CheckpointTuple,
    },
    error::Result,
};
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

/// Type alias for async stream of checkpoint tuples
pub type CheckpointStream =
    Pin<Box<dyn Stream<Item = Result<CheckpointTuple>> + Send + 'static>>;

/// Core trait for implementing checkpoint storage backends
///
/// `CheckpointSaver` defines the interface that all checkpoint storage
/// implementations must provide. This trait enables LangGraph graphs to
/// persist their state across executions, supporting features like fault
/// recovery, time-travel debugging, and human-in-the-loop workflows.
///
/// ## Required Methods
///
/// Implementations must provide:
/// - `get_tuple` - Retrieve a specific checkpoint
/// - `list` - List checkpoints with filtering
/// - `put` - Save a new checkpoint
///
/// ## Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`) to support
/// concurrent graph executions.
///
/// ## Example: Custom Database Backend
///
/// ```rust,no_run
/// use langgraph_checkpoint::{
///     CheckpointSaver, Checkpoint, CheckpointConfig, CheckpointMetadata,
///     CheckpointTuple, CheckpointStream, ChannelVersions,
/// };
/// use async_trait::async_trait;
/// use std::sync::Arc;
///
/// struct PostgresCheckpointSaver {
///     pool: Arc<PgPool>,
/// }
///
/// #[async_trait]
/// impl CheckpointSaver for PostgresCheckpointSaver {
///     async fn get_tuple(
///         &self,
///         config: &CheckpointConfig,
///     ) -> langgraph_checkpoint::Result<Option<CheckpointTuple>> {
///         // Query database for checkpoint
///         let query = "SELECT * FROM checkpoints WHERE thread_id = $1 AND ts = $2";
///         // ... execute query and deserialize ...
///         Ok(None)
///     }
///
///     async fn list(
///         &self,
///         config: Option<&CheckpointConfig>,
///         filter: Option<std::collections::HashMap<String, serde_json::Value>>,
///         before: Option<&CheckpointConfig>,
///         limit: Option<usize>,
///     ) -> langgraph_checkpoint::Result<CheckpointStream> {
///         // Implementation here
///         unimplemented!()
///     }
///
///     async fn put(
///         &self,
///         config: &CheckpointConfig,
///         checkpoint: Checkpoint,
///         metadata: CheckpointMetadata,
///         new_versions: ChannelVersions,
///     ) -> langgraph_checkpoint::Result<CheckpointConfig> {
///         // Implementation here
///         Ok(config.clone())
///     }
///
///     async fn put_writes(
///         &self,
///         config: &CheckpointConfig,
///         writes: Vec<(String, serde_json::Value)>,
///         task_id: String,
///     ) -> langgraph_checkpoint::Result<()> {
///         // Implementation here
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait CheckpointSaver: Send + Sync {
    /// Fetch a checkpoint using the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration specifying which checkpoint to retrieve
    ///
    /// # Returns
    ///
    /// The requested checkpoint, or `None` if not found
    async fn get(&self, config: &CheckpointConfig) -> Result<Option<Checkpoint>> {
        if let Some(tuple) = self.get_tuple(config).await? {
            Ok(Some(tuple.checkpoint))
        } else {
            Ok(None)
        }
    }

    /// Retrieve a complete checkpoint tuple with metadata.
    ///
    /// This is the **primary read operation** for checkpoint retrieval, providing the full
    /// checkpoint state plus metadata for inspection, debugging, and resumption.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration specifying which checkpoint to retrieve:
    ///   - If `checkpoint_id` is provided: Retrieve that specific version
    ///   - If only `thread_id` provided: Retrieve the **latest** checkpoint for that thread
    ///   - `checkpoint_ns` further filters within a thread
    ///
    /// # Returns
    ///
    /// - `Ok(Some(CheckpointTuple))` - Checkpoint found and loaded
    /// - `Ok(None)` - No checkpoint exists matching the config
    /// - `Err` - Storage error occurred
    ///
    /// # CheckpointTuple Structure
    ///
    /// The returned tuple contains:
    /// - `config` - Actual config of the stored checkpoint (with real checkpoint_id)
    /// - `checkpoint` - Complete graph state (channels, versions, tasks)
    /// - `metadata` - Execution metadata (step number, source, writes)
    /// - `parent_config` - Optional config of the previous checkpoint (for history traversal)
    /// - `pending_writes` - Optional uncommitted writes
    ///
    /// # Implementation Requirements
    ///
    /// Your implementation MUST:
    /// 1. **Query by thread_id** (primary key) and checkpoint_id (if provided)
    /// 2. **Return latest** if no checkpoint_id specified
    /// 3. **Deserialize** checkpoint and metadata from storage
    /// 4. **Include parent_config** for checkpoint chaining
    /// 5. **Return None** (not error) if checkpoint doesn't exist
    ///
    /// # Example Implementation (PostgreSQL)
    ///
    /// ```rust,ignore
    /// async fn get_tuple(&self, config: &CheckpointConfig) -> Result<Option<CheckpointTuple>> {
    ///     let row = if let Some(checkpoint_id) = &config.checkpoint_id {
    ///         // Get specific checkpoint
    ///         sqlx::query!(
    ///             "SELECT thread_id, checkpoint_id, checkpoint_ns, checkpoint, metadata, parent_checkpoint_id, created_at
    ///              FROM checkpoints
    ///              WHERE thread_id = $1 AND checkpoint_id = $2",
    ///             config.thread_id,
    ///             checkpoint_id
    ///         )
    ///         .fetch_optional(&self.pool)
    ///         .await?
    ///     } else {
    ///         // Get latest checkpoint for thread
    ///         sqlx::query!(
    ///             "SELECT thread_id, checkpoint_id, checkpoint_ns, checkpoint, metadata, parent_checkpoint_id, created_at
    ///              FROM checkpoints
    ///              WHERE thread_id = $1
    ///              ORDER BY created_at DESC
    ///              LIMIT 1",
    ///             config.thread_id
    ///         )
    ///         .fetch_optional(&self.pool)
    ///         .await?
    ///     };
    ///
    ///     let Some(row) = row else {
    ///         return Ok(None);
    ///     };
    ///
    ///     Ok(Some(CheckpointTuple {
    ///         config: CheckpointConfig {
    ///             thread_id: row.thread_id,
    ///             checkpoint_ns: row.checkpoint_ns,
    ///             checkpoint_id: Some(row.checkpoint_id),
    ///         },
    ///         checkpoint: serde_json::from_value(row.checkpoint)?,
    ///         metadata: serde_json::from_value(row.metadata)?,
    ///         parent_config: row.parent_checkpoint_id.map(|id| CheckpointConfig {
    ///             thread_id: row.thread_id.clone(),
    ///             checkpoint_ns: row.checkpoint_ns.clone(),
    ///             checkpoint_id: Some(id),
    ///         }),
    ///         pending_writes: None,
    ///     }))
    /// }
    /// ```
    ///
    /// # Usage Patterns
    ///
    /// **Resume from latest checkpoint:**
    /// ```rust,ignore
    /// let config = CheckpointConfig::builder()
    ///     .with_thread_id("conversation-123")
    ///     .build();
    /// let tuple = saver.get_tuple(&config).await?;
    /// // Returns most recent checkpoint for this thread
    /// ```
    ///
    /// **Load specific checkpoint:**
    /// ```rust,ignore
    /// let config = CheckpointConfig::builder()
    ///     .with_thread_id("conversation-123")
    ///     .with_checkpoint_id("checkpoint_456")
    ///     .build();
    /// let tuple = saver.get_tuple(&config).await?;
    /// // Returns exact checkpoint_456
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// - Called on every graph invocation with checkpoints
    /// - Optimize for single-row retrieval by primary key
    /// - Index on (thread_id, created_at DESC) for "latest" queries
    /// - Consider caching frequently accessed checkpoints
    ///
    /// # See Also
    ///
    /// - [`get`](Self::get) - Simplified version returning just the Checkpoint
    /// - [`put`](Self::put) - Store new checkpoints
    /// - [`list`](Self::list) - Query multiple checkpoints
    /// - [`CheckpointTuple`] - Return type structure
    async fn get_tuple(&self, config: &CheckpointConfig) -> Result<Option<CheckpointTuple>>;

    /// Query and stream checkpoints matching specified criteria.
    ///
    /// This method enables **checkpoint history traversal**, audit trails, and time-travel
    /// debugging by streaming checkpoints in reverse chronological order (newest first).
    ///
    /// # Arguments
    ///
    /// * `config` - Optional base configuration for filtering:
    ///   - If provided: Filter by `thread_id` and optionally `checkpoint_ns`
    ///   - If `None`: Return checkpoints from all threads
    /// * `filter` - Optional metadata field filters:
    ///   - Key-value pairs that must match in checkpoint metadata
    ///   - Example: `{"source": "human", "approved": true}`
    /// * `before` - Optional pagination cursor:
    ///   - Return only checkpoints created before this checkpoint
    ///   - Enables paginated history browsing
    /// * `limit` - Optional maximum number of results:
    ///   - Limits stream output for memory efficiency
    ///   - Use with `before` for pagination
    ///
    /// # Returns
    ///
    /// Async stream of [`CheckpointTuple`] in **reverse chronological order** (newest first).
    /// Stream may be empty if no checkpoints match the criteria.
    ///
    /// # Implementation Requirements
    ///
    /// Your implementation MUST:
    /// 1. **Order results** by timestamp DESC (newest first)
    /// 2. **Filter by thread_id** if config provided
    /// 3. **Apply metadata filters** if provided
    /// 4. **Support pagination** via `before` parameter
    /// 5. **Stream results** asynchronously (don't load all in memory)
    ///
    /// Your implementation SHOULD:
    /// - Use database indexes on (thread_id, created_at)
    /// - Handle large result sets efficiently
    /// - Support concurrent queries
    ///
    /// # Example Implementation (PostgreSQL with SQLx)
    ///
    /// ```rust,ignore
    /// async fn list(
    ///     &self,
    ///     config: Option<&CheckpointConfig>,
    ///     filter: Option<HashMap<String, Value>>,
    ///     before: Option<&CheckpointConfig>,
    ///     limit: Option<usize>,
    /// ) -> Result<CheckpointStream> {
    ///     // Build query dynamically based on filters
    ///     let mut query = String::from(
    ///         "SELECT thread_id, checkpoint_id, checkpoint_ns, checkpoint, metadata, parent_checkpoint_id, created_at
    ///          FROM checkpoints WHERE 1=1"
    ///     );
    ///     let mut bindings = vec![];
    ///
    ///     // Filter by thread_id
    ///     if let Some(cfg) = config {
    ///         query.push_str(&format!(" AND thread_id = ${}", bindings.len() + 1));
    ///         bindings.push(cfg.thread_id.clone());
    ///     }
    ///
    ///     // Filter by metadata fields
    ///     if let Some(filter_map) = filter {
    ///         for (key, value) in filter_map {
    ///             query.push_str(&format!(
    ///                 " AND metadata->>'{}' = ${}",
    ///                 key,
    ///                 bindings.len() + 1
    ///             ));
    ///             bindings.push(value.to_string());
    ///         }
    ///     }
    ///
    ///     // Pagination cursor
    ///     if let Some(before_cfg) = before {
    ///         query.push_str(&format!(
    ///             " AND created_at < (SELECT created_at FROM checkpoints WHERE checkpoint_id = ${})",
    ///             bindings.len() + 1
    ///         ));
    ///         bindings.push(before_cfg.checkpoint_id.clone().unwrap());
    ///     }
    ///
    ///     // Order and limit
    ///     query.push_str(" ORDER BY created_at DESC");
    ///     if let Some(lim) = limit {
    ///         query.push_str(&format!(" LIMIT {}", lim));
    ///     }
    ///
    ///     // Execute and stream results
    ///     let stream = sqlx::query(&query)
    ///         .fetch(&self.pool)
    ///         .map(|row| {
    ///             // Convert row to CheckpointTuple
    ///             // ... deserialize fields ...
    ///         });
    ///
    ///     Ok(Box::pin(stream))
    /// }
    /// ```
    ///
    /// # Usage Patterns
    ///
    /// **Get full thread history:**
    /// ```rust,ignore
    /// let config = CheckpointConfig::builder()
    ///     .with_thread_id("conversation-123")
    ///     .build();
    /// let mut stream = saver.list(Some(&config), None, None, None).await?;
    ///
    /// while let Some(tuple) = stream.next().await {
    ///     let tuple = tuple?;
    ///     println!("Checkpoint: {:?}", tuple.checkpoint.id);
    /// }
    /// ```
    ///
    /// **Find human-approved checkpoints:**
    /// ```rust,ignore
    /// let mut filter = HashMap::new();
    /// filter.insert("source".to_string(), json!("human"));
    /// filter.insert("approved".to_string(), json!(true));
    ///
    /// let mut stream = saver.list(Some(&config), Some(filter), None, Some(10)).await?;
    /// // Returns up to 10 human-approved checkpoints
    /// ```
    ///
    /// **Paginated history browsing:**
    /// ```rust,ignore
    /// // Page 1
    /// let mut page1 = saver.list(Some(&config), None, None, Some(10)).await?;
    /// let last_checkpoint = /* get last item from page1 */;
    ///
    /// // Page 2
    /// let mut page2 = saver.list(
    ///     Some(&config),
    ///     None,
    ///     Some(&last_checkpoint.config), // Use last as cursor
    ///     Some(10)
    /// ).await?;
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// - Results are streamed (not loaded into memory)
    /// - Database indexes critical for performance
    /// - Metadata filters may require JSONB indexing (PostgreSQL)
    /// - Consider view materialization for complex filters
    ///
    /// # Use Cases
    ///
    /// 1. **Audit Trail**: Review all state changes in execution
    /// 2. **Time-Travel Debugging**: Find checkpoint where issue occurred
    /// 3. **Analytics**: Analyze execution patterns over time
    /// 4. **Compliance**: Generate reports of human interventions
    /// 5. **Recovery**: Find last good checkpoint before failure
    ///
    /// # See Also
    ///
    /// - [`get_tuple`](Self::get_tuple) - Retrieve single checkpoint
    /// - [`put`](Self::put) - Store checkpoints
    /// - [`CheckpointStream`] - Returned stream type
    /// - [`CheckpointTuple`] - Stream item structure
    async fn list(
        &self,
        config: Option<&CheckpointConfig>,
        filter: Option<std::collections::HashMap<String, serde_json::Value>>,
        before: Option<&CheckpointConfig>,
        limit: Option<usize>,
    ) -> Result<CheckpointStream>;

    /// Store a checkpoint with its configuration and metadata.
    ///
    /// This is the **primary write operation** for checkpoint persistence. It's called automatically
    /// after each Pregel superstep to save execution state, enabling time-travel debugging,
    /// fault recovery, and human-in-the-loop workflows.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration identifying the checkpoint location:
    ///   - `thread_id` - Required execution thread identifier
    ///   - `checkpoint_ns` - Optional namespace for hierarchical storage
    ///   - `checkpoint_id` - Optional specific version ID (usually auto-generated)
    /// * `checkpoint` - Complete serialized graph state including:
    ///   - `channel_values` - Current values of all state channels
    ///   - `channel_versions` - Version numbers for change tracking
    ///   - `versions_seen` - What each node has processed
    ///   - Pending tasks and configuration
    /// * `metadata` - Execution metadata for this checkpoint:
    ///   - `step` - Superstep number in execution
    ///   - `source` - Source of this checkpoint ("loop", "update", etc.)
    ///   - `writes` - Channel writes that occurred in this step
    ///   - Custom fields via `extra` map
    /// * `new_versions` - Channel version updates for this checkpoint
    ///   - Maps channel name → version number
    ///   - Used to determine which nodes trigger next
    ///
    /// # Returns
    ///
    /// Updated [`CheckpointConfig`] with:
    /// - New `checkpoint_id` if auto-generated
    /// - Updated timestamp
    /// - Used for subsequent operations on this checkpoint
    ///
    /// # Implementation Requirements
    ///
    /// Your implementation MUST:
    /// 1. **Serialize** the checkpoint data (use `serde_json` or similar)
    /// 2. **Generate** a unique checkpoint_id if not provided
    /// 3. **Store** atomically to prevent partial writes
    /// 4. **Index** by thread_id and timestamp for efficient queries
    /// 5. **Return** the config with the actual stored checkpoint_id
    ///
    /// Your implementation SHOULD:
    /// - Support concurrent writes to different thread_ids
    /// - Handle large checkpoints efficiently (>1MB state)
    /// - Provide transactional guarantees if possible
    /// - Include error context for debugging
    ///
    /// # Example Implementation (PostgreSQL)
    ///
    /// ```rust,ignore
    /// async fn put(
    ///     &self,
    ///     config: &CheckpointConfig,
    ///     checkpoint: Checkpoint,
    ///     metadata: CheckpointMetadata,
    ///     new_versions: ChannelVersions,
    /// ) -> Result<CheckpointConfig> {
    ///     // 1. Generate ID if needed
    ///     let checkpoint_id = config.checkpoint_id.clone()
    ///         .unwrap_or_else(|| format!("{}_{}", config.thread_id, chrono::Utc::now().timestamp_millis()));
    ///
    ///     // 2. Serialize data
    ///     let checkpoint_json = serde_json::to_value(&checkpoint)?;
    ///     let metadata_json = serde_json::to_value(&metadata)?;
    ///     let versions_json = serde_json::to_value(&new_versions)?;
    ///
    ///     // 3. Store in database
    ///     sqlx::query!(
    ///         "INSERT INTO checkpoints (thread_id, checkpoint_id, checkpoint_ns, checkpoint, metadata, versions, created_at)
    ///          VALUES ($1, $2, $3, $4, $5, $6, NOW())
    ///          ON CONFLICT (thread_id, checkpoint_id) DO UPDATE SET
    ///            checkpoint = EXCLUDED.checkpoint,
    ///            metadata = EXCLUDED.metadata,
    ///            versions = EXCLUDED.versions",
    ///         config.thread_id,
    ///         checkpoint_id,
    ///         config.checkpoint_ns,
    ///         checkpoint_json,
    ///         metadata_json,
    ///         versions_json,
    ///     )
    ///     .execute(&self.pool)
    ///     .await?;
    ///
    ///     // 4. Return updated config
    ///     Ok(CheckpointConfig {
    ///         thread_id: config.thread_id.clone(),
    ///         checkpoint_ns: config.checkpoint_ns.clone(),
    ///         checkpoint_id: Some(checkpoint_id),
    ///     })
    /// }
    /// ```
    ///
    /// # Call Frequency
    ///
    /// Called once per superstep during graph execution:
    /// - 10-step execution = 10 put() calls
    /// - Long-running agent = hundreds/thousands of calls
    /// - Design for high throughput
    ///
    /// # Error Handling
    ///
    /// Return `Err` if:
    /// - Serialization fails
    /// - Storage backend is unavailable
    /// - Disk/memory is full
    /// - Permissions are insufficient
    ///
    /// The graph execution will halt on storage errors.
    ///
    /// # See Also
    ///
    /// - [`get_tuple`](Self::get_tuple) - Retrieve stored checkpoints
    /// - [`put_writes`](Self::put_writes) - Store intermediate writes
    /// - [`list`](Self::list) - Query checkpoint history
    /// - [`Checkpoint`] - Structure of saved state
    /// - [`CheckpointMetadata`] - Metadata format
    async fn put(
        &self,
        config: &CheckpointConfig,
        checkpoint: Checkpoint,
        metadata: CheckpointMetadata,
        new_versions: ChannelVersions,
    ) -> Result<CheckpointConfig>;

    /// Store intermediate writes linked to a checkpoint.
    ///
    /// This method stores **pending writes** from task execution before they're applied
    /// to the checkpoint. It enables write buffering, transactional semantics, and
    /// detailed audit trails of state changes.
    ///
    /// # Purpose
    ///
    /// Writes are stored separately from checkpoints to support:
    /// - **Streaming writes** - Emit state changes as they occur
    /// - **Transactional rollback** - Discard writes if task fails
    /// - **Audit trail** - Track which task produced which writes
    /// - **Debugging** - Inspect writes before application
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration of the related checkpoint
    ///   - Identifies the execution context for these writes
    /// * `writes` - List of pending channel writes: `(channel_name, value)`
    ///   - Each tuple represents a write to a specific channel
    ///   - Values are JSON-serialized channel updates
    /// * `task_id` - Unique identifier for the task producing these writes
    ///   - Format: `"{checkpoint_id}:{node_name}"`
    ///   - Used for tracking write provenance
    ///
    /// # Returns
    ///
    /// `Ok(())` on successful storage, `Err` if storage fails.
    ///
    /// # Implementation Requirements
    ///
    /// Your implementation MUST:
    /// 1. **Link writes to checkpoint** via config (thread_id, checkpoint_id)
    /// 2. **Store task_id** for write attribution
    /// 3. **Preserve order** of writes within a task
    /// 4. **Handle duplicates** gracefully (idempotent if possible)
    ///
    /// Your implementation MAY:
    /// - Batch writes for performance
    /// - Store writes in separate table from checkpoints
    /// - Use time-to-live (TTL) to auto-expire old writes
    ///
    /// # Example Implementation (SQLite)
    ///
    /// ```rust,ignore
    /// async fn put_writes(
    ///     &self,
    ///     config: &CheckpointConfig,
    ///     writes: Vec<(String, serde_json::Value)>,
    ///     task_id: String,
    /// ) -> Result<()> {
    ///     let mut conn = self.pool.acquire().await?;
    ///
    ///     // Begin transaction for atomic multi-write
    ///     let tx = conn.begin().await?;
    ///
    ///     for (idx, (channel, value)) in writes.into_iter().enumerate() {
    ///         sqlx::query!(
    ///             "INSERT INTO checkpoint_writes
    ///              (thread_id, checkpoint_id, task_id, channel, value, idx, created_at)
    ///              VALUES ($1, $2, $3, $4, $5, $6, datetime('now'))",
    ///             config.thread_id,
    ///             config.checkpoint_id,
    ///             task_id,
    ///             channel,
    ///             value,
    ///             idx as i64,
    ///         )
    ///         .execute(&mut tx)
    ///         .await?;
    ///     }
    ///
    ///     tx.commit().await?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Call Pattern
    ///
    /// Called during task execution, before checkpoint persistence:
    ///
    /// ```text
    /// 1. Task executes
    /// 2. Task produces writes
    /// 3. put_writes() called      ← You are here
    /// 4. Writes applied to channels
    /// 5. put() saves checkpoint
    /// ```
    ///
    /// # Performance Notes
    ///
    /// - Called frequently (once per task with writes)
    /// - Optimize for write throughput
    /// - Consider batch inserts
    /// - Async I/O is critical
    ///
    /// # Error Handling
    ///
    /// Return `Err` if storage fails. The task execution may be retried.
    ///
    /// # See Also
    ///
    /// - [`put`](Self::put) - Main checkpoint storage
    /// - [`get_tuple`](Self::get_tuple) - Retrieve checkpoints (doesn't include writes)
    async fn put_writes(
        &self,
        config: &CheckpointConfig,
        writes: Vec<(String, serde_json::Value)>,
        task_id: String,
    ) -> Result<()>;

    /// Delete all checkpoints and writes associated with a specific thread ID
    ///
    /// # Arguments
    ///
    /// * `thread_id` - The thread ID whose checkpoints should be deleted
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    async fn delete_thread(&self, thread_id: &str) -> Result<()> {
        let _ = thread_id;
        Ok(())
    }
}
