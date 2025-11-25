//! Dynamic Agent Builder Service
//!
//! Builds LangGraph agents dynamically based on pattern configuration.
//! Currently supports ReAct pattern with dynamic configuration.

use crate::error::{OrcaError, Result};
use crate::models::{PatternConfig, PatternType};
use langgraph_prebuilt::{create_react_agent, Message, Tool};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Type alias for LLM function (matches langgraph-prebuilt)
pub type LlmFunction = Arc<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = langgraph_prebuilt::Result<Message>> + Send>>
        + Send
        + Sync,
>;

/// Builder result containing the compiled graph
pub type BuildResult = Result<langgraph_core::compiled::CompiledGraph>;

/// Dynamic agent builder that creates agents from pattern configurations
pub struct DynamicAgentBuilder<F>
where
    F: Fn() -> Vec<Box<dyn Tool>> + Send + Sync,
{
    llm_fn: LlmFunction,
    tool_factory: F,
}

impl<F> DynamicAgentBuilder<F>
where
    F: Fn() -> Vec<Box<dyn Tool>> + Send + Sync,
{
    /// Create a new dynamic agent builder
    ///
    /// # Arguments
    /// * `llm_fn` - The LLM function to use for agent execution
    /// * `tool_factory` - A factory function that creates tools (called for each build)
    pub fn new(llm_fn: LlmFunction, tool_factory: F) -> Self {
        Self {
            llm_fn,
            tool_factory,
        }
    }

    /// Build an agent from a pattern configuration
    pub fn build(&self, config: &PatternConfig) -> BuildResult {
        info!(
            pattern = %config.pattern_type,
            config_name = %config.name,
            max_iterations = config.max_iterations,
            "Building agent from pattern config"
        );

        match config.pattern_type() {
            PatternType::React => self.build_react(config),
            PatternType::PlanExecute => {
                // Plan-Execute uses ReAct with planning prompt
                warn!("Plan-Execute pattern using ReAct with planning prompt");
                self.build_react_with_planning(config)
            }
            PatternType::Reflection => {
                // Reflection uses ReAct with reflection prompt
                warn!("Reflection pattern using ReAct with quality prompt");
                self.build_react_with_reflection(config)
            }
            _ => {
                warn!(
                    pattern = %config.pattern_type,
                    "Pattern type falling back to ReAct"
                );
                self.build_react(config)
            }
        }
    }

    /// Build a standard ReAct agent
    fn build_react(&self, config: &PatternConfig) -> BuildResult {
        let all_tools = (self.tool_factory)();
        let tools = self.filter_tools(all_tools, &config.tool_list());

        debug!(
            tool_count = tools.len(),
            max_iterations = config.max_iterations,
            "Building ReAct agent"
        );

        let mut builder = create_react_agent(self.llm_fn.clone(), tools)
            .with_max_iterations(config.max_iterations as usize);

        if let Some(ref prompt) = config.system_prompt {
            if !prompt.is_empty() {
                builder = builder.with_system_prompt(prompt.clone());
            }
        }

        builder
            .build()
            .map_err(|e| OrcaError::Execution(format!("Failed to build ReAct agent: {}", e)))
    }

    /// Build a ReAct agent with planning-oriented prompt
    fn build_react_with_planning(&self, config: &PatternConfig) -> BuildResult {
        let all_tools = (self.tool_factory)();
        let tools = self.filter_tools(all_tools, &config.tool_list());

        let max_steps = config.get_config::<usize>("max_steps").unwrap_or(10);

        debug!(
            tool_count = tools.len(),
            max_iterations = config.max_iterations,
            max_steps = max_steps,
            "Building ReAct agent with planning prompt"
        );

        // Use planning-oriented system prompt
        let planning_prompt = config.system_prompt.clone().unwrap_or_else(|| {
            format!(
                "You are a planning assistant. Before executing, create a clear plan with up to {} steps. \
                 For each step: 1) State what you will do, 2) Execute it, 3) Verify the result. \
                 If a step fails, adjust your plan and continue.",
                max_steps
            )
        });

        create_react_agent(self.llm_fn.clone(), tools)
            .with_max_iterations(config.max_iterations as usize)
            .with_system_prompt(planning_prompt)
            .build()
            .map_err(|e| OrcaError::Execution(format!("Failed to build planning agent: {}", e)))
    }

    /// Build a ReAct agent with reflection-oriented prompt
    fn build_react_with_reflection(&self, config: &PatternConfig) -> BuildResult {
        let all_tools = (self.tool_factory)();
        let tools = self.filter_tools(all_tools, &config.tool_list());

        let quality_threshold = config
            .get_config::<f64>("quality_threshold")
            .unwrap_or(0.8);

        debug!(
            tool_count = tools.len(),
            max_iterations = config.max_iterations,
            quality_threshold = quality_threshold,
            "Building ReAct agent with reflection prompt"
        );

        // Use reflection-oriented system prompt
        let reflection_prompt = config.system_prompt.clone().unwrap_or_else(|| {
            "You are an expert assistant focused on quality. For each response: \
             1) Generate your best answer, 2) Critically evaluate it for accuracy, completeness, and clarity, \
             3) Identify any issues or improvements needed, 4) Refine your answer if needed. \
             Only provide the final, refined answer. Aim for excellence.".to_string()
        });

        create_react_agent(self.llm_fn.clone(), tools)
            .with_max_iterations(config.max_iterations as usize)
            .with_system_prompt(reflection_prompt)
            .build()
            .map_err(|e| OrcaError::Execution(format!("Failed to build reflection agent: {}", e)))
    }

    /// Filter tools to only those allowed by the config
    fn filter_tools(
        &self,
        all_tools: Vec<Box<dyn Tool>>,
        allowed_tools: &[String],
    ) -> Vec<Box<dyn Tool>> {
        if allowed_tools.is_empty() {
            debug!("No tool filter specified, using all {} tools", all_tools.len());
            return all_tools;
        }

        let filtered: Vec<Box<dyn Tool>> = all_tools
            .into_iter()
            .filter(|tool| {
                let name = tool.name().to_lowercase();
                allowed_tools.iter().any(|allowed| {
                    let allowed_lower = allowed.to_lowercase();
                    name == allowed_lower || name.contains(&allowed_lower)
                })
            })
            .collect();

        debug!(
            requested = allowed_tools.len(),
            matched = filtered.len(),
            "Filtered tools"
        );

        if filtered.is_empty() && !allowed_tools.is_empty() {
            warn!(
                requested = ?allowed_tools,
                "No tools matched filter, need to create new tools"
            );
        }

        filtered
    }

    /// Get tool names from factory (for informational purposes)
    pub fn available_tool_names(&self) -> Vec<String> {
        let tools = (self.tool_factory)();
        tools.iter().map(|t| t.name().to_string()).collect()
    }
}

