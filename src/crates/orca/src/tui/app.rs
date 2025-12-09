//! Application state management for TUI

use crate::HealthReport;
use crate::db::Database;
use crate::models::{LlmProviderConfig, PatternConfig};
use crate::repositories::{BudgetRepository, LlmProviderRepository, PatternConfigRepository};
use crate::services::ModelDiscoveryService;
use crate::config::{OrcaConfig, DatabaseConfig, ExecutionConfig, LoggingConfig, BudgetConfig, WorkflowConfig, BackupConfig};
use super::dialog::Dialog;
use std::collections::VecDeque;
use std::sync::Arc;
use chrono::Utc;

/// Maximum number of conversation/log entries to keep
const MAX_ENTRIES: usize = 1000;

/// Which sidebar tab is currently active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    History,
    Todo,
    Bugs,
    Patterns,
}

/// Which area is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedArea {
    Conversation,
    Prompts,
    Sidebar,
    Menu,
}

/// Menu bar state - which menu is open (if any)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuState {
    Closed,
    FileOpen,
    EditOpen,
    ConfigOpen,
    WorkflowOpen,
    HelpOpen,
}

/// Dialog state - what dialog is currently open
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogState {
    None,
    BudgetList,
    BudgetCreate,
    BudgetEdit,
    ConfigViewer,
    ExternalEditor,
    PatternSelect,
    PatternCreate,
    PatternEdit,
    PatternList,
}

/// View mode - what is displayed in the main content area
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Conversation,
    ConfigEditor,
}

/// Config editor section
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSection {
    Llm,
    Execution,
    Logging,
    Budget,
    Workflow,
}

impl ConfigSection {
    pub fn all() -> &'static [ConfigSection] {
        &[
            ConfigSection::Llm,
            ConfigSection::Execution,
            ConfigSection::Logging,
            ConfigSection::Budget,
            ConfigSection::Workflow,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ConfigSection::Llm => "LLM",
            ConfigSection::Execution => "Execution",
            ConfigSection::Logging => "Logging",
            ConfigSection::Budget => "Budget",
            ConfigSection::Workflow => "Workflow",
        }
    }

    pub fn next(&self) -> ConfigSection {
        match self {
            ConfigSection::Llm => ConfigSection::Execution,
            ConfigSection::Execution => ConfigSection::Logging,
            ConfigSection::Logging => ConfigSection::Budget,
            ConfigSection::Budget => ConfigSection::Workflow,
            ConfigSection::Workflow => ConfigSection::Llm,
        }
    }

    pub fn prev(&self) -> ConfigSection {
        match self {
            ConfigSection::Llm => ConfigSection::Workflow,
            ConfigSection::Execution => ConfigSection::Llm,
            ConfigSection::Logging => ConfigSection::Execution,
            ConfigSection::Budget => ConfigSection::Logging,
            ConfigSection::Workflow => ConfigSection::Budget,
        }
    }
}

/// LLM Profile entry for config editor
#[derive(Debug, Clone)]
pub struct LlmProfileEntry {
    pub id: Option<String>,  // None for new profiles
    pub name: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub api_base: String,
    pub temperature: String,
    pub max_tokens: String,
    pub enabled: bool,       // Whether this provider is enabled
    pub is_default: bool,
    pub workflow: String,  // "adaptive_react" or workflow path
    pub budget: String,
}

impl Default for LlmProfileEntry {
    fn default() -> Self {
        // Empty defaults - actual values come from database via orca_install
        Self {
            id: None,
            name: String::new(),
            provider: String::new(),
            model: String::new(),
            api_key: String::new(),
            api_base: String::new(),
            temperature: String::new(),
            max_tokens: String::new(),
            enabled: true,  // Default to enabled
            is_default: false,
            workflow: String::new(),
            budget: String::new(),
        }
    }
}

/// Supported LLM providers for dropdown
pub const LLM_PROVIDERS: &[&str] = &[
    "claude",      // Anthropic
    "openai",      // OpenAI
    "gemini",      // Google
    "grok",        // xAI
    "deepseek",    // Deepseek
    "openrouter",  // OpenRouter
    "ollama",      // Ollama (local)
    "llama_cpp",   // llama.cpp (local)
    "lmstudio",    // LM Studio (local)
    "claude_code", // Claude Code CLI (local)
];

/// Pattern types for dropdown
pub const PATTERN_TYPES: &[&str] = &[
    "react",
    "plan_execute",
    "reflection",
    "lats",
    "storm",
    "code_act",
    "tot",
    "cot",
    "got",
];

/// Get models for a given provider
pub fn models_for_provider(provider: &str) -> Vec<&'static str> {
    match provider {
        "claude" => vec!["claude-sonnet-4-5-20250514", "claude-3-5-sonnet-20241022", "claude-3-opus-20240229", "claude-3-haiku-20240307"],
        "openai" => vec!["gpt-4o", "gpt-4-turbo", "gpt-4", "gpt-3.5-turbo", "o1", "o1-mini"],
        "gemini" => vec!["gemini-pro", "gemini-pro-vision"],
        "grok" => vec!["grok-beta"],
        "deepseek" => vec!["deepseek-chat", "deepseek-coder", "deepseek-reasoner"],
        "openrouter" => vec!["anthropic/claude-3-opus", "openai/gpt-4-turbo", "google/gemini-pro"],
        "ollama" => vec!["llama3.2", "llama3.1", "mistral", "mixtral", "codellama", "phi3"],
        "llama_cpp" => vec!["default"],
        "lmstudio" => vec!["default"],
        "claude_code" => vec!["claude-sonnet-4-5-20250514"],
        _ => vec![],
    }
}

/// Get default API base URL for a provider (returns Some for local providers, None for cloud)
pub fn default_api_base_for_provider(provider: &str) -> Option<&'static str> {
    match provider {
        "ollama" => Some("http://localhost:11434"),
        "llama_cpp" => Some("http://localhost:8080"),
        "lmstudio" => Some("http://localhost:1234"),
        _ => None, // Cloud providers use their own endpoints
    }
}

impl LlmProfileEntry {
    pub fn from_provider_config(config: &LlmProviderConfig) -> Self {
        Self {
            id: Some(config.id.clone()),
            name: config.name.clone(),
            provider: config.provider_type.clone(),
            model: config.model.clone(),
            api_key: config.api_key.clone().unwrap_or_default(),
            api_base: config.api_base.clone().unwrap_or_default(),
            temperature: config.temperature.to_string(),
            max_tokens: config.max_tokens.to_string(),
            enabled: true,  // Default to enabled (can be extended when LlmProviderConfig has enabled field)
            is_default: config.is_default,
            workflow: "adaptive_react".to_string(),  // Default workflow
            budget: String::new(),
        }
    }

    /// Field count for detail form
    pub fn field_count() -> usize {
        11  // name, provider, model, api_key, api_base, temperature, max_tokens, enabled, is_default, workflow, budget
    }

    /// Field name by index
    pub fn field_name(index: usize) -> &'static str {
        match index {
            0 => "Name",
            1 => "Provider",
            2 => "Model",
            3 => "API Key",
            4 => "API Base URL",
            5 => "Temperature",
            6 => "Max Tokens",
            7 => "Enabled",
            8 => "Is Default",
            9 => "Workflow",
            10 => "Budget",
            _ => "",
        }
    }

    /// Get field value by index
    pub fn field_value(&self, index: usize) -> String {
        match index {
            0 => self.name.clone(),
            1 => self.provider.clone(),
            2 => self.model.clone(),
            3 => if self.api_key.is_empty() { "(not set)".to_string() } else { "********".to_string() },
            4 => if self.api_base.is_empty() { "(default)".to_string() } else { self.api_base.clone() },
            5 => self.temperature.clone(),
            6 => self.max_tokens.clone(),
            7 => if self.enabled { "true" } else { "false" }.to_string(),
            8 => if self.is_default { "true" } else { "false" }.to_string(),
            9 => self.workflow.clone(),
            10 => if self.budget.is_empty() { "(none)".to_string() } else { self.budget.clone() },
            _ => String::new(),
        }
    }

    /// Check if field is boolean
    pub fn is_bool_field(index: usize) -> bool {
        matches!(index, 7 | 8)  // enabled, is_default
    }

    /// Toggle boolean field
    pub fn toggle_bool(&mut self, index: usize) {
        match index {
            7 => self.enabled = !self.enabled,
            8 => self.is_default = !self.is_default,
            _ => {}
        }
    }

    /// Get mutable reference to field string value
    pub fn get_field_mut(&mut self, index: usize) -> Option<&mut String> {
        match index {
            0 => Some(&mut self.name),
            1 => Some(&mut self.provider),
            2 => Some(&mut self.model),
            3 => Some(&mut self.api_key),
            4 => Some(&mut self.api_base),
            5 => Some(&mut self.temperature),
            6 => Some(&mut self.max_tokens),
            // 7 = enabled (bool), 8 = is_default (bool) - not editable as strings
            9 => Some(&mut self.workflow),
            10 => Some(&mut self.budget),
            _ => None,
        }
    }

    /// Check if field at index is a dropdown field
    pub fn is_dropdown_field(index: usize) -> bool {
        matches!(index, 1 | 2 | 9 | 10)  // Provider=1, Model=2, Workflow=9, Budget=10
    }

    /// Get dropdown options for a field
    pub fn dropdown_options_for_field(index: usize, current_provider: &str) -> Vec<String> {
        match index {
            1 => LLM_PROVIDERS.iter().map(|s| s.to_string()).collect(),
            2 => models_for_provider(current_provider).iter().map(|s| s.to_string()).collect(),
            9 => PATTERN_TYPES.iter().map(|s| s.to_string()).collect(),
            10 => vec!["(none)".to_string()],  // Budget loaded separately from database
            _ => vec![],
        }
    }
}

/// LLM section focus area
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmFocusArea {
    Table,
    Detail,
}

/// Config editor form state
#[derive(Debug, Clone)]
pub struct ConfigEditorForm {
    /// Current section being edited
    pub section: ConfigSection,
    /// Currently selected field index within section
    pub field_index: usize,
    /// Whether currently editing a field value
    pub editing: bool,
    /// Edit buffer for current field
    pub edit_buffer: String,
    /// Has unsaved changes
    pub modified: bool,
    /// Save/Cancel button focus (0=none, 1=Save, 2=Cancel)
    pub button_focus: usize,

