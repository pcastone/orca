//! Domain models for Orca
//!
//! Defines domain models for LLM providers, prompts, bugs, permissions, and AST cache.

pub mod llm_provider;
pub mod prompt;
pub mod workflow_template;
pub mod bug;
pub mod tool_permission;
pub mod ast_cache;
pub mod project_rule;

pub use llm_provider::LlmProviderConfig;
pub use prompt::Prompt;
pub use workflow_template::WorkflowTemplate;
pub use bug::{Bug, BugStatus, BugPriority};
pub use tool_permission::{ToolPermission, PermissionLevel};
pub use ast_cache::AstCache;
pub use project_rule::ProjectRule;
