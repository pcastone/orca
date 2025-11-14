/// Conversion functions between Protocol Buffer and domain models

use crate::proto::{tasks, workflows};
use chrono::Utc;

// ============================================================================
// Task Conversions
// ============================================================================

/// Convert domain Task to Proto Task
pub fn task_to_proto(task: &domain::Task) -> tasks::Task {
    tasks::Task {
        id: task.id.clone(),
        title: task.title.clone(),
        description: task.description.clone(),
        task_type: task.task_type.to_string(),
        status: task.status as i32,
        config: task.config.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default()),
        metadata: task.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()),
        workspace_path: task.workspace_path.clone().unwrap_or_default(),
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
    }
}

/// Convert Proto CreateTaskRequest to domain Task
pub fn proto_to_task(request: &tasks::CreateTaskRequest) -> domain::Task {
    let now = Utc::now();
    domain::Task {
        id: uuid::Uuid::new_v4().to_string(),
        title: request.title.clone(),
        description: request.description.clone(),
        task_type: request.task_type.clone().into(),
        status: domain::TaskStatus::Pending,
        config: request.config.as_ref().and_then(|c| serde_json::from_str(c).ok()),
        metadata: request.metadata.as_ref().and_then(|m| serde_json::from_str(m).ok()),
        workspace_path: Some(request.workspace_path.clone()),
        created_at: now,
        updated_at: now,
    }
}

// ============================================================================
// Workflow Conversions
// ============================================================================

/// Convert domain Workflow to Proto Workflow
pub fn workflow_to_proto(workflow: &domain::Workflow) -> workflows::Workflow {
    workflows::Workflow {
        id: workflow.id.clone(),
        name: workflow.name.clone(),
        description: workflow.description.clone().unwrap_or_default(),
        definition: workflow.definition.clone(),
        status: workflow.status.clone(),
        created_at: workflow.created_at.to_rfc3339(),
        updated_at: workflow.updated_at.to_rfc3339(),
    }
}

/// Convert Proto CreateWorkflowRequest to domain Workflow
pub fn proto_to_workflow(request: &workflows::CreateWorkflowRequest) -> domain::Workflow {
    let now = Utc::now();
    domain::Workflow {
        id: uuid::Uuid::new_v4().to_string(),
        name: request.name.clone(),
        description: Some(request.description.clone()),
        definition: request.definition.clone(),
        status: "draft".to_string(),
        created_at: now,
        updated_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_to_proto() {
        let now = Utc::now();
        let task = domain::Task {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            description: "A test task".to_string(),
            task_type: domain::TaskType::Code,
            status: domain::TaskStatus::Pending,
            config: None,
            metadata: None,
            workspace_path: Some("/tmp/workspace".to_string()),
            created_at: now,
            updated_at: now,
        };

        let proto = task_to_proto(&task);
        assert_eq!(proto.id, "task-1");
        assert_eq!(proto.title, "Test Task");
        assert_eq!(proto.description, "A test task");
    }

    #[test]
    fn test_workflow_to_proto() {
        let now = Utc::now();
        let workflow = domain::Workflow {
            id: "workflow-1".to_string(),
            name: "Test Workflow".to_string(),
            description: Some("A test workflow".to_string()),
            definition: r#"{"nodes": []}"#.to_string(),
            status: "draft".to_string(),
            created_at: now,
            updated_at: now,
        };

        let proto = workflow_to_proto(&workflow);
        assert_eq!(proto.id, "workflow-1");
        assert_eq!(proto.name, "Test Workflow");
        assert_eq!(proto.status, "draft");
    }
}
