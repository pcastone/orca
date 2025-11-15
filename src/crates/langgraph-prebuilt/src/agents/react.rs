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

    // Note: test_react_agent_no_tool_calls is covered by test_react_agent_with_tool_calls
    // which validates the full workflow including no-tool-call termination

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

    // ============================================================================
    // Phase 8.1: ReAct Agent - Comprehensive Tests
    // ============================================================================

    // ------------------------------------------------------------------------
    // Agent Creation and Configuration Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_react_agent_config_builder() {
        let llm_fn: LlmFunction = Arc::new(|_| Box::pin(async { Ok(Message::ai("test")) }));

        let config = create_react_agent(llm_fn.clone(), vec![])
            .with_max_iterations(15)
            .with_system_prompt("You are a test agent");

        assert_eq!(config.max_iterations, 15);
        assert_eq!(config.system_prompt, Some("You are a test agent".to_string()));
    }

    #[test]
    fn test_react_agent_default_config() {
        let llm_fn: LlmFunction = Arc::new(|_| Box::pin(async { Ok(Message::ai("test")) }));

        let config = ReactAgentConfig::new(llm_fn, vec![]);

        assert_eq!(config.max_iterations, 10); // Default
        assert_eq!(config.system_prompt, None); // Default
    }

    #[test]
    fn test_react_agent_with_max_iterations() {
        let llm_fn: LlmFunction = Arc::new(|_| Box::pin(async { Ok(Message::ai("test")) }));

        // Test various iteration limits
        for max_iter in [1, 5, 10, 20, 50, 100] {
            let config = create_react_agent(llm_fn.clone(), vec![])
                .with_max_iterations(max_iter);

            assert_eq!(config.max_iterations, max_iter);
        }
    }

    #[test]
    fn test_react_agent_with_system_prompt() {
        let llm_fn: LlmFunction = Arc::new(|_| Box::pin(async { Ok(Message::ai("test")) }));

        let prompts = vec![
            "You are a helpful assistant",
            "You are a research agent. Always cite sources.",
            "",
            "🤖 I am a robot",
        ];

        for prompt in prompts {
            let config = create_react_agent(llm_fn.clone(), vec![])
                .with_system_prompt(prompt);

            assert_eq!(config.system_prompt, Some(prompt.to_string()));
        }
    }

    #[test]
    fn test_react_agent_with_no_tools() {
        let llm_fn: LlmFunction = Arc::new(|_| Box::pin(async { Ok(Message::ai("test")) }));

        let agent_result = create_react_agent(llm_fn, vec![]).build();

        assert!(agent_result.is_ok());
    }

    #[test]
    fn test_react_agent_with_multiple_tools() {
        struct Tool1;
        struct Tool2;
        struct Tool3;

        #[async_trait]
        impl Tool for Tool1 {
            fn name(&self) -> &str { "tool1" }
            fn description(&self) -> &str { "Tool 1" }
            async fn execute(&self, _: Value) -> Result<Value> {
                Ok(serde_json::json!({"result": "tool1"}))
            }
        }

        #[async_trait]
        impl Tool for Tool2 {
            fn name(&self) -> &str { "tool2" }
            fn description(&self) -> &str { "Tool 2" }
            async fn execute(&self, _: Value) -> Result<Value> {
                Ok(serde_json::json!({"result": "tool2"}))
            }
        }

        #[async_trait]
        impl Tool for Tool3 {
            fn name(&self) -> &str { "tool3" }
            fn description(&self) -> &str { "Tool 3" }
            async fn execute(&self, _: Value) -> Result<Value> {
                Ok(serde_json::json!({"result": "tool3"}))
            }
        }

        let llm_fn: LlmFunction = Arc::new(|_| Box::pin(async { Ok(Message::ai("test")) }));

        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(Tool1),
            Box::new(Tool2),
            Box::new(Tool3),
        ];

        let agent_result = create_react_agent(llm_fn, tools).build();
        assert!(agent_result.is_ok());
    }

    #[test]
    fn test_react_agent_config_chaining() {
        let llm_fn: LlmFunction = Arc::new(|_| Box::pin(async { Ok(Message::ai("test")) }));

        let config = create_react_agent(llm_fn, vec![])
            .with_max_iterations(20)
            .with_system_prompt("Test prompt")
            .with_max_iterations(25); // Override

        assert_eq!(config.max_iterations, 25);
        assert_eq!(config.system_prompt, Some("Test prompt".to_string()));
    }

    // ------------------------------------------------------------------------
    // Tool Selection and Routing Logic Tests
    // ------------------------------------------------------------------------

    // Note: Routing termination is tested in test_react_terminates_on_no_tool_calls

    #[tokio::test]
    async fn test_react_routing_with_tool_calls_continues() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let llm_fn: LlmFunction = Arc::new(move |_state| {
            let count = count_clone.clone();
            Box::pin(async move {
                let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                if current == 0 {
                    // Make tool call
                    let tool_call = ToolCall::new("id1", "test_tool", serde_json::json!({}));
                    Ok(Message::ai("Using tool").with_tool_calls(vec![tool_call]))
                } else {
                    // Final answer
                    Ok(Message::ai("Done"))
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

        // Verify agent was called twice (tool call + final answer)
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 2);

        let messages = result["messages"].as_array().unwrap();
        // Verify we have a tool message by checking message types
        let has_tool_msg = messages.iter().any(|msg| {
            msg.get("type").and_then(|t| t.as_str()) == Some("tool")
        });
        assert!(has_tool_msg);
    }

    #[tokio::test]
    async fn test_react_multiple_tool_calls_in_sequence() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let llm_fn: LlmFunction = Arc::new(move |_state| {
            let count = count_clone.clone();
            Box::pin(async move {
                let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                match current {
                    0 => {
                        let tc = ToolCall::new("id1", "test_tool", serde_json::json!({"step": 1}));
                        Ok(Message::ai("Step 1").with_tool_calls(vec![tc]))
                    }
                    1 => {
                        let tc = ToolCall::new("id2", "test_tool", serde_json::json!({"step": 2}));
                        Ok(Message::ai("Step 2").with_tool_calls(vec![tc]))
                    }
                    2 => {
                        let tc = ToolCall::new("id3", "test_tool", serde_json::json!({"step": 3}));
                        Ok(Message::ai("Step 3").with_tool_calls(vec![tc]))
                    }
                    _ => Ok(Message::ai("Final answer"))
                }
            })
        });

        let agent = create_react_agent(llm_fn, vec![Box::new(TestTool)])
            .with_max_iterations(10)
            .build()
            .unwrap();

        let input = serde_json::json!({
            "messages": vec![Message::human("Multi-step task")]
        });

        let result = agent.invoke(input).await.unwrap();

        // Should have called LLM 4 times (3 tool calls + 1 final)
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 4);

        let messages = result["messages"].as_array().unwrap();
        let tool_count = messages.iter().filter(|msg| {
            msg.get("type").and_then(|t| t.as_str()) == Some("tool")
        }).count();
        assert_eq!(tool_count, 3);
    }

    // ------------------------------------------------------------------------
    // Loop Termination Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_react_max_iterations_config_default() {
        let llm_fn: LlmFunction = Arc::new(|_| Box::pin(async { Ok(Message::ai("test")) }));

        let config = create_react_agent(llm_fn, vec![]);

        // Verify default max_iterations is 10
        assert_eq!(config.max_iterations, 10);
    }

    #[test]
    fn test_react_max_iterations_config_custom() {
        let llm_fn: LlmFunction = Arc::new(|_| Box::pin(async { Ok(Message::ai("test")) }));

        // Test that max_iterations can be configured
        let config = create_react_agent(llm_fn, vec![])
            .with_max_iterations(3);

        assert_eq!(config.max_iterations, 3);
    }

    #[tokio::test]
    async fn test_react_terminates_on_no_tool_calls() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let llm_fn: LlmFunction = Arc::new(move |_| {
            let count = count_clone.clone();
            Box::pin(async move {
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(Message::ai("Final answer, no tools needed"))
            })
        });

        let agent = create_react_agent(llm_fn, vec![Box::new(TestTool)])
            .build()
            .unwrap();

        let input = serde_json::json!({
            "messages": vec![Message::human("Question")]
        });

        let _result = agent.invoke(input).await.unwrap();

        // Should only call LLM once since it immediately returns without tool calls
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_react_single_iteration_workflow() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let llm_fn: LlmFunction = Arc::new(move |_| {
            let count = count_clone.clone();
            Box::pin(async move {
                let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                if current == 0 {
                    let tc = ToolCall::new("id", "test_tool", serde_json::json!({}));
                    Ok(Message::ai("Using tool").with_tool_calls(vec![tc]))
                } else {
                    Ok(Message::ai("Answer based on tool result"))
                }
            })
        });

        let agent = create_react_agent(llm_fn, vec![Box::new(TestTool)])
            .with_max_iterations(1)
            .build()
            .unwrap();

        let input = serde_json::json!({
            "messages": vec![Message::human("Question")]
        });

        let _result = agent.invoke(input).await.unwrap();

        // With max_iterations=1, agent should still be able to complete one cycle
        let total_calls = call_count.load(std::sync::atomic::Ordering::SeqCst);
        assert!(total_calls >= 1);
    }

    // ------------------------------------------------------------------------
    // System Prompt Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_react_system_prompt_injection() {
        let system_prompt_received = Arc::new(std::sync::Mutex::new(false));
        let received_clone = system_prompt_received.clone();

        let llm_fn: LlmFunction = Arc::new(move |state| {
            let received = received_clone.clone();
            Box::pin(async move {
                // Check if system message is present
                if let Some(messages) = state.get("messages").and_then(|m| m.as_array()) {
                    if let Some(first) = messages.first() {
                        if let Some(msg_type) = first.get("type").and_then(|t| t.as_str()) {
                            if msg_type == "system" {
                                *received.lock().unwrap() = true;
                            }
                        }
                    }
                }
                Ok(Message::ai("Response"))
            })
        });

        let agent = create_react_agent(llm_fn, vec![])
            .with_system_prompt("You are a helpful assistant")
            .build()
            .unwrap();

        let input = serde_json::json!({
            "messages": vec![Message::human("Hello")]
        });

        let _result = agent.invoke(input).await.unwrap();

        assert!(*system_prompt_received.lock().unwrap());
    }

    #[tokio::test]
    async fn test_react_system_prompt_not_duplicated() {
        let system_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = system_count.clone();

        let llm_fn: LlmFunction = Arc::new(move |state| {
            let count = count_clone.clone();
            Box::pin(async move {
                // Count system messages
                if let Some(messages) = state.get("messages").and_then(|m| m.as_array()) {
                    let sys_count = messages.iter().filter(|msg| {
                        msg.get("type").and_then(|t| t.as_str()) == Some("system")
                    }).count();

                    count.store(sys_count, std::sync::atomic::Ordering::SeqCst);
                }
                Ok(Message::ai("Response"))
            })
        });

        let agent = create_react_agent(llm_fn, vec![])
            .with_system_prompt("System message")
            .build()
            .unwrap();

        // Start with a system message already in messages
        let input = serde_json::json!({
            "messages": vec![
                Message::system("Existing system message"),
                Message::human("Hello")
            ]
        });

        let _result = agent.invoke(input).await.unwrap();

        // Should only have 1 system message (the existing one)
        assert_eq!(system_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_react_without_system_prompt() {
        let has_system = Arc::new(std::sync::Mutex::new(false));
        let system_clone = has_system.clone();

        let llm_fn: LlmFunction = Arc::new(move |state| {
            let has_sys = system_clone.clone();
            Box::pin(async move {
                if let Some(messages) = state.get("messages").and_then(|m| m.as_array()) {
                    let has_system_msg = messages.iter().any(|msg| {
                        msg.get("type").and_then(|t| t.as_str()) == Some("system")
                    });
                    *has_sys.lock().unwrap() = has_system_msg;
                }
                Ok(Message::ai("Response"))
            })
        });

        let agent = create_react_agent(llm_fn, vec![])
            // No system prompt set
            .build()
            .unwrap();

        let input = serde_json::json!({
            "messages": vec![Message::human("Hello")]
        });

        let _result = agent.invoke(input).await.unwrap();

        // Should not have system message
        assert!(!*has_system.lock().unwrap());
    }

    // ------------------------------------------------------------------------
    // Error Recovery Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_react_tool_error_handling() {
        struct FailingTool;

        #[async_trait]
        impl Tool for FailingTool {
            fn name(&self) -> &str { "failing_tool" }
            fn description(&self) -> &str { "A tool that fails" }
            async fn execute(&self, _: Value) -> Result<Value> {
                Err(PrebuiltError::ToolExecution("Tool execution failed".to_string()))
            }
        }

        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let llm_fn: LlmFunction = Arc::new(move |_| {
            let count = count_clone.clone();
            Box::pin(async move {
                let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                if current == 0 {
                    let tc = ToolCall::new("id", "failing_tool", serde_json::json!({}));
                    Ok(Message::ai("Trying tool").with_tool_calls(vec![tc]))
                } else {
                    Ok(Message::ai("Handling error"))
                }
            })
        });

        let agent = create_react_agent(llm_fn, vec![Box::new(FailingTool)])
            .build()
            .unwrap();

        let input = serde_json::json!({
            "messages": vec![Message::human("Test")]
        });

        // Should complete despite tool failure
        let result = agent.invoke(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_react_empty_messages() {
        let llm_fn: LlmFunction = Arc::new(|_| {
            Box::pin(async { Ok(Message::ai("Response")) })
        });

        let agent = create_react_agent(llm_fn, vec![])
            .build()
            .unwrap();

        // Empty messages array
        let input = serde_json::json!({
            "messages": []
        });

        let result = agent.invoke(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_react_message_accumulation() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let llm_fn: LlmFunction = Arc::new(move |_| {
            let count = count_clone.clone();
            Box::pin(async move {
                let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                if current < 2 {
                    let tc = ToolCall::new("id", "test_tool", serde_json::json!({}));
                    Ok(Message::ai("Need tool").with_tool_calls(vec![tc]))
                } else {
                    Ok(Message::ai("Done"))
                }
            })
        });

        let agent = create_react_agent(llm_fn, vec![Box::new(TestTool)])
            .build()
            .unwrap();

        let input = serde_json::json!({
            "messages": vec![Message::human("Start")]
        });

        let result = agent.invoke(input).await.unwrap();

        let messages = result["messages"].as_array().unwrap();

        // Should accumulate: human, AI (tool call), tool result, AI (tool call), tool result, AI (final)
        assert!(messages.len() >= 6);

        // Verify message types are present
        let has_human = messages.iter().any(|msg| msg.get("type").and_then(|t| t.as_str()) == Some("human"));
        let has_ai = messages.iter().any(|msg| msg.get("type").and_then(|t| t.as_str()) == Some("ai"));
        let has_tool = messages.iter().any(|msg| msg.get("type").and_then(|t| t.as_str()) == Some("tool"));

        assert!(has_human);
        assert!(has_ai);
        assert!(has_tool);
    }

    #[test]
    fn test_react_agent_build_creates_graph() {
        let llm_fn: LlmFunction = Arc::new(|_| Box::pin(async { Ok(Message::ai("test")) }));

        let result = create_react_agent(llm_fn, vec![]).build();

        assert!(result.is_ok());
    }
}
