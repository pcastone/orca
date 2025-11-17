//! Configuration schema for Orca standalone orchestrator

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main Orca configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrcaConfig {
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,

    /// LLM configuration
    #[serde(default)]
    pub llm: LlmConfig,

    /// Execution configuration
    #[serde(default)]
    pub execution: ExecutionConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Budget configuration
    #[serde(default)]
    pub budget: BudgetConfig,

    /// Workflow configuration
    #[serde(default)]
    pub workflow: WorkflowConfig,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database file path (relative to ~/.orca or absolute)
    pub path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "orca.db".to_string(),
        }
    }
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// LLM provider: "openai", "anthropic", "gemini", "ollama", etc.
    pub provider: String,

    /// Model name
    pub model: String,

    /// API key (supports environment variable interpolation)
    pub api_key: Option<String>,

    /// Temperature for generation (0.0-1.0)
    pub temperature: f32,

    /// Maximum tokens to generate
    pub max_tokens: u32,

    /// API base URL (for custom endpoints)
    pub api_base: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            model: "claude-3-sonnet".to_string(),
            api_key: None,
            temperature: 0.7,
            max_tokens: 4096,
            api_base: None,
        }
    }
}

/// Execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,

    /// Task timeout in seconds
    pub task_timeout: u64,

    /// Enable streaming output
    pub streaming: bool,

    /// Workspace root directory (defaults to current directory)
    pub workspace_root: Option<PathBuf>,

    /// Maximum agent iterations (for ReAct, Plan-Execute, etc.)
    pub max_iterations: usize,

    /// Default agent pattern to use: "react", "plan_execute", "reflection"
    /// Can be overridden per-task via metadata
    #[serde(default = "default_pattern")]
    pub default_pattern: String,

    /// Quality threshold for Reflection pattern (0.0-1.0)
    /// Higher values require more iterations to meet quality standards
    #[serde(default = "default_quality_threshold")]
    pub reflection_quality_threshold: f64,

    /// Maximum planning steps for Plan-Execute pattern
    #[serde(default = "default_max_plan_steps")]
    pub plan_execute_max_steps: usize,

    /// Enable automatic retry on task failure
    #[serde(default = "default_retry_enabled")]
    pub retry_enabled: bool,

    /// Maximum number of retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,

    /// Initial retry delay in seconds
    #[serde(default = "default_initial_retry_delay")]
    pub initial_retry_delay_secs: u64,

    /// Maximum retry delay in seconds
    #[serde(default = "default_max_retry_delay")]
    pub max_retry_delay_secs: u64,

    /// Retry backoff multiplier
    #[serde(default = "default_retry_multiplier")]
    pub retry_multiplier: f64,
}

fn default_pattern() -> String {
    "react".to_string()
}

fn default_quality_threshold() -> f64 {
    0.75
}

fn default_max_plan_steps() -> usize {
    10
}

fn default_retry_enabled() -> bool {
    false
}

fn default_max_retries() -> usize {
    3
}

fn default_initial_retry_delay() -> u64 {
    1
}

fn default_max_retry_delay() -> u64 {
    60
}

fn default_retry_multiplier() -> f64 {
    2.0
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 5,
            task_timeout: 300,
            streaming: true,
            workspace_root: None,
            max_iterations: 10,
            default_pattern: default_pattern(),
            reflection_quality_threshold: default_quality_threshold(),
            plan_execute_max_steps: default_max_plan_steps(),
            retry_enabled: default_retry_enabled(),
            max_retries: default_max_retries(),
            initial_retry_delay_secs: default_initial_retry_delay(),
            max_retry_delay_secs: default_max_retry_delay(),
            retry_multiplier: default_retry_multiplier(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level: "trace", "debug", "info", "warn", "error"
    pub level: String,

    /// Log format: "compact", "pretty", "json"
    pub format: String,

    /// Enable colored output
    pub colored: bool,

    /// Show timestamps
    pub timestamps: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "compact".to_string(),
            colored: true,
            timestamps: true,
        }
    }
}

/// Budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Default budget to use for workflows (by name)
    pub default_budget: Option<String>,

    /// Enable automatic budget enforcement during execution
    pub enforce_budgets: bool,

    /// Log budget usage details
    pub log_usage: bool,

    /// Alert threshold percentage (0.0-100.0) for budget usage
    pub alert_threshold: f64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            default_budget: None,
            enforce_budgets: true,
            log_usage: true,
            alert_threshold: 80.0,
        }
    }
}

/// Workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Default LLM profile to use for workflows (by name)
    pub default_llm_profile: Option<String>,

    /// Default planner LLM (format: provider:model)
    pub default_planner_llm: Option<String>,

    /// Default worker LLM (format: provider:model)
    pub default_worker_llm: Option<String>,

    /// Enable workflow caching
    pub enable_caching: bool,

    /// Cache time-to-live in seconds
    pub cache_ttl_secs: u64,

    /// Maximum workflow execution duration in seconds
    pub max_duration_secs: u64,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            default_llm_profile: None,
            default_planner_llm: None,
            default_worker_llm: None,
            enable_caching: false,
            cache_ttl_secs: 3600,
            max_duration_secs: 3600,
        }
    }
}

impl OrcaConfig {
    /// Merge another config into this one (other takes precedence)
    ///
    /// The loader handles priority: defaults → user → project
    pub fn merge(&mut self, other: OrcaConfig) {
        // Simple field replacement - serde fills in defaults for missing fields
        self.database = other.database;
        self.llm = other.llm;
        self.execution = other.execution;
        self.logging = other.logging;
        self.budget = other.budget;
        self.workflow = other.workflow;
    }

    /// Resolve environment variables in configuration values
    ///
    /// Supports ${VAR_NAME} syntax in string fields
    pub fn resolve_env_vars(&mut self) {
        // Resolve API key environment variable
        if let Some(ref api_key) = self.llm.api_key {
            self.llm.api_key = Some(Self::expand_env_var(api_key));
        }

        // Resolve API base URL if present
        if let Some(ref api_base) = self.llm.api_base {
            self.llm.api_base = Some(Self::expand_env_var(api_base));
        }
    }

    /// Expand environment variable in a string
    ///
    /// Supports ${VAR_NAME} syntax
    fn expand_env_var(value: &str) -> String {
        if value.starts_with("${") && value.ends_with('}') {
            let var_name = &value[2..value.len() - 1];
            std::env::var(var_name).unwrap_or_else(|_| value.to_string())
        } else {
            value.to_string()
        }
    }

