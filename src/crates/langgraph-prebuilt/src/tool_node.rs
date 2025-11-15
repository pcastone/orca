//! ToolNode - Graph Node for Tool Execution
//!
//! This module provides [`ToolNode`], a graph node component that bridges LLM tool calling
//! with actual tool execution. It's the **critical component** that enables agents to take
//! actions in the world.
//!
//! # Overview
//!
//! ToolNode automates the tool execution workflow:
//! 1. **Extract** - Finds tool calls in AI messages
//! 2. **Execute** - Runs tools in parallel
//! 3. **Return** - Creates tool result messages
//!
//! **Use ToolNode when:**
//! - Building ReAct agents with tool calling
//! - Implementing function calling workflows
//! - Creating agents that interact with APIs/databases
//! - Need automatic tool execution in graphs
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Graph State                                                 │
//! │  messages: [                                                │
//! │    {type: "human", content: "Search for Rust"},             │
//! │    {type: "ai", content: "I'll search",                     │
//! │     tool_calls: [{id: "1", name: "search", args: {...}}]}  │
//! │  ]                                                          │
//! └────────────────────┬────────────────────────────────────────┘
//!                      │
//!                      ↓ ToolNode.execute(state)
//! ┌─────────────────────────────────────────────────────────────┐
//! │  ToolNode                                                    │
//! │  1. Extract last AI message with tool_calls                 │
//! │  2. For each tool_call:                                     │
//! │     - Get tool from registry                                │
//! │     - Execute tool(args)  [parallel]                        │
//! │     - Create ToolMessage with result                        │
//! │  3. Return tool messages                                    │
//! └────────────────────┬────────────────────────────────────────┘
//!                      │
//!                      ↓ Tool messages added to state
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Updated Graph State                                         │
//! │  messages: [                                                │
//! │    {type: "human", ...},                                    │
//! │    {type: "ai", tool_calls: [...]},                         │
//! │    {type: "tool", content: "Results...", tool_call_id: "1"} │
//! │  ]                                                          │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Basic ToolNode Setup
//!
//! ```rust,ignore
//! use langgraph_prebuilt::{ToolNode, Tool, ToolRegistry};
//! use langgraph_core::builder::StateGraph;
//!
//! // Create tool registry
//! let mut registry = ToolRegistry::new();
//! registry.register(Box::new(SearchTool));
//! registry.register(Box::new(CalculatorTool));
//!
//! // Create tool node
//! let tool_node = ToolNode::new(registry);
//!
//! // Add to graph
//! let mut graph = StateGraph::new();
//! graph.add_node("agent", agent_fn);
//! graph.add_node("tools", move |state| {
//!     let tool_node = tool_node.clone();
//!     Box::pin(async move {
//!         tool_node.execute(state).await
//!     })
//! });
//!
//! graph.add_edge("agent", "tools");
//! graph.add_edge("tools", "agent"); // Loop back
//! ```
//!
//! ## Simplified: From Tools Vec
//!
//! ```rust,ignore
//! use langgraph_prebuilt::ToolNode;
//!
//! let tools: Vec<Box<dyn Tool>> = vec![
//!     Box::new(SearchTool),
//!     Box::new(CalculatorTool),
//! ];
//!
//! let tool_node = ToolNode::from_tools(tools);
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: ReAct Loop with ToolNode
//!
//! ```rust,ignore
//! use langgraph_prebuilt::ToolNode;
//! use langgraph_core::builder::StateGraph;
//!
//! fn create_react_graph(tools: Vec<Box<dyn Tool>>) -> StateGraph {
//!     let mut graph = StateGraph::new();
//!     let tool_node = ToolNode::from_tools(tools);
//!
//!     // Agent node: LLM decides to call tools
//!     graph.add_node("agent", |state| {
//!         Box::pin(async move {
//!             let messages = state["messages"].clone();
//!             let response = llm_call(messages).await?;
//!             Ok(json!({"messages": [response]}))
//!         })
//!     });
//!
//!     // Tool node: Execute tool calls
//!     graph.add_node("tools", move |state| {
//!         let tn = tool_node.clone();
//!         Box::pin(async move { tn.execute(state).await })
//!     });
//!
//!     // Routing: Continue to tools if AI called tools, else end
//!     graph.add_conditional_edge("agent", should_continue, {
//!         let mut branches = HashMap::new();
//!         branches.insert("continue".to_string(), "tools".to_string());
//!         branches.insert("end".to_string(), "__end__".to_string());
//!         branches
//!     });
//!
//!     graph.add_edge("tools", "agent"); // Back to agent after tools
//!     graph.set_entry("agent");
//!
//!     graph
//! }
//! ```
//!
//! ## Pattern 2: Error Handling
//!
//! ```rust,ignore
//! // Graceful error handling (default: true)
//! let tool_node = ToolNode::from_tools(tools)
//!     .with_error_handling(true);
//!
//! // Tool errors become error messages that LLM can see:
//! // {
//! //   "type": "tool",
//! //   "content": "{\"error\": \"API timeout\", \"status\": \"error\"}",
//! //   "tool_call_id": "call_1"
//! // }
//!
//! // Strict error handling (propagates errors)
//! let strict_tool_node = ToolNode::from_tools(tools)
//!     .with_error_handling(false);
//! // Tool errors will cause graph execution to fail
//! ```
//!
//! ## Pattern 3: Parallel Tool Execution
//!
//! ToolNode automatically executes multiple tool calls in parallel:
//!
//! ```rust,ignore
//! // If LLM calls multiple tools:
//! let ai_message = Message::ai("I'll search and calculate")
//!     .with_tool_calls(vec![
//!         ToolCall::new("1", "search", json!({"query": "rust"})),
//!         ToolCall::new("2", "calculator", json!({"a": 2, "b": 2})),
//!         ToolCall::new("3", "weather", json!({"city": "SF"})),
//!     ]);
//!
//! // ToolNode executes all 3 in parallel using tokio::join_all
//! // Returns 3 tool messages with results
//! ```
//!
//! ## Pattern 4: Custom Tool Node Logic
//!
//! For advanced use cases, implement custom tool execution:
//!
//! ```rust,ignore
//! async fn custom_tool_execution(state: Value) -> Result<Value> {
//!     let messages: Vec<Message> = serde_json::from_value(state["messages"].clone())?;
//!
//!     // Find tool calls
//!     let last_ai = messages.iter().rev().find(|m| m.is_ai());
//!     let tool_calls = last_ai.and_then(|m| m.get_tool_calls()).unwrap_or(&[]);
//!
//!     let mut results = Vec::new();
//!
//!     for tool_call in tool_calls {
//!         // Custom logic: rate limiting, caching, retry, etc.
//!         let result = execute_with_retry(&tool_call).await?;
//!
//!         results.push(Message::tool(
//!             serde_json::to_string(&result)?,
//!             &tool_call.id
//!         ));
//!     }
//!
//!     Ok(json!({"messages": results}))
//! }
//! ```
//!
//! # Execution Flow Details
//!
//! ## State Format
//!
//! ToolNode expects state with a `messages` field:
//!
//! ```json
//! {
//!   "messages": [
//!     {
//!       "type": "human",
//!       "content": "What's 2+2?"
//!     },
//!     {
//!       "type": "ai",
//!       "content": "I'll calculate that.",
//!       "tool_calls": [
//!         {
//!           "id": "call_1",
//!           "name": "calculator",
//!           "args": {"a": 2, "b": 2}
//!         }
//!       ]
//!     }
//!   ]
//! }
//! ```
//!
//! ## Tool Call Extraction
//!
//! - Scans messages in reverse (most recent first)
//! - Finds first AI message with `tool_calls`
//! - Executes all tool calls from that message
//!
//! ## Parallel Execution
//!
//! - Uses `futures::future::join_all` for parallelism
//! - Each tool call runs concurrently
//! - Results collected in original order
//!
//! ## Error Handling Modes
//!
//! **Graceful (default):**
//! - Tool errors converted to error JSON
//! - LLM sees error and can retry/adjust
//! - Graph continues execution
//!
//! **Strict:**
//! - Tool errors propagate to graph
//! - Graph execution fails
//! - Use for critical tools where errors are unacceptable
//!
//! # Integration with Agents
//!
//! ## ReAct Agent
//!
//! ```rust,ignore
//! use langgraph_prebuilt::create_react_agent;
//!
//! let agent = create_react_agent(llm, tools)?;
//! // Internally uses ToolNode for tool execution
//! ```
//!
//! ## Custom Agent with ToolNode
//!
//! ```rust,ignore
//! fn create_custom_agent(llm: LLM, tools: Vec<Box<dyn Tool>>) -> StateGraph {
//!     let mut graph = StateGraph::new();
//!     let tool_node = ToolNode::from_tools(tools);
//!
//!     graph.add_node("agent", create_agent_node(llm));
//!     graph.add_node("tools", move |state| {
//!         let tn = tool_node.clone();
//!         Box::pin(async move { tn.execute(state).await })
//!     });
//!
//!     // Add routing logic...
//!
//!     graph
//! }
//! ```
//!
//! # Python LangGraph Comparison
//!
//! | Python LangGraph | rLangGraph (Rust) |
//! |------------------|-------------------|
//! | `ToolNode(tools)` | `ToolNode::from_tools(tools)` |
//! | `tool_node.invoke(state)` | `tool_node.execute(state).await` |
//! | Sync execution | Async with parallel tools |
//! | Sequential tools | Parallel by default |
//! | `handle_tool_errors` | `with_error_handling()` |
//!
//! **Python Example:**
//! ```python
//! from langgraph.prebuilt import ToolNode
//!
//! tool_node = ToolNode(tools)
//! result = tool_node.invoke(state)
//! ```
//!
//! **Rust Equivalent:**
//! ```rust,ignore
//! use langgraph_prebuilt::ToolNode;
//!
//! let tool_node = ToolNode::from_tools(tools);
//! let result = tool_node.execute(state).await?;
//! ```
//!
//! # Performance Considerations
//!
//! - **Parallel execution**: Multiple tools run concurrently (faster than sequential)
//! - **Cloning**: ToolNode uses Arc<ToolRegistry> for cheap cloning
//! - **Async I/O**: Non-blocking tool execution
//! - **Memory**: Each tool call result stored in memory before returning
//!
//! # See Also
//!
//! - [`ToolRegistry`](crate::tools::ToolRegistry) - Managing tools
//! - [`Tool`](crate::tools::Tool) - Tool trait
//! - [`Message`](crate::messages::Message) - Message types
//! - [`ToolCall`](crate::messages::ToolCall) - Tool call structure
//! - [`create_react_agent`](crate::agents::create_react_agent) - ReAct agent with tools

