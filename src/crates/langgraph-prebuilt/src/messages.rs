//! Message Types - Structured Communication for Agents
//!
//! This module provides standardized message types for LLM agent communication,
//! compatible with common LLM API formats (OpenAI, Anthropic, etc.).
//!
//! # Overview
//!
//! Messages form the **conversation history** that agents use to maintain context
//! and coordinate tool calling. The message system supports:
//!
//! - **Multiple message types**: Human, AI, System, Tool
//! - **Tool calling**: AI can request tool execution via ToolCall
//! - **Tool responses**: Tools return results as Tool messages
//! - **Message history**: Track full conversation context
//! - **Metadata**: Attach custom data to messages
//!
//! # Quick Start
//!
//! ## Basic Message Creation
//!
//! ```rust
//! use langgraph_prebuilt::Message;
//!
//! // Human message (user input)
//! let user_msg = Message::human("What's the weather in SF?");
//!
//! // AI message (assistant response)
//! let ai_msg = Message::ai("Let me check that for you.");
//!
//! // System message (instructions)
//! let sys_msg = Message::system("You are a helpful assistant.");
//!
//! // Tool message (tool result)
//! let tool_msg = Message::tool("Temperature: 72°F", "call_123");
//! ```
//!
//! ## Tool Calling Flow
//!
//! ```rust,ignore
//! use langgraph_prebuilt::{Message, ToolCall};
//! use serde_json::json;
//!
//! // 1. Human asks a question
//! let human = Message::human("What's 2 + 2?");
//!
//! // 2. AI decides to use calculator tool
//! let tool_call = ToolCall::new("call_1", "calculator", json!({"a": 2, "b": 2}));
//! let ai_with_tool = Message::ai("I'll calculate that.")
//!     .with_tool_calls(vec![tool_call]);
//!
//! // 3. Tool executes and returns result
//! let tool_result = Message::tool("4", "call_1");
//!
//! // 4. AI provides final answer
//! let final_answer = Message::ai("The answer is 4.");
//! ```
//!
//! ## Message History
//!
//! ```rust
//! use langgraph_prebuilt::{Message, MessageHistory};
//!
//! let mut history = MessageHistory::new();
//!
//! history.add(Message::system("You are a helpful assistant"));
//! history.add(Message::human("Hello!"));
//! history.add(Message::ai("Hi! How can I help?"));
//!
//! println!("Conversation has {} messages", history.len());
//! println!("Last message: {}", history.last().unwrap().content);
//! ```
//!
//! # Message Types Explained
//!
//! ## Human Messages
//!
//! User input or questions:
//!
//! ```rust
//! use langgraph_prebuilt::Message;
//!
//! let msg = Message::human("Search for Rust tutorials")
//!     .with_name("user_123")
//!     .with_metadata("timestamp".to_string(),
//!                    serde_json::json!("2024-01-01T10:00:00Z"));
//! ```
//!
//! **Use for:**
//! - User queries
//! - User commands
//! - User feedback
//!
//! ## AI Messages
//!
//! Assistant responses, can include tool calls:
//!
//! ```rust,ignore
//! use langgraph_prebuilt::{Message, ToolCall};
//!
//! // Simple response
//! let response = Message::ai("Here's what I found...");
//!
//! // Response with tool call
//! let with_tools = Message::ai("I'll search for that.")
//!     .with_tool_calls(vec![
//!         ToolCall::new("call_1", "search", serde_json::json!({
//!             "query": "Rust tutorials"
//!         }))
//!     ]);
//!
//! // Check if AI wants to use tools
//! if with_tools.has_tool_calls() {
//!     for tool_call in with_tools.get_tool_calls().unwrap() {
//!         println!("AI wants to call: {}", tool_call.name);
//!     }
//! }
//! ```
//!
//! **Use for:**
//! - LLM responses
//! - Tool call requests
//! - Reasoning steps (ReAct pattern)
//!
//! ## System Messages
//!
//! Instructions and context for the LLM:
//!
//! ```rust
//! use langgraph_prebuilt::Message;
//!
//! let system = Message::system(
//!     "You are an expert Rust developer. \
//!      Provide concise, accurate answers with code examples."
//! );
//! ```
//!
//! **Use for:**
//! - Agent instructions
//! - Role definition
//! - Behavior constraints
//! - Context setting
//!
//! ## Tool Messages
//!
//! Results from tool execution:
//!
//! ```rust
//! use langgraph_prebuilt::Message;
//!
//! // Tool returns search results
//! let tool_result = Message::tool(
//!     "Found 10 Rust tutorials...",
//!     "call_1"  // Must match ToolCall.id
//! );
//! ```
//!
//! **Use for:**
//! - Tool execution results
//! - API responses
//! - Database query results
//! - File read results
//!
//! # Tool Calling Pattern
//!
//! The standard flow for tool calling in agents:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  1. Human Message                                         │
//! │     "What's the weather in Paris?"                       │
//! └────────────────────────┬─────────────────────────────────┘
//!                          ↓
//! ┌──────────────────────────────────────────────────────────┐
//! │  2. AI Message with ToolCall                             │
//! │     content: "Let me check the weather."                 │
//! │     tool_calls: [                                        │
//! │       { id: "call_1", name: "weather", args: {...} }    │
//! │     ]                                                    │
//! └────────────────────────┬─────────────────────────────────┘
//!                          ↓
//! ┌──────────────────────────────────────────────────────────┐
//! │  3. Tool Node Executes                                   │
//! │     - Calls weather API                                  │
//! │     - Returns Tool Message                               │
//! └────────────────────────┬─────────────────────────────────┘
//!                          ↓
//! ┌──────────────────────────────────────────────────────────┐
//! │  4. Tool Message                                         │
//! │     content: "Temperature: 18°C, Cloudy"                 │
//! │     tool_call_id: "call_1"                               │
//! └────────────────────────┬─────────────────────────────────┘
//!                          ↓
//! ┌──────────────────────────────────────────────────────────┐
//! │  5. AI Message (Final Answer)                            │
//! │     "The weather in Paris is 18°C and cloudy."           │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! # Message History Management
//!
//! ## Trimming for Context Windows
//!
//! ```rust,ignore
//! use langgraph_prebuilt::MessageHistory;
//!
//! fn trim_history(history: &MessageHistory, max_messages: usize) -> MessageHistory {
//!     let mut trimmed = MessageHistory::new();
//!
//!     // Keep system message if present
//!     if let Some(first) = history.messages().first() {
//!         if first.is_system() {
//!             trimmed.add(first.clone());
//!         }
//!     }
//!
//!     // Add last N messages
//!     for msg in history.last_n(max_messages) {
//!         if !msg.is_system() {
//!             trimmed.add(msg.clone());
//!         }
//!     }
//!
//!     trimmed
//! }
//! ```
//!
//! ## Extracting Tool Calls
//!
//! ```rust,ignore
//! fn get_pending_tool_calls(history: &MessageHistory) -> Vec<&ToolCall> {
//!     history.messages()
//!         .iter()
//!         .filter(|m| m.is_ai() && m.has_tool_calls())
//!         .flat_map(|m| m.get_tool_calls().unwrap_or(&[]))
//!         .collect()
//! }
//! ```
//!
//! # Serialization Format
//!
//! Messages serialize to JSON compatible with LLM APIs:
//!
//! ```json
//! {
//!   "type": "human",
//!   "content": "Hello!",
//!   "name": "user_123"
//! }
//!
//! {
//!   "type": "ai",
//!   "content": "Let me search for that.",
//!   "tool_calls": [
//!     {
//!       "id": "call_1",
//!       "name": "search",
//!       "args": {"query": "rust"},
//!       "type": "tool_call"
//!     }
//!   ]
//! }
//!
//! {
//!   "type": "tool",
//!   "content": "Found 10 results...",
//!   "tool_call_id": "call_1"
//! }
//! ```
//!
//! # Python LangGraph Comparison
//!
//! | Python LangGraph | rLangGraph (Rust) |
//! |------------------|-------------------|
//! | `HumanMessage("hi")` | `Message::human("hi")` |
//! | `AIMessage("hello")` | `Message::ai("hello")` |
//! | `SystemMessage(...)` | `Message::system(...)` |
//! | `ToolMessage(...)` | `Message::tool(...)` |
//! | `msg.tool_calls` | `msg.get_tool_calls()` |
//! | Class hierarchy | Enum with type field |
//! | `BaseMessage` | `Message` struct |
//!
//! **Python Example:**
//! ```python
//! from langchain_core.messages import HumanMessage, AIMessage
//!
//! messages = [
//!     HumanMessage(content="Hello"),
//!     AIMessage(content="Hi there!")
//! ]
//! ```
//!
//! **Rust Equivalent:**
//! ```rust
//! use langgraph_prebuilt::{Message, MessageHistory};
//!
//! let mut history = MessageHistory::new();
//! history.add(Message::human("Hello"));
//! history.add(Message::ai("Hi there!"));
//! ```
//!
//! # See Also
//!
//! - [`ToolCall`] - Tool invocation structure
//! - [`MessageHistory`] - Message collection
//! - [`crate::tools::Tool`] - Tool trait for execution
//! - [`crate::tool_node::ToolNode`] - Graph node for tool execution
//! - [`crate::agents`] - Agent patterns using messages

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Tool call structure representing a function/tool invocation request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,

    /// Name of the tool to call
    pub name: String,

    /// Arguments to pass to the tool
    pub args: Value,

    /// Optional type field (default: "tool_call")
    #[serde(default = "default_tool_call_type")]
    #[serde(rename = "type")]
    pub call_type: String,
}

