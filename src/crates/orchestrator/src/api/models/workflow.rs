//! Workflow API models and DTOs

use serde::{Deserialize, Serialize};

/// Request to create a new workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowRequest {
    /// Workflow name (required)
    pub name: String,

    /// Workflow description (optional)
    pub description: Option<String>,

    /// Workflow definition as JSON string (required)
    pub definition: String,
}

impl CreateWorkflowRequest {
    /// Validate the create request
    pub fn validate(&self) -> crate::api::error::ApiResult<()> {
        crate::api::middleware::validation::validate_not_empty(&self.name, "name")?;
        crate::api::middleware::validation::validate_string_length(&self.name, "name", 1, 255)?;
        Ok(())
    }
}

/// Request to update an existing workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkflowRequest {
    /// Updated workflow name (optional)
    pub name: Option<String>,

    /// Updated workflow description (optional)
    pub description: Option<String>,

    /// Updated workflow definition (optional)
    pub definition: Option<String>,

    /// Updated workflow status (optional)
    pub status: Option<String>,
}

impl UpdateWorkflowRequest {
    /// Check if any fields are being updated
    pub fn has_updates(&self) -> bool {
        self.name.is_some()
            || self.description.is_some()
            || self.definition.is_some()
            || self.status.is_some()
    }
}

/// Workflow response for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResponse {
    /// Workflow ID
    pub id: String,

    /// Workflow name
    pub name: String,

    /// Workflow description
    pub description: Option<String>,

    /// Workflow definition
    pub definition: String,

    /// Workflow status
    pub status: String,

    /// Creation timestamp
    pub created_at: String,

    /// Last update timestamp
    pub updated_at: String,
}

impl WorkflowResponse {
    /// Create a WorkflowResponse from database Workflow model
    pub fn from_db_workflow(workflow: crate::db::models::Workflow) -> Self {
        Self {
            id: workflow.id,
            name: workflow.name,
            description: workflow.description,
            definition: workflow.definition,
            status: workflow.status,
            created_at: workflow.created_at,
            updated_at: workflow.updated_at,
        }
    }
}

/// Query parameters for listing workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListQuery {
    /// Filter by status (optional)
    pub status: Option<String>,

    /// Search in name and description (optional)
    pub search: Option<String>,

    /// Current page (0-indexed, default 0)
    pub page: Option<u32>,

    /// Items per page (default 20, max 100)
    pub per_page: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_workflow_request_valid() {
        let req = CreateWorkflowRequest {
            name: "Test Workflow".to_string(),
            description: None,
            definition: "{}".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_workflow_request_empty_name() {
        let req = CreateWorkflowRequest {
            name: "".to_string(),
            description: None,
            definition: "{}".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_workflow_request_has_updates() {
        let req = UpdateWorkflowRequest {
            name: Some("New Name".to_string()),
            description: None,
            definition: None,
            status: None,
        };
        assert!(req.has_updates());
    }

    #[test]
    fn test_update_workflow_request_no_updates() {
        let req = UpdateWorkflowRequest {
            name: None,
            description: None,
            definition: None,
            status: None,
        };
        assert!(!req.has_updates());
    }
}
