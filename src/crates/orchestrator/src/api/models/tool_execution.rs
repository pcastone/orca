//! Tool Execution API models and DTOs

use serde::{Deserialize, Serialize};

/// Request to execute a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteToolRequest {
    /// Tool name/identifier
    pub tool_name: String,

    /// Tool arguments as JSON
    pub arguments: String,
}

impl ExecuteToolRequest {
    /// Validate the execute request
    pub fn validate(&self) -> crate::api::error::ApiResult<()> {
        crate::api::middleware::validation::validate_not_empty(&self.tool_name, "tool_name")?;
        crate::api::middleware::validation::validate_not_empty(&self.arguments, "arguments")?;
        Ok(())
    }
}

/// Tool execution response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResponse {
    /// Execution ID
    pub id: String,

    /// Task ID that triggered this execution
    pub task_id: String,

    /// Tool name
    pub tool_name: String,

    /// Execution status (pending, running, completed, failed, timeout)
    pub status: String,

    /// Tool arguments as JSON
    pub arguments: String,

    /// Execution output
    pub output: Option<String>,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution creation timestamp (ISO8601)
    pub created_at: String,

    /// Execution completion timestamp
    pub completed_at: Option<String>,

    /// Execution duration in milliseconds
    pub duration_ms: Option<i32>,
}

impl ToolExecutionResponse {
    /// Create from database ToolExecution model
    pub fn from_db_execution(execution: crate::db::models::ToolExecution) -> Self {
        Self {
            id: execution.id,
            task_id: execution.task_id,
            tool_name: execution.tool_name,
            status: execution.status,
            arguments: execution.arguments,
            output: execution.output,
            error: execution.error,
            created_at: execution.created_at,
            completed_at: execution.completed_at,
            duration_ms: execution.duration_ms,
        }
    }
}

/// Query parameters for listing executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionListQuery {
    /// Filter by status (optional)
    pub status: Option<String>,

    /// Filter by tool name (optional)
    pub tool_name: Option<String>,

    /// Current page (0-indexed, default 0)
    pub page: Option<u32>,

    /// Items per page (default 20, max 100)
    pub per_page: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_tool_request_valid() {
        let req = ExecuteToolRequest {
            tool_name: "shell_exec".to_string(),
            arguments: r#"{"command": "ls"}"#.to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_execute_tool_request_empty_tool() {
        let req = ExecuteToolRequest {
            tool_name: "".to_string(),
            arguments: "{}".to_string(),
        };
        assert!(req.validate().is_err());
    }
}
