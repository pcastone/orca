//! Pattern configuration structures
//!
//! Defines configuration schemas for different agent patterns like ReAct,
//! Plan-Execute, Reflection, LATS, STORM, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatternConfig {
    /// ReAct pattern (Reasoning + Acting)
    React(ReactConfig),
    /// Plan-Execute pattern
    PlanExecute(PlanExecuteConfig),
    /// Reflection pattern (self-critique)
    Reflection(ReflectionConfig),
    /// LATS (Language Agent Tree Search)
    Lats(LatsConfig),
    /// STORM (Structured Research)
    Storm(StormConfig),
    /// CodeAct (Code generation and execution)
    CodeAct(CodeActConfig),
    /// Tree of Thought
    Tot(TotConfig),
    /// Chain of Thought
    Cot(CotConfig),
    /// Graph of Thought
    Got(GotConfig),
}

/// Base pattern settings shared across patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasePatternSettings {
    /// Unique pattern identifier
    pub id: String,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// System prompt override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Maximum iterations
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// Additional custom settings
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

fn default_max_iterations() -> usize {
    10
}

/// ReAct pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactConfig {
    #[serde(flatten)]
    pub base: BasePatternSettings,
    /// Tools available to the agent
    #[serde(default)]
    pub tools: Vec<String>,
    /// Temperature for LLM calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

/// Plan-Execute pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecuteConfig {
    #[serde(flatten)]
    pub base: BasePatternSettings,
    /// Planner prompt template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planner_prompt: Option<String>,
    /// Executor tools
    #[serde(default)]
    pub executor_tools: Vec<String>,
    /// Maximum number of steps in a plan
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    /// Enable replanning on failure
    #[serde(default = "default_true")]
    pub enable_replanning: bool,
}

fn default_max_steps() -> usize {
    5
}

fn default_true() -> bool {
    true
}

/// Reflection pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionConfig {
    #[serde(flatten)]
    pub base: BasePatternSettings,
    /// Generator prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator_prompt: Option<String>,
    /// Critic prompt for evaluation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critic_prompt: Option<String>,
    /// Quality threshold (0.0-1.0)
    #[serde(default = "default_quality_threshold")]
    pub quality_threshold: f64,
    /// Maximum refinement iterations
    #[serde(default = "default_max_iterations")]
    pub max_refinements: usize,
}

fn default_quality_threshold() -> f64 {
    0.8
}

/// LATS (Language Agent Tree Search) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatsConfig {
    #[serde(flatten)]
    pub base: BasePatternSettings,
    /// Branching factor for tree search
    #[serde(default = "default_branching_factor")]
    pub branching_factor: usize,
    /// Maximum tree depth
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    /// Exploration constant for UCB
    #[serde(default = "default_exploration_constant")]
    pub exploration_constant: f64,
    /// Number of simulations
    #[serde(default = "default_simulations")]
    pub simulations: usize,
}

fn default_branching_factor() -> usize {
    3
}

fn default_max_depth() -> usize {
    5
}

fn default_exploration_constant() -> f64 {
    1.414 // sqrt(2)
}

fn default_simulations() -> usize {
    100
}

/// STORM (Structured Research) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StormConfig {
    #[serde(flatten)]
    pub base: BasePatternSettings,
    /// Number of parallel research paths
    #[serde(default = "default_parallel_paths")]
    pub parallel_paths: usize,
    /// Sections to generate
    #[serde(default)]
    pub sections: Vec<String>,
    /// Interview depth
    #[serde(default = "default_interview_depth")]
    pub interview_depth: usize,
}

fn default_parallel_paths() -> usize {
    3
}

fn default_interview_depth() -> usize {
    2
}

/// CodeAct pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeActConfig {
    #[serde(flatten)]
    pub base: BasePatternSettings,
    /// Allowed programming languages
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
    /// Enable code execution
    #[serde(default = "default_true")]
    pub enable_execution: bool,
    /// Execution timeout in seconds
    #[serde(default = "default_execution_timeout")]
    pub execution_timeout: u64,
}

fn default_languages() -> Vec<String> {
    vec!["python".to_string()]
}

