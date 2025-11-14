//! Session information tracking
//!
//! Tracks metadata about execution sessions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Information about an execution session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier
    pub session_id: String,

    /// Session creation timestamp
    pub created_at: DateTime<Utc>,

    /// Session description or name
    pub description: Option<String>,

    /// User identifier (if applicable)
    pub user_id: Option<String>,

    /// Additional session metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SessionInfo {
    /// Create a new session with generated ID
    pub fn new() -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            description: None,
            user_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a session with a specific ID
    pub fn with_id(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            created_at: Utc::now(),
            description: None,
            user_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Set session description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Add metadata entry
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get session age in seconds
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = SessionInfo::new();
        assert!(!session.session_id.is_empty());
        assert!(session.description.is_none());
        assert!(session.user_id.is_none());
        assert!(session.metadata.is_empty());
    }

    #[test]
    fn test_session_with_id() {
        let session = SessionInfo::with_id("test-session-123");
        assert_eq!(session.session_id, "test-session-123");
    }

    #[test]
    fn test_session_builder() {
        let session = SessionInfo::new()
            .with_description("Test session")
            .with_user_id("user-123")
            .with_metadata("key1", serde_json::json!("value1"));

        assert_eq!(session.description, Some("Test session".to_string()));
        assert_eq!(session.user_id, Some("user-123".to_string()));
        assert_eq!(session.metadata.get("key1").unwrap(), &serde_json::json!("value1"));
    }

    #[test]
    fn test_session_age() {
        let session = SessionInfo::new();
        let age = session.age_seconds();
        assert!(age >= 0 && age < 2); // Should be very recent
    }
}
