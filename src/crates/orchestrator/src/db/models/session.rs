//! Session model for database persistence
//!
//! Represents WebSocket connection sessions for real-time communication.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a client session (WebSocket connection)
///
/// Sessions track connected clients for real-time communication.
/// Each session maintains a heartbeat to detect disconnected clients.
///
/// # Timestamps
/// All timestamp fields are ISO8601 strings due to SQLite type limitations.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    /// Unique session identifier (UUID string)
    pub id: String,

    /// Client identifier (typically from the client application)
    pub client_id: String,

    /// Optional user identifier (when authenticated)
    pub user_id: Option<String>,

    /// Connection type: websocket, http, grpc
    pub connection_type: String,

    /// Whether the session is currently active (0 = inactive, 1 = active)
    pub is_active: i32,

    /// Session metadata as JSON string (optional)
    pub metadata: Option<String>,

    /// Session creation timestamp (ISO8601 string)
    pub created_at: String,

    /// Session last update timestamp (ISO8601 string)
    pub updated_at: String,

    /// Last heartbeat timestamp (ISO8601 string)
    pub last_heartbeat: String,
}

impl Session {
    /// Create a new session
    ///
    /// # Arguments
    /// * `id` - Unique session identifier
    /// * `client_id` - Client identifier
    /// * `connection_type` - Type of connection (websocket, http, grpc)
    ///
    /// # Returns
    /// A new Session with active status and current timestamp
    pub fn new(id: String, client_id: String, connection_type: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            client_id,
            user_id: None,
            connection_type,
            is_active: 1,
            metadata: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_heartbeat: now,
        }
    }

    /// Builder method to set user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Builder method to set session metadata
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }

    /// Update the last heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = chrono::Utc::now().to_rfc3339();
        self.updated_at = self.last_heartbeat.clone();
    }

    /// Deactivate the session
    pub fn deactivate(&mut self) {
        self.is_active = 0;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Activate the session
    pub fn activate(&mut self) {
        self.is_active = 1;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Check if session is active
    pub fn is_active(&self) -> bool {
        self.is_active == 1
    }

    /// Check if session is inactive
    pub fn is_inactive(&self) -> bool {
        self.is_active == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new(
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        );

        assert_eq!(session.id, "session-1");
        assert_eq!(session.client_id, "client-1");
        assert_eq!(session.connection_type, "websocket");
        assert!(session.is_active());
        assert!(!session.is_inactive());
    }

    #[test]
    fn test_session_with_user_id() {
        let session = Session::new(
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        )
        .with_user_id("user-1");

        assert_eq!(session.user_id, Some("user-1".to_string()));
    }

    #[test]
    fn test_session_with_metadata() {
        let session = Session::new(
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        )
        .with_metadata(r#"{"browser": "Chrome"}"#);

        assert_eq!(session.metadata, Some(r#"{"browser": "Chrome"}"#.to_string()));
    }

    #[test]
    fn test_session_deactivate() {
        let mut session = Session::new(
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        );

        assert!(session.is_active());
        session.deactivate();
        assert!(!session.is_active());
        assert!(session.is_inactive());
    }

    #[test]
    fn test_session_activate() {
        let mut session = Session::new(
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        );

        session.is_active = 0;
        assert!(!session.is_active());

        session.activate();
        assert!(session.is_active());
    }

    #[test]
    fn test_session_update_heartbeat() {
        let mut session = Session::new(
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        );

        let old_heartbeat = session.last_heartbeat.clone();
        std::thread::sleep(std::time::Duration::from_millis(10));
        session.update_heartbeat();

        assert!(session.last_heartbeat > old_heartbeat);
        assert_eq!(session.last_heartbeat, session.updated_at);
    }
}
