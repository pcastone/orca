//! Task model for database persistence

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a task in the orchestrator database
///
/// Tasks are the fundamental unit of work in the orchestration system.
/// Each task has a status, type, and associated configuration.
///
/// # Timestamps
/// All timestamp fields are ISO8601 strings due to SQLite type limitations.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    /// Unique task identifier (UUID string)
    pub id: String,

    /// Task title/name
    pub title: String,

    /// Optional task description
    pub description: Option<String>,

    /// Task type (e.g., "workflow", "execution", "validation")
    pub task_type: String,

    /// Current task status: pending, running, completed, failed, cancelled
    pub status: String,

    /// Task configuration as JSON string
    pub config: Option<String>,

    /// Task metadata as JSON string
    pub metadata: Option<String>,

    /// Workspace path where task is executed
    pub workspace_path: Option<String>,

    /// Task creation timestamp (ISO8601 string)
    pub created_at: String,

    /// Task last update timestamp (ISO8601 string)
    pub updated_at: String,

    /// Task start timestamp (ISO8601 string, optional)
    pub started_at: Option<String>,

    /// Task completion timestamp (ISO8601 string, optional)
    pub completed_at: Option<String>,

    /// Error message if task failed (optional)
    pub error: Option<String>,
}

impl Task {
    /// Create a new task with required fields
    ///
    /// # Arguments
    /// * `id` - Unique task identifier
    /// * `title` - Task title/name
    /// * `task_type` - Type of task
    /// * `workspace_path` - Where the task executes
    ///
    /// # Returns
    /// A new Task with default values for optional fields
    pub fn new(id: String, title: String, task_type: String, workspace_path: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            title,
            description: None,
            task_type,
            status: "pending".to_string(),
            config: None,
            metadata: None,
            workspace_path: Some(workspace_path),
            created_at: now.clone(),
            updated_at: now,
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    /// Builder method to set task description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builder method to set task configuration
    pub fn with_config(mut self, config: impl Into<String>) -> Self {
        self.config = Some(config.into());
        self
    }

    /// Builder method to set task metadata
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(
            "task-1".to_string(),
            "Test Task".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        );

        assert_eq!(task.id, "task-1");
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.task_type, "execution");
        assert_eq!(task.status, "pending");
        assert_eq!(task.workspace_path, Some("/workspace".to_string()));
    }

    #[test]
    fn test_task_with_description() {
        let task = Task::new(
            "task-1".to_string(),
            "Test Task".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .with_description("A test task");

        assert_eq!(task.description, Some("A test task".to_string()));
    }

    #[test]
    fn test_task_with_config() {
        let task = Task::new(
            "task-1".to_string(),
            "Test Task".to_string(),
            "execution".to_string(),
            "/workspace".to_string(),
        )
        .with_config(r#"{"timeout": 3600}"#);

        assert_eq!(task.config, Some(r#"{"timeout": 3600}"#.to_string()));
    }
}
