//! Message types and utilities for conversational AI applications
//!
//! This module provides comprehensive message handling for building chat-based agents,
//! LLM workflows, and conversational AI systems. It includes message types, intelligent
//! message merging, history management, and utilities for working with chat sequences.
//!
//! # Overview
//!
//! The message system is designed for:
//! - **Chat Applications**: Multi-turn conversations with users
//! - **LLM Integration**: Messages formatted for language models
//! - **Tool Calling**: Managing tool calls and responses
//! - **History Management**: Intelligent message deduplication and merging
//! - **Context Window Management**: Trimming messages to fit model limits
//!
//! # Core Types
//!
//! ## [`Message`]
//!
//! The central message type representing a single message in a conversation:
//!
//! ```rust
//! use langgraph_core::messages::{Message, MessageRole};
//!
//! // Simple text message
//! let msg = Message::human("Hello, how are you?");
//!
//! // AI response
//! let response = Message::ai("I'm doing well, thanks for asking!");
//!
//! // System message
//! let system = Message::system("You are a helpful assistant.");
//! ```
//!
//! ## [`MessageRole`]
//!
//! Identifies the speaker in a conversation:
//! - **System** - Instructions or context for the AI
//! - **Human** - Messages from the user
//! - **Assistant/AI** - Responses from the AI model
//! - **Tool** - Results from tool executions
//!
//! ## [`MessageContent`]
//!
//! Flexible content representation supporting:
//! - Simple text strings
//! - Multi-part content (text + images)
//! - Structured data
//!
//! # Key Features
//!
//! ## Intelligent Message Merging with `add_messages`
//!
//! The [`add_messages`] reducer intelligently merges message lists, handling:
//! - **Deduplication** by message ID
//! - **Replacement** of existing messages
//! - **Deletion** via [`RemoveMessage`]
//! - **Ordering** preservation
//!
//! ```rust
//! use langgraph_core::messages::{Message, add_messages};
//!
//! let history = vec![
//!     Message::human("Question 1").with_id("msg1"),
//!     Message::ai("Answer 1").with_id("msg2"),
//! ];
//!
//! let new_messages = vec![
//!     Message::human("Question 2").with_id("msg3"),
//! ];
//!
//! // Intelligently merges, avoiding duplicates
//! let merged = add_messages(history, new_messages);
//! assert_eq!(merged.len(), 3);
//! ```
//!
//! ## Context Window Management
//!
//! Trim messages to fit model context limits using [`trim_messages`]:
//!
//! ```rust
//! use langgraph_core::messages::{Message, trim_messages, TrimOptions, TrimStrategy};
//!
//! let messages = vec![
//!     Message::system("You are helpful"),
//!     Message::human("Q1"),
//!     Message::ai("A1"),
//!     Message::human("Q2"),
//!     Message::ai("A2"),
//! ];
//!
//! // Keep only last 3 messages, preserving system message
//! let options = TrimOptions::last(3)
//!     .with_include_system(true);
//!
//! let trimmed = trim_messages(messages, options);
//! // Result: [system, human("Q2"), ai("A2")]
//! ```
//!
//! ## Message Filtering
//!
//! Filter messages by role or ID:
//!
//! ```rust
//! use langgraph_core::messages::{Message, MessageRole, filter_by_role};
//!
//! let messages = vec![
//!     Message::system("Instructions"),
//!     Message::human("Hello"),
//!     Message::ai("Hi!"),
//! ];
//!
//! // Get only human messages
//! let human_only = filter_by_role(&messages, MessageRole::Human);
//! assert_eq!(human_only.len(), 1);
//! ```
//!
//! # Common Patterns
//!
//! ## Building a Chat Agent
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, messages::{Message, add_messages}};
//! use serde_json::json;
//!
//! let mut graph = StateGraph::new();
//!
//! // Add LLM node that appends AI responses
//! graph.add_node("llm", |state| {
//!     Box::pin(async move {
//!         let messages = state["messages"].as_array().unwrap();
//!
//!         // Call LLM (pseudo-code)
//!         let response = call_llm(messages).await?;
//!
//!         // Return new AI message
//!         Ok(json!({
//!             "messages": vec![Message::ai(response)]
//!         }))
//!     })
//! });
//!
//! // State automatically merges messages using add_messages reducer
//! ```
//!
//! ## Managing Tool Calls
//!
//! ```rust,ignore
//! use langgraph_core::messages::Message;
//! use langgraph_core::tool::ToolCall;
//!
//! // AI requests tool call
//! let ai_msg = Message::ai("Let me check that for you")
//!     .with_tool_calls(vec![
//!         ToolCall::new("search", serde_json::json!({"query": "weather"})),
//!     ]);
//!
//! // Tool response
//! let tool_msg = Message::tool(
//!     "Weather is sunny, 72°F",
//!     ai_msg.tool_calls[0].id.clone()
//! );
//! ```
//!
//! ## Multi-Modal Messages
//!
//! ```rust
//! use langgraph_core::messages::{Message, MessageContent, ContentPart};
//!
//! let image_message = Message::human(MessageContent::Parts(vec![
//!     ContentPart::text("What's in this image?"),
//!     ContentPart::image_url("https://example.com/image.jpg"),
//! ]));
//! ```
//!
//! # Message ID Management
//!
//! Messages can have IDs for tracking and deduplication:
//!
//! ```rust
//! use langgraph_core::messages::Message;
//!
//! // Explicit ID
//! let msg1 = Message::human("Hello").with_id("custom-id-123");
//!
//! // Auto-generate ID
//! let mut msg2 = Message::human("Hi");
//! msg2.ensure_id(); // Generates UUID if no ID exists
//! ```
//!
//! # Performance Considerations
//!
//! - **Message Merging**: O(n+m) where n and m are message list lengths
//! - **Trimming**: O(n) for simple strategies, may be O(n log n) for complex filtering
//! - **Filtering**: O(n) iteration over messages
//! - **Deduplication**: Uses HashMap for O(1) lookups
//!
//! # Integration with StateGraph
//!
//! Messages work seamlessly with StateGraph reducers:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, messages::add_messages};
//! use serde_json::json;
//!
//! let mut graph = StateGraph::new();
//!
//! // Configure state with message reducer
//! graph.add_channel("messages", Box::new(add_messages));
//!
//! // Nodes can now append messages
//! graph.add_node("chat", |state| {
//!     Box::pin(async move {
//!         // New message automatically merged with history
//!         Ok(json!({"messages": vec![Message::ai("Hello!")]}))
//!     })
//! });
//! ```
//!
//! # See Also
//!
//! - [`StateGraph`](crate::StateGraph) - Building conversational graphs
//! - [`tool`](crate::tool) - Tool calling system
//! - [`Command`](crate::Command) - Dynamic graph control
//! - Python LangGraph Messages - <https://langchain-ai.github.io/langgraph/concepts/low_level/#messages>

use crate::tool::ToolCall;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Role of the message sender in a conversation.
///
/// Message roles define the **source and purpose** of each message in a conversation,
/// enabling proper routing, formatting, and processing by LLMs and agents.
///
/// # Role Semantics
///
/// - **System**: Context, instructions, and constraints for the conversation
/// - **Human**: Input from end users
/// - **Assistant**: Responses from AI agents
/// - **Tool**: Output from tool/function executions
/// - **Custom**: Application-specific roles
///
/// # Examples
///
/// ```rust
/// use langgraph_core::messages::{Message, MessageRole};
///
/// // System instructions
/// let system_msg = Message::new(
///     MessageRole::System,
///     "You are a helpful assistant. Be concise."
/// );
///
/// // Human query
/// let user_msg = Message::new(
///     MessageRole::Human,
///     "What is 2+2?"
/// );
///
/// // AI response
/// let ai_msg = Message::new(
///     MessageRole::Assistant,
///     "2+2 equals 4"
/// );
///
/// // Tool result
/// let tool_msg = Message::new(
///     MessageRole::Tool,
///     r#"{"result": 4, "operation": "add"}"#
/// );
///
/// // Custom role for specialized workflows
/// let custom_msg = Message::new(
///     MessageRole::Custom("moderator".to_string()),
///     "Content approved"
/// );
/// ```
///
/// # Serialization
///
/// Roles serialize to lowercase strings compatible with OpenAI/Anthropic APIs:
/// - `System` → `"system"`
/// - `Human` → `"human"`
/// - `Assistant` → `"assistant"`
/// - `Tool` → `"tool"`
/// - `Custom("x")` → `"x"`
///
/// # See Also
///
/// - [`Message`] - Message structure using this role type
/// - [`MessageContent`] - Content format for messages
/// - [`add_messages`] - Reducer function for message state management
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message providing instructions, context, or constraints.
    ///
    /// Used to set conversation parameters, guidelines, and behavioral expectations.
    System,

    /// Human/user message containing input or queries.
    ///
    /// Represents end-user input in the conversation flow.
    Human,

    /// AI assistant message containing generated responses.
    ///
    /// Represents output from LLM or agent reasoning.
    Assistant,

    /// Tool execution result message.
    ///
    /// Contains output from function/tool calls, typically in structured format.
    Tool,

    /// Custom application-specific role.
    ///
    /// Enables domain-specific message types beyond standard roles.
    Custom(String),
}

