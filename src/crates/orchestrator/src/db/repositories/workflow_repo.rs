//! Workflow repository for database operations

use crate::db::connection::DatabasePool;
use crate::db::models::Workflow;
use chrono::Utc;

/// Workflow repository for managing workflow database operations
pub struct WorkflowRepository;

impl WorkflowRepository {
    /// Create a new workflow
    pub async fn create(
        pool: &DatabasePool,
        id: String,
        name: String,
        definition: String,
    ) -> Result<Workflow, sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query_as::<_, Workflow>(
            "INSERT INTO workflows (id, name, definition, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING *"
        )
        .bind(&id)
        .bind(&name)
        .bind(&definition)
        .bind("draft")
        .bind(&now)
        .bind(&now)
        .fetch_one(pool)
        .await
    }

    /// Get a workflow by ID
    pub async fn get_by_id(pool: &DatabasePool, id: &str) -> Result<Option<Workflow>, sqlx::Error> {
        sqlx::query_as::<_, Workflow>("SELECT * FROM workflows WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Get a workflow by name
    pub async fn find_by_name(
        pool: &DatabasePool,
        name: &str,
    ) -> Result<Option<Workflow>, sqlx::Error> {
        sqlx::query_as::<_, Workflow>("SELECT * FROM workflows WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await
    }

    /// List all workflows
    pub async fn list(pool: &DatabasePool) -> Result<Vec<Workflow>, sqlx::Error> {
        sqlx::query_as::<_, Workflow>("SELECT * FROM workflows ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    }

    /// List workflows by status
    pub async fn list_by_status(
        pool: &DatabasePool,
        status: &str,
    ) -> Result<Vec<Workflow>, sqlx::Error> {
        sqlx::query_as::<_, Workflow>(
            "SELECT * FROM workflows WHERE status = ? ORDER BY created_at DESC"
        )
        .bind(status)
        .fetch_all(pool)
        .await
    }

    /// Update workflow status
    pub async fn update_status(
        pool: &DatabasePool,
        id: &str,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE workflows SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Update workflow definition
    pub async fn update_definition(
        pool: &DatabasePool,
        id: &str,
        definition: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE workflows SET definition = ?, updated_at = ? WHERE id = ?")
            .bind(definition)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Update workflow description
    pub async fn update_description(
        pool: &DatabasePool,
        id: &str,
        description: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE workflows SET description = ?, updated_at = ? WHERE id = ?")
            .bind(description)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete a workflow
    pub async fn delete(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM workflows WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Count total workflows
    pub async fn count(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workflows")
            .fetch_one(pool)
            .await?;

        Ok(result.0)
    }

    /// Count workflows by status
    pub async fn count_by_status(
        pool: &DatabasePool,
        status: &str,
    ) -> Result<i64, sqlx::Error> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM workflows WHERE status = ?")
                .bind(status)
                .fetch_one(pool)
                .await?;

        Ok(result.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> sqlx::sqlite::SqlitePool {
        let pool = sqlx::sqlite::SqlitePool::connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE workflows (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                definition TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                CHECK (status IN ('draft', 'active', 'archived', 'paused'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_workflow() {
        let pool = setup_db().await;

        let workflow = WorkflowRepository::create(
            &pool,
            "workflow-1".to_string(),
            "Test Workflow".to_string(),
            r#"{"steps": []}"#.to_string(),
        )
        .await
        .unwrap();

        assert_eq!(workflow.id, "workflow-1");
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.status, "draft");
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let pool = setup_db().await;

        WorkflowRepository::create(
            &pool,
            "workflow-1".to_string(),
            "Test Workflow".to_string(),
            r#"{"steps": []}"#.to_string(),
        )
        .await
        .unwrap();

        let fetched = WorkflowRepository::get_by_id(&pool, "workflow-1")
            .await
            .unwrap();

        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "Test Workflow");
    }

    #[tokio::test]
    async fn test_find_by_name() {
        let pool = setup_db().await;

        WorkflowRepository::create(
            &pool,
            "workflow-1".to_string(),
            "Unique Workflow".to_string(),
            r#"{"steps": []}"#.to_string(),
        )
        .await
        .unwrap();

        let fetched = WorkflowRepository::find_by_name(&pool, "Unique Workflow")
            .await
            .unwrap();

        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, "workflow-1");
    }

    #[tokio::test]
    async fn test_update_status() {
        let pool = setup_db().await;

        WorkflowRepository::create(
            &pool,
            "workflow-1".to_string(),
            "Test Workflow".to_string(),
            r#"{"steps": []}"#.to_string(),
        )
        .await
        .unwrap();

        WorkflowRepository::update_status(&pool, "workflow-1", "active")
            .await
            .unwrap();

        let workflow = WorkflowRepository::get_by_id(&pool, "workflow-1")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(workflow.status, "active");
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_db().await;

        WorkflowRepository::create(
            &pool,
            "workflow-1".to_string(),
            "Test Workflow".to_string(),
            r#"{"steps": []}"#.to_string(),
        )
        .await
        .unwrap();

        WorkflowRepository::delete(&pool, "workflow-1")
            .await
            .unwrap();

        let fetched = WorkflowRepository::get_by_id(&pool, "workflow-1")
            .await
            .unwrap();

        assert!(fetched.is_none());
    }
}
