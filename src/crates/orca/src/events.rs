//! Execution event logging for observability
//!
//! This module provides event types and logging infrastructure for tracking
//! task and workflow execution events.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Event types for execution tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionEvent {
    /// Task execution started
    TaskStarted {
        task_id: String,
        description: String,
        timestamp: i64,
    },
    /// Task execution completed successfully
    TaskCompleted {
        task_id: String,
        result: Option<String>,
        timestamp: i64,
        duration_ms: u64,
    },
    /// Task execution failed
    TaskFailed {
        task_id: String,
        error: String,
        timestamp: i64,
        duration_ms: u64,
    },
    /// Task was cancelled
    TaskCancelled {
        task_id: String,
        timestamp: i64,
    },
    /// Workflow execution started
    WorkflowStarted {
        workflow_id: String,
        name: String,
        task_count: usize,
        timestamp: i64,
    },
    /// Workflow execution completed
    WorkflowCompleted {
        workflow_id: String,
        completed_tasks: usize,
        failed_tasks: usize,
        timestamp: i64,
        duration_ms: u64,
    },
    /// Workflow was paused
    WorkflowPaused {
        workflow_id: String,
        timestamp: i64,
    },
    /// Workflow was resumed
    WorkflowResumed {
        workflow_id: String,
        timestamp: i64,
    },
    /// Pattern execution started
    PatternExecutionStarted {
        task_id: String,
        pattern: String,
        timestamp: i64,
    },
    /// Pattern execution completed
    PatternExecutionCompleted {
        task_id: String,
        pattern: String,
        iterations: usize,
        timestamp: i64,
        duration_ms: u64,
    },
}

impl ExecutionEvent {
    /// Get the timestamp of the event
    pub fn timestamp(&self) -> i64 {
        match self {
            ExecutionEvent::TaskStarted { timestamp, .. }
            | ExecutionEvent::TaskCompleted { timestamp, .. }
            | ExecutionEvent::TaskFailed { timestamp, .. }
            | ExecutionEvent::TaskCancelled { timestamp, .. }
            | ExecutionEvent::WorkflowStarted { timestamp, .. }
            | ExecutionEvent::WorkflowCompleted { timestamp, .. }
            | ExecutionEvent::WorkflowPaused { timestamp, .. }
            | ExecutionEvent::WorkflowResumed { timestamp, .. }
            | ExecutionEvent::PatternExecutionStarted { timestamp, .. }
            | ExecutionEvent::PatternExecutionCompleted { timestamp, .. } => *timestamp,
        }
    }

    /// Get a human-readable description of the event
    pub fn description(&self) -> String {
        match self {
            ExecutionEvent::TaskStarted { task_id, description, .. } => {
                format!("Task started: {} ({})", task_id, description)
            }
            ExecutionEvent::TaskCompleted { duration_ms, .. } => {
                format!("Task completed ({}ms)", duration_ms)
            }
            ExecutionEvent::TaskFailed { task_id, error, .. } => {
                format!("Task failed: {} - {}", task_id, error)
            }
            ExecutionEvent::TaskCancelled { task_id, .. } => {
                format!("Task cancelled: {}", task_id)
            }
            ExecutionEvent::WorkflowStarted { name, task_count, .. } => {
                format!("Workflow started: {} ({} tasks)", name, task_count)
            }
            ExecutionEvent::WorkflowCompleted { completed_tasks, failed_tasks, .. } => {
                format!("Workflow completed: {} succeeded, {} failed", completed_tasks, failed_tasks)
            }
            ExecutionEvent::WorkflowPaused { workflow_id, .. } => {
                format!("Workflow paused: {}", workflow_id)
            }
            ExecutionEvent::WorkflowResumed { workflow_id, .. } => {
                format!("Workflow resumed: {}", workflow_id)
            }
            ExecutionEvent::PatternExecutionStarted { task_id, pattern, .. } => {
                format!("Pattern execution started: {} ({})", task_id, pattern)
            }
            ExecutionEvent::PatternExecutionCompleted { task_id, pattern, iterations, duration_ms, .. } => {
                format!("Pattern execution completed: {} ({}) - {} iterations in {}ms",
                    task_id, pattern, iterations, duration_ms)
            }
        }
    }

    /// Create a TaskStarted event
    pub fn task_started(task_id: impl Into<String>, description: impl Into<String>) -> Self {
        ExecutionEvent::TaskStarted {
            task_id: task_id.into(),
            description: description.into(),
            timestamp: Utc::now().timestamp(),
        }
    }

