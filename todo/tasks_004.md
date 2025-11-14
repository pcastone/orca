# Task 004: Implement Proto-Domain Type Conversions

## Objective
Create bidirectional type converters between Protocol Buffer types (generated from .proto files) and domain model types to maintain clean separation between wire format and business logic.

## Priority
**HIGH** - Required for both client and server implementations

## Dependencies
- Task 001 (Protocol Buffer definitions)
- Task 003 (Shared domain models)

## Implementation Details

### Files to Create

1. **`src/crates/orchestrator/src/proto_conv/mod.rs`**:
```rust
pub mod task;
pub mod workflow;
pub mod execution;

pub use task::{task_to_proto, proto_to_task};
pub use workflow::{workflow_to_proto, proto_to_workflow};
pub use execution::{execution_event_to_proto, proto_to_execution_event};
```

2. **`src/crates/orchestrator/src/proto_conv/task.rs`**:
```rust
use domain::{Task, TaskStatus, TaskType};
use crate::proto::tasks;
use anyhow::{Result, Context};

/// Convert domain Task to protobuf Task
pub fn task_to_proto(task: &Task) -> tasks::Task {
    tasks::Task {
        id: task.id.clone(),
        title: task.title.clone(),
        description: task.description.clone(),
        task_type: task_type_to_string(&task.task_type),
        status: task_status_to_proto(task.status) as i32,
        config: task.config.as_ref()
            .and_then(|v| serde_json::to_string(v).ok())
            .unwrap_or_default(),
        metadata: task.metadata.as_ref()
            .and_then(|v| serde_json::to_string(v).ok())
            .unwrap_or_default(),
        workspace_path: task.workspace_path.clone().unwrap_or_default(),
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
        started_at: task.started_at.map(|dt| dt.to_rfc3339()),
        completed_at: task.completed_at.map(|dt| dt.to_rfc3339()),
        error: task.error.clone(),
    }
}

/// Convert protobuf Task to domain Task
pub fn proto_to_task(proto: &tasks::Task) -> Result<Task> {
    Ok(Task {
        id: proto.id.clone(),
        title: proto.title.clone(),
        description: proto.description.clone(),
        task_type: task_type_from_string(&proto.task_type)?,
        status: proto_to_task_status(proto.status)?,
        config: if proto.config.is_empty() {
            None
        } else {
            Some(serde_json::from_str(&proto.config)?)
        },
        metadata: if proto.metadata.is_empty() {
            None
        } else {
            Some(serde_json::from_str(&proto.metadata)?)
        },
        workspace_path: if proto.workspace_path.is_empty() {
            None
        } else {
            Some(proto.workspace_path.clone())
        },
        created_at: chrono::DateTime::parse_from_rfc3339(&proto.created_at)?
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&proto.updated_at)?
            .with_timezone(&chrono::Utc),
        started_at: proto.started_at.as_ref()
            .map(|s| chrono::DateTime::parse_from_rfc3339(s))
            .transpose()?
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        completed_at: proto.completed_at.as_ref()
            .map(|s| chrono::DateTime::parse_from_rfc3339(s))
            .transpose()?
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        error: proto.error.clone(),
    })
}

fn task_status_to_proto(status: TaskStatus) -> tasks::TaskStatus {
    match status {
        TaskStatus::Pending => tasks::TaskStatus::Pending,
        TaskStatus::Running => tasks::TaskStatus::Running,
        TaskStatus::Completed => tasks::TaskStatus::Completed,
        TaskStatus::Failed => tasks::TaskStatus::Failed,
        TaskStatus::Cancelled => tasks::TaskStatus::Cancelled,
    }
}

fn proto_to_task_status(status: i32) -> Result<TaskStatus> {
    tasks::TaskStatus::try_from(status)
        .ok()
        .and_then(|s| match s {
            tasks::TaskStatus::Unspecified => None,
            tasks::TaskStatus::Pending => Some(TaskStatus::Pending),
            tasks::TaskStatus::Running => Some(TaskStatus::Running),
            tasks::TaskStatus::Completed => Some(TaskStatus::Completed),
            tasks::TaskStatus::Failed => Some(TaskStatus::Failed),
            tasks::TaskStatus::Cancelled => Some(TaskStatus::Cancelled),
        })
        .context("Invalid task status")
}

fn task_type_to_string(task_type: &TaskType) -> String {
    match task_type {
        TaskType::Code => "code".to_string(),
        TaskType::Research => "research".to_string(),
        TaskType::Review => "review".to_string(),
        TaskType::Custom(s) => s.clone(),
    }
}

fn task_type_from_string(s: &str) -> Result<TaskType> {
    Ok(match s {
        "code" => TaskType::Code,
        "research" => TaskType::Research,
        "review" => TaskType::Review,
        other => TaskType::Custom(other.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_roundtrip_task_conversion() {
        let original = Task::new(
            "Test Task".to_string(),
            "Test Description".to_string(),
            TaskType::Code,
        );

        let proto = task_to_proto(&original);
        let converted = proto_to_task(&proto).unwrap();

        assert_eq!(original.id, converted.id);
        assert_eq!(original.title, converted.title);
        assert_eq!(original.task_type, converted.task_type);
        assert_eq!(original.status, converted.status);
    }

    #[test]
    fn test_status_conversion() {
        assert_eq!(
            task_status_to_proto(TaskStatus::Running) as i32,
            tasks::TaskStatus::Running as i32
        );

        assert_eq!(
            proto_to_task_status(tasks::TaskStatus::Completed as i32).unwrap(),
            TaskStatus::Completed
        );
    }

    #[test]
    fn test_task_type_conversion() {
        assert_eq!(task_type_to_string(&TaskType::Code), "code");
        assert_eq!(
            task_type_from_string("code").unwrap(),
            TaskType::Code
        );

        let custom = TaskType::Custom("custom-type".to_string());
        assert_eq!(task_type_to_string(&custom), "custom-type");
        assert_eq!(
            task_type_from_string("custom-type").unwrap(),
            custom
        );
    }

    #[test]
    fn test_optional_fields() {
        let mut task = Task::new("T".into(), "D".into(), TaskType::Code);
        task.workspace_path = Some("/path/to/workspace".to_string());
        task.config = Some(serde_json::json!({"key": "value"}));

        let proto = task_to_proto(&task);
        let converted = proto_to_task(&proto).unwrap();

        assert_eq!(converted.workspace_path, task.workspace_path);
        assert_eq!(converted.config, task.config);
    }
}
```

