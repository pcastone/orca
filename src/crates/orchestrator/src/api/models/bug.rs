//! Bug API models and DTOs

use serde::{Deserialize, Serialize};

/// Request to create a new bug
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBugRequest {
    /// Bug title (required)
    pub title: String,

    /// Bug description (optional)
    pub description: Option<String>,

    /// Bug severity: low, medium, high, critical (default: medium)
    pub severity: Option<String>,

    /// Associated task ID (optional)
    pub task_id: Option<String>,

    /// Associated workflow ID (optional)
    pub workflow_id: Option<String>,

    /// Associated execution ID (optional)
    pub execution_id: Option<String>,

    /// Error message (optional)
    pub error_message: Option<String>,

    /// Stack trace (optional)
    pub stack_trace: Option<String>,

    /// Reproduction steps (optional)
    pub reproduction_steps: Option<String>,

    /// Expected behavior (optional)
    pub expected_behavior: Option<String>,

    /// Actual behavior (optional)
    pub actual_behavior: Option<String>,

    /// Environment info (optional)
    pub environment: Option<String>,

    /// Reporter (optional)
    pub reporter: Option<String>,

    /// Labels (optional, JSON array)
    pub labels: Option<String>,
}

impl CreateBugRequest {
    /// Validate the create request
    pub fn validate(&self) -> crate::api::error::ApiResult<()> {
        crate::api::middleware::validation::validate_not_empty(&self.title, "title")?;
        crate::api::middleware::validation::validate_string_length(&self.title, "title", 1, 255)?;
        if let Some(ref severity) = self.severity {
            if !["low", "medium", "high", "critical"].contains(&severity.as_str()) {
                return Err(crate::api::error::ApiError::BadRequest(
                    "Invalid severity. Must be: low, medium, high, or critical".to_string()
                ));
            }
        }
        Ok(())
    }
}

/// Request to update an existing bug
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBugRequest {
    /// Updated title (optional)
    pub title: Option<String>,

    /// Updated description (optional)
    pub description: Option<String>,

    /// Updated severity (optional)
    pub severity: Option<String>,

    /// Updated status (optional)
    pub status: Option<String>,

    /// Updated assignee (optional)
    pub assignee: Option<String>,

    /// Updated labels (optional)
    pub labels: Option<String>,
}

impl UpdateBugRequest {
    /// Check if any fields are being updated
    pub fn has_updates(&self) -> bool {
        self.title.is_some()
            || self.description.is_some()
            || self.severity.is_some()
            || self.status.is_some()
            || self.assignee.is_some()
            || self.labels.is_some()
    }
}

/// Bug response for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub severity: String,
    pub status: String,
    pub task_id: Option<String>,
    pub workflow_id: Option<String>,
    pub execution_id: Option<String>,
    pub error_message: Option<String>,
    pub stack_trace: Option<String>,
    pub reproduction_steps: Option<String>,
    pub expected_behavior: Option<String>,
    pub actual_behavior: Option<String>,
    pub environment: Option<String>,
    pub assignee: Option<String>,
    pub reporter: Option<String>,
    pub labels: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
}

impl BugResponse {
    /// Create a BugResponse from database Bug model
    pub fn from_db_bug(bug: crate::db::models::Bug) -> Self {
        Self {
            id: bug.id,
            title: bug.title,
            description: bug.description,
            severity: bug.severity,
            status: bug.status,
            task_id: bug.task_id,
            workflow_id: bug.workflow_id,
            execution_id: bug.execution_id,
            error_message: bug.error_message,
            stack_trace: bug.stack_trace,
            reproduction_steps: bug.reproduction_steps,
            expected_behavior: bug.expected_behavior,
            actual_behavior: bug.actual_behavior,
            environment: bug.environment,
            assignee: bug.assignee,
            reporter: bug.reporter,
            labels: bug.labels,
            created_at: bug.created_at,
            updated_at: bug.updated_at,
            resolved_at: bug.resolved_at,
        }
    }
}

/// Query parameters for listing bugs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugListQuery {
    /// Filter by status (optional)
    pub status: Option<String>,

    /// Filter by severity (optional)
    pub severity: Option<String>,

    /// Filter by task ID (optional)
    pub task_id: Option<String>,

    /// Filter by assignee (optional)
    pub assignee: Option<String>,

    /// Search in title and description (optional)
    pub search: Option<String>,

    /// Current page (0-indexed, default 0)
    pub page: Option<u32>,

    /// Items per page (default 20, max 100)
    pub per_page: Option<u32>,
}
