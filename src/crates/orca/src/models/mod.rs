//! Domain models for Orca
//!
//! Defines domain models for LLM providers, prompts, bugs, permissions, AST cache, budgets, and LLM profiles.

pub mod ast_cache;
pub mod bug;
pub mod budget;
pub mod llm_profile;
pub mod llm_provider;
pub mod pricing;
pub mod project_rule;
pub mod prompt;
pub mod tool_permission;
pub mod workflow_template;

pub use ast_cache::AstCache;
pub use budget::{Budget, BudgetEnforcement, BudgetType, RenewalInterval};
pub use bug::{Bug, BugPriority, BugStatus};
pub use llm_profile::{LlmConfig, LlmProfile};
pub use llm_provider::LlmProviderConfig;
pub use pricing::{default_pricing, LlmPricing};
pub use project_rule::ProjectRule;
pub use prompt::Prompt;
pub use tool_permission::{PermissionLevel, ToolPermission};
pub use workflow_template::WorkflowTemplate;
