//! Prompt template model

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Global prompt template
///
/// Reusable prompt templates stored in user database (~/.orca/user.db)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Prompt {
    /// Unique prompt identifier (UUID string)
    pub id: String,

    /// Prompt name (unique)
    pub name: String,

    /// Prompt description
    pub description: Option<String>,

    /// Prompt template text
    pub template: String,

    /// Category (system, task, workflow, custom)
    pub category: Option<String>,

    /// Variables used in template (JSON array)
    pub variables: String,

    /// Additional metadata (JSON)
    pub metadata: String,

    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,
}

impl Prompt {
    /// Create a new prompt template
    pub fn new(name: String, template: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            template,
            category: Some("custom".to_string()),
            variables: "[]".to_string(),
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

    /// Builder: Set category
    pub fn with_category(mut self, category: String) -> Self {
        self.category = Some(category);
        self
    }

    /// Builder: Set variables (JSON array string)
    pub fn with_variables(mut self, variables: String) -> Self {
        self.variables = variables;
        self
    }
}
