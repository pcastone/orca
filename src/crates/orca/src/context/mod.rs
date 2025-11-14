//! Execution context management
//!
//! Provides unified access to resources during workflow and task execution.
//!
//! # Components
//!
//! - **ExecutionContext** - Main context struct with database, tools, LLM, and config
//! - **SessionInfo** - Session tracking information
//! - **ContextBuilder** - Fluent builder for creating contexts

mod execution_context;
mod session_info;

pub use execution_context::{ExecutionContext, ContextBuilder};
pub use session_info::SessionInfo;
