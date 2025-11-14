//! WebSocket handler for real-time updates
//!
//! Provides WebSocket endpoint for real-time event streaming and client communication.

use axum::extract::State;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::db::DatabaseConnection;

/// WebSocket event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsEvent {
    /// Connection established
    #[serde(rename = "connected")]
    Connected { client_id: String },

    /// Task status update
    #[serde(rename = "task_updated")]
    TaskUpdated { task_id: String, status: String },

    /// Workflow status update
    #[serde(rename = "workflow_updated")]
    WorkflowUpdated { workflow_id: String, status: String },

    /// Tool execution completed
    #[serde(rename = "execution_completed")]
    ExecutionCompleted {
        execution_id: String,
        status: String,
        output: Option<String>,
    },

    /// Error event
    #[serde(rename = "error")]
    Error { message: String },

    /// Keep-alive ping
    #[serde(rename = "ping")]
    Ping,

    /// Keep-alive pong response
    #[serde(rename = "pong")]
    Pong,
}

/// WebSocket broadcast state
#[derive(Clone)]
pub struct BroadcastState {
    /// Broadcast sender for events
    pub tx: broadcast::Sender<WsEvent>,
}

impl BroadcastState {
    /// Create a new broadcast state
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self { tx }
    }

    /// Broadcast an event to all subscribers
    pub async fn broadcast(&self, event: WsEvent) {
        let _ = self.tx.send(event);
    }
}

impl Default for BroadcastState {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler (placeholder for now - WebSocket requires additional feature)
///
/// GET /ws
pub async fn ws_handler(
    State(_db): State<DatabaseConnection>,
    State(_broadcast): State<Arc<BroadcastState>>,
) -> impl IntoResponse {
    // WebSocket support requires more setup with tokio-tungstenite
    // This is a placeholder that returns a 200 OK response
    axum::Json(serde_json::json!({
        "message": "WebSocket endpoint - upgrade required for full implementation"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_event_serialization() {
        let event = WsEvent::Ping;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ping"));
    }

    #[test]
    fn test_ws_event_task_updated() {
        let event = WsEvent::TaskUpdated {
            task_id: "task-1".to_string(),
            status: "completed".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("task_updated"));
        assert!(json.contains("task-1"));
        assert!(json.contains("completed"));
    }

    #[test]
    fn test_broadcast_state_creation() {
        let state = BroadcastState::new();
        assert_eq!(state.tx.receiver_count(), 0);
    }
}