/// Individual content part in a multimodal message.
///
/// `ContentPart` enables **multimodal messages** by representing different content types
/// (text, images, etc.) that can be combined in a single message. This matches the
/// format used by modern LLM APIs (OpenAI, Anthropic, Google) for vision and multimodal capabilities.
///
/// # Content Types
///
/// - **Text**: Textual content with optional caching hints
/// - **Image**: Visual content via URL or base64 data
/// - **Custom**: Application-specific content types
///
/// # Examples
///
/// ## Simple Text Part
///
/// ```rust
/// use langgraph_core::messages::ContentPart;
///
/// let text_part = ContentPart::text("Describe this image");
/// ```
///
/// ## Image from URL
///
/// ```rust
/// # use langgraph_core::messages::ContentPart;
/// let image_part = ContentPart::image_url("https://example.com/photo.jpg");
/// ```
///
/// ## Image from Base64 Data
///
/// ```rust
/// # use langgraph_core::messages::ContentPart;
/// let image_part = ContentPart::image_data(
///     "image/png",
///     "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
/// );
/// ```
///
/// ## Multimodal Message Composition
///
/// ```rust
/// use langgraph_core::messages::{Message, MessageRole, MessageContent, ContentPart};
///
/// let parts = vec![
///     ContentPart::text("What's in this image?"),
///     ContentPart::image_url("https://example.com/scene.jpg"),
/// ];
///
/// let multimodal_msg = Message {
///     id: Some("msg_1".to_string()),
///     role: MessageRole::Human,
///     content: MessageContent::Parts(parts),
///     name: None,
///     additional_kwargs: Default::default(),
/// };
/// ```
///
/// # Caching Support
///
/// Text parts support prompt caching for Anthropic's Claude API:
///
/// ```rust
/// # use langgraph_core::messages::ContentPart;
/// # use serde_json::json;
/// let cached_text = ContentPart::text_with_cache(
///     "Long system instructions...",
///     json!({"type": "ephemeral"})
/// );
/// ```
///
/// # Serialization Format
///
/// Serializes to tagged JSON compatible with LLM APIs:
///
/// ```json
/// // Text
/// {"type": "text", "text": "Hello"}
///
/// // Image URL
/// {"type": "image", "url": "https://..."}
///
/// // Image Base64
/// {
///   "type": "image",
///   "source": {
///     "type": "base64",
///     "media_type": "image/png",
///     "data": "iVBORw0K..."
///   }
/// }
/// ```
///
/// # See Also
///
/// - [`MessageContent`] - Container for one or more content parts
/// - [`Message`] - Full message structure
/// - [Anthropic Vision API](https://docs.anthropic.com/claude/docs/vision)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text content with optional caching control.
    ///
    /// The primary content type for textual messages. Supports Anthropic's
    /// prompt caching when `cache_control` is specified.
    Text {
        /// The text content
        text: String,
        /// Optional cache control hints (Anthropic-specific)
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<Value>,
    },

    /// Image content via URL or embedded data.
    ///
    /// Supports both URL references and base64-encoded image data.
    /// Use `ContentPart::image_url()` or `ContentPart::image_data()` constructors.
    Image {
        /// Image URL (mutually exclusive with source)
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        /// Base64 image data with metadata (mutually exclusive with url)
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<Value>,
    },

    /// Custom application-specific content type.
    ///
    /// Enables extension with domain-specific content formats.
    Custom {
        /// Arbitrary JSON data for custom content
        data: Value,
    },
}

impl ContentPart {
    /// Create a text content part
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            cache_control: None,
        }
    }

    /// Create a text content part with cache control
    pub fn text_with_cache(text: impl Into<String>, cache_control: Value) -> Self {
        Self::Text {
            text: text.into(),
            cache_control: Some(cache_control),
        }
    }

    /// Create an image content part from URL
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            url: Some(url.into()),
            source: None,
        }
    }

    /// Create an image content part from base64 data
    pub fn image_data(media_type: &str, data: &str) -> Self {
        Self::Image {
            url: None,
            source: Some(serde_json::json!({
                "type": "base64",
                "media_type": media_type,
                "data": data
            })),
        }
    }
}

/// Message content representation supporting both simple text and multimodal parts.
///
/// `MessageContent` provides a **flexible content model** that can represent either:
/// - Simple string messages (most common case)
/// - Multimodal messages with multiple content parts (text + images, etc.)
///
/// This enum uses `#[serde(untagged)]` to allow seamless deserialization from both
/// JSON strings and arrays, matching LLM API formats.
///
/// # When to Use Each Variant
///
/// ## Text Variant
/// Use for standard text-only messages:
/// - Simple chat messages
/// - Tool results as strings
/// - System instructions
/// - Single-modality content
///
/// ## Parts Variant
/// Use for multimodal or structured content:
/// - Messages with images (vision tasks)
/// - Mixed text + image content
/// - Content with caching annotations
/// - Complex structured outputs
///
/// # Examples
///
/// ## Simple Text Content
///
/// ```rust
/// use langgraph_core::messages::{Message, MessageRole, MessageContent};
///
/// let msg = Message {
///     id: Some("msg_1".to_string()),
///     role: MessageRole::Human,
///     content: MessageContent::Text("Hello!".to_string()),
///     name: None,
///     additional_kwargs: Default::default(),
///   };
/// ```
///
/// ## Automatic String Conversion
///
/// ```rust
/// # use langgraph_core::messages::{Message, MessageContent};
/// // String implements Into<MessageContent>
/// let content: MessageContent = "Hello!".into();
/// let content2: MessageContent = String::from("World!").into();
/// ```
///
/// ## Multimodal Content
///
/// ```rust
/// use langgraph_core::messages::{MessageContent, ContentPart};
///
/// let multimodal = MessageContent::Parts(vec![
///     ContentPart::text("Analyze this image:"),
///     ContentPart::image_url("https://example.com/diagram.png"),
///     ContentPart::text("What patterns do you see?"),
/// ]);
/// ```
///
/// ## Mixed Content with Vision
///
/// ```rust
/// # use langgraph_core::messages::{Message, MessageRole, MessageContent, ContentPart};
/// let vision_msg = Message {
///     id: Some("vision_1".to_string()),
///     role: MessageRole::Human,
///     content: MessageContent::Parts(vec![
///         ContentPart::text("Compare these two images:"),
///         ContentPart::image_url("https://example.com/before.jpg"),
///         ContentPart::image_url("https://example.com/after.jpg"),
///     ]),
///     name: None,
///     additional_kwargs: Default::default(),
/// };
/// ```
///
/// # JSON Serialization
///
/// The untagged serialization means:
///
/// ```json
/// // Text variant
/// "Hello, world!"
///
/// // Parts variant
/// [
///   {"type": "text", "text": "Hello"},
///   {"type": "image", "url": "https://..."}
/// ]
/// ```
///
/// # Migration Path
///
/// When upgrading from text-only to multimodal:
///
/// ```rust
/// # use langgraph_core::messages::{MessageContent, ContentPart};
/// // Before: Text-only
/// let old_content = MessageContent::Text("Hello".to_string());
///
/// // After: Multimodal-ready
/// let new_content = MessageContent::Parts(vec![
///     ContentPart::text("Hello"),
///     // Can now add images without changing structure
/// ]);
/// ```
///
/// # Performance Notes
///
/// - `Text` variant is zero-cost wrapper around String
/// - `Parts` variant allocates Vec, use for multimodal only
/// - Automatic conversions from `&str`/`String` are zero-copy when possible
///
/// # See Also
///
/// - [`ContentPart`] - Individual content part types
/// - [`Message`] - Full message structure using this content type
/// - [`MessageRole`] - Message sender role types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text-only content.
    ///
    /// The most common case for standard chat messages. Automatically
    /// used when constructing messages from strings.
    Text(String),

    /// Structured multimodal content with multiple parts.
    ///
    /// Enables vision, mixed content, and advanced formatting.
    /// Use when you need images, caching, or complex layouts.
    Parts(Vec<ContentPart>),
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl From<Vec<ContentPart>> for MessageContent {
    fn from(parts: Vec<ContentPart>) -> Self {
        Self::Parts(parts)
    }
}

