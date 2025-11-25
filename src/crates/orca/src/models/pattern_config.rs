//! Pattern configuration model
//!
//! Defines the PatternConfig model for storing dynamic workflow configurations
//! in the database. This enables task-specific pattern selection and configuration.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Pattern type enumeration matching database constraint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    /// ReAct (Reasoning + Acting) - Most common pattern
    React,
    /// Plan-Execute - Explicit planning before execution
    PlanExecute,
    /// Reflection - Generate, critique, refine loop
    Reflection,
    /// LATS (Language Agent Tree Search)
    Lats,
    /// STORM (Structured Research)
    Storm,
    /// CodeAct (Code generation and execution)
    CodeAct,
    /// Tree of Thought
    Tot,
    /// Chain of Thought
    Cot,
    /// Graph of Thought
    Got,
}

impl PatternType {
    /// Get string representation for database
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::React => "react",
            Self::PlanExecute => "plan_execute",
            Self::Reflection => "reflection",
            Self::Lats => "lats",
            Self::Storm => "storm",
            Self::CodeAct => "code_act",
            Self::Tot => "tot",
            Self::Cot => "cot",
            Self::Got => "got",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "react" => Some(Self::React),
            "plan_execute" | "plan-execute" => Some(Self::PlanExecute),
            "reflection" => Some(Self::Reflection),
            "lats" => Some(Self::Lats),
            "storm" => Some(Self::Storm),
            "code_act" | "codeact" => Some(Self::CodeAct),
            "tot" => Some(Self::Tot),
            "cot" => Some(Self::Cot),
            "got" => Some(Self::Got),
            _ => None,
        }
    }

    /// Get human-readable name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::React => "ReAct",
            Self::PlanExecute => "Plan-Execute",
            Self::Reflection => "Reflection",
            Self::Lats => "LATS",
            Self::Storm => "STORM",
            Self::CodeAct => "CodeAct",
            Self::Tot => "Tree of Thought",
            Self::Cot => "Chain of Thought",
            Self::Got => "Graph of Thought",
        }
    }
}

impl std::fmt::Display for PatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl From<&str> for PatternType {
    fn from(s: &str) -> Self {
        Self::from_str(s).unwrap_or(Self::React)
    }
}

/// Pattern configuration stored in database
///
/// Represents a reusable workflow pattern configuration that can be
/// associated with tasks for dynamic agent behavior.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PatternConfig {
    /// Unique configuration identifier (UUID or slug)
    pub id: String,

    /// Human-readable name (unique)
    pub name: String,

    /// Pattern type (react, plan_execute, reflection, etc.)
    pub pattern_type: String,

    /// Extended configuration as JSON
    /// Contains pattern-specific settings like temperature, quality_threshold, etc.
    pub config: String,

    /// Tools available for this configuration as JSON array
    pub tools: String,

    /// Optional system prompt override
    pub system_prompt: Option<String>,

    /// Maximum iterations for the pattern
    pub max_iterations: i64,

    /// Whether this is a default configuration
    pub is_default: bool,

    /// Number of times this configuration has been used
    pub usage_count: i64,

    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,
}

impl PatternConfig {
    /// Create a new pattern configuration
    pub fn new(name: impl Into<String>, pattern_type: PatternType) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            pattern_type: pattern_type.as_str().to_string(),
            config: "{}".to_string(),
            tools: "[]".to_string(),
            system_prompt: None,
            max_iterations: 10,
            is_default: false,
            usage_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create with a specific ID (for predefined configs)
    pub fn with_id(id: impl Into<String>, name: impl Into<String>, pattern_type: PatternType) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: id.into(),
            name: name.into(),
            pattern_type: pattern_type.as_str().to_string(),
            config: "{}".to_string(),
            tools: "[]".to_string(),
            system_prompt: None,
            max_iterations: 10,
            is_default: false,
            usage_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set configuration JSON
    pub fn with_config(mut self, config: impl Into<String>) -> Self {
        self.config = config.into();
        self
    }

