//! Task repository for database operations

use crate::db::connection::DatabasePool;
use crate::db::models::Task;
use chrono::Utc;

/// Task repository for managing task database operations
pub struct TaskRepository;

impl TaskRepository {
    /// Create a new task in the database
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Unique task identifier
    /// * `title` - Task title
    /// * `task_type` - Type of task
    /// * `workspace_path` - Execution workspace path
    ///
    /// # Returns
    /// Created task or database error
    pub async fn create(
        pool: &DatabasePool,
        id: String,
        title: String,
        task_type: String,
        workspace_path: String,
    ) -> Result<Task, sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query_as::<_, Task>(
            "INSERT INTO tasks (id, title, task_type, status, workspace_path, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             RETURNING *"
        )
        .bind(&id)
        .bind(&title)
        .bind(&task_type)
        .bind("pending")
        .bind(&workspace_path)
        .bind(&now)
        .bind(&now)
        .fetch_one(pool)
        .await
    }

    /// Get a task by ID
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Task identifier
    ///
    /// # Returns
    /// Task if found, None if not found, or database error
    pub async fn get_by_id(pool: &DatabasePool, id: &str) -> Result<Option<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Get all tasks
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// Vector of all tasks or database error
    pub async fn list(pool: &DatabasePool) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>("SELECT * FROM tasks ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    }

    /// List tasks by status
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `status` - Task status to filter by
    ///
    /// # Returns
    /// Vector of tasks with the specified status or database error
    pub async fn list_by_status(
        pool: &DatabasePool,
        status: &str,
    ) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE status = ? ORDER BY created_at DESC"
        )
        .bind(status)
        .fetch_all(pool)
        .await
    }

    /// List tasks by type
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `task_type` - Task type to filter by
    ///
    /// # Returns
    /// Vector of tasks with the specified type or database error
    pub async fn list_by_type(
        pool: &DatabasePool,
        task_type: &str,
    ) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE task_type = ? ORDER BY created_at DESC"
        )
        .bind(task_type)
        .fetch_all(pool)
        .await
    }

    /// Get tasks by workspace path
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `workspace_path` - Workspace path to filter by
    ///
    /// # Returns
    /// Vector of tasks in the specified workspace or database error
    pub async fn get_by_workspace(
        pool: &DatabasePool,
        workspace_path: &str,
    ) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE workspace_path = ? ORDER BY created_at DESC"
        )
        .bind(workspace_path)
        .fetch_all(pool)
        .await
    }

    /// Update task status
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Task identifier
    /// * `status` - New status value
    ///
    /// # Returns
    /// Success or database error
    pub async fn update_status(
        pool: &DatabasePool,
        id: &str,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Update task with error information
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Task identifier
    /// * `error` - Error message
    ///
    /// # Returns
    /// Success or database error
    pub async fn update_error(
        pool: &DatabasePool,
        id: &str,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE tasks SET status = ?, error = ?, updated_at = ?, completed_at = ? WHERE id = ?"
        )
        .bind("failed")
        .bind(error)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Mark task as started
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Task identifier
    ///
    /// # Returns
    /// Success or database error
    pub async fn mark_started(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE tasks SET status = ?, started_at = ?, updated_at = ? WHERE id = ?"
        )
        .bind("running")
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Mark task as completed
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Task identifier
    ///
    /// # Returns
    /// Success or database error
    pub async fn mark_completed(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE tasks SET status = ?, completed_at = ?, updated_at = ? WHERE id = ?"
        )
        .bind("completed")
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete a task
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Task identifier
    ///
    /// # Returns
    /// Success or database error
    pub async fn delete(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Count total tasks
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// Total task count or database error
    pub async fn count(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(pool)
            .await?;

        Ok(result.0)
    }

    /// Count tasks by status
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `status` - Status to count
    ///
    /// # Returns
    /// Count of tasks with the specified status or database error
    pub async fn count_by_status(
        pool: &DatabasePool,
        status: &str,
    ) -> Result<i64, sqlx::Error> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE status = ?")
                .bind(status)
                .fetch_one(pool)
                .await?;

        Ok(result.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_task() {
        let pool = sqlx::sqlite::SqlitePool::connect("sqlite::memory:")
            .await
            .unwrap();

        // Run migrations
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
                error TEXT,
                CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        let task = TaskRepository::create(
            &pool,
            "task-1".to_string(),
            "Test Task".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(task.id, "task-1");
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.status, "pending");
    }

    #[tokio::test]
    async fn test_get_by_id() {
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
                error TEXT,
                CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        let created = TaskRepository::create(
            &pool,
            "task-1".to_string(),
            "Test Task".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .await
        .unwrap();

        let fetched = TaskRepository::get_by_id(&pool, "task-1")
            .await
            .unwrap();

        assert_eq!(fetched.map(|t| t.id), Some(created.id));
    }

    #[tokio::test]
    async fn test_list_tasks() {
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
                error TEXT,
                CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        TaskRepository::create(
            &pool,
            "task-1".to_string(),
            "Task 1".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .await
        .unwrap();

        TaskRepository::create(
            &pool,
            "task-2".to_string(),
            "Task 2".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .await
        .unwrap();

        let tasks = TaskRepository::list(&pool).await.unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_list_by_status() {
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
                error TEXT,
                CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        TaskRepository::create(
            &pool,
            "task-1".to_string(),
            "Task 1".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .await
        .unwrap();

        TaskRepository::update_status(&pool, "task-1", "running")
            .await
            .unwrap();

        let running = TaskRepository::list_by_status(&pool, "running")
            .await
            .unwrap();
        assert_eq!(running.len(), 1);

        let pending = TaskRepository::list_by_status(&pool, "pending")
            .await
            .unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_update_status() {
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
                error TEXT,
                CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        TaskRepository::create(
            &pool,
            "task-1".to_string(),
            "Task 1".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .await
        .unwrap();

        TaskRepository::update_status(&pool, "task-1", "running")
            .await
            .unwrap();

        let task = TaskRepository::get_by_id(&pool, "task-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(task.status, "running");
    }

    #[tokio::test]
    async fn test_delete_task() {
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
                error TEXT,
                CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        TaskRepository::create(
            &pool,
            "task-1".to_string(),
            "Task 1".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .await
        .unwrap();

        TaskRepository::delete(&pool, "task-1")
            .await
            .unwrap();

        let task = TaskRepository::get_by_id(&pool, "task-1")
            .await
            .unwrap();
        assert!(task.is_none());
    }

    #[tokio::test]
    async fn test_count_tasks() {
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
                error TEXT,
                CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        TaskRepository::create(
            &pool,
            "task-1".to_string(),
            "Task 1".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .await
        .unwrap();

        TaskRepository::create(
            &pool,
            "task-2".to_string(),
            "Task 2".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .await
        .unwrap();

        let count = TaskRepository::count(&pool).await.unwrap();
        assert_eq!(count, 2);
    }
}
