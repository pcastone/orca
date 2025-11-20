//! Bug model for database persistence

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a bug/issue in the orchestrator database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bug {
    /// Unique bug identifier (UUID string)
    pub id: String,

    /// Bug title/summary
    pub title: String,

    /// Detailed bug description
    pub description: Option<String>,

    /// Bug severity: low, medium, high, critical
    pub severity: String,

    /// Bug status: open, in_progress, resolved, closed, wont_fix
    pub status: String,

    /// Associated task ID (optional)
    pub task_id: Option<String>,

    /// Associated workflow ID (optional)
    pub workflow_id: Option<String>,

    /// Associated execution ID (optional)
    pub execution_id: Option<String>,

    /// Error message that caused the bug
    pub error_message: Option<String>,

    /// Stack trace if available
    pub stack_trace: Option<String>,

    /// Steps to reproduce the bug
    pub reproduction_steps: Option<String>,

    /// Expected behavior
    pub expected_behavior: Option<String>,

    /// Actual behavior observed
    pub actual_behavior: Option<String>,

    /// Environment information (JSON string)
    pub environment: Option<String>,

    /// Assigned user/team
    pub assignee: Option<String>,

    /// Bug reporter
    pub reporter: Option<String>,

    /// Labels/tags (JSON array string)
    pub labels: Option<String>,

    /// Creation timestamp (ISO8601 string)
    pub created_at: String,

    /// Last update timestamp (ISO8601 string)
    pub updated_at: String,

    /// Resolution timestamp (ISO8601 string, optional)
    pub resolved_at: Option<String>,
}

impl Bug {
    /// Create a new bug with required fields
    pub fn new(id: String, title: String, severity: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            title,
            description: None,
            severity,
            status: "open".to_string(),
            task_id: None,
            workflow_id: None,
            execution_id: None,
            error_message: None,
            stack_trace: None,
            reproduction_steps: None,
            expected_behavior: None,
            actual_behavior: None,
            environment: None,
            assignee: None,
            reporter: None,
            labels: None,
            created_at: now.clone(),
            updated_at: now,
            resolved_at: None,
        }
    }
}
