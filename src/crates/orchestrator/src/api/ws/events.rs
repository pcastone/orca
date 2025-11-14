//! Event definitions and serialization for real-time streaming
//!
//! Provides structured event types for all real-time communications.

use serde::{Deserialize, Serialize};

/// Event priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    /// Low priority (background info)
    Low = 0,
    /// Normal priority (standard updates)
    Normal = 1,
    /// High priority (important events)
    High = 2,
}

/// Real-time event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RealtimeEvent {
    /// Task progress update
    #[serde(rename = "task.progress")]
    TaskProgress {
        task_id: String,
        progress: u32,
        message: String,
    },

    /// Task status update
    #[serde(rename = "task.status")]
    TaskStatus {
        task_id: String,
        status: String,
        timestamp: String,
    },

    /// Task completed
    #[serde(rename = "task.completed")]
    TaskCompleted {
        task_id: String,
        result: Option<String>,
        timestamp: String,
    },

    /// Task failed
    #[serde(rename = "task.failed")]
    TaskFailed {
        task_id: String,
        error: String,
        timestamp: String,
    },

    /// Tool execution started
    #[serde(rename = "tool.started")]
    ToolStarted {
        execution_id: String,
        tool_name: String,
        timestamp: String,
    },

    /// Tool output received
    #[serde(rename = "tool.output")]
    ToolOutput {
        execution_id: String,
        output: String,
        is_stderr: bool,
    },

    /// Tool execution completed
    #[serde(rename = "tool.completed")]
    ToolCompleted {
        execution_id: String,
        status: String,
        exit_code: Option<i32>,
        timestamp: String,
    },

    /// Tool execution failed
    #[serde(rename = "tool.failed")]
    ToolFailed {
        execution_id: String,
        error: String,
        timestamp: String,
    },

    /// Workflow progress update
    #[serde(rename = "workflow.progress")]
    WorkflowProgress {
        workflow_id: String,
        current_task: String,
        progress: u32,
    },

    /// Workflow completed
    #[serde(rename = "workflow.completed")]
    WorkflowCompleted {
        workflow_id: String,
        timestamp: String,
    },

    /// Workflow failed
    #[serde(rename = "workflow.failed")]
    WorkflowFailed {
        workflow_id: String,
        error: String,
        timestamp: String,
    },

    /// Connection established
    #[serde(rename = "connection.established")]
    ConnectionEstablished {
        client_id: String,
        timestamp: String,
    },

    /// Heartbeat/ping
    #[serde(rename = "connection.heartbeat")]
    Heartbeat {
        timestamp: String,
    },

    /// Generic error event
    #[serde(rename = "error")]
    Error {
        message: String,
        code: Option<String>,
    },
}

impl RealtimeEvent {
    /// Get event type as string
    pub fn event_type(&self) -> &str {
        match self {
            RealtimeEvent::TaskProgress { .. } => "task.progress",
            RealtimeEvent::TaskStatus { .. } => "task.status",
            RealtimeEvent::TaskCompleted { .. } => "task.completed",
            RealtimeEvent::TaskFailed { .. } => "task.failed",
            RealtimeEvent::ToolStarted { .. } => "tool.started",
            RealtimeEvent::ToolOutput { .. } => "tool.output",
            RealtimeEvent::ToolCompleted { .. } => "tool.completed",
            RealtimeEvent::ToolFailed { .. } => "tool.failed",
            RealtimeEvent::WorkflowProgress { .. } => "workflow.progress",
            RealtimeEvent::WorkflowCompleted { .. } => "workflow.completed",
            RealtimeEvent::WorkflowFailed { .. } => "workflow.failed",
            RealtimeEvent::ConnectionEstablished { .. } => "connection.established",
            RealtimeEvent::Heartbeat { .. } => "connection.heartbeat",
            RealtimeEvent::Error { .. } => "error",
        }
    }