    /// Set tools list
    pub fn with_tools(mut self, tools: Vec<&str>) -> Self {
        self.tools = serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string());
        self
    }

    /// Set tools from JSON string
    pub fn with_tools_json(mut self, tools: impl Into<String>) -> Self {
        self.tools = tools.into();
        self
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set max iterations
    pub fn with_max_iterations(mut self, max: i64) -> Self {
        self.max_iterations = max;
        self
    }

    /// Mark as default configuration
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }

    /// Get pattern type as enum
    pub fn pattern_type(&self) -> PatternType {
        PatternType::from(self.pattern_type.as_str())
    }

    /// Get tools as vector
    pub fn tool_list(&self) -> Vec<String> {
        serde_json::from_str(&self.tools).unwrap_or_default()
    }

    /// Parse config JSON into a Value
    pub fn config_value(&self) -> serde_json::Value {
        serde_json::from_str(&self.config).unwrap_or(serde_json::json!({}))
    }

    /// Get a specific config value
    pub fn get_config<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let value = self.config_value();
        value.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Increment usage count
    pub fn increment_usage(&mut self) {
        self.usage_count += 1;
        self.updated_at = Utc::now().timestamp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_type_conversion() {
        assert_eq!(PatternType::from_str("react"), Some(PatternType::React));
        assert_eq!(PatternType::from_str("plan_execute"), Some(PatternType::PlanExecute));
        assert_eq!(PatternType::from_str("plan-execute"), Some(PatternType::PlanExecute));
        assert_eq!(PatternType::from_str("reflection"), Some(PatternType::Reflection));
        assert_eq!(PatternType::from_str("invalid"), None);
    }

    #[test]
    fn test_pattern_type_as_str() {
        assert_eq!(PatternType::React.as_str(), "react");
        assert_eq!(PatternType::PlanExecute.as_str(), "plan_execute");
        assert_eq!(PatternType::Reflection.as_str(), "reflection");
    }

    #[test]
    fn test_pattern_config_creation() {
        let config = PatternConfig::new("Test Config", PatternType::React);
        assert_eq!(config.name, "Test Config");
        assert_eq!(config.pattern_type, "react");
        assert_eq!(config.max_iterations, 10);
        assert!(!config.is_default);
    }

    #[test]
    fn test_pattern_config_builder() {
        let config = PatternConfig::new("Code Gen", PatternType::Reflection)
            .with_config(r#"{"quality_threshold": 0.9}"#)
            .with_tools(vec!["read_file", "write_file", "run_tests"])
            .with_system_prompt("You are an expert coder.")
            .with_max_iterations(15)
            .as_default();

        assert_eq!(config.name, "Code Gen");
        assert_eq!(config.pattern_type, "reflection");
        assert_eq!(config.max_iterations, 15);
        assert!(config.is_default);
        assert_eq!(config.system_prompt, Some("You are an expert coder.".to_string()));

        let tools = config.tool_list();
        assert_eq!(tools.len(), 3);
        assert!(tools.contains(&"read_file".to_string()));
    }

    #[test]
    fn test_pattern_config_get_config() {
        let config = PatternConfig::new("Test", PatternType::Reflection)
            .with_config(r#"{"quality_threshold": 0.85, "max_refinements": 3}"#);

        let threshold: Option<f64> = config.get_config("quality_threshold");
        assert_eq!(threshold, Some(0.85));

        let refinements: Option<i64> = config.get_config("max_refinements");
        assert_eq!(refinements, Some(3));

        let missing: Option<String> = config.get_config("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_pattern_config_tool_list() {
        let config = PatternConfig::new("Test", PatternType::React)
            .with_tools_json(r#"["search", "read", "write"]"#);

        let tools = config.tool_list();
        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0], "search");
    }

    #[test]
    fn test_pattern_config_increment_usage() {
        let mut config = PatternConfig::new("Test", PatternType::React);
        assert_eq!(config.usage_count, 0);

        config.increment_usage();
        assert_eq!(config.usage_count, 1);

        config.increment_usage();
        assert_eq!(config.usage_count, 2);
    }
}
