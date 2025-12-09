//! Database repositories
//!
//! Provides repository patterns for database operations on tasks, workflows,
//! LLM providers, prompts, bugs, permissions, AST cache, pattern configs, and execution metrics.

// Existing repositories
pub mod task_repository;
pub mod workflow_repository;

// User DB repositories
pub mod llm_provider_repository;
pub mod llm_profile_repository;
pub mod prompt_repository;
pub mod workflow_template_repository;
pub mod budget_repository;
pub mod pattern_config_repository;
pub mod execution_metrics_repository;
pub mod yaml_file_repository;

// Project DB repositories
pub mod bug_repository;
pub mod tool_permission_repository;
pub mod ast_cache_repository;
pub mod project_rule_repository;

// Re-exports
pub use task_repository::TaskRepository;
pub use workflow_repository::WorkflowRepository;
pub use llm_provider_repository::LlmProviderRepository;
pub use llm_profile_repository::LlmProfileRepository;
pub use prompt_repository::PromptRepository;
pub use workflow_template_repository::WorkflowTemplateRepository;
pub use budget_repository::BudgetRepository;
pub use pattern_config_repository::PatternConfigRepository;
pub use execution_metrics_repository::ExecutionMetricsRepository;
pub use bug_repository::BugRepository;
pub use tool_permission_repository::ToolPermissionRepository;
pub use ast_cache_repository::AstCacheRepository;
pub use project_rule_repository::ProjectRuleRepository;
pub use yaml_file_repository::{YamlFileRepository, YamlFileStats};
