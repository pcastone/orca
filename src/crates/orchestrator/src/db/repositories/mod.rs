//! Repository pattern implementations for database access
//!
//! This module provides repository structs for managing database operations
//! on all core entities: tasks, workflows, tool executions, sessions, and configurations.

pub mod task_repo;
pub mod workflow_repo;
pub mod workflow_task_repo;
pub mod tool_execution_repo;
pub mod session_repo;
pub mod configuration_repo;
pub mod bug_repo;
pub mod prompt_history_repo;
pub mod checkpoint_repo;

// Re-export all repositories for convenient access
pub use task_repo::TaskRepository;
pub use workflow_repo::WorkflowRepository;
pub use workflow_task_repo::WorkflowTaskRepository;
pub use tool_execution_repo::ToolExecutionRepository;
pub use session_repo::SessionRepository;
pub use configuration_repo::ConfigurationRepository;
pub use bug_repo::BugRepository;
pub use prompt_history_repo::PromptHistoryRepository;
pub use checkpoint_repo::CheckpointRepository;
