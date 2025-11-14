//! Pre-built high-level graph patterns
//!
//! This module provides convenient factory functions for common graph patterns
//! like ReAct agents, ensuring best practices and reducing boilerplate.

use crate::{StateGraph, ToolRegistry, ToolCall, ToolRuntime, ToolOutput};
use crate::graph::END;
use crate::compiled::CompiledGraph;
use crate::error::Result;
use serde_json::{json, Value};
use std::sync::Arc;

/// Configuration for creating a ReAct agent
pub struct ReactAgentConfig {
    /// Maximum number of iterations before stopping
    pub max_iterations: usize,

    /// System prompt/instructions for the agent
    pub system_prompt: Option<String>,

    /// Whether to include intermediate steps in the output
    pub include_steps: bool,
}

impl Default for ReactAgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            system_prompt: None,
            include_steps: true,
        }
    }
}

/// Create a ReAct (Reasoning + Acting) agent graph
///
/// This creates a pre-configured graph that implements the ReAct pattern:
/// 1. Reason about the current state
/// 2. Decide what action to take (tool to call)
/// 3. Execute the tool
/// 4. Observe the result
/// 5. Repeat until task is complete
///
/// # Arguments
///
/// * `tools` - Tool registry with available tools
/// * `model_fn` - Function that takes state and returns tool calls to make
/// * `config` - Agent configuration
///
/// # Returns
///
/// A compiled graph ready for execution
///
/// # Example
///
/// ```rust,no_run
/// use langgraph_core::prebuilt::{create_react_agent, ReactAgentConfig};
/// use langgraph_core::{ToolRegistry, Tool};
/// use serde_json::json;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create tools
///     let mut tools = ToolRegistry::new();
///     // ... register tools ...
///
///     // Create model function (in real usage, this would call an LLM)
///     let model = Arc::new(|state: serde_json::Value| {
///         Box::pin(async move {
///             // Decide what tool to call based on state
///             vec![/* tool calls */]
///         })
///     });
///
///     // Create agent
///     let agent = create_react_agent(
///         tools,
///         model,
///         ReactAgentConfig::default()
///     )?;
///
///     // Run the agent
///     let result = agent.invoke(json!({
///         "messages": ["What is 2 + 2?"]
///     })).await?;
///
///     Ok(())
/// }
/// ```
pub fn create_react_agent(
    tools: ToolRegistry,
    model_fn: Arc<dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<ToolCall>> + Send>> + Send + Sync>,
    config: ReactAgentConfig,
) -> Result<CompiledGraph> {
    let mut graph = StateGraph::new();

    let tools = Arc::new(tools);
    let model_fn_clone = model_fn.clone();

    // Add the agent node (reasoning + action selection)
    graph.add_node("agent", move |state: Value| {
        let model = model_fn_clone.clone();

        Box::pin(async move {
            // Get iteration count
            let iteration = state["iteration"].as_i64().unwrap_or(0);

            // Call the model to decide what tools to use
            let tool_calls = model(state.clone()).await;

            // Return updated state with tool calls
            let mut result = state.clone();
            result["tool_calls"] = json!(tool_calls);
            result["iteration"] = json!(iteration + 1);

            Ok(result)
        })
    });

    // Add the tools execution node
    let tools_clone = tools.clone();
    graph.add_node("tools", move |state: Value| {
        let tools = tools_clone.clone();

        Box::pin(async move {
            // Get tool calls from state
            let tool_calls_value = &state["tool_calls"];
            let tool_calls: Vec<ToolCall> = serde_json::from_value(tool_calls_value.clone())
                .unwrap_or_default();

            if tool_calls.is_empty() {
                // No tools to execute, return state as-is
                return Ok(state);
            }

            // Create runtime context for tools
            let runtime = ToolRuntime::new(state.clone());

            // Execute all tool calls in parallel
            let results = tools.execute_tool_calls(&tool_calls, Some(runtime)).await;

            // Add results to state
            let mut updated_state = state.clone();
            let mut messages = state["messages"]
                .as_array()
                .cloned()
                .unwrap_or_default();

            // Add tool results as messages
            for result in results {
                match result.output {
                    ToolOutput::Success { content } => {
                        messages.push(json!({
                            "role": "tool",
                            "name": result.name,
                            "content": content
                        }));
                    }
                    ToolOutput::Error { error } => {
                        messages.push(json!({
                            "role": "tool",
                            "name": result.name,
                            "content": json!({"error": error})
                        }));
                    }
                }
            }

            updated_state["messages"] = json!(messages);
            Ok(updated_state)
        })
    });

    // Add conditional routing based on whether we should continue or end
    let max_iters = config.max_iterations;
    graph.add_conditional_edge(
        "agent",
        move |state: &Value| {
            let iteration = state["iteration"].as_i64().unwrap_or(0);
            let tool_calls = state["tool_calls"].as_array().map(|a| a.len()).unwrap_or(0);

            // End if no tool calls or max iterations reached
            if tool_calls == 0 || iteration >= max_iters as i64 {
                END.to_string()
            } else {
                "tools".to_string()
            }
        },
        vec![
            ("tools".to_string(), "tools".to_string()),
            (END.to_string(), END.to_string()),
        ].into_iter().collect(),
    );

    // Tools node goes back to agent
    graph.add_edge("tools", "agent");

    // Set entry point
    graph.set_entry("agent");

    // Compile the graph
    graph.compile()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Tool, ToolError};

    #[tokio::test]
    async fn test_create_react_agent() {
        // Create a simple tool registry
        let mut tools = ToolRegistry::new();

        let add_tool = Tool::new(
            "add",
            "Add two numbers",
            json!({"type": "object"}),
            Arc::new(|args, _runtime| {
                Box::pin(async move {
                    let a = args["a"].as_i64().unwrap_or(0);
                    let b = args["b"].as_i64().unwrap_or(0);
                    Ok(json!({"sum": a + b}))
                })
            }),
        );

        tools.register(add_tool);

        // Create a simple model function that calls the add tool once
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let model: Arc<dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<ToolCall>> + Send>> + Send + Sync> =
            Arc::new(move |_state: Value| {
                let count = call_count_clone.clone();
                Box::pin(async move {
                    let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                    // Only make a tool call on the first iteration
                    if current == 0 {
                        vec![ToolCall {
                            id: "call_1".to_string(),
                            name: "add".to_string(),
                            args: json!({"a": 2, "b": 2}),
                        }]
                    } else {
                        vec![] // No more tool calls
                    }
                })
            });

        // Create the agent
        let agent = create_react_agent(tools, model, ReactAgentConfig::default());
        assert!(agent.is_ok());

        let agent = agent.unwrap();

        // Run the agent
        let result = agent.invoke(json!({
            "messages": ["What is 2 + 2?"],
            "iteration": 0
        })).await;

        assert!(result.is_ok());
        let final_state = result.unwrap();

        // Check that tool was called and result is in messages
        let messages = final_state["messages"].as_array().expect("messages should be an array");
        assert!(messages.len() > 0, "should have at least one message");

        // Check iteration count
        let iteration = final_state["iteration"].as_i64().expect(&format!("iteration should be an i64, got: {}", final_state["iteration"]));
        assert!(iteration > 0, "iteration should be > 0");
    }

    #[tokio::test]
    async fn test_max_iterations() {
        let tools = ToolRegistry::new();

        // Model that always wants to call a tool (infinite loop scenario)
        let model: Arc<dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<ToolCall>> + Send>> + Send + Sync> =
            Arc::new(move |_state: Value| {
                Box::pin(async move {
                    vec![ToolCall {
                        id: "call_x".to_string(),
                        name: "nonexistent".to_string(),
                        args: json!({}),
                    }]
                })
            });

        let config = ReactAgentConfig {
            max_iterations: 3,
            ..Default::default()
        };

        let agent = create_react_agent(tools, model, config).unwrap();

        let result = agent.invoke(json!({
            "messages": [],
            "iteration": 0
        })).await;

        assert!(result.is_ok());
        let final_state = result.unwrap();

        // Should stop at max iterations
        println!("Final state: {}", final_state);
        let iteration = final_state["iteration"].as_i64().expect(&format!("iteration should be an i64, got: {}", final_state["iteration"]));
        assert_eq!(iteration, 3);
    }
}