fn default_tool_call_type() -> String {
    "tool_call".to_string()
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(id: impl Into<String>, name: impl Into<String>, args: Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            args,
            call_type: default_tool_call_type(),
        }
    }
}

/// Message type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    /// Human message
    Human,
    /// AI/Assistant message
    AI,
    /// System message
    System,
    /// Tool message
    Tool,
    /// Function message
    Function,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message type
    #[serde(rename = "type")]
    pub message_type: MessageType,

    /// Message content
    pub content: String,

    /// Optional message name/identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional tool call ID (for tool response messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Tool calls requested by AI (for AI messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

impl Message {
    /// Create a new message
    pub fn new(message_type: MessageType, content: impl Into<String>) -> Self {
        Self {
            message_type,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a human message
    pub fn human(content: impl Into<String>) -> Self {
        Self::new(MessageType::Human, content)
    }

    /// Create an AI message
    pub fn ai(content: impl Into<String>) -> Self {
        Self::new(MessageType::AI, content)
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageType::System, content)
    }

    /// Create a tool message
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Tool,
            content: content.into(),
            name: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the message name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if this is a human message
    pub fn is_human(&self) -> bool {
        self.message_type == MessageType::Human
    }

    /// Check if this is an AI message
    pub fn is_ai(&self) -> bool {
        self.message_type == MessageType::AI
    }

    /// Check if this is a system message
    pub fn is_system(&self) -> bool {
        self.message_type == MessageType::System
    }

    /// Check if this is a tool message
    pub fn is_tool(&self) -> bool {
        self.message_type == MessageType::Tool
    }

    /// Add tool calls to an AI message
    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    /// Check if message has tool calls
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls.as_ref().map_or(false, |calls| !calls.is_empty())
    }

