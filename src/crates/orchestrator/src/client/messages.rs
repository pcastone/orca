//! WebSocket message types for orchestrator-aco communication.
//!
//! These types wrap the tooling crate's ToolRequest and ToolResponse
//! for WebSocket transport with additional metadata.

use serde::{Deserialize, Serialize};
use tooling::runtime::{ToolRequest, ToolResponse};

/// WebSocket message envelope for all communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Tool execution request
    ToolRequest(ToolRequest),
    /// Tool execution response
    ToolResponse(ToolResponse),
    /// Session initialization
    SessionInit(SessionInit),
    /// Session acknowledgment
    SessionAck(SessionAck),
    /// Heartbeat/keepalive
    Heartbeat(Heartbeat),
    /// Error message
    Error(ErrorMessage),
}

/// Session initialization message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInit {
    /// Session ID
    pub session_id: String,
    /// Workspace root path
    pub workspace_root: Option<String>,
    /// Client capabilities
    pub capabilities: Vec<String>,
}

/// Session acknowledgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAck {
    /// Session ID
    pub session_id: String,
    /// Server capabilities
    pub capabilities: Vec<String>,
    /// Server version
    pub version: String,
}

/// Heartbeat message for keepalive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Timestamp (milliseconds)
    pub timestamp: i64,
}

impl Heartbeat {
    /// Create a new heartbeat
    pub fn new() -> Self {
        use chrono::Utc;
        Self {
            timestamp: Utc::now().timestamp_millis(),
        }
    }
}

impl Default for Heartbeat {
    fn default() -> Self {
        Self::new()
    }
}

/// Error message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Optional request ID for correlation
    pub request_id: Option<String>,
}

impl ErrorMessage {
    /// Create a new error message
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            request_id: None,
        }
    }

    /// Set request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_ws_message_serialization() {
        let heartbeat = WsMessage::Heartbeat(Heartbeat::new());
        let json = serde_json::to_string(&heartbeat).unwrap();
        assert!(json.contains("heartbeat"));
    }

    #[test]
    fn test_session_init() {
        let init = SessionInit {
            session_id: "session-123".to_string(),
            workspace_root: Some("/workspace".to_string()),
            capabilities: vec!["file_read".to_string(), "git_status".to_string()],
        };
        let json = serde_json::to_string(&init).unwrap();
        assert!(json.contains("session-123"));
    }
}

