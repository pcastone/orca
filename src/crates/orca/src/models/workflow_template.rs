//! Workflow template model

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Workflow template (reusable across projects)
///
/// Workflow templates stored in user database (~/.orca/user.db)
/// Can be instantiated in project databases
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkflowTemplate {
    /// Unique template identifier (UUID string)
    pub id: String,

    /// Template name (unique)
    pub name: String,

    /// Template description
    pub description: Option<String>,

    /// Workflow pattern (react, plan_execute, reflection)
    pub pattern: String,

    /// Workflow definition (JSON)
    pub definition: String,

    /// Tags for categorization (JSON array)
    pub tags: String,

    /// Whether template is public/shareable
    pub is_public: bool,

    /// Number of times this template has been used
    pub usage_count: i64,

    /// Additional metadata (JSON)
    pub metadata: String,

    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,
}

impl WorkflowTemplate {
    /// Create a new workflow template
    pub fn new(name: String, pattern: String, definition: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            pattern,
            definition,
            tags: "[]".to_string(),
            is_public: true,
            usage_count: 0,
            metadata: "{}".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Builder: Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Builder: Set tags (JSON array string)
    pub fn with_tags(mut self, tags: String) -> Self {
        self.tags = tags;
        self
    }

    /// Builder: Make private
    pub fn make_private(mut self) -> Self {
        self.is_public = false;
        self
    }

    /// Increment usage count
    pub fn increment_usage(&mut self) {
        self.usage_count += 1;
        self.updated_at = Utc::now().timestamp();
    }
}
