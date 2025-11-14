//! Orchestration engine for coordinating workflows in acolib
//!
//! This crate provides orchestration capabilities for managing and coordinating
//! complex workflows, tasks, and execution pipelines.

pub mod api;
pub mod client;
pub mod config;
pub mod context;
pub mod db;
pub mod executor;
pub mod integration;
pub mod interpreter;
pub mod pattern;
pub mod router;
pub mod version;
pub mod workflow;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during orchestration
#[derive(Debug, Error)]
pub enum OrchestratorError {
    /// Task not found
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    /// Workflow execution error
    #[error("Workflow execution failed: {0}")]
    ExecutionFailed(String),

    /// Invalid state transition
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    /// General error
    #[error("Orchestrator error: {0}")]
    General(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for orchestrator operations
pub type Result<T> = std::result::Result<T, OrchestratorError>;

/// Task execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,
    /// Task is currently running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::Running => write!(f, "Running"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Failed => write!(f, "Failed"),
            TaskStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// A task in the orchestration workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: Uuid,
    /// Task name
    pub name: String,
    /// Task description
    pub description: Option<String>,
    /// Current status
    pub status: TaskStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Task metadata
    pub metadata: HashMap<String, String>,
}

impl Task {
    /// Create a new task
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Set task description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add metadata to task
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Update task status
    pub fn update_status(&mut self, status: TaskStatus) -> Result<()> {
        // Validate state transition
        match (&self.status, &status) {
            (TaskStatus::Completed, _) | (TaskStatus::Failed, _) | (TaskStatus::Cancelled, _) => {
                return Err(OrchestratorError::InvalidStateTransition {
                    from: self.status.to_string(),
                    to: status.to_string(),
                });
            }
            _ => {}
        }

        self.status = status;
        self.updated_at = Utc::now();
        Ok(())
    }
}

/// Trait for executing tasks
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task
    async fn execute(&self, task: &Task) -> Result<()>;
}

/// Workflow orchestrator
#[derive(Debug)]
pub struct Orchestrator {
    /// Active tasks
    tasks: HashMap<Uuid, Task>,
    /// Orchestrator configuration
    config: OrchestratorConfig,
}

/// Configuration for the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 10,
            verbose: false,
        }
    }
}

impl OrchestratorConfig {
    /// Create a new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum concurrent tasks
    pub fn with_max_concurrent_tasks(mut self, max: usize) -> Self {
        self.max_concurrent_tasks = max;
        self
    }

    /// Set verbose mode
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

impl Orchestrator {
    /// Create a new orchestrator with default configuration
    pub fn new() -> Self {
        Self::with_config(OrchestratorConfig::default())
    }

    /// Create a new orchestrator with custom configuration
    pub fn with_config(config: OrchestratorConfig) -> Self {
        Self {
            tasks: HashMap::new(),
            config,
        }
    }

    /// Add a task to the orchestrator
    pub fn add_task(&mut self, task: Task) -> Uuid {
        let id = task.id;
        self.tasks.insert(id, task);
        tracing::debug!("Added task {}", id);
        id
    }

    /// Get a task by ID
    pub fn get_task(&self, id: &Uuid) -> Option<&Task> {
        self.tasks.get(id)
    }

    /// Get a mutable task by ID
    pub fn get_task_mut(&mut self, id: &Uuid) -> Option<&mut Task> {
        self.tasks.get_mut(id)
    }

    /// Remove a task
    pub fn remove_task(&mut self, id: &Uuid) -> Result<Task> {
        self.tasks
            .remove(id)
            .ok_or_else(|| OrchestratorError::TaskNotFound(id.to_string()))
    }

    /// Get all tasks
    pub fn tasks(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }

    /// Get tasks by status
    pub fn tasks_by_status(&self, status: TaskStatus) -> impl Iterator<Item = &Task> {
        self.tasks.values().filter(move |task| task.status == status)
    }

    /// Get running task count
    pub fn running_count(&self) -> usize {
        self.tasks_by_status(TaskStatus::Running).count()
    }

    /// Check if orchestrator can accept more tasks
    pub fn can_accept_task(&self) -> bool {
        self.running_count() < self.config.max_concurrent_tasks
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Get version information
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("test_task")
            .with_description("A test task")
            .with_metadata("key", "value");

        assert_eq!(task.name, "test_task");
        assert_eq!(task.description, Some("A test task".to_string()));
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_task_status_update() {
        let mut task = Task::new("test");

        assert!(task.update_status(TaskStatus::Running).is_ok());
        assert_eq!(task.status, TaskStatus::Running);

        assert!(task.update_status(TaskStatus::Completed).is_ok());
        assert_eq!(task.status, TaskStatus::Completed);

        // Cannot update from completed
        assert!(task.update_status(TaskStatus::Running).is_err());
    }

    #[test]
    fn test_orchestrator_add_task() {
        let mut orchestrator = Orchestrator::new();
        let task = Task::new("test");
        let id = orchestrator.add_task(task);

        assert!(orchestrator.get_task(&id).is_some());
        assert_eq!(orchestrator.tasks().count(), 1);
    }

    #[test]
    fn test_orchestrator_remove_task() {
        let mut orchestrator = Orchestrator::new();
        let task = Task::new("test");
        let id = orchestrator.add_task(task);

        assert!(orchestrator.remove_task(&id).is_ok());
        assert!(orchestrator.get_task(&id).is_none());
    }

    #[test]
    fn test_orchestrator_tasks_by_status() {
        let mut orchestrator = Orchestrator::new();

        let mut task1 = Task::new("task1");
        task1.update_status(TaskStatus::Running).unwrap();
        orchestrator.add_task(task1);

        let task2 = Task::new("task2");
        orchestrator.add_task(task2);

        assert_eq!(orchestrator.tasks_by_status(TaskStatus::Running).count(), 1);
        assert_eq!(orchestrator.tasks_by_status(TaskStatus::Pending).count(), 1);
    }

    #[test]
    fn test_orchestrator_config() {
        let config = OrchestratorConfig::new()
            .with_max_concurrent_tasks(5)
            .with_verbose(true);

        assert_eq!(config.max_concurrent_tasks, 5);
        assert!(config.verbose);
    }

    #[test]
    fn test_orchestrator_can_accept_task() {
        let config = OrchestratorConfig::new().with_max_concurrent_tasks(2);
        let mut orchestrator = Orchestrator::with_config(config);

        assert!(orchestrator.can_accept_task());

        let mut task1 = Task::new("task1");
        task1.update_status(TaskStatus::Running).unwrap();
        orchestrator.add_task(task1);

        assert!(orchestrator.can_accept_task());

        let mut task2 = Task::new("task2");
        task2.update_status(TaskStatus::Running).unwrap();
        orchestrator.add_task(task2);

        assert!(!orchestrator.can_accept_task());
    }

    #[test]
    fn test_version() {
        let v = version();
        assert!(!v.is_empty());
    }
}