fn default_execution_timeout() -> u64 {
    30
}

/// Tree of Thought configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotConfig {
    #[serde(flatten)]
    pub base: BasePatternSettings,
    /// Number of thoughts per step
    #[serde(default = "default_thoughts_per_step")]
    pub thoughts_per_step: usize,
    /// Evaluation strategy
    #[serde(default = "default_evaluation_strategy")]
    pub evaluation_strategy: EvaluationStrategy,
}

fn default_thoughts_per_step() -> usize {
    3
}

fn default_evaluation_strategy() -> EvaluationStrategy {
    EvaluationStrategy::Vote
}

/// Chain of Thought configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CotConfig {
    #[serde(flatten)]
    pub base: BasePatternSettings,
    /// Enable step-by-step reasoning
    #[serde(default = "default_true")]
    pub enable_reasoning: bool,
    /// Show intermediate steps
    #[serde(default = "default_true")]
    pub show_steps: bool,
}

/// Graph of Thought configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GotConfig {
    #[serde(flatten)]
    pub base: BasePatternSettings,
    /// Maximum nodes in thought graph
    #[serde(default = "default_max_nodes")]
    pub max_nodes: usize,
    /// Merge similar thoughts
    #[serde(default = "default_true")]
    pub merge_similar: bool,
}

fn default_max_nodes() -> usize {
    50
}

/// Evaluation strategy for ToT
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationStrategy {
    /// Vote on best thought
    Vote,
    /// Score each thought
    Score,
    /// Sample thoughts
    Sample,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_react_config_deserialization() {
        let yaml = r#"
            type: react
            id: "react_agent_1"
            description: "A ReAct agent"
            max_iterations: 5
            tools: ["search", "calculator"]
            temperature: 0.7
        "#;

        let config: PatternConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            PatternConfig::React(react) => {
                assert_eq!(react.base.id, "react_agent_1");
                assert_eq!(react.base.max_iterations, 5);
                assert_eq!(react.tools, vec!["search", "calculator"]);
                assert_eq!(react.temperature, Some(0.7));
            }
            _ => panic!("Expected ReactConfig"),
        }
    }

    #[test]
    fn test_plan_execute_config() {
        let yaml = r#"
            type: plan_execute
            id: "planner_1"
            max_steps: 7
            enable_replanning: true
            executor_tools: ["tool1", "tool2"]
        "#;

        let config: PatternConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            PatternConfig::PlanExecute(plan) => {
                assert_eq!(plan.base.id, "planner_1");
                assert_eq!(plan.max_steps, 7);
                assert!(plan.enable_replanning);
            }
            _ => panic!("Expected PlanExecuteConfig"),
        }
    }

    #[test]
    fn test_reflection_config() {
        let yaml = r#"
            type: reflection
            id: "reflection_1"
            quality_threshold: 0.85
            max_refinements: 3
        "#;

        let config: PatternConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            PatternConfig::Reflection(reflection) => {
                assert_eq!(reflection.base.id, "reflection_1");
                assert_eq!(reflection.quality_threshold, 0.85);
                assert_eq!(reflection.max_refinements, 3);
            }
            _ => panic!("Expected ReflectionConfig"),
        }
    }

    #[test]
    fn test_lats_config_defaults() {
        let yaml = r#"
            type: lats
            id: "lats_1"
        "#;

        let config: PatternConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            PatternConfig::Lats(lats) => {
                assert_eq!(lats.branching_factor, 3);
                assert_eq!(lats.max_depth, 5);
                assert_eq!(lats.exploration_constant, 1.414);
                assert_eq!(lats.simulations, 100);
            }
            _ => panic!("Expected LatsConfig"),
        }
    }

    #[test]
    fn test_cot_config() {
        let yaml = r#"
            type: cot
            id: "cot_1"
            enable_reasoning: true
            show_steps: false
        "#;

        let config: PatternConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            PatternConfig::Cot(cot) => {
                assert_eq!(cot.base.id, "cot_1");
                assert!(cot.enable_reasoning);
                assert!(!cot.show_steps);
            }
            _ => panic!("Expected CotConfig"),
        }
    }
}
