//! YAML schema definitions for installation configuration
//!
//! Defines the structure of orca_base_install.yaml and aco_base_install.yaml

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Orca Install Configuration
// =============================================================================

/// Root configuration for orca installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaInstallConfig {
    pub version: String,
    pub orca: OrcaConfig,
}

/// Orca-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaConfig {
    pub toml: OrcaTomlConfig,
    pub database: OrcaDatabaseSeed,
}

/// TOML configuration defaults for orca
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaTomlConfig {
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub execution: ExecutionConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub budget: BudgetConfig,
    #[serde(default)]
    pub workflow: WorkflowConfig,
    #[serde(default)]
    pub backup: Option<BackupConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

fn default_db_path() -> String {
    "orca.db".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            api_key: None,
            api_base: None,
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_provider() -> String {
    "anthropic".to_string()
}
fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}
fn default_temperature() -> f64 {
    0.7
}
fn default_max_tokens() -> u32 {
    4096
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_tasks: u32,
    #[serde(default = "default_task_timeout")]
    pub task_timeout: u64,
    #[serde(default = "default_true")]
    pub streaming: bool,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_pattern")]
    pub default_pattern: String,
    #[serde(default = "default_true")]
    pub show_thinking: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: default_max_concurrent(),
            task_timeout: default_task_timeout(),
            streaming: true,
            max_iterations: default_max_iterations(),
            default_pattern: default_pattern(),
            show_thinking: true,
        }
    }
}

fn default_max_concurrent() -> u32 {
    5
}
fn default_task_timeout() -> u64 {
    300
}
fn default_max_iterations() -> u32 {
    10
}
fn default_pattern() -> String {
    "react".to_string()
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default = "default_true")]
    pub colored: bool,
    #[serde(default = "default_true")]
    pub timestamps: bool,
    #[serde(default)]
    pub log_directory: Option<String>,
    #[serde(default = "default_log_prefix")]
    pub log_prefix: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            colored: true,
            timestamps: true,
            log_directory: None,
            log_prefix: default_log_prefix(),
        }
    }
}

fn default_log_prefix() -> String {
    "orca".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "compact".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    pub default_budget: Option<String>,
    #[serde(default = "default_true")]
    pub enforce_budgets: bool,
    #[serde(default = "default_true")]
    pub log_usage: bool,
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            default_budget: None,
            enforce_budgets: true,
            log_usage: true,
            alert_threshold: default_alert_threshold(),
        }
    }
}

fn default_alert_threshold() -> f64 {
    80.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub default_llm_profile: Option<String>,
    pub default_planner_llm: Option<String>,
    pub default_worker_llm: Option<String>,
    #[serde(default)]
    pub enable_caching: bool,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_secs: u64,
    #[serde(default = "default_max_duration")]
    pub max_duration_secs: u64,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            default_llm_profile: None,
            default_planner_llm: None,
            default_worker_llm: None,
            enable_caching: false,
            cache_ttl_secs: default_cache_ttl(),
            max_duration_secs: default_max_duration(),
        }
    }
}

