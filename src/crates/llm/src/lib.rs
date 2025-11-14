//! LLM provider implementations for acolib.
//!
//! This crate provides concrete implementations of the `ChatModel` trait from
//! `langgraph-core` for various LLM providers, both local and remote.
//!
//! # Local Providers
//!
//! Local providers connect to LLM servers running on localhost or local network:
//! - **Ollama** - Popular local LLM runner with wide model support
//! - **llama.cpp** - Direct llama.cpp server integration
//! - **LM Studio** - User-friendly local LLM interface
//!
//! # Remote Providers
//!
//! Remote providers connect to cloud-hosted LLM APIs:
//! - **Claude** - Anthropic's Claude models (Claude 3, etc.)
//! - **OpenAI** - OpenAI models (GPT-4, o1, etc.)
//! - **Grok** - xAI's Grok models
//! - **Deepseek** - Deepseek models including R1 (thinking model)
//! - **OpenRouter** - Unified API for multiple providers
//!
//! # Example Usage
//!
//! ## Local Provider (Ollama)
//!
//! ```rust,ignore
//! use llm::local::OllamaClient;
//! use llm::config::LocalLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
//!     let client = OllamaClient::new(config);
//!
//!     let request = ChatRequest::new(vec![
//!         Message::human("What is Rust?")
//!     ]);
//!
//!     let response = client.chat(request).await?;
//!     println!("Response: {}", response.message.text().unwrap());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Remote Provider (OpenAI)
//!
//! ```rust,ignore
//! use llm::remote::OpenAiClient;
//! use llm::config::RemoteLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = RemoteLlmConfig::from_env(
//!         "OPENAI_API_KEY",
//!         "https://api.openai.com/v1",
//!         "gpt-4"
//!     )?;
//!     let client = OpenAiClient::new(config);
//!
//!     let request = ChatRequest::new(vec![
//!         Message::human("Explain quantum computing briefly")
//!     ]).with_temperature(0.7);
//!
//!     let response = client.chat(request).await?;
//!     println!("Response: {}", response.message.text().unwrap());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Remote Provider (Google Gemini)
//!
//! ```rust,ignore
//! use llm::remote::GeminiClient;
//! use llm::config::RemoteLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = RemoteLlmConfig::from_env(
//!         "GOOGLE_API_KEY",
//!         "https://generativelanguage.googleapis.com/v1beta",
//!         "gemini-pro"
//!     )?;
//!     let client = GeminiClient::new(config);
//!
//!     let request = ChatRequest::new(vec![
//!         Message::human("What is machine learning?")
//!     ]).with_temperature(0.7);
//!
//!     let response = client.chat(request).await?;
//!     println!("Response: {}", response.message.text().unwrap());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Remote Provider with Thinking Model (Deepseek R1)
//!
//! ```rust,ignore
//! use llm::remote::DeepseekClient;
//! use llm::config::RemoteLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest, ReasoningMode};
//! use langgraph_core::Message;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = RemoteLlmConfig::from_env(
//!         "DEEPSEEK_API_KEY",
//!         "https://api.deepseek.com",
//!         "deepseek-reasoner"
//!     )?;
//!     let client = DeepseekClient::new(config);
//!
//!     let request = ChatRequest::new(vec![
//!         Message::human("Solve this logic puzzle: ...")
//!     ]).with_reasoning(ReasoningMode::Separated);
//!
//!     let response = client.chat(request).await?;
//!     
//!     // Access the thinking process
//!     if let Some(reasoning) = response.reasoning {
//!         println!("Model's thinking: {}", reasoning.content);
//!     }
//!     
//!     // Access the final answer
//!     println!("Final answer: {}", response.message.text().unwrap());
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod provider_utils;

#[macro_use]
mod provider_macros;

#[cfg(feature = "local")]
pub mod local;

#[cfg(feature = "remote")]
pub mod remote;

// Re-export commonly used types
pub use config::{LocalLlmConfig, RemoteLlmConfig};
pub use error::{LlmError, Result};
pub use provider_utils::{ModelInfo, ProviderUtils};

// Re-export langgraph-core types for convenience
pub use langgraph_core::llm::{
    ChatConfig, ChatModel, ChatRequest, ChatResponse, ChatStreamResponse, ReasoningContent,
    ReasoningMode, ToolCall, ToolDefinition, ToolResult, UsageMetadata,
};
pub use langgraph_core::Message;

