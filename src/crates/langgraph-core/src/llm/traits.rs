//! Core traits for LLM integration.
//!
//! This module defines the foundational traits that allow users to integrate
//! their own LLM providers (OpenAI, Anthropic, Ollama, llama.cpp, etc.) with
//! rLangGraph.
//!
//! # Architecture Philosophy
//!
//! rLangGraph is an **orchestration framework**, not an LLM client library.
//! Therefore:
//! - The core library provides **traits** for LLM interaction
//! - **Users implement** these traits for their chosen LLM provider
//! - The framework remains provider-agnostic and focused on graph execution
//!
//! # Design Principles
//!
//! 1. **Minimal Core**: The trait includes only essential methods (chat, stream)
//! 2. **Config-Driven**: Features like reasoning modes are configuration options
//! 3. **Tool Support**: Tool calling is a first-class concern
//! 4. **Async-First**: All I/O operations are asynchronous
//! 5. **Provider-Agnostic**: Works with any LLM (local, remote, thinking models)
//!
//! # Example Implementation
//!
//! ```rust,ignore
//! use langgraph_core::llm::{ChatModel, ChatRequest, ChatResponse};
//! use async_trait::async_trait;
//!
//! struct MyLLMClient {
//!     api_key: String,
//!     model: String,
//! }
//!
//! #[async_trait]
//! impl ChatModel for MyLLMClient {
//!     async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
//!         // 1. Convert rLangGraph messages to provider format
//!         // 2. Make API call
//!         // 3. Convert response back to rLangGraph format
//!         // 4. Return ChatResponse
//!         todo!("Implement your LLM provider")
//!     }
//!
//!     async fn stream(&self, request: ChatRequest) -> Result<ChatStreamResponse> {
//!         // Similar but returns a stream
//!         todo!("Implement streaming")
//!     }
//! }
//! ```

use crate::error::Result;
use crate::llm::config::ChatRequest;
use crate::llm::response::{ChatResponse, ChatStreamResponse};
use crate::llm::tools::ToolDefinition;
use async_trait::async_trait;