    // Dropdown state
    pub dropdown_open: bool,
    pub dropdown_field: Option<usize>,  // Which field has dropdown open
    pub dropdown_options: Vec<String>,
    pub dropdown_selected: usize,
    pub available_budgets: Vec<String>,  // Budget names from database
    pub available_llm_profile_names: Vec<String>,  // LLM profile names for workflow dropdowns

    // LLM section - multiple profiles
    pub llm_profiles: Vec<LlmProfileEntry>,
    pub llm_selected_index: usize,
    pub llm_focus: LlmFocusArea,
    pub llm_detail_field: usize,  // Selected field in detail view
    pub available_workflows: Vec<String>,

    // Execution section
    pub exec_max_concurrent: String,
    pub exec_timeout: String,
    pub exec_streaming: bool,
    pub exec_max_iterations: String,
    pub exec_default_pattern: String,
    pub exec_show_thinking: bool,
    pub exec_retry_enabled: bool,
    pub exec_max_retries: String,

    // Logging section
    pub log_level: String,
    pub log_format: String,
    pub log_colored: bool,
    pub log_timestamps: bool,
    pub log_directory: Option<String>,
    pub log_prefix: String,

    // Budget section
    pub budget_default: String,
    pub budget_enforce: bool,
    pub budget_log_usage: bool,
    pub budget_alert_threshold: String,

    // Workflow section
    pub workflow_planner_llm: String,
    pub workflow_worker_llm: String,
    pub workflow_caching: bool,
    pub workflow_cache_ttl: String,
    pub workflow_max_duration: String,
}

impl Default for ConfigEditorForm {
    fn default() -> Self {
        Self {
            section: ConfigSection::Llm,
            field_index: 0,
            editing: false,
            edit_buffer: String::new(),
            modified: false,
            button_focus: 0,

            // Dropdown defaults
            dropdown_open: false,
            dropdown_field: None,
            dropdown_options: Vec::new(),
            dropdown_selected: 0,
            available_budgets: vec!["(none)".to_string()],
            available_llm_profile_names: Vec::new(),  // Populated when LLM profiles are loaded

            // LLM profiles - loaded from database via orca_install
            llm_profiles: Vec::new(),  // Empty until loaded from database
            llm_selected_index: 0,
            llm_focus: LlmFocusArea::Table,
            llm_detail_field: 0,
            available_workflows: Vec::new(),  // Loaded from database

            // Execution defaults
            exec_max_concurrent: "5".to_string(),
            exec_timeout: "300".to_string(),
            exec_streaming: true,
            exec_max_iterations: "10".to_string(),
            exec_default_pattern: "react".to_string(),
            exec_show_thinking: true,
            exec_retry_enabled: false,
            exec_max_retries: "3".to_string(),

            // Logging defaults
            log_level: "info".to_string(),
            log_format: "compact".to_string(),
            log_colored: true,
            log_timestamps: true,
            log_directory: None,
            log_prefix: "orca".to_string(),

            // Budget defaults
            budget_default: String::new(),
            budget_enforce: true,
            budget_log_usage: true,
            budget_alert_threshold: "80.0".to_string(),

            // Workflow defaults
            workflow_planner_llm: String::new(),
            workflow_worker_llm: String::new(),
            workflow_caching: false,
            workflow_cache_ttl: "3600".to_string(),
            workflow_max_duration: "3600".to_string(),
        }
    }
}

impl ConfigEditorForm {
    /// Get field count for current section (non-LLM sections only, LLM uses profile detail)
    pub fn field_count(&self) -> usize {
        match self.section {
            ConfigSection::Llm => LlmProfileEntry::field_count(),  // Detail form field count
            ConfigSection::Execution => 8,
            ConfigSection::Logging => 4,
            ConfigSection::Budget => 4,
            ConfigSection::Workflow => 5,  // Planner LLM, Worker LLM, Caching, Cache TTL, Max Duration
        }
    }

