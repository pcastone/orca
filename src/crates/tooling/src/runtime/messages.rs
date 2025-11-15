//! Message types for Tool Runtime SDK
//!
//! Defines the core message types for tool request/response communication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Tool request message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolRequest {
    /// Message type identifier
    #[serde(rename = "type")]
    pub type_: String,

    /// Tool name to execute
    pub tool: String,

    /// Tool arguments as JSON value
    pub args: serde_json::Value,

    /// Unique request identifier
    pub request_id: String,

    /// Session identifier
    pub session_id: String,

    /// Request timestamp (Unix timestamp)
    #[serde(default = "default_timestamp")]
    pub timestamp: u64,
}

impl ToolRequest {
    /// Create a new tool request
    pub fn new(
        tool: impl Into<String>,
        args: serde_json::Value,
        request_id: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            type_: "ToolRequest".to_string(),
            tool: tool.into(),
            args,
            request_id: request_id.into(),
            session_id: session_id.into(),
            timestamp: current_timestamp(),
        }
    }
}

/// Tool response message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResponse {
    /// Message type identifier
    #[serde(rename = "type")]
    pub type_: String,

    /// Success indicator
    pub ok: bool,

    /// Tool name that was executed
    pub tool: String,

    /// Request identifier this response corresponds to
    pub request_id: String,

    /// Execution duration in milliseconds
    pub duration_ms: u64,

    /// Response data (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,

    /// Error messages (if failed)
    #[serde(default)]
    pub errors: Vec<String>,

    /// Warning messages
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,

    /// Response timestamp
    #[serde(default = "current_system_time")]
    pub timestamp: SystemTime,
}

impl ToolResponse {
    /// Create a successful response
    pub fn success(
        tool: impl Into<String>,
        request_id: impl Into<String>,
        duration_ms: u64,
        data: serde_json::Value,
    ) -> Self {
        Self {
            type_: "ToolResponse".to_string(),
            ok: true,
            tool: tool.into(),
            request_id: request_id.into(),
            duration_ms,
            data: Some(data),
            errors: Vec::new(),
            warnings: Vec::new(),
            timestamp: SystemTime::now(),
        }
    }

    /// Create an error response
    pub fn error(
        tool: impl Into<String>,
        request_id: impl Into<String>,
        duration_ms: u64,
        errors: Vec<String>,
    ) -> Self {
        Self {
            type_: "ToolResponse".to_string(),
            ok: false,
            tool: tool.into(),
            request_id: request_id.into(),
            duration_ms,
            data: None,
            errors,
            warnings: Vec::new(),
            timestamp: SystemTime::now(),
        }
    }
}

/// Event message for progress updates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventMessage {
    /// Message type identifier
    #[serde(rename = "type")]
    pub type_: String,

    /// Event type
    pub event: String,

    /// Request identifier this event relates to
    pub request_id: String,

    /// Progress information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressInfo>,
}

impl EventMessage {
    /// Create a new event message
    pub fn new(
        event: impl Into<String>,
        request_id: impl Into<String>,
        progress: Option<ProgressInfo>,
    ) -> Self {
        Self {
            type_: "EventMessage".to_string(),
            event: event.into(),
            request_id: request_id.into(),
            progress,
        }
    }
}

/// Progress information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgressInfo {
    /// Percentage complete (0-100)
    pub pct: u8,

    /// Human-readable progress message
    pub message: String,
}

impl ProgressInfo {
    /// Create new progress info
    pub fn new(pct: u8, message: impl Into<String>) -> Self {
        Self {
            pct: pct.min(100),
            message: message.into(),
        }
    }
}

/// Error message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorMessage {
    /// Message type identifier
    #[serde(rename = "type")]
    pub type_: String,

    /// Error code
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Request identifier this error relates to
    pub request_id: String,

    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, serde_json::Value>>,
}

impl ErrorMessage {
    /// Create a new error message
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Self {
        Self {
            type_: "ErrorMessage".to_string(),
            code: code.into(),
            message: message.into(),
            request_id: request_id.into(),
            details: None,
        }
    }

    /// Add error details
    pub fn with_details(mut self, details: HashMap<String, serde_json::Value>) -> Self {
        self.details = Some(details);
        self
    }
}

/// Heartbeat message for session keepalive
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Heartbeat {
    /// Message type identifier
    #[serde(rename = "type")]
    pub type_: String,

    /// Session identifier
    pub session_id: String,

    /// Heartbeat timestamp (Unix timestamp)
    pub timestamp: u64,
}