    /// Create a TaskCompleted event
    pub fn task_completed(
        task_id: impl Into<String>,
        result: Option<String>,
        duration_ms: u64,
    ) -> Self {
        ExecutionEvent::TaskCompleted {
            task_id: task_id.into(),
            result,
            timestamp: Utc::now().timestamp(),
            duration_ms,
        }
    }

    /// Create a TaskFailed event
    pub fn task_failed(
        task_id: impl Into<String>,
        error: impl Into<String>,
        duration_ms: u64,
    ) -> Self {
        ExecutionEvent::TaskFailed {
            task_id: task_id.into(),
            error: error.into(),
            timestamp: Utc::now().timestamp(),
            duration_ms,
        }
    }

    /// Create a TaskCancelled event
    pub fn task_cancelled(task_id: impl Into<String>) -> Self {
        ExecutionEvent::TaskCancelled {
            task_id: task_id.into(),
            timestamp: Utc::now().timestamp(),
        }
    }

    /// Create a WorkflowStarted event
    pub fn workflow_started(
        workflow_id: impl Into<String>,
        name: impl Into<String>,
        task_count: usize,
    ) -> Self {
        ExecutionEvent::WorkflowStarted {
            workflow_id: workflow_id.into(),
            name: name.into(),
            task_count,
            timestamp: Utc::now().timestamp(),
        }
    }

    /// Create a WorkflowCompleted event
    pub fn workflow_completed(
        workflow_id: impl Into<String>,
        completed_tasks: usize,
        failed_tasks: usize,
        duration_ms: u64,
    ) -> Self {
        ExecutionEvent::WorkflowCompleted {
            workflow_id: workflow_id.into(),
            completed_tasks,
            failed_tasks,
            timestamp: Utc::now().timestamp(),
            duration_ms,
        }
    }

    /// Create a WorkflowPaused event
    pub fn workflow_paused(workflow_id: impl Into<String>) -> Self {
        ExecutionEvent::WorkflowPaused {
            workflow_id: workflow_id.into(),
            timestamp: Utc::now().timestamp(),
        }
    }

    /// Create a WorkflowResumed event
    pub fn workflow_resumed(workflow_id: impl Into<String>) -> Self {
        ExecutionEvent::WorkflowResumed {
            workflow_id: workflow_id.into(),
            timestamp: Utc::now().timestamp(),
        }
    }

    /// Create a PatternExecutionStarted event
    pub fn pattern_execution_started(
        task_id: impl Into<String>,
        pattern: impl Into<String>,
    ) -> Self {
        ExecutionEvent::PatternExecutionStarted {
            task_id: task_id.into(),
            pattern: pattern.into(),
            timestamp: Utc::now().timestamp(),
        }
    }

    /// Create a PatternExecutionCompleted event
    pub fn pattern_execution_completed(
        task_id: impl Into<String>,
        pattern: impl Into<String>,
        iterations: usize,
        duration_ms: u64,
    ) -> Self {
        ExecutionEvent::PatternExecutionCompleted {
            task_id: task_id.into(),
            pattern: pattern.into(),
            iterations,
            timestamp: Utc::now().timestamp(),
            duration_ms,
        }
    }
}

/// Event logger for recording execution events
#[derive(Debug, Clone)]
pub struct EventLogger {
    enabled: bool,
}

impl EventLogger {
    /// Create a new event logger
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Log an execution event
    pub fn log(&self, event: &ExecutionEvent) {
        if !self.enabled {
            return;
        }

        // Log using tracing infrastructure
        tracing::info!(
            event_type = ?event,
            timestamp = event.timestamp(),
            description = %event.description(),
            "Execution event"
        );
    }

    /// Check if logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for EventLogger {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_started_event() {
        let event = ExecutionEvent::task_started("task-123", "Test task");

        match &event {
            ExecutionEvent::TaskStarted { task_id, description, .. } => {
                assert_eq!(task_id, "task-123");
                assert_eq!(description, "Test task");
            }
            _ => panic!("Expected TaskStarted event"),
        }