    /// Get field name for current section and index
    pub fn field_name(&self, index: usize) -> &'static str {
        match self.section {
            ConfigSection::Llm => LlmProfileEntry::field_name(index),
            ConfigSection::Execution => match index {
                0 => "Max Concurrent Tasks",
                1 => "Task Timeout (sec)",
                2 => "Streaming",
                3 => "Max Iterations",
                4 => "Default Pattern",
                5 => "Show Thinking",
                6 => "Retry Enabled",
                7 => "Max Retries",
                _ => "",
            },
            ConfigSection::Logging => match index {
                0 => "Log Level",
                1 => "Format",
                2 => "Colored Output",
                3 => "Show Timestamps",
                _ => "",
            },
            ConfigSection::Budget => match index {
                0 => "Default Budget",
                1 => "Enforce Budgets",
                2 => "Log Usage",
                3 => "Alert Threshold %",
                _ => "",
            },
            ConfigSection::Workflow => match index {
                0 => "Planner LLM",
                1 => "Worker LLM",
                2 => "Enable Caching",
                3 => "Cache TTL (sec)",
                4 => "Max Duration (sec)",
                _ => "",
            },
        }
    }

    /// Get field value as string for current section and index
    pub fn field_value(&self, index: usize) -> String {
        match self.section {
            ConfigSection::Llm => {
                // For LLM section, get value from selected profile
                if let Some(profile) = self.llm_profiles.get(self.llm_selected_index) {
                    profile.field_value(index)
                } else {
                    String::new()
                }
            },
            ConfigSection::Execution => match index {
                0 => self.exec_max_concurrent.clone(),
                1 => self.exec_timeout.clone(),
                2 => if self.exec_streaming { "true" } else { "false" }.to_string(),
                3 => self.exec_max_iterations.clone(),
                4 => self.exec_default_pattern.clone(),
                5 => if self.exec_show_thinking { "true" } else { "false" }.to_string(),
                6 => if self.exec_retry_enabled { "true" } else { "false" }.to_string(),
                7 => self.exec_max_retries.clone(),
                _ => String::new(),
            },
            ConfigSection::Logging => match index {
                0 => self.log_level.clone(),
                1 => self.log_format.clone(),
                2 => if self.log_colored { "true" } else { "false" }.to_string(),
                3 => if self.log_timestamps { "true" } else { "false" }.to_string(),
                _ => String::new(),
            },
            ConfigSection::Budget => match index {
                0 => if self.budget_default.is_empty() { "(none)".to_string() } else { self.budget_default.clone() },
                1 => if self.budget_enforce { "true" } else { "false" }.to_string(),
                2 => if self.budget_log_usage { "true" } else { "false" }.to_string(),
                3 => self.budget_alert_threshold.clone(),
                _ => String::new(),
            },
            ConfigSection::Workflow => match index {
                0 => if self.workflow_planner_llm.is_empty() {
                    // Show "(default: ProfileName)" when using default
                    match self.get_default_llm_profile_name() {
                        Some(name) => format!("(default: {})", name),
                        None => "(default)".to_string(),
                    }
                } else {
                    self.workflow_planner_llm.clone()
                },
                1 => if self.workflow_worker_llm.is_empty() {
                    // Show "(default: ProfileName)" when using default
                    match self.get_default_llm_profile_name() {
                        Some(name) => format!("(default: {})", name),
                        None => "(default)".to_string(),
                    }
                } else {
                    self.workflow_worker_llm.clone()
                },
                2 => if self.workflow_caching { "true" } else { "false" }.to_string(),
                3 => self.workflow_cache_ttl.clone(),
                4 => self.workflow_max_duration.clone(),
                _ => String::new(),
            },
        }
    }

    /// Check if field at index is a boolean toggle
    pub fn is_bool_field(&self, index: usize) -> bool {
        match self.section {
            ConfigSection::Llm => LlmProfileEntry::is_bool_field(index),
            ConfigSection::Execution => matches!(index, 2 | 5 | 6),
            ConfigSection::Logging => matches!(index, 2 | 3),
            ConfigSection::Budget => matches!(index, 1 | 2),
            ConfigSection::Workflow => matches!(index, 2),  // Enable Caching is now at index 2
        }
    }

    /// Toggle boolean field at current index
    pub fn toggle_bool_field(&mut self) {
        match self.section {
            ConfigSection::Llm => {
                // Toggle boolean in selected profile
                if let Some(profile) = self.llm_profiles.get_mut(self.llm_selected_index) {
                    profile.toggle_bool(self.llm_detail_field);
                    self.modified = true;
                }
            },
            ConfigSection::Execution => match self.field_index {
                2 => { self.exec_streaming = !self.exec_streaming; self.modified = true; },
                5 => { self.exec_show_thinking = !self.exec_show_thinking; self.modified = true; },
                6 => { self.exec_retry_enabled = !self.exec_retry_enabled; self.modified = true; },
                _ => {}
            },
            ConfigSection::Logging => match self.field_index {
                2 => { self.log_colored = !self.log_colored; self.modified = true; },
                3 => { self.log_timestamps = !self.log_timestamps; self.modified = true; },
                _ => {}
            },
            ConfigSection::Budget => match self.field_index {
                1 => { self.budget_enforce = !self.budget_enforce; self.modified = true; },
                2 => { self.budget_log_usage = !self.budget_log_usage; self.modified = true; },
                _ => {}
            },
            ConfigSection::Workflow => match self.field_index {
                2 => { self.workflow_caching = !self.workflow_caching; self.modified = true; },
                _ => {}
            },
        }
    }

    /// Get mutable reference to field value for editing (non-LLM sections)
    pub fn get_field_mut(&mut self) -> Option<&mut String> {
        match self.section {
            ConfigSection::Llm => None,  // LLM uses profile-specific editing
            ConfigSection::Execution => match self.field_index {
                0 => Some(&mut self.exec_max_concurrent),
                1 => Some(&mut self.exec_timeout),
                3 => Some(&mut self.exec_max_iterations),
                4 => Some(&mut self.exec_default_pattern),
                7 => Some(&mut self.exec_max_retries),
                _ => None,
            },
            ConfigSection::Logging => match self.field_index {
                0 => Some(&mut self.log_level),
                1 => Some(&mut self.log_format),
                _ => None,
            },
            ConfigSection::Budget => match self.field_index {
                0 => Some(&mut self.budget_default),
                3 => Some(&mut self.budget_alert_threshold),
                _ => None,
            },
            ConfigSection::Workflow => match self.field_index {
                0 => Some(&mut self.workflow_planner_llm),
                1 => Some(&mut self.workflow_worker_llm),
                3 => Some(&mut self.workflow_cache_ttl),
                4 => Some(&mut self.workflow_max_duration),
                _ => None,
            },
        }
    }

    /// Get selected LLM profile (immutable)
    pub fn selected_llm_profile(&self) -> Option<&LlmProfileEntry> {
        self.llm_profiles.get(self.llm_selected_index)
    }

    /// Get selected LLM profile (mutable)
    pub fn selected_llm_profile_mut(&mut self) -> Option<&mut LlmProfileEntry> {
        self.llm_profiles.get_mut(self.llm_selected_index)
    }

    /// Add a new LLM profile
    pub fn add_llm_profile(&mut self) {
        let new_profile = LlmProfileEntry {
            name: format!("profile_{}", self.llm_profiles.len() + 1),
            is_default: self.llm_profiles.is_empty(),
            ..Default::default()
        };
        self.llm_profiles.push(new_profile);
        self.llm_selected_index = self.llm_profiles.len() - 1;
        self.llm_focus = LlmFocusArea::Detail;
        self.modified = true;
    }

    /// Copy selected LLM profile
    pub fn copy_llm_profile(&mut self) {
        if let Some(profile) = self.llm_profiles.get(self.llm_selected_index) {
            let mut new_profile = profile.clone();
            new_profile.id = None; // New profile, no ID yet
            new_profile.name = format!("{}_copy", profile.name);
            new_profile.is_default = false; // Copy shouldn't be default
            self.llm_profiles.push(new_profile);
            self.llm_selected_index = self.llm_profiles.len() - 1;
            self.llm_focus = LlmFocusArea::Detail;
            self.modified = true;
        }
    }

    /// Delete selected LLM profile
    pub fn delete_selected_llm_profile(&mut self) {
        if self.llm_profiles.len() > 1 {
            self.llm_profiles.remove(self.llm_selected_index);
            if self.llm_selected_index >= self.llm_profiles.len() {
                self.llm_selected_index = self.llm_profiles.len() - 1;
            }
            self.modified = true;
        }
    }

    /// Get the name of the default LLM profile (the one with is_default=true)
    pub fn get_default_llm_profile_name(&self) -> Option<String> {
        self.llm_profiles
            .iter()
            .find(|p| p.is_default)
            .map(|p| p.name.clone())
    }

    /// Load from OrcaConfig (loads execution, logging, budget, workflow settings)
    /// Note: LLM profiles are loaded separately from database via load_llm_profiles_into_editor
    pub fn load_from_config(&mut self, config: &OrcaConfig) {
        // Don't create LLM profiles from toml - they come from database only
        // This ensures View Config and LLM Config show the same source data

        // Execution
        self.exec_max_concurrent = config.execution.max_concurrent_tasks.to_string();
        self.exec_timeout = config.execution.task_timeout.to_string();
        self.exec_streaming = config.execution.streaming;
        self.exec_max_iterations = config.execution.max_iterations.to_string();
        self.exec_default_pattern = config.execution.default_pattern.clone();
        self.exec_show_thinking = config.execution.show_thinking;
        self.exec_retry_enabled = config.execution.retry_enabled;
        self.exec_max_retries = config.execution.max_retries.to_string();

        // Logging
        self.log_level = config.logging.level.clone();
        self.log_format = config.logging.format.clone();
        self.log_colored = config.logging.colored;
        self.log_timestamps = config.logging.timestamps;
        self.log_directory = config.logging.log_directory.clone();
        self.log_prefix = config.logging.log_prefix.clone();

        // Budget
        self.budget_default = config.budget.default_budget.clone().unwrap_or_default();
        self.budget_enforce = config.budget.enforce_budgets;
        self.budget_log_usage = config.budget.log_usage;
        self.budget_alert_threshold = config.budget.alert_threshold.to_string();

        // Workflow - planner/worker LLM config is now database-only (llm_profiles)
        self.workflow_planner_llm = String::new(); // Loaded from database LLM profiles
        self.workflow_worker_llm = String::new();  // Loaded from database LLM profiles
        self.workflow_caching = config.workflow.enable_caching;
        self.workflow_cache_ttl = config.workflow.cache_ttl_secs.to_string();
        self.workflow_max_duration = config.workflow.max_duration_secs.to_string();

        self.modified = false;
    }

    /// Convert to OrcaConfig (LLM config is now database-only, not in config)
    pub fn to_config(&self) -> OrcaConfig {
        // LLM configuration is now database-only (llm_providers and llm_profiles tables)
        // The OrcaConfig no longer contains an llm section

        OrcaConfig {
            project_name: None,
            database: DatabaseConfig::default(),
            execution: ExecutionConfig {
                max_concurrent_tasks: self.exec_max_concurrent.parse().unwrap_or(5),
                task_timeout: self.exec_timeout.parse().unwrap_or(300),
                streaming: self.exec_streaming,
                workspace_root: None,
                max_iterations: self.exec_max_iterations.parse().unwrap_or(10),
                default_pattern: self.exec_default_pattern.clone(),
                reflection_quality_threshold: 0.75,
                plan_execute_max_steps: 10,
                retry_enabled: self.exec_retry_enabled,
                max_retries: self.exec_max_retries.parse().unwrap_or(3),
                initial_retry_delay_secs: 1,
                max_retry_delay_secs: 60,
                retry_multiplier: 2.0,
                show_thinking: self.exec_show_thinking,
            },
            logging: LoggingConfig {
                level: self.log_level.clone(),
                format: self.log_format.clone(),
                colored: self.log_colored,
                timestamps: self.log_timestamps,
                log_directory: self.log_directory.clone(),
                log_prefix: self.log_prefix.clone(),
            },
            budget: BudgetConfig {
                default_budget: if self.budget_default.is_empty() { None } else { Some(self.budget_default.clone()) },
                enforce_budgets: self.budget_enforce,
                log_usage: self.budget_log_usage,
                alert_threshold: self.budget_alert_threshold.parse().unwrap_or(80.0),
            },
            workflow: WorkflowConfig {
                // planner/worker LLM config moved to database llm_profiles
                enable_caching: self.workflow_caching,
                cache_ttl_secs: self.workflow_cache_ttl.parse().unwrap_or(3600),
                max_duration_secs: self.workflow_max_duration.parse().unwrap_or(3600),
            },
            backup: BackupConfig::default(),
        }
    }

    // === Dropdown Management Methods ===

    /// Open dropdown for current field
    pub fn open_dropdown(&mut self) {
        let field_index = self.llm_detail_field;
        if !LlmProfileEntry::is_dropdown_field(field_index) {
            return;
        }

        // Get current provider for model dropdown
        let current_provider = self.selected_llm_profile()
            .map(|p| p.provider.clone())
            .unwrap_or_default();

        // Special handling for budget field - use available_budgets
        let options = if field_index == 10 {
            self.available_budgets.clone()
        } else {
            LlmProfileEntry::dropdown_options_for_field(field_index, &current_provider)
        };

        if options.is_empty() {
            return;
        }

        // Find current value in options to pre-select
        let current_value = self.selected_llm_profile()
            .map(|p| p.field_value(field_index))
            .unwrap_or_default();

        let selected = options.iter()
            .position(|o| o == &current_value)
            .unwrap_or(0);

        self.dropdown_open = true;
        self.dropdown_field = Some(field_index);
        self.dropdown_options = options;
        self.dropdown_selected = selected;
    }

    /// Close dropdown without applying selection
    pub fn close_dropdown(&mut self) {
        self.dropdown_open = false;
        self.dropdown_field = None;
        self.dropdown_options.clear();
        self.dropdown_selected = 0;
    }

    /// Move dropdown selection up
    pub fn dropdown_prev(&mut self) {
        if self.dropdown_selected > 0 {
            self.dropdown_selected -= 1;
        } else if !self.dropdown_options.is_empty() {
            self.dropdown_selected = self.dropdown_options.len() - 1;
        }
    }

    /// Move dropdown selection down
    pub fn dropdown_next(&mut self) {
        if !self.dropdown_options.is_empty() {
            self.dropdown_selected = (self.dropdown_selected + 1) % self.dropdown_options.len();
        }
    }

    /// Apply dropdown selection and close
    pub fn apply_dropdown_selection(&mut self) {
        let Some(field_index) = self.dropdown_field else {
            self.close_dropdown();
            return;
        };

        let Some(selected_value) = self.dropdown_options.get(self.dropdown_selected).cloned() else {
            self.close_dropdown();
            return;
        };

        // Apply the selection to the profile field
        if let Some(profile) = self.selected_llm_profile_mut() {
            // Handle "(none)" for budget field
            let value = if field_index == 10 && selected_value == "(none)" {
                String::new()
            } else {
                selected_value.clone()
            };

            match field_index {
                1 => {
                    let provider_changed = profile.provider != value;
                    profile.provider = value;
                    // If provider changed, reset model and api_base for new provider
                    if provider_changed {
                        let new_models = models_for_provider(&profile.provider);
                        if let Some(first_model) = new_models.first() {
                            profile.model = first_model.to_string();
                        }
                        // Set default api_base for local providers
                        if let Some(default_url) = default_api_base_for_provider(&profile.provider) {
                            profile.api_base = default_url.to_string();
                        } else {
                            // Cloud providers - clear api_base (they use their own endpoints)
                            profile.api_base = String::new();
                        }
                    }
                }
                2 => profile.model = value,
                9 => profile.workflow = value,
                10 => profile.budget = value,
                _ => {}
            }
            self.modified = true;
        }

        self.close_dropdown();
    }

    /// Check if current field is a dropdown field
    pub fn is_current_field_dropdown(&self) -> bool {
        match self.section {
            ConfigSection::Llm => {
                self.llm_focus == LlmFocusArea::Detail
                    && LlmProfileEntry::is_dropdown_field(self.llm_detail_field)
            }
            ConfigSection::Workflow => Self::is_workflow_dropdown_field(self.field_index),
            _ => false,
        }
    }

    /// Check if workflow field at index is a dropdown
    pub fn is_workflow_dropdown_field(index: usize) -> bool {
        matches!(index, 0 | 1)  // Planner LLM, Worker LLM
    }

    /// Get dropdown options for workflow field
    pub fn workflow_dropdown_options(&self, field_index: usize) -> Vec<String> {
        match field_index {
            0 | 1 => {
                // Show "(default: ProfileName)" if a default profile exists
                let default_option = match self.get_default_llm_profile_name() {
                    Some(name) => format!("(default: {})", name),
                    None => "(default)".to_string(),
                };
                let mut options = vec![default_option];
                options.extend(self.available_llm_profile_names.clone());
                options
            }
            _ => vec![],
        }
    }

    /// Open dropdown for workflow field
    pub fn open_workflow_dropdown(&mut self) {
        let field_index = self.field_index;
        if !Self::is_workflow_dropdown_field(field_index) {
            return;
        }

        let options = self.workflow_dropdown_options(field_index);
        if options.is_empty() {
            return;
        }

        // Find current value in options to pre-select
        let current_value = self.field_value(field_index);
        let selected = options.iter()
            .position(|o| o == &current_value)
            .unwrap_or(0);

        self.dropdown_open = true;
        self.dropdown_field = Some(field_index);
        self.dropdown_options = options;
        self.dropdown_selected = selected;
    }

    /// Apply dropdown selection for workflow section
    pub fn apply_workflow_dropdown_selection(&mut self) {
        let Some(field_index) = self.dropdown_field else {
            self.close_dropdown();
            return;
        };

        let Some(selected_value) = self.dropdown_options.get(self.dropdown_selected).cloned() else {
            self.close_dropdown();
            return;
        };

        // Handle "(default)" or "(default: ProfileName)" selection - store empty string
        let value = if selected_value.starts_with("(default") {
            String::new()
        } else {
            selected_value
        };

        match field_index {
            0 => self.workflow_planner_llm = value,
            1 => self.workflow_worker_llm = value,
            _ => {}
        }
        self.modified = true;
        self.close_dropdown();
    }
}

