//! Bug repository for database operations

use crate::db::connection::DatabasePool;
use crate::db::models::Bug;
use chrono::Utc;

/// Bug repository for managing bug database operations
pub struct BugRepository;

impl BugRepository {
    /// Create a new bug in the database
    pub async fn create(
        pool: &DatabasePool,
        id: String,
        title: String,
        severity: String,
        description: Option<String>,
        task_id: Option<String>,
        workflow_id: Option<String>,
        reporter: Option<String>,
    ) -> Result<Bug, sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query_as::<_, Bug>(
            "INSERT INTO bugs (id, title, severity, description, task_id, workflow_id, reporter, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 'open', ?, ?)
             RETURNING *"
        )
        .bind(&id)
        .bind(&title)
        .bind(&severity)
        .bind(&description)
        .bind(&task_id)
        .bind(&workflow_id)
        .bind(&reporter)
        .bind(&now)
        .bind(&now)
        .fetch_one(pool)
        .await
    }

    /// Get a bug by ID
    pub async fn get_by_id(pool: &DatabasePool, id: &str) -> Result<Option<Bug>, sqlx::Error> {
        sqlx::query_as::<_, Bug>("SELECT * FROM bugs WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// List all bugs
    pub async fn list(pool: &DatabasePool) -> Result<Vec<Bug>, sqlx::Error> {
        sqlx::query_as::<_, Bug>("SELECT * FROM bugs ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    }

    /// List bugs by status
    pub async fn list_by_status(pool: &DatabasePool, status: &str) -> Result<Vec<Bug>, sqlx::Error> {
        sqlx::query_as::<_, Bug>("SELECT * FROM bugs WHERE status = ? ORDER BY created_at DESC")
            .bind(status)
            .fetch_all(pool)
            .await
    }

    /// List bugs by severity
    pub async fn list_by_severity(pool: &DatabasePool, severity: &str) -> Result<Vec<Bug>, sqlx::Error> {
        sqlx::query_as::<_, Bug>("SELECT * FROM bugs WHERE severity = ? ORDER BY created_at DESC")
            .bind(severity)
            .fetch_all(pool)
            .await
    }

    /// List bugs by task
    pub async fn list_by_task(pool: &DatabasePool, task_id: &str) -> Result<Vec<Bug>, sqlx::Error> {
        sqlx::query_as::<_, Bug>("SELECT * FROM bugs WHERE task_id = ? ORDER BY created_at DESC")
            .bind(task_id)
            .fetch_all(pool)
            .await
    }

    /// Update bug status
    pub async fn update_status(pool: &DatabasePool, id: &str, status: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        let resolved_at = if status == "resolved" || status == "closed" {
            Some(now.clone())
        } else {
            None
        };

        sqlx::query("UPDATE bugs SET status = ?, updated_at = ?, resolved_at = COALESCE(?, resolved_at) WHERE id = ?")
            .bind(status)
            .bind(&now)
            .bind(&resolved_at)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Update bug assignee
    pub async fn update_assignee(pool: &DatabasePool, id: &str, assignee: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE bugs SET assignee = ?, updated_at = ? WHERE id = ?")
            .bind(assignee)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete a bug
    pub async fn delete(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM bugs WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Count bugs by status
    pub async fn count_by_status(pool: &DatabasePool, status: &str) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bugs WHERE status = ?")
            .bind(status)
            .fetch_one(pool)
            .await?;
        Ok(result.0)
    }

    /// Count total bugs
    pub async fn count(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bugs")
            .fetch_one(pool)
            .await?;
        Ok(result.0)
    }
}