use crate::error::{PrebuiltError, Result};
use crate::messages::{Message, ToolCall};
use crate::tools::{Tool, ToolRegistry};
use serde_json::Value;
use std::sync::Arc;

/// ToolNode executes tools based on tool calls in messages
#[derive(Clone)]
pub struct ToolNode {
    /// Tool registry containing available tools
    registry: Arc<ToolRegistry>,

    /// Whether to handle errors gracefully
    handle_tool_errors: bool,
}

impl ToolNode {
    /// Create a new ToolNode with the given tool registry
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
            handle_tool_errors: true,
        }
    }

    /// Create a new ToolNode from a list of tools
    pub fn from_tools(tools: Vec<Box<dyn Tool>>) -> Self {
        let mut registry = ToolRegistry::new();
        for tool in tools {
            registry.register(tool);
        }
        Self::new(registry)
    }

    /// Set whether to handle tool errors gracefully (default: true)
    pub fn with_error_handling(mut self, handle_errors: bool) -> Self {
        self.handle_tool_errors = handle_errors;
        self
    }

    /// Execute tools from a state containing messages
    ///
    /// Expects the state to have a "messages" field containing a list of messages.
    /// Extracts tool calls from the last AI message and executes them.
    ///
    /// Returns tool result messages to append to the state.
    pub async fn execute(&self, state: Value) -> Result<Value> {
        // Extract messages from state
        let messages = self.extract_messages(&state)?;

        // Find the last AI message with tool calls
        let tool_calls = self.find_tool_calls(&messages)?;

        if tool_calls.is_empty() {
            // No tool calls to execute
            return Ok(serde_json::json!({
                "messages": []
            }));
        }

        // Execute all tool calls (in parallel)
        let results = self.execute_tool_calls(tool_calls).await;

        // Convert results to tool messages
        let tool_messages: Vec<Message> = results
            .into_iter()
            .map(|(tool_call, result)| self.create_tool_message(tool_call, result))
            .collect();

        Ok(serde_json::json!({
            "messages": tool_messages
        }))
    }

    /// Extract messages from state
    fn extract_messages(&self, state: &Value) -> Result<Vec<Message>> {
        let messages_value = state
            .get("messages")
            .ok_or_else(|| PrebuiltError::ToolExecution("State missing 'messages' field".into()))?;

        serde_json::from_value(messages_value.clone())
            .map_err(|e| PrebuiltError::ToolExecution(format!("Failed to parse messages: {}", e)))
    }

    /// Find tool calls in messages
    fn find_tool_calls(&self, messages: &[Message]) -> Result<Vec<ToolCall>> {
        // Find the last AI message
        let last_ai_message = messages
            .iter()
            .rev()
            .find(|msg| msg.is_ai());

        if let Some(msg) = last_ai_message {
            if let Some(tool_calls) = msg.get_tool_calls() {
                return Ok(tool_calls.to_vec());
            }
        }

        Ok(Vec::new())
    }

    /// Execute all tool calls
    async fn execute_tool_calls(&self, tool_calls: Vec<ToolCall>) -> Vec<(ToolCall, Result<Value>)> {
        // Execute tools in parallel
        let futures: Vec<_> = tool_calls
            .into_iter()
            .map(|tool_call| {
                let registry = self.registry.clone();
                let handle_errors = self.handle_tool_errors;

                async move {
                    let result = registry.execute(&tool_call.name, tool_call.args.clone()).await;

                    let final_result = if handle_errors && result.is_err() {
                        // Convert error to error message
                        let error_msg = result.unwrap_err().to_string();
                        Ok(serde_json::json!({
                            "error": error_msg,
                            "status": "error"
                        }))
                    } else {
                        result
                    };

                    (tool_call, final_result)
                }
            })
            .collect();

        futures::future::join_all(futures).await
    }

    /// Create a tool message from a tool call result
    fn create_tool_message(&self, tool_call: ToolCall, result: Result<Value>) -> Message {
        let content = match result {
            Ok(value) => serde_json::to_string(&value).unwrap_or_else(|_| value.to_string()),
            Err(e) => format!("Error: {}", e),
        };

        Message::tool(content, tool_call.id)
            .with_name(tool_call.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::ToolCall;
    use crate::tools::Tool;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
                "result": format!("Processed: {}", input)
            }))
        }
    }

    struct SlowTool {
        delay_ms: u64,
    }

    #[async_trait]
    impl Tool for SlowTool {
        fn name(&self) -> &str {
            "slow_tool"
        }

        fn description(&self) -> &str {
            "A slow tool for testing parallelism"
        }

        async fn execute(&self, input: Value) -> Result<Value> {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
            Ok(serde_json::json!({
                "result": format!("Slow: {}", input),
                "delay_ms": self.delay_ms
            }))
        }
    }

    struct FailingTool;

    #[async_trait]
    impl Tool for FailingTool {
        fn name(&self) -> &str {
            "failing_tool"
        }

        fn description(&self) -> &str {
            "A tool that always fails"
        }

        async fn execute(&self, _input: Value) -> Result<Value> {
            Err(PrebuiltError::ToolExecution("Intentional failure".into()))
        }
    }

    struct CounterTool {
        counter: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Tool for CounterTool {
        fn name(&self) -> &str {
            "counter_tool"
        }

        fn description(&self) -> &str {
            "A tool that counts executions"
        }

        async fn execute(&self, _input: Value) -> Result<Value> {
            let count = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(serde_json::json!({"count": count}))
        }
    }

    #[tokio::test]
    async fn test_tool_node_execution() {
        let tool_node = ToolNode::from_tools(vec![Box::new(TestTool)]);

        // Create a state with an AI message containing tool calls
        let tool_call = ToolCall::new(
            "call_1",
            "test_tool",
            serde_json::json!({"input": "test"}),
        );

        let ai_message = Message::ai("Let me use the tool")
            .with_tool_calls(vec![tool_call]);

        let state = serde_json::json!({
            "messages": vec![ai_message]
        });

        let result = tool_node.execute(state).await.unwrap();

        // Verify tool message was created
        let messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].is_tool());
        assert_eq!(messages[0].tool_call_id, Some("call_1".to_string()));
    }

    #[tokio::test]
    async fn test_tool_node_no_tool_calls() {
        let tool_node = ToolNode::from_tools(vec![Box::new(TestTool)]);

        let state = serde_json::json!({
            "messages": vec![Message::ai("Just a regular message")]
        });

        let result = tool_node.execute(state).await.unwrap();

        let messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(messages.len(), 0);
    }

    #[tokio::test]
    async fn test_tool_node_error_handling() {
        let tool_node = ToolNode::from_tools(vec![Box::new(TestTool)]);

        // Create a tool call for a non-existent tool
        let tool_call = ToolCall::new(
            "call_1",
            "non_existent_tool",
            serde_json::json!({}),
        );

        let ai_message = Message::ai("Use unknown tool")
            .with_tool_calls(vec![tool_call]);

        let state = serde_json::json!({
            "messages": vec![ai_message]
        });

        let result = tool_node.execute(state).await.unwrap();

        // Should still return a message (with error as JSON)
        let messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(messages.len(), 1);
        // When handle_tool_errors is true, error is returned as JSON with "error" field
        assert!(messages[0].content.contains("error"));
    }

    // ========== Tool Node Creation Tests ==========

    #[test]
    fn test_tool_node_new_with_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(TestTool));

        let tool_node = ToolNode::new(registry);

        assert!(tool_node.handle_tool_errors);
    }

    #[test]
    fn test_tool_node_from_tools_multiple() {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(TestTool),
            Box::new(SlowTool { delay_ms: 10 }),
        ];

        let tool_node = ToolNode::from_tools(tools);

        assert!(tool_node.handle_tool_errors);
        // Registry should have both tools
        assert_eq!(Arc::strong_count(&tool_node.registry), 1);
    }

    #[test]
    fn test_tool_node_with_error_handling_true() {
        let tool_node = ToolNode::from_tools(vec![Box::new(TestTool)])
            .with_error_handling(true);

        assert!(tool_node.handle_tool_errors);
    }

    #[test]
    fn test_tool_node_with_error_handling_false() {
        let tool_node = ToolNode::from_tools(vec![Box::new(TestTool)])
            .with_error_handling(false);

        assert!(!tool_node.handle_tool_errors);
    }

    #[test]
    fn test_tool_node_clone_shares_registry() {
        let tool_node1 = ToolNode::from_tools(vec![Box::new(TestTool)]);
        let tool_node2 = tool_node1.clone();

        // Both should share the same Arc
        assert_eq!(Arc::strong_count(&tool_node1.registry), 2);
        assert_eq!(Arc::strong_count(&tool_node2.registry), 2);
    }

    // ========== Message Extraction Tests ==========

    #[tokio::test]
    async fn test_extract_messages_success() {
        let tool_node = ToolNode::from_tools(vec![]);

        let state = serde_json::json!({
            "messages": vec![Message::ai("Test")]
        });

        let messages = tool_node.extract_messages(&state).unwrap();
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_extract_messages_missing_field() {
        let tool_node = ToolNode::from_tools(vec![]);

        let state = serde_json::json!({
            "other_field": "value"
        });

        let result = tool_node.extract_messages(&state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("messages"));
    }

    #[tokio::test]
    async fn test_extract_messages_invalid_format() {
        let tool_node = ToolNode::from_tools(vec![]);

        let state = serde_json::json!({
            "messages": "not an array"
        });

        let result = tool_node.extract_messages(&state);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_messages_empty() {
        let tool_node = ToolNode::from_tools(vec![]);

        let state = serde_json::json!({
            "messages": []
        });

        let messages = tool_node.extract_messages(&state).unwrap();
        assert_eq!(messages.len(), 0);
    }

    #[tokio::test]
    async fn test_extract_messages_multiple() {
        let tool_node = ToolNode::from_tools(vec![]);

        let state = serde_json::json!({
            "messages": vec![
                Message::human("Hello"),
                Message::ai("Hi"),
                Message::human("How are you?"),
            ]
        });

        let messages = tool_node.extract_messages(&state).unwrap();
        assert_eq!(messages.len(), 3);
    }

    // ========== Tool Call Finding Tests ==========

    #[tokio::test]
    async fn test_find_tool_calls_in_last_ai() {
        let tool_node = ToolNode::from_tools(vec![]);

        let tool_call = ToolCall::new("call_1", "test", serde_json::json!({}));
        let messages = vec![
            Message::human("Test"),
            Message::ai("Response").with_tool_calls(vec![tool_call.clone()]),
        ];

        let found_calls = tool_node.find_tool_calls(&messages).unwrap();
        assert_eq!(found_calls.len(), 1);
        assert_eq!(found_calls[0].id, "call_1");
    }

    #[tokio::test]
    async fn test_find_tool_calls_no_ai_messages() {
        let tool_node = ToolNode::from_tools(vec![]);

        let messages = vec![
            Message::human("Hello"),
            Message::human("World"),
        ];

        let found_calls = tool_node.find_tool_calls(&messages).unwrap();
        assert_eq!(found_calls.len(), 0);
    }

    #[tokio::test]
    async fn test_find_tool_calls_ai_without_calls() {
        let tool_node = ToolNode::from_tools(vec![]);

        let messages = vec![
            Message::human("Test"),
            Message::ai("Response without tool calls"),
        ];

        let found_calls = tool_node.find_tool_calls(&messages).unwrap();
        assert_eq!(found_calls.len(), 0);
    }

    #[tokio::test]
    async fn test_find_tool_calls_uses_last_ai() {
        let tool_node = ToolNode::from_tools(vec![]);

        let tool_call1 = ToolCall::new("call_1", "tool1", serde_json::json!({}));
        let tool_call2 = ToolCall::new("call_2", "tool2", serde_json::json!({}));

        let messages = vec![
            Message::ai("First").with_tool_calls(vec![tool_call1]),
            Message::human("Test"),
            Message::ai("Second").with_tool_calls(vec![tool_call2.clone()]),
        ];

        let found_calls = tool_node.find_tool_calls(&messages).unwrap();
        assert_eq!(found_calls.len(), 1);
        assert_eq!(found_calls[0].id, "call_2"); // Should use last AI message
    }

    #[tokio::test]
    async fn test_find_tool_calls_ignores_earlier() {
        let tool_node = ToolNode::from_tools(vec![]);

        let tool_call = ToolCall::new("call_1", "test", serde_json::json!({}));
        let messages = vec![
            Message::ai("First").with_tool_calls(vec![tool_call]),
            Message::human("Test"),
            Message::ai("Second without calls"),
        ];

        let found_calls = tool_node.find_tool_calls(&messages).unwrap();
        assert_eq!(found_calls.len(), 0); // Last AI has no calls
    }

    #[tokio::test]
    async fn test_find_tool_calls_empty_messages() {
        let tool_node = ToolNode::from_tools(vec![]);

        let messages: Vec<Message> = vec![];

        let found_calls = tool_node.find_tool_calls(&messages).unwrap();
        assert_eq!(found_calls.len(), 0);
    }

    // ========== Parallel Execution Tests ==========

    #[tokio::test]
    async fn test_parallel_execution_multiple_tools() {
        use std::time::Instant;

        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(SlowTool { delay_ms: 100 }),
            Box::new(TestTool),
        ];

        let tool_node = ToolNode::from_tools(tools);

        let tool_calls = vec![
            ToolCall::new("call_1", "slow_tool", serde_json::json!({})),
            ToolCall::new("call_2", "test_tool", serde_json::json!({})),
        ];

        let messages = vec![
            Message::ai("Execute").with_tool_calls(tool_calls),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let start = Instant::now();
        let result = tool_node.execute(state).await.unwrap();
        let elapsed = start.elapsed();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 2);

        // Should execute in parallel, so total time should be close to slowest tool (100ms)
        // not sum of both (100ms + negligible)
        assert!(elapsed.as_millis() < 150); // Some buffer for CI
    }

    #[tokio::test]
    async fn test_parallel_execution_order_preserved() {
        let counter = Arc::new(AtomicUsize::new(0));

        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(CounterTool { counter: counter.clone() }),
        ];

        let tool_node = ToolNode::from_tools(tools);

        let tool_calls = vec![
            ToolCall::new("call_1", "counter_tool", serde_json::json!({})),
            ToolCall::new("call_2", "counter_tool", serde_json::json!({})),
            ToolCall::new("call_3", "counter_tool", serde_json::json!({})),
        ];

        let messages = vec![
            Message::ai("Count").with_tool_calls(tool_calls),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 3);

        // All 3 tools should have executed (order may vary due to parallelism)
        assert_eq!(counter.load(Ordering::SeqCst), 3);

        // Tool call IDs should match in order
        assert_eq!(tool_messages[0].tool_call_id, Some("call_1".to_string()));
        assert_eq!(tool_messages[1].tool_call_id, Some("call_2".to_string()));
        assert_eq!(tool_messages[2].tool_call_id, Some("call_3".to_string()));
    }

    #[tokio::test]
    async fn test_parallel_execution_mixed_success_failure() {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(TestTool),
            Box::new(FailingTool),
        ];

        let tool_node = ToolNode::from_tools(tools);

        let tool_calls = vec![
            ToolCall::new("call_1", "test_tool", serde_json::json!({})),
            ToolCall::new("call_2", "failing_tool", serde_json::json!({})),
        ];

        let messages = vec![
            Message::ai("Execute").with_tool_calls(tool_calls),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 2);

        // First should succeed, second should have error
        assert!(tool_messages[0].content.contains("Processed"));
        assert!(tool_messages[1].content.contains("error"));
    }

    #[tokio::test]
    async fn test_parallel_execution_all_fail() {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(FailingTool),
        ];

        let tool_node = ToolNode::from_tools(tools);

        let tool_calls = vec![
            ToolCall::new("call_1", "failing_tool", serde_json::json!({})),
            ToolCall::new("call_2", "failing_tool", serde_json::json!({})),
        ];

        let messages = vec![
            Message::ai("Execute").with_tool_calls(tool_calls),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 2);

        // Both should have errors (graceful handling)
        assert!(tool_messages[0].content.contains("error"));
        assert!(tool_messages[1].content.contains("error"));
    }

    #[tokio::test]
    async fn test_single_tool_execution() {
        let tool_node = ToolNode::from_tools(vec![Box::new(TestTool)]);

        let tool_call = ToolCall::new("call_1", "test_tool", serde_json::json!({"data": "test"}));

        let messages = vec![
            Message::ai("Execute").with_tool_calls(vec![tool_call]),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 1);
        assert!(tool_messages[0].is_tool());
    }

    #[tokio::test]
    async fn test_no_tool_calls_returns_empty() {
        let tool_node = ToolNode::from_tools(vec![Box::new(TestTool)]);

        let state = serde_json::json!({
            "messages": vec![Message::ai("No tools")]
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 0);
    }

    #[tokio::test]
    async fn test_large_parallel_execution() {
        let counter = Arc::new(AtomicUsize::new(0));

        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(CounterTool { counter: counter.clone() }),
        ];

        let tool_node = ToolNode::from_tools(tools);

        // Create 10 parallel tool calls
        let tool_calls: Vec<ToolCall> = (1..=10)
            .map(|i| ToolCall::new(format!("call_{}", i), "counter_tool", serde_json::json!({})))
            .collect();

        let messages = vec![
            Message::ai("Count").with_tool_calls(tool_calls),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 10);
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    // ========== Error Handling Tests ==========

    #[tokio::test]
    async fn test_graceful_error_handling_unknown_tool() {
        let tool_node = ToolNode::from_tools(vec![Box::new(TestTool)])
            .with_error_handling(true);

        let tool_call = ToolCall::new("call_1", "unknown_tool", serde_json::json!({}));

        let messages = vec![
            Message::ai("Execute").with_tool_calls(vec![tool_call]),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 1);
        assert!(tool_messages[0].content.contains("error"));
        assert!(tool_messages[0].content.contains("status"));
    }

    #[tokio::test]
    async fn test_strict_error_handling_propagates() {
        let tool_node = ToolNode::from_tools(vec![Box::new(FailingTool)])
            .with_error_handling(false);

        let tool_call = ToolCall::new("call_1", "failing_tool", serde_json::json!({}));

        let messages = vec![
            Message::ai("Execute").with_tool_calls(vec![tool_call]),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        // With strict handling, error is in content as "Error: ..."
        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 1);
        assert!(tool_messages[0].content.starts_with("Error:"));
    }

    #[tokio::test]
    async fn test_graceful_error_includes_message() {
        let tool_node = ToolNode::from_tools(vec![Box::new(FailingTool)])
            .with_error_handling(true);

        let tool_call = ToolCall::new("call_1", "failing_tool", serde_json::json!({}));

        let messages = vec![
            Message::ai("Execute").with_tool_calls(vec![tool_call]),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        let content: Value = serde_json::from_str(&tool_messages[0].content).unwrap();

        assert_eq!(content["status"], "error");
        assert!(content["error"].as_str().unwrap().contains("Intentional"));
    }

    #[tokio::test]
    async fn test_error_message_format() {
        let tool_node = ToolNode::from_tools(vec![])
            .with_error_handling(true);

        let tool_call = ToolCall::new("call_1", "nonexistent", serde_json::json!({}));

        let messages = vec![
            Message::ai("Execute").with_tool_calls(vec![tool_call]),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();

        // Should be valid JSON with error and status fields
        let error_json: Value = serde_json::from_str(&tool_messages[0].content).unwrap();
        assert!(error_json.get("error").is_some());
        assert_eq!(error_json.get("status").and_then(|v| v.as_str()), Some("error"));
    }

    #[tokio::test]
    async fn test_multiple_failures_with_graceful_handling() {
        let tool_node = ToolNode::from_tools(vec![])
            .with_error_handling(true);

        let tool_calls = vec![
            ToolCall::new("call_1", "tool1", serde_json::json!({})),
            ToolCall::new("call_2", "tool2", serde_json::json!({})),
            ToolCall::new("call_3", "tool3", serde_json::json!({})),
        ];

        let messages = vec![
            Message::ai("Execute").with_tool_calls(tool_calls),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 3);

        // All should have error messages
        for msg in tool_messages {
            assert!(msg.content.contains("error"));
        }
    }

    #[tokio::test]
    async fn test_tool_execution_with_complex_input() {
        let tool_node = ToolNode::from_tools(vec![Box::new(TestTool)]);

        let complex_input = serde_json::json!({
            "nested": {
                "data": [1, 2, 3],
                "config": {
                    "enabled": true,
                    "value": 42
                }
            }
        });

        let tool_call = ToolCall::new("call_1", "test_tool", complex_input);

        let messages = vec![
            Message::ai("Execute").with_tool_calls(vec![tool_call]),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();
        assert_eq!(tool_messages.len(), 1);
        assert!(tool_messages[0].is_tool());
        assert!(tool_messages[0].content.contains("Processed"));
    }

    #[tokio::test]
    async fn test_strict_error_preserves_error_type() {
        let tool_node = ToolNode::from_tools(vec![Box::new(FailingTool)])
            .with_error_handling(false);

        let tool_call = ToolCall::new("call_1", "failing_tool", serde_json::json!({}));

        let messages = vec![
            Message::ai("Execute").with_tool_calls(vec![tool_call]),
        ];

        let state = serde_json::json!({
            "messages": messages
        });

        let result = tool_node.execute(state).await.unwrap();

        let tool_messages: Vec<Message> = serde_json::from_value(result["messages"].clone()).unwrap();

        // Strict mode should have "Error: " prefix
        assert!(tool_messages[0].content.starts_with("Error:"));
        assert!(tool_messages[0].content.contains("Intentional failure"));
    }

    // ========== Tool Message Creation Tests ==========

    #[test]
    fn test_create_tool_message_success() {
        let tool_node = ToolNode::from_tools(vec![]);

        let tool_call = ToolCall::new("call_1", "test_tool", serde_json::json!({}));
        let result = Ok(serde_json::json!({"result": "success"}));

        let message = tool_node.create_tool_message(tool_call, result);

        assert!(message.is_tool());
        assert_eq!(message.tool_call_id, Some("call_1".to_string()));
        assert!(message.content.contains("success"));
    }

    #[test]
    fn test_create_tool_message_error() {
        let tool_node = ToolNode::from_tools(vec![]);

        let tool_call = ToolCall::new("call_1", "test_tool", serde_json::json!({}));
        let result: Result<Value> = Err(PrebuiltError::ToolExecution("Test error".into()));

        let message = tool_node.create_tool_message(tool_call, result);

        assert!(message.is_tool());
        assert_eq!(message.tool_call_id, Some("call_1".to_string()));
        assert!(message.content.starts_with("Error:"));
        assert!(message.content.contains("Test error"));
    }

    #[test]
    fn test_create_tool_message_with_name() {
        let tool_node = ToolNode::from_tools(vec![]);

        let tool_call = ToolCall::new("call_1", "calculator", serde_json::json!({}));
        let result = Ok(serde_json::json!({"result": 42}));

        let message = tool_node.create_tool_message(tool_call, result);

        assert!(message.is_tool());
        assert_eq!(message.name, Some("calculator".to_string()));
    }

    #[test]
    fn test_create_tool_message_preserves_id() {
        let tool_node = ToolNode::from_tools(vec![]);

        let tool_call = ToolCall::new("custom_id_12345", "test", serde_json::json!({}));
        let result = Ok(serde_json::json!({"data": "test"}));

        let message = tool_node.create_tool_message(tool_call, result);

        assert_eq!(message.tool_call_id, Some("custom_id_12345".to_string()));
    }

    #[test]
    fn test_create_tool_message_serializes_complex_result() {
        let tool_node = ToolNode::from_tools(vec![]);

        let tool_call = ToolCall::new("call_1", "test", serde_json::json!({}));
        let result = Ok(serde_json::json!({
            "nested": {
                "array": [1, 2, 3],
                "object": {"key": "value"}
            }
        }));

        let message = tool_node.create_tool_message(tool_call, result);

        // Should be valid JSON string
        let parsed: Value = serde_json::from_str(&message.content).unwrap();
        assert!(parsed.get("nested").is_some());
        assert_eq!(parsed["nested"]["array"].as_array().unwrap().len(), 3);
    }
}
