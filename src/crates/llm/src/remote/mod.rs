//! Remote LLM provider implementations.
//!
//! This module contains implementations for cloud-hosted LLM APIs.
//! These providers require API keys and offer:
//! - Access to powerful models (GPT-4, Claude 3, Gemini, etc.)
//! - No local hardware requirements
//! - Managed infrastructure and scaling
//! - Regular model updates
//!
//! # Providers
//!
//! - **Claude** - Anthropic's Claude models (Claude 3 Opus, Sonnet, Haiku)
//! - **OpenAI** - OpenAI models (GPT-4, GPT-3.5, o1)
//! - **Gemini** - Google's Gemini models (Gemini Pro, Gemini Pro Vision)
//! - **Grok** - xAI's Grok models
//! - **Deepseek** - Deepseek models including R1 (thinking model)
//! - **OpenRouter** - Unified API for multiple providers

pub mod claude;
pub mod openai;
pub mod gemini;
pub mod grok;
pub mod deepseek;
pub mod openrouter;

pub use claude::ClaudeClient;
pub use openai::OpenAiClient;
pub use gemini::GeminiClient;
pub use grok::GrokClient;
pub use deepseek::DeepseekClient;
pub use openrouter::OpenRouterClient;