/// LLM configuration form state
#[derive(Debug, Clone)]
pub struct LlmConfigForm {
    pub id: Option<String>,  // None for new, Some for editing existing
    pub name: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub api_base: String,
    pub temperature: String,
    pub max_tokens: String,
    pub selected_field: usize,
}

impl Default for LlmConfigForm {
    fn default() -> Self {
        // Empty defaults - actual values come from database via orca_install
        Self {
            id: None,
            name: String::new(),
            provider: String::new(),
            model: String::new(),
            api_key: String::new(),
            api_base: String::new(),
            temperature: String::new(),
            max_tokens: String::new(),
            selected_field: 0,
        }
    }
}

impl LlmConfigForm {
    pub fn field_count() -> usize {
        7 // name, provider, model, api_key, api_base, temperature, max_tokens
    }

    pub fn get_field_value(&self, index: usize) -> &str {
        match index {
            0 => &self.name,
            1 => &self.provider,
            2 => &self.model,
            3 => &self.api_key,
            4 => &self.api_base,
            5 => &self.temperature,
            6 => &self.max_tokens,
            _ => "",
        }
    }

    pub fn get_field_value_mut(&mut self, index: usize) -> &mut String {
        match index {
            0 => &mut self.name,
            1 => &mut self.provider,
            2 => &mut self.model,
            3 => &mut self.api_key,
            4 => &mut self.api_base,
            5 => &mut self.temperature,
            6 => &mut self.max_tokens,
            _ => &mut self.provider, // fallback
        }
    }

    pub fn field_name(index: usize) -> &'static str {
        match index {
            0 => "Name",
            1 => "Provider",
            2 => "Model",
            3 => "API Key",
            4 => "API Base URL",
            5 => "Temperature",
            6 => "Max Tokens",
            _ => "",
        }
    }

    pub fn providers() -> &'static [&'static str] {
        &["ollama", "openai", "claude", "claude-code", "deepseek", "grok", "gemini", "openrouter", "lmstudio", "llamacpp"]
    }
}

/// Application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub should_quit: bool,
}

/// Main application structure
pub struct App {
    pub state: AppState,
    pub focused: FocusedArea,
    pub active_tab: SidebarTab,
    pub health_report: Option<HealthReport>,

    // View mode - what's displayed in main content area
    pub view_mode: ViewMode,

    // Left side: conversation and prompts
    pub conversation: VecDeque<String>,
    pub prompt_lines: Vec<String>,
    pub prompt_cursor_line: usize,
    pub prompt_cursor_col: usize,
    pub conversation_scroll: u16,

    // Right sidebar content
    pub history: VecDeque<String>,
    pub todo_items: VecDeque<String>,
    pub bugs: VecDeque<String>,
    pub sidebar_selected: usize,
    pub sidebar_scroll: u16,

    // Status bar info
    pub current_model: String,
    pub tokens_used: u32,
    pub runtime: String,
    pub status: String,

    // Budget tracking
    pub active_budget: Option<String>,
    pub budget_usage: f64,
    pub budget_remaining: Option<f64>,
    pub budget_status: String,

    // LLM profile tracking
    pub llm_profile: Option<String>,
    pub planner_llm: Option<String>,
    pub worker_llm: Option<String>,

    // Menu management
    pub menu_state: MenuState,
    pub menu_selected_index: usize,
    pub dialog_state: DialogState,
    pub dialog: Option<Dialog>,

    // LLM configuration form
    pub llm_config_form: LlmConfigForm,
    pub pending_llm_save: bool,

    // Config editor form
    pub config_editor: ConfigEditorForm,
    pub pending_config_save: bool,

    // LLM Prompt service
    pub prompt_service: Option<crate::services::PromptService>,

    // User database connection
    pub user_db: Option<Arc<Database>>,

    // Pattern selection state
    pub patterns: Vec<PatternConfig>,
    pub selected_pattern_index: Option<usize>,
    pub active_pattern: Option<PatternConfig>,
    pub pending_pattern_load: bool,

    // Data management pending flags
    pub pending_backup: bool,
    pub pending_restore: bool,
    pub pending_export: bool,
    pub pending_import: bool,

    // Model discovery pending flag
    pub pending_model_query: bool,

    // Prompt submission pending flag
    pub pending_prompt_submit: bool,
    pub pending_prompt_text: String,
}

impl App {
    /// Create a new app instance
    pub fn new() -> Self {
        Self {
            state: AppState {
                should_quit: false,
            },
            focused: FocusedArea::Conversation,
            active_tab: SidebarTab::History,
            health_report: None,
            view_mode: ViewMode::Conversation,
            conversation: VecDeque::new(),
            prompt_lines: vec![String::new()],
            prompt_cursor_line: 0,
            prompt_cursor_col: 0,
            conversation_scroll: 0,
            history: VecDeque::new(),
            todo_items: VecDeque::new(),
            bugs: VecDeque::new(),
            sidebar_selected: 0,
            sidebar_scroll: 0,
            current_model: String::new(),  // Set from database
            tokens_used: 0,
            runtime: "0ms".to_string(),
            status: "Ready".to_string(),
            active_budget: None,
            budget_usage: 0.0,
            budget_remaining: None,
            budget_status: "No budget".to_string(),
            llm_profile: None,
            planner_llm: None,
            worker_llm: None,
            menu_state: MenuState::Closed,
            menu_selected_index: 0,
            dialog_state: DialogState::None,
            dialog: None,
            llm_config_form: LlmConfigForm::default(),
            pending_llm_save: false,
            config_editor: ConfigEditorForm::default(),
            pending_config_save: false,
            prompt_service: None,
            user_db: None,
            patterns: Vec::new(),
            selected_pattern_index: None,
            active_pattern: None,
            pending_pattern_load: false,
            pending_backup: false,
            pending_restore: false,
            pending_export: false,
            pending_import: false,
            pending_model_query: false,
            pending_prompt_submit: false,
            pending_prompt_text: String::new(),
        }
    }

    /// Open config editor
    pub async fn open_config_editor(&mut self) {
        // Load current config into the editor
        match crate::config::load_config().await {
            Ok(config) => {
                self.config_editor = ConfigEditorForm::default();
                self.config_editor.load_from_config(&config);
                self.view_mode = ViewMode::ConfigEditor;
                self.config_editor.section = ConfigSection::Llm;
                self.config_editor.field_index = 0;
                self.config_editor.button_focus = 0;

                // Load LLM profiles from database
                self.load_llm_profiles_into_editor().await;

                // Load available budgets from database
                self.load_available_budgets().await;

                // Load available workflows
                self.load_available_workflows();
            }
            Err(e) => {
                self.add_message(format!("Failed to load config: {}", e));
            }
        }
    }

    /// Load LLM profiles from database into config editor
    async fn load_llm_profiles_into_editor(&mut self) {
        let Some(db) = &self.user_db else {
            self.add_message("Database not connected. Run 'orca_install init' first.".to_string());
            return;
        };

        let repo = LlmProviderRepository::new(Arc::clone(db));

        match repo.list().await {
            Ok(providers) => {
                self.config_editor.llm_profiles = providers
                    .iter()
                    .map(LlmProfileEntry::from_provider_config)
                    .collect();

                // Populate available_llm_profile_names for workflow dropdowns
                self.config_editor.available_llm_profile_names = self.config_editor.llm_profiles
                    .iter()
                    .map(|p| p.name.clone())
                    .collect();

                // Set current_model from default profile
                if let Some(default) = self.config_editor.llm_profiles.iter().find(|p| p.is_default) {
                    self.current_model = format!("{}/{}", default.provider, default.model);
                    // Select the default profile in the editor
                    if let Some(idx) = self.config_editor.llm_profiles.iter().position(|p| p.is_default) {
                        self.config_editor.llm_selected_index = idx;
                    }
                }
                // If empty, llm_profiles stays empty - TUI will show setup message
            }
            Err(e) => {
                self.add_message(format!("Failed to load LLM profiles: {}", e));
            }
        }
    }

    /// Load available budgets from database into config editor
    async fn load_available_budgets(&mut self) {
        let Some(db) = &self.user_db else {
            return;
        };

        let repo = BudgetRepository::new(Arc::clone(db));

        match repo.list_all().await {
            Ok(budgets) => {
                // Start with "(none)" option
                let mut budget_names = vec!["(none)".to_string()];

                // Add budget names
                for budget in budgets {
                    budget_names.push(budget.name);
                }

                self.config_editor.available_budgets = budget_names;
            }
            Err(e) => {
                // Keep default "(none)" option on error
                self.add_message(format!("Failed to load budgets: {}", e));
            }
        }
    }