3. **`src/crates/orchestrator/src/proto_conv/workflow.rs`**:
```rust
use domain::{Workflow, WorkflowStatus, WorkflowDefinition, WorkflowNode, WorkflowEdge};
use crate::proto::workflows;
use anyhow::{Result, Context};

pub fn workflow_to_proto(workflow: &Workflow) -> workflows::Workflow {
    workflows::Workflow {
        id: workflow.id.clone(),
        name: workflow.name.clone(),
        description: workflow.description.clone(),
        definition: serde_json::to_string(&workflow.definition)
            .unwrap_or_default(),
        status: workflow_status_to_proto(workflow.status) as i32,
        created_at: workflow.created_at.to_rfc3339(),
        updated_at: workflow.updated_at.to_rfc3339(),
        task_ids: workflow.task_ids.clone(),
    }
}

pub fn proto_to_workflow(proto: &workflows::Workflow) -> Result<Workflow> {
    Ok(Workflow {
        id: proto.id.clone(),
        name: proto.name.clone(),
        description: proto.description.clone(),
        definition: serde_json::from_str(&proto.definition)
            .context("Failed to parse workflow definition")?,
        status: proto_to_workflow_status(proto.status)?,
        created_at: chrono::DateTime::parse_from_rfc3339(&proto.created_at)?
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&proto.updated_at)?
            .with_timezone(&chrono::Utc),
        task_ids: proto.task_ids.clone(),
    })
}

fn workflow_status_to_proto(status: WorkflowStatus) -> workflows::WorkflowStatus {
    match status {
        WorkflowStatus::Draft => workflows::WorkflowStatus::Draft,
        WorkflowStatus::Active => workflows::WorkflowStatus::Active,
        WorkflowStatus::Archived => workflows::WorkflowStatus::Archived,
    }
}

fn proto_to_workflow_status(status: i32) -> Result<WorkflowStatus> {
    workflows::WorkflowStatus::try_from(status)
        .ok()
        .and_then(|s| match s {
            workflows::WorkflowStatus::Unspecified => None,
            workflows::WorkflowStatus::Draft => Some(WorkflowStatus::Draft),
            workflows::WorkflowStatus::Active => Some(WorkflowStatus::Active),
            workflows::WorkflowStatus::Archived => Some(WorkflowStatus::Archived),
        })
        .context("Invalid workflow status")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_roundtrip() {
        let def = WorkflowDefinition {
            nodes: vec![],
            edges: vec![],
        };
        let workflow = Workflow::new("Test".into(), "Desc".into(), def);

        let proto = workflow_to_proto(&workflow);
        let converted = proto_to_workflow(&proto).unwrap();

        assert_eq!(workflow.id, converted.id);
        assert_eq!(workflow.name, converted.name);
        assert_eq!(workflow.status, converted.status);
    }
}
```