        assert!(event.description().contains("task-123"));
        assert!(event.description().contains("Test task"));
    }

    #[test]
    fn test_task_completed_event() {
        let event = ExecutionEvent::task_completed("task-123", Some("Success".to_string()), 1500);

        match &event {
            ExecutionEvent::TaskCompleted { task_id, result, duration_ms, .. } => {
                assert_eq!(task_id, "task-123");
                assert_eq!(result, &Some("Success".to_string()));
                assert_eq!(*duration_ms, 1500);
            }
            _ => panic!("Expected TaskCompleted event"),
        }

        let desc = event.description();
        assert!(desc.contains("task-123"));
        assert!(desc.contains("1500ms"));
    }

    #[test]
    fn test_task_failed_event() {
        let event = ExecutionEvent::task_failed("task-123", "Connection timeout", 3000);

        match &event {
            ExecutionEvent::TaskFailed { task_id, error, duration_ms, .. } => {
                assert_eq!(task_id, "task-123");
                assert_eq!(error, "Connection timeout");
                assert_eq!(*duration_ms, 3000);
            }
            _ => panic!("Expected TaskFailed event"),
        }

        let desc = event.description();
        assert!(desc.contains("task-123"));
        assert!(desc.contains("Connection timeout"));
    }

    #[test]
    fn test_workflow_started_event() {
        let event = ExecutionEvent::workflow_started("wf-456", "Test Workflow", 5);

        match &event {
            ExecutionEvent::WorkflowStarted { workflow_id, name, task_count, .. } => {
                assert_eq!(workflow_id, "wf-456");
                assert_eq!(name, "Test Workflow");
                assert_eq!(*task_count, 5);
            }
            _ => panic!("Expected WorkflowStarted event"),
        }

        let desc = event.description();
        assert!(desc.contains("Test Workflow"));
        assert!(desc.contains("5 tasks"));
    }

    #[test]
    fn test_workflow_completed_event() {
        let event = ExecutionEvent::workflow_completed("wf-456", 4, 1, 5000);

        match &event {
            ExecutionEvent::WorkflowCompleted { workflow_id, completed_tasks, failed_tasks, duration_ms, .. } => {
                assert_eq!(workflow_id, "wf-456");
                assert_eq!(*completed_tasks, 4);
                assert_eq!(*failed_tasks, 1);
                assert_eq!(*duration_ms, 5000);
            }
            _ => panic!("Expected WorkflowCompleted event"),
        }

        let desc = event.description();
        assert!(desc.contains("4 succeeded"));
        assert!(desc.contains("1 failed"));
    }

    #[test]
    fn test_pattern_execution_events() {
        let start_event = ExecutionEvent::pattern_execution_started("task-789", "react");
        let complete_event = ExecutionEvent::pattern_execution_completed("task-789", "react", 3, 2500);

        match &start_event {
            ExecutionEvent::PatternExecutionStarted { task_id, pattern, .. } => {
                assert_eq!(task_id, "task-789");
                assert_eq!(pattern, "react");
            }
            _ => panic!("Expected PatternExecutionStarted event"),
        }

        match &complete_event {
            ExecutionEvent::PatternExecutionCompleted { task_id, pattern, iterations, duration_ms, .. } => {
                assert_eq!(task_id, "task-789");
                assert_eq!(pattern, "react");
                assert_eq!(*iterations, 3);
                assert_eq!(*duration_ms, 2500);
            }
            _ => panic!("Expected PatternExecutionCompleted event"),
        }
    }

    #[test]
    fn test_event_timestamp() {
        let event = ExecutionEvent::task_started("task-123", "Test");
        let timestamp = event.timestamp();

        // Timestamp should be close to current time
        let now = Utc::now().timestamp();
        assert!((timestamp - now).abs() <= 1);
    }

    #[test]
    fn test_event_logger() {
        let logger = EventLogger::new(true);
        assert!(logger.is_enabled());

        let event = ExecutionEvent::task_started("task-123", "Test task");
        logger.log(&event); // Should not panic

        let disabled_logger = EventLogger::new(false);
        assert!(!disabled_logger.is_enabled());
        disabled_logger.log(&event); // Should be no-op
    }

    #[test]
    fn test_event_logger_default() {
        let logger = EventLogger::default();
        assert!(logger.is_enabled());
    }

    #[test]
    fn test_event_serialization() {
        let event = ExecutionEvent::task_started("task-123", "Test task");

        // Serialize to JSON
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("task_started"));
        assert!(json.contains("task-123"));

        // Deserialize back
        let deserialized: ExecutionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, event);
    }
}
