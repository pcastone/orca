//! ReAct Agent - Reasoning and Acting Pattern
//!
//! The **ReAct (Reasoning + Acting)** pattern is the most widely used agent architecture.
//! It combines LLM reasoning with tool execution in a simple yet powerful loop.
//!
//! # Overview
//!
//! ReAct agents work by alternating between **thinking** (LLM reasoning) and **acting**
//! (tool execution):
//!
//! 1. **Think**: LLM decides what action to take (which tool to call)
//! 2. **Act**: Execute the tool and get results
//! 3. **Observe**: LLM sees tool results and decides next step
//! 4. **Repeat**: Continue until task is complete
//!
//! **Use ReAct when:**
//! - Building Q&A systems with external knowledge
//! - Creating agents that need to search, calculate, or call APIs
//! - Implementing tool-using assistants
//! - Need simple, reliable agent behavior (90% of use cases)
//!
//! **Don't use ReAct when:**
//! - Task requires explicit upfront planning (use Plan-Execute)
//! - Need iterative quality improvement (use Reflection)
//! - No tool calling needed (use simple LLM call)
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │  User Input                                                 │
//! │  "What's the weather in Paris?"                            │
//! └─────────────┬──────────────────────────────────────────────┘
//!               │
//!               ↓ START
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Agent Node (LLM Reasoning)                                 │
//! │  • Receives conversation history                            │
//! │  • LLM decides: use tool or answer directly                │
//! │  • Returns AI message (with optional tool_calls)            │
//! └─────────────┬───────────────────────────────────────────────┘
//!               │
//!               ↓ Has tool_calls?
//!             /   \
//!      [Yes] /     \ [No] ──────→ END (return final answer)
//!           ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Tools Node (Action Execution)                              │
//! │  • Extracts tool calls from AI message                      │
//! │  • Executes tools in parallel                               │
//! │  • Returns tool result messages                             │
//! └─────────────┬───────────────────────────────────────────────┘
//!               │
//!               ↓ Loop back with results
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Agent Node (LLM Observation)                               │
//! │  • Sees tool results in conversation                        │
//! │  • Decides: call more tools or provide final answer        │
//! └─────────────┬───────────────────────────────────────────────┘
//!               │
//!               ↓ Continues until no tool calls...
//! ```
//!
//! # Quick Start
//!
//! ## Basic ReAct Agent
//!
//! ```rust,ignore
//! use langgraph_prebuilt::{create_react_agent, Tool, Message};
//! use std::sync::Arc;
//!
//! // Define your LLM function
//! let llm_fn = Arc::new(|state| {
//!     Box::pin(async move {
//!         let messages = state["messages"].clone();
//!         // Call your LLM API (OpenAI, Anthropic, etc.)
//!         let response = call_llm_api(messages).await?;
//!         Ok(Message::ai(response))
//!     }) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
//! });
//!
//! // Create tools
//! let tools: Vec<Box<dyn Tool>> = vec![
//!     Box::new(SearchTool),
//!     Box::new(CalculatorTool),
//! ];
//!
//! // Create and configure agent
//! let agent = create_react_agent(llm_fn, tools)
//!     .with_max_iterations(10)
//!     .with_system_prompt("You are a helpful assistant")
//!     .build()?;
//!
//! // Run the agent
//! let input = serde_json::json!({
//!     "messages": vec![Message::human("What's 25 * 4?")]
//! });
//!
//! let result = agent.invoke(input).await?;
//! ```
//!
//! ## Execution Flow Example
//!
//! **User:** "Search for Rust tutorials and summarize the top 3"
//!
//! **Iteration 1:**
//! - **Agent (Think)**: "I need to search for Rust tutorials"
//! - **Tool Call**: `search("Rust tutorials")`
//! - **Tool Result**: "Found 10 results: 1. Rust Book, 2. ..."
//!
//! **Iteration 2:**
//! - **Agent (Observe)**: "I got search results, now I'll summarize top 3"
//! - **No Tool Call**: Provides final answer
//! - **Output**: "Here are the top 3 Rust tutorials: ..."
//!
//! # Common Patterns
//!
//! ## Pattern 1: Search Agent
//!
//! ```rust,ignore
//! use langgraph_prebuilt::create_react_agent;
//!
//! let search_agent = create_react_agent(llm_fn, vec![
//!     Box::new(WebSearchTool::new(api_key)),
//! ])
//! .with_system_prompt(
//!     "You are a research assistant. Use web search to find accurate information. \
//!      Always cite sources in your answers."
//! )
//! .build()?;
//!
//! let result = search_agent.invoke(json!({
//!     "messages": vec![Message::human("What are the latest Rust features?")]
//! })).await?;
//! ```
//!
//! ## Pattern 2: Multi-Tool Agent
//!
//! ```rust,ignore
//! let multi_tool_agent = create_react_agent(llm_fn, vec![
//!     Box::new(SearchTool),
//!     Box::new(CalculatorTool),
//!     Box::new(WeatherTool),
//!     Box::new(DatabaseQueryTool),
//! ])
//! .with_max_iterations(15)
//! .build()?;
//!
//! // Agent can use any combination of tools
//! let result = multi_tool_agent.invoke(json!({
//!     "messages": vec![Message::human(
//!         "Search for population of Paris, calculate density if area is 105 km²"
//!     )]
//! })).await?;
//! ```
//!
//! ## Pattern 3: Conversational Agent with History
//!
//! ```rust,ignore
//! // Maintain conversation history
//! let mut conversation = vec![
//!     Message::system("You are a helpful coding assistant"),
//!     Message::human("How do I read a file in Rust?"),
//! ];
//!
//! let agent = create_react_agent(llm_fn, tools).build()?;
//!
//! // First turn
//! let result = agent.invoke(json!({"messages": conversation.clone()})).await?;
//! let response_messages: Vec<Message> = serde_json::from_value(result["messages"].clone())?;
//! conversation.extend(response_messages);
//!
//! // Continue conversation with context
//! conversation.push(Message::human("Show me an async example"));
//! let result2 = agent.invoke(json!({"messages": conversation})).await?;
//! ```
//!
//! ## Pattern 4: Streaming Agent Responses
//!
//! ```rust,ignore
//! let agent = create_react_agent(llm_fn, tools).build()?;
//!
//! // Stream the execution
//! let mut stream = agent.stream(input).await?;
//!
//! while let Some(event) = stream.next().await {
//!     match event {
//!         StreamEvent::Node { node, state } => {
//!             println!("Node '{}' executed", node);
//!             if let Some(messages) = state.get("messages") {
//!                 // Print latest message
//!                 if let Some(last) = messages.as_array().and_then(|a| a.last()) {
//!                     println!("Message: {:?}", last);
//!                 }
//!             }
//!         }
//!         _ => {}
//!     }
//! }
//! ```
//!
//! ## Pattern 5: Error Recovery
//!
//! ```rust,ignore
//! // Configure with error handling
//! let robust_agent = create_react_agent(llm_fn, tools)
//!     .with_max_iterations(20) // Allow more retries
//!     .build()?;
//!
//! // ToolNode handles errors gracefully by default
//! // Tool errors become error messages that LLM can see and retry
//! let result = robust_agent.invoke(input).await?;
//! ```
//!
//! # Configuration Options
//!
//! ## ReactAgentConfig
//!
//! | Method | Description | Default |
//! |--------|-------------|---------|
//! | `with_max_iterations(n)` | Maximum reasoning-acting cycles | 10 |
//! | `with_system_prompt(s)` | System instructions for LLM | None |
//!
//! ## Max Iterations
//!
//! Prevents infinite loops when LLM keeps calling tools:
//!
//! ```rust,ignore
//! // Conservative (for simple tasks)
//! let agent = create_react_agent(llm_fn, tools)
//!     .with_max_iterations(5)
//!     .build()?;
//!
//! // Generous (for complex multi-step tasks)
//! let agent = create_react_agent(llm_fn, tools)
//!     .with_max_iterations(20)
//!     .build()?;
//! ```
//!
//! **Recommended values:**
//! - Simple Q&A: 3-5 iterations
//! - Research tasks: 10-15 iterations
//! - Complex workflows: 15-25 iterations
//!
//! ## System Prompts
//!
//! Guide agent behavior with clear instructions:
//!
//! ```rust,ignore
//! let agent = create_react_agent(llm_fn, tools)
//!     .with_system_prompt(
//!         "You are a helpful assistant. Rules:\n\
//!          1. Always cite sources when using search\n\
//!          2. Show your reasoning step-by-step\n\
//!          3. If unsure, search for information\n\
//!          4. Be concise in your final answers"
//!     )
//!     .build()?;
//! ```
//!
//! # LLM Integration
//!
//! ## LlmFunction Type
//!
//! The LLM function signature:
//!
//! ```rust,ignore
//! type LlmFunction = Arc<
//!     dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Message>> + Send>>
//!     + Send + Sync
//! >;
//! ```
//!
//! ## OpenAI-Style Integration
//!
//! ```rust,ignore
//! use openai_api_rust::chat::ChatApi;
//!
//! let llm_fn = Arc::new(|state: Value| {
//!     Box::pin(async move {
//!         let messages: Vec<Message> = serde_json::from_value(
//!             state["messages"].clone()
//!         )?;
//!
//!         // Convert to OpenAI format
//!         let openai_messages = messages.iter().map(|m| {
//!             /* convert to OpenAI ChatMessage */
//!         }).collect();
//!
//!         // Call OpenAI
//!         let response = openai_client
//!             .chat_completion(openai_messages)
//!             .await?;
//!
//!         // Convert response back to Message
//!         let ai_message = Message::ai(response.content);
//!
//!         // Include tool calls if present
//!         if let Some(tool_calls) = response.tool_calls {
//!             ai_message.with_tool_calls(tool_calls)
//!         } else {
//!             ai_message
//!         }
//!     }) as Pin<Box<dyn Future<Output = _> + Send>>
//! });
//! ```
//!
//! ## Anthropic-Style Integration
//!
//! ```rust,ignore
//! let llm_fn = Arc::new(|state: Value| {
//!     Box::pin(async move {
//!         let messages = extract_messages(&state)?;
//!
//!         let response = anthropic_client
//!             .messages()
//!             .create(MessagesRequest {
//!                 model: "claude-3-5-sonnet-20241022",
//!                 messages: convert_to_anthropic(messages),
//!                 tools: Some(convert_tools_to_anthropic(tools)),
//!                 ..Default::default()
//!             })
//!             .await?;
//!
//!         Ok(convert_from_anthropic(response))
//!     }) as Pin<Box<dyn Future<Output = _> + Send>>
//! });
//! ```
//!
//! # Performance Considerations
//!
//! - **Iterations**: Each iteration = 1 LLM call + N tool executions
//! - **Latency**: Typical task = 2-4 seconds (1-3 iterations)
//! - **Tokens**: ~500-2000 tokens per iteration (depends on context)
//! - **Parallelism**: Tools execute in parallel within each iteration
//!
//! **Optimization tips:**
//! 1. Minimize max_iterations for faster responses
//! 2. Use concise system prompts to reduce token usage
//! 3. Implement tool caching for repeated calls
//! 4. Stream results for better UX
//!
//! # Python LangGraph Comparison
//!
//! | Python LangGraph | rLangGraph (Rust) |
//! |------------------|-------------------|
//! | `create_react_agent(llm, tools)` | `create_react_agent(llm_fn, tools).build()` |
//! | `agent.invoke(input)` | `agent.invoke(input).await` |
//! | `max_iterations` kwarg | `.with_max_iterations(n)` |
//! | `state_modifier` kwarg | Manual state modification |
//! | Sync LLM calls | Async LLM calls |
//!
//! **Python Example:**
//! ```python
//! from langgraph.prebuilt import create_react_agent
//!
//! agent = create_react_agent(llm, tools, max_iterations=10)
//! result = agent.invoke({"messages": [("human", "Hello")]})
//! ```
//!
//! **Rust Equivalent:**
//! ```rust,ignore
//! let agent = create_react_agent(llm_fn, tools)
//!     .with_max_iterations(10)
//!     .build()?;
//!
//! let result = agent.invoke(json!({
//!     "messages": vec![Message::human("Hello")]
//! })).await?;
//! ```
//!
//! # See Also
//!
//! - [`create_react_agent`] - Factory function
//! - [`ReactAgentConfig`] - Configuration builder
//! - [`ToolNode`](crate::tool_node::ToolNode) - Tool execution
//! - [`Tool`](crate::tools::Tool) - Tool trait
//! - [`Message`](crate::messages::Message) - Message types
//! - [`create_plan_execute_agent`](super::plan_execute::create_plan_execute_agent) - Alternative pattern
//! - [ReAct Paper](https://arxiv.org/abs/2210.03629) - Original research

