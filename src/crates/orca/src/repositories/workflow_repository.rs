//! Workflow repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::workflow::Workflow;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for workflow database operations
#[derive(Clone, Debug)]
pub struct WorkflowRepository {
    db: Arc<Database>,
}

impl WorkflowRepository {
    /// Create a new workflow repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save a workflow to the database
    pub async fn save(&self, workflow: &Workflow) -> Result<()> {
        let created_at = Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO workflows (id, name, description, status, pattern, created_at, updated_at, metadata)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&workflow.id)
        .bind(&workflow.name)
        .bind(&workflow.description)
        .bind(&workflow.status)
        .bind(&workflow.pattern)
        .bind(created_at)
        .bind(created_at)
        .bind(&workflow.metadata)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save workflow: {}", e)))?;

        Ok(())
    }

    /// Load a workflow from the database by ID
    pub async fn find_by_id(&self, id: &str) -> Result<Workflow> {
        let row = sqlx::query(
            "SELECT id, name, description, status, pattern, created_at, updated_at,
                    started_at, completed_at, metadata
             FROM workflows WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load workflow: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Workflow not found: {}", id)))?;

        let workflow = Workflow {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            status: row.get("status"),
            pattern: row.get("pattern"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
            metadata: row.get("metadata"),
        };

        Ok(workflow)
    }

    /// List all workflows from the database
    pub async fn list(&self) -> Result<Vec<Workflow>> {
        let rows = sqlx::query(
            "SELECT id, name, description, status, pattern, created_at, updated_at,
                    started_at, completed_at, metadata
             FROM workflows
             ORDER BY created_at DESC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list workflows: {}", e)))?;

        let workflows = rows
            .into_iter()
            .map(|row| Workflow {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                status: row.get("status"),
                pattern: row.get("pattern"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                metadata: row.get("metadata"),
            })
            .collect();

        Ok(workflows)
    }

    /// List workflows by status
    pub async fn list_by_status(&self, status: &str) -> Result<Vec<Workflow>> {
        let rows = sqlx::query(
            "SELECT id, name, description, status, pattern, created_at, updated_at,
                    started_at, completed_at, metadata
             FROM workflows
             WHERE status = ?
             ORDER BY created_at DESC"
        )
        .bind(status)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list workflows by status: {}", e)))?;

        let workflows = rows
            .into_iter()
            .map(|row| Workflow {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                status: row.get("status"),
                pattern: row.get("pattern"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                metadata: row.get("metadata"),
            })
            .collect();

        Ok(workflows)
    }

    /// Update a workflow in the database
    pub async fn update(&self, workflow: &Workflow) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE workflows
             SET name = ?, description = ?, status = ?, pattern = ?,
                 updated_at = ?, started_at = ?, completed_at = ?, metadata = ?
             WHERE id = ?"
        )
        .bind(&workflow.name)
        .bind(&workflow.description)
        .bind(&workflow.status)
        .bind(&workflow.pattern)
        .bind(updated_at)
        .bind(workflow.started_at)
        .bind(workflow.completed_at)
        .bind(&workflow.metadata)
        .bind(&workflow.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update workflow: {}", e)))?;

        Ok(())
    }

    /// Delete a workflow from the database
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM workflows WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete workflow: {}", e)))?;

        Ok(())
    }

    /// Add a task to a workflow
    pub async fn add_task(&self, workflow_id: &str, task_id: &str, sequence: i32) -> Result<()> {
        let created_at = Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO workflow_tasks (workflow_id, task_id, sequence, created_at)
             VALUES (?, ?, ?, ?)"
        )
        .bind(workflow_id)
        .bind(task_id)
        .bind(sequence)
        .bind(created_at)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to add task to workflow: {}", e)))?;

        Ok(())
    }

    /// Remove a task from a workflow
    pub async fn remove_task(&self, workflow_id: &str, task_id: &str) -> Result<()> {
        sqlx::query(
            "DELETE FROM workflow_tasks
             WHERE workflow_id = ? AND task_id = ?"
        )
        .bind(workflow_id)
        .bind(task_id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to remove task from workflow: {}", e)))?;

        Ok(())
    }

    /// Load workflow task IDs in sequence order
    pub async fn get_task_ids(&self, workflow_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT task_id FROM workflow_tasks
             WHERE workflow_id = ?
             ORDER BY sequence ASC"
        )
        .bind(workflow_id)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load workflow tasks: {}", e)))?;

        let task_ids = rows
            .into_iter()
            .map(|row| row.get("task_id"))
            .collect();

        Ok(task_ids)
    }

    /// Count workflows by status
    pub async fn count_by_status(&self, status: &str) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM workflows WHERE status = ?")
            .bind(status)
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to count workflows: {}", e)))?;

        Ok(row.get("count"))
    }

    /// Check if a workflow exists
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM workflows WHERE id = ?")
            .bind(id)
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to check workflow existence: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }

    /// Get task count for a workflow
    pub async fn get_task_count(&self, workflow_id: &str) -> Result<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM workflow_tasks WHERE workflow_id = ?"
        )
        .bind(workflow_id)
        .fetch_one(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to count workflow tasks: {}", e)))?;

        Ok(row.get("count"))
    }

    /// Pause a running workflow
    ///
    /// Updates the workflow status to "paused".
    /// Only workflows in "running" status can be paused.
    pub async fn pause_workflow(&self, id: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        // First check if workflow exists and is pausable
        let workflow = self.find_by_id(id).await?;

        if workflow.status != "running" {
            return Err(OrcaError::Other(format!(
                "Cannot pause workflow with status '{}'. Only 'running' workflows can be paused.",
                workflow.status
            )));
        }

        // Update workflow to paused status
        sqlx::query(
            "UPDATE workflows
             SET status = 'paused', updated_at = ?
             WHERE id = ?"
        )
        .bind(updated_at)
        .bind(id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to pause workflow: {}", e)))?;

        Ok(())
    }

    /// Resume a paused workflow
    ///
    /// Updates the workflow status from "paused" back to "running".
    /// Only workflows in "paused" status can be resumed.
    pub async fn resume_workflow(&self, id: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        // First check if workflow exists and is resumable
        let workflow = self.find_by_id(id).await?;

        if workflow.status != "paused" {
            return Err(OrcaError::Other(format!(
                "Cannot resume workflow with status '{}'. Only 'paused' workflows can be resumed.",
                workflow.status
            )));
        }

        // Update workflow to running status
        sqlx::query(
            "UPDATE workflows
             SET status = 'running', updated_at = ?
             WHERE id = ?"
        )
        .bind(updated_at)
        .bind(id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to resume workflow: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::Task;
    use crate::repositories::TaskRepository;
    use sqlx::sqlite::SqlitePoolOptions;

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
        let repo = WorkflowRepository::new(db);

        let workflow = Workflow::new("Test workflow", "react");
        repo.save(&workflow).await.unwrap();

        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.id, workflow.id);
        assert_eq!(loaded.name, workflow.name);
        assert_eq!(loaded.pattern, "react");
    }

    #[tokio::test]
    async fn test_list_workflows() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflow1 = Workflow::new("Workflow 1", "react");
        let workflow2 = Workflow::new("Workflow 2", "plan_execute");

        repo.save(&workflow1).await.unwrap();
        repo.save(&workflow2).await.unwrap();

        let workflows = repo.list().await.unwrap();
        assert_eq!(workflows.len(), 2);
    }

    #[tokio::test]
    async fn test_update_workflow() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let mut workflow = Workflow::new("Test workflow", "react");
        repo.save(&workflow).await.unwrap();

        workflow.status = "completed".to_string();
        workflow.description = Some("Updated description".to_string());
        repo.update(&workflow).await.unwrap();

        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.status, "completed");
        assert_eq!(loaded.description, Some("Updated description".to_string()));
    }

    #[tokio::test]
    async fn test_delete_workflow() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflow = Workflow::new("Test workflow", "react");
        repo.save(&workflow).await.unwrap();

        repo.delete(&workflow.id).await.unwrap();

        let result = repo.find_by_id(&workflow.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_workflow_tasks() {
        let db = setup_test_db().await;
        let workflow_repo = WorkflowRepository::new(db.clone());
        let task_repo = TaskRepository::new(db);

        // Create workflow
        let workflow = Workflow::new("Test workflow", "react");
        workflow_repo.save(&workflow).await.unwrap();

        // Create tasks
        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        task_repo.save(&task1).await.unwrap();
        task_repo.save(&task2).await.unwrap();

        // Add tasks to workflow
        workflow_repo.add_task(&workflow.id, &task1.id, 0).await.unwrap();
        workflow_repo.add_task(&workflow.id, &task2.id, 1).await.unwrap();

        // Get task IDs
        let task_ids = workflow_repo.get_task_ids(&workflow.id).await.unwrap();
        assert_eq!(task_ids.len(), 2);
        assert_eq!(task_ids[0], task1.id);
        assert_eq!(task_ids[1], task2.id);

        // Get task count
        let count = workflow_repo.get_task_count(&workflow.id).await.unwrap();
        assert_eq!(count, 2);

        // Remove a task
        workflow_repo.remove_task(&workflow.id, &task1.id).await.unwrap();
        let task_ids = workflow_repo.get_task_ids(&workflow.id).await.unwrap();
        assert_eq!(task_ids.len(), 1);
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let mut workflow1 = Workflow::new("Workflow 1", "react");
        let workflow2 = Workflow::new("Workflow 2", "react");

        workflow1.status = "completed".to_string();

        repo.save(&workflow1).await.unwrap();
        repo.save(&workflow2).await.unwrap();

        let completed = repo.list_by_status("completed").await.unwrap();
        assert_eq!(completed.len(), 1);

        let pending = repo.list_by_status("pending").await.unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_count_and_exists() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflow = Workflow::new("Test workflow", "react");
        repo.save(&workflow).await.unwrap();

        let count = repo.count_by_status("pending").await.unwrap();
        assert_eq!(count, 1);

        let exists = repo.exists(&workflow.id).await.unwrap();
        assert!(exists);

        let not_exists = repo.exists("nonexistent").await.unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_pause_running_workflow() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let mut workflow = Workflow::new("Test workflow", "react");
        workflow.status = "running".to_string();
        repo.save(&workflow).await.unwrap();

        // Pause the workflow
        repo.pause_workflow(&workflow.id).await.unwrap();

        // Verify workflow is paused
        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.status, "paused");
    }

    #[tokio::test]
    async fn test_pause_non_running_workflow_fails() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflow = Workflow::new("Test workflow", "react");
        repo.save(&workflow).await.unwrap();

        // Workflow is in pending status
        assert_eq!(workflow.status, "pending");

        // Attempt to pause pending workflow should fail
        let result = repo.pause_workflow(&workflow.id).await;
        assert!(result.is_err());

        // Verify workflow status unchanged
        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.status, "pending");
    }

    #[tokio::test]
    async fn test_resume_paused_workflow() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let mut workflow = Workflow::new("Test workflow", "react");
        workflow.status = "paused".to_string();
        repo.save(&workflow).await.unwrap();

        // Resume the workflow
        repo.resume_workflow(&workflow.id).await.unwrap();

        // Verify workflow is running
        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.status, "running");
    }

    #[tokio::test]
    async fn test_resume_non_paused_workflow_fails() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflow = Workflow::new("Test workflow", "react");
        repo.save(&workflow).await.unwrap();

        // Workflow is in pending status
        assert_eq!(workflow.status, "pending");

        // Attempt to resume pending workflow should fail
        let result = repo.resume_workflow(&workflow.id).await;
        assert!(result.is_err());

        // Verify workflow status unchanged
        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.status, "pending");
    }

    #[tokio::test]
    async fn test_pause_resume_cycle() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let mut workflow = Workflow::new("Test workflow", "react");
        workflow.status = "running".to_string();
        repo.save(&workflow).await.unwrap();

        // Pause
        repo.pause_workflow(&workflow.id).await.unwrap();
        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.status, "paused");

        // Resume
        repo.resume_workflow(&workflow.id).await.unwrap();
        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.status, "running");
    }

    #[tokio::test]
    async fn test_pause_nonexistent_workflow_fails() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        // Attempt to pause nonexistent workflow should fail
        let result = repo.pause_workflow("nonexistent-id").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resume_nonexistent_workflow_fails() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        // Attempt to resume nonexistent workflow should fail
        let result = repo.resume_workflow("nonexistent-id").await;
        assert!(result.is_err());
    }

    // ============================================================================
    // Phase 5.2: Repository Concurrent Access - WorkflowRepository Tests
    // ============================================================================

    #[tokio::test]
    async fn test_concurrent_workflow_save_operations() {
        let db = setup_test_db().await;
        let repo = Arc::new(WorkflowRepository::new(db));

        // Spawn 20 concurrent save operations
        let mut handles = vec![];

        for i in 0..20 {
            let repo_clone = repo.clone();
            let handle = tokio::task::spawn(async move {
                let workflow = Workflow::new(&format!("Concurrent workflow {}", i), "react");
                repo_clone.save(&workflow).await.unwrap();
                workflow.id
            });
            handles.push(handle);
        }

        // Wait for all saves to complete
        let mut workflow_ids = vec![];
        for handle in handles {
            let id = handle.await.unwrap();
            workflow_ids.push(id);
        }

        // Verify all workflows were saved
        let all_workflows = repo.list().await.unwrap();
        assert_eq!(all_workflows.len(), 20);

        // Verify each workflow exists
        for id in workflow_ids {
            assert!(repo.exists(&id).await.unwrap());
        }
    }

    #[tokio::test]
    async fn test_concurrent_workflow_update_operations() {
        let db = setup_test_db().await;
        let repo = Arc::new(WorkflowRepository::new(db));

        // Create initial workflow
        let workflow = Workflow::new("Concurrent update test", "react");
        repo.save(&workflow).await.unwrap();

        // Spawn 10 concurrent updates
        let mut handles = vec![];

        for i in 0..10 {
            let repo_clone = repo.clone();
            let workflow_id = workflow.id.clone();
            let handle = tokio::task::spawn(async move {
                let mut wf = repo_clone.find_by_id(&workflow_id).await.unwrap();
                wf.metadata = format!("{{\"iteration\": {}}}", i);
                repo_clone.update(&wf).await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all updates to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify workflow still exists
        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.id, workflow.id);
    }

    #[tokio::test]
    async fn test_concurrent_workflow_read_operations() {
        let db = setup_test_db().await;
        let repo = Arc::new(WorkflowRepository::new(db));

        // Create 10 workflows
        for i in 0..10 {
            let workflow = Workflow::new(&format!("Workflow {}", i), "react");
            repo.save(&workflow).await.unwrap();
        }

        // Spawn 50 concurrent read operations
        let mut handles = vec![];

        for _ in 0..50 {
            let repo_clone = repo.clone();
            let handle = tokio::task::spawn(async move {
                let workflows = repo_clone.list().await.unwrap();
                assert_eq!(workflows.len(), 10);
            });
            handles.push(handle);
        }

        // All reads should succeed
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_concurrent_workflow_task_operations() {
        let db = setup_test_db().await;
        let workflow_repo = Arc::new(WorkflowRepository::new(db.clone()));
        let task_repo = Arc::new(TaskRepository::new(db));

        // Create workflow
        let workflow = Workflow::new("Concurrent task ops", "react");
        workflow_repo.save(&workflow).await.unwrap();

        // Create tasks
        let mut task_ids = vec![];
        for i in 0..20 {
            let task = Task::new(&format!("Task {}", i));
            task_repo.save(&task).await.unwrap();
            task_ids.push(task.id);
        }

        // Spawn concurrent add_task operations
        let mut handles = vec![];

        for (i, task_id) in task_ids.iter().enumerate() {
            let workflow_repo_clone = workflow_repo.clone();
            let workflow_id = workflow.id.clone();
            let task_id_clone = task_id.clone();
            let handle = tokio::task::spawn(async move {
                workflow_repo_clone
                    .add_task(&workflow_id, &task_id_clone, i as i32)
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        // Wait for all adds to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all tasks were added
        let count = workflow_repo.get_task_count(&workflow.id).await.unwrap();
        assert_eq!(count, 20);
    }

    #[tokio::test]
    async fn test_concurrent_mixed_workflow_operations() {
        let db = setup_test_db().await;
        let repo = Arc::new(WorkflowRepository::new(db));

        // Spawn mixed operations
        let mut handles = vec![];

        // 10 save operations
        for i in 0..10 {
            let repo_clone = repo.clone();
            let handle = tokio::task::spawn(async move {
                let workflow = Workflow::new(&format!("Save workflow {}", i), "react");
                repo_clone.save(&workflow).await.unwrap();
            });
            handles.push(handle);
        }

        // 10 read operations
        for _ in 0..10 {
            let repo_clone = repo.clone();
            let handle = tokio::task::spawn(async move {
                let _ = repo_clone.list().await.unwrap();
            });
            handles.push(handle);
        }

        // 5 count operations
        for _ in 0..5 {
            let repo_clone = repo.clone();
            let handle = tokio::task::spawn(async move {
                let _ = repo_clone.count_by_status("pending").await.unwrap();
            });
            handles.push(handle);
        }

        // All operations should succeed
        for handle in handles {
            handle.await.unwrap();
        }
    }

    // ============================================================================
    // Phase 5.2: Foreign Key Constraint Tests
    // ============================================================================

    #[tokio::test]
    async fn test_cascade_delete_workflow_removes_tasks() {
        let db = setup_test_db().await;
        let workflow_repo = WorkflowRepository::new(db.clone());
        let task_repo = TaskRepository::new(db);

        // Create workflow
        let workflow = Workflow::new("Test workflow", "react");
        workflow_repo.save(&workflow).await.unwrap();

        // Create and add tasks
        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        task_repo.save(&task1).await.unwrap();
        task_repo.save(&task2).await.unwrap();

        workflow_repo.add_task(&workflow.id, &task1.id, 0).await.unwrap();
        workflow_repo.add_task(&workflow.id, &task2.id, 1).await.unwrap();

        // Verify tasks added
        let count = workflow_repo.get_task_count(&workflow.id).await.unwrap();
        assert_eq!(count, 2);

        // Delete workflow - should CASCADE delete workflow_tasks entries
        workflow_repo.delete(&workflow.id).await.unwrap();

        // Workflow should be gone
        assert!(!workflow_repo.exists(&workflow.id).await.unwrap());

        // Tasks should still exist (only workflow_tasks entries are deleted)
        assert!(task_repo.exists(&task1.id).await.unwrap());
        assert!(task_repo.exists(&task2.id).await.unwrap());

        // workflow_tasks entries should be gone (verified by re-creating workflow)
        let workflow2 = Workflow {
            id: workflow.id.clone(), // Re-use same ID
            name: "Recreated".to_string(),
            description: None,
            status: "pending".to_string(),
            pattern: "react".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            started_at: None,
            completed_at: None,
            metadata: "{}".to_string(),
        };
        workflow_repo.save(&workflow2).await.unwrap();

        // Should have 0 tasks (old entries were CASCADE deleted)
        let count = workflow_repo.get_task_count(&workflow2.id).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_cascade_delete_task_removes_workflow_tasks() {
        let db = setup_test_db().await;
        let workflow_repo = WorkflowRepository::new(db.clone());
        let task_repo = TaskRepository::new(db);

        // Create workflow
        let workflow = Workflow::new("Test workflow", "react");
        workflow_repo.save(&workflow).await.unwrap();

        // Create and add tasks
        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        task_repo.save(&task1).await.unwrap();
        task_repo.save(&task2).await.unwrap();

        workflow_repo.add_task(&workflow.id, &task1.id, 0).await.unwrap();
        workflow_repo.add_task(&workflow.id, &task2.id, 1).await.unwrap();

        // Verify 2 tasks added
        let count = workflow_repo.get_task_count(&workflow.id).await.unwrap();
        assert_eq!(count, 2);

        // Delete task1 - should CASCADE delete workflow_tasks entry
        task_repo.delete(&task1.id).await.unwrap();

        // workflow_tasks should now have only 1 entry
        let count = workflow_repo.get_task_count(&workflow.id).await.unwrap();
        assert_eq!(count, 1);

        // Verify remaining task is task2
        let task_ids = workflow_repo.get_task_ids(&workflow.id).await.unwrap();
        assert_eq!(task_ids.len(), 1);
        assert_eq!(task_ids[0], task2.id);
    }

    #[tokio::test]
    async fn test_add_task_to_nonexistent_workflow_fails() {
        let db = setup_test_db().await;
        let workflow_repo = WorkflowRepository::new(db.clone());
        let task_repo = TaskRepository::new(db);

        // Create task but not workflow
        let task = Task::new("Test task");
        task_repo.save(&task).await.unwrap();

        // Try to add task to nonexistent workflow
        let result = workflow_repo
            .add_task("nonexistent-workflow-id", &task.id, 0)
            .await;

        // Should fail due to foreign key constraint
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_nonexistent_task_to_workflow_fails() {
        let db = setup_test_db().await;
        let workflow_repo = WorkflowRepository::new(db);

        // Create workflow but not task
        let workflow = Workflow::new("Test workflow", "react");
        workflow_repo.save(&workflow).await.unwrap();

        // Try to add nonexistent task to workflow
        let result = workflow_repo
            .add_task(&workflow.id, "nonexistent-task-id", 0)
            .await;

        // Should fail due to foreign key constraint
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_duplicate_workflow_task_entry_fails() {
        let db = setup_test_db().await;
        let workflow_repo = WorkflowRepository::new(db.clone());
        let task_repo = TaskRepository::new(db);

        // Create workflow and task
        let workflow = Workflow::new("Test workflow", "react");
        let task = Task::new("Test task");
        workflow_repo.save(&workflow).await.unwrap();
        task_repo.save(&task).await.unwrap();

        // Add task to workflow
        workflow_repo.add_task(&workflow.id, &task.id, 0).await.unwrap();

        // Try to add same task again - should fail due to PRIMARY KEY constraint
        let result = workflow_repo.add_task(&workflow.id, &task.id, 1).await;
        assert!(result.is_err());
    }

    // ============================================================================
    // Phase 5.2: Query Methods with Edge Cases
    // ============================================================================

    #[tokio::test]
    async fn test_workflow_with_special_characters() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflow = Workflow::new("Workflow: @#$%^&*()[]{}|\\;':\",.<>?/", "react");
        repo.save(&workflow).await.unwrap();

        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.name, workflow.name);
    }

    #[tokio::test]
    async fn test_workflow_with_unicode() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflow = Workflow::new("工作流程 🔥 Рабочий процесс", "react");
        repo.save(&workflow).await.unwrap();

        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.name, workflow.name);
    }

    #[tokio::test]
    async fn test_workflow_with_very_long_name() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let long_name = "W".repeat(10000);
        let workflow = Workflow::new(&long_name, "react");
        repo.save(&workflow).await.unwrap();

        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert_eq!(loaded.name.len(), 10000);
    }

    #[tokio::test]
    async fn test_list_by_status_empty_result() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflows = repo.list_by_status("nonexistent_status").await.unwrap();
        assert_eq!(workflows.len(), 0);
    }

    #[tokio::test]
    async fn test_list_workflows_empty() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflows = repo.list().await.unwrap();
        assert_eq!(workflows.len(), 0);
    }

    #[tokio::test]
    async fn test_get_task_ids_empty_workflow() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflow = Workflow::new("Empty workflow", "react");
        repo.save(&workflow).await.unwrap();

        let task_ids = repo.get_task_ids(&workflow.id).await.unwrap();
        assert_eq!(task_ids.len(), 0);
    }

    #[tokio::test]
    async fn test_get_task_count_nonexistent_workflow() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        // Query nonexistent workflow should return 0
        let count = repo.get_task_count("nonexistent-id").await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_workflow_task_sequence_ordering() {
        let db = setup_test_db().await;
        let workflow_repo = WorkflowRepository::new(db.clone());
        let task_repo = TaskRepository::new(db);

        // Create workflow
        let workflow = Workflow::new("Test workflow", "react");
        workflow_repo.save(&workflow).await.unwrap();

        // Create tasks and add in non-sequential order
        let task1 = Task::new("Task 1");
        let task2 = Task::new("Task 2");
        let task3 = Task::new("Task 3");

        task_repo.save(&task1).await.unwrap();
        task_repo.save(&task2).await.unwrap();
        task_repo.save(&task3).await.unwrap();

        // Add with sequence: 2, 0, 1
        workflow_repo.add_task(&workflow.id, &task3.id, 2).await.unwrap();
        workflow_repo.add_task(&workflow.id, &task1.id, 0).await.unwrap();
        workflow_repo.add_task(&workflow.id, &task2.id, 1).await.unwrap();

        // Get task IDs - should be ordered by sequence
        let task_ids = workflow_repo.get_task_ids(&workflow.id).await.unwrap();
        assert_eq!(task_ids.len(), 3);
        assert_eq!(task_ids[0], task1.id);
        assert_eq!(task_ids[1], task2.id);
        assert_eq!(task_ids[2], task3.id);
    }

    #[tokio::test]
    async fn test_workflow_with_null_optional_fields() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflow = Workflow::new("Test workflow", "react");
        // description, started_at, completed_at should be None/NULL
        repo.save(&workflow).await.unwrap();

        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert!(loaded.description.is_none());
        assert!(loaded.started_at.is_none());
        assert!(loaded.completed_at.is_none());
    }

    #[tokio::test]
    async fn test_workflow_metadata_json() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let mut workflow = Workflow::new("Test workflow", "react");
        workflow.metadata = r#"{"config":{"retries":3},"tags":["important"]}"#.to_string();
        repo.save(&workflow).await.unwrap();

        let loaded = repo.find_by_id(&workflow.id).await.unwrap();
        assert!(loaded.metadata.contains("config"));
        assert!(loaded.metadata.contains("retries"));
    }

    #[tokio::test]
    async fn test_large_batch_workflow_operations() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        // Save 100 workflows
        for i in 0..100 {
            let workflow = Workflow::new(&format!("Batch workflow {}", i), "react");
            repo.save(&workflow).await.unwrap();
        }

        // Verify count
        let workflows = repo.list().await.unwrap();
        assert_eq!(workflows.len(), 100);

        // Count by status
        let count = repo.count_by_status("pending").await.unwrap();
        assert_eq!(count, 100);
    }

    #[tokio::test]
    async fn test_remove_task_from_empty_workflow() {
        let db = setup_test_db().await;
        let workflow_repo = WorkflowRepository::new(db.clone());
        let task_repo = TaskRepository::new(db);

        // Create workflow and task
        let workflow = Workflow::new("Test workflow", "react");
        let task = Task::new("Test task");
        workflow_repo.save(&workflow).await.unwrap();
        task_repo.save(&task).await.unwrap();

        // Try to remove task that was never added - should succeed (affect 0 rows)
        let result = workflow_repo.remove_task(&workflow.id, &task.id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_nonexistent_workflow() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        let workflow = Workflow::new("Nonexistent", "react");
        // Don't save, just try to update
        let result = repo.update(&workflow).await;

        // Should succeed but affect 0 rows (not an error in this implementation)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_workflow() {
        let db = setup_test_db().await;
        let repo = WorkflowRepository::new(db);

        // Should succeed but affect 0 rows
        let result = repo.delete("nonexistent-id").await;
        assert!(result.is_ok());
    }
}
