//! Project rule model

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Project rule configuration
///
/// Defines rules for code style, security, workflow enforcement
/// Stored in project database (<project>/.orca/project.db)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectRule {
    /// Unique rule identifier (UUID string)
    pub id: String,

    /// Rule name
    pub name: String,

    /// Rule type (style, security, workflow, custom)
    pub rule_type: String,

    /// Rule description
    pub description: Option<String>,

    /// Rule configuration (JSON)
    pub config: String,

    /// Severity level (error, warning, info)
    pub severity: String,

    /// Whether rule is enabled
    pub enabled: bool,

    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,
}

impl ProjectRule {
    /// Create a new project rule
    pub fn new(name: String, rule_type: String, config: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            rule_type,
            description: None,
            config,
            severity: "warning".to_string(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Builder: Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Builder: Set severity
    pub fn with_severity(mut self, severity: &str) -> Self {
        self.severity = severity.to_string();
        self
    }

    /// Builder: Disable rule
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Enable the rule
    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now().timestamp();
    }

    /// Disable the rule
    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now().timestamp();
    }

    /// Check if rule is an error (blocks execution)
    pub fn is_error(&self) -> bool {
        self.severity == "error"
    }

    /// Check if rule is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
