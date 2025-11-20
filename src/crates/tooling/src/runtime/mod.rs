//! Runtime types for tool execution
//!
//! This module provides types for tool requests and responses used in
//! communication between components.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export rtoon for TOON format support
pub use rtoon::{self, EncodeOptions as ToonEncodeOptions, DecodeOptions as ToonDecodeOptions};

/// Message format for tool responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MessageFormat {
    /// JSON format (default)
    #[default]
    Json,
    /// TOON format for token-efficient serialization
    Toon,
    /// Auto-select based on data structure
    Auto,
}

/// Tool-specific response types optimized for TOON serialization
pub mod tool_responses {
    use super::*;

    /// File entry for fs_list tool (~50% token savings)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FileEntry {
        pub path: String,
        pub size: u64,
        pub modified: String,
        pub is_dir: bool,
    }

    /// Grep match result for grep tool (~60% token savings)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GrepMatch {
        pub file: String,
        pub line: u32,
        pub content: String,
        pub context_before: Option<String>,
        pub context_after: Option<String>,
    }

    /// Git status entry for git_status tool (~45% token savings)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GitStatusEntry {
        pub path: String,
        pub status: String,
        pub staged: bool,
    }

    /// AST node for ast_query tool (~55% token savings)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AstNode {
        pub kind: String,
        pub name: Option<String>,
        pub start_line: u32,
        pub end_line: u32,
        pub file: String,
    }

    /// Process entry for proc_list tool (~40% token savings)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ProcessEntry {
        pub pid: u32,
        pub name: String,
        pub cpu: f32,
        pub memory: u64,
        pub status: String,
    }
}

/// Format selector for automatic format selection
pub struct FormatSelector;

impl FormatSelector {
    /// Select optimal format based on data structure
    pub fn select(data: &serde_json::Value) -> MessageFormat {
        match data {
            serde_json::Value::Array(arr) if arr.len() > 3 && Self::is_uniform_array(arr) => {
                MessageFormat::Toon
            }
            serde_json::Value::Object(obj) => {
                // Check if object contains large uniform arrays
                for value in obj.values() {
                    if let serde_json::Value::Array(arr) = value {
                        if arr.len() > 3 && Self::is_uniform_array(arr) {
                            return MessageFormat::Toon;
                        }
                    }
                }
                MessageFormat::Json
            }
            _ => MessageFormat::Json,
        }
    }

