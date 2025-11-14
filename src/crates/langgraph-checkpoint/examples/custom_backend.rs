//! Example of implementing a custom checkpoint backend
//!
//! This example demonstrates how downstream projects can implement
//! the CheckpointSaver trait for their own database backends.
//!
//! NOTE: This is a documentation example showing the pattern.
//! For a working example, see the InMemoryCheckpointSaver implementation.

use async_trait::async_trait;
use futures::stream::Stream;
use langgraph_checkpoint::{
    Checkpoint, CheckpointConfig, CheckpointError, CheckpointMetadata, CheckpointSaver,
    CheckpointTuple, Result,
};
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;

/// Example: PostgreSQL checkpoint backend (conceptual)
///
/// In a real implementation with PostgreSQL using `sqlx`:
///
/// ```rust,ignore
/// use sqlx::PgPool;
/// use async_trait::async_trait;
/// use langgraph_checkpoint::*;
///
/// pub struct PostgresCheckpointSaver {
///     pool: PgPool,
/// }
///
/// impl PostgresCheckpointSaver {
///     pub async fn new(database_url: &str) -> Result<Self> {
///         let pool = PgPool::connect(database_url).await?;
///         Ok(Self { pool })
///     }
/// }
///
/// #[async_trait]
/// impl CheckpointSaver for PostgresCheckpointSaver {
///     async fn get_tuple(
///         &self,
///         config: &CheckpointConfig,
///     ) -> Result<Option<CheckpointTuple>> {
///         let thread_id = config.thread_id.as_ref()
///             .ok_or(CheckpointError::NotFound)?;
///
///         // Query PostgreSQL
///         let row = sqlx::query!(
///             r#"
///             SELECT checkpoint_data, metadata, created_at
///             FROM checkpoints
///             WHERE thread_id = $1
///             ORDER BY created_at DESC
///             LIMIT 1
///             "#,
///             thread_id
///         )
///         .fetch_optional(&self.pool)
///         .await?;
///
///         if let Some(row) = row {
///             let checkpoint: Checkpoint = serde_json::from_value(row.checkpoint_data)?;
///             let metadata: CheckpointMetadata = serde_json::from_value(row.metadata)?;
///
///             Ok(Some(CheckpointTuple {
///                 config: config.clone(),
///                 checkpoint,
///                 metadata,
///                 parent_config: None,
///             }))
///         } else {
///             Ok(None)
///         }
///     }
///
///     async fn put(
///         &self,
///         config: &CheckpointConfig,
///         checkpoint: Checkpoint,
///         metadata: CheckpointMetadata,
///         new_versions: HashMap<String, ChannelVersion>,
///     ) -> Result<CheckpointConfig> {
///         let thread_id = config.thread_id.as_ref()
///             .ok_or(CheckpointError::NotFound)?;
///
///         let checkpoint_id = uuid::Uuid::new_v4().to_string();
///         let checkpoint_json = serde_json::to_value(&checkpoint)?;
///         let metadata_json = serde_json::to_value(&metadata)?;
///
///         // Insert into PostgreSQL
///         sqlx::query!(
///             r#"
///             INSERT INTO checkpoints (thread_id, checkpoint_id, checkpoint_data, metadata)
///             VALUES ($1, $2, $3, $4)
///             "#,
///             thread_id,
///             checkpoint_id,
///             checkpoint_json,
///             metadata_json
///         )
///         .execute(&self.pool)
///         .await?;
///
///         // Return updated config with checkpoint ID
///         Ok(config.clone().with_checkpoint_id(Some(checkpoint_id)))
///     }
///
///     async fn list(
///         &self,
///         config: Option<&CheckpointConfig>,
///         filter: Option<HashMap<String, Value>>,
///         before: Option<&CheckpointConfig>,
///         limit: Option<usize>,
///     ) -> Result<Pin<Box<dyn Stream<Item = Result<CheckpointTuple>> + Send>>> {
///         // Return a stream of checkpoints from PostgreSQL
///         // Implementation would use sqlx::query_as with streaming
///         todo!("Implement streaming query")
///     }
///
///     async fn put_writes(
///         &self,
///         config: &CheckpointConfig,
///         writes: Vec<(String, Value)>,
///         task_id: String,
///     ) -> Result<()> {
///         // Store pending writes in database
///         // Useful for tracking incomplete operations
///         todo!("Implement write storage")
///     }
/// }
/// ```
///
/// ## SQLite Example
///
/// ```rust,ignore
/// use sqlx::SqlitePool;
///
/// pub struct SqliteCheckpointSaver {
///     pool: SqlitePool,
/// }
///
/// impl SqliteCheckpointSaver {
///     pub async fn new(database_path: &str) -> Result<Self> {
///         let pool = SqlitePool::connect(database_path).await?;
///         Ok(Self { pool })
///     }
/// }
///
/// // Implement CheckpointSaver similar to PostgreSQL
/// // SQLite uses JSON instead of JSONB, but API is similar
/// ```
///
/// ## Redis Example
///
/// ```rust,ignore
/// use redis::aio::Connection;
///
/// pub struct RedisCheckpointSaver {
///     client: redis::Client,
/// }
///
/// impl RedisCheckpointSaver {
///     pub async fn new(redis_url: &str) -> Result<Self> {
///         let client = redis::Client::open(redis_url)?;
///         Ok(Self { client })
///     }
/// }
///
/// #[async_trait]
/// impl CheckpointSaver for RedisCheckpointSaver {
///     async fn get_tuple(&self, config: &CheckpointConfig) -> Result<Option<CheckpointTuple>> {
///         let mut conn = self.client.get_async_connection().await?;
///         let key = format!("checkpoint:{}", config.thread_id.as_ref().unwrap());
///
///         let data: Option<String> = redis::cmd("GET")
///             .arg(&key)
///             .query_async(&mut conn)
///             .await?;
///
///         if let Some(json) = data {
///             let tuple: CheckpointTuple = serde_json::from_str(&json)?;
///             Ok(Some(tuple))
///         } else {
///             Ok(None)
///         }
///     }
///
///     // ... implement other methods
/// }
/// ```

fn main() {
    println!("=== Custom Checkpoint Backend Examples ===\n");
    println!("This file demonstrates how to implement custom checkpoint backends.");
    println!("\nSupported databases:");
    println!("  • PostgreSQL - Full ACID, JSONB support, connection pooling");
    println!("  • SQLite - Lightweight, file-based, good for dev/small deployments");
    println!("  • Redis - In-memory, very fast, good for caching");
    println!("  • MongoDB - Document store, flexible schema");
    println!("  • DynamoDB - AWS managed, serverless");
    println!("\nKey considerations:");
    println!("  1. Thread safety - Use connection pooling");
    println!("  2. Serialization - JSONB (Postgres) or JSON (others)");
    println!("  3. Indexing - Index on thread_id and created_at");
    println!("  4. Transactions - Ensure atomic writes");
    println!("  5. Migrations - Handle schema evolution");
    println!("\nFor a working in-memory implementation, see:");
    println!("  crates/langgraph-checkpoint/src/memory.rs");
}
