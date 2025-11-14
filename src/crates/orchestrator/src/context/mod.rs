//! Context window management for LLM conversations
//!
//! This module provides tools for managing LLM context windows including
//! token counting, message truncation, and priority-based retention.

pub mod manager;
pub mod token_counter;
pub mod trimmer;

pub use manager::ContextManager;
pub use token_counter::{TokenCounter, TokenCount};
pub use trimmer::{ContextTrimmer, TrimStrategy, MessagePriority};
