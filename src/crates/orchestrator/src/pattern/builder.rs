//! Pattern builder for constructing CompiledGraph instances from configurations
//!
//! Provides a builder interface to create agent graphs from YAML-defined patterns,
//! accepting runtime components like LLM functions and tools.
//!
//! NOTE: This is the initial infrastructure. Full agent integration will be completed
//! in the pattern factory module.

use crate::config::PatternConfig;

/// Pattern builder for creating CompiledGraph from PatternConfig
///
/// This builder stores pattern configuration and provides methods to construct
/// graphs when runtime components (LLM functions, tools) are available.
pub struct PatternBuilder {
    config: PatternConfig,
}

impl PatternBuilder {
    /// Create a new pattern builder from a configuration
    pub fn new(config: PatternConfig) -> Self {
        Self { config }
    }

    /// Get a reference to the pattern configuration
    pub fn config(&self) -> &PatternConfig {
        &self.config
    }

    /// Get the pattern ID from the configuration
    pub fn pattern_id(&self) -> &str {
        match &self.config {
            PatternConfig::React(c) => &c.base.id,
            PatternConfig::PlanExecute(c) => &c.base.id,
            PatternConfig::Reflection(c) => &c.base.id,
            PatternConfig::Lats(c) => &c.base.id,
            PatternConfig::Storm(c) => &c.base.id,
            PatternConfig::CodeAct(c) => &c.base.id,
            PatternConfig::Tot(c) => &c.base.id,
            PatternConfig::Cot(c) => &c.base.id,
            PatternConfig::Got(c) => &c.base.id,
        }
    }

    /// Get the pattern description if available
    pub fn description(&self) -> Option<&str> {
        match &self.config {
            PatternConfig::React(c) => c.base.description.as_deref(),
            PatternConfig::PlanExecute(c) => c.base.description.as_deref(),
            PatternConfig::Reflection(c) => c.base.description.as_deref(),
            PatternConfig::Lats(c) => c.base.description.as_deref(),
            PatternConfig::Storm(c) => c.base.description.as_deref(),
            PatternConfig::CodeAct(c) => c.base.description.as_deref(),
            PatternConfig::Tot(c) => c.base.description.as_deref(),
            PatternConfig::Cot(c) => c.base.description.as_deref(),
            PatternConfig::Got(c) => c.base.description.as_deref(),
        }
    }

    /// Get the maximum iterations setting
    pub fn max_iterations(&self) -> usize {
        match &self.config {
            PatternConfig::React(c) => c.base.max_iterations,
            PatternConfig::PlanExecute(c) => c.base.max_iterations,
            PatternConfig::Reflection(c) => c.base.max_iterations,
            PatternConfig::Lats(c) => c.base.max_iterations,
            PatternConfig::Storm(c) => c.base.max_iterations,
            PatternConfig::CodeAct(c) => c.base.max_iterations,
            PatternConfig::Tot(c) => c.base.max_iterations,
            PatternConfig::Cot(c) => c.base.max_iterations,
            PatternConfig::Got(c) => c.base.max_iterations,
        }
    }

    /// Check if this pattern type is currently supported
    pub fn is_supported(&self) -> bool {
        matches!(
            self.config,
            PatternConfig::React(_)
                | PatternConfig::PlanExecute(_)
                | PatternConfig::Reflection(_)
        )
    }

    /// Get the pattern type as a string
    pub fn pattern_type(&self) -> &str {
        match &self.config {
            PatternConfig::React(_) => "react",
            PatternConfig::PlanExecute(_) => "plan_execute",
            PatternConfig::Reflection(_) => "reflection",
            PatternConfig::Lats(_) => "lats",
            PatternConfig::Storm(_) => "storm",
            PatternConfig::CodeAct(_) => "code_act",
            PatternConfig::Tot(_) => "tot",
            PatternConfig::Cot(_) => "cot",
            PatternConfig::Got(_) => "got",
        }
    }
}

/// Helper function to create a pattern builder from a configuration
pub fn build_pattern(config: PatternConfig) -> PatternBuilder {
    PatternBuilder::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BasePatternSettings, ReactConfig};
    use std::collections::HashMap;

    fn create_test_react_config() -> PatternConfig {
        PatternConfig::React(ReactConfig {
            base: BasePatternSettings {
                id: "test_react".to_string(),
                description: Some("Test ReAct pattern".to_string()),
                system_prompt: Some("You are a helpful assistant".to_string()),
                max_iterations: 10,
                custom: HashMap::new(),
            },
            tools: vec!["search".to_string(), "calculator".to_string()],
            temperature: Some(0.7),
        })
    }

    #[test]
    fn test_builder_creation() {
        let config = create_test_react_config();
        let builder = PatternBuilder::new(config);

        assert_eq!(builder.pattern_id(), "test_react");
        assert_eq!(builder.description(), Some("Test ReAct pattern"));
        assert_eq!(builder.max_iterations(), 10);
        assert_eq!(builder.pattern_type(), "react");
        assert!(builder.is_supported());
    }

    #[test]
    fn test_build_pattern_helper() {
        let config = create_test_react_config();
        let builder = build_pattern(config);

        assert_eq!(builder.pattern_type(), "react");
    }

    #[test]
    fn test_unsupported_patterns() {
        use crate::config::{LatsConfig, StormConfig, TotConfig};

        let lats_config = PatternConfig::Lats(LatsConfig {
            base: BasePatternSettings {
                id: "test_lats".to_string(),
                description: None,
                system_prompt: None,
                max_iterations: 10,
                custom: HashMap::new(),
            },
            branching_factor: 3,
            max_depth: 5,
            exploration_constant: 1.414,
            simulations: 10,
        });

        let builder = PatternBuilder::new(lats_config);
        assert_eq!(builder.pattern_type(), "lats");
        assert!(!builder.is_supported());
    }

    #[test]
    fn test_config_access() {
        let config = create_test_react_config();
        let builder = PatternBuilder::new(config);

        match builder.config() {
            PatternConfig::React(react_config) => {
                assert_eq!(react_config.base.id, "test_react");
                assert_eq!(react_config.tools.len(), 2);
                assert_eq!(react_config.temperature, Some(0.7));
            }
            _ => panic!("Expected React config"),
        }
    }
}
