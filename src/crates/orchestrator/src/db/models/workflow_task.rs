//! WorkflowTask junction model for database persistence
//!
//! Represents the many-to-many relationship between workflows and tasks.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents the association between a workflow and a task
///
/// This junction table establishes a many-to-many relationship between
/// workflows and tasks, allowing tasks to be reused across multiple workflows
/// with different execution orders defined by the sequence field.
///
/// # Timestamps
/// All timestamp fields are ISO8601 strings due to SQLite type limitations.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowTask {
    /// Workflow identifier (foreign key)
    pub workflow_id: String,

    /// Task identifier (foreign key)
    pub task_id: String,

    /// Execution sequence/order within the workflow
    pub sequence: i32,

    /// Association creation timestamp (ISO8601 string)
    pub created_at: String,
}

impl WorkflowTask {
    /// Create a new workflow-task association
    ///
    /// # Arguments
    /// * `workflow_id` - ID of the parent workflow
    /// * `task_id` - ID of the associated task
    /// * `sequence` - Execution order (0-based)
    ///
    /// # Returns
    /// A new WorkflowTask with current timestamp
    pub fn new(workflow_id: String, task_id: String, sequence: i32) -> Self {
        Self {
            workflow_id,
            task_id,
            sequence,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a workflow-task association with a custom creation timestamp
    ///
    /// # Arguments
    /// * `workflow_id` - ID of the parent workflow
    /// * `task_id` - ID of the associated task
    /// * `sequence` - Execution order (0-based)
    /// * `created_at` - Custom creation timestamp (ISO8601 string)
    ///
    /// # Returns
    /// A new WorkflowTask with the specified timestamp
    pub fn with_timestamp(
        workflow_id: String,
        task_id: String,
        sequence: i32,
        created_at: String,
    ) -> Self {
        Self {
            workflow_id,
            task_id,
            sequence,
            created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_task_creation() {
        let wt = WorkflowTask::new(
            "workflow-1".to_string(),
            "task-1".to_string(),
            0,
        );

        assert_eq!(wt.workflow_id, "workflow-1");
        assert_eq!(wt.task_id, "task-1");
        assert_eq!(wt.sequence, 0);
    }

    #[test]
    fn test_workflow_task_with_timestamp() {
        let ts = "2025-11-10T12:00:00Z";
        let wt = WorkflowTask::with_timestamp(
            "workflow-1".to_string(),
            "task-1".to_string(),
            1,
            ts.to_string(),
        );

        assert_eq!(wt.workflow_id, "workflow-1");
        assert_eq!(wt.task_id, "task-1");
        assert_eq!(wt.sequence, 1);
        assert_eq!(wt.created_at, ts);
    }

    #[test]
    fn test_workflow_task_sequence() {
        let wt1 = WorkflowTask::new("wf-1".to_string(), "task-1".to_string(), 0);
        let wt2 = WorkflowTask::new("wf-1".to_string(), "task-2".to_string(), 1);
        let wt3 = WorkflowTask::new("wf-1".to_string(), "task-3".to_string(), 2);

        assert!(wt1.sequence < wt2.sequence);
        assert!(wt2.sequence < wt3.sequence);
    }
}
