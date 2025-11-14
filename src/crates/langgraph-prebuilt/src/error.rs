//! Error Types - Prebuilt Component Errors
//!
//! This module defines error types that can occur when using prebuilt agent patterns,
//! tools, and message handling.
//!
//! # Error Categories
//!
//! - **Tool Errors** - Tool execution, validation, and I/O errors
//! - **Message Errors** - Message parsing and serialization errors
//! - **Graph Errors** - Underlying graph execution errors
//!
//! # Example
//!
//! ```rust,ignore
//! use langgraph_prebuilt::{PrebuiltError, Result};
//!
//! fn execute_tool() -> Result<String> {
//!     // Tool execution that might fail
//!     Err(PrebuiltError::ToolExecution("API timeout".to_string()))
//! }
//!
//! match execute_tool() {
//!     Ok(result) => println!("Success: {}", result),
//!     Err(PrebuiltError::ToolExecution(msg)) => eprintln!("Tool failed: {}", msg),
//!     Err(e) => eprintln!("Other error: {}", e),
//! }
//! ```

use thiserror::Error;

/// Result type for prebuilt operations
pub type Result<T> = std::result::Result<T, PrebuiltError>;

/// Errors that can occur in prebuilt components
#[derive(Error, Debug)]
pub enum PrebuiltError {
    /// Tool execution error
    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    /// Tool validation error
    #[error("Tool validation failed: {0}")]
    ToolValidation(String),

    /// Invalid tool input
    #[error("Invalid tool input: {0}")]
    InvalidInput(String),

    /// Invalid tool output
    #[error("Invalid tool output: {0}")]
    InvalidOutput(String),

    /// Message parsing error
    #[error("Message parsing failed: {0}")]
    MessageParsing(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Graph error
    #[error("Graph error: {0}")]
    Graph(#[from] langgraph_core::GraphError),

    /// Custom error
    #[error("{0}")]
    Custom(String),
}
