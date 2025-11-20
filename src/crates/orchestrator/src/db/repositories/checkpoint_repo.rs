//! Checkpoint repository for database operations

use crate::db::connection::DatabasePool;
use crate::db::models::Checkpoint;
use chrono::Utc;

/// Checkpoint repository for managing execution checkpoint database operations
pub struct CheckpointRepository;

impl CheckpointRepository {
    /// Create a new checkpoint
    pub async fn create(
        pool: &DatabasePool,
        id: String,
        execution_id: String,
        workflow_id: String,
        state: String,
        node_id: Option<String>,
        superstep: i32,
        parent_checkpoint_id: Option<String>,
        metadata: Option<String>,
    ) -> Result<Checkpoint, sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query_as::<_, Checkpoint>(
            "INSERT INTO checkpoints (id, execution_id, workflow_id, state, node_id, superstep, parent_checkpoint_id, metadata, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING *"
        )
        .bind(&id)
        .bind(&execution_id)
        .bind(&workflow_id)
        .bind(&state)
        .bind(&node_id)
        .bind(&superstep)
        .bind(&parent_checkpoint_id)
        .bind(&metadata)
        .bind(&now)
        .fetch_one(pool)
        .await
    }

    /// Get a checkpoint by ID
    pub async fn get_by_id(pool: &DatabasePool, id: &str) -> Result<Option<Checkpoint>, sqlx::Error> {
        sqlx::query_as::<_, Checkpoint>("SELECT * FROM checkpoints WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// List all checkpoints
    pub async fn list(pool: &DatabasePool) -> Result<Vec<Checkpoint>, sqlx::Error> {
        sqlx::query_as::<_, Checkpoint>("SELECT * FROM checkpoints ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    }

    /// List checkpoints by execution
    pub async fn list_by_execution(pool: &DatabasePool, execution_id: &str) -> Result<Vec<Checkpoint>, sqlx::Error> {
        sqlx::query_as::<_, Checkpoint>(
            "SELECT * FROM checkpoints WHERE execution_id = ? ORDER BY superstep ASC, created_at ASC"
        )
        .bind(execution_id)
        .fetch_all(pool)
        .await
    }

    /// List checkpoints by workflow
    pub async fn list_by_workflow(pool: &DatabasePool, workflow_id: &str) -> Result<Vec<Checkpoint>, sqlx::Error> {
        sqlx::query_as::<_, Checkpoint>(
            "SELECT * FROM checkpoints WHERE workflow_id = ? ORDER BY created_at DESC"
        )
        .bind(workflow_id)
        .fetch_all(pool)
        .await
    }

    /// Get latest checkpoint for an execution
    pub async fn get_latest_for_execution(pool: &DatabasePool, execution_id: &str) -> Result<Option<Checkpoint>, sqlx::Error> {
        sqlx::query_as::<_, Checkpoint>(
            "SELECT * FROM checkpoints WHERE execution_id = ? ORDER BY superstep DESC, created_at DESC LIMIT 1"
        )
        .bind(execution_id)
        .fetch_optional(pool)
        .await
    }

    /// Delete a checkpoint
    pub async fn delete(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM checkpoints WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete checkpoints by execution
    pub async fn delete_by_execution(pool: &DatabasePool, execution_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM checkpoints WHERE execution_id = ?")
            .bind(execution_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Count checkpoints
    pub async fn count(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM checkpoints")
            .fetch_one(pool)
            .await?;
        Ok(result.0)
    }

    /// Count checkpoints by execution
    pub async fn count_by_execution(pool: &DatabasePool, execution_id: &str) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM checkpoints WHERE execution_id = ?")
            .bind(execution_id)
            .fetch_one(pool)
            .await?;
        Ok(result.0)
    }
}
