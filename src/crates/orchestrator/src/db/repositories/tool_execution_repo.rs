//! ToolExecution repository for database operations

use crate::db::connection::DatabasePool;
use crate::db::models::ToolExecution;
use chrono::Utc;

/// ToolExecution repository for managing tool execution database operations
pub struct ToolExecutionRepository;

impl ToolExecutionRepository {
    /// Create a new tool execution record
    pub async fn create(
        pool: &DatabasePool,
        id: String,
        task_id: String,
        tool_name: String,
        arguments: String,
    ) -> Result<ToolExecution, sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query_as::<_, ToolExecution>(
            "INSERT INTO tool_executions (id, task_id, tool_name, arguments, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING *"
        )
        .bind(&id)
        .bind(&task_id)
        .bind(&tool_name)
        .bind(&arguments)
        .bind("pending")
        .bind(&now)
        .fetch_one(pool)
        .await
    }

    /// Get a tool execution by ID
    pub async fn get_by_id(
        pool: &DatabasePool,
        id: &str,
    ) -> Result<Option<ToolExecution>, sqlx::Error> {
        sqlx::query_as::<_, ToolExecution>("SELECT * FROM tool_executions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// List executions for a specific task
    pub async fn list_by_task(
        pool: &DatabasePool,
        task_id: &str,
    ) -> Result<Vec<ToolExecution>, sqlx::Error> {
        sqlx::query_as::<_, ToolExecution>(
            "SELECT * FROM tool_executions WHERE task_id = ? ORDER BY created_at DESC"
        )
        .bind(task_id)
        .fetch_all(pool)
        .await
    }

    /// List executions for a specific tool
    pub async fn list_by_tool(
        pool: &DatabasePool,
        tool_name: &str,
    ) -> Result<Vec<ToolExecution>, sqlx::Error> {
        sqlx::query_as::<_, ToolExecution>(
            "SELECT * FROM tool_executions WHERE tool_name = ? ORDER BY created_at DESC"
        )
        .bind(tool_name)
        .fetch_all(pool)
        .await
    }

    /// List executions by status
    pub async fn list_by_status(
        pool: &DatabasePool,
        status: &str,
    ) -> Result<Vec<ToolExecution>, sqlx::Error> {
        sqlx::query_as::<_, ToolExecution>(
            "SELECT * FROM tool_executions WHERE status = ? ORDER BY created_at DESC"
        )
        .bind(status)
        .fetch_all(pool)
        .await
    }

    /// Update execution status
    pub async fn update_status(
        pool: &DatabasePool,
        id: &str,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE tool_executions SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Mark execution as completed with output and duration
    pub async fn mark_completed(
        pool: &DatabasePool,
        id: &str,
        output: &str,
        duration_ms: i32,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE tool_executions SET status = ?, output = ?, duration_ms = ?, completed_at = ? WHERE id = ?"
        )
        .bind("completed")
        .bind(output)
        .bind(duration_ms)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Mark execution as failed with error and duration
    pub async fn mark_failed(
        pool: &DatabasePool,
        id: &str,
        error: &str,
        duration_ms: i32,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE tool_executions SET status = ?, error = ?, duration_ms = ?, completed_at = ? WHERE id = ?"
        )
        .bind("failed")
        .bind(error)
        .bind(duration_ms)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Mark execution as timeout with duration
    pub async fn mark_timeout(
        pool: &DatabasePool,
        id: &str,
        duration_ms: i32,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE tool_executions SET status = ?, duration_ms = ?, completed_at = ? WHERE id = ?"
        )
        .bind("timeout")
        .bind(duration_ms)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Mark execution as running
    pub async fn mark_running(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE tool_executions SET status = ? WHERE id = ?")
            .bind("running")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete a tool execution
    pub async fn delete(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tool_executions WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete all executions for a task
    pub async fn delete_by_task(pool: &DatabasePool, task_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tool_executions WHERE task_id = ?")
            .bind(task_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Count total executions
    pub async fn count(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tool_executions")
            .fetch_one(pool)
            .await?;

        Ok(result.0)
    }

    /// Count executions by status
    pub async fn count_by_status(
        pool: &DatabasePool,
        status: &str,
    ) -> Result<i64, sqlx::Error> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tool_executions WHERE status = ?")
                .bind(status)
                .fetch_one(pool)
                .await?;

        Ok(result.0)
    }

    /// Count executions for a specific task
    pub async fn count_by_task(
        pool: &DatabasePool,
        task_id: &str,
    ) -> Result<i64, sqlx::Error> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tool_executions WHERE task_id = ?")
                .bind(task_id)
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
            "CREATE TABLE tasks (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                task_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                config TEXT,
                metadata TEXT,
                workspace_path TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT,
                error TEXT
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE tool_executions (
                id TEXT PRIMARY KEY NOT NULL,
                task_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                arguments TEXT NOT NULL,
                output TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                error TEXT,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                duration_ms INTEGER,
                CHECK (status IN ('pending', 'running', 'completed', 'failed', 'timeout'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_execution() {
        let pool = setup_db().await;

        // Create task
        sqlx::query(
            "INSERT INTO tasks (id, title, task_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind("task-1")
        .bind("Task 1")
        .bind("execution")
        .bind("pending")
        .bind("2025-11-10T00:00:00Z")
        .bind("2025-11-10T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        let exec = ToolExecutionRepository::create(
            &pool,
            "exec-1".to_string(),
            "task-1".to_string(),
            "bash".to_string(),
            r#"{"script": "echo hello"}"#.to_string(),
        )
        .await
        .unwrap();

        assert_eq!(exec.id, "exec-1");
        assert_eq!(exec.task_id, "task-1");
        assert_eq!(exec.tool_name, "bash");
        assert_eq!(exec.status, "pending");
    }

    #[tokio::test]
    async fn test_list_by_task() {
        let pool = setup_db().await;

        sqlx::query(
            "INSERT INTO tasks (id, title, task_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind("task-1")
        .bind("Task 1")
        .bind("execution")
        .bind("pending")
        .bind("2025-11-10T00:00:00Z")
        .bind("2025-11-10T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        for i in 1..=3 {
            ToolExecutionRepository::create(
                &pool,
                format!("exec-{}", i),
                "task-1".to_string(),
                "bash".to_string(),
                r#"{"script": "echo hello"}"#.to_string(),
            )
            .await
            .unwrap();
        }

        let executions = ToolExecutionRepository::list_by_task(&pool, "task-1")
            .await
            .unwrap();

        assert_eq!(executions.len(), 3);
    }

    #[tokio::test]
    async fn test_mark_completed() {
        let pool = setup_db().await;

        sqlx::query(
            "INSERT INTO tasks (id, title, task_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind("task-1")
        .bind("Task 1")
        .bind("execution")
        .bind("pending")
        .bind("2025-11-10T00:00:00Z")
        .bind("2025-11-10T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        ToolExecutionRepository::create(
            &pool,
            "exec-1".to_string(),
            "task-1".to_string(),
            "bash".to_string(),
            r#"{"script": "echo hello"}"#.to_string(),
        )
        .await
        .unwrap();

        ToolExecutionRepository::mark_completed(&pool, "exec-1", "hello", 100)
            .await
            .unwrap();

        let exec = ToolExecutionRepository::get_by_id(&pool, "exec-1")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(exec.status, "completed");
        assert_eq!(exec.output, Some("hello".to_string()));
        assert_eq!(exec.duration_ms, Some(100));
    }

    #[tokio::test]
    async fn test_mark_failed() {
        let pool = setup_db().await;

        sqlx::query(
            "INSERT INTO tasks (id, title, task_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind("task-1")
        .bind("Task 1")
        .bind("execution")
        .bind("pending")
        .bind("2025-11-10T00:00:00Z")
        .bind("2025-11-10T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        ToolExecutionRepository::create(
            &pool,
            "exec-1".to_string(),
            "task-1".to_string(),
            "bash".to_string(),
            r#"{"script": "false"}"#.to_string(),
        )
        .await
        .unwrap();

        ToolExecutionRepository::mark_failed(&pool, "exec-1", "Exit code 1", 50)
            .await
            .unwrap();

        let exec = ToolExecutionRepository::get_by_id(&pool, "exec-1")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(exec.status, "failed");
        assert_eq!(exec.error, Some("Exit code 1".to_string()));
    }

    #[tokio::test]
    async fn test_count_by_status() {
        let pool = setup_db().await;

        sqlx::query(
            "INSERT INTO tasks (id, title, task_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind("task-1")
        .bind("Task 1")
        .bind("execution")
        .bind("pending")
        .bind("2025-11-10T00:00:00Z")
        .bind("2025-11-10T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        ToolExecutionRepository::create(
            &pool,
            "exec-1".to_string(),
            "task-1".to_string(),
            "bash".to_string(),
            "{}".to_string(),
        )
        .await
        .unwrap();

        ToolExecutionRepository::create(
            &pool,
            "exec-2".to_string(),
            "task-1".to_string(),
            "bash".to_string(),
            "{}".to_string(),
        )
        .await
        .unwrap();

        ToolExecutionRepository::mark_completed(&pool, "exec-1", "output", 100)
            .await
            .unwrap();

        let completed_count = ToolExecutionRepository::count_by_status(&pool, "completed")
            .await
            .unwrap();
        let pending_count = ToolExecutionRepository::count_by_status(&pool, "pending")
            .await
            .unwrap();

        assert_eq!(completed_count, 1);
        assert_eq!(pending_count, 1);
    }
}
