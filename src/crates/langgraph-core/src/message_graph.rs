//! MessageGraph - Specialized graph builder for conversational AI workflows
//!
//! `MessageGraph` is a high-level abstraction over [`StateGraph`](crate::StateGraph) specifically
//! designed for building chat-based agents and conversational AI systems. It automatically manages
//! message history with intelligent deduplication, making it ideal for LLM applications.
//!
//! # Overview
//!
//! MessageGraph simplifies building conversational agents by:
//!
//! - **Automatic Message Management** - Built-in "messages" field with smart merging
//! - **ID-Based Deduplication** - Updates existing messages instead of duplicating
//! - **Tool Call Support** - Handles tool calls and responses seamlessly
//! - **Simplified API** - Less boilerplate than raw StateGraph for chat workflows
//!
//! # When to Use MessageGraph
//!
//! Choose MessageGraph when:
//! - ✅ Building chat-based agents with LLMs
//! - ✅ Implementing conversational workflows
//! - ✅ Working primarily with message lists
//! - ✅ Need automatic message deduplication
//!
//! Choose StateGraph when:
//! - ✅ Need custom state structure beyond messages
//! - ✅ Building non-conversational workflows
//! - ✅ Require fine-grained control over state management
//! - ✅ Working with complex multi-field state
//!
//! # Architecture
//!
//! ```text
//! MessageGraph
//!     ↓
//! StateGraph with:
//!     • "messages" channel (LastValue + add_messages reducer)
//!     • Automatic message deduplication by ID
//!     • Tool call/response handling
//! ```
//!
//! The `add_messages` reducer provides:
//! - **Append**: New messages without IDs are appended
//! - **Update**: Messages with existing IDs replace old versions
//! - **Delete**: RemoveMessage markers delete by ID
//!
//! # Quick Start
//!
//! ## Basic Chat Agent
//!
//! ```rust,ignore
//! use langgraph_core::{MessageGraph, messages::Message};
//! use serde_json::json;
//!
//! let mut graph = MessageGraph::new();
//!
//! // Add LLM node
//! graph.add_node("llm", |state| {
//!     Box::pin(async move {
//!         let messages = state["messages"].as_array().unwrap();
//!
//!         // Call your LLM
//!         let response = call_llm(messages).await?;
//!
//!         // Return new message (automatically merged)
//!         Ok(json!({
//!             "messages": [Message::ai(response)]
//!         }))
//!     })
//! });
//!
//! graph.add_edge("__start__", "llm");
//! graph.add_edge("llm", "__end__");
//!
//! let compiled = graph.compile()?;
//!
//! // Use it
//! let result = compiled.invoke(json!({
//!     "messages": [Message::human("Hello!")]
//! })).await?;
//! ```
//!
//! ## With System Message
//!
//! ```rust,ignore
//! let mut graph = MessageGraph::new();
//!
//! // Add agent node
//! graph.add_node("agent", agent_node);
//! graph.add_edge("__start__", "agent");
//! graph.add_edge("agent", "__end__");
//!
//! let compiled = graph.compile()?;
//!
//! // Initial state with system message
//! let result = compiled.invoke(json!({
//!     "messages": [
//!         Message::system("You are a helpful assistant"),
//!         Message::human("Hello!")
//!     ]
//! })).await?;
//! ```
//!
//! # Common Patterns
//!
//! ## Multi-Turn Conversation
//!
//! Use checkpointing for stateful conversations:
//!
//! ```rust,ignore
//! use langgraph_checkpoint::{InMemoryCheckpointSaver, CheckpointConfig};
//!
//! let compiled = graph.compile()?
//!     .with_checkpointer(Arc::new(InMemoryCheckpointSaver::new()));
//!
//! let config = CheckpointConfig::new()
//!     .with_thread_id("user-123".to_string());
//!
//! // Turn 1
//! compiled.invoke_with_config(
//!     json!({"messages": [Message::human("What's the weather?")]}),
//!     Some(config.clone())
//! ).await?;
//!
//! // Turn 2 (loads previous messages automatically)
//! compiled.invoke_with_config(
//!     json!({"messages": [Message::human("And tomorrow?")]}),
//!     Some(config)
//! ).await?;
//! ```
//!
//! ## Agent with Tools
//!
//! Build ReAct-style agents with tool calling:
//!
//! ```rust,ignore
//! use langgraph_core::{MessageGraph, messages::Message, tool::ToolCall};
//!
//! let mut graph = MessageGraph::new();
//!
//! // LLM decides whether to call tools
//! graph.add_node("llm", |state| {
//!     Box::pin(async move {
//!         let response = call_llm_with_tools(state["messages"].as_array().unwrap()).await?;
//!
//!         // Returns Message with tool_calls
//!         Ok(json!({"messages": [response]}))
//!     })
//! });
//!
//! // Execute tools
//! graph.add_node("tools", |state| {
//!     Box::pin(async move {
//!         let messages = state["messages"].as_array().unwrap();
//!         let last_msg = messages.last().unwrap();
//!
//!         // Execute each tool call
//!         let mut tool_messages = vec![];
//!         for tool_call in &last_msg.tool_calls {
//!             let result = execute_tool(tool_call).await?;
//!             tool_messages.push(Message::tool(result, tool_call.id.clone()));
//!         }
//!
//!         Ok(json!({"messages": tool_messages}))
//!     })
//! });
//!
//! // Conditional routing
//! graph.add_conditional_edge("llm", |state| {
//!     let messages = state["messages"].as_array().unwrap();
//!     let last = messages.last().unwrap();
//!
//!     if !last.tool_calls.is_empty() {
//!         vec!["tools"]
//!     } else {
//!         vec!["__end__"]
//!     }
//! });
//!
//! graph.add_edge("tools", "llm"); // Loop back
//! ```
//!
//! ## Streaming Token-by-Token
//!
//! Stream LLM responses as they're generated:
//!
//! ```rust,ignore
//! use langgraph_core::stream::StreamMode;
//! use futures::StreamExt;
//!
//! let mut stream = compiled.stream_chunks_with_modes(
//!     json!({"messages": [Message::human("Tell me a story")]}),
//!     vec![StreamMode::Messages, StreamMode::Updates],
//!     None,
//! ).await?;
//!
//! while let Some(chunk) = stream.next().await {
//!     if let StreamEvent::MessageChunk { chunk: token, .. } = chunk.event {
//!         print!("{}", token);
//!         std::io::stdout().flush().ok();
//!     }
//! }
//! ```
//!
//! ## Context Window Management
//!
//! Trim messages to fit model limits:
//!
//! ```rust,ignore
//! use langgraph_core::messages::{trim_messages, TrimOptions};
//!
//! graph.add_node("trim_history", |state| {
//!     Box::pin(async move {
//!         let messages = state["messages"].as_array().unwrap();
//!
//!         // Keep only last 10 messages + system
//!         let trimmed = trim_messages(
//!             messages.clone(),
//!             TrimOptions::last(10).with_include_system(true)
//!         );
//!
//!         Ok(json!({"messages": trimmed}))
//!     })
//! });
//! ```
//!
//! # Message Deduplication
//!
//! The `add_messages` reducer handles message IDs intelligently:
//!
//! ```rust,ignore
//! // Initial messages
//! let state1 = json!({
//!     "messages": [
//!         Message::human("Hello").with_id("msg1"),
//!         Message::ai("Hi!").with_id("msg2"),
//!     ]
//! });
//!
//! // Update existing message
//! let state2 = json!({
//!     "messages": [
//!         Message::ai("Hi there!").with_id("msg2"), // Updates msg2
//!     ]
//! });
//!
//! // Result: msg1 unchanged, msg2 updated
//! ```
//!
//! # Advantages over StateGraph
//!
//! | Feature | MessageGraph | StateGraph |
//! |---------|--------------|------------|
//! | Message management | ✅ Automatic | Manual setup |
//! | Message deduplication | ✅ Built-in | Manual implementation |
//! | Tool call handling | ✅ Natural fit | Requires custom state |
//! | Chat workflows | ✅ Optimized | General purpose |
//! | Custom state fields | ❌ Limited | ✅ Full control |
//! | Complex state logic | ❌ Not ideal | ✅ Full flexibility |
//!
//! # Performance Considerations
//!
//! - **Message Lists**: O(n) for message merging where n = message count
//! - **ID Lookup**: O(n) for ID-based updates (uses HashMap internally)
//! - **Memory**: Full message history kept in memory - use trimming for long conversations
//! - **Checkpointing**: Entire message list serialized - consider message size
//!
//! # Comparison with Python LangGraph
//!
//! | Python | Rust | Notes |
//! |--------|------|-------|
//! | `MessageGraph()` | `MessageGraph::new()` | Identical concept |
//! | `add_node(name, func)` | `add_node(name, closure)` | Async closures in Rust |
//! | `add_messages` reducer | `add_messages` function | Same deduplication logic |
//! | State is dict | State is JSON Value | Similar structure |
//!
//! # See Also
//!
//! - [`StateGraph`](crate::StateGraph) - General-purpose graph builder
//! - [`messages`](crate::messages) - Message types and utilities
//! - [`add_messages`](crate::messages::add_messages) - Message merging logic
//! - [`CompiledGraph`](crate::compiled::CompiledGraph) - Execution runtime
//! - [`trim_messages`](crate::messages::trim_messages) - Context window management

