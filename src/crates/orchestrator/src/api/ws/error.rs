//! WebSocket error handling
//!
//! Provides error types and handling strategies for WebSocket operations.

use std::fmt;

/// WebSocket error types
#[derive(Debug, Clone)]
pub enum WsError {
    /// Connection error (transient)
    ConnectionError(String),
    /// Protocol error (permanent)
    ProtocolError(String),
    /// Rate limit exceeded (transient)
    RateLimitExceeded { client_id: String, retry_after_secs: u64 },
    /// Backpressure detected (transient)
    BackpressureFull { client_id: String, queue_size: usize },
    /// Invalid message format
    InvalidMessage(String),
    /// Client disconnected
    ClientDisconnected(String),
    /// Server error
    ServerError(String),
    /// Timeout (transient)
    Timeout { client_id: String, timeout_secs: u64 },
    /// Unknown error
    Unknown(String),
}

impl fmt::Display for WsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WsError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            WsError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            WsError::RateLimitExceeded { client_id, retry_after_secs } => {
                write!(f, "Rate limit exceeded for {}, retry after {} seconds",
                       client_id, retry_after_secs)
            }
            WsError::BackpressureFull { client_id, queue_size } => {
                write!(f, "Backpressure queue full for {}, queue size: {}",
                       client_id, queue_size)
            }
            WsError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
            WsError::ClientDisconnected(client_id) => write!(f, "Client {} disconnected", client_id),
            WsError::ServerError(msg) => write!(f, "Server error: {}", msg),
            WsError::Timeout { client_id, timeout_secs } => {
                write!(f, "Timeout for {} after {} seconds", client_id, timeout_secs)
            }
            WsError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for WsError {}

impl WsError {
    /// Check if error is transient (can retry)
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            WsError::ConnectionError(_)
                | WsError::RateLimitExceeded { .. }
                | WsError::BackpressureFull { .. }
                | WsError::Timeout { .. }
        )
    }

    /// Check if error is permanent (should not retry)
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            WsError::ProtocolError(_)
                | WsError::InvalidMessage(_)
                | WsError::ClientDisconnected(_)
        )
    }

    /// Get recommended retry delay in seconds
    pub fn retry_delay_secs(&self) -> Option<u64> {
        match self {
            WsError::RateLimitExceeded { retry_after_secs, .. } => Some(*retry_after_secs),
            WsError::BackpressureFull { .. } => Some(1),
            WsError::Timeout { .. } => Some(2),
            _ => None,
        }
    }

    /// Convert to client-friendly error response
    pub fn to_response(&self) -> serde_json::Value {
        match self {
            WsError::ConnectionError(msg) => serde_json::json!({
                "error": "connection_error",
                "message": msg,
                "transient": true,
            }),
            WsError::ProtocolError(msg) => serde_json::json!({
                "error": "protocol_error",
                "message": msg,
                "transient": false,
            }),
            WsError::RateLimitExceeded { retry_after_secs, .. } => serde_json::json!({
                "error": "rate_limit_exceeded",
                "message": "Too many messages",
                "retry_after": retry_after_secs,
                "transient": true,
            }),
            WsError::BackpressureFull { queue_size, .. } => serde_json::json!({
                "error": "backpressure_full",
                "message": "Server queue is full, try again",
                "queue_size": queue_size,
                "transient": true,
            }),
            WsError::InvalidMessage(msg) => serde_json::json!({
                "error": "invalid_message",
                "message": msg,
                "transient": false,
            }),
            WsError::ClientDisconnected(id) => serde_json::json!({
                "error": "client_disconnected",
                "message": format!("Client {} was disconnected", id),
                "transient": false,
            }),
            WsError::ServerError(msg) => serde_json::json!({
                "error": "server_error",
                "message": msg,
                "transient": false,
            }),
            WsError::Timeout { timeout_secs, .. } => serde_json::json!({
                "error": "timeout",
                "message": format!("Request timeout after {} seconds", timeout_secs),
                "transient": true,
            }),
            WsError::Unknown(msg) => serde_json::json!({
                "error": "unknown_error",
                "message": msg,
                "transient": false,
            }),
        }
    }
}

/// Result type for WebSocket operations
pub type WsResult<T> = Result<T, WsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transient_errors() {
        let err = WsError::ConnectionError("test".to_string());
        assert!(err.is_transient());
        assert!(!err.is_permanent());
    }

    #[test]
    fn test_permanent_errors() {
        let err = WsError::ProtocolError("test".to_string());
        assert!(err.is_permanent());
        assert!(!err.is_transient());
    }

    #[test]
    fn test_retry_delay() {
        let err = WsError::RateLimitExceeded {
            client_id: "c1".to_string(),
            retry_after_secs: 5,
        };
        assert_eq!(err.retry_delay_secs(), Some(5));
    }

    #[test]
    fn test_error_response() {
        let err = WsError::RateLimitExceeded {
            client_id: "c1".to_string(),
            retry_after_secs: 5,
        };
        let resp = err.to_response();
        assert_eq!(resp["error"], "rate_limit_exceeded");
        assert_eq!(resp["transient"], true);
    }

    #[test]
    fn test_display() {
        let err = WsError::ConnectionError("test".to_string());
        assert!(err.to_string().contains("Connection error"));
    }
}
