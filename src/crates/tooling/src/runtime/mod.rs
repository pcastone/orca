//! Runtime types for tool execution
//!
//! This module provides types for tool requests and responses used in
//! communication between components.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A request to execute a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    /// Tool name/identifier
    pub tool: String,

    /// Tool arguments
    pub args: HashMap<String, serde_json::Value>,

    /// Request ID for tracking
    pub request_id: Option<String>,

    /// Session ID for context
    pub session_id: Option<String>,

    /// Request metadata
    pub metadata: HashMap<String, String>,
}

impl ToolRequest {
    /// Create a new tool request
    pub fn new(tool: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            args: HashMap::new(),
            request_id: None,
            session_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Add an argument to the request
    pub fn with_arg(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.args.insert(key.into(), value);
        self
    }

    /// Set the session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Response from tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    /// Tool that was executed
    pub tool: String,

    /// Request ID (for correlation with request)
    pub request_id: Option<String>,

    /// Execution status
    pub status: ToolStatus,

    /// Success flag (true if tool executed successfully)
    pub ok: bool,

    /// Result data (if successful)
    pub result: Option<serde_json::Value>,

    /// Data field (alias for result for compatibility)
    pub data: Option<serde_json::Value>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Error messages (list format for compatibility)
    pub errors: Vec<String>,

    /// Warnings generated during execution
    pub warnings: Vec<String>,

    /// Execution duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Response metadata
    pub metadata: HashMap<String, String>,
}

impl ToolResponse {
    /// Create a successful response
    pub fn success(tool: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            tool: tool.into(),
            request_id: None,
            status: ToolStatus::Success,
            ok: true,
            result: Some(result.clone()),
            data: Some(result),
            error: None,
            errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an error response
    pub fn error(tool: impl Into<String>, error: impl Into<String>) -> Self {
        let error_str = error.into();
        Self {
            tool: tool.into(),
            request_id: None,
            status: ToolStatus::Error,
            ok: false,
            result: None,
            data: None,
            error: Some(error_str.clone()),
            errors: vec![error_str],
            warnings: Vec::new(),
            duration_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Tool execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    /// Tool executed successfully
    Success,

    /// Tool execution failed
    Error,

    /// Tool execution timed out
    Timeout,

    /// Tool not found
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_request_builder() {
        let request = ToolRequest::new("test_tool")
            .with_arg("arg1", json!("value1"))
            .with_session_id("session123")
            .with_metadata("key1", "meta1");

        assert_eq!(request.tool, "test_tool");
        assert_eq!(request.args.len(), 1);
        assert_eq!(request.session_id, Some("session123".to_string()));
        assert_eq!(request.metadata.get("key1"), Some(&"meta1".to_string()));
    }

    #[test]
    fn test_tool_response_success() {
        let response = ToolResponse::success("test_tool", json!({"result": "data"}));

        assert_eq!(response.tool, "test_tool");
        assert_eq!(response.status, ToolStatus::Success);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_tool_response_error() {
        let response = ToolResponse::error("test_tool", "Test error");

        assert_eq!(response.tool, "test_tool");
        assert_eq!(response.status, ToolStatus::Error);
        assert!(response.result.is_none());
        assert_eq!(response.error, Some("Test error".to_string()));
    }
}