    /// Get tool calls if present
    pub fn get_tool_calls(&self) -> Option<&[ToolCall]> {
        self.tool_calls.as_deref()
    }
}

/// A collection of messages
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageHistory {
    messages: Vec<Message>,
}

impl MessageHistory {
    /// Create a new message history
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Add a message
    pub fn add(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Get all messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get the last message
    pub fn last(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Get the last N messages
    pub fn last_n(&self, n: usize) -> &[Message] {
        let start = self.messages.len().saturating_sub(n);
        &self.messages[start..]
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Number of messages
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::human("Hello");
        assert_eq!(msg.message_type, MessageType::Human);
        assert_eq!(msg.content, "Hello");
        assert!(msg.is_human());
    }

    #[test]
    fn test_message_types() {
        let human = Message::human("Hi");
        let ai = Message::ai("Hello");
        let system = Message::system("System");
        let tool = Message::tool("Result", "tool-1");

        assert!(human.is_human());
        assert!(ai.is_ai());
        assert!(system.is_system());
        assert!(tool.is_tool());
    }

    #[test]
    fn test_message_with_metadata() {
        let msg = Message::human("Test")
            .with_name("user")
            .with_metadata("key".to_string(), serde_json::json!("value"));

        assert_eq!(msg.name, Some("user".to_string()));
        assert_eq!(msg.metadata.get("key"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn test_message_history() {
        let mut history = MessageHistory::new();

        history.add(Message::human("Hello"));
        history.add(Message::ai("Hi there"));

        assert_eq!(history.len(), 2);
        assert!(history.last().unwrap().is_ai());

        let last_two = history.last_n(2);
        assert_eq!(last_two.len(), 2);
    }

    #[test]
    fn test_serialization() {
        let msg = Message::human("Test");
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(msg.content, deserialized.content);
        assert_eq!(msg.message_type, deserialized.message_type);
    }
}