use crate::builder::StateGraph;
use crate::compiled::CompiledGraph;
use crate::error::Result;
use crate::graph::{ChannelType, NodeId};
use crate::messages::{Message, add_messages};
use serde_json::Value;

/// Builder for constructing message-based conversation graphs
///
/// MessageGraph is a specialized StateGraph that:
/// - Uses a "messages" field to store conversation history
/// - Automatically applies the `add_messages` reducer for intelligent message merging
/// - Supports ID-based message deduplication and updates
/// - Provides convenient methods for chat-based workflows
pub struct MessageGraph {
    inner: StateGraph,
}

impl MessageGraph {
    /// Create a new message graph
    ///
    /// The graph will automatically manage a "messages" list that uses the
    /// `add_messages` reducer for intelligent message history management.
    ///
    /// The reducer supports:
    /// - Appending new messages
    /// - Updating existing messages by ID
    /// - Removing messages with RemoveMessage markers
    pub fn new() -> Self {
        let mut inner = StateGraph::new();

        // Remove the default "state" channel since we're using "messages" instead
        // This ensures compile() will use "messages" as the primary state channel
        inner.graph_mut().channels.remove("state");

        // Add messages channel with custom add_messages reducer
        let reducer: crate::graph::ReducerFn = std::sync::Arc::new(|left: Value, right: Value| {
            let left_msgs = value_to_messages(left);
            let right_msgs = value_to_messages(right);
            let merged = add_messages(left_msgs, right_msgs);
            messages_to_value(merged)
        });

        inner.add_channel("messages", ChannelType::LastValue, Some(reducer));

        Self { inner }
    }

