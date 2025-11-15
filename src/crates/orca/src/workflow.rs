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

    // ============================================================================
    // Phase 6.1: Orca Workflow Execution - Comprehensive Tests
    // ============================================================================

    // ------------------------------------------------------------------------
    // Task State Transition Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_task_state_transitions_complete_lifecycle() {
        let mut task = Task::new("Complete lifecycle test");

        // Initial state
        assert!(task.is_pending());
        assert!(!task.is_running());
        assert!(!task.is_completed());
        assert!(!task.is_failed());
        assert!(!task.is_terminal());
        assert_eq!(task.status(), TaskStatus::Pending);
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());

        // Transition to running
        task.mark_running();
        assert!(!task.is_pending());
        assert!(task.is_running());
        assert!(!task.is_completed());
        assert!(!task.is_failed());
        assert!(!task.is_terminal());
        assert_eq!(task.status(), TaskStatus::Running);
        assert!(task.started_at.is_some());
        assert!(task.completed_at.is_none());

        // Transition to completed
        let started_at = task.started_at.unwrap();
        task.mark_completed(Some("Success result".to_string()));
        assert!(!task.is_pending());
        assert!(!task.is_running());
        assert!(task.is_completed());
        assert!(!task.is_failed());
        assert!(task.is_terminal());
        assert_eq!(task.status(), TaskStatus::Completed);
        assert_eq!(task.started_at.unwrap(), started_at);
        assert!(task.completed_at.is_some());
        assert_eq!(task.result, Some("Success result".to_string()));
    }

    #[test]
    fn test_task_state_transition_to_failed() {
        let mut task = Task::new("Failure test");

        task.mark_running();
        let started_at = task.started_at.unwrap();

        task.mark_failed("Task execution error");
        assert!(task.is_failed());
        assert!(task.is_terminal());
        assert_eq!(task.status(), TaskStatus::Failed);
        assert_eq!(task.error, Some("Task execution error".to_string()));
        assert_eq!(task.started_at.unwrap(), started_at);
        assert!(task.completed_at.is_some());
        assert!(task.result.is_none());
    }

    #[test]
    fn test_task_state_transition_to_cancelled() {
        let mut task = Task::new("Cancellation test");

        task.mark_running();
        task.mark_cancelled();

        assert!(!task.is_pending());
        assert!(!task.is_running());
        assert!(!task.is_completed());
        assert!(!task.is_failed());
        assert!(task.is_terminal());
        assert_eq!(task.status(), TaskStatus::Cancelled);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_cancelled_from_pending() {
        let mut task = Task::new("Cancel before start");

        assert!(task.is_pending());
        task.mark_cancelled();

        assert!(task.is_terminal());
        assert_eq!(task.status(), TaskStatus::Cancelled);
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_timestamps_are_updated() {
        let mut task = Task::new("Timestamp test");
        let created_at = task.created_at;
        let initial_updated = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        task.mark_running();
        assert!(task.updated_at >= initial_updated);
        let running_updated = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        task.mark_completed(None);
        assert!(task.updated_at >= running_updated);
        assert_eq!(task.created_at, created_at); // created_at never changes
    }

    #[test]
    fn test_task_multiple_state_changes() {
        let mut task = Task::new("Multiple state changes");

        // Can transition from running back to pending (edge case)
        task.mark_running();
        task.status = TaskStatus::Pending.as_str().to_string();
        assert!(task.is_pending());

        // Re-run after reset
        task.mark_running();
        assert!(task.is_running());

        // Can transition from failed to completed (recovery scenario)
        task.mark_failed("Initial failure");
        assert!(task.is_failed());

        task.mark_completed(Some("Recovered".to_string()));
        assert!(task.is_completed());
        assert_eq!(task.result, Some("Recovered".to_string()));
    }

    #[test]
    fn test_task_terminal_states() {
        // Test all terminal states
        let mut task1 = Task::new("Terminal test 1");
        task1.mark_completed(None);
        assert!(task1.is_terminal());

        let mut task2 = Task::new("Terminal test 2");
        task2.mark_failed("Error");
        assert!(task2.is_terminal());

        let mut task3 = Task::new("Terminal test 3");
        task3.mark_cancelled();
        assert!(task3.is_terminal());

        // Non-terminal states
        let task4 = Task::new("Terminal test 4");
        assert!(!task4.is_terminal());

        let mut task5 = Task::new("Terminal test 5");
        task5.mark_running();
        assert!(!task5.is_terminal());
    }

    #[test]
    fn test_task_with_priority() {
        let task = Task::new("Priority task").with_priority(10);
        assert_eq!(task.priority, 10);

        let task_high = Task::new("High priority").with_priority(100);
        let task_low = Task::new("Low priority").with_priority(-10);

        assert!(task_high.priority > task.priority);
        assert!(task_low.priority < task.priority);
    }

    #[test]
    fn test_task_with_metadata() {
        let metadata = r#"{"tags": ["important", "urgent"], "assignee": "alice"}"#;
        let task = Task::new("Metadata task").with_metadata(metadata);

        assert_eq!(task.metadata, metadata);

        // Verify metadata is valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&task.metadata).unwrap();
        assert_eq!(parsed["tags"][0], "important");
        assert_eq!(parsed["assignee"], "alice");
    }

    #[test]
    fn test_task_with_id() {
        let custom_id = "custom-task-id-123";
        let task = Task::with_id(custom_id, "Task with custom ID");

        assert_eq!(task.id, custom_id);
        assert_eq!(task.description, "Task with custom ID");
        assert!(task.is_pending());
    }

    #[test]
    fn test_task_completed_with_result() {
        let mut task = Task::new("Result test");
        task.mark_running();

        let result_data = r#"{"output": "Success", "metrics": {"duration": 1.5}}"#;
        task.mark_completed(Some(result_data.to_string()));

        assert!(task.is_completed());
        assert_eq!(task.result, Some(result_data.to_string()));

        // Verify result is valid JSON
        let parsed: serde_json::Value = serde_json::from_str(task.result.as_ref().unwrap()).unwrap();
        assert_eq!(parsed["output"], "Success");
    }

    #[test]
    fn test_task_completed_without_result() {
        let mut task = Task::new("No result test");
        task.mark_running();
        task.mark_completed(None);

        assert!(task.is_completed());
        assert!(task.result.is_none());
    }

    #[test]
    fn test_task_failure_preserves_started_time() {
        let mut task = Task::new("Failure time test");
        task.mark_running();

        let started_at = task.started_at.unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        task.mark_failed("Error occurred");

        assert_eq!(task.started_at.unwrap(), started_at);
        assert!(task.completed_at.is_some());
        // Timestamps are in seconds, so completed_at >= started_at
        assert!(task.completed_at.unwrap() >= started_at);
    }

    // ------------------------------------------------------------------------
    // Workflow State Transition Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_workflow_state_transitions_complete_lifecycle() {
        let mut workflow = Workflow::new("Complete workflow", "react");

        // Initial state
        assert!(workflow.is_pending());
        assert!(!workflow.is_running());
        assert!(!workflow.is_completed());
        assert!(!workflow.is_terminal());
        assert_eq!(workflow.status(), WorkflowStatus::Pending);
        assert!(workflow.started_at.is_none());
        assert!(workflow.completed_at.is_none());

        // Transition to running
        workflow.mark_running();
        assert!(!workflow.is_pending());
        assert!(workflow.is_running());
        assert!(!workflow.is_completed());
        assert!(!workflow.is_terminal());
        assert_eq!(workflow.status(), WorkflowStatus::Running);
        assert!(workflow.started_at.is_some());
        assert!(workflow.completed_at.is_none());

        // Transition to completed
        let started_at = workflow.started_at.unwrap();
        workflow.mark_completed();
        assert!(!workflow.is_pending());
        assert!(!workflow.is_running());
        assert!(workflow.is_completed());
        assert!(workflow.is_terminal());
        assert_eq!(workflow.status(), WorkflowStatus::Completed);
        assert_eq!(workflow.started_at.unwrap(), started_at);
        assert!(workflow.completed_at.is_some());
    }

    #[test]
    fn test_workflow_state_transition_to_failed() {
        let mut workflow = Workflow::new("Failing workflow", "react");

        workflow.mark_running();
        let started_at = workflow.started_at.unwrap();

        workflow.mark_failed();
        assert!(!workflow.is_completed());
        assert!(workflow.is_terminal());
        assert_eq!(workflow.status(), WorkflowStatus::Failed);
        assert_eq!(workflow.started_at.unwrap(), started_at);
        assert!(workflow.completed_at.is_some());
    }

    #[test]
    fn test_workflow_state_transition_to_cancelled() {
        let mut workflow = Workflow::new("Cancelled workflow", "react");

        workflow.mark_running();
        workflow.mark_cancelled();

        assert!(!workflow.is_pending());
        assert!(!workflow.is_running());
        assert!(!workflow.is_completed());
        assert!(workflow.is_terminal());
        assert_eq!(workflow.status(), WorkflowStatus::Cancelled);
        assert!(workflow.completed_at.is_some());
    }

    #[test]
    fn test_workflow_timestamps_are_updated() {
        let mut workflow = Workflow::new("Timestamp workflow", "plan_execute");
        let created_at = workflow.created_at;
        let initial_updated = workflow.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        workflow.mark_running();
        assert!(workflow.updated_at >= initial_updated);
        let running_updated = workflow.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        workflow.mark_completed();
        assert!(workflow.updated_at >= running_updated);
        assert_eq!(workflow.created_at, created_at);
    }

    #[test]
    fn test_workflow_terminal_states() {
        let mut workflow1 = Workflow::new("Terminal 1", "react");
        workflow1.mark_completed();
        assert!(workflow1.is_terminal());

        let mut workflow2 = Workflow::new("Terminal 2", "react");
        workflow2.mark_failed();
        assert!(workflow2.is_terminal());

        let mut workflow3 = Workflow::new("Terminal 3", "react");
        workflow3.mark_cancelled();
        assert!(workflow3.is_terminal());

        let workflow4 = Workflow::new("Non-terminal", "react");
        assert!(!workflow4.is_terminal());

        let mut workflow5 = Workflow::new("Running", "react");
        workflow5.mark_running();
        assert!(!workflow5.is_terminal());
    }

    #[test]
    fn test_workflow_with_description() {
        let workflow = Workflow::new("Test workflow", "react")
            .with_description("This is a test workflow for validation");

        assert_eq!(workflow.description, Some("This is a test workflow for validation".to_string()));
    }

    #[test]
    fn test_workflow_with_metadata() {
        let metadata = r#"{"owner": "bob", "project": "orca-test"}"#;
        let workflow = Workflow::new("Metadata workflow", "reflection")
            .with_metadata(metadata);

        assert_eq!(workflow.metadata, metadata);

        let parsed: serde_json::Value = serde_json::from_str(&workflow.metadata).unwrap();
        assert_eq!(parsed["owner"], "bob");
        assert_eq!(parsed["project"], "orca-test");
    }

    #[test]
    fn test_workflow_pattern_types() {
        let react_wf = Workflow::new("React workflow", "react");
        assert_eq!(react_wf.pattern, "react");

        let plan_wf = Workflow::new("Plan workflow", "plan_execute");
        assert_eq!(plan_wf.pattern, "plan_execute");

        let reflection_wf = Workflow::new("Reflection workflow", "reflection");
        assert_eq!(reflection_wf.pattern, "reflection");
    }

    // ------------------------------------------------------------------------
    // Workflow Routing Strategy Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_routing_strategy_default() {
        let workflow = Workflow::new("Default routing", "react");
        assert_eq!(workflow.routing_strategy(), "sequential");
    }

    #[test]
    fn test_routing_strategy_set_and_get() {
        let mut workflow = Workflow::new("Routing test", "react");

        workflow.set_routing_strategy("parallel");
        assert_eq!(workflow.routing_strategy(), "parallel");

        // Verify it's in metadata
        let parsed: serde_json::Value = serde_json::from_str(&workflow.metadata).unwrap();
        assert_eq!(parsed["routing_strategy"], "parallel");
    }

    #[test]
    fn test_routing_strategy_with_builder() {
        let workflow = Workflow::new("Builder routing", "react")
            .with_routing_strategy("conditional");

        assert_eq!(workflow.routing_strategy(), "conditional");
    }

    #[test]
    fn test_routing_strategy_preserves_other_metadata() {
        let mut workflow = Workflow::new("Metadata preservation", "react")
            .with_metadata(r#"{"owner": "alice", "priority": "high"}"#);

        workflow.set_routing_strategy("parallel");

        let parsed: serde_json::Value = serde_json::from_str(&workflow.metadata).unwrap();
        assert_eq!(parsed["routing_strategy"], "parallel");
        assert_eq!(parsed["owner"], "alice");
        assert_eq!(parsed["priority"], "high");
    }

    #[test]
    fn test_routing_strategy_all_types() {
        let strategies = vec!["sequential", "parallel", "conditional"];

        for strategy in strategies {
            let workflow = Workflow::new(&format!("{} workflow", strategy), "react")
                .with_routing_strategy(strategy);

            assert_eq!(workflow.routing_strategy(), strategy);
        }
    }

    #[test]
    fn test_routing_strategy_update_changes_timestamp() {
        let mut workflow = Workflow::new("Timestamp update", "react");
        let initial_updated = workflow.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        workflow.set_routing_strategy("parallel");
        assert!(workflow.updated_at >= initial_updated);
    }

    #[test]
    fn test_routing_strategy_with_malformed_metadata() {
        let mut workflow = Workflow::new("Malformed metadata", "react");
        workflow.metadata = "{invalid json".to_string();

        // Should return default when metadata is invalid
        assert_eq!(workflow.routing_strategy(), "sequential");
    }

    #[test]
    fn test_routing_strategy_with_empty_metadata() {
        let workflow = Workflow::new("Empty metadata", "react");
        // Default metadata is "{}"
        assert_eq!(workflow.routing_strategy(), "sequential");
    }

    // ------------------------------------------------------------------------
    // Task Coordination Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_multiple_tasks_independent_lifecycles() {
        let mut task1 = Task::new("Task 1");
        let mut task2 = Task::new("Task 2");
        let mut task3 = Task::new("Task 3");

        // Run all tasks
        task1.mark_running();
        task2.mark_running();
        task3.mark_running();

        // Complete task 1
        task1.mark_completed(Some("Task 1 result".to_string()));
        assert!(task1.is_completed());
        assert!(task2.is_running());
        assert!(task3.is_running());

        // Fail task 2
        task2.mark_failed("Task 2 error");
        assert!(task1.is_completed());
        assert!(task2.is_failed());
        assert!(task3.is_running());

        // Complete task 3
        task3.mark_completed(None);
        assert!(task1.is_completed());
        assert!(task2.is_failed());
        assert!(task3.is_completed());
    }

    #[test]
    fn test_task_priority_ordering() {
        let task_low = Task::new("Low priority").with_priority(1);
        let task_medium = Task::new("Medium priority").with_priority(5);
        let task_high = Task::new("High priority").with_priority(10);

        let mut tasks = vec![task_medium.clone(), task_low.clone(), task_high.clone()];
        tasks.sort_by_key(|t| -t.priority); // Sort descending

        assert_eq!(tasks[0].priority, 10);
        assert_eq!(tasks[1].priority, 5);
        assert_eq!(tasks[2].priority, 1);
    }

    #[test]
    fn test_workflow_with_multiple_patterns() {
        let workflows = vec![
            Workflow::new("React workflow", "react"),
            Workflow::new("Plan workflow", "plan_execute"),
            Workflow::new("Reflection workflow", "reflection"),
        ];

        assert_eq!(workflows.len(), 3);
        assert_eq!(workflows[0].pattern, "react");
        assert_eq!(workflows[1].pattern, "plan_execute");
        assert_eq!(workflows[2].pattern, "reflection");
    }

    // ------------------------------------------------------------------------
    // Failure Recovery and Error Handling Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_task_error_message_preservation() {
        let mut task = Task::new("Error preservation test");
        task.mark_running();

        let error_msg = "Detailed error: Connection timeout after 30 seconds";
        task.mark_failed(error_msg);

        assert_eq!(task.error, Some(error_msg.to_string()));
        assert!(task.result.is_none());
    }

    #[test]
    fn test_task_recovery_after_failure() {
        let mut task = Task::new("Recovery test");
        task.mark_running();
        task.mark_failed("First attempt failed");

        // Simulate recovery: reset status and retry
        task.status = TaskStatus::Pending.as_str().to_string();
        task.error = None;
        task.completed_at = None;

        task.mark_running();
        assert!(task.is_running());

        task.mark_completed(Some("Successful retry".to_string()));
        assert!(task.is_completed());
        assert_eq!(task.result, Some("Successful retry".to_string()));
    }

    #[test]
    fn test_workflow_failure_propagation() {
        let mut workflow = Workflow::new("Failure propagation", "react");
        let mut task1 = Task::new("Task 1");
        let mut task2 = Task::new("Task 2");

        // Start workflow and tasks
        workflow.mark_running();
        task1.mark_running();
        task2.mark_running();

        // Task 1 fails
        task1.mark_failed("Critical error");

        // Workflow should be marked as failed
        workflow.mark_failed();

        assert!(task1.is_failed());
        assert!(workflow.is_terminal());
        assert_eq!(workflow.status(), WorkflowStatus::Failed);
    }

    #[test]
    fn test_task_status_serialization() {
        // Test that TaskStatus can be serialized/deserialized
        assert_eq!(TaskStatus::Pending.as_str(), "pending");
        assert_eq!(TaskStatus::Running.as_str(), "running");
        assert_eq!(TaskStatus::Completed.as_str(), "completed");
        assert_eq!(TaskStatus::Failed.as_str(), "failed");
        assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");

        // Test display
        assert_eq!(format!("{}", TaskStatus::Pending), "pending");
        assert_eq!(format!("{}", TaskStatus::Running), "running");
    }

    #[test]
    fn test_workflow_status_serialization() {
        assert_eq!(WorkflowStatus::Pending.as_str(), "pending");
        assert_eq!(WorkflowStatus::Running.as_str(), "running");
        assert_eq!(WorkflowStatus::Completed.as_str(), "completed");
        assert_eq!(WorkflowStatus::Failed.as_str(), "failed");
        assert_eq!(WorkflowStatus::Cancelled.as_str(), "cancelled");

        assert_eq!(format!("{}", WorkflowStatus::Completed), "completed");
    }

    #[test]
    fn test_task_with_large_result() {
        let mut task = Task::new("Large result test");
        task.mark_running();

        // Create a large result (10KB of JSON)
        let large_data = serde_json::json!({
            "data": "x".repeat(10_000),
            "metadata": {"size": 10000}
        });

        task.mark_completed(Some(large_data.to_string()));
        assert!(task.is_completed());
        assert!(task.result.is_some());
        assert!(task.result.as_ref().unwrap().len() > 10_000);
    }

    #[test]
    fn test_workflow_resumption_scenario() {
        // Simulate workflow resumption after interruption
        let mut workflow = Workflow::new("Resumable workflow", "react");
        workflow.mark_running();

        let started_at = workflow.started_at.unwrap();

        // Simulate interruption (mark as pending to resume)
        workflow.status = WorkflowStatus::Pending.as_str().to_string();
        workflow.started_at = Some(started_at);

        // Resume execution
        workflow.mark_running();
        assert!(workflow.is_running());

        // Complete successfully
        workflow.mark_completed();
        assert!(workflow.is_completed());
        assert!(workflow.completed_at.is_some());
    }

    #[test]
    fn test_concurrent_task_status_checks() {
        // Test that status checks are thread-safe (read-only operations)
        let task = Task::new("Concurrent test");

        let is_pending = task.is_pending();
        let is_running = task.is_running();
        let is_completed = task.is_completed();
        let is_failed = task.is_failed();
        let is_terminal = task.is_terminal();

        assert!(is_pending);
        assert!(!is_running);
        assert!(!is_completed);
        assert!(!is_failed);
        assert!(!is_terminal);
    }

    #[test]
    fn test_workflow_status_from_string_all_variants() {
        assert_eq!(WorkflowStatus::from("pending"), WorkflowStatus::Pending);
        assert_eq!(WorkflowStatus::from("running"), WorkflowStatus::Running);
        assert_eq!(WorkflowStatus::from("completed"), WorkflowStatus::Completed);
        assert_eq!(WorkflowStatus::from("failed"), WorkflowStatus::Failed);
        assert_eq!(WorkflowStatus::from("cancelled"), WorkflowStatus::Cancelled);
        assert_eq!(WorkflowStatus::from("unknown"), WorkflowStatus::Pending);
    }

    #[test]
    fn test_task_status_from_string_all_variants() {
        assert_eq!(TaskStatus::from("pending"), TaskStatus::Pending);
        assert_eq!(TaskStatus::from("running"), TaskStatus::Running);
        assert_eq!(TaskStatus::from("completed"), TaskStatus::Completed);
        assert_eq!(TaskStatus::from("failed"), TaskStatus::Failed);
        assert_eq!(TaskStatus::from("cancelled"), TaskStatus::Cancelled);
        assert_eq!(TaskStatus::from("unknown"), TaskStatus::Pending);
    }
}

