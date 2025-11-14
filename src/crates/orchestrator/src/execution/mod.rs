//! Task Execution Module
//!
//! Provides task execution capabilities with LLM integration,
//! state management, and streaming support.

pub mod task_engine;
pub mod streaming;

pub use task_engine::TaskExecutionEngine;
pub use streaming::{ExecutionStreamHandler, ExecutionEventType, ExecutionEventBuilder};