/// Base message type for conversational AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for this message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Role of the message sender
    pub role: MessageRole,

    /// Message content
    pub content: MessageContent,

    /// Optional message name (for system messages, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Tool calls (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Tool call ID (for tool messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl Message {
    /// Create a new message with the given role and content
    pub fn new(role: MessageRole, content: impl Into<MessageContent>) -> Self {
        Self {
            id: Some(Uuid::new_v4().to_string()),
            role,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<MessageContent>) -> Self {
        Self::new(MessageRole::System, content)
    }

    /// Create a human message
    pub fn human(content: impl Into<MessageContent>) -> Self {
        Self::new(MessageRole::Human, content)
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<MessageContent>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    /// Create an AI assistant message (alias for `assistant`)
    ///
    /// This is a convenience method that's equivalent to `Message::assistant()`.
    /// Use whichever naming convention you prefer.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::messages::Message;
    ///
    /// let msg = Message::ai("Hello from AI!");
    /// assert_eq!(msg.text(), Some("Hello from AI!"));
    /// ```
    pub fn ai(content: impl Into<MessageContent>) -> Self {
        Self::assistant(content)
    }

    /// Create a user message (alias for `human`)
    ///
    /// This is a convenience method that's equivalent to `Message::human()`.
    /// Use whichever naming convention you prefer.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::messages::Message;
    ///
    /// let msg = Message::user("Hello from user!");
    /// assert_eq!(msg.text(), Some("Hello from user!"));
    /// ```
    pub fn user(content: impl Into<MessageContent>) -> Self {
        Self::human(content)
    }

    /// Create a tool message
    pub fn tool(content: impl Into<MessageContent>, tool_call_id: impl Into<String>) -> Self {
        Self {
            id: Some(Uuid::new_v4().to_string()),
            role: MessageRole::Tool,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            metadata: None,
        }
    }

    /// Set the message ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the message name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set tool calls (for assistant messages)
    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Get the text content if this is a simple text message
    pub fn text(&self) -> Option<&str> {
        match &self.content {
            MessageContent::Text(s) => Some(s),
            MessageContent::Parts(_) => None,
        }
    }

    /// Ensure this message has an ID (generate one if missing)
    pub fn ensure_id(&mut self) {
        if self.id.is_none() {
            self.id = Some(Uuid::new_v4().to_string());
        }
    }
}

/// Special marker for removing a message by ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveMessage {
    /// ID of the message to remove
    pub id: String,
}

impl RemoveMessage {
    /// Create a new remove message marker
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// Special ID to remove all messages
    pub const REMOVE_ALL: &'static str = "__remove_all__";

    /// Create a marker to remove all messages
    pub fn remove_all() -> Self {
        Self {
            id: Self::REMOVE_ALL.to_string(),
        }
    }
}

/// Flexible message input type supporting messages, deletions, and shortcuts.
///
/// `MessageLike` provides a **unified interface** for message operations in the
/// [`add_message_likes`] reducer, enabling:
/// - Adding full `Message` objects
/// - Deleting messages via `RemoveMessage` markers
/// - Quick message creation via `(role, content)` tuples
///
/// This type is the input to the advanced message reducer and enables sophisticated
/// conversation management including targeted deletions and bulk operations.
///
/// # Variants
///
/// - **Message**: Full message object with all metadata
/// - **Remove**: Deletion marker for removing specific messages or clearing history
/// - **Tuple**: Shorthand `(role, content)` for quick message creation
///
/// # Examples
///
/// ## Using Full Messages
///
/// ```rust
/// use langgraph_core::messages::{Message, MessageLike};
///
/// let likes = vec![
///     MessageLike::Message(Message::human("Hello").with_id("1")),
///     MessageLike::Message(Message::assistant("Hi there!").with_id("2")),
/// ];
/// ```
///
/// ## Using Tuple Shorthand
///
/// ```rust
/// # use langgraph_core::messages::MessageLike;
/// let likes = vec![
///     MessageLike::Tuple(("human".to_string(), "Quick message".to_string())),
///     MessageLike::Tuple(("assistant".to_string(), "Quick response".to_string())),
/// ];
/// ```
///
/// ## Mixing Operations
///
/// ```rust
/// use langgraph_core::messages::{Message, RemoveMessage, MessageLike};
///
/// let operations = vec![
///     // Keep first message
///     MessageLike::Message(Message::system("Context").with_id("sys")),
///     // Delete second message
///     MessageLike::Remove(RemoveMessage::new("old_msg")),
///     // Add new message via shorthand
///     MessageLike::Tuple(("human".to_string(), "New query".to_string())),
/// ];
/// ```
///
/// ## Complete Workflow with add_message_likes
///
/// ```rust
/// use langgraph_core::messages::{Message, RemoveMessage, MessageLike, add_message_likes};
///
/// // Existing conversation
/// let existing = vec![
///     MessageLike::Message(Message::human("First").with_id("1")),
///     MessageLike::Message(Message::assistant("Response").with_id("2")),
///     MessageLike::Message(Message::human("Error message").with_id("3")),
/// ];
///
/// // Update: remove error and add correction
/// let updates = vec![
///     MessageLike::Remove(RemoveMessage::new("3")),  // Delete error
///     MessageLike::Tuple(("human".to_string(), "Corrected message".to_string())),
/// ];
///
/// let result = add_message_likes(existing, updates);
/// assert_eq!(result.len(), 3); // sys, response, corrected
/// ```
///
/// ## Clearing History
///
/// ```rust
/// # use langgraph_core::messages::{Message, RemoveMessage, MessageLike, add_message_likes};
/// let old_conversation = vec![
///     MessageLike::Message(Message::human("Old topic...")),
///     // ... many messages ...
/// ];
///
/// let reset = vec![
///     MessageLike::Remove(RemoveMessage::remove_all()),
///     MessageLike::Tuple(("system".to_string(), "New conversation".to_string())),
/// ];
///
/// let fresh_start = add_message_likes(old_conversation, reset);
/// assert_eq!(fresh_start.len(), 1); // Only new system message
/// ```
///
/// # Automatic Conversions
///
/// The type implements `From` for convenient construction:
///
/// ```rust
/// # use langgraph_core::messages::{Message, RemoveMessage, MessageLike};
/// let from_message: MessageLike = Message::human("Hello").into();
/// let from_remove: MessageLike = RemoveMessage::new("msg_1").into();
/// let from_tuple: MessageLike = ("human".to_string(), "Hi".to_string()).into();
/// ```
///
/// # Serialization
///
/// Uses `#[serde(untagged)]` for flexible JSON representation:
///
/// ```json
/// // Message variant
/// {
///   "id": "msg_1",
///   "role": "human",
///   "content": "Hello"
/// }
///
/// // Remove variant
/// {
///   "remove": {"id": "msg_1"}
/// }
///
/// // Tuple variant
/// ["human", "Hello"]
/// ```
///
/// # Use Cases
///
/// 1. **Message Addition**: Standard message appending
/// 2. **Error Correction**: Remove incorrect messages and add fixes
/// 3. **History Management**: Clear old messages for token limits
/// 4. **Quick Prototyping**: Use tuples for fast testing
/// 5. **Batch Operations**: Mix multiple operation types
///
/// # See Also
///
/// - [`add_message_likes`] - Core reducer function using this type
/// - [`Message`] - Full message structure
/// - [`RemoveMessage`] - Deletion marker type
/// - [`add_messages`] - Simpler reducer for message-only operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageLike {
    /// Complete message with full metadata and control.
    ///
    /// Use when you need IDs, tool calls, or other message features.
    Message(Message),

    /// Deletion marker for removing messages or clearing history.
    ///
    /// - `RemoveMessage::new(id)` - Delete specific message
    /// - `RemoveMessage::remove_all()` - Clear all history
    Remove(RemoveMessage),

    /// Tuple shorthand for quick message creation.
    ///
    /// Format: `(role, content)` where role is "human", "assistant", etc.
    /// Messages created this way get auto-generated IDs.
    Tuple((String, String)),
}