    /// Get the resolved database path
    ///
    /// If path is relative, resolves it relative to ~/.orca
    pub fn database_path(&self) -> PathBuf {
        let path = PathBuf::from(&self.database.path);

        if path.is_absolute() {
            path
        } else {
            // Resolve relative to ~/.orca
            dirs::home_dir()
                .expect("Failed to get home directory")
                .join(".orca")
                .join(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OrcaConfig::default();
        assert_eq!(config.database.path, "orca.db");
        assert_eq!(config.llm.provider, "anthropic");
        assert_eq!(config.llm.model, "claude-3-sonnet");
        assert_eq!(config.execution.max_concurrent_tasks, 5);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_merge_config() {
        let mut base = OrcaConfig::default();
        let mut override_config = OrcaConfig::default();
        override_config.llm.model = "claude-3-opus".to_string();
        override_config.execution.max_concurrent_tasks = 10;

        base.merge(override_config);

        assert_eq!(base.llm.model, "claude-3-opus");
        assert_eq!(base.execution.max_concurrent_tasks, 10);
        assert_eq!(base.llm.provider, "anthropic"); // Unchanged
    }

    #[test]
    fn test_env_var_expansion() {
        let mut config = OrcaConfig::default();
        config.llm.api_key = Some("${TEST_API_KEY}".to_string());

        std::env::set_var("TEST_API_KEY", "test-key-123");
        config.resolve_env_vars();

        assert_eq!(config.llm.api_key, Some("test-key-123".to_string()));

        std::env::remove_var("TEST_API_KEY");
    }

    #[test]
    fn test_database_path_relative() {
        let config = OrcaConfig::default();
        let path = config.database_path();

        assert!(path.to_string_lossy().contains(".orca"));
        assert!(path.to_string_lossy().contains("orca.db"));
    }

    #[test]
    fn test_database_path_absolute() {
        let mut config = OrcaConfig::default();
        config.database.path = "/tmp/test.db".to_string();

        let path = config.database_path();
        assert_eq!(path, PathBuf::from("/tmp/test.db"));
    }

    #[test]
    fn test_default_pattern_configuration() {
        let config = ExecutionConfig::default();

        // Verify default pattern is react
        assert_eq!(config.default_pattern, "react");

        // Verify pattern-specific defaults
        assert_eq!(config.reflection_quality_threshold, 0.75);
        assert_eq!(config.plan_execute_max_steps, 10);
    }

    #[test]
    fn test_pattern_config_deserializes() {
        let toml = r#"
            max_concurrent_tasks = 5
            task_timeout = 300
            streaming = true
            max_iterations = 10
            default_pattern = "plan_execute"
            reflection_quality_threshold = 0.85
            plan_execute_max_steps = 15
        "#;

        let config: ExecutionConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.default_pattern, "plan_execute");
        assert_eq!(config.reflection_quality_threshold, 0.85);
        assert_eq!(config.plan_execute_max_steps, 15);
    }

    #[test]
    fn test_pattern_config_missing_fields_use_defaults() {
        let toml = r#"
            max_concurrent_tasks = 5
            task_timeout = 300
            streaming = false
            max_iterations = 10
        "#;

        let config: ExecutionConfig = toml::from_str(toml).unwrap();

        // Missing fields should use defaults
        assert_eq!(config.default_pattern, "react");
        assert_eq!(config.reflection_quality_threshold, 0.75);
        assert_eq!(config.plan_execute_max_steps, 10);
    }

    #[test]
    fn test_pattern_config_validation() {
        let config = ExecutionConfig::default();

        // Verify quality threshold is in valid range
        assert!(config.reflection_quality_threshold >= 0.0);
        assert!(config.reflection_quality_threshold <= 1.0);

        // Verify max_plan_steps is reasonable
        assert!(config.plan_execute_max_steps > 0);
        assert!(config.plan_execute_max_steps <= 100);
    }

    #[test]
    fn test_retry_config_defaults() {
        let config = ExecutionConfig::default();

        // Verify retry is disabled by default
        assert!(!config.retry_enabled);

        // Verify sensible retry defaults
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_retry_delay_secs, 1);
        assert_eq!(config.max_retry_delay_secs, 60);
        assert_eq!(config.retry_multiplier, 2.0);
    }

    #[test]
    fn test_retry_config_deserializes() {
        let toml = r#"
            max_concurrent_tasks = 5
            task_timeout = 300
            streaming = true
            max_iterations = 10
            retry_enabled = true
            max_retries = 5
            initial_retry_delay_secs = 2
            max_retry_delay_secs = 120
            retry_multiplier = 3.0
        "#;

        let config: ExecutionConfig = toml::from_str(toml).unwrap();

        assert!(config.retry_enabled);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_retry_delay_secs, 2);
        assert_eq!(config.max_retry_delay_secs, 120);
        assert_eq!(config.retry_multiplier, 3.0);
    }

    #[test]
    fn test_retry_config_missing_fields_use_defaults() {
        let toml = r#"
            max_concurrent_tasks = 5
            task_timeout = 300
            streaming = false
            max_iterations = 10
        "#;

        let config: ExecutionConfig = toml::from_str(toml).unwrap();

        // Missing retry fields should use defaults
        assert!(!config.retry_enabled);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_retry_delay_secs, 1);
        assert_eq!(config.max_retry_delay_secs, 60);
        assert_eq!(config.retry_multiplier, 2.0);
    }
}