    /// Load available workflows from release/workflows directory
    fn load_available_workflows(&mut self) {
        let mut workflows = vec!["adaptive_react".to_string()];

        // Try to find workflows directory relative to executable or release path
        let workflow_paths = [
            std::path::PathBuf::from("release/lastbuild/workflows"),
            std::path::PathBuf::from("workflows"),
        ];

        for base_path in workflow_paths {
            if base_path.exists() {
                if let Ok(entries) = std::fs::read_dir(&base_path) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_dir() {
                            // Get category name
                            if let Some(category) = path.file_name().and_then(|n| n.to_str()) {
                                // Skip hidden directories
                                if category.starts_with('.') {
                                    continue;
                                }
                                // Scan for yaml files in category
                                if let Ok(yaml_files) = std::fs::read_dir(&path) {
                                    for yaml_entry in yaml_files.filter_map(|e| e.ok()) {
                                        let yaml_path = yaml_entry.path();
                                        if yaml_path.extension().map_or(false, |e| e == "yaml" || e == "yml") {
                                            if let Some(name) = yaml_path.file_stem().and_then(|n| n.to_str()) {
                                                workflows.push(format!("{}/{}", category, name));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                break;  // Use first found workflows directory
            }
        }

        self.config_editor.available_workflows = workflows;
    }

    /// Close config editor without saving
    pub fn close_config_editor(&mut self) {
        self.view_mode = ViewMode::Conversation;
        self.config_editor.modified = false;
        self.config_editor.editing = false;
    }

    /// Save config and close editor
    pub async fn save_and_close_config_editor(&mut self) {
        let config = self.config_editor.to_config();

        // Save to project config file (./.orca/orca.toml)
        let project_config_path = std::env::current_dir()
            .unwrap_or_default()
            .join(".orca")
            .join("orca.toml");

        // Ensure directory exists
        if let Some(parent) = project_config_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                self.add_message(format!("Failed to create config directory: {}", e));
                return;
            }
        }

        // Serialize and save
        match toml::to_string_pretty(&config) {
            Ok(toml_str) => {
                match tokio::fs::write(&project_config_path, toml_str).await {
                    Ok(()) => {
                        self.add_message(format!("Config saved to {}", project_config_path.display()));
                        self.config_editor.modified = false;
                        self.view_mode = ViewMode::Conversation;

                        // Reinitialize prompt service with new config
                        self.init_prompt_service().await;
                    }
                    Err(e) => {
                        self.add_message(format!("Failed to save config: {}", e));
                    }
                }
            }
            Err(e) => {
                self.add_message(format!("Failed to serialize config: {}", e));
            }
        }
    }

    /// Handle backup operation from TUI menu
    pub async fn handle_backup(&mut self) {
        use crate::services::BackupService;
        use crate::db::manager::DatabaseManager;

        // Load config to get backup directory
        match crate::load_config().await {
            Ok(config) => {
                let backup_service = BackupService::new(Some(config.backup_dir()));

                // Create database manager
                match DatabaseManager::new(".").await {
                    Ok(db_manager) => {
                        match backup_service.backup(&db_manager, true).await {
                            Ok(info) => {
                                let msg = format!(
                                    "Backup created: {}\nSize: {:.2} KB",
                                    info.path.file_name().and_then(|n| n.to_str()).unwrap_or("backup"),
                                    info.size_bytes as f64 / 1024.0
                                );
                                let dialog = super::dialog::Dialog::info("Backup Complete", &msg);
                                self.show_dialog(dialog);
                            }
                            Err(e) => {
                                let dialog = super::dialog::Dialog::info("Backup Failed", &format!("Error: {}", e));
                                self.show_dialog(dialog);
                            }
                        }
                    }
                    Err(e) => {
                        let dialog = super::dialog::Dialog::info("Backup Failed", &format!("Database error: {}", e));
                        self.show_dialog(dialog);
                    }
                }
            }
            Err(e) => {
                let dialog = super::dialog::Dialog::info("Backup Failed", &format!("Config error: {}", e));
                self.show_dialog(dialog);
            }
        }
    }

    /// Handle restore operation from TUI menu
    pub async fn handle_restore(&mut self) {
        use crate::services::BackupService;

        // Load config to get backup directory
        match crate::load_config().await {
            Ok(config) => {
                let backup_service = BackupService::new(Some(config.backup_dir()));

                match backup_service.list_backups() {
                    Ok(backups) => {
                        if backups.is_empty() {
                            let msg = format!(
                                "No backups found.\n\nBackup directory: {}",
                                config.backup_dir().display()
                            );
                            let dialog = super::dialog::Dialog::info("No Backups", &msg);
                            self.show_dialog(dialog);
                        } else {
                            let mut msg = String::from("Available backups:\n\n");
                            for backup in backups.iter().take(5) {
                                let filename = backup.path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown");
                                let size_kb = backup.size_bytes as f64 / 1024.0;
                                msg.push_str(&format!("  {} ({:.1} KB)\n", filename, size_kb));
                            }
                            if backups.len() > 5 {
                                msg.push_str(&format!("\n  ... and {} more\n", backups.len() - 5));
                            }
                            msg.push_str("\nUse CLI to restore:\n  orca data restore --file <backup>");
                            let dialog = super::dialog::Dialog::info("Restore", &msg);
                            self.show_dialog(dialog);
                        }
                    }
                    Err(e) => {
                        let dialog = super::dialog::Dialog::info("Restore Failed", &format!("Error: {}", e));
                        self.show_dialog(dialog);
                    }
                }
            }
            Err(e) => {
                let dialog = super::dialog::Dialog::info("Restore Failed", &format!("Config error: {}", e));
                self.show_dialog(dialog);
            }
        }
    }

    /// Handle export operation from TUI menu
    pub async fn handle_export(&mut self) {
        use crate::services::BackupService;
        use crate::db::manager::DatabaseManager;
        use chrono::Utc;

        match crate::load_config().await {
            Ok(config) => {
                let backup_service = BackupService::new(Some(config.backup_dir()));

                match DatabaseManager::new(".").await {
                    Ok(db_manager) => {
                        // Export all tables to a timestamped file
                        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
                        let output_path = std::path::PathBuf::from(format!("export_{}.sql", timestamp));
                        let tables = vec!["all".to_string()];

                        match backup_service.export(&db_manager, &tables, &output_path).await {
                            Ok(()) => {
                                let msg = format!("Export complete!\n\nFile: {}", output_path.display());
                                let dialog = super::dialog::Dialog::info("Export Complete", &msg);
                                self.show_dialog(dialog);
                            }
                            Err(e) => {
                                let dialog = super::dialog::Dialog::info("Export Failed", &format!("Error: {}", e));
                                self.show_dialog(dialog);
                            }
                        }
                    }
                    Err(e) => {
                        let dialog = super::dialog::Dialog::info("Export Failed", &format!("Database error: {}", e));
                        self.show_dialog(dialog);
                    }
                }
            }
            Err(e) => {
                let dialog = super::dialog::Dialog::info("Export Failed", &format!("Config error: {}", e));
                self.show_dialog(dialog);
            }
        }
    }

    /// Handle import operation from TUI menu
    pub async fn handle_import(&mut self) {
        // Import requires a file path - show instructions for CLI usage
        let msg = "Import requires specifying a file.\n\n\
            Use the CLI to import:\n\n\
            orca data import <file.sql>     # SQL dump\n\
            orca data import <backup.db>    # Database backup\n\n\
            Options:\n\
            --tables <list>  Import specific tables";
        let dialog = super::dialog::Dialog::info("Import", msg);
        self.show_dialog(dialog);
    }

    /// Handle prompt submission - send to LLM and display response
    pub async fn handle_prompt_submit(&mut self) {
        let prompt_text = std::mem::take(&mut self.pending_prompt_text);

        if prompt_text.trim().is_empty() {
            return;
        }

        // Check if prompt service is initialized
        if self.prompt_service.is_none() {
            self.add_message("Assistant:\nLLM service not configured. Please configure an LLM provider in Config → View Config → LLM.".to_string());
            return;
        }

        // Update status to show we're processing
        self.status = "Processing...".to_string();

        // Show thinking indicator
        self.add_message("Assistant:\n[Thinking...]".to_string());

        // Send prompt to LLM (take ownership temporarily to avoid borrow issues)
        let prompt_service = self.prompt_service.take().unwrap();
        let start = std::time::Instant::now();
        let result = prompt_service.send_prompt(&prompt_text).await;

        // Restore prompt service
        self.prompt_service = Some(prompt_service);

        match result {
            Ok(response) => {
                // Remove thinking indicator and show response
                self.conversation.pop_back();
                self.add_message(format!("Assistant:\n{}", response));

                // Update runtime
                let elapsed = start.elapsed();
                self.runtime = format!("{:.1}s", elapsed.as_secs_f64());
            }
            Err(e) => {
                // Remove thinking indicator and show error
                self.conversation.pop_back();
                self.add_message(format!("Assistant:\n[Error: {}]", e));
            }
        }

        // Reset status to Ready
        self.status = "Ready".to_string();
    }

    /// Initialize user database connection
    pub async fn init_user_db(&mut self) {
        let user_db_path = dirs::home_dir()
            .expect("Failed to get home directory")
            .join(".orca")
            .join("user.db");

        match Database::new(&user_db_path).await {
            Ok(db) => {
                self.user_db = Some(Arc::new(db));
            }
            Err(e) => {
                self.add_message(format!("Failed to connect to user database: {}", e));
            }
        }
    }

    /// Load current LLM config into the form from database
    pub async fn load_llm_config_form(&mut self) {
        let Some(db) = &self.user_db else {
            self.add_message("Database not initialized".to_string());
            return;
        };

        let repo = LlmProviderRepository::new(Arc::clone(db));

        match repo.get_default().await {
            Ok(provider) => {
                self.llm_config_form = LlmConfigForm {
                    id: Some(provider.id),
                    name: provider.name,
                    provider: provider.provider_type,
                    model: provider.model,
                    api_key: provider.api_key.unwrap_or_default(),
                    api_base: provider.api_base.unwrap_or_default(),
                    temperature: provider.temperature.to_string(),
                    max_tokens: provider.max_tokens.to_string(),
                    selected_field: 0,
                };
                // Update display
                self.current_model = format!("{}/{}", self.llm_config_form.provider, self.llm_config_form.model);
            }
            Err(_) => {
                // No default provider, use form defaults
                self.llm_config_form = LlmConfigForm::default();
                self.add_message("No LLM provider configured. Using defaults.".to_string());
            }
        }
    }

    /// Save LLM config from config editor to database
    pub async fn save_llm_config(&mut self) -> bool {
        let Some(db) = &self.user_db else {
            self.add_message("Database not initialized".to_string());
            return false;
        };

        // Get the selected profile from config editor (not the legacy llm_config_form)
        let Some(profile) = self.config_editor.selected_llm_profile() else {
            self.add_message("No LLM profile selected".to_string());
            return false;
        };

        let repo = LlmProviderRepository::new(Arc::clone(db));
        let now = Utc::now().timestamp();

        // Build the provider config from the selected profile
        let provider = LlmProviderConfig {
            id: profile.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: profile.name.clone(),
            provider_type: profile.provider.clone(),
            model: profile.model.clone(),
            api_key: if profile.api_key.is_empty() {
                None
            } else {
                Some(profile.api_key.clone())
            },
            api_base: if profile.api_base.is_empty() {
                None
            } else {
                Some(profile.api_base.clone())
            },
            temperature: profile.temperature.parse().unwrap_or(0.7),
            max_tokens: profile.max_tokens.parse().unwrap_or(4096),
            settings: String::new(),
            is_default: profile.is_default,
            created_at: now,
            updated_at: now,
        };

        // Save or update based on whether we have an existing ID
        let has_id = profile.id.is_some();
        let result = if has_id {
            repo.update(&provider).await
        } else {
            // New provider - save and optionally set as default
            match repo.save(&provider).await {
                Ok(()) => {
                    // Update the profile with the new ID
                    if let Some(p) = self.config_editor.selected_llm_profile_mut() {
                        p.id = Some(provider.id.clone());
                    }
                    if provider.is_default {
                        repo.set_default(&provider.id).await
                    } else {
                        Ok(())
                    }
                }
                Err(e) => Err(e),
            }
        };

        match result {
            Ok(()) => {
                self.add_message("LLM configuration saved to database".to_string());
                self.current_model = format!("{}/{}", provider.provider_type, provider.model);
                self.config_editor.modified = false;

                // Reinitialize prompt service with new config and LLM provider
                if let Ok(config) = crate::config::load_config().await {
                    // Create LlmProvider from the saved provider config
                    match crate::executor::LlmProvider::from_params(
                        &provider.provider_type,
                        &provider.model,
                        provider.api_key.as_deref(),
                        provider.api_base.as_deref(),
                    ) {
                        Ok(llm_provider) => {
                            let llm_provider = std::sync::Arc::new(llm_provider);
                            match crate::services::PromptService::new(&config, llm_provider) {
                                Ok(mut service) => {
                                    // Suppress stdout output for TUI mode
                                    service.set_suppress_stdout(true);
                                    self.prompt_service = Some(service);
                                    self.add_message("LLM service reinitialized".to_string());
                                }
                                Err(e) => {
                                    self.add_message(format!("Failed to reinitialize LLM: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.add_message(format!("Failed to create LLM provider: {}", e));
                        }
                    }
                }
                true
            }
            Err(e) => {
                self.add_message(format!("Failed to save LLM config: {}", e));
                false
            }
        }
    }

    /// Initialize the prompt service from configuration (async)
    pub async fn init_prompt_service(&mut self) {
        match crate::config::load_config().await {
            Ok(config) => {
                // Get default LLM profile entry from loaded profiles
                if let Some(profile) = self.config_editor.llm_profiles.iter().find(|p| p.is_default).or(self.config_editor.llm_profiles.first()) {
                    // Create LlmProviderConfig from profile entry
                    let provider_config = LlmProviderConfig::new(
                        profile.name.clone(),
                        profile.provider.clone(),
                        profile.model.clone(),
                    )
                    .with_temperature(profile.temperature.parse().unwrap_or(0.7))
                    .with_max_tokens(profile.max_tokens.parse().unwrap_or(4096));

                    let provider_config = if !profile.api_key.is_empty() {
                        provider_config.with_api_key(profile.api_key.clone())
                    } else {
                        provider_config
                    };

                    let provider_config = if !profile.api_base.is_empty() {
                        provider_config.with_api_base(profile.api_base.clone())
                    } else {
                        provider_config
                    };

                    match crate::executor::LlmProvider::from_provider_config(&provider_config) {
                        Ok(llm_provider) => {
                            let llm_provider = std::sync::Arc::new(llm_provider);
                            match crate::services::PromptService::new(&config, llm_provider) {
                                Ok(mut service) => {
                                    // Suppress stdout output for TUI mode
                                    service.set_suppress_stdout(true);
                                    self.prompt_service = Some(service);
                                }
                                Err(e) => {
                                    self.add_message(format!("Failed to initialize LLM: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.add_message(format!("Failed to create LLM provider: {}", e));
                        }
                    }
                } else {
                    self.add_message("No LLM profile configured. Use 'orca llm-profile create' to add one.".to_string());
                }
            }
            Err(e) => {
                self.add_message(format!("Failed to load config: {}", e));
            }
        }
    }

    /// Load patterns from database
    pub async fn load_patterns(&mut self) {
        let Some(db) = &self.user_db else {
            self.add_message("Database not initialized".to_string());
            return;
        };

        let repo = PatternConfigRepository::new(Arc::clone(db));

        match repo.list().await {
            Ok(patterns) => {
                // Find default pattern index
                let default_index = patterns.iter().position(|p| p.is_default);
                self.patterns = patterns;
                self.selected_pattern_index = default_index;

                // Set active pattern to default
                if let Some(idx) = default_index {
                    self.active_pattern = self.patterns.get(idx).cloned();
                }
            }
            Err(e) => {
                self.add_message(format!("Failed to load patterns: {}", e));
            }
        }
    }

    /// Select a pattern by index
    pub fn select_pattern(&mut self, index: usize) {
        if index < self.patterns.len() {
            self.selected_pattern_index = Some(index);
            self.active_pattern = self.patterns.get(index).cloned();
        }
    }

    /// Get the currently selected pattern name for display
    pub fn get_active_pattern_display(&self) -> String {
        self.active_pattern
            .as_ref()
            .map(|p| format!("{} ({})", p.name, p.pattern_type))
            .unwrap_or_else(|| "Auto".to_string())
    }

    /// Move pattern selection up in list
    pub fn pattern_select_prev(&mut self) {
        if self.patterns.is_empty() {
            return;
        }

        let current = self.selected_pattern_index.unwrap_or(0);
        let new_index = if current > 0 {
            current - 1
        } else {
            self.patterns.len() - 1
        };
        self.selected_pattern_index = Some(new_index);
    }

    /// Move pattern selection down in list
    pub fn pattern_select_next(&mut self) {
        if self.patterns.is_empty() {
            return;
        }

        let current = self.selected_pattern_index.unwrap_or(0);
        let new_index = (current + 1) % self.patterns.len();
        self.selected_pattern_index = Some(new_index);
    }

    /// Confirm pattern selection
    pub fn confirm_pattern_selection(&mut self) {
        if let Some(idx) = self.selected_pattern_index {
            self.active_pattern = self.patterns.get(idx).cloned();
            if let Some(ref pattern) = self.active_pattern {
                self.add_message(format!("Pattern selected: {} ({})", pattern.name, pattern.pattern_type));
            }
        }
        self.dialog_state = DialogState::None;
    }

    /// Add a message to conversation
    pub fn add_message(&mut self, message: String) {
        self.conversation.push_back(message);
        while self.conversation.len() > MAX_ENTRIES {
            self.conversation.pop_front();
        }
    }

    /// Add to history
    pub fn add_history(&mut self, entry: String) {
        self.history.push_back(entry);
        while self.history.len() > MAX_ENTRIES {
            self.history.pop_front();
        }
    }

    /// Add todo item
    pub fn add_todo(&mut self, item: String) {
        self.todo_items.push_back(item);
    }

    /// Add bug
    pub fn add_bug(&mut self, bug: String) {
        self.bugs.push_back(bug);
    }

    /// Switch sidebar tab
    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            SidebarTab::History => SidebarTab::Todo,
            SidebarTab::Todo => SidebarTab::Bugs,
            SidebarTab::Bugs => SidebarTab::Patterns,
            SidebarTab::Patterns => SidebarTab::History,
        };
        self.sidebar_selected = 0;
        self.sidebar_scroll = 0;
    }

    /// Switch to previous tab
    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            SidebarTab::History => SidebarTab::Patterns,
            SidebarTab::Todo => SidebarTab::History,
            SidebarTab::Bugs => SidebarTab::Todo,
            SidebarTab::Patterns => SidebarTab::Bugs,
        };
        self.sidebar_selected = 0;
        self.sidebar_scroll = 0;
    }

    /// Move focus between areas
    pub fn next_focus(&mut self) {
        self.focused = match self.focused {
            FocusedArea::Conversation => FocusedArea::Prompts,
            FocusedArea::Prompts => FocusedArea::Sidebar,
            FocusedArea::Sidebar => FocusedArea::Conversation,
            FocusedArea::Menu => FocusedArea::Conversation,
        };
    }

    /// Move focus to previous area
    pub fn prev_focus(&mut self) {
        self.focused = match self.focused {
            FocusedArea::Conversation => FocusedArea::Sidebar,
            FocusedArea::Prompts => FocusedArea::Conversation,
            FocusedArea::Sidebar => FocusedArea::Prompts,
            FocusedArea::Menu => FocusedArea::Sidebar,
        };
    }

    /// Scroll conversation down
    pub fn scroll_conversation_down(&mut self) {
        self.conversation_scroll = self.conversation_scroll.saturating_add(1);
    }

    /// Scroll conversation up
    pub fn scroll_conversation_up(&mut self) {
        self.conversation_scroll = self.conversation_scroll.saturating_sub(1);
    }

    /// Scroll sidebar down
    pub fn scroll_sidebar_down(&mut self) {
        self.sidebar_scroll = self.sidebar_scroll.saturating_add(1);
    }

    /// Scroll sidebar up
    pub fn scroll_sidebar_up(&mut self) {
        self.sidebar_scroll = self.sidebar_scroll.saturating_sub(1);
    }

    /// Move sidebar selection down
    pub fn sidebar_next(&mut self) {
        self.sidebar_selected = self.sidebar_selected.saturating_add(1);
    }

    /// Move sidebar selection up
    pub fn sidebar_prev(&mut self) {
        self.sidebar_selected = self.sidebar_selected.saturating_sub(1);
    }

    /// Clear conversation
    pub fn clear_conversation(&mut self) {
        self.conversation.clear();
        self.conversation_scroll = 0;
    }

    /// Get full prompt text
    pub fn get_prompt_text(&self) -> String {
        self.prompt_lines.join("\n")
    }

    /// Add character to prompt at cursor position
    pub fn add_prompt_char(&mut self, c: char) {
        if self.prompt_cursor_line < self.prompt_lines.len() {
            self.prompt_lines[self.prompt_cursor_line].insert(self.prompt_cursor_col, c);
            self.prompt_cursor_col += 1;
        }
    }

    /// Remove character before cursor in prompt
    pub fn backspace_prompt(&mut self) {
        if self.prompt_cursor_line < self.prompt_lines.len() {
            if self.prompt_cursor_col > 0 {
                self.prompt_lines[self.prompt_cursor_line].remove(self.prompt_cursor_col - 1);
                self.prompt_cursor_col -= 1;
            } else if self.prompt_cursor_line > 0 {
                // Move to end of previous line
                let line = self.prompt_lines.remove(self.prompt_cursor_line);
                self.prompt_cursor_line -= 1;
                self.prompt_cursor_col = self.prompt_lines[self.prompt_cursor_line].len();
                self.prompt_lines[self.prompt_cursor_line].push_str(&line);
            }
        }
    }

    /// Add newline in prompt (max 3 lines)
    pub fn newline_prompt(&mut self) {
        if self.prompt_lines.len() < 3 && self.prompt_cursor_line < self.prompt_lines.len() {
            let rest = self.prompt_lines[self.prompt_cursor_line].split_off(self.prompt_cursor_col);
            self.prompt_lines.insert(self.prompt_cursor_line + 1, rest);
            self.prompt_cursor_line += 1;
            self.prompt_cursor_col = 0;
        }
    }

    /// Move cursor left
    pub fn prompt_cursor_left(&mut self) {
        if self.prompt_cursor_col > 0 {
            self.prompt_cursor_col -= 1;
        } else if self.prompt_cursor_line > 0 {
            self.prompt_cursor_line -= 1;
            self.prompt_cursor_col = self.prompt_lines[self.prompt_cursor_line].len();
        }
    }

    /// Move cursor right
    pub fn prompt_cursor_right(&mut self) {
        if self.prompt_cursor_line < self.prompt_lines.len() {
            let line_len = self.prompt_lines[self.prompt_cursor_line].len();
            if self.prompt_cursor_col < line_len {
                self.prompt_cursor_col += 1;
            } else if self.prompt_cursor_line < self.prompt_lines.len() - 1 {
                self.prompt_cursor_line += 1;
                self.prompt_cursor_col = 0;
            }
        }
    }

    /// Clear prompt
    pub fn clear_prompt(&mut self) {
        self.prompt_lines = vec![String::new()];
        self.prompt_cursor_line = 0;
        self.prompt_cursor_col = 0;
    }

    /// Set active budget and usage information
    pub fn set_budget(&mut self, name: String, usage: f64, remaining: Option<f64>) {
        self.active_budget = Some(name);
        self.budget_usage = usage;
        self.budget_remaining = remaining;
        self.budget_status = if usage >= 100.0 {
            "Budget exceeded".to_string()
        } else if usage >= 80.0 {
            "Budget near limit".to_string()
        } else {
            "Budget OK".to_string()
        };
    }

    /// Clear active budget
    pub fn clear_budget(&mut self) {
        self.active_budget = None;
        self.budget_usage = 0.0;
        self.budget_remaining = None;
        self.budget_status = "No budget".to_string();
    }

    /// Set LLM profile configuration
    pub fn set_llm_profile(
        &mut self,
        profile_name: Option<String>,
        planner: Option<String>,
        worker: Option<String>,
    ) {
        self.llm_profile = profile_name;
        self.planner_llm = planner;
        self.worker_llm = worker;
    }

    /// Clear LLM profile configuration
    pub fn clear_llm_profile(&mut self) {
        self.llm_profile = None;
        self.planner_llm = None;
        self.worker_llm = None;
    }

    // === Menu Management Methods ===

    /// Open a menu
    pub fn open_menu(&mut self, menu: MenuState) {
        self.menu_state = menu;
        self.menu_selected_index = 0;
        self.focused = FocusedArea::Menu;
    }

    /// Close the current menu
    pub fn close_menu(&mut self) {
        self.menu_state = MenuState::Closed;
        self.menu_selected_index = 0;
        // Don't change focus if a dialog is open (dialog should have focus)
        if !self.has_dialog() {
            self.focused = FocusedArea::Conversation;
        }
    }

    /// Move to next menu item
    pub fn menu_next(&mut self) {
        let max_items = self.get_menu_items_count();
        if max_items > 0 {
            self.menu_selected_index = (self.menu_selected_index + 1) % max_items;
        }
    }

    /// Move to previous menu item
    pub fn menu_prev(&mut self) {
        let max_items = self.get_menu_items_count();
        if max_items > 0 {
            self.menu_selected_index = if self.menu_selected_index > 0 {
                self.menu_selected_index - 1
            } else {
                max_items - 1
            };
        }
    }

    /// Switch to next menu (right arrow navigation)
    pub fn next_menu(&mut self) {
        self.menu_state = match self.menu_state {
            MenuState::Closed => MenuState::FileOpen,
            MenuState::FileOpen => MenuState::EditOpen,
            MenuState::EditOpen => MenuState::ConfigOpen,
            MenuState::ConfigOpen => MenuState::WorkflowOpen,
            MenuState::WorkflowOpen => MenuState::HelpOpen,
            MenuState::HelpOpen => MenuState::FileOpen, // Wrap around
        };
        self.menu_selected_index = 0;
        self.focused = FocusedArea::Menu;
    }

    /// Switch to previous menu (left arrow navigation)
    pub fn prev_menu(&mut self) {
        self.menu_state = match self.menu_state {
            MenuState::Closed => MenuState::HelpOpen,
            MenuState::FileOpen => MenuState::HelpOpen, // Wrap around
            MenuState::EditOpen => MenuState::FileOpen,
            MenuState::ConfigOpen => MenuState::EditOpen,
            MenuState::WorkflowOpen => MenuState::ConfigOpen,
            MenuState::HelpOpen => MenuState::WorkflowOpen,
        };
        self.menu_selected_index = 0;
        self.focused = FocusedArea::Menu;
    }

    /// Get the count of items in the current menu
    fn get_menu_items_count(&self) -> usize {
        match self.menu_state {
            MenuState::Closed => 0,
            MenuState::FileOpen => 8,      // New, Open, Save, Backup, Restore, Export, Import, Quit
            MenuState::EditOpen => 5,      // Build, Update, Refine, Purge, Search
            MenuState::ConfigOpen => 4,    // View Config, Budget, Pattern, Editor
            MenuState::WorkflowOpen => 4,  // Run, View, Create, Manage
            MenuState::HelpOpen => 3,      // About, Shortcuts, Documentation
        }
    }

    /// Get the selected menu item action
    pub fn get_selected_menu_action(&self) -> Option<String> {
        match self.menu_state {
            MenuState::Closed => None,
            MenuState::FileOpen => match self.menu_selected_index {
                0 => Some("file_init".to_string()),
                1 => Some("file_update".to_string()),
                2 => Some("file_save".to_string()),
                3 => Some("file_backup".to_string()),
                4 => Some("file_restore".to_string()),
                5 => Some("file_export".to_string()),
                6 => Some("file_import".to_string()),
                7 => Some("file_quit".to_string()),
                _ => None,
            },
            MenuState::EditOpen => match self.menu_selected_index {
                0 => Some("edit_build".to_string()),
                1 => Some("edit_update".to_string()),
                2 => Some("edit_refine".to_string()),
                3 => Some("edit_purge".to_string()),
                4 => Some("edit_search".to_string()),
                _ => None,
            },
            MenuState::ConfigOpen => match self.menu_selected_index {
                0 => Some("config_view".to_string()),
                1 => Some("config_budget".to_string()),
                2 => Some("config_pattern".to_string()),
                3 => Some("config_editor".to_string()),
                _ => None,
            },
            MenuState::WorkflowOpen => match self.menu_selected_index {
                0 => Some("workflow_run".to_string()),
                1 => Some("workflow_view".to_string()),
                2 => Some("workflow_create".to_string()),
                3 => Some("workflow_manage".to_string()),
                _ => None,
            },
            MenuState::HelpOpen => match self.menu_selected_index {
                0 => Some("help_about".to_string()),
                1 => Some("help_shortcuts".to_string()),
                2 => Some("help_documentation".to_string()),
                _ => None,
            },
        }
    }

    // === Dialog Management Methods ===

    /// Show a dialog
    pub fn show_dialog(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
        self.focused = FocusedArea::Menu; // Change focus to dialog
    }

    /// Close the current dialog
    pub fn close_dialog(&mut self) {
        self.dialog = None;
        self.dialog_state = DialogState::None;
    }

    /// Navigate up in dialog (for list and confirmation dialogs)
    pub fn dialog_prev(&mut self) {
        if let Some(ref mut dialog) = self.dialog {
            dialog.select_prev();
        }
    }

    /// Navigate down in dialog (for list and confirmation dialogs)
    pub fn dialog_next(&mut self) {
        if let Some(ref mut dialog) = self.dialog {
            dialog.select_next();
        }
    }

    /// Add character to dialog input
    pub fn dialog_add_char(&mut self, c: char) {
        if let Some(ref mut dialog) = self.dialog {
            dialog.add_char(c);
        }
    }

    /// Backspace in dialog input
    pub fn dialog_backspace(&mut self) {
        if let Some(ref mut dialog) = self.dialog {
            dialog.backspace();
        }
    }

    /// Get selected option from dialog
    pub fn dialog_selected_option(&self) -> Option<&str> {
        self.dialog.as_ref().and_then(|d| d.selected_option())
    }

    /// Get input from text input dialog
    pub fn dialog_get_input(&self) -> Option<String> {
        self.dialog.as_ref().map(|d| d.get_input())
    }

    /// Check if dialog is open
    pub fn has_dialog(&self) -> bool {
        self.dialog.is_some()
    }

    /// Query available models from the current provider
    pub async fn query_provider_models(&mut self) {
        // Get the current provider and API base from the selected profile
        let (provider, api_base) = {
            let profile = self.config_editor.selected_llm_profile();
            match profile {
                Some(p) => {
                    let api_base = if p.api_base.is_empty() { None } else { Some(p.api_base.as_str()) };
                    (p.provider.clone(), api_base.map(String::from))
                }
                None => return,
            }
        };

        // Query models from the provider
        let discovery = ModelDiscoveryService::new();
        let models = discovery.query_models(&provider, api_base.as_deref()).await;

        // Update dropdown options with queried models
        if !models.is_empty() {
            self.config_editor.dropdown_options = models;

            // Find current model in the list and pre-select it
            if let Some(profile) = self.config_editor.selected_llm_profile() {
                if let Some(idx) = self.config_editor.dropdown_options.iter().position(|m| m == &profile.model) {
                    self.config_editor.dropdown_selected = idx;
                }
            }
        }

        self.pending_model_query = false;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initialization() {
        let app = App::new();
        assert_eq!(app.focused, FocusedArea::Conversation);
        assert_eq!(app.active_tab, SidebarTab::History);
        assert_eq!(app.menu_state, MenuState::Closed);
        assert!(!app.state.should_quit);
        assert!(app.conversation.is_empty());
        assert_eq!(app.prompt_lines, vec![String::new()]);
    }

    #[test]
    fn test_focus_navigation_forward() {
        let mut app = App::new();
        assert_eq!(app.focused, FocusedArea::Conversation);

        app.next_focus();
        assert_eq!(app.focused, FocusedArea::Prompts);

        app.next_focus();
        assert_eq!(app.focused, FocusedArea::Sidebar);

        app.next_focus();
        assert_eq!(app.focused, FocusedArea::Conversation); // Wraps around
    }

    #[test]
    fn test_focus_navigation_backward() {
        let mut app = App::new();
        assert_eq!(app.focused, FocusedArea::Conversation);

        app.prev_focus();
        assert_eq!(app.focused, FocusedArea::Sidebar);

        app.prev_focus();
        assert_eq!(app.focused, FocusedArea::Prompts);

        app.prev_focus();
        assert_eq!(app.focused, FocusedArea::Conversation);
    }

    #[test]
    fn test_sidebar_tab_navigation() {
        let mut app = App::new();
        assert_eq!(app.active_tab, SidebarTab::History);

        app.next_tab();
        assert_eq!(app.active_tab, SidebarTab::Todo);

        app.next_tab();
        assert_eq!(app.active_tab, SidebarTab::Bugs);

        app.next_tab();
        assert_eq!(app.active_tab, SidebarTab::Patterns);

        app.next_tab();
        assert_eq!(app.active_tab, SidebarTab::History); // Wraps around

        // Test backward navigation
        app.prev_tab();
        assert_eq!(app.active_tab, SidebarTab::Patterns);
    }

    #[test]
    fn test_menu_open_close() {
        let mut app = App::new();
        assert_eq!(app.menu_state, MenuState::Closed);

        app.open_menu(MenuState::FileOpen);
        assert_eq!(app.menu_state, MenuState::FileOpen);
        assert_eq!(app.focused, FocusedArea::Menu);
        assert_eq!(app.menu_selected_index, 0);

        app.close_menu();
        assert_eq!(app.menu_state, MenuState::Closed);
        assert_eq!(app.focused, FocusedArea::Conversation);
    }

    #[test]
    fn test_menu_navigation() {
        let mut app = App::new();
        app.open_menu(MenuState::FileOpen);

        // File menu has 4 items
        app.menu_next();
        assert_eq!(app.menu_selected_index, 1);

        app.menu_next();
        assert_eq!(app.menu_selected_index, 2);

        app.menu_next();
        assert_eq!(app.menu_selected_index, 3);

        app.menu_next();
        assert_eq!(app.menu_selected_index, 0); // Wraps around

        app.menu_prev();
        assert_eq!(app.menu_selected_index, 3);
    }

    #[test]
    fn test_menu_switching() {
        let mut app = App::new();
        app.open_menu(MenuState::FileOpen);

        app.next_menu();
        assert_eq!(app.menu_state, MenuState::EditOpen);

        app.next_menu();
        assert_eq!(app.menu_state, MenuState::ConfigOpen);

        app.prev_menu();
        assert_eq!(app.menu_state, MenuState::EditOpen);

        app.prev_menu();
        assert_eq!(app.menu_state, MenuState::FileOpen);
    }

    #[test]
    fn test_prompt_input() {
        let mut app = App::new();

        app.add_prompt_char('H');
        app.add_prompt_char('i');
        assert_eq!(app.get_prompt_text(), "Hi");

        app.add_prompt_char('!');
        assert_eq!(app.get_prompt_text(), "Hi!");

        app.clear_prompt();
        assert_eq!(app.get_prompt_text(), "");
        assert_eq!(app.prompt_cursor_col, 0);
        assert_eq!(app.prompt_cursor_line, 0);
    }

    #[test]
    fn test_prompt_backspace() {
        let mut app = App::new();

        app.add_prompt_char('H');
        app.add_prompt_char('e');
        app.add_prompt_char('y');
        assert_eq!(app.get_prompt_text(), "Hey");

        app.backspace_prompt();
        assert_eq!(app.get_prompt_text(), "He");

        app.backspace_prompt();
        assert_eq!(app.get_prompt_text(), "H");
    }

    #[test]
    fn test_prompt_newline() {
        let mut app = App::new();

        app.add_prompt_char('L');
        app.add_prompt_char('1');
        app.newline_prompt();

        assert_eq!(app.prompt_lines.len(), 2);
        assert_eq!(app.prompt_cursor_line, 1);
        assert_eq!(app.get_prompt_text(), "L1\n");

        app.add_prompt_char('L');
        app.add_prompt_char('2');
        assert_eq!(app.get_prompt_text(), "L1\nL2");
    }

    #[test]
    fn test_prompt_max_lines() {
        let mut app = App::new();

        // Max 3 lines allowed
        app.newline_prompt(); // Line 2
        app.newline_prompt(); // Line 3
        app.newline_prompt(); // Should be ignored (already at 3)

        assert_eq!(app.prompt_lines.len(), 3);
    }

    #[test]
    fn test_conversation_messages() {
        let mut app = App::new();

        app.add_message("Test message 1".to_string());
        assert_eq!(app.conversation.len(), 1);

        app.add_message("Test message 2".to_string());
        assert_eq!(app.conversation.len(), 2);

        app.clear_conversation();
        assert_eq!(app.conversation.len(), 0);
    }

    #[test]
    fn test_history_tracking() {
        let mut app = App::new();

        app.add_history("First prompt".to_string());
        app.add_history("Second prompt".to_string());

        assert_eq!(app.history.len(), 2);
        assert_eq!(app.history[0], "First prompt");
        assert_eq!(app.history[1], "Second prompt");
    }

    #[test]
    fn test_sidebar_navigation() {
        let mut app = App::new();

        app.sidebar_next();
        assert_eq!(app.sidebar_selected, 1);

        app.sidebar_next();
        assert_eq!(app.sidebar_selected, 2);

        app.sidebar_prev();
        assert_eq!(app.sidebar_selected, 1);

        app.sidebar_prev();
        assert_eq!(app.sidebar_selected, 0);

        // Should not go negative
        app.sidebar_prev();
        assert_eq!(app.sidebar_selected, 0);
    }

    #[test]
    fn test_get_selected_menu_action() {
        let mut app = App::new();
        app.open_menu(MenuState::FileOpen);

        assert_eq!(app.get_selected_menu_action(), Some("file_init".to_string()));

        app.menu_next();
        assert_eq!(app.get_selected_menu_action(), Some("file_update".to_string()));

        app.menu_next();
        assert_eq!(app.get_selected_menu_action(), Some("file_save".to_string()));

        app.menu_next();
        assert_eq!(app.get_selected_menu_action(), Some("file_backup".to_string()));
    }

    #[test]
    fn test_scroll_operations() {
        let mut app = App::new();

        app.scroll_conversation_down();
        assert_eq!(app.conversation_scroll, 1);

        app.scroll_conversation_down();
        assert_eq!(app.conversation_scroll, 2);

        app.scroll_conversation_up();
        assert_eq!(app.conversation_scroll, 1);

        app.scroll_conversation_up();
        assert_eq!(app.conversation_scroll, 0);

        // Should not go negative
        app.scroll_conversation_up();
        assert_eq!(app.conversation_scroll, 0);
    }

    #[test]
    fn test_prompt_cursor_movement() {
        let mut app = App::new();

        app.add_prompt_char('A');
        app.add_prompt_char('B');
        app.add_prompt_char('C');
        assert_eq!(app.prompt_cursor_col, 3);

        app.prompt_cursor_left();
        assert_eq!(app.prompt_cursor_col, 2);

        app.prompt_cursor_left();
        assert_eq!(app.prompt_cursor_col, 1);

        app.prompt_cursor_right();
        assert_eq!(app.prompt_cursor_col, 2);

        // Can't go past end
        app.prompt_cursor_right();
        app.prompt_cursor_right();
        assert_eq!(app.prompt_cursor_col, 3);
    }

    #[test]
    fn test_budget_management() {
        let mut app = App::new();

        app.set_budget("Test Budget".to_string(), 50.0, Some(100.0));
        assert_eq!(app.active_budget, Some("Test Budget".to_string()));
        assert_eq!(app.budget_usage, 50.0);
        assert_eq!(app.budget_remaining, Some(100.0));
        assert_eq!(app.budget_status, "Budget OK");

        app.set_budget("High Usage".to_string(), 85.0, Some(30.0));
        assert_eq!(app.budget_status, "Budget near limit");

        app.set_budget("Over Budget".to_string(), 110.0, None);
        assert_eq!(app.budget_status, "Budget exceeded");

        app.clear_budget();
        assert!(app.active_budget.is_none());
        assert_eq!(app.budget_status, "No budget");
    }

    #[test]
    fn test_llm_profile_management() {
        let mut app = App::new();

        app.set_llm_profile(
            Some("Multi-Model".to_string()),
            Some("Claude".to_string()),
            Some("GPT-4".to_string()),
        );

        assert_eq!(app.llm_profile, Some("Multi-Model".to_string()));
        assert_eq!(app.planner_llm, Some("Claude".to_string()));
        assert_eq!(app.worker_llm, Some("GPT-4".to_string()));

        app.clear_llm_profile();
        assert!(app.llm_profile.is_none());
        assert!(app.planner_llm.is_none());
        assert!(app.worker_llm.is_none());
    }

    #[test]
    fn test_quit_flag() {
        let mut app = App::new();
        assert!(!app.state.should_quit);

        app.state.should_quit = true;
        assert!(app.state.should_quit);
    }
}
