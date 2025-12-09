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
//! - **Claude Code** - Claude Code CLI (uses Claude Pro/Max subscription)

pub mod claude_code;
pub mod ollama;
pub mod llama_cpp;
pub mod lmstudio;

pub use claude_code::ClaudeCodeClient;
pub use ollama::OllamaClient;
pub use llama_cpp::LlamaCppClient;
pub use lmstudio::LmStudioClient;

