//! # Orca - Standalone Orchestrator
//!
//! A simplified, standalone orchestrator for AI agent workflows that operates
//! without requiring a separate aco server. Orca uses direct tool execution
//! for a streamlined, single-process architecture.
//!
//! ## Features
//!
//! - **Standalone Operation** - No server dependency, runs as a single process
//! - **Direct Tool Execution** - Tools execute in-process without network overhead
//! - **SQLite Database** - Persistent state stored in `~/.orca/orca.db`
//! - **Dual-Location Config** - User-level and project-level configuration
//! - **Full CLI Interface** - Comprehensive command-line tools
//! - **LLM Integration** - Support for multiple LLM providers
//! - **Agent Patterns** - ReAct, Plan-Execute, Reflection patterns
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use orca::{DirectToolBridge, OrcaConfig};
//! use std::path::PathBuf;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Initialize direct tool bridge
//! let bridge = DirectToolBridge::new(
//!     PathBuf::from("."),
//!     "session-123".to_string(),
//! )?;
//!
//! // Execute a tool
//! let result = bridge.execute_tool(
//!     "file_read",
//!     serde_json::json!({"path": "README.md"}),
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! Orca is designed for local development and single-machine deployments where
//! simplicity is preferred over distributed architecture. It provides the full
//! power of the acolib orchestration system without the complexity of WebSocket
//! servers and network communication.

// Core modules
pub mod cli;
pub mod config;
pub mod context;
pub mod db;
pub mod events;
pub mod health;
pub mod init;
pub mod interpreter;
pub mod models;
pub mod pattern;
pub mod repositories;
pub mod router;
pub mod shutdown;
pub mod tools;
pub mod version;
pub mod workflow;

// Executor module
pub mod executor;

// Error types and utilities
mod error;

// Re-export key types for convenience
pub use tools::DirectToolBridge;
pub use workflow::{Task, TaskStatus, Workflow, WorkflowStatus};
pub use pattern::PatternType;
pub use executor::{TaskExecutor, ExecutionResult, LlmProvider, ToolAdapter};
pub use context::{ExecutionContext, ContextBuilder, SessionInfo};
pub use shutdown::ShutdownCoordinator;

// Error types
pub use error::{OrcaError, Result};

// Re-export version utilities
pub use version::{full_version as version_info, short_version, VersionInfo};

// Re-export database and config types
pub use db::{Database, manager::DatabaseManager};
pub use config::{OrcaConfig, ConfigLoader, load_config};

// Re-export repositories
pub use repositories::{
    TaskRepository, WorkflowRepository,
    LlmProviderRepository, PromptRepository, WorkflowTemplateRepository,
    BugRepository, ToolPermissionRepository, AstCacheRepository, ProjectRuleRepository,
};

// Re-export models
pub use models::{
    LlmProviderConfig, Prompt, WorkflowTemplate,
    Bug, BugStatus, BugPriority,
    ToolPermission, PermissionLevel,
    AstCache, ProjectRule,
};

// Re-export health types
pub use health::{HealthChecker, HealthReport, HealthStatus, ComponentHealth};

// Re-export event types
pub use events::{ExecutionEvent, EventLogger};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let info = version_info();
        assert!(info.contains("Orca"));
        assert!(info.contains(version::VERSION));
    }
}