    /// Get event priority
    pub fn priority(&self) -> EventPriority {
        match self {
            RealtimeEvent::TaskFailed { .. }
            | RealtimeEvent::WorkflowFailed { .. }
            | RealtimeEvent::ToolFailed { .. }
            | RealtimeEvent::Error { .. } => EventPriority::High,

            RealtimeEvent::TaskCompleted { .. }
            | RealtimeEvent::TaskStatus { .. }
            | RealtimeEvent::ToolCompleted { .. }
            | RealtimeEvent::ToolStarted { .. }
            | RealtimeEvent::WorkflowCompleted { .. } => EventPriority::Normal,

            RealtimeEvent::TaskProgress { .. }
            | RealtimeEvent::ToolOutput { .. }
            | RealtimeEvent::WorkflowProgress { .. }
            | RealtimeEvent::Heartbeat { .. }
            | RealtimeEvent::ConnectionEstablished { .. } => EventPriority::Low,
        }
    }

    /// Extract task ID if present
    pub fn task_id(&self) -> Option<&str> {
        match self {
            RealtimeEvent::TaskProgress { task_id, .. }
            | RealtimeEvent::TaskStatus { task_id, .. }
            | RealtimeEvent::TaskCompleted { task_id, .. }
            | RealtimeEvent::TaskFailed { task_id, .. } => Some(task_id),
            _ => None,
        }
    }

    /// Extract execution ID if present
    pub fn execution_id(&self) -> Option<&str> {
        match self {
            RealtimeEvent::ToolStarted { execution_id, .. }
            | RealtimeEvent::ToolOutput { execution_id, .. }
            | RealtimeEvent::ToolCompleted { execution_id, .. }
            | RealtimeEvent::ToolFailed { execution_id, .. } => Some(execution_id),
            _ => None,
        }
    }

    /// Extract workflow ID if present
    pub fn workflow_id(&self) -> Option<&str> {
        match self {
            RealtimeEvent::WorkflowProgress { workflow_id, .. }
            | RealtimeEvent::WorkflowCompleted { workflow_id, .. }
            | RealtimeEvent::WorkflowFailed { workflow_id, .. } => Some(workflow_id),
            _ => None,
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Convert to JSON value
    pub fn to_json_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_progress_event() {
        let event = RealtimeEvent::TaskProgress {
            task_id: "task1".to_string(),
            progress: 50,
            message: "Processing".to_string(),
        };
        assert_eq!(event.event_type(), "task.progress");
        assert_eq!(event.task_id(), Some("task1"));
        assert_eq!(event.priority(), EventPriority::Low);
    }

    #[test]
    fn test_task_failed_event() {
        let event = RealtimeEvent::TaskFailed {
            task_id: "task1".to_string(),
            error: "Error message".to_string(),
            timestamp: "2025-11-10T00:00:00Z".to_string(),
        };
        assert_eq!(event.event_type(), "task.failed");
        assert_eq!(event.priority(), EventPriority::High);
    }

    #[test]
    fn test_tool_output_event() {
        let event = RealtimeEvent::ToolOutput {
            execution_id: "exec1".to_string(),
            output: "Output text".to_string(),
            is_stderr: false,
        };
        assert_eq!(event.event_type(), "tool.output");
        assert_eq!(event.execution_id(), Some("exec1"));
    }

    #[test]
    fn test_workflow_event() {
        let event = RealtimeEvent::WorkflowProgress {
            workflow_id: "wf1".to_string(),
            current_task: "task1".to_string(),
            progress: 25,
        };
        assert_eq!(event.workflow_id(), Some("wf1"));
    }

    #[test]
    fn test_event_serialization() {
        let event = RealtimeEvent::TaskProgress {
            task_id: "task1".to_string(),
            progress: 50,
            message: "Processing".to_string(),
        };
        let json = event.to_json().unwrap();
        assert!(json.contains("task.progress"));
        assert!(json.contains("task1"));
    }

    #[test]
    fn test_error_event() {
        let event = RealtimeEvent::Error {
            message: "Error occurred".to_string(),
            code: Some("ERROR_CODE".to_string()),
        };
        assert_eq!(event.priority(), EventPriority::High);
    }
}