impl From<Message> for MessageLike {
    fn from(m: Message) -> Self {
        Self::Message(m)
    }
}

impl From<RemoveMessage> for MessageLike {
    fn from(r: RemoveMessage) -> Self {
        Self::Remove(r)
    }
}

impl From<(String, String)> for MessageLike {
    fn from((role, content): (String, String)) -> Self {
        Self::Tuple((role, content))
    }
}

impl From<(&str, &str)> for MessageLike {
    fn from((role, content): (&str, &str)) -> Self {
        Self::Tuple((role.to_string(), content.to_string()))
    }
}

/// Convert message-like values to proper Messages
pub fn convert_to_messages(values: Vec<MessageLike>) -> Vec<Message> {
    values
        .into_iter()
        .filter_map(|v| match v {
            MessageLike::Message(m) => Some(m),
            MessageLike::Remove(_) => None,
            MessageLike::Tuple((role, content)) => {
                let role = match role.as_str() {
                    "system" => MessageRole::System,
                    "human" | "user" => MessageRole::Human,
                    "assistant" | "ai" => MessageRole::Assistant,
                    "tool" => MessageRole::Tool,
                    custom => MessageRole::Custom(custom.to_string()),
                };
                Some(Message::new(role, content))
            }
        })
        .collect()
}

/// Filter messages by role
///
/// Returns a new vector containing only messages matching the specified role.
///
/// # Example
///
/// ```rust
/// use langgraph_core::messages::{Message, MessageRole, filter_by_role};
///
/// let messages = vec![
///     Message::human("Hello"),
///     Message::assistant("Hi there!"),
///     Message::human("How are you?"),
/// ];
///
/// let human_msgs = filter_by_role(&messages, MessageRole::Human);
/// assert_eq!(human_msgs.len(), 2);
/// ```
pub fn filter_by_role(messages: &[Message], role: MessageRole) -> Vec<Message> {
    messages.iter()
        .filter(|m| m.role == role)
        .cloned()
        .collect()
}

/// Get the last message from a message list
///
/// Returns None if the list is empty.
///
/// # Example
///
/// ```rust
/// use langgraph_core::messages::{Message, get_last_message};
///
/// let messages = vec![
///     Message::human("Hello"),
///     Message::assistant("Hi!"),
/// ];
///
/// let last = get_last_message(&messages);
/// assert_eq!(last.unwrap().text(), Some("Hi!"));
/// ```
pub fn get_last_message(messages: &[Message]) -> Option<&Message> {
    messages.last()
}

