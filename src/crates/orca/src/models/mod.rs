//! Domain models for Orca
//!
//! Defines domain models for LLM providers, prompts, bugs, permissions, AST cache, budgets, pattern configs, and execution metrics.

pub mod ast_cache;
pub mod bug;
pub mod budget;
pub mod execution_metrics;
pub mod llm_profile;
pub mod llm_provider;
pub mod pattern_config;
pub mod pricing;
pub mod project_rule;
pub mod prompt;
pub mod tool_permission;
pub mod workflow_template;
pub mod yaml_file;

pub use ast_cache::AstCache;
pub use budget::{Budget, BudgetEnforcement, BudgetType, RenewalInterval};
pub use bug::{Bug, BugPriority, BugStats, BugStatus};
pub use execution_metrics::{
    AggregatedMetrics, ExecutionDetails, ExecutionIteration, ExecutionSummary,
    IterationMetrics, LlmCall, LlmMetrics, PromptExecution,
};
pub use llm_profile::LlmProfile;
pub use llm_provider::LlmProviderConfig;
pub use pattern_config::{PatternConfig, PatternType};
pub use pricing::{default_pricing, LlmPricing};
pub use project_rule::ProjectRule;
pub use prompt::Prompt;
pub use tool_permission::{PermissionLevel, ToolPermission};
pub use workflow_template::WorkflowTemplate;
pub use yaml_file::{YamlFile, YamlFileType, SyncStatus};
