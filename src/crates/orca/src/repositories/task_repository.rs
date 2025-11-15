//! Task repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::workflow::Task;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for task database operations
#[derive(Clone, Debug)]
pub struct TaskRepository {
    db: Arc<Database>,
}

impl TaskRepository {
    /// Create a new task repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save a task to the database
    pub async fn save(&self, task: &Task) -> Result<()> {
        let created_at = Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO tasks (id, description, status, priority, created_at, updated_at, metadata)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&task.id)
        .bind(&task.description)
        .bind(&task.status)
        .bind(task.priority)
        .bind(created_at)
        .bind(created_at)
        .bind(&task.metadata)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save task: {}", e)))?;

        Ok(())
    }

    /// Load a task from the database by ID
    pub async fn find_by_id(&self, id: &str) -> Result<Task> {
        let row = sqlx::query(
            "SELECT id, description, status, priority, result, error, created_at, updated_at,
                    started_at, completed_at, metadata
             FROM tasks WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load task: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Task not found: {}", id)))?;

        let task = Task {
            id: row.get("id"),
            description: row.get("description"),
            status: row.get("status"),
            priority: row.get("priority"),
            result: row.get("result"),
            error: row.get("error"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
            metadata: row.get("metadata"),
        };

        Ok(task)
    }

    /// List all tasks from the database
    pub async fn list(&self) -> Result<Vec<Task>> {
        let rows = sqlx::query(
            "SELECT id, description, status, priority, result, error, created_at, updated_at,
                    started_at, completed_at, metadata
             FROM tasks
             ORDER BY created_at DESC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list tasks: {}", e)))?;

        let tasks = rows
            .into_iter()
            .map(|row| Task {
                id: row.get("id"),
                description: row.get("description"),
                status: row.get("status"),
                priority: row.get("priority"),
                result: row.get("result"),
                error: row.get("error"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                metadata: row.get("metadata"),
            })
            .collect();

        Ok(tasks)
    }

    /// List tasks by status
    pub async fn list_by_status(&self, status: &str) -> Result<Vec<Task>> {
        let rows = sqlx::query(
            "SELECT id, description, status, priority, result, error, created_at, updated_at,
                    started_at, completed_at, metadata
             FROM tasks
             WHERE status = ?
             ORDER BY created_at DESC"
        )
        .bind(status)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list tasks by status: {}", e)))?;

        let tasks = rows
            .into_iter()
            .map(|row| Task {
                id: row.get("id"),
                description: row.get("description"),
                status: row.get("status"),
                priority: row.get("priority"),
                result: row.get("result"),
                error: row.get("error"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                metadata: row.get("metadata"),
            })
            .collect();

        Ok(tasks)
    }

    /// Update a task in the database
    pub async fn update(&self, task: &Task) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE tasks
             SET description = ?, status = ?, priority = ?, result = ?, error = ?,
                 updated_at = ?, started_at = ?, completed_at = ?, metadata = ?
             WHERE id = ?"
        )
        .bind(&task.description)
        .bind(&task.status)
        .bind(task.priority)
        .bind(&task.result)
        .bind(&task.error)
        .bind(updated_at)
        .bind(task.started_at)
        .bind(task.completed_at)
        .bind(&task.metadata)
        .bind(&task.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update task: {}", e)))?;

        Ok(())
    }

    /// Delete a task from the database
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete task: {}", e)))?;

        Ok(())
    }

    /// Cancel a running or pending task
    ///
    /// Updates the task status to "cancelled" and sets the completed_at timestamp.
    /// Only tasks in "pending" or "running" status can be cancelled.
    pub async fn cancel_task(&self, id: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp();
        let completed_at = Utc::now().timestamp();

        // First check if task exists and is cancellable
        let task = self.find_by_id(id).await?;

        if task.status != "pending" && task.status != "running" {
            return Err(OrcaError::Other(format!(
                "Cannot cancel task with status '{}'. Only 'pending' or 'running' tasks can be cancelled.",
                task.status
            )));
        }

        // Update task to cancelled status
        sqlx::query(
            "UPDATE tasks
             SET status = 'cancelled', completed_at = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(completed_at)
        .bind(updated_at)
        .bind(id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to cancel task: {}", e)))?;

        Ok(())
    }

    /// Count tasks by status
    pub async fn count_by_status(&self, status: &str) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM tasks WHERE status = ?")
            .bind(status)
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to count tasks: {}", e)))?;

        Ok(row.get("count"))
    }

    /// Check if a task exists
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM tasks WHERE id = ?")
            .bind(id)
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to check task existence: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use tokio::task;

    async fn setup_test_db() -> Arc<Database> {
        let pool = SqlitePoolOptions::new()
            .max_connections(10) // Increased for concurrent tests
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Run migrations
        db.run_migrations().await.unwrap();

        db
    }

    #[tokio::test]
    async fn test_save_and_find() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let task = Task::new("Test task");
        repo.save(&task).await.unwrap();

        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert_eq!(loaded.id, task.id);
        assert_eq!(loaded.description, task.description);
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");

        repo.save(&task1).await.unwrap();
        repo.save(&task2).await.unwrap();

        let tasks = repo.list().await.unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_update_task() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let mut task = Task::new("Test task");
        repo.save(&task).await.unwrap();

        task.status = "completed".to_string();
        task.result = Some("Success".to_string());
        repo.update(&task).await.unwrap();

        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert_eq!(loaded.status, "completed");
        assert_eq!(loaded.result, Some("Success".to_string()));
    }

    #[tokio::test]
    async fn test_delete_task() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let task = Task::new("Test task");
        repo.save(&task).await.unwrap();

        repo.delete(&task.id).await.unwrap();

        let result = repo.find_by_id(&task.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let mut task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");

        task1.status = "completed".to_string();

        repo.save(&task1).await.unwrap();
        repo.save(&task2).await.unwrap();

        let completed = repo.list_by_status("completed").await.unwrap();
        assert_eq!(completed.len(), 1);

        let pending = repo.list_by_status("pending").await.unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_count_and_exists() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let task = Task::new("Test task");
        repo.save(&task).await.unwrap();

        let count = repo.count_by_status("pending").await.unwrap();
        assert_eq!(count, 1);

        let exists = repo.exists(&task.id).await.unwrap();
        assert!(exists);

        let not_exists = repo.exists("nonexistent").await.unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_cancel_pending_task() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let task = Task::new("Test task");
        repo.save(&task).await.unwrap();

        // Task should be in pending status
        assert_eq!(task.status, "pending");

        // Cancel the task
        repo.cancel_task(&task.id).await.unwrap();

        // Verify task is cancelled
        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert_eq!(loaded.status, "cancelled");
        assert!(loaded.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_cancel_running_task() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let mut task = Task::new("Test task");
        task.status = "running".to_string();
        repo.save(&task).await.unwrap();

        // Cancel the task
        repo.cancel_task(&task.id).await.unwrap();

        // Verify task is cancelled
        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert_eq!(loaded.status, "cancelled");
        assert!(loaded.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_cancel_completed_task_fails() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let mut task = Task::new("Test task");
        task.status = "completed".to_string();
        repo.save(&task).await.unwrap();

        // Attempt to cancel completed task should fail
        let result = repo.cancel_task(&task.id).await;
        assert!(result.is_err());

        // Verify task status unchanged
        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert_eq!(loaded.status, "completed");
    }

    #[tokio::test]
    async fn test_cancel_failed_task_fails() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let mut task = Task::new("Test task");
        task.status = "failed".to_string();
        repo.save(&task).await.unwrap();

        // Attempt to cancel failed task should fail
        let result = repo.cancel_task(&task.id).await;
        assert!(result.is_err());

        // Verify task status unchanged
        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert_eq!(loaded.status, "failed");
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_task_fails() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        // Attempt to cancel nonexistent task should fail
        let result = repo.cancel_task("nonexistent-id").await;
        assert!(result.is_err());
    }

    // ============================================================================
    // Phase 5.2: Repository Concurrent Access - TaskRepository Tests
    // ============================================================================

    #[tokio::test]
    async fn test_concurrent_save_operations() {
        let db = setup_test_db().await;
        let repo = Arc::new(TaskRepository::new(db));

        // Spawn 20 concurrent save operations
        let mut handles = vec![];

        for i in 0..20 {
            let repo_clone = repo.clone();
            let handle = task::spawn(async move {
                let task = Task::new(&format!("Concurrent task {}", i));
                repo_clone.save(&task).await.unwrap();
                task.id
            });
            handles.push(handle);
        }

        // Wait for all saves to complete
        let mut task_ids = vec![];
        for handle in handles {
            let id = handle.await.unwrap();
            task_ids.push(id);
        }

        // Verify all tasks were saved
        let all_tasks = repo.list().await.unwrap();
        assert_eq!(all_tasks.len(), 20);

        // Verify each task exists
        for id in task_ids {
            assert!(repo.exists(&id).await.unwrap());
        }
    }

    #[tokio::test]
    async fn test_concurrent_update_operations() {
        let db = setup_test_db().await;
        let repo = Arc::new(TaskRepository::new(db));

        // Create initial task
        let task = Task::new("Concurrent update test");
        repo.save(&task).await.unwrap();

        // Spawn 10 concurrent updates
        let mut handles = vec![];

        for i in 0..10 {
            let repo_clone = repo.clone();
            let task_id = task.id.clone();
            let handle = task::spawn(async move {
                let mut task = repo_clone.find_by_id(&task_id).await.unwrap();
                task.metadata = format!("{{\"iteration\": {}}}", i);
                repo_clone.update(&task).await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all updates to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify task still exists
        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert_eq!(loaded.id, task.id);
    }

    #[tokio::test]
    async fn test_concurrent_read_operations() {
        let db = setup_test_db().await;
        let repo = Arc::new(TaskRepository::new(db));

        // Create 10 tasks
        for i in 0..10 {
            let task = Task::new(&format!("Task {}", i));
            repo.save(&task).await.unwrap();
        }

        // Spawn 50 concurrent read operations
        let mut handles = vec![];

        for _ in 0..50 {
            let repo_clone = repo.clone();
            let handle = task::spawn(async move {
                let tasks = repo_clone.list().await.unwrap();
                assert_eq!(tasks.len(), 10);
            });
            handles.push(handle);
        }

        // All reads should succeed
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_concurrent_mixed_operations() {
        let db = setup_test_db().await;
        let repo = Arc::new(TaskRepository::new(db));

        // Spawn mixed operations
        let mut handles = vec![];

        // 10 save operations
        for i in 0..10 {
            let repo_clone = repo.clone();
            let handle = task::spawn(async move {
                let task = Task::new(&format!("Save task {}", i));
                repo_clone.save(&task).await.unwrap();
            });
            handles.push(handle);
        }

        // 10 read operations
        for _ in 0..10 {
            let repo_clone = repo.clone();
            let handle = task::spawn(async move {
                let _ = repo_clone.list().await.unwrap();
            });
            handles.push(handle);
        }

        // 5 count operations
        for _ in 0..5 {
            let repo_clone = repo.clone();
            let handle = task::spawn(async move {
                let _ = repo_clone.count_by_status("pending").await.unwrap();
            });
            handles.push(handle);
        }

        // All operations should succeed
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_concurrent_status_queries() {
        let db = setup_test_db().await;
        let repo = Arc::new(TaskRepository::new(db));

        // Create tasks with different statuses
        for i in 0..10 {
            let mut task = Task::new(&format!("Task {}", i));
            task.status = if i % 3 == 0 { "completed" } else if i % 3 == 1 { "running" } else { "pending" }.to_string();
            repo.save(&task).await.unwrap();
        }

        // Spawn concurrent status queries
        let mut handles = vec![];

        for _ in 0..10 {
            let repo_clone = repo.clone();
            let handle = task::spawn(async move {
                let pending = repo_clone.list_by_status("pending").await.unwrap();
                let running = repo_clone.list_by_status("running").await.unwrap();
                let completed = repo_clone.list_by_status("completed").await.unwrap();

                // Verify counts
                assert!(pending.len() + running.len() + completed.len() == 10);
            });
            handles.push(handle);
        }

        // All queries should succeed
        for handle in handles {
            handle.await.unwrap();
        }
    }

    // ============================================================================
    // Phase 5.2: Query Methods with Edge Cases
    // ============================================================================

    #[tokio::test]
    async fn test_task_with_special_characters() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let task = Task::new("Task with special chars: @#$%^&*()[]{}|\\;':\",.<>?/");
        repo.save(&task).await.unwrap();

        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert_eq!(loaded.description, task.description);
    }

    #[tokio::test]
    async fn test_task_with_unicode() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let task = Task::new("任務 🚀 Задача Aufgabe");
        repo.save(&task).await.unwrap();

        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert_eq!(loaded.description, task.description);
    }

    #[tokio::test]
    async fn test_task_with_very_long_description() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let long_desc = "A".repeat(10000);
        let task = Task::new(&long_desc);
        repo.save(&task).await.unwrap();

        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert_eq!(loaded.description.len(), 10000);
    }

    #[tokio::test]
    async fn test_list_by_status_empty_result() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        // No tasks saved
        let tasks = repo.list_by_status("nonexistent_status").await.unwrap();
        assert_eq!(tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_count_by_status_zero() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let count = repo.count_by_status("completed").await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_list_tasks_empty() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let tasks = repo.list().await.unwrap();
        assert_eq!(tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_task_with_null_optional_fields() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let task = Task::new("Task with nulls");
        // result, error, started_at, completed_at should all be None/NULL
        repo.save(&task).await.unwrap();

        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert!(loaded.result.is_none());
        assert!(loaded.error.is_none());
        assert!(loaded.started_at.is_none());
        assert!(loaded.completed_at.is_none());
    }

    #[tokio::test]
    async fn test_task_metadata_json() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let mut task = Task::new("Task with metadata");
        task.metadata = r#"{"key":"value","nested":{"foo":"bar"}}"#.to_string();
        repo.save(&task).await.unwrap();

        let loaded = repo.find_by_id(&task.id).await.unwrap();
        assert!(loaded.metadata.contains("key"));
        assert!(loaded.metadata.contains("value"));
    }

    #[tokio::test]
    async fn test_large_batch_operations() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        // Save 100 tasks
        for i in 0..100 {
            let task = Task::new(&format!("Batch task {}", i));
            repo.save(&task).await.unwrap();
        }

        // Verify count
        let tasks = repo.list().await.unwrap();
        assert_eq!(tasks.len(), 100);

        // Count by status
        let count = repo.count_by_status("pending").await.unwrap();
        assert_eq!(count, 100);
    }

    #[tokio::test]
    async fn test_update_nonexistent_task() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        let task = Task::new("Nonexistent");
        // Don't save, just try to update
        let result = repo.update(&task).await;

        // Should succeed but affect 0 rows (not an error in this implementation)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_task() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db);

        // Should succeed but affect 0 rows
        let result = repo.delete("nonexistent-id").await;
        assert!(result.is_ok());
    }
}
