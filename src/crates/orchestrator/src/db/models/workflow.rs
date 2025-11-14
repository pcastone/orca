//! Workflow model for database persistence

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a workflow in the orchestrator database
///
/// Workflows are collections of tasks with defined execution patterns.
/// They can be in various states: draft, active, archived, or paused.
///
/// # Timestamps
/// All timestamp fields are ISO8601 strings due to SQLite type limitations.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Workflow {
    /// Unique workflow identifier (UUID string)
    pub id: String,

    /// Workflow name
    pub name: String,

    /// Optional workflow description
    pub description: Option<String>,

    /// Workflow definition as JSON string
    pub definition: String,

    /// Current workflow status: draft, active, archived, paused
    pub status: String,

    /// Workflow creation timestamp (ISO8601 string)
    pub created_at: String,

    /// Workflow last update timestamp (ISO8601 string)
    pub updated_at: String,
}

impl Workflow {
    /// Create a new workflow with required fields
    ///
    /// # Arguments
    /// * `id` - Unique workflow identifier
    /// * `name` - Workflow name
    /// * `definition` - Workflow definition as JSON
    ///
    /// # Returns
    /// A new Workflow with default values for optional fields
    pub fn new(id: String, name: String, definition: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name,
            description: None,
            definition,
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Builder method to set workflow description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builder method to set initial workflow status
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    /// Check if workflow is active
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    /// Check if workflow is archived
    pub fn is_archived(&self) -> bool {
        self.status == "archived"
    }

    /// Check if workflow is paused
    pub fn is_paused(&self) -> bool {
        self.status == "paused"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let definition = r#"{"steps": [{"id": "step1"}]}"#;
        let workflow = Workflow::new(
            "workflow-1".to_string(),
            "Test Workflow".to_string(),
            definition.to_string(),
        );

        assert_eq!(workflow.id, "workflow-1");
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.definition, definition);
        assert_eq!(workflow.status, "draft");
    }

    #[test]
    fn test_workflow_with_description() {
        let workflow = Workflow::new(
            "workflow-1".to_string(),
            "Test Workflow".to_string(),
            r#"{"steps": []}"#.to_string(),
        )
        .with_description("A test workflow");

        assert_eq!(workflow.description, Some("A test workflow".to_string()));
    }

    #[test]
    fn test_workflow_status_checks() {
        let mut workflow = Workflow::new(
            "workflow-1".to_string(),
            "Test Workflow".to_string(),
            r#"{"steps": []}"#.to_string(),
        );

        assert!(!workflow.is_active());
        assert!(!workflow.is_archived());
        assert!(!workflow.is_paused());

        workflow.status = "active".to_string();
        assert!(workflow.is_active());

        workflow.status = "archived".to_string();
        assert!(workflow.is_archived());

        workflow.status = "paused".to_string();
        assert!(workflow.is_paused());
    }
}
