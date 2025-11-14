# Task 003: Create Shared Domain Models Library

## Objective
Create a shared library crate (`domain`) containing common types, traits, and conversions used by both aco client, orchestrator server, and orca standalone to avoid code duplication and ensure type safety.

## Priority
**HIGH** - Enables code reuse across all three applications

## Dependencies
- Task 001 (Protocol Buffer definitions)

## Implementation Details

### Files to Create

1. **Create new crate `src/crates/domain/Cargo.toml`**:
```toml
[package]
name = "domain"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true, features = ["v4", "serde"] }
thiserror = { workspace = true }
```

2. **`src/crates/domain/src/lib.rs`**:
```rust
pub mod task;
pub mod workflow;
pub mod execution;
pub mod error;

pub use task::{Task, TaskStatus, TaskType};
pub use workflow::{Workflow, WorkflowStatus, WorkflowDefinition};
pub use execution::{ExecutionEvent, ExecutionEventType, ExecutionStatus};
pub use error::DomainError;
```

3. **`src/crates/domain/src/task.rs`**:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub config: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub workspace_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Code,
    Research,
    Review,
    Custom(String),
}

impl Task {
    pub fn new(title: String, description: String, task_type: TaskType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            description,
            task_type,
            status: TaskStatus::Pending,
            config: None,
            metadata: None,
            workspace_path: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }

    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }
}

impl TaskStatus {
    pub fn to_i32(self) -> i32 {
        match self {
            TaskStatus::Pending => 1,
            TaskStatus::Running => 2,
            TaskStatus::Completed => 3,
            TaskStatus::Failed => 4,
            TaskStatus::Cancelled => 5,
        }
    }

    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            1 => Some(TaskStatus::Pending),
            2 => Some(TaskStatus::Running),
            3 => Some(TaskStatus::Completed),
            4 => Some(TaskStatus::Failed),
            5 => Some(TaskStatus::Cancelled),
            _ => None,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Some(TaskStatus::Pending),
            "running" => Some(TaskStatus::Running),
            "completed" => Some(TaskStatus::Completed),
            "failed" => Some(TaskStatus::Failed),
            "cancelled" => Some(TaskStatus::Cancelled),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_task_defaults() {
        let task = Task::new(
            "Test".to_string(),
            "Description".to_string(),
            TaskType::Code,
        );

        assert_eq!(task.status, TaskStatus::Pending);
        assert!(!task.is_terminal());
        assert!(task.duration().is_none());
    }

    #[test]
    fn test_task_status_conversions() {
        assert_eq!(TaskStatus::Running.to_i32(), 2);
        assert_eq!(TaskStatus::from_i32(2), Some(TaskStatus::Running));
        assert_eq!(TaskStatus::from_i32(99), None);
        assert_eq!(TaskStatus::from_str("pending"), Some(TaskStatus::Pending));
        assert_eq!(TaskStatus::from_str("INVALID"), None);
    }

    #[test]
    fn test_is_terminal() {
        assert!(Task {
            status: TaskStatus::Completed,
            ..Task::new("T".into(), "D".into(), TaskType::Code)
        }.is_terminal());

        assert!(!Task {
            status: TaskStatus::Running,
            ..Task::new("T".into(), "D".into(), TaskType::Code)
        }.is_terminal());
    }
}
```

4. **`src/crates/domain/src/workflow.rs`**:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub definition: WorkflowDefinition,
    pub status: WorkflowStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowDefinition {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowNode {
    pub id: String,
    pub node_type: String,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowEdge {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    Draft,
    Active,
    Archived,
}

impl Workflow {
    pub fn new(name: String, description: String, definition: WorkflowDefinition) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            definition,
            status: WorkflowStatus::Draft,
            created_at: now,
            updated_at: now,
            task_ids: Vec::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.definition.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.definition.edges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_workflow() {
        let def = WorkflowDefinition {
            nodes: vec![],
            edges: vec![],
        };
        let workflow = Workflow::new("Test".into(), "Desc".into(), def);

        assert_eq!(workflow.status, WorkflowStatus::Draft);
        assert_eq!(workflow.node_count(), 0);
        assert_eq!(workflow.edge_count(), 0);
    }
}
```

5. **`src/crates/domain/src/execution.rs`**:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionEvent {
    pub task_id: String,
    pub event_type: ExecutionEventType,
    pub timestamp: DateTime<Utc>,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEventType {
    Started,
    Progress,
    Output,
    Completed,
    Failed,
    ToolCall,
    ToolResult,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
}

impl ExecutionEvent {
    pub fn started(task_id: String) -> Self {
        Self {
            task_id,
            event_type: ExecutionEventType::Started,
            timestamp: Utc::now(),
            data: None,
            error: None,
        }
    }

    pub fn completed(task_id: String, data: Option<serde_json::Value>) -> Self {
        Self {
            task_id,
            event_type: ExecutionEventType::Completed,
            timestamp: Utc::now(),
            data,
            error: None,
        }
    }

    pub fn failed(task_id: String, error: String) -> Self {
        Self {
            task_id,
            event_type: ExecutionEventType::Failed,
            timestamp: Utc::now(),
            data: None,
            error: Some(error),
        }
    }
}
```

6. **`src/crates/domain/src/error.rs`**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Invalid task status: {0}")]
    InvalidTaskStatus(String),

    #[error("Invalid workflow status: {0}")]
    InvalidWorkflowStatus(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
}
```

## Update Dependencies

Add `domain` to workspace dependencies where needed:

**`src/crates/orchestrator/Cargo.toml`**:
```toml
[dependencies]
domain = { path = "../domain" }
```

**`src/crates/aco/Cargo.toml`**:
```toml
[dependencies]
domain = { path = "../domain" }
```

**`src/crates/orca/Cargo.toml`**:
```toml
[dependencies]
domain = { path = "../domain" }
```

## Unit Tests

All unit tests are embedded in the implementation files above. Additional integration tests:

**`src/crates/domain/tests/integration.rs`**:
```rust
use domain::*;

#[test]
fn test_task_lifecycle() {
    let mut task = Task::new("Test".into(), "Desc".into(), TaskType::Code);
    assert_eq!(task.status, TaskStatus::Pending);

    task.status = TaskStatus::Running;
    task.started_at = Some(chrono::Utc::now());
    assert!(!task.is_terminal());

    task.status = TaskStatus::Completed;
    task.completed_at = Some(chrono::Utc::now());
    assert!(task.is_terminal());
    assert!(task.duration().is_some());
}
```

## Acceptance Criteria

- [ ] Domain crate compiles successfully
- [ ] All shared types defined (Task, Workflow, ExecutionEvent)
- [ ] Status enums with conversions (to_i32, from_i32, from_str)
- [ ] Helper methods (is_terminal, duration, node_count, etc.)
- [ ] Error types defined with thiserror
- [ ] All tests pass
- [ ] Can be used by orchestrator, aco, and orca
- [ ] Serde serialization/deserialization works
- [ ] UUID generation for IDs

## Complexity
**Simple** - Straightforward data structures and enums

## Estimated Effort
**4-5 hours**

## Notes
- This crate has no async dependencies - pure data structures
- Keep types simple and focused on domain concepts
- Conversions to/from protobuf types will be in separate mappers
- chrono for timestamps, UUID for IDs