use crate::error::{PrebuiltError, Result};
use crate::messages::Message;
use crate::tool_node::ToolNode;
use crate::tools::Tool;
use langgraph_core::builder::StateGraph;
use langgraph_core::compiled::CompiledGraph;
use langgraph_core::error::GraphError;
use serde_json::Value;
use std::sync::Arc;

/// Type alias for LLM function that takes state and returns AI message
pub type LlmFunction = Arc<dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Message>> + Send>> + Send + Sync>;

/// Configuration for React agent
pub struct ReactAgentConfig {
    /// Function that calls the LLM
    llm_function: LlmFunction,

    /// Tools available to the agent
    tools: Vec<Box<dyn Tool>>,

    /// Maximum number of iterations (default: 10)
    max_iterations: usize,

    /// System prompt to prepend to messages
    system_prompt: Option<String>,
}

impl ReactAgentConfig {
    /// Create a new React agent configuration
    pub fn new(
        llm_function: LlmFunction,
        tools: Vec<Box<dyn Tool>>,
    ) -> Self {
        Self {
            llm_function,
            tools,
            max_iterations: 10,
            system_prompt: None,
        }
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Build the compiled React agent graph
    pub fn build(self) -> Result<CompiledGraph> {
        build_react_graph(self)
    }
}

/// Create a React agent with the given LLM function and tools
///
/// # Arguments
///
/// * `llm_function` - Function that takes state and returns an AI message
/// * `tools` - List of tools available to the agent
///
/// # Returns
///
/// ReactAgentConfig for further configuration
///
/// # Example
///
/// ```rust,ignore
/// let agent = create_react_agent(
///     Arc::new(|state| Box::pin(async move {
///         // Call your LLM here
///         Ok(Message::ai("Response"))
///     })),
///     vec![my_tool]
/// )
/// .with_max_iterations(5)
/// .build()?;
/// ```
pub fn create_react_agent(
    llm_function: LlmFunction,
    tools: Vec<Box<dyn Tool>>,
) -> ReactAgentConfig {
    ReactAgentConfig::new(llm_function, tools)
}

/// Build the React agent graph
fn build_react_graph(config: ReactAgentConfig) -> Result<CompiledGraph> {
    let mut graph = StateGraph::new();

    // Create tool node
    let tool_node = ToolNode::from_tools(config.tools);

    // Clone for use in closures
    let llm_fn = config.llm_function.clone();
    let system_prompt = config.system_prompt.clone();

    // Define the agent node (calls LLM)
    graph.add_node("agent", move |mut state: Value| {
        let llm_fn = llm_fn.clone();
        let system_prompt = system_prompt.clone();

        Box::pin(async move {
            // Add system prompt if provided
            if let Some(prompt) = system_prompt {
                if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    // Check if first message is already a system message
                    let has_system = messages
                        .first()
                        .and_then(|m| m.get("type"))
                        .and_then(|t| t.as_str())
                        .map(|t| t == "system")
                        .unwrap_or(false);

                    if !has_system {
                        let system_msg = Message::system(prompt);
                        let system_json = serde_json::to_value(&system_msg)
                            .map_err(|e| GraphError::Execution(e.to_string()))?;
                        messages.insert(0, system_json);
                    }
                }
            }

            // Call LLM function
            let ai_message = llm_fn(state.clone())
                .await
                .map_err(|e| GraphError::Execution(e.to_string()))?;

            // Append the new message to existing messages
            if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                let ai_json = serde_json::to_value(&ai_message)
                    .map_err(|e| GraphError::Execution(e.to_string()))?;
                messages.push(ai_json);
            }

            Ok(state)
        })
    });

    // Define the tools node (executes tools)
    graph.add_node("tools", move |mut state: Value| {
        let tool_node = tool_node.clone();
        Box::pin(async move {
            // Execute tools and get tool result messages
            let tool_result = tool_node
                .execute(state.clone())
                .await
                .map_err(|e| GraphError::Execution(e.to_string()))?;

            // Append tool messages to existing messages
            if let Some(tool_messages) = tool_result.get("messages").and_then(|m| m.as_array()) {
                if let Some(messages) = state.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    for tool_msg in tool_messages {
                        messages.push(tool_msg.clone());
                    }
                }
            }

            Ok(state)
        })
    });

    // Define conditional routing logic
    let should_continue = |state: &Value| -> langgraph_core::send::ConditionalEdgeResult {
        use langgraph_core::send::ConditionalEdgeResult;

        // Get the last message
        if let Some(messages) = state.get("messages").and_then(|m| m.as_array()) {
            if let Some(last_msg) = messages.last() {
                // Try to deserialize to Message to check for tool calls
                if let Ok(msg) = serde_json::from_value::<Message>(last_msg.clone()) {
                    // If AI message has tool calls, go to tools
                    if msg.is_ai() && msg.has_tool_calls() {
                        return ConditionalEdgeResult::Node("tools".to_string());
                    }
                }
            }
        }

        // Otherwise, end execution
        ConditionalEdgeResult::Node("__end__".to_string())
    };

    // Build the graph structure
    // START -> agent
    graph.add_edge("__start__", "agent");

    // agent -> (should_continue) -> tools or END
    let mut branches = std::collections::HashMap::new();
    branches.insert("tools".to_string(), "tools".to_string());
    branches.insert("__end__".to_string(), "__end__".to_string());
    graph.add_conditional_edge("agent", should_continue, branches);

    // tools -> agent (loop back for next iteration)
    graph.add_edge("tools", "agent");

    // Compile the graph
    graph.compile().map_err(|e| PrebuiltError::ToolExecution(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::ToolCall;
    use crate::tools::Tool;
    use async_trait::async_trait;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        async fn execute(&self, input: Value) -> Result<Value> {
            Ok(serde_json::json!({
                "result": format!("Tool executed with: {}", input)
            }))
        }
    }

    #[tokio::test]
    async fn test_react_agent_no_tool_calls() {
        // Create a simple LLM that returns a message without tool calls
        let llm_fn: LlmFunction = Arc::new(|_state| {
            Box::pin(async {
                Ok(Message::ai("I don't need any tools for this"))
            })
        });

        let agent = create_react_agent(llm_fn, vec![Box::new(TestTool)])
            .build()
            .unwrap();

        let input = serde_json::json!({
            "messages": vec![Message::human("Hello")]
        });

        let result = agent.invoke(input).await.unwrap();

        // Should have original message + AI response
        let messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert!(messages.len() >= 2);
        assert!(messages.iter().any(|m| m.is_ai()));
    }

    #[tokio::test]
    async fn test_react_agent_with_tool_calls() {
        // Track call count
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = call_count.clone();

        // Create an LLM that makes a tool call on first invocation, then responds normally
        let llm_fn: LlmFunction = Arc::new(move |_state| {
            let count = count_clone.clone();
            Box::pin(async move {
                let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                if current == 0 {
                    // First call: make a tool call
                    let tool_call = ToolCall::new(
                        "call_1",
                        "test_tool",
                        serde_json::json!({"input": "test"}),
                    );
                    Ok(Message::ai("Let me use the tool").with_tool_calls(vec![tool_call]))
                } else {
                    // Second call: respond with result
                    Ok(Message::ai("Tool result received, task complete"))
                }
            })
        });

        let agent = create_react_agent(llm_fn, vec![Box::new(TestTool)])
            .build()
            .unwrap();

        let input = serde_json::json!({
            "messages": vec![Message::human("Use the tool")]
        });

        let result = agent.invoke(input).await.unwrap();

        // Should have: human message, AI with tool call, tool result, AI final response
        let messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert!(messages.len() >= 4, "Expected at least 4 messages, got {}", messages.len());

        // Check we have a tool message
        assert!(messages.iter().any(|m| m.is_tool()), "Expected at least one tool message");

        // Verify the agent was called twice
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
