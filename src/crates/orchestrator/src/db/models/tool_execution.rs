//! ToolExecution model for database persistence
//!
//! Represents the execution of an external tool invoked during task processing.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents the execution of an external tool
///
/// Tool executions are audit logs for all invocations of external tools
/// from tasks. Each execution tracks arguments, output, status, and duration.
///
/// # Timestamps
/// All timestamp fields are ISO8601 strings due to SQLite type limitations.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ToolExecution {
    /// Unique execution identifier (UUID string)
    pub id: String,

    /// ID of the task that invoked this tool (foreign key)
    pub task_id: String,

    /// Name of the tool that was executed
    pub tool_name: String,

    /// Arguments passed to the tool (JSON string)
    pub arguments: String,

    /// Tool output as JSON string (optional)
    pub output: Option<String>,

    /// Execution status: pending, running, completed, failed, timeout
    pub status: String,

    /// Error message if execution failed (optional)
    pub error: Option<String>,

    /// Execution start timestamp (ISO8601 string)
    pub created_at: String,

    /// Execution completion timestamp (ISO8601 string, optional)
    pub completed_at: Option<String>,

    /// Execution duration in milliseconds (optional)
    pub duration_ms: Option<i32>,
}

impl ToolExecution {
    /// Create a new tool execution record
    ///
    /// # Arguments
    /// * `id` - Unique execution identifier
    /// * `task_id` - ID of the invoking task
    /// * `tool_name` - Name of the tool
    /// * `arguments` - Tool arguments as JSON string
    ///
    /// # Returns
    /// A new ToolExecution with pending status and current timestamp
    pub fn new(id: String, task_id: String, tool_name: String, arguments: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            task_id,
            tool_name,
            arguments,
            output: None,
            status: "pending".to_string(),
            error: None,
            created_at: now,
            completed_at: None,
            duration_ms: None,
        }
    }

    /// Builder method to set tool output
    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Mark execution as running
    pub fn mark_running(&mut self) {
        self.status = "running".to_string();
    }

    /// Mark execution as completed with output and duration
    pub fn mark_completed(&mut self, output: impl Into<String>, duration_ms: i32) {
        self.status = "completed".to_string();
        self.output = Some(output.into());
        self.duration_ms = Some(duration_ms);
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Mark execution as failed with error message and duration
    pub fn mark_failed(&mut self, error: impl Into<String>, duration_ms: i32) {
        self.status = "failed".to_string();
        self.error = Some(error.into());
        self.duration_ms = Some(duration_ms);
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Mark execution as timeout with duration
    pub fn mark_timeout(&mut self, duration_ms: i32) {
        self.status = "timeout".to_string();
        self.duration_ms = Some(duration_ms);
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Check if execution is complete (completed, failed, or timeout)
    pub fn is_complete(&self) -> bool {
        matches!(self.status.as_str(), "completed" | "failed" | "timeout")
    }

    /// Check if execution is successful
    pub fn is_successful(&self) -> bool {
        self.status == "completed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_execution_creation() {
        let exec = ToolExecution::new(
            "exec-1".to_string(),
            "task-1".to_string(),
            "bash".to_string(),
            r#"{"script": "echo hello"}"#.to_string(),
        );

        assert_eq!(exec.id, "exec-1");
        assert_eq!(exec.task_id, "task-1");
        assert_eq!(exec.tool_name, "bash");
        assert_eq!(exec.status, "pending");
        assert!(!exec.is_complete());
    }

    #[test]
    fn test_tool_execution_mark_completed() {
        let mut exec = ToolExecution::new(
            "exec-1".to_string(),
            "task-1".to_string(),
            "bash".to_string(),
            r#"{"script": "echo hello"}"#.to_string(),
        );

        exec.mark_completed("hello", 100);

        assert_eq!(exec.status, "completed");
        assert_eq!(exec.output, Some("hello".to_string()));
        assert_eq!(exec.duration_ms, Some(100));
        assert!(exec.is_complete());
        assert!(exec.is_successful());
    }

    #[test]
    fn test_tool_execution_mark_failed() {
        let mut exec = ToolExecution::new(
            "exec-1".to_string(),
            "task-1".to_string(),
            "bash".to_string(),
            r#"{"script": "false"}"#.to_string(),
        );

        exec.mark_failed("Exit code: 1", 50);

        assert_eq!(exec.status, "failed");
        assert_eq!(exec.error, Some("Exit code: 1".to_string()));
        assert_eq!(exec.duration_ms, Some(50));
        assert!(exec.is_complete());
        assert!(!exec.is_successful());
    }

    #[test]
    fn test_tool_execution_mark_timeout() {
        let mut exec = ToolExecution::new(
            "exec-1".to_string(),
            "task-1".to_string(),
            "bash".to_string(),
            r#"{"script": "sleep 3600"}"#.to_string(),
        );

        exec.mark_timeout(30000);

        assert_eq!(exec.status, "timeout");
        assert_eq!(exec.duration_ms, Some(30000));
        assert!(exec.is_complete());
        assert!(!exec.is_successful());
    }
}
