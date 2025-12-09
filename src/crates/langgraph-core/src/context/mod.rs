//! Context Management Module
//!
//! Provides automatic context summarization to prevent token overflow in long-running
//! agent sessions. Inspired by DeepAgents' automatic summarization when tokens exceed
//! a threshold (e.g., 170k tokens).
//!
//! # Features
//!
//! - **Token Counting**: Accurate token estimation using tiktoken
//! - **Automatic Summarization**: Compress conversation history when approaching limits
//! - **Configurable Thresholds**: Set max tokens and summarization trigger points
//! - **Message Preservation**: Always keeps system prompt and recent messages intact
//!
//! # Example
//!
//! ```rust,ignore
//! use langgraph_core::context::{ContextManager, ContextConfig};
//! use langgraph_core::Message;
//!
//! let config = ContextConfig::default()
//!     .with_max_tokens(100_000)
//!     .with_threshold(0.8);
//!
//! let mut manager = ContextManager::new(config);
//!
//! // Add messages to context
//! let mut messages = vec![
//!     Message::system("You are a helpful assistant"),
//!     Message::human("Hello!"),
//!     Message::assistant("Hi there!"),
//!     // ... many more messages
//! ];
//!
//! // Check if summarization is needed and apply it
//! if manager.should_summarize(&messages) {
//!     manager.summarize(&mut messages, summarizer_fn).await?;
//! }
//! ```

mod manager;
mod token_counter;

pub use manager::{ContextConfig, ContextManager, SummarizationResult};
pub use token_counter::{TiktokenCounter, TokenCounter};