/// Get messages by their IDs
///
/// Returns a vector of messages that match any of the provided IDs.
///
/// # Example
///
/// ```rust
/// use langgraph_core::messages::{Message, get_messages_by_id};
///
/// let messages = vec![
///     Message::human("First").with_id("1"),
///     Message::human("Second").with_id("2"),
///     Message::human("Third").with_id("3"),
/// ];
///
/// let selected = get_messages_by_id(&messages, &["1", "3"]);
/// assert_eq!(selected.len(), 2);
/// ```
pub fn get_messages_by_id(messages: &[Message], ids: &[&str]) -> Vec<Message> {
    messages.iter()
        .filter(|m| {
            m.id.as_ref()
                .map(|id| ids.contains(&id.as_str()))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

/// Merge consecutive messages with the same role
///
/// Combines adjacent messages from the same role into a single message.
/// Useful for consolidating multi-turn conversations.
///
/// # Example
///
/// ```rust
/// use langgraph_core::messages::{Message, merge_consecutive_messages};
///
/// let messages = vec![
///     Message::human("Hello"),
///     Message::human("How are you?"),  // Same role - will merge
///     Message::assistant("I'm good!"),
/// ];
///
/// let merged = merge_consecutive_messages(messages);
/// assert_eq!(merged.len(), 2);  // Two human messages merged into one
/// ```
pub fn merge_consecutive_messages(messages: Vec<Message>) -> Vec<Message> {
    if messages.is_empty() {
        return messages;
    }

    let mut result = Vec::new();
    let mut current = messages[0].clone();

    for message in messages.into_iter().skip(1) {
        if message.role == current.role {
            // Merge content
            match (&current.content, &message.content) {
                (MessageContent::Text(curr_text), MessageContent::Text(msg_text)) => {
                    current.content = MessageContent::Text(format!("{}\n{}", curr_text, msg_text));
                }
                _ => {
                    // For non-text content, just keep the current message
                }
            }
        } else {
            // Different role - push current and start new
            result.push(current);
            current = message;
        }
    }

    result.push(current);
    result
}

/// Truncate message history to a maximum number of messages
///
/// Keeps the most recent N messages. Useful for managing context window limits.
///
/// # Example
///
/// ```rust
/// use langgraph_core::messages::{Message, truncate_messages};
///
/// let messages = vec![
///     Message::human("Message 1"),
///     Message::assistant("Response 1"),
///     Message::human("Message 2"),
///     Message::assistant("Response 2"),
///     Message::human("Message 3"),
/// ];
///
/// let truncated = truncate_messages(messages, 3);
/// assert_eq!(truncated.len(), 3);  // Keeps last 3 messages
/// ```
pub fn truncate_messages(messages: Vec<Message>, max_count: usize) -> Vec<Message> {
    if messages.len() <= max_count {
        messages
    } else {
        let skip_count = messages.len() - max_count;
        messages.into_iter().skip(skip_count).collect()
    }
}

/// Options for trimming messages
#[derive(Debug, Clone)]
pub struct TrimOptions {
    /// Maximum number of messages to keep
    pub max_messages: usize,

    /// Strategy: "first" keeps oldest messages, "last" keeps newest messages
    pub strategy: TrimStrategy,

    /// Whether to preserve the system message if it's first
    pub include_system: bool,

    /// Ensure the history starts with a human message (after optional system message)
    pub start_on_human: bool,
}

/// Strategy for trimming messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimStrategy {
    /// Keep the first (oldest) messages
    First,
    /// Keep the last (newest) messages
    Last,
}

impl Default for TrimOptions {
    fn default() -> Self {
        Self {
            max_messages: 10,
            strategy: TrimStrategy::Last,
            include_system: true,
            start_on_human: true,
        }
    }
}

impl TrimOptions {
    /// Create options to keep the last N messages
    pub fn last(max_messages: usize) -> Self {
        Self {
            max_messages,
            strategy: TrimStrategy::Last,
            ..Default::default()
        }
    }

    /// Create options to keep the first N messages
    pub fn first(max_messages: usize) -> Self {
        Self {
            max_messages,
            strategy: TrimStrategy::First,
            ..Default::default()
        }
    }

    /// Set whether to preserve the system message
    pub fn with_include_system(mut self, include: bool) -> Self {
        self.include_system = include;
        self
    }

    /// Set whether to ensure history starts on human message
    pub fn with_start_on_human(mut self, start_on_human: bool) -> Self {
        self.start_on_human = start_on_human;
        self
    }
}

/// Trim messages with advanced options
///
/// This provides intelligent message history management that:
/// - Preserves system messages (if `include_system` is true)
/// - Ensures valid chat structure (starts with human message)
/// - Supports keeping either oldest or newest messages
///
/// # Example
///
/// ```rust
/// use langgraph_core::messages::{Message, trim_messages, TrimOptions};
///
/// let messages = vec![
///     Message::system("You are a helpful assistant"),
///     Message::human("Hello"),
///     Message::assistant("Hi there!"),
///     Message::human("How are you?"),
///     Message::assistant("I'm doing well!"),
/// ];
///
/// // Keep last 3 messages, preserve system message
/// let trimmed = trim_messages(messages, TrimOptions::last(3));
/// // Result: [system, human("How are you?"), assistant("I'm doing well!")]
/// ```
pub fn trim_messages(messages: Vec<Message>, options: TrimOptions) -> Vec<Message> {
    if messages.is_empty() {
        return messages;
    }

    let mut result = Vec::new();
    let mut working_messages = messages;

    // Extract system message if it exists and should be preserved
    let system_message = if options.include_system
        && !working_messages.is_empty()
        && working_messages[0].role == MessageRole::System
    {
        Some(working_messages.remove(0))
    } else {
        None
    };

    // Apply trimming strategy
    let trimmed = match options.strategy {
        TrimStrategy::Last => {
            if working_messages.len() > options.max_messages {
                let skip_count = working_messages.len() - options.max_messages;
                working_messages.into_iter().skip(skip_count).collect()
            } else {
                working_messages
            }
        }
        TrimStrategy::First => {
            working_messages.into_iter().take(options.max_messages).collect()
        }
    };

    // Add system message back if it was preserved
    if let Some(sys_msg) = system_message {
        result.push(sys_msg);
    }

    result.extend(trimmed);

    // Ensure starts on human message if requested
    if options.start_on_human {
        // Skip messages until we find a human message (but keep system if present)
        let system_count = if result.first().map(|m| &m.role) == Some(&MessageRole::System) {
            1
        } else {
            0
        };

        let first_human_idx = result[system_count..]
            .iter()
            .position(|m| m.role == MessageRole::Human)
            .map(|idx| idx + system_count);

        if let Some(idx) = first_human_idx {
            // Keep system message (if any) + messages from first human onward
            if system_count > 0 {
                let system = result[0].clone();
                result = result.into_iter().skip(idx).collect();
                result.insert(0, system);
            } else {
                result = result.into_iter().skip(idx).collect();
            }
        }
    }

    result
}

/// Manually emit a message to the streaming output during graph execution
///
/// This function allows nodes to incrementally stream message updates rather than
/// waiting until the node completes. It's useful for:
/// - Streaming LLM responses token by token
/// - Providing progress updates during long-running operations
/// - Emitting intermediate results
///
/// The message will be:
/// 1. Ensured to have an ID (generated if missing)
/// 2. Emitted via the stream writer (if `stream_mode="messages"` is active)
/// 3. Returned for further use
///
/// # Example
///
/// ```rust,no_run
/// use langgraph_core::messages::{Message, push_message};
/// use langgraph_core::StateGraph;
///
/// let mut graph = StateGraph::new();
///
/// graph.add_node("llm_node", |state| {
///     Box::pin(async move {
///         // Stream message chunks as they're generated
///         let chunk1 = Message::ai("Hello ").with_id("msg_1");
///         push_message(chunk1.clone());
///
///         let chunk2 = Message::ai("Hello world!").with_id("msg_1");
///         push_message(chunk2);
///
///         Ok(state)
///     })
/// });
/// ```
///
/// # Errors
///
/// Returns an error if the stream writer is not available or if sending fails.
pub fn push_message(mut message: Message) -> Result<Message, String> {
    use crate::runtime::get_stream_writer;

    // Ensure the message has an ID
    message.ensure_id();

    // Get the stream writer from the current runtime
    if let Some(writer) = get_stream_writer() {
        // Serialize message and emit to stream
        let message_json = serde_json::to_value(&message)
            .map_err(|e| format!("Failed to serialize message: {}", e))?;

        // Emit as a message event
        writer.write_message(message_json, None)?;
    }
    // If no stream writer available, silently succeed (not streaming mode)

    Ok(message)
}

/// Push multiple messages to the streaming output
///
/// Convenience function for emitting multiple messages at once.
///
/// # Example
///
/// ```rust,no_run
/// use langgraph_core::messages::{Message, push_messages};
///
/// let messages = vec![
///     Message::ai("First response"),
///     Message::ai("Second response"),
/// ];
///
/// push_messages(messages).ok();
/// ```
pub fn push_messages(messages: Vec<Message>) -> Result<Vec<Message>, String> {
    messages.into_iter().map(push_message).collect()
}

/// The add_messages reducer function
///
/// Intelligently merges message lists using ID-based deduplication.
/// - Messages with the same ID are replaced
/// - RemoveMessage markers delete messages by ID
/// - New messages are appended
/// - Missing IDs are auto-generated
///
/// # Examples
///
/// ```rust
/// use langgraph_core::messages::{Message, add_messages};
///
/// let msgs1 = vec![Message::human("Hello").with_id("1")];
/// let msgs2 = vec![Message::human("Hello again").with_id("1")];
/// let result = add_messages(msgs1, msgs2);
/// // Result: [Message { id: "1", content: "Hello again", ... }]
/// ```
pub fn add_messages(left: Vec<Message>, right: Vec<Message>) -> Vec<Message> {
    add_message_likes(
        left.into_iter().map(MessageLike::Message).collect(),
        right.into_iter().map(MessageLike::Message).collect(),
    )
}

/// Advanced message state reducer with comprehensive message management capabilities.
///
/// This is the **core reducer function** for message-based graph state, providing
/// sophisticated message history management including replacement, deletion, and
/// bulk operations. It's designed for conversational AI applications where precise
/// control over message history is critical.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────┐
/// │                   add_message_likes() Flow                      │
/// └─────────────────────────────────────────────────────────────────┘
///
///  Left Messages              Processing               Right Messages
///   (Existing)                                          (Updates)
///       │                        │                          │
///       ▼                        ▼                          ▼
/// ┌────────────┐         ┌──────────────┐         ┌────────────────┐
/// │ [Msg1(id:1),│         │ 1. Check for │         │ [Remove(id:1), │
/// │  Msg2(id:2)]│ ──────> │    REMOVE_ALL│ <────── │  Msg3(id:3),   │
/// └────────────┘         │ 2. Build Index│         │  Msg4(id:2)]   │
///                        │ 3. Apply Ops  │         └────────────────┘
///                        │ 4. Filter     │
///                        └──────────────┘
///                                │
///                                ▼
///                        ┌──────────────┐
///                        │ Final State: │
///                        │ [Msg4(id:2), │
///                        │  Msg3(id:3)] │
///                        └──────────────┘
/// ```
///
/// # Message Operations
///
/// ## 1. REMOVE_ALL - Complete History Reset
/// ```text
/// Before: [Msg1, Msg2, Msg3]
/// Operation: [REMOVE_ALL, Msg4, Msg5]
/// After: [Msg4, Msg5]
///
/// Everything before REMOVE_ALL is discarded
/// ```
///
/// ## 2. Targeted Deletion
/// ```text
/// Before: [Msg(id:1), Msg(id:2), Msg(id:3)]
/// Operation: [Remove(id:2)]
/// After: [Msg(id:1), Msg(id:3)]
/// ```
///
/// ## 3. Message Replacement
/// ```text
/// Before: [Msg(id:1, text:"old")]
/// Operation: [Msg(id:1, text:"new")]
/// After: [Msg(id:1, text:"new")]
/// ```
///
/// ## 4. Message Appending
/// ```text
/// Before: [Msg(id:1), Msg(id:2)]
/// Operation: [Msg(id:3)]
/// After: [Msg(id:1), Msg(id:2), Msg(id:3)]
/// ```
///
/// # Algorithm Deep Dive
///
/// ## Phase 1: REMOVE_ALL Check
/// ```rust,ignore
/// if right contains REMOVE_ALL:
///     return messages after REMOVE_ALL marker
/// ```
///
/// ## Phase 2: Index Building
/// ```rust,ignore
/// merged_by_id = HashMap<id, index>
/// for (i, msg) in left.enumerate():
///     if msg has id:
///         merged_by_id[msg.id] = i
/// ```
///
/// ## Phase 3: Operation Application
/// ```rust,ignore
/// for item in right:
///     match item:
///         Message(m) ->
///             if m.id exists:
///                 replace existing
///             else:
///                 append new
///         Remove(id) ->
///             mark id for deletion
///         Tuple(role, content) ->
///             convert to Message and process
/// ```
///
/// ## Phase 4: Filtering
/// ```rust,ignore
/// return merged.filter(|m| !ids_to_remove.contains(m.id))
/// ```
///
/// # ID Management
///
/// - **Auto-generation**: Messages without IDs get UUIDs assigned
/// - **Uniqueness**: Each message SHOULD have unique ID (not enforced)
/// - **Stability**: IDs persist through operations unless explicitly removed
///
/// # Behavior Matrix
///
/// | Left State | Right Operation | Result |
/// |------------|-----------------|---------|
/// | `[A, B]` | `[C]` | `[A, B, C]` - Append |
/// | `[A(id:1)]` | `[A'(id:1)]` | `[A'(id:1)]` - Replace |
/// | `[A, B]` | `[Remove(A)]` | `[B]` - Delete |
/// | `[A, B]` | `[REMOVE_ALL, C]` | `[C]` - Reset |
/// | `[]` | `[A]` | `[A]` - Initialize |
///
/// # Examples
///
/// ## Basic Usage
/// ```rust
/// use langgraph_core::messages::{Message, RemoveMessage, MessageLike, add_message_likes};
///
/// let msgs1 = vec![
///     MessageLike::Message(Message::human("Hello").with_id("1")),
///     MessageLike::Message(Message::assistant("Hi there!").with_id("2")),
/// ];
/// let msgs2 = vec![
///     MessageLike::Remove(RemoveMessage::new("1")), // Delete greeting
///     MessageLike::Message(Message::human("Let's start over").with_id("3")),
/// ];
/// let result = add_message_likes(msgs1, msgs2);
/// assert_eq!(result.len(), 2); // Message "1" deleted
/// ```
///
/// ## Conversation Management
/// ```rust
/// # use langgraph_core::messages::{Message, RemoveMessage, MessageLike, add_message_likes};
/// // Clear history and start fresh
/// let existing = vec![
///     MessageLike::Message(Message::human("old conversation...")),
///     // ... many messages ...
/// ];
/// let reset = vec![
///     MessageLike::Remove(RemoveMessage::remove_all()),
///     MessageLike::Message(Message::system("New conversation started")),
/// ];
/// let result = add_message_likes(existing, reset);
/// assert_eq!(result.len(), 1); // Only system message remains
/// ```
///
/// ## Error Correction
/// ```rust
/// # use langgraph_core::messages::{Message, MessageLike, add_message_likes};
/// // Replace a message that had an error
/// let msgs1 = vec![
///     MessageLike::Message(Message::assistant("2+2=5").with_id("math-1")),
/// ];
/// let correction = vec![
///     MessageLike::Message(Message::assistant("2+2=4").with_id("math-1")),
/// ];
/// let result = add_message_likes(msgs1, correction);
/// assert_eq!(result[0].content, "2+2=4");
/// ```
///
/// # Performance Characteristics
///
/// - **Time Complexity**: O(n + m) where n=left.len(), m=right.len()
/// - **Space Complexity**: O(n) for the ID index
/// - **Allocation**: Creates new Vec, doesn't modify inputs
///
/// # Error Cases
///
/// - **Panic**: Attempting to remove non-existent message ID
/// - **Silent**: Duplicate IDs in input (last one wins)
///
/// # Design Rationale
///
/// This function is designed for LLM conversation management where you need:
/// 1. **Precise control** over conversation history
/// 2. **Efficient updates** without rebuilding entire history
/// 3. **Audit trail** via explicit operations
/// 4. **Memory management** via REMOVE_ALL for long conversations
///
/// # See Also
///
/// - [`add_messages`] - Simpler version without deletion support
/// - [`Message`] - Core message type
/// - [`RemoveMessage`] - Deletion marker type
/// - [`MessageLike`] - Unified message operation enum
pub fn add_message_likes(
    left: Vec<MessageLike>,
    right: Vec<MessageLike>,
) -> Vec<Message> {
    // Check for REMOVE_ALL marker (both as RemoveMessage and as Message with REMOVE_ALL ID for backward compat)
    let remove_all_idx = right.iter().position(|ml| match ml {
        MessageLike::Remove(rm) => rm.id == RemoveMessage::REMOVE_ALL,
        MessageLike::Message(m) => m.id.as_deref() == Some(RemoveMessage::REMOVE_ALL),
        _ => false,
    });

    if let Some(idx) = remove_all_idx {
        // Return only messages after the REMOVE_ALL marker
        return right
            .into_iter()
            .skip(idx + 1)
            .filter_map(|ml| match ml {
                MessageLike::Message(mut m) => {
                    m.ensure_id();
                    Some(m)
                }
                MessageLike::Remove(_) => None,
                MessageLike::Tuple((role, content)) => {
                    let role = match role.as_str() {
                        "system" => MessageRole::System,
                        "human" | "user" => MessageRole::Human,
                        "assistant" | "ai" => MessageRole::Assistant,
                        "tool" => MessageRole::Tool,
                        custom => MessageRole::Custom(custom.to_string()),
                    };
                    Some(Message::new(role, content))
                }
            })
            .collect();
    }

    // Convert left to messages and ensure IDs
    let mut merged: Vec<Message> = left
        .into_iter()
        .filter_map(|ml| match ml {
            MessageLike::Message(mut m) => {
                m.ensure_id();
                Some(m)
            }
            MessageLike::Remove(_) => None, // Ignore remove markers in left
            MessageLike::Tuple((role, content)) => {
                let role = match role.as_str() {
                    "system" => MessageRole::System,
                    "human" | "user" => MessageRole::Human,
                    "assistant" | "ai" => MessageRole::Assistant,
                    "tool" => MessageRole::Tool,
                    custom => MessageRole::Custom(custom.to_string()),
                };
                let mut m = Message::new(role, content);
                m.ensure_id();
                Some(m)
            }
        })
        .collect();

    // Build index of existing messages by ID
    let mut merged_by_id: HashMap<String, usize> = merged
        .iter()
        .enumerate()
        .filter_map(|(i, m)| m.id.clone().map(|id| (id, i)))
        .collect();

    // Track IDs marked for removal
    let mut ids_to_remove = std::collections::HashSet::new();

    // Process right messages
    for ml in right {
        match ml {
            MessageLike::Message(mut m) => {
                m.ensure_id();
                let id = m.id.clone().unwrap(); // Safe because we called ensure_id

                if let Some(&existing_idx) = merged_by_id.get(&id) {
                    // Message with this ID exists
                    // If it was marked for removal, unmark it (we're replacing it)
                    ids_to_remove.remove(&id);
                    // Replace the existing message
                    merged[existing_idx] = m;
                } else {
                    // New message - append it
                    merged_by_id.insert(id.clone(), merged.len());
                    merged.push(m);
                }
            }
            MessageLike::Remove(rm) => {
                // Check if this ID exists in the merged messages
                if merged_by_id.contains_key(&rm.id) {
                    // Mark this ID for removal
                    ids_to_remove.insert(rm.id.clone());
                } else {
                    // Trying to remove a message that doesn't exist - this is an error
                    panic!(
                        "Attempting to delete a message with an ID that doesn't exist ('{}')",
                        rm.id
                    );
                }
            }
            MessageLike::Tuple((role, content)) => {
                let role = match role.as_str() {
                    "system" => MessageRole::System,
                    "human" | "user" => MessageRole::Human,
                    "assistant" | "ai" => MessageRole::Assistant,
                    "tool" => MessageRole::Tool,
                    custom => MessageRole::Custom(custom.to_string()),
                };
                let mut m = Message::new(role, content);
                m.ensure_id();
                let id = m.id.clone().unwrap();

                if let Some(&existing_idx) = merged_by_id.get(&id) {
                    ids_to_remove.remove(&id);
                    merged[existing_idx] = m;
                } else {
                    merged_by_id.insert(id.clone(), merged.len());
                    merged.push(m);
                }
            }
        }
    }

    // Filter out messages marked for removal
    merged
        .into_iter()
        .filter(|m| {
            !m.id
                .as_ref()
                .map(|id| ids_to_remove.contains(id))
                .unwrap_or(false)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::human("Hello world");
        assert_eq!(msg.role, MessageRole::Human);
        assert_eq!(msg.text(), Some("Hello world"));
        assert!(msg.id.is_some());
    }

    #[test]
    fn test_message_ai_alias() {
        // Test that Message::ai() is an alias for Message::assistant()
        let ai_msg = Message::ai("AI response");
        let assistant_msg = Message::assistant("AI response");

        assert_eq!(ai_msg.role, MessageRole::Assistant);
        assert_eq!(ai_msg.role, assistant_msg.role);
        assert_eq!(ai_msg.text(), Some("AI response"));
    }

    #[test]
    fn test_message_user_alias() {
        // Test that Message::user() is an alias for Message::human()
        let user_msg = Message::user("User input");
        let human_msg = Message::human("User input");

        assert_eq!(user_msg.role, MessageRole::Human);
        assert_eq!(user_msg.role, human_msg.role);
        assert_eq!(user_msg.text(), Some("User input"));
    }

    #[test]
    fn test_message_with_id() {
        let msg = Message::assistant("Response").with_id("msg_123");
        assert_eq!(msg.id, Some("msg_123".to_string()));
    }

    #[test]
    fn test_tool_message() {
        let msg = Message::tool("Result", "call_123");
        assert_eq!(msg.role, MessageRole::Tool);
        assert_eq!(msg.tool_call_id, Some("call_123".to_string()));
    }

    #[test]
    fn test_message_with_tool_calls() {
        use crate::tool::ToolCall;

        let msg = Message::assistant("Let me search")
            .with_tool_calls(vec![ToolCall {
                id: "call_1".to_string(),
                name: "search".to_string(),
                args: serde_json::json!({"query": "test"}),
            }]);

        assert!(msg.tool_calls.is_some());
        assert_eq!(msg.tool_calls.unwrap().len(), 1);
    }

    #[test]
    fn test_content_parts() {
        let parts = vec![
            ContentPart::text("Hello"),
            ContentPart::image_url("https://example.com/image.jpg"),
        ];
        let msg = Message::human(MessageContent::Parts(parts));

        match msg.content {
            MessageContent::Parts(p) => assert_eq!(p.len(), 2),
            _ => panic!("Expected Parts"),
        }
    }

    #[test]
    fn test_add_messages_append() {
        let msgs1 = vec![Message::human("First").with_id("1")];
        let msgs2 = vec![Message::human("Second").with_id("2")];

        let result = add_messages(msgs1, msgs2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, Some("1".to_string()));
        assert_eq!(result[1].id, Some("2".to_string()));
    }

    #[test]
    fn test_add_messages_replace() {
        let msgs1 = vec![Message::human("Original").with_id("1")];
        let msgs2 = vec![Message::human("Updated").with_id("1")];

        let result = add_messages(msgs1, msgs2);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text(), Some("Updated"));
    }

    #[test]
    fn test_add_messages_auto_id() {
        let mut msg1 = Message::human("First");
        msg1.id = None;
        let mut msg2 = Message::human("Second");
        msg2.id = None;

        let result = add_messages(vec![msg1], vec![msg2]);
        assert_eq!(result.len(), 2);
        assert!(result[0].id.is_some());
        assert!(result[1].id.is_some());
        // Different IDs because they're auto-generated
        assert_ne!(result[0].id, result[1].id);
    }

    #[test]
    fn test_add_messages_remove_all() {
        let msgs1 = vec![
            Message::human("First").with_id("1"),
            Message::human("Second").with_id("2"),
        ];

        let mut remove_all = Message::human("placeholder");
        remove_all.id = Some(RemoveMessage::REMOVE_ALL.to_string());
        let msgs2 = vec![remove_all, Message::human("New").with_id("3")];

        let result = add_messages(msgs1, msgs2);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, Some("3".to_string()));
    }

    #[test]
    fn test_convert_to_messages() {
        let values = vec![
            MessageLike::Tuple(("human".to_string(), "Hello".to_string())),
            MessageLike::Message(Message::assistant("Hi")),
        ];

        let messages = convert_to_messages(values);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::Human);
        assert_eq!(messages[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_remove_message_marker() {
        let remove = RemoveMessage::new("msg_123");
        assert_eq!(remove.id, "msg_123");

        let remove_all = RemoveMessage::remove_all();
        assert_eq!(remove_all.id, RemoveMessage::REMOVE_ALL);
    }

    #[test]
    fn test_add_message_likes_remove_by_id() {
        // Test removing a specific message by ID
        let msgs1 = vec![
            MessageLike::Message(Message::human("First").with_id("1")),
            MessageLike::Message(Message::human("Second").with_id("2")),
            MessageLike::Message(Message::human("Third").with_id("3")),
        ];

        let msgs2 = vec![
            MessageLike::Remove(RemoveMessage::new("2")), // Remove message with id="2"
        ];

        let result = add_message_likes(msgs1, msgs2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, Some("1".to_string()));
        assert_eq!(result[0].text(), Some("First"));
        assert_eq!(result[1].id, Some("3".to_string()));
        assert_eq!(result[1].text(), Some("Third"));
    }

    #[test]
    fn test_add_message_likes_remove_multiple() {
        // Test removing multiple messages
        let msgs1 = vec![
            MessageLike::Message(Message::human("First").with_id("1")),
            MessageLike::Message(Message::human("Second").with_id("2")),
            MessageLike::Message(Message::human("Third").with_id("3")),
            MessageLike::Message(Message::human("Fourth").with_id("4")),
        ];

        let msgs2 = vec![
            MessageLike::Remove(RemoveMessage::new("1")),
            MessageLike::Remove(RemoveMessage::new("3")),
        ];

        let result = add_message_likes(msgs1, msgs2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text(), Some("Second"));
        assert_eq!(result[1].text(), Some("Fourth"));
    }

    #[test]
    fn test_add_message_likes_remove_and_add() {
        // Test removing and adding messages in the same operation
        let msgs1 = vec![
            MessageLike::Message(Message::human("First").with_id("1")),
            MessageLike::Message(Message::human("Second").with_id("2")),
        ];

        let msgs2 = vec![
            MessageLike::Remove(RemoveMessage::new("1")),
            MessageLike::Message(Message::human("Third").with_id("3")),
        ];

        let result = add_message_likes(msgs1, msgs2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text(), Some("Second"));
        assert_eq!(result[1].text(), Some("Third"));
    }

    #[test]
    fn test_add_message_likes_remove_then_replace() {
        // Test that replacing a message after marking it for removal works
        let msgs1 = vec![
            MessageLike::Message(Message::human("First").with_id("1")),
            MessageLike::Message(Message::human("Second").with_id("2")),
        ];

        let msgs2 = vec![
            MessageLike::Remove(RemoveMessage::new("2")),
            MessageLike::Message(Message::human("New Second").with_id("2")), // Replace after removal
        ];

        let result = add_message_likes(msgs1, msgs2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text(), Some("First"));
        assert_eq!(result[1].text(), Some("New Second")); // Should be replaced, not removed
    }

    #[test]
    #[should_panic(expected = "Attempting to delete a message with an ID that doesn't exist")]
    fn test_add_message_likes_remove_nonexistent() {
        // Test that removing a non-existent message panics
        let msgs1 = vec![
            MessageLike::Message(Message::human("First").with_id("1")),
        ];

        let msgs2 = vec![
            MessageLike::Remove(RemoveMessage::new("999")), // ID doesn't exist
        ];

        add_message_likes(msgs1, msgs2); // Should panic
    }

    #[test]
    fn test_add_message_likes_with_tuples() {
        // Test that tuple syntax works with remove markers
        let msgs1 = vec![
            MessageLike::Tuple(("human".to_string(), "First".to_string())),
            MessageLike::Message(Message::human("Second").with_id("2")),
        ];

        let msgs2 = vec![
            MessageLike::Remove(RemoveMessage::new("2")),
            MessageLike::Tuple(("assistant".to_string(), "Response".to_string())),
        ];

        let result = add_message_likes(msgs1, msgs2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, MessageRole::Human);
        assert_eq!(result[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_add_message_likes_remove_all_with_additions() {
        // Test REMOVE_ALL with new messages after it
        let msgs1 = vec![
            MessageLike::Message(Message::human("Old 1").with_id("1")),
            MessageLike::Message(Message::human("Old 2").with_id("2")),
        ];

        let msgs2 = vec![
            MessageLike::Remove(RemoveMessage::remove_all()),
            MessageLike::Message(Message::human("New 1").with_id("3")),
            MessageLike::Message(Message::human("New 2").with_id("4")),
        ];

        let result = add_message_likes(msgs1, msgs2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text(), Some("New 1"));
        assert_eq!(result[1].text(), Some("New 2"));
    }

    #[test]
    fn test_filter_by_role() {
        let messages = vec![
            Message::human("Hello"),
            Message::assistant("Hi"),
            Message::human("How are you?"),
            Message::assistant("Good!"),
        ];

        let human_msgs = filter_by_role(&messages, MessageRole::Human);
        assert_eq!(human_msgs.len(), 2);
        assert_eq!(human_msgs[0].text(), Some("Hello"));
        assert_eq!(human_msgs[1].text(), Some("How are you?"));

        let assistant_msgs = filter_by_role(&messages, MessageRole::Assistant);
        assert_eq!(assistant_msgs.len(), 2);
    }

    #[test]
    fn test_get_last_message() {
        let messages = vec![
            Message::human("First"),
            Message::assistant("Second"),
        ];

        let last = get_last_message(&messages);
        assert!(last.is_some());
        assert_eq!(last.unwrap().text(), Some("Second"));

        let empty: Vec<Message> = vec![];
        assert!(get_last_message(&empty).is_none());
    }

    #[test]
    fn test_get_messages_by_id() {
        let messages = vec![
            Message::human("First").with_id("1"),
            Message::human("Second").with_id("2"),
            Message::human("Third").with_id("3"),
        ];

        let selected = get_messages_by_id(&messages, &["1", "3"]);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].text(), Some("First"));
        assert_eq!(selected[1].text(), Some("Third"));

        let none_selected = get_messages_by_id(&messages, &["99"]);
        assert_eq!(none_selected.len(), 0);
    }

    #[test]
    fn test_merge_consecutive_messages() {
        let messages = vec![
            Message::human("Hello"),
            Message::human("How are you?"),
            Message::assistant("I'm good!"),
            Message::assistant("Thanks for asking."),
            Message::human("Great!"),
        ];

        let merged = merge_consecutive_messages(messages);
        assert_eq!(merged.len(), 3);

        // First two human messages merged
        assert_eq!(merged[0].role, MessageRole::Human);
        assert_eq!(merged[0].text(), Some("Hello\nHow are you?"));

        // Two assistant messages merged
        assert_eq!(merged[1].role, MessageRole::Assistant);
        assert_eq!(merged[1].text(), Some("I'm good!\nThanks for asking."));

        // Last human message
        assert_eq!(merged[2].role, MessageRole::Human);
        assert_eq!(merged[2].text(), Some("Great!"));
    }

    #[test]
    fn test_truncate_messages() {
        let messages = vec![
            Message::human("1"),
            Message::assistant("2"),
            Message::human("3"),
            Message::assistant("4"),
            Message::human("5"),
        ];

        let truncated = truncate_messages(messages.clone(), 3);
        assert_eq!(truncated.len(), 3);
        assert_eq!(truncated[0].text(), Some("3"));
        assert_eq!(truncated[1].text(), Some("4"));
        assert_eq!(truncated[2].text(), Some("5"));

        // Test with max >= length
        let not_truncated = truncate_messages(messages.clone(), 10);
        assert_eq!(not_truncated.len(), 5);

        // Test with exact length
        let exact = truncate_messages(messages, 5);
        assert_eq!(exact.len(), 5);
    }

    #[test]
    fn test_push_message_without_runtime() {
        // Test that push_message works gracefully when no runtime is available
        let msg = Message::ai("Test message");
        let result = push_message(msg.clone());

        // Should succeed even without runtime (just doesn't stream)
        assert!(result.is_ok());
        let pushed_msg = result.unwrap();
        assert!(pushed_msg.id.is_some());
        assert_eq!(pushed_msg.text(), Some("Test message"));
    }

    #[test]
    fn test_push_messages_multiple() {
        // Test pushing multiple messages
        let messages = vec![
            Message::ai("First"),
            Message::human("Second"),
            Message::ai("Third"),
        ];

        let result = push_messages(messages);
        assert!(result.is_ok());
        let pushed = result.unwrap();
        assert_eq!(pushed.len(), 3);
        assert!(pushed.iter().all(|m| m.id.is_some()));
    }

    #[test]
    fn test_push_message_generates_id() {
        // Test that push_message generates an ID if missing
        let mut msg = Message::ai("No ID yet");
        msg.id = None;

        let result = push_message(msg);
        assert!(result.is_ok());
        let pushed_msg = result.unwrap();
        assert!(pushed_msg.id.is_some(), "ID should be generated");
    }

    #[test]
    fn test_trim_messages_last_strategy() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::human("Hello"),
            Message::assistant("Hi there!"),
            Message::human("How are you?"),
            Message::assistant("I'm good!"),
        ];

        let trimmed = trim_messages(messages, TrimOptions::last(2));

        // Should keep: system + last 2 messages (starting from human)
        assert_eq!(trimmed.len(), 3);
        assert_eq!(trimmed[0].role, MessageRole::System);
        assert_eq!(trimmed[1].role, MessageRole::Human);
        assert_eq!(trimmed[1].text(), Some("How are you?"));
        assert_eq!(trimmed[2].role, MessageRole::Assistant);
    }

    #[test]
    fn test_trim_messages_first_strategy() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::human("First"),
            Message::assistant("Response 1"),
            Message::human("Second"),
            Message::assistant("Response 2"),
        ];

        let trimmed = trim_messages(messages, TrimOptions::first(2));

        // Should keep: system + first 2 messages
        assert_eq!(trimmed.len(), 3);
        assert_eq!(trimmed[0].role, MessageRole::System);
        assert_eq!(trimmed[1].text(), Some("First"));
        assert_eq!(trimmed[2].text(), Some("Response 1"));
    }

    #[test]
    fn test_trim_messages_without_system_preservation() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::human("Hello"),
            Message::assistant("Hi!"),
        ];

        let opts = TrimOptions::last(2).with_include_system(false);
        let trimmed = trim_messages(messages, opts);

        // Should not preserve system message
        assert_eq!(trimmed.len(), 2);
        assert_eq!(trimmed[0].role, MessageRole::Human);
        assert_eq!(trimmed[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_trim_messages_start_on_human() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::assistant("Let me help"), // Should be removed
            Message::human("Hello"),
            Message::assistant("Hi!"),
        ];

        let trimmed = trim_messages(messages, TrimOptions::last(10));

        // Should skip the assistant message before first human
        assert_eq!(trimmed.len(), 3);
        assert_eq!(trimmed[0].role, MessageRole::System);
        assert_eq!(trimmed[1].role, MessageRole::Human);
        assert_eq!(trimmed[2].role, MessageRole::Assistant);
    }

    #[test]
    fn test_trim_messages_no_human_message() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::assistant("Response 1"),
            Message::assistant("Response 2"),
        ];

        let trimmed = trim_messages(messages, TrimOptions::last(5));

        // If no human message exists and start_on_human is true,
        // we can't find a human to start from, so we keep all messages
        assert_eq!(trimmed.len(), 3);
        assert_eq!(trimmed[0].role, MessageRole::System);
        assert_eq!(trimmed[1].text(), Some("Response 1"));
        assert_eq!(trimmed[2].text(), Some("Response 2"));
    }

    #[test]
    fn test_trim_messages_disable_start_on_human() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::assistant("Let me help"),
            Message::human("Hello"),
            Message::assistant("Hi!"),
        ];

        let opts = TrimOptions::last(10).with_start_on_human(false);
        let trimmed = trim_messages(messages, opts);

        // Should keep all messages including the assistant before human
        assert_eq!(trimmed.len(), 4);
        assert_eq!(trimmed[1].text(), Some("Let me help"));
    }

    #[test]
    fn test_trim_messages_empty_list() {
        let messages: Vec<Message> = vec![];
        let trimmed = trim_messages(messages, TrimOptions::last(5));
        assert_eq!(trimmed.len(), 0);
    }

    #[test]
    fn test_trim_options_builder() {
        let opts = TrimOptions::last(5)
            .with_include_system(false)
            .with_start_on_human(false);

        assert_eq!(opts.max_messages, 5);
        assert_eq!(opts.strategy, TrimStrategy::Last);
        assert_eq!(opts.include_system, false);
        assert_eq!(opts.start_on_human, false);
    }
}