fn default_cache_ttl() -> u64 {
    3600
}
fn default_max_duration() -> u64 {
    3600
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackupConfig {
    pub backup_dir: Option<String>,
}

// =============================================================================
// Database Seed Data
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrcaDatabaseSeed {
    #[serde(default)]
    pub llm_providers: Vec<LlmProviderSeed>,
    #[serde(default)]
    pub llm_pricing: Vec<LlmPricingSeed>,
    #[serde(default)]
    pub budgets: Vec<BudgetSeed>,
    #[serde(default)]
    pub llm_profiles: Vec<LlmProfileSeed>,
    #[serde(default)]
    pub prompts: Vec<PromptSeed>,
    #[serde(default)]
    pub workflow_templates: Vec<WorkflowTemplateSeed>,
    #[serde(default)]
    pub pattern_configs: Vec<PatternConfigSeed>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderSeed {
    pub name: String,
    pub provider_type: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub settings: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPricingSeed {
    pub provider: String,
    pub model: String,
    pub cost_per_input_token: f64,
    pub cost_per_output_token: f64,
    #[serde(default)]
    pub cost_per_reasoning_token: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetSeed {
    pub name: String,
    #[serde(rename = "type")]
    pub budget_type: String,
    pub renewal_interval_unit: Option<String>,
    pub renewal_interval_value: Option<u32>,
    pub credit_amount: f64,
    pub credit_cap: Option<f64>,
    #[serde(default = "default_enforcement")]
    pub enforcement: String,
    #[serde(default)]
    pub active: bool,
}

fn default_enforcement() -> String {
    "warn".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProfileSeed {
    pub name: String,
    pub description: Option<String>,
    pub planner_provider: String,
    pub planner_model: String,
    pub worker_provider: String,
    pub worker_model: String,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSeed {
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub template: String,
    #[serde(default)]
    pub variables: Vec<String>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateSeed {
    pub name: String,
    pub description: Option<String>,
    pub pattern: String,
    pub definition: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfigSeed {
    pub name: String,
    pub pattern_type: String,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default)]
    pub tools: Vec<String>,
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub config: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub is_default: bool,
}

// =============================================================================
// ACO Install Configuration
// =============================================================================

/// Root configuration for aco installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcoInstallConfig {
    pub version: String,
    pub aco: AcoConfig,
}

/// ACO-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcoConfig {
    pub toml: AcoTomlConfig,
}

/// TOML configuration defaults for aco
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcoTomlConfig {
    #[serde(default)]
    pub server: AcoServerConfig,
    #[serde(default)]
    pub client: AcoClientConfig,
    #[serde(default)]
    pub tools: AcoToolsConfig,
    #[serde(default)]
    pub ui: AcoUiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcoServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_ws_path")]
    pub ws_path: String,
    #[serde(default)]
    pub enable_tls: bool,
}

impl Default for AcoServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            ws_path: default_ws_path(),
            enable_tls: false,
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_ws_path() -> String {
    "/ws".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcoClientConfig {
    #[serde(default = "default_orchestrator_url")]
    pub orchestrator_url: String,
    #[serde(default)]
    pub auto_connect: bool,
    #[serde(default = "default_session_timeout")]
    pub session_timeout: u64,
    #[serde(default = "default_reconnect_attempts")]
    pub reconnect_attempts: u32,
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_ms: u64,
}

impl Default for AcoClientConfig {
    fn default() -> Self {
        Self {
            orchestrator_url: default_orchestrator_url(),
            auto_connect: false,
            session_timeout: default_session_timeout(),
            reconnect_attempts: default_reconnect_attempts(),
            reconnect_delay_ms: default_reconnect_delay(),
        }
    }
}

fn default_orchestrator_url() -> String {
    "ws://127.0.0.1:8080/ws".to_string()
}
fn default_session_timeout() -> u64 {
    3600
}
fn default_reconnect_attempts() -> u32 {
    5
}
fn default_reconnect_delay() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcoToolsConfig {
    #[serde(default)]
    pub enabled_tools: Vec<String>,
    #[serde(default = "default_execution_timeout")]
    pub execution_timeout: u64,
}

impl Default for AcoToolsConfig {
    fn default() -> Self {
        Self {
            enabled_tools: Vec::new(),
            execution_timeout: default_execution_timeout(),
        }
    }
}

fn default_execution_timeout() -> u64 {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcoUiConfig {
    #[serde(default)]
    pub enable_tui: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_true")]
    pub colored_output: bool,
    #[serde(default = "default_true")]
    pub show_timestamps: bool,
    #[serde(default = "default_true")]
    pub show_thinking: bool,
}

impl Default for AcoUiConfig {
    fn default() -> Self {
        Self {
            enable_tui: false,
            log_level: default_log_level(),
            colored_output: true,
            show_timestamps: true,
            show_thinking: true,
        }
    }
}

// =============================================================================
// Utility functions
// =============================================================================

impl OrcaInstallConfig {
    /// Load from YAML file
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

impl AcoInstallConfig {
    /// Load from YAML file
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
