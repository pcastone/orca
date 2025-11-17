//! Database repositories
//!
//! Provides repository patterns for database operations on tasks, workflows,
//! LLM providers, prompts, bugs, permissions, and AST cache.

// Existing repositories
pub mod task_repository;
pub mod workflow_repository;

// User DB repositories
pub mod llm_provider_repository;
pub mod prompt_repository;
pub mod workflow_template_repository;
pub mod budget_repository;
pub mod llm_profile_repository;

// Project DB repositories
pub mod bug_repository;
pub mod tool_permission_repository;
pub mod ast_cache_repository;
pub mod project_rule_repository;

// Re-exports
pub use task_repository::TaskRepository;
pub use workflow_repository::WorkflowRepository;
pub use llm_provider_repository::LlmProviderRepository;
pub use prompt_repository::PromptRepository;
pub use workflow_template_repository::WorkflowTemplateRepository;
pub use budget_repository::BudgetRepository;
pub use llm_profile_repository::LlmProfileRepository;
pub use bug_repository::BugRepository;
pub use tool_permission_repository::ToolPermissionRepository;
pub use ast_cache_repository::AstCacheRepository;
pub use project_rule_repository::ProjectRuleRepository;
