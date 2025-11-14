//! Workflow and task definitions
//!
//! Defines the core Task and Workflow types with status tracking,
//! persistence, and lifecycle management.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Task status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task is queued and waiting to start
    Pending,
    /// Task is currently executing
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was cancelled by user
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for TaskStatus {
    fn from(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Pending,
        }
    }
}

/// Represents a task in the Orca database
///
/// Tasks are the fundamental unit of work in the orchestration system.
/// Each task has a description, status, priority, and optional result/error.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    /// Unique task identifier (UUID string)
    pub id: String,

    /// Task description/prompt
    pub description: String,

    /// Current task status
    pub status: String,

    /// Task priority (higher = more important)
    pub priority: i64,

    /// Task result (JSON string, optional)
    pub result: Option<String>,

    /// Error message if task failed (optional)
    pub error: Option<String>,

    /// Task creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Task last update timestamp (Unix timestamp)
    pub updated_at: i64,

    /// Task start timestamp (Unix timestamp, optional)
    pub started_at: Option<i64>,

    /// Task completion timestamp (Unix timestamp, optional)
    pub completed_at: Option<i64>,

    /// Task metadata as JSON string
    pub metadata: String,
}

impl Task {
    /// Create a new task
    ///
    /// # Arguments
    /// * `description` - Task description/prompt
    ///
    /// # Returns
    /// A new Task with pending status and generated UUID
    pub fn new(description: impl Into<String>) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.into(),
            status: TaskStatus::Pending.as_str().to_string(),
            priority: 0,
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            metadata: "{}".to_string(),
        }
    }

    /// Create a task with a specific ID
    pub fn with_id(id: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: id.into(),
            description: description.into(),
            status: TaskStatus::Pending.as_str().to_string(),
            priority: 0,
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            metadata: "{}".to_string(),
        }
    }

    /// Set task priority
    pub fn with_priority(mut self, priority: i64) -> Self {
        self.priority = priority;
        self
    }

    /// Set task metadata
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = metadata.into();
        self
    }

    /// Get task status as enum
    pub fn status(&self) -> TaskStatus {
        TaskStatus::from(self.status.as_str())
    }

    /// Mark task as running
    pub fn mark_running(&mut self) {
        let now = Utc::now().timestamp();
        self.status = TaskStatus::Running.as_str().to_string();
        self.started_at = Some(now);
        self.updated_at = now;
    }

    /// Mark task as completed with result
    pub fn mark_completed(&mut self, result: Option<String>) {
        let now = Utc::now().timestamp();
        self.status = TaskStatus::Completed.as_str().to_string();
        self.result = result;
        self.completed_at = Some(now);
        self.updated_at = now;
    }

    /// Mark task as failed with error
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        let now = Utc::now().timestamp();
        self.status = TaskStatus::Failed.as_str().to_string();
        self.error = Some(error.into());
        self.completed_at = Some(now);
        self.updated_at = now;
    }

    /// Mark task as cancelled
    pub fn mark_cancelled(&mut self) {
        let now = Utc::now().timestamp();
        self.status = TaskStatus::Cancelled.as_str().to_string();
        self.completed_at = Some(now);
        self.updated_at = now;
    }

    /// Check if task is pending
    pub fn is_pending(&self) -> bool {
        self.status() == TaskStatus::Pending
    }

    /// Check if task is running
    pub fn is_running(&self) -> bool {
        self.status() == TaskStatus::Running
    }

    /// Check if task is completed
    pub fn is_completed(&self) -> bool {
        self.status() == TaskStatus::Completed
    }

    /// Check if task is failed
    pub fn is_failed(&self) -> bool {
        self.status() == TaskStatus::Failed
    }

    /// Check if task is terminal (completed, failed, or cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status(),
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }
}

/// Workflow status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    /// Workflow is queued and waiting to start
    Pending,
    /// Workflow is currently executing
    Running,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed with an error
    Failed,
    /// Workflow was cancelled by user
    Cancelled,
}

impl WorkflowStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for WorkflowStatus {
    fn from(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Pending,
        }
    }
}

/// Represents a workflow in the Orca database
///
/// Workflows are collections of tasks with a defined execution pattern.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Workflow {
    /// Unique workflow identifier (UUID string)
    pub id: String,

    /// Workflow name
    pub name: String,

    /// Optional workflow description
    pub description: Option<String>,

    /// Current workflow status
    pub status: String,

    /// Agent pattern to use: "react", "plan_execute", "reflection"
    pub pattern: String,

    /// Workflow creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Workflow last update timestamp (Unix timestamp)
    pub updated_at: i64,

    /// Workflow start timestamp (Unix timestamp, optional)
    pub started_at: Option<i64>,

    /// Workflow completion timestamp (Unix timestamp, optional)
    pub completed_at: Option<i64>,

    /// Workflow metadata as JSON string
    pub metadata: String,
}

