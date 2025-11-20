//! Checkpoint model for database persistence

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a workflow execution checkpoint in the database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Checkpoint {
    /// Unique checkpoint identifier (UUID string)
    pub id: String,

    /// Associated execution ID
    pub execution_id: String,

    /// Associated workflow ID
    pub workflow_id: String,

    /// Node ID where checkpoint was taken
    pub node_id: Option<String>,

    /// Superstep number in Pregel execution
    pub superstep: i32,

    /// Serialized state (JSON string)
    pub state: String,

    /// Parent checkpoint ID for checkpoint tree
    pub parent_checkpoint_id: Option<String>,

    /// Additional metadata (JSON string)
    pub metadata: Option<String>,

    /// Creation timestamp (ISO8601 string)
    pub created_at: String,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(
        id: String,
        execution_id: String,
        workflow_id: String,
        state: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            execution_id,
            workflow_id,
            node_id: None,
            superstep: 0,
            state,
            parent_checkpoint_id: None,
            metadata: None,
            created_at: now,
        }
    }
}
