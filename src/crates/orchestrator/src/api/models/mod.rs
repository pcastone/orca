//! API data transfer objects (DTOs) and response models
//!
//! Provides request/response structures for API endpoints with validation.

pub mod task;
pub mod workflow;
pub mod tool_execution;
pub mod bug;
pub mod prompt_history;
pub mod checkpoint;

pub use task::{CreateTaskRequest, UpdateTaskRequest, TaskResponse, TaskListQuery};
pub use workflow::{CreateWorkflowRequest, UpdateWorkflowRequest, WorkflowResponse, WorkflowListQuery};
pub use tool_execution::{ExecuteToolRequest, ToolExecutionResponse, ExecutionListQuery};
pub use bug::{CreateBugRequest, UpdateBugRequest, BugResponse, BugListQuery};
pub use prompt_history::{CreatePromptHistoryRequest, PromptHistoryResponse, PromptHistoryListQuery, PromptHistoryStatsResponse};
pub use checkpoint::{CreateCheckpointRequest, CheckpointResponse, CheckpointListQuery};

/// System health response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthResponse {
    /// Overall status
    pub status: String,

    /// Database connection status
    pub database: String,

    /// API version
    pub version: String,

    /// Current timestamp
    pub timestamp: String,
}

impl HealthResponse {
    /// Create a new health response
    pub fn new(status: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            database: database.into(),
            version: crate::version::VERSION.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// System info response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemInfoResponse {
    /// Application version
    pub version: String,

    /// Build timestamp
    pub build_timestamp: String,

    /// Git commit hash
    pub git_commit: String,

    /// Rust version
    pub rust_version: String,
}

/// System metrics response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemMetricsResponse {
    /// Total tasks
    pub total_tasks: i64,

    /// Active tasks (running)
    pub active_tasks: i64,

    /// Total workflows
    pub total_workflows: i64,

    /// Total tool executions
    pub total_executions: i64,

    /// Average task duration (milliseconds)
    pub avg_task_duration_ms: Option<i64>,

    /// Memory usage estimate (bytes)
    pub memory_bytes: Option<u64>,
}

/// Server status response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatusResponse {
    /// Server name
    pub name: String,

    /// Server UUID
    pub uuid: String,

    /// Server version
    pub version: String,

    /// Server status
    pub status: String,

    /// Number of connected clients
    pub connected_clients: u32,

    /// Database connection status
    pub database: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response() {
        let health = HealthResponse::new("ok", "connected");
        assert_eq!(health.status, "ok");
        assert_eq!(health.database, "connected");
        assert!(!health.version.is_empty());
        assert!(!health.timestamp.is_empty());
    }

    #[test]
    fn test_system_info_response() {
        let info = SystemInfoResponse {
            version: "1.0.0".to_string(),
            build_timestamp: "2025-01-01".to_string(),
            git_commit: "abc123".to_string(),
            rust_version: "1.75".to_string(),
        };
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.git_commit, "abc123");
    }

    #[test]
    fn test_system_metrics_response() {
        let metrics = SystemMetricsResponse {
            total_tasks: 100,
            active_tasks: 10,
            total_workflows: 20,
            total_executions: 500,
            avg_task_duration_ms: Some(5000),
            memory_bytes: Some(1024 * 1024),
        };
        assert_eq!(metrics.total_tasks, 100);
        assert_eq!(metrics.active_tasks, 10);
    }
}
