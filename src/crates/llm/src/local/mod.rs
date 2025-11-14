//! Local LLM provider implementations.
//!
//! This module contains implementations for LLMs running on localhost or
//! local networks. These providers don't require API keys and offer:
//! - Lower latency
//! - Better privacy (data stays local)
//! - No API costs
//! - Offline operation
//!
//! # Providers
//!
//! - **Ollama** - Popular local LLM runner with wide model support
//! - **llama.cpp** - Direct llama.cpp server integration
//! - **LM Studio** - User-friendly local LLM interface

pub mod ollama;
pub mod llama_cpp;
pub mod lmstudio;

pub use ollama::OllamaClient;
pub use llama_cpp::LlamaCppClient;
pub use lmstudio::LmStudioClient;