4. **`src/crates/orchestrator/src/proto_conv/execution.rs`**:
```rust
use domain::{ExecutionEvent, ExecutionEventType};
use crate::proto::tasks;
use anyhow::{Result, Context};

pub fn execution_event_to_proto(event: &ExecutionEvent) -> tasks::ExecutionEvent {
    tasks::ExecutionEvent {
        task_id: event.task_id.clone(),
        event_type: event_type_to_proto(event.event_type) as i32,
        timestamp: event.timestamp.to_rfc3339(),
        data: event.data.as_ref()
            .and_then(|v| serde_json::to_string(v).ok()),
        error: event.error.clone(),
    }
}

pub fn proto_to_execution_event(proto: &tasks::ExecutionEvent) -> Result<ExecutionEvent> {
    Ok(ExecutionEvent {
        task_id: proto.task_id.clone(),
        event_type: proto_to_event_type(proto.event_type)?,
        timestamp: chrono::DateTime::parse_from_rfc3339(&proto.timestamp)?
            .with_timezone(&chrono::Utc),
        data: proto.data.as_ref()
            .map(|s| serde_json::from_str(s))
            .transpose()?,
        error: proto.error.clone(),
    })
}

fn event_type_to_proto(event_type: ExecutionEventType) -> tasks::ExecutionEventType {
    match event_type {
        ExecutionEventType::Started => tasks::ExecutionEventType::Started,
        ExecutionEventType::Progress => tasks::ExecutionEventType::Progress,
        ExecutionEventType::Output => tasks::ExecutionEventType::Output,
        ExecutionEventType::Completed => tasks::ExecutionEventType::Completed,
        ExecutionEventType::Failed => tasks::ExecutionEventType::Failed,
        ExecutionEventType::ToolCall => tasks::ExecutionEventType::ToolCall,
        ExecutionEventType::ToolResult => tasks::ExecutionEventType::ToolResult,
    }
}

fn proto_to_event_type(event_type: i32) -> Result<ExecutionEventType> {
    tasks::ExecutionEventType::try_from(event_type)
        .ok()
        .and_then(|et| match et {
            tasks::ExecutionEventType::Unspecified => None,
            tasks::ExecutionEventType::Started => Some(ExecutionEventType::Started),
            tasks::ExecutionEventType::Progress => Some(ExecutionEventType::Progress),
            tasks::ExecutionEventType::Output => Some(ExecutionEventType::Output),
            tasks::ExecutionEventType::Completed => Some(ExecutionEventType::Completed),
            tasks::ExecutionEventType::Failed => Some(ExecutionEventType::Failed),
            tasks::ExecutionEventType::ToolCall => Some(ExecutionEventType::ToolCall),
            tasks::ExecutionEventType::ToolResult => Some(ExecutionEventType::ToolResult),
        })
        .context("Invalid execution event type")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_event_roundtrip() {
        let event = ExecutionEvent::started("task-123".to_string());

        let proto = execution_event_to_proto(&event);
        let converted = proto_to_execution_event(&proto).unwrap();

        assert_eq!(event.task_id, converted.task_id);
        assert_eq!(event.event_type, converted.event_type);
    }
}
```

5. **Update `src/crates/orchestrator/src/lib.rs`**:
```rust
pub mod proto {
    tonic::include_proto!("orchestrator.tasks");
    tonic::include_proto!("orchestrator.workflows");
    tonic::include_proto!("orchestrator.health");
    tonic::include_proto!("orchestrator.auth");
}

pub mod proto_conv;
pub mod auth;
```

## Unit Tests

All unit tests embedded in the implementation. Additional edge case tests:

**`src/crates/orchestrator/tests/proto_conv_edge_cases.rs`**:
```rust
use orchestrator::proto_conv::*;
use domain::*;

#[test]
fn test_empty_optional_fields() {
    let task = Task::new("T".into(), "D".into(), TaskType::Code);
    let proto = task_to_proto(&task);
    let converted = proto_to_task(&proto).unwrap();

    assert!(converted.config.is_none());
    assert!(converted.metadata.is_none());
    assert!(converted.workspace_path.is_none());
    assert!(converted.started_at.is_none());
}

#[test]
fn test_invalid_json_in_config() {
    let mut proto = task_to_proto(&Task::new("T".into(), "D".into(), TaskType::Code));
    proto.config = "{invalid json}".to_string();

    let result = proto_to_task(&proto);
    assert!(result.is_err());
}

#[test]
fn test_invalid_timestamp() {
    let mut proto = task_to_proto(&Task::new("T".into(), "D".into(), TaskType::Code));
    proto.created_at = "not-a-timestamp".to_string();

    let result = proto_to_task(&proto);
    assert!(result.is_err());
}
```

## Acceptance Criteria

- [ ] Bidirectional conversion for Task ↔ proto Task
- [ ] Bidirectional conversion for Workflow ↔ proto Workflow
- [ ] Bidirectional conversion for ExecutionEvent ↔ proto ExecutionEvent
- [ ] Proper handling of optional fields
- [ ] JSON serialization for config/metadata/definition
- [ ] RFC3339 timestamp conversion
- [ ] Enum conversions with validation
- [ ] Error handling for invalid data
- [ ] All roundtrip tests pass
- [ ] Edge case tests pass

## Complexity
**Moderate** - Requires careful handling of optional fields, JSON, and timestamps

## Estimated Effort
**5-7 hours**

## Notes
- Use `anyhow::Context` for error messages
- Empty strings in proto should convert to None in domain
- Validate enums on conversion (reject Unspecified)
- Preserve timezone information (convert to UTC)
- JSON errors should provide context