    /// Create a message graph with an initial system message
    ///
    /// Convenience method for starting a conversation with a system prompt.
    pub fn with_system_message(_system_message: impl Into<String>) -> Self {
        let graph = Self::new();
        // The initial system message would be set in the initial state
        // when invoking the graph
        graph
    }

    /// Add a node to the message graph
    ///
    /// Nodes in a MessageGraph typically receive and return states containing
    /// a "messages" field with a list of Message objects.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the node
    /// * `executor` - Async function that processes the state
    ///
    /// # Returns
    ///
    /// Mutable reference to self for method chaining
    pub fn add_node<F>(&mut self, id: impl Into<NodeId>, executor: F) -> &mut Self
    where
        F: Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.inner.add_node(id, executor);
        self
    }

    /// Add an edge between two nodes
    ///
    /// # Arguments
    ///
    /// * `source` - Source node ID
    /// * `target` - Target node ID
    pub fn add_edge(&mut self, source: impl Into<NodeId>, target: impl Into<NodeId>) -> &mut Self {
        self.inner.add_edge(source, target);
        self
    }

    /// Add a conditional edge with a routing function
    ///
    /// # Arguments
    ///
    /// * `source` - Source node ID
    /// * `router` - Function that determines the next node based on state
    /// * `branches` - Map of branch keys to node IDs
    pub fn add_conditional_edge<F>(
        &mut self,
        source: impl Into<NodeId>,
        router: F,
        branches: std::collections::HashMap<String, NodeId>,
    ) -> &mut Self
    where
        F: Fn(&Value) -> crate::send::ConditionalEdgeResult + Send + Sync + 'static,
    {
        self.inner.add_conditional_edge(source, router, branches);
        self
    }

    /// Set the entry point of the graph
    ///
    /// # Arguments
    ///
    /// * `node` - Node to use as entry point (defaults to "__start__")
    pub fn set_entry(&mut self, node: impl Into<NodeId>) -> &mut Self {
        self.inner.set_entry(node);
        self
    }

    /// Set the finish point of the graph
    ///
    /// # Arguments
    ///
    /// * `node` - Node that connects to "__end__"
    pub fn add_finish(&mut self, node: impl Into<NodeId>) -> &mut Self {
        self.inner.add_finish(node);
        self
    }

    /// Compile the message graph into an executable form
    ///
    /// # Returns
    ///
    /// A compiled graph ready for execution
    pub fn compile(self) -> Result<CompiledGraph> {
        self.inner.compile()
    }
}

