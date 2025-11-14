//! LLM integration traits and types for rLangGraph.
//!
//! This module provides **traits** for integrating LLM providers with rLangGraph.
//! The framework does not include concrete LLM implementations - users implement
//! the [`ChatModel`] trait for their chosen provider.
//!
//! # Architecture Philosophy
//!
//! rLangGraph is an **orchestration framework**, not an LLM client library:
//! - Core library provides **traits** and types
//! - **Users implement** `ChatModel` for their LLM provider
//! - Framework remains provider-agnostic
//! - Focus on graph execution, not API integration
//!
//! # Supported LLM Types
//!
//! The trait architecture supports all categories of LLMs:
//!
//! ## Thinking Models
//! Models with extended reasoning (OpenAI o1, DeepSeek R1, etc.)
//! - Use [`ReasoningMode::Separated`] to access thinking process
//! - Access via `response.reasoning.content`
//!
//! ## Local Models
//! Self-hosted models (Ollama, llama.cpp, etc.)
//! - No API keys required
//! - Lower latency, better privacy
//! - Implement connection checking via `is_available()`
//!
//! ## Remote Models
//! Cloud-hosted APIs (OpenAI, Anthropic, etc.)
//! - Authentication handled by implementation
//! - Rate limiting in implementation
//! - Network error handling required
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use langgraph_core::llm::{ChatModel, ChatRequest, ChatResponse, ReasoningMode};
//! use async_trait::async_trait;
//!
//! // 1. Implement the ChatModel trait for your provider
//! struct MyLLMClient {
//!     api_key: String,
//!     model: String,
//! }
//!
//! #[async_trait]
//! impl ChatModel for MyLLMClient {
//!     async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
//!         // Convert messages to your provider's format
//!         // Make API call
//!         // Convert response back to rLangGraph format
//!         todo!()
//!     }
//!
//!     async fn stream(&self, request: ChatRequest) -> Result<ChatStreamResponse> {
//!         todo!()
//!     }
//! }
//!
//! // 2. Use in graph nodes
//! let model: Arc<dyn ChatModel> = Arc::new(MyLLMClient { /* ... */ });
//!
//! let request = ChatRequest::new(vec![Message::human("Hello!")])
//!     .with_temperature(0.7)
//!     .with_reasoning(ReasoningMode::Separated);
//!
//! let response = model.chat(request).await?;
//! println!("Response: {}", response.message.text());
//! ```
//!
//! # See Also
//!
//! - [`ChatModel`] - The main trait to implement
//! - [`ChatRequest`] - Request configuration with builder pattern
//! - [`ChatResponse`] - Response with message, usage, reasoning
//! - [`ReasoningMode`] - Control thinking model behavior
//! - [`ToolDefinition`] - Define functions for tool calling
//!
//! # Example Implementations
//!
//! Reference implementations are available in `examples/ollama_subgraph/llm_providers/`:
//! - `ollama.rs` - Ollama local model client
//! - More coming soon!

// Core trait and types
pub mod traits;
pub mod config;
pub mod response;
pub mod tools;

// Re-exports for convenient access
pub use traits::ChatModel;
pub use config::{ChatConfig, ChatRequest, ReasoningMode};
pub use response::{ChatResponse, ChatStreamResponse, ReasoningContent, UsageMetadata};
pub use tools::{ToolCall, ToolDefinition, ToolResult};

// Re-export streaming types from parent module
pub use crate::llm_stream::{MessageChunk, MessageChunkStream, TokenStream};