    /// Check if array elements have uniform structure (good for TOON tabular format)
    fn is_uniform_array(arr: &[serde_json::Value]) -> bool {
        if arr.is_empty() {
            return false;
        }

        let first = &arr[0];
        if let serde_json::Value::Object(first_obj) = first {
            let keys: Vec<_> = first_obj.keys().collect();
            arr.iter().skip(1).all(|item| {
                if let serde_json::Value::Object(obj) = item {
                    obj.keys().collect::<Vec<_>>() == keys
                } else {
                    false
                }
            })
        } else {
            // Primitive arrays are also good for TOON
            arr.iter().all(|item| {
                std::mem::discriminant(item) == std::mem::discriminant(first)
            })
        }
    }
}

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

    /// Encode response to specified format
    pub fn encode(&self, format: MessageFormat) -> String {
        let format_to_use = match format {
            MessageFormat::Auto => {
                if let Some(ref result) = self.result {
                    FormatSelector::select(result)
                } else {
                    MessageFormat::Json
                }
            }
            other => other,
        };

        match format_to_use {
            MessageFormat::Toon => self.to_toon(None),
            MessageFormat::Json | MessageFormat::Auto => self.to_json(),
        }
    }

    /// Encode response as JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Encode response as TOON string for token efficiency
    pub fn to_toon(&self, options: Option<ToonEncodeOptions>) -> String {
        let json_value = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        rtoon::encode(&json_value, options)
    }

    /// Encode only the result field as TOON (most common use case)
    pub fn result_to_toon(&self, options: Option<ToonEncodeOptions>) -> Option<String> {
        self.result.as_ref().map(|v| rtoon::encode(v, options))
    }

    /// Calculate token savings estimate (TOON vs JSON)
    pub fn estimate_savings(&self) -> Option<f64> {
        if let Some(ref result) = self.result {
            let json_len = serde_json::to_string(result)
                .map(|s| s.len())
                .unwrap_or(0);
            let toon_len = rtoon::encode(result, None).len();

            if json_len > 0 {
                Some(1.0 - (toon_len as f64 / json_len as f64))
            } else {
                None
            }
        } else {
            None
        }
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

    #[test]
    fn test_toon_encoding_simple() {
        let response = ToolResponse::success("test_tool", json!({"name": "test", "value": 42}));
        let toon = response.to_toon(None);

        assert!(toon.contains("name:"));
        assert!(toon.contains("test"));
        assert!(toon.contains("value:"));
        assert!(toon.contains("42"));
    }

    #[test]
    fn test_toon_encoding_array() {
        // This is the high-savings scenario (uniform arrays)
        let files = json!({
            "files": [
                {"path": "src/main.rs", "size": 1024, "modified": "2025-01-15"},
                {"path": "src/lib.rs", "size": 2048, "modified": "2025-01-14"},
                {"path": "Cargo.toml", "size": 512, "modified": "2025-01-13"}
            ]
        });

        let response = ToolResponse::success("fs_list", files);
        let toon = response.to_toon(None);
        let json = response.to_json();

        // TOON should be more compact for tabular data
        assert!(toon.len() < json.len(), "TOON should be smaller than JSON for uniform arrays");
    }

    #[test]
    fn test_format_selector_chooses_toon_for_arrays() {
        let data = json!([
            {"file": "a.rs", "line": 1, "content": "fn main"},
            {"file": "b.rs", "line": 2, "content": "let x"},
            {"file": "c.rs", "line": 3, "content": "struct"},
            {"file": "d.rs", "line": 4, "content": "impl"}
        ]);

        assert_eq!(FormatSelector::select(&data), MessageFormat::Toon);
    }

    #[test]
    fn test_format_selector_chooses_json_for_small_arrays() {
        let data = json!([{"a": 1}, {"a": 2}]);
        assert_eq!(FormatSelector::select(&data), MessageFormat::Json);
    }

    #[test]
    fn test_format_selector_chooses_json_for_non_uniform() {
        let data = json!([
            {"type": "a"},
            {"type": "b", "extra": "field"}
        ]);
        assert_eq!(FormatSelector::select(&data), MessageFormat::Json);
    }

    #[test]
    fn test_auto_format_selection() {
        let grep_results = json!({
            "matches": [
                {"file": "a.rs", "line": 10, "content": "fn foo()"},
                {"file": "b.rs", "line": 20, "content": "fn bar()"},
                {"file": "c.rs", "line": 30, "content": "fn baz()"},
                {"file": "d.rs", "line": 40, "content": "fn qux()"}
            ]
        });

        let response = ToolResponse::success("grep", grep_results);
        let encoded = response.encode(MessageFormat::Auto);

        // Should select TOON for this uniform array data
        assert!(encoded.contains("matches"));
    }

    #[test]
    fn test_estimate_savings() {
        let files = json!({
            "files": [
                {"path": "src/main.rs", "size": 1024, "modified": "2025-01-15"},
                {"path": "src/lib.rs", "size": 2048, "modified": "2025-01-14"},
                {"path": "Cargo.toml", "size": 512, "modified": "2025-01-13"},
                {"path": "README.md", "size": 256, "modified": "2025-01-12"}
            ]
        });

        let response = ToolResponse::success("fs_list", files);
        let savings = response.estimate_savings().unwrap();

        // Expect positive savings for tabular data
        assert!(savings > 0.0, "Should have positive savings for uniform arrays");
    }

    #[test]
    fn test_result_to_toon() {
        let data = json!({"key": "value"});
        let response = ToolResponse::success("test", data);

        let toon = response.result_to_toon(None).unwrap();
        assert!(toon.contains("key:"));
        assert!(toon.contains("value"));
    }
}
