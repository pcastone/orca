//! Task API models and DTOs
//!
//! Data transfer objects for task-related API operations.

use serde::{Deserialize, Serialize};

/// Request to create a new task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    /// Task title/name (required, 1-255 characters)
    pub title: String,

    /// Task description (optional)
    pub description: Option<String>,

    /// Task type (e.g., "workflow", "execution")
    pub task_type: String,

    /// Workspace path where task executes
    pub workspace_path: Option<String>,

    /// Task configuration as JSON string (optional)
    pub config: Option<String>,

    /// Task metadata as JSON string (optional)
    pub metadata: Option<String>,
}

impl CreateTaskRequest {
    /// Validate the create request
    pub fn validate(&self) -> crate::api::error::ApiResult<()> {
        crate::api::middleware::validation::validate_not_empty(&self.title, "title")?;
        crate::api::middleware::validation::validate_string_length(&self.title, "title", 1, 255)?;
        crate::api::middleware::validation::validate_not_empty(&self.task_type, "task_type")?;
        Ok(())
    }
}

/// Request to update an existing task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    /// Updated task title (optional)
    pub title: Option<String>,

    /// Updated task description (optional)
    pub description: Option<String>,

    /// Updated task type (optional)
    pub task_type: Option<String>,

    /// Updated task status (optional)
    pub status: Option<String>,

    /// Updated task configuration (optional)
    pub config: Option<String>,

    /// Updated task metadata (optional)
    pub metadata: Option<String>,

    /// Workspace path update (optional)
    pub workspace_path: Option<String>,
}

impl UpdateTaskRequest {
    /// Check if any fields are being updated
    pub fn has_updates(&self) -> bool {
        self.title.is_some()
            || self.description.is_some()
            || self.task_type.is_some()
            || self.status.is_some()
            || self.config.is_some()
            || self.metadata.is_some()
            || self.workspace_path.is_some()
    }
}

/// Task response for API (flattened database model)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    /// Task ID
    pub id: String,

    /// Task title
    pub title: String,

    /// Task description
    pub description: Option<String>,

    /// Task type
    pub task_type: String,

    /// Task status
    pub status: String,

    /// Task configuration
    pub config: Option<String>,

    /// Task metadata
    pub metadata: Option<String>,

    /// Workspace path
    pub workspace_path: Option<String>,

    /// Creation timestamp
    pub created_at: String,

    /// Last update timestamp
    pub updated_at: String,

    /// Task start timestamp
    pub started_at: Option<String>,

    /// Task completion timestamp
    pub completed_at: Option<String>,

    /// Error message if task failed
    pub error: Option<String>,
}

impl TaskResponse {
    /// Create a TaskResponse from database Task model
    pub fn from_db_task(task: crate::db::models::Task) -> Self {
        Self {
            id: task.id,
            title: task.title,
            description: task.description,
            task_type: task.task_type,
            status: task.status,
            config: task.config,
            metadata: task.metadata,
            workspace_path: task.workspace_path,
            created_at: task.created_at,
            updated_at: task.updated_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
            error: task.error,
        }
    }
}

/// Query parameters for listing tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListQuery {
    /// Filter by status (optional)
    pub status: Option<String>,

    /// Filter by task type (optional)
    pub task_type: Option<String>,

    /// Search in title and description (optional)
    pub search: Option<String>,

    /// Current page (0-indexed, default 0)
    pub page: Option<u32>,

    /// Items per page (default 20, max 100)
    pub per_page: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task_request_valid() {
        let req = CreateTaskRequest {
            title: "Test Task".to_string(),
            description: None,
            task_type: "execution".to_string(),
            workspace_path: Some("/workspace".to_string()),
            config: None,
            metadata: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_task_request_empty_title() {
        let req = CreateTaskRequest {
            title: "".to_string(),
            description: None,
            task_type: "execution".to_string(),
            workspace_path: None,
            config: None,
            metadata: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_task_request_has_updates() {
        let req = UpdateTaskRequest {
            title: Some("New Title".to_string()),
            description: None,
            task_type: None,
            status: None,
            config: None,
            metadata: None,
            workspace_path: None,
        };
        assert!(req.has_updates());
    }

    #[test]
    fn test_update_task_request_no_updates() {
        let req = UpdateTaskRequest {
            title: None,
            description: None,
            task_type: None,
            status: None,
            config: None,
            metadata: None,
            workspace_path: None,
        };
        assert!(!req.has_updates());
    }
}
