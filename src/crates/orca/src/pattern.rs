//! LangGraph agent patterns
//!
//! Provides ReAct, Plan-Execute, and Reflection patterns adapted for
//! direct tool execution in Orca.
//!
//! This module defines pattern types and provides infrastructure for
//! integrating DirectToolBridge with langgraph-prebuilt agent patterns.
//!
//! # Pattern Selection Guide
//!
//! Choose the right pattern based on your task requirements:
//!
//! ## ReAct - Best for Most Use Cases (90%)
//!
//! **When to use**:
//! - General Q&A and conversational tasks
//! - Tool-using assistants
//! - Tasks requiring quick responses
//! - Simple to moderate complexity
//!
//! **Advantages**:
//! - Low latency (fewest LLM calls)
//! - Token efficient
//! - Reliable and well-tested
//! - Good for most agent tasks
//!
//! **Example tasks**:
//! - "List all files in directory X"
//! - "Calculate the sum of numbers in file Y"
//! - "Search codebase for function Z"
//!
//! ## Plan-Execute - For Complex Multi-Step Tasks
//!
//! **When to use**:
//! - Tasks requiring multiple steps
//! - Research and analysis workflows
//! - When upfront planning improves outcomes
//! - Complex problem decomposition needed
//!
//! **Advantages**:
//! - Explicit planning phase
//! - Better for complex tasks
//! - Can replan on failures
//! - Clear execution steps
//!
//! **Example tasks**:
//! - "Research topic X and create a comprehensive report"
//! - "Analyze all test files and identify coverage gaps"
//! - "Refactor module Y following best practices"
//!
//! ## Reflection - For Quality-Critical Output
//!
//! **When to use**:
//! - Code generation and refactoring
//! - Technical writing
//! - Tasks where quality matters more than speed
//! - Output requiring multiple iterations
//!
//! **Advantages**:
//! - Self-critique and refinement
//! - Higher quality output
//! - Iterative improvement
//! - Quality threshold support
//!
//! **Example tasks**:
//! - "Write a production-ready function with tests"
//! - "Generate comprehensive API documentation"
//! - "Create a detailed technical design document"
//!
//! # Usage Examples
//!
//! ## Setting Pattern in Task Metadata
//!
//! ```rust,ignore
//! use orca::workflow::Task;
//!
//! // Use ReAct pattern (default)
//! let task = Task::new("List files in /tmp");
//!
//! // Use Plan-Execute pattern
//! let task = Task::new("Research AI agents")
//!     .with_metadata(r#"{"pattern": "plan_execute"}"#);
//!
//! // Use Reflection pattern
//! let task = Task::new("Write a sorting function with tests")
//!     .with_metadata(r#"{"pattern": "reflection"}"#);
//! ```
//!
//! ## Pattern Selection in Code
//!
//! ```rust
//! use orca::pattern::PatternType;
//!
//! // Parse from string
//! let pattern = PatternType::from_str("reflection").unwrap();
//!
//! // Get metadata
//! println!("Pattern: {}", pattern.name());
//! println!("Description: {}", pattern.description());
//! ```

/// Pattern type for agent execution
///
/// Represents the different agent patterns available from langgraph-prebuilt.
/// These patterns can be used with DirectToolBridge for standalone execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// ReAct (Reasoning + Acting) - Most common pattern
    ///
    /// Best for: Q&A systems, tool-using assistants, general agent tasks
    /// Characteristics: Simple, reliable, low latency, token efficient
    React,

    /// Plan-Execute - Explicit planning before execution
    ///
    /// Best for: Complex multi-step tasks, research workflows, tasks requiring upfront planning
    /// Characteristics: Creates explicit plan, handles failures with replanning
    PlanExecute,

    /// Reflection - Generate, critique, refine loop
    ///
    /// Best for: Quality-critical output (code, writing), iterative improvement
    /// Characteristics: Self-critique and refinement, higher quality output
    Reflection,
}

impl PatternType {
    /// Parse pattern type from string
    ///
    /// Accepts: "react", "plan_execute", "plan-execute", "reflection"
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "react" => Some(Self::React),
            "plan_execute" | "plan-execute" => Some(Self::PlanExecute),
            "reflection" => Some(Self::Reflection),
            _ => None,
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::React => "react",
            Self::PlanExecute => "plan_execute",
            Self::Reflection => "reflection",
        }
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::React => "ReAct",
            Self::PlanExecute => "Plan-Execute",
            Self::Reflection => "Reflection",
        }
    }

    /// Get pattern description
    pub fn description(&self) -> &'static str {
        match self {
            Self::React => "Reasoning and Acting - alternates between thinking and tool use",
            Self::PlanExecute => "Explicit planning followed by step-by-step execution",
            Self::Reflection => "Generate, critique, and refine loop for quality output",
        }
    }
}

impl std::fmt::Display for PatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// Implementation Note:
//
// All three patterns are fully implemented in the executor module:
// - TaskExecutor::execute_react() - ReAct pattern implementation
// - TaskExecutor::execute_plan_execute() - Plan-Execute pattern
// - TaskExecutor::execute_reflection() - Reflection pattern
//
// Each pattern integrates with:
// - DirectToolBridge for tool execution (15+ built-in tools)
// - LangGraph prebuilt agents (create_react_agent, etc.)
// - Multi-provider LLM support (Ollama, OpenAI, Claude, etc.)
// - Optional streaming output (StreamMode::Messages, StreamMode::Updates)
//
// Pattern selection happens via task metadata JSON:
// {"pattern": "react"}        - Uses ReAct (default)
// {"pattern": "plan_execute"} - Uses Plan-Execute
// {"pattern": "reflection"}   - Uses Reflection

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_type_conversion() {
        assert_eq!(PatternType::from_str("react"), Some(PatternType::React));
        assert_eq!(
            PatternType::from_str("plan_execute"),
            Some(PatternType::PlanExecute)
        );
        assert_eq!(
            PatternType::from_str("plan-execute"),
            Some(PatternType::PlanExecute)
        );
        assert_eq!(
            PatternType::from_str("reflection"),
            Some(PatternType::Reflection)
        );
        assert_eq!(PatternType::from_str("invalid"), None);
    }

    #[test]
    fn test_pattern_type_as_str() {
        assert_eq!(PatternType::React.as_str(), "react");
        assert_eq!(PatternType::PlanExecute.as_str(), "plan_execute");
        assert_eq!(PatternType::Reflection.as_str(), "reflection");
    }

    #[test]
    fn test_pattern_type_name() {
        assert_eq!(PatternType::React.name(), "ReAct");
        assert_eq!(PatternType::PlanExecute.name(), "Plan-Execute");
        assert_eq!(PatternType::Reflection.name(), "Reflection");
    }

    #[test]
    fn test_pattern_type_description() {
        assert!(PatternType::React.description().contains("Reasoning"));
        assert!(PatternType::PlanExecute.description().contains("planning"));
        assert!(PatternType::Reflection.description().contains("critique"));
    }

    #[test]
    fn test_pattern_type_display() {
        assert_eq!(format!("{}", PatternType::React), "ReAct");
        assert_eq!(format!("{}", PatternType::PlanExecute), "Plan-Execute");
        assert_eq!(format!("{}", PatternType::Reflection), "Reflection");
    }
}

