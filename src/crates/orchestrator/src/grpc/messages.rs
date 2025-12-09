//! Protocol messages for worker communication
//!
//! Defines the message types used for communication between orchestrator
//! and ACO worker clients via SSE and HTTP.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Worker registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterWorkerRequest {
    /// Human-readable worker name
    pub name: String,
    /// List of tool capabilities (e.g., ["file_read", "shell_exec", "git_status"])
    pub capabilities: Vec<String>,
    /// Workspace root path for file operations
    pub workspace_path: String,
}

/// Worker registration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterWorkerResponse {
    /// Unique worker ID assigned by orchestrator
    pub worker_id: String,
    /// Recommended heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
}

/// Tool execution request sent to worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    /// Unique request ID for correlation
    pub request_id: String,
    /// Target worker ID (for routing)
    pub worker_id: String,
    /// Name of the tool to execute
    pub tool_name: String,
    /// Tool arguments as JSON
    pub arguments: Value,
    /// Execution timeout in milliseconds
    pub timeout_ms: u64,
}

/// Tool execution result from worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Request ID for correlation
    pub request_id: String,
    /// Worker that executed the tool
    pub worker_id: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Tool output (if successful)
    pub output: Option<Value>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution duration in milliseconds
    pub duration_ms: i64,
}

/// SSE events sent to workers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerEvent {
    /// Tool execution request
    ToolRequest(ToolRequest),
    /// Heartbeat to keep connection alive
    Heartbeat { timestamp: String },
    /// Server shutdown notification
    Shutdown { reason: String },
}

/// Query parameters for worker events endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct WorkerEventsParams {
    /// Worker ID to receive events for
    pub worker_id: String,
}
