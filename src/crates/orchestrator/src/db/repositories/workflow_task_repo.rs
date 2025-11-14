//! WorkflowTask repository for managing workflow-task associations

use crate::db::connection::DatabasePool;
use crate::db::models::WorkflowTask;

/// WorkflowTask repository for managing workflow-task junction table operations
pub struct WorkflowTaskRepository;

impl WorkflowTaskRepository {
    /// Create a new workflow-task association
    pub async fn create(
        pool: &DatabasePool,
        workflow_id: String,
        task_id: String,
        sequence: i32,
    ) -> Result<WorkflowTask, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query_as::<_, WorkflowTask>(
            "INSERT INTO workflow_tasks (workflow_id, task_id, sequence, created_at)
             VALUES (?, ?, ?, ?)
             RETURNING *"
        )
        .bind(&workflow_id)
        .bind(&task_id)
        .bind(sequence)
        .bind(&now)
        .fetch_one(pool)
        .await
    }

    /// Get a workflow-task association
    pub async fn get(
        pool: &DatabasePool,
        workflow_id: &str,
        task_id: &str,
    ) -> Result<Option<WorkflowTask>, sqlx::Error> {
        sqlx::query_as::<_, WorkflowTask>(
            "SELECT * FROM workflow_tasks WHERE workflow_id = ? AND task_id = ?"
        )
        .bind(workflow_id)
        .bind(task_id)
        .fetch_optional(pool)
        .await
    }

    /// List all tasks in a workflow ordered by sequence
    pub async fn list_by_workflow(
        pool: &DatabasePool,
        workflow_id: &str,
    ) -> Result<Vec<WorkflowTask>, sqlx::Error> {
        sqlx::query_as::<_, WorkflowTask>(
            "SELECT * FROM workflow_tasks WHERE workflow_id = ? ORDER BY sequence ASC"
        )
        .bind(workflow_id)
        .fetch_all(pool)
        .await
    }

    /// List all workflows containing a specific task
    pub async fn list_by_task(
        pool: &DatabasePool,
        task_id: &str,
    ) -> Result<Vec<WorkflowTask>, sqlx::Error> {
        sqlx::query_as::<_, WorkflowTask>(
            "SELECT * FROM workflow_tasks WHERE task_id = ? ORDER BY created_at DESC"
        )
        .bind(task_id)
        .fetch_all(pool)
        .await
    }

    /// Update the sequence of a workflow-task association
    pub async fn update_sequence(
        pool: &DatabasePool,
        workflow_id: &str,
        task_id: &str,
        sequence: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE workflow_tasks SET sequence = ? WHERE workflow_id = ? AND task_id = ?"
        )
        .bind(sequence)
        .bind(workflow_id)
        .bind(task_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete a workflow-task association
    pub async fn delete(
        pool: &DatabasePool,
        workflow_id: &str,
        task_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM workflow_tasks WHERE workflow_id = ? AND task_id = ?"
        )
        .bind(workflow_id)
        .bind(task_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete all tasks from a workflow
    pub async fn delete_by_workflow(
        pool: &DatabasePool,
        workflow_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM workflow_tasks WHERE workflow_id = ?")
            .bind(workflow_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Count tasks in a workflow
    pub async fn count_by_workflow(
        pool: &DatabasePool,
        workflow_id: &str,
    ) -> Result<i64, sqlx::Error> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM workflow_tasks WHERE workflow_id = ?")
                .bind(workflow_id)
                .fetch_one(pool)
                .await?;

        Ok(result.0)
    }

    /// Count workflows containing a task
    pub async fn count_by_task(
        pool: &DatabasePool,
        task_id: &str,
    ) -> Result<i64, sqlx::Error> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM workflow_tasks WHERE task_id = ?")
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
            "CREATE TABLE workflows (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                definition TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"
        )
        .execute(&pool)
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
            "CREATE TABLE workflow_tasks (
                workflow_id TEXT NOT NULL,
                task_id TEXT NOT NULL,
                sequence INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                PRIMARY KEY (workflow_id, task_id),
                FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_workflow_task() {
        let pool = setup_db().await;

        // Create workflow and task
        sqlx::query("INSERT INTO workflows (id, name, definition, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("wf-1")
            .bind("Workflow 1")
            .bind("{}")
            .bind("draft")
            .bind("2025-11-10T00:00:00Z")
            .bind("2025-11-10T00:00:00Z")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO tasks (id, title, task_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("task-1")
            .bind("Task 1")
            .bind("execution")
            .bind("pending")
            .bind("2025-11-10T00:00:00Z")
            .bind("2025-11-10T00:00:00Z")
            .execute(&pool)
            .await
            .unwrap();

        let wt = WorkflowTaskRepository::create(
            &pool,
            "wf-1".to_string(),
            "task-1".to_string(),
            0,
        )
        .await
        .unwrap();

        assert_eq!(wt.workflow_id, "wf-1");
        assert_eq!(wt.task_id, "task-1");
        assert_eq!(wt.sequence, 0);
    }

    #[tokio::test]
    async fn test_list_by_workflow() {
        let pool = setup_db().await;

        // Setup
        sqlx::query("INSERT INTO workflows (id, name, definition, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("wf-1")
            .bind("Workflow 1")
            .bind("{}")
            .bind("draft")
            .bind("2025-11-10T00:00:00Z")
            .bind("2025-11-10T00:00:00Z")
            .execute(&pool)
            .await
            .unwrap();

        for i in 1..=3 {
            let task_id = format!("task-{}", i);
            sqlx::query("INSERT INTO tasks (id, title, task_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(&task_id)
                .bind(format!("Task {}", i))
                .bind("execution")
                .bind("pending")
                .bind("2025-11-10T00:00:00Z")
                .bind("2025-11-10T00:00:00Z")
                .execute(&pool)
                .await
                .unwrap();

            WorkflowTaskRepository::create(
                &pool,
                "wf-1".to_string(),
                task_id,
                (i - 1) as i32,
            )
            .await
            .unwrap();
        }

        let tasks = WorkflowTaskRepository::list_by_workflow(&pool, "wf-1")
            .await
            .unwrap();

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].sequence, 0);
        assert_eq!(tasks[1].sequence, 1);
        assert_eq!(tasks[2].sequence, 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_db().await;

        // Setup
        sqlx::query("INSERT INTO workflows (id, name, definition, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("wf-1")
            .bind("Workflow 1")
            .bind("{}")
            .bind("draft")
            .bind("2025-11-10T00:00:00Z")
            .bind("2025-11-10T00:00:00Z")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO tasks (id, title, task_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("task-1")
            .bind("Task 1")
            .bind("execution")
            .bind("pending")
            .bind("2025-11-10T00:00:00Z")
            .bind("2025-11-10T00:00:00Z")
            .execute(&pool)
            .await
            .unwrap();

        WorkflowTaskRepository::create(
            &pool,
            "wf-1".to_string(),
            "task-1".to_string(),
            0,
        )
        .await
        .unwrap();

        WorkflowTaskRepository::delete(&pool, "wf-1", "task-1")
            .await
            .unwrap();

        let wt = WorkflowTaskRepository::get(&pool, "wf-1", "task-1")
            .await
            .unwrap();

        assert!(wt.is_none());
    }
}
