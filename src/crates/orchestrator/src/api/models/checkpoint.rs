//! Checkpoint API models and DTOs

use serde::{Deserialize, Serialize};

/// Request to create a new checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCheckpointRequest {
    /// Execution ID (required)
    pub execution_id: String,

    /// Workflow ID (required)
    pub workflow_id: String,

    /// Serialized state (required)
    pub state: String,

    /// Node ID (optional)
    pub node_id: Option<String>,

    /// Superstep number (optional, default 0)
    pub superstep: Option<i32>,

    /// Parent checkpoint ID (optional)
    pub parent_checkpoint_id: Option<String>,

    /// Additional metadata (optional, JSON)
    pub metadata: Option<String>,
}

impl CreateCheckpointRequest {
    /// Validate the create request
    pub fn validate(&self) -> crate::api::error::ApiResult<()> {
        crate::api::middleware::validation::validate_not_empty(&self.execution_id, "execution_id")?;
        crate::api::middleware::validation::validate_not_empty(&self.workflow_id, "workflow_id")?;
        crate::api::middleware::validation::validate_not_empty(&self.state, "state")?;
        Ok(())
    }
}

/// Checkpoint response for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointResponse {
    pub id: String,
    pub execution_id: String,
    pub workflow_id: String,
    pub node_id: Option<String>,
    pub superstep: i32,
    pub state: String,
    pub parent_checkpoint_id: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

impl CheckpointResponse {
    /// Create a CheckpointResponse from database model
    pub fn from_db_checkpoint(checkpoint: crate::db::models::Checkpoint) -> Self {
        Self {
            id: checkpoint.id,
            execution_id: checkpoint.execution_id,
            workflow_id: checkpoint.workflow_id,
            node_id: checkpoint.node_id,
            superstep: checkpoint.superstep,
            state: checkpoint.state,
            parent_checkpoint_id: checkpoint.parent_checkpoint_id,
            metadata: checkpoint.metadata,
            created_at: checkpoint.created_at,
        }
    }
}

/// Query parameters for listing checkpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointListQuery {
    /// Filter by execution ID (optional)
    pub execution_id: Option<String>,

    /// Filter by workflow ID (optional)
    pub workflow_id: Option<String>,

    /// Current page (0-indexed, default 0)
    pub page: Option<u32>,

    /// Items per page (default 20, max 100)
    pub per_page: Option<u32>,
}