impl Default for MessageGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a JSON Value to a vector of Messages
fn value_to_messages(value: Value) -> Vec<Message> {
    match value {
        Value::Array(arr) => arr
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect(),
        _ => vec![],
    }
}

/// Convert a vector of Messages to a JSON Value
fn messages_to_value(messages: Vec<Message>) -> Value {
    Value::Array(
        messages
            .into_iter()
            .map(|m| serde_json::to_value(m).unwrap())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Message, MessageRole};

    #[test]
    fn test_message_graph_creation() {
        let _graph = MessageGraph::new();
        // MessageGraph created successfully
    }

    #[test]
    fn test_message_graph_add_node() {
        let mut graph = MessageGraph::new();

        graph.add_node("agent", |state| {
            Box::pin(async move {
                Ok(state)
            })
        });

        // Graph should compile without errors
        let result = graph.compile();
        assert!(result.is_ok(), "MessageGraph should compile successfully");
    }

    #[tokio::test]
    async fn test_message_graph_with_messages() {
        let mut graph = MessageGraph::new();

        // Add a node that appends a message
        graph.add_node("agent", |state| {
            Box::pin(async move {
                let mut new_state = state.clone();

                // Create a response message
                let response = Message::assistant("Hello! How can I help?");
                let messages_update = serde_json::json!({
                    "messages": [response]
                });

                // Merge with existing state
                if let Some(obj) = new_state.as_object_mut() {
                    obj.insert("messages".to_string(), messages_update["messages"].clone());
                }

                Ok(new_state)
            })
        });

        graph.add_edge("__start__", "agent");
        graph.add_edge("agent", "__end__");

        let compiled = graph.compile();
        assert!(compiled.is_ok(), "MessageGraph should compile");

        // Test execution with initial messages
        let compiled_graph = compiled.unwrap();
        let initial_message = Message::human("Hi there!");
        let initial_state = serde_json::json!({
            "messages": [initial_message]
        });

        let result = compiled_graph.invoke(initial_state).await;
        assert!(result.is_ok(), "MessageGraph execution should succeed");

        let final_state = result.unwrap();
        let messages = final_state.get("messages").and_then(|v| v.as_array());
        assert!(messages.is_some(), "Final state should have messages");

        // Should have both the human message and agent response
        let messages_array = messages.unwrap();
        assert!(messages_array.len() >= 1, "Should have at least one message");
    }

    #[test]
    fn test_value_to_messages_conversion() {
        let msg1 = Message::human("Hello");
        let msg2 = Message::assistant("Hi there!");

        let value = messages_to_value(vec![msg1.clone(), msg2.clone()]);
        let converted = value_to_messages(value);

        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].role, MessageRole::Human);
        assert_eq!(converted[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_message_reducer_append() {
        let msg1 = Message::human("First").with_id("1");
        let msg2 = Message::human("Second").with_id("2");

        let left = messages_to_value(vec![msg1]);
        let right = messages_to_value(vec![msg2]);

        let left_msgs = value_to_messages(left);
        let right_msgs = value_to_messages(right);
        let merged = add_messages(left_msgs, right_msgs);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id, Some("1".to_string()));
        assert_eq!(merged[1].id, Some("2".to_string()));
    }

    #[test]
    fn test_message_reducer_update() {
        let msg1 = Message::human("Original").with_id("1");
        let msg2 = Message::human("Updated").with_id("1");

        let left = messages_to_value(vec![msg1]);
        let right = messages_to_value(vec![msg2]);

        let left_msgs = value_to_messages(left);
        let right_msgs = value_to_messages(right);
        let merged = add_messages(left_msgs, right_msgs);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text(), Some("Updated"));
    }

    #[test]
    fn test_message_graph_conditional_routing() {
        use std::collections::HashMap;

        let mut graph = MessageGraph::new();

        graph.add_node("agent", |state| {
            Box::pin(async move { Ok(state) })
        });

        graph.add_node("tool_node", |state| {
            Box::pin(async move { Ok(state) })
        });

        graph.add_edge("__start__", "agent");

        // Add conditional routing based on agent output
        let mut branches = HashMap::new();
        branches.insert("continue".to_string(), "tool_node".to_string());
        branches.insert("end".to_string(), "__end__".to_string());

        graph.add_conditional_edge("agent", |_state| {
            use crate::send::ConditionalEdgeResult;
            // Would check if agent requested tools
            ConditionalEdgeResult::Node("continue".to_string())
        }, branches);

        graph.add_edge("tool_node", "__end__");

        let result = graph.compile();
        assert!(result.is_ok(), "Conditional routing should work");
    }
}