impl Heartbeat {
    /// Create a new heartbeat message
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            type_: "Heartbeat".to_string(),
            session_id: session_id.into(),
            timestamp: current_timestamp(),
        }
    }
}

/// Session acknowledgment message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionAck {
    /// Message type identifier
    #[serde(rename = "type")]
    pub type_: String,

    /// Session identifier
    pub session_id: String,

    /// Whether the session was accepted
    pub accepted: bool,

    /// Server version information
    pub server_version: String,

    /// Session timestamp
    pub timestamp: u64,
}

impl SessionAck {
    /// Create a new session acknowledgment
    pub fn new(
        session_id: impl Into<String>,
        accepted: bool,
        server_version: impl Into<String>,
    ) -> Self {
        Self {
            type_: "SessionAck".to_string(),
            session_id: session_id.into(),
            accepted,
            server_version: server_version.into(),
            timestamp: current_timestamp(),
        }
    }
}

/// Canonical error codes for tool runtime
pub mod error_codes {
    pub const FILE_IO: &str = "E_FILE_IO";
    pub const AST_PARSE: &str = "E_AST_PARSE";
    pub const AST_EDIT: &str = "E_AST_EDIT";
    pub const VALIDATION_FAIL: &str = "E_VALIDATION_FAIL";
    pub const GIT: &str = "E_GIT";
    pub const HTTP: &str = "E_HTTP";
    pub const SHELL: &str = "E_SHELL";
    pub const POLICY: &str = "E_POLICY";
    pub const TIMEOUT: &str = "E_TIMEOUT";
    pub const INTERNAL: &str = "E_INTERNAL";
}

// Helper functions
fn default_timestamp() -> u64 {
    current_timestamp()
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn current_system_time() -> SystemTime {
    SystemTime::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_request_creation() {
        let req = ToolRequest::new(
            "file_read",
            serde_json::json!({"path": "src/main.rs"}),
            "req-123",
            "sess-456",
        );

        assert_eq!(req.type_, "ToolRequest");
        assert_eq!(req.tool, "file_read");
        assert_eq!(req.request_id, "req-123");
        assert_eq!(req.session_id, "sess-456");
    }

    #[test]
    fn test_tool_response_success() {
        let resp = ToolResponse::success(
            "file_read",
            "req-123",
            50,
            serde_json::json!({"content": "test"}),
        );

        assert!(resp.ok);
        assert_eq!(resp.tool, "file_read");
        assert_eq!(resp.duration_ms, 50);
        assert!(resp.data.is_some());
        assert!(resp.errors.is_empty());
    }

    #[test]
    fn test_tool_response_error() {
        let resp = ToolResponse::error(
            "file_read",
            "req-123",
            10,
            vec!["File not found".to_string()],
        );

        assert!(!resp.ok);
        assert_eq!(resp.errors.len(), 1);
        assert!(resp.data.is_none());
    }

    #[test]
    fn test_event_message() {
        let progress = ProgressInfo::new(65, "running tests");
        let event = EventMessage::new("TaskProgress", "req-123", Some(progress));

        assert_eq!(event.event, "TaskProgress");
        assert!(event.progress.is_some());
        assert_eq!(event.progress.unwrap().pct, 65);
    }

    #[test]
    fn test_error_message() {
        let err = ErrorMessage::new("E_FILE_IO", "File not found", "req-123");

        assert_eq!(err.code, "E_FILE_IO");
        assert_eq!(err.message, "File not found");
        assert!(err.details.is_none());
    }

    #[test]
    fn test_heartbeat() {
        let hb = Heartbeat::new("sess-456");

        assert_eq!(hb.type_, "Heartbeat");
        assert_eq!(hb.session_id, "sess-456");
        assert!(hb.timestamp > 0);
    }

    #[test]
    fn test_session_ack() {
        let ack = SessionAck::new("sess-456", true, "1.0.0");

        assert_eq!(ack.session_id, "sess-456");
        assert!(ack.accepted);
        assert_eq!(ack.server_version, "1.0.0");
    }

    #[test]
    fn test_progress_info_clamps_percentage() {
        let progress = ProgressInfo::new(150, "test");
        assert_eq!(progress.pct, 100);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let req = ToolRequest::new(
            "test_tool",
            serde_json::json!({"key": "value"}),
            "req-1",
            "sess-1",
        );

        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ToolRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(req, deserialized);
    }
}
