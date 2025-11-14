/// Conversion functions between Protocol Buffer and database models

use crate::db::models::Task as DbTask;
use crate::db::models::workflow::Workflow as DbWorkflow;
use crate::proto::{tasks, workflows};

// ============================================================================
// Task Conversions
// ============================================================================

/// Convert database Task to Proto Task
pub fn task_to_proto(task: &DbTask) -> tasks::Task {
    // Convert status string to integer
    let status = match task.status.as_str() {
        "pending" => 0,
        "running" => 1,
        "completed" => 2,
        "failed" => 3,
        "cancelled" => 4,
        _ => 0,
    };

    tasks::Task {
        id: task.id.clone(),
        title: task.title.clone(),
        description: task.description.clone().unwrap_or_default(),
        task_type: task.task_type.clone(),
        status,
        config: task.config.clone(),
        metadata: task.metadata.clone(),
        workspace_path: task.workspace_path.clone().unwrap_or_default(),
        created_at: task.created_at.clone(),
        updated_at: task.updated_at.clone(),
    }
}

/// Convert status integer to string
pub fn status_int_to_string(status: i32) -> String {
    match status {
        0 => "pending".to_string(),
        1 => "running".to_string(),
        2 => "completed".to_string(),
        3 => "failed".to_string(),
        4 => "cancelled".to_string(),
        _ => "pending".to_string(),
    }
}

// ============================================================================
// Workflow Conversions
// ============================================================================

/// Convert database Workflow to Proto Workflow
pub fn workflow_to_proto(workflow: &DbWorkflow) -> workflows::Workflow {
    workflows::Workflow {
        id: workflow.id.clone(),
        name: workflow.name.clone(),
        description: workflow.description.clone().unwrap_or_default(),
        definition: workflow.definition.clone(),
        status: workflow.status.clone(),
        created_at: workflow.created_at.clone(),
        updated_at: workflow.updated_at.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_task_to_proto() {
        let now = Utc::now().to_rfc3339();
        let task = DbTask {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            description: Some("A test task".to_string()),
            task_type: "code".to_string(),
            status: "pending".to_string(),
            config: None,
            metadata: None,
            workspace_path: Some("/tmp/workspace".to_string()),
            created_at: now.clone(),
            updated_at: now,
            started_at: None,
            completed_at: None,
            error: None,
        };

        let proto = task_to_proto(&task);
        assert_eq!(proto.id, "task-1");
        assert_eq!(proto.title, "Test Task");
        assert_eq!(proto.description, "A test task");
        assert_eq!(proto.status, 0); // pending
    }

    #[test]
    fn test_workflow_to_proto() {
        let now = Utc::now().to_rfc3339();
        let workflow = DbWorkflow {
            id: "workflow-1".to_string(),
            name: "Test Workflow".to_string(),
            description: Some("A test workflow".to_string()),
            definition: r#"{"nodes": []}"#.to_string(),
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        let proto = workflow_to_proto(&workflow);
        assert_eq!(proto.id, "workflow-1");
        assert_eq!(proto.name, "Test Workflow");
        assert_eq!(proto.status, "draft");
    }

    #[test]
    fn test_status_conversions() {
        assert_eq!(status_int_to_string(0), "pending");
        assert_eq!(status_int_to_string(1), "running");
        assert_eq!(status_int_to_string(2), "completed");
        assert_eq!(status_int_to_string(3), "failed");
        assert_eq!(status_int_to_string(4), "cancelled");
        assert_eq!(status_int_to_string(999), "pending"); // default
    }
}
