//! Task Execution Module
//!
//! This module provides task execution capabilities, including LLM-based execution,
//! retry logic, streaming, configuration, and response parsing.

pub mod config;
pub mod llm_executor;
pub mod parser;
pub mod retry;
pub mod streaming;

pub use config::ExecutorConfig;
pub use llm_executor::LlmTaskExecutor;
pub use parser::{ParsedResult, ResponseParser};
pub use retry::{classify_error, retry_with_backoff, ErrorClass, RetryConfig};
pub use streaming::{
    execute_with_updates, handle_stream_interruption, StreamBuilder, TaskUpdate,
    TaskUpdateSender, TaskUpdateStream, UpdateType,
};
