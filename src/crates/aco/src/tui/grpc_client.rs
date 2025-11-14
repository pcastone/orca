//! Simple gRPC client for TUI to fetch tasks and workflows
//!
//! This is a lightweight wrapper around the orchestrator proto definitions
//! specifically for the TUI, avoiding dependencies on broken client code.

use crate::error::{AcoError, Result};
use std::time::Duration;

/// Task info from server
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub task_type: String,
    pub config: String,
    pub metadata: String,
    pub workspace_path: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Workflow info from server
#[derive(Debug, Clone)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
}

/// Simple gRPC client for TUI
#[derive(Debug)]
pub struct TuiGrpcClient {
    server_url: String,
    timeout: Duration,
}

impl TuiGrpcClient {
    /// Create a new client
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            timeout: Duration::from_secs(10),
        }
    }

    /// Fetch tasks from server
    pub async fn fetch_tasks(&self) -> Result<Vec<TaskInfo>> {
        // For now, return mock data until gRPC client is fully working
        // TODO: Implement real gRPC call once orchestrator proto client is fixed
        tracing::debug!("Fetching tasks from {}", self.server_url);

        let now = chrono::Utc::now();
        let earlier = now - chrono::Duration::hours(2);

        // Mock data
        Ok(vec![
            TaskInfo {
                id: "task-001".to_string(),
                title: "Sample Task 1".to_string(),
                description: "This is a detailed description of task 1, which involves processing data.".to_string(),
                status: "pending".to_string(),
                task_type: "execution".to_string(),
                config: r#"{"timeout": 300, "retries": 3}"#.to_string(),
                metadata: r#"{"priority": "high", "tags": ["production", "critical"]}"#.to_string(),
                workspace_path: "/tmp/workspace/task-001".to_string(),
                created_at: earlier.to_rfc3339(),
                updated_at: earlier.to_rfc3339(),
            },
            TaskInfo {
                id: "task-002".to_string(),
                title: "Sample Task 2".to_string(),
                description: "Task 2 is currently running and processing workflow steps.".to_string(),
                status: "running".to_string(),
                task_type: "workflow".to_string(),
                config: r#"{"max_steps": 10, "parallel": true}"#.to_string(),
                metadata: r#"{"priority": "medium", "tags": ["development"]}"#.to_string(),
                workspace_path: "/tmp/workspace/task-002".to_string(),
                created_at: earlier.to_rfc3339(),
                updated_at: now.to_rfc3339(),
            },
            TaskInfo {
                id: "task-003".to_string(),
                title: "Sample Task 3".to_string(),
                description: "Validation task that has completed successfully.".to_string(),
                status: "completed".to_string(),
                task_type: "validation".to_string(),
                config: r#"{"validators": ["schema", "integrity"]}"#.to_string(),
                metadata: r#"{"priority": "low", "tags": ["testing"]}"#.to_string(),
                workspace_path: "/tmp/workspace/task-003".to_string(),
                created_at: earlier.to_rfc3339(),
                updated_at: now.to_rfc3339(),
            },
        ])
    }

    /// Fetch workflows from server
    pub async fn fetch_workflows(&self) -> Result<Vec<WorkflowInfo>> {
        // For now, return mock data until gRPC client is fully working
        // TODO: Implement real gRPC call once orchestrator proto client is fixed
        tracing::debug!("Fetching workflows from {}", self.server_url);

        // Mock data
        Ok(vec![
            WorkflowInfo {
                id: "wf-001".to_string(),
                name: "Sample Workflow 1".to_string(),
                status: "draft".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            WorkflowInfo {
                id: "wf-002".to_string(),
                name: "Sample Workflow 2".to_string(),
                status: "active".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        ])
    }

    /// Execute a task and return execution events
    pub async fn execute_task(&self, task_id: &str) -> Result<Vec<crate::tui::app::ExecutionEvent>> {
        // For now, return mock execution events
        // TODO: Implement real gRPC streaming call
        tracing::debug!("Executing task {} on {}", task_id, self.server_url);

        // Simulate execution delay
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Ok(vec![
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "started".to_string(),
                message: format!("Task {} started", task_id),
                status: "started".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "progress".to_string(),
                message: "Initializing execution environment...".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "output".to_string(),
                message: "Processing data from input sources".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "tool_call".to_string(),
                message: "Tool: file_read | Input: {\"path\": \"/data/input.txt\"}".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "tool_result".to_string(),
                message: "Tool: file_read | Result: Successfully read 1024 bytes".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "progress".to_string(),
                message: "Analyzing data patterns...".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "output".to_string(),
                message: "Found 42 matching patterns in the dataset".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "completed".to_string(),
                message: format!("Task {} completed successfully", task_id),
                status: "completed".to_string(),
            },
        ])
    }

    /// Execute a workflow and return execution events
    pub async fn execute_workflow(&self, workflow_id: &str) -> Result<Vec<crate::tui::app::ExecutionEvent>> {
        // For now, return mock execution events
        // TODO: Implement real gRPC streaming call
        tracing::debug!("Executing workflow {} on {}", workflow_id, self.server_url);

        // Simulate execution delay
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Ok(vec![
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "started".to_string(),
                message: format!("Workflow {} started", workflow_id),
                status: "started".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "progress".to_string(),
                message: "Entering node: start_node".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "output".to_string(),
                message: "Node start_node completed successfully".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "progress".to_string(),
                message: "Entering node: process_node".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "tool_call".to_string(),
                message: "Tool: data_processor | Input: {\"mode\": \"batch\"}".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "tool_result".to_string(),
                message: "Tool: data_processor | Result: Processed 156 records".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "progress".to_string(),
                message: "Workflow checkpoint: step 1".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "progress".to_string(),
                message: "Entering node: end_node".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "output".to_string(),
                message: "Node end_node completed successfully".to_string(),
                status: "in_progress".to_string(),
            },
            crate::tui::app::ExecutionEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: "completed".to_string(),
                message: format!("Workflow {} completed: Completed 3 nodes in 1 steps", workflow_id),
                status: "completed".to_string(),
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = TuiGrpcClient::new("http://localhost:50051".to_string());
        assert_eq!(client.server_url, "http://localhost:50051");
    }

    #[tokio::test]
    async fn test_fetch_tasks() {
        let client = TuiGrpcClient::new("http://localhost:50051".to_string());
        let tasks = client.fetch_tasks().await.unwrap();
        assert!(!tasks.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_workflows() {
        let client = TuiGrpcClient::new("http://localhost:50051".to_string());
        let workflows = client.fetch_workflows().await.unwrap();
        assert!(!workflows.is_empty());
    }
}