/// Simple agent builder that uses a pre-created tool list
pub struct SimpleAgentBuilder {
    llm_fn: LlmFunction,
}

impl SimpleAgentBuilder {
    /// Create a new simple agent builder
    pub fn new(llm_fn: LlmFunction) -> Self {
        Self { llm_fn }
    }

    /// Build an agent with the given tools and config
    pub fn build_with_tools(
        &self,
        config: &PatternConfig,
        tools: Vec<Box<dyn Tool>>,
    ) -> BuildResult {
        info!(
            pattern = %config.pattern_type,
            config_name = %config.name,
            max_iterations = config.max_iterations,
            tool_count = tools.len(),
            "Building agent"
        );

        let mut builder = create_react_agent(self.llm_fn.clone(), tools)
            .with_max_iterations(config.max_iterations as usize);

        if let Some(ref prompt) = config.system_prompt {
            if !prompt.is_empty() {
                builder = builder.with_system_prompt(prompt.clone());
            }
        }

        builder
            .build()
            .map_err(|e| OrcaError::Execution(format!("Failed to build agent: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use langgraph_prebuilt::{PrebuiltError, ToolInput, ToolOutput};

    // Simple test tool
    #[derive(Clone)]
    struct TestTool {
        name: String,
    }

    impl TestTool {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        async fn execute(&self, _input: ToolInput) -> std::result::Result<ToolOutput, PrebuiltError> {
            Ok(serde_json::json!({"result": "test"}))
        }
    }

    fn create_test_llm_fn() -> LlmFunction {
        Arc::new(|_state: Value| {
            Box::pin(async move {
                Ok(Message::ai("Test response"))
            }) as Pin<Box<dyn Future<Output = langgraph_prebuilt::Result<Message>> + Send>>
        })
    }

    fn create_test_tools() -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(TestTool::new("read_file")),
            Box::new(TestTool::new("write_file")),
            Box::new(TestTool::new("search")),
            Box::new(TestTool::new("run_tests")),
        ]
    }

    #[test]
    fn test_builder_creation() {
        let llm_fn = create_test_llm_fn();
        let builder = DynamicAgentBuilder::new(llm_fn, create_test_tools);
        assert_eq!(builder.available_tool_names().len(), 4);
    }

    #[test]
    fn test_build_react() {
        let llm_fn = create_test_llm_fn();
        let builder = DynamicAgentBuilder::new(llm_fn, create_test_tools);

        let config = PatternConfig::new("Test ReAct", PatternType::React)
            .with_max_iterations(5)
            .with_system_prompt("You are a test assistant.");

        let result = builder.build(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_with_tool_filter() {
        let llm_fn = create_test_llm_fn();
        let builder = DynamicAgentBuilder::new(llm_fn, create_test_tools);

        let config = PatternConfig::new("Filtered ReAct", PatternType::React)
            .with_tools(vec!["read_file", "search"]);

        let result = builder.build(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_plan_execute_fallback() {
        let llm_fn = create_test_llm_fn();
        let builder = DynamicAgentBuilder::new(llm_fn, create_test_tools);

        let config = PatternConfig::new("Plan Execute", PatternType::PlanExecute)
            .with_max_iterations(15);

        let result = builder.build(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_reflection_fallback() {
        let llm_fn = create_test_llm_fn();
        let builder = DynamicAgentBuilder::new(llm_fn, create_test_tools);

        let config = PatternConfig::new("Reflection", PatternType::Reflection)
            .with_max_iterations(10);

        let result = builder.build(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_simple_builder() {
        let llm_fn = create_test_llm_fn();
        let builder = SimpleAgentBuilder::new(llm_fn);

        let config = PatternConfig::new("Simple", PatternType::React)
            .with_max_iterations(5);

        let tools = create_test_tools();
        let result = builder.build_with_tools(&config, tools);
        assert!(result.is_ok());
    }

    #[test]
    fn test_available_tool_names() {
        let llm_fn = create_test_llm_fn();
        let builder = DynamicAgentBuilder::new(llm_fn, create_test_tools);
        let names = builder.available_tool_names();

        assert!(names.contains(&"read_file".to_string()));
        assert!(names.contains(&"write_file".to_string()));
        assert!(names.contains(&"search".to_string()));
        assert!(names.contains(&"run_tests".to_string()));
    }
}
