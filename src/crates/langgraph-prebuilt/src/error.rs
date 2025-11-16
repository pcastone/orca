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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_execution_error_display() {
        let err = PrebuiltError::ToolExecution("API timeout".to_string());
        assert_eq!(err.to_string(), "Tool execution failed: API timeout");
    }

    #[test]
    fn test_tool_validation_error_display() {
        let err = PrebuiltError::ToolValidation("missing required field".to_string());
        assert_eq!(
            err.to_string(),
            "Tool validation failed: missing required field"
        );
    }

    #[test]
    fn test_invalid_input_error_display() {
        let err = PrebuiltError::InvalidInput("expected number, got string".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid tool input: expected number, got string"
        );
    }

    #[test]
    fn test_invalid_output_error_display() {
        let err = PrebuiltError::InvalidOutput("malformed JSON".to_string());
        assert_eq!(err.to_string(), "Invalid tool output: malformed JSON");
    }

    #[test]
    fn test_message_parsing_error_display() {
        let err = PrebuiltError::MessageParsing("unknown message type".to_string());
        assert_eq!(
            err.to_string(),
            "Message parsing failed: unknown message type"
        );
    }

    #[test]
    fn test_custom_error_display() {
        let err = PrebuiltError::Custom("custom error message".to_string());
        assert_eq!(err.to_string(), "custom error message");
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let prebuilt_err: PrebuiltError = json_err.into();

        match prebuilt_err {
            PrebuiltError::Serialization(_) => {}
            _ => panic!("Expected Serialization error"),
        }
    }

    #[test]
    fn test_serialization_error_conversion() {
        let json = "{invalid json}";
        let result: std::result::Result<serde_json::Value, serde_json::Error> =
            serde_json::from_str(json);

        match result {
            Err(e) => {
                let prebuilt_error: PrebuiltError = e.into();
                assert!(matches!(prebuilt_error, PrebuiltError::Serialization(_)));
                assert!(prebuilt_error.to_string().contains("Serialization error"));
            }
            Ok(_) => panic!("Expected JSON parsing to fail"),
        }
    }

    #[test]
    fn test_result_type_usage() {
        fn returns_ok() -> Result<String> {
            Ok("success".to_string())
        }

        fn returns_error() -> Result<String> {
            Err(PrebuiltError::ToolExecution("failed".to_string()))
        }

        assert!(returns_ok().is_ok());
        assert!(returns_error().is_err());
    }

    #[test]
    fn test_error_variants_can_be_constructed() {
        let _tool_exec = PrebuiltError::ToolExecution("test".to_string());
        let _tool_val = PrebuiltError::ToolValidation("test".to_string());
        let _invalid_in = PrebuiltError::InvalidInput("test".to_string());
        let _invalid_out = PrebuiltError::InvalidOutput("test".to_string());
        let _msg_parse = PrebuiltError::MessageParsing("test".to_string());
        let _custom = PrebuiltError::Custom("test".to_string());
    }

    #[test]
    fn test_error_debug_format() {
        let err = PrebuiltError::ToolExecution("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("ToolExecution"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_error_type_matching() {
        let errors = vec![
            PrebuiltError::ToolExecution("test".to_string()),
            PrebuiltError::ToolValidation("test".to_string()),
            PrebuiltError::InvalidInput("test".to_string()),
            PrebuiltError::InvalidOutput("test".to_string()),
            PrebuiltError::MessageParsing("test".to_string()),
            PrebuiltError::Custom("test".to_string()),
        ];

        for err in errors {
            match &err {
                PrebuiltError::ToolExecution(_) => assert!(matches!(
                    err,
                    PrebuiltError::ToolExecution(_)
                )),
                PrebuiltError::ToolValidation(_) => assert!(matches!(
                    err,
                    PrebuiltError::ToolValidation(_)
                )),
                PrebuiltError::InvalidInput(_) => assert!(matches!(
                    err,
                    PrebuiltError::InvalidInput(_)
                )),
                PrebuiltError::InvalidOutput(_) => assert!(matches!(
                    err,
                    PrebuiltError::InvalidOutput(_)
                )),
                PrebuiltError::MessageParsing(_) => assert!(matches!(
                    err,
                    PrebuiltError::MessageParsing(_)
                )),
                PrebuiltError::Custom(_) => assert!(matches!(err, PrebuiltError::Custom(_))),
                _ => {}
            }
        }
    }

    #[test]
    fn test_error_message_preservation() {
        let original_msg = "connection refused on port 8080";
        let err = PrebuiltError::ToolExecution(original_msg.to_string());
        let err_string = err.to_string();

        assert!(err_string.contains(original_msg));
        assert!(err_string.contains("Tool execution failed"));
    }

    #[test]
    fn test_empty_error_messages() {
        let err = PrebuiltError::Custom("".to_string());
        assert_eq!(err.to_string(), "");

        let err2 = PrebuiltError::ToolExecution("".to_string());
        assert_eq!(err2.to_string(), "Tool execution failed: ");
    }

    #[test]
    fn test_multiline_error_messages() {
        let multiline_msg = "Line 1\nLine 2\nLine 3";
        let err = PrebuiltError::ToolValidation(multiline_msg.to_string());

        assert!(err.to_string().contains("Line 1"));
        assert!(err.to_string().contains("Line 2"));
        assert!(err.to_string().contains("Line 3"));
    }

    #[test]
    fn test_special_characters_in_errors() {
        let special_msg = "Error: <tag> & \"quotes\" & 'apostrophes'";
        let err = PrebuiltError::InvalidInput(special_msg.to_string());

        let err_string = err.to_string();
        assert!(err_string.contains("<tag>"));
        assert!(err_string.contains("\"quotes\""));
        assert!(err_string.contains("'apostrophes'"));
    }

    #[test]
    fn test_tool_execution_error_context() {
        let err = PrebuiltError::ToolExecution("failed to execute calculator".to_string());

        match err {
            PrebuiltError::ToolExecution(msg) => {
                assert_eq!(msg, "failed to execute calculator");
            }
            _ => panic!("Expected ToolExecution variant"),
        }
    }

    #[test]
    fn test_tool_validation_error_context() {
        let err = PrebuiltError::ToolValidation("schema mismatch".to_string());

        match err {
            PrebuiltError::ToolValidation(msg) => {
                assert_eq!(msg, "schema mismatch");
            }
            _ => panic!("Expected ToolValidation variant"),
        }
    }

    #[test]
    fn test_invalid_input_error_context() {
        let err = PrebuiltError::InvalidInput("type error".to_string());

        match err {
            PrebuiltError::InvalidInput(msg) => {
                assert_eq!(msg, "type error");
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    #[test]
    fn test_invalid_output_error_context() {
        let err = PrebuiltError::InvalidOutput("unexpected format".to_string());

        match err {
            PrebuiltError::InvalidOutput(msg) => {
                assert_eq!(msg, "unexpected format");
            }
            _ => panic!("Expected InvalidOutput variant"),
        }
    }

    #[test]
    fn test_message_parsing_error_context() {
        let err = PrebuiltError::MessageParsing("invalid structure".to_string());

        match err {
            PrebuiltError::MessageParsing(msg) => {
                assert_eq!(msg, "invalid structure");
            }
            _ => panic!("Expected MessageParsing variant"),
        }
    }
}