impl Workflow {
    /// Create a new workflow
    ///
    /// # Arguments
    /// * `name` - Workflow name
    /// * `pattern` - Agent pattern ("react", "plan_execute", "reflection")
    ///
    /// # Returns
    /// A new Workflow with pending status and generated UUID
    pub fn new(name: impl Into<String>, pattern: impl Into<String>) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            status: WorkflowStatus::Pending.as_str().to_string(),
            pattern: pattern.into(),
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            metadata: "{}".to_string(),
        }
    }

    /// Set workflow description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set workflow metadata
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = metadata.into();
        self
    }

    /// Get workflow status as enum
    pub fn status(&self) -> WorkflowStatus {
        WorkflowStatus::from(self.status.as_str())
    }

    /// Mark workflow as running
    pub fn mark_running(&mut self) {
        let now = Utc::now().timestamp();
        self.status = WorkflowStatus::Running.as_str().to_string();
        self.started_at = Some(now);
        self.updated_at = now;
    }

    /// Mark workflow as completed
    pub fn mark_completed(&mut self) {
        let now = Utc::now().timestamp();
        self.status = WorkflowStatus::Completed.as_str().to_string();
        self.completed_at = Some(now);
        self.updated_at = now;
    }

    /// Mark workflow as failed
    pub fn mark_failed(&mut self) {
        let now = Utc::now().timestamp();
        self.status = WorkflowStatus::Failed.as_str().to_string();
        self.completed_at = Some(now);
        self.updated_at = now;
    }

    /// Mark workflow as cancelled
    pub fn mark_cancelled(&mut self) {
        let now = Utc::now().timestamp();
        self.status = WorkflowStatus::Cancelled.as_str().to_string();
        self.completed_at = Some(now);
        self.updated_at = now;
    }

    /// Check if workflow is pending
    pub fn is_pending(&self) -> bool {
        self.status() == WorkflowStatus::Pending
    }

    /// Check if workflow is running
    pub fn is_running(&self) -> bool {
        self.status() == WorkflowStatus::Running
    }

    /// Check if workflow is completed
    pub fn is_completed(&self) -> bool {
        self.status() == WorkflowStatus::Completed
    }

    /// Check if workflow is terminal (completed, failed, or cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status(),
            WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled
        )
    }

    /// Get routing strategy from workflow metadata
    ///
    /// Returns the routing strategy specified in metadata, or "sequential" as default.
    /// Possible values: "sequential", "conditional", "parallel"
    pub fn routing_strategy(&self) -> String {
        if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&self.metadata) {
            if let Some(strategy) = metadata.get("routing_strategy").and_then(|s| s.as_str()) {
                return strategy.to_string();
            }
        }
        "sequential".to_string()
    }

    /// Set routing strategy in workflow metadata
    ///
    /// Updates the metadata JSON to include the routing_strategy field.
    /// Preserves other metadata fields if they exist.
    pub fn set_routing_strategy(&mut self, strategy: impl Into<String>) {
        let strategy = strategy.into();

        // Parse existing metadata or start with empty object
        let mut metadata: serde_json::Value = serde_json::from_str(&self.metadata)
            .unwrap_or(serde_json::json!({}));

        // Set routing_strategy field
        if let Some(obj) = metadata.as_object_mut() {
            obj.insert("routing_strategy".to_string(), serde_json::json!(strategy));
        }

        // Serialize back to string
        self.metadata = serde_json::to_string(&metadata).unwrap_or_else(|_| "{}".to_string());
        self.updated_at = Utc::now().timestamp();
    }

    /// Create a workflow with routing strategy
    pub fn with_routing_strategy(mut self, strategy: impl Into<String>) -> Self {
        self.set_routing_strategy(strategy);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task description");
        assert_eq!(task.description, "Test task description");
        assert_eq!(task.status(), TaskStatus::Pending);
        assert_eq!(task.priority, 0);
        assert!(task.is_pending());
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new("Test task");

        task.mark_running();
        assert!(task.is_running());
        assert!(task.started_at.is_some());

        task.mark_completed(Some("result".to_string()));
        assert!(task.is_completed());
        assert!(task.is_terminal());
        assert_eq!(task.result, Some("result".to_string()));
    }

    #[test]
    fn test_task_failure() {
        let mut task = Task::new("Test task");

        task.mark_failed("Something went wrong");
        assert!(task.is_failed());
        assert!(task.is_terminal());
        assert_eq!(task.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_workflow_creation() {
        let workflow = Workflow::new("Test Workflow", "react");
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.pattern, "react");
        assert_eq!(workflow.status(), WorkflowStatus::Pending);
        assert!(workflow.is_pending());
    }

    #[test]
    fn test_workflow_lifecycle() {
        let mut workflow = Workflow::new("Test Workflow", "plan_execute");

        workflow.mark_running();
        assert!(workflow.is_running());
        assert!(workflow.started_at.is_some());

        workflow.mark_completed();
        assert!(workflow.is_completed());
        assert!(workflow.is_terminal());
    }

    #[test]
    fn test_task_status_conversion() {
        assert_eq!(TaskStatus::from("pending"), TaskStatus::Pending);
        assert_eq!(TaskStatus::from("running"), TaskStatus::Running);
        assert_eq!(TaskStatus::from("completed"), TaskStatus::Completed);
        assert_eq!(TaskStatus::from("failed"), TaskStatus::Failed);
        assert_eq!(TaskStatus::from("invalid"), TaskStatus::Pending);
    }
}

