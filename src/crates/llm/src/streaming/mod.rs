//! Streaming utilities for LLM providers
//!
//! This module provides common streaming infrastructure for parsing
//! Server-Sent Events (SSE) and NDJSON streams from various LLM APIs.
//!
//! ## Supported Formats
//!
//! - **OpenAI-compatible** - SSE with OpenAI JSON structure (OpenAI, Deepseek, Grok, OpenRouter, LM Studio, llama.cpp)
//! - **Claude (Anthropic)** - SSE with custom event types (message_start, content_block_delta, etc.)
//! - **Gemini (Google)** - SSE with Google's JSON structure
//! - **Ollama** - NDJSON (newline-delimited JSON)

pub mod claude;
pub mod gemini;
pub mod ollama;
pub mod openai_compat;

pub use claude::stream_claude;
pub use gemini::stream_gemini;
pub use ollama::stream_ollama;
pub use openai_compat::stream_openai_compatible;