/// Core trait for chat-based language models.
///
/// This trait provides a minimal, provider-agnostic interface for interacting
/// with LLMs. Implementations handle the specifics of converting messages,
/// making API calls, and parsing responses for their particular provider.
///
/// # Supported Model Types
///
/// This trait supports three categories of LLMs:
///
/// ## 1. Thinking Models (OpenAI o1, DeepSeek R1, etc.)
/// Models that produce extended reasoning before generating a final answer.
/// - Use `ReasoningMode::Separated` in request config
/// - Access reasoning via `response.reasoning`
///
/// ## 2. Local Models (Ollama, llama.cpp, etc.)
/// Self-hosted models running on localhost or local network.
/// - No API keys needed
/// - Lower latency
/// - Privacy benefits
///
/// ## 3. Remote Models (OpenAI, Anthropic, etc.)
/// Cloud-hosted models accessed via API.
/// - API authentication handled by implementation
/// - Implementation should handle rate limiting
/// - Network error handling required
///
/// # Tool Calling
///
/// Models that support function/tool calling should:
/// 1. Accept `ToolDefinition`s via `with_tools()` on `ChatRequest`
/// 2. Return tool calls in `response.message.tool_calls`
/// 3. Accept tool results in subsequent messages
///
/// # Threading and Safety
///
/// Implementations must be `Send + Sync` to work with rLangGraph's async runtime.
/// Use `Arc<dyn ChatModel>` to share across graph nodes.
#[async_trait]
pub trait ChatModel: Send + Sync {
    /// Generate a complete chat response from messages.
    ///
    /// This is the primary method for LLM interaction. It takes a request
    /// containing messages and configuration, makes the LLM call, and returns
    /// a complete response.
    ///
    /// # Arguments
    ///
    /// * `request` - The chat request containing messages and configuration
    ///
    /// # Returns
    ///
    /// A `ChatResponse` containing:
    /// - The assistant's response message
    /// - Token usage statistics
    /// - Optional reasoning content (for thinking models)
    /// - Provider-specific metadata
    ///
    /// # Errors
    ///
    /// Implementations should return `GraphError::Validation` for:
    /// - Network failures
    /// - Authentication errors
    /// - Invalid requests
    /// - Rate limiting
    /// - Model not found
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let request = ChatRequest::new(vec![
    ///     Message::human("What is 2 + 2?")
    /// ]).with_temperature(0.7);
    ///
    /// let response = model.chat(request).await?;
    /// println!("Answer: {}", response.message.text());
    /// ```
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;

    /// Stream a chat response token by token.
    ///
    /// This method provides real-time streaming of the LLM's output, which is
    /// useful for providing responsive UIs or processing partial results.
    ///
    /// # Arguments
    ///
    /// * `request` - The chat request containing messages and configuration
    ///
    /// # Returns
    ///
    /// A `ChatStreamResponse` containing:
    /// - A stream of message chunks
    /// - Optional reasoning stream (for thinking models)
    /// - Final usage statistics (when stream completes)
    ///
    /// # Errors
    ///
    /// Same error conditions as `chat()`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use futures::StreamExt;
    ///
    /// let request = ChatRequest::new(messages);
    /// let mut stream_response = model.stream(request).await?;
    ///
    /// while let Some(chunk) = stream_response.stream.next().await {
    ///     print!("{}", chunk.content);
    /// }
    /// ```
    async fn stream(&self, request: ChatRequest) -> Result<ChatStreamResponse>;

    /// Check if the model/provider is available and healthy.
    ///
    /// This method is particularly useful for local models (Ollama, llama.cpp)
    /// where the server might not be running, or for checking API connectivity.
    ///
    /// Default implementation returns `Ok(true)`, assuming availability.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if !model.is_available().await? {
    ///     return Err(GraphError::Validation(
    ///         "Ollama server is not running".to_string()
    ///     ));
    /// }
    /// ```
    async fn is_available(&self) -> Result<bool> {
        Ok(true)
    }

    /// Get a list of tools/functions that can be bound to this model.
    ///
    /// Implementations that support tool calling should override this to return
    /// the tools that have been configured.
    ///
    /// Default implementation returns an empty vector (no tools).
    fn bound_tools(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    /// Clone this model into a boxed trait object.
    ///
    /// This method enables cloning of `Arc<dyn ChatModel>` and similar patterns.
    /// Implementations typically just clone the underlying struct.
    ///
    /// # Example Implementation
    ///
    /// ```rust,ignore
    /// fn clone_box(&self) -> Box<dyn ChatModel> {
    ///     Box::new(self.clone())
    /// }
    /// ```
    fn clone_box(&self) -> Box<dyn ChatModel>;
}

/// Enable cloning for boxed ChatModel trait objects.
impl Clone for Box<dyn ChatModel> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Message, MessageRole};
    use std::sync::Arc;

    /// Mock model for testing trait usage patterns.
    #[derive(Clone)]
    struct MockModel {
        response_text: String,
    }

    #[async_trait]
    impl ChatModel for MockModel {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
            use crate::llm::response::{ChatResponse, UsageMetadata};
            use crate::MessageContent;

            Ok(ChatResponse {
                message: Message {
                    id: None,
                    role: MessageRole::Assistant,
                    content: MessageContent::Text(self.response_text.clone()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    metadata: None,
                },
                usage: Some(UsageMetadata {
                    input_tokens: 10,
                    output_tokens: 5,
                    reasoning_tokens: None,
                    total_tokens: 15,
                }),
                reasoning: None,
                metadata: std::collections::HashMap::new(),
            })
        }

        async fn stream(&self, _request: ChatRequest) -> Result<ChatStreamResponse> {
            todo!("Mock stream implementation")
        }

        fn clone_box(&self) -> Box<dyn ChatModel> {
            Box::new(self.clone())
        }
    }

    #[tokio::test]
    async fn test_trait_object() {
        let model: Arc<dyn ChatModel> = Arc::new(MockModel {
            response_text: "Hello!".to_string(),
        });

        let request = ChatRequest::new(vec![Message::human("Hi")]);
        let response = model.chat(request).await.unwrap();

        assert_eq!(response.message.text(), Some("Hello!"));
    }

    #[tokio::test]
    async fn test_default_is_available() {
        let model = MockModel {
            response_text: "test".to_string(),
        };

        assert!(model.is_available().await.unwrap());
    }
}
