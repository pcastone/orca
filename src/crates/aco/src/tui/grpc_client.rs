//! HTTP client for TUI to fetch tasks and workflows from orchestrator
//!
//! This client communicates with the orchestrator's gRPC-compatible REST endpoints
//! at /grpc/* to fetch and execute tasks and workflows.

use crate::error::{AcoError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Task info from server (matches orchestrator proto)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub task_type: String,
    #[serde(default)]
    pub config: String,
    #[serde(default)]
    pub metadata: String,
    pub workspace_path: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Workflow info from server (matches orchestrator proto)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
}

/// Bug info from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugInfo {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
    pub priority: i64,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub reporter: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Bug statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BugStats {
    pub total: i64,
    pub open: i64,
    pub in_progress: i64,
    pub fixed: i64,
    pub wontfix: i64,
    pub duplicate: i64,
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub trivial: i64,
}

/// Request to create a bug
#[derive(Debug, Clone, Serialize)]
pub struct CreateBugRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
}

/// Request to update a bug
#[derive(Debug, Clone, Serialize)]
pub struct UpdateBugRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
}

/// Response from list tasks endpoint
#[derive(Debug, Deserialize)]
struct ListTasksResponse {
    tasks: Vec<TaskProto>,
    total: i32,
}

/// Task from proto format (status is i32)
#[derive(Debug, Deserialize)]
struct TaskProto {
    id: String,
    title: String,
    description: String,
    task_type: String,
    status: i32,
    #[serde(default)]
    config: Option<String>,
    #[serde(default)]
    metadata: Option<String>,
    workspace_path: String,
    created_at: String,
    updated_at: String,
}

/// Response from list workflows endpoint
#[derive(Debug, Deserialize)]
struct ListWorkflowsResponse {
    workflows: Vec<WorkflowProto>,
    total: i32,
}

/// Workflow from proto format
#[derive(Debug, Deserialize)]
struct WorkflowProto {
    id: String,
    name: String,
    description: String,
    definition: String,
    status: String,
    created_at: String,
    updated_at: String,
}

/// Execution event from proto format
#[derive(Debug, Deserialize)]
struct ExecutionEventProto {
    timestamp: String,
    event_type: String,
    message: String,
    #[serde(default)]
    status: String,
}

/// Convert status i32 to string
fn status_from_i32(status: i32) -> String {
    match status {
        0 => "pending".to_string(),
        1 => "running".to_string(),
        2 => "completed".to_string(),
        3 => "failed".to_string(),
        4 => "cancelled".to_string(),
        _ => "pending".to_string(),
    }
}

/// HTTP client for TUI
#[derive(Debug)]
pub struct TuiGrpcClient {
    server_url: String,
    client: Client,
}

impl TuiGrpcClient {
    /// Create a new client
    pub fn new(server_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { server_url, client }
    }

    /// Fetch tasks from server
    pub async fn fetch_tasks(&self) -> Result<Vec<TaskInfo>> {
        tracing::debug!("Fetching tasks from {}/grpc/tasks", self.server_url);

        let url = format!("{}/grpc/tasks", self.server_url);

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to fetch tasks: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Failed to fetch tasks: {} - {}", status, error_text);
            // Return empty list on error to allow TUI to continue
            return Ok(Vec::new());
        }

        let list_response: ListTasksResponse = response.json()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to parse tasks response: {}", e)))?;

        // Convert proto format to TaskInfo
        let tasks = list_response.tasks.into_iter().map(|t| TaskInfo {
            id: t.id,
            title: t.title,
            description: t.description,
            status: status_from_i32(t.status),
            task_type: t.task_type,
            config: t.config.unwrap_or_default(),
            metadata: t.metadata.unwrap_or_default(),
            workspace_path: t.workspace_path,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }).collect();

        tracing::debug!("Fetched {} tasks from server", list_response.total);
        Ok(tasks)
    }

    /// Fetch workflows from server
    pub async fn fetch_workflows(&self) -> Result<Vec<WorkflowInfo>> {
        tracing::debug!("Fetching workflows from {}/grpc/workflows", self.server_url);

        let url = format!("{}/grpc/workflows", self.server_url);

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to fetch workflows: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Failed to fetch workflows: {} - {}", status, error_text);
            // Return empty list on error to allow TUI to continue
            return Ok(Vec::new());
        }

        let list_response: ListWorkflowsResponse = response.json()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to parse workflows response: {}", e)))?;

        // Convert proto format to WorkflowInfo
        let workflows = list_response.workflows.into_iter().map(|w| WorkflowInfo {
            id: w.id,
            name: w.name,
            status: w.status,
            created_at: w.created_at,
        }).collect();

        tracing::debug!("Fetched {} workflows from server", list_response.total);
        Ok(workflows)
    }

    /// Execute a task and return execution events
    pub async fn execute_task(&self, task_id: &str) -> Result<Vec<crate::tui::app::ExecutionEvent>> {
        tracing::debug!("Executing task {} on {}/grpc/tasks/{}/execute", task_id, self.server_url, task_id);

        let url = format!("{}/grpc/tasks/{}/execute", self.server_url, task_id);

        let response = self.client.post(&url)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to execute task: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Failed to execute task: {} - {}", status, error_text);
            return Err(AcoError::Connection(format!("Task execution failed: {} - {}", status, error_text)));
        }

        let proto_events: Vec<ExecutionEventProto> = response.json()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to parse execution events: {}", e)))?;

        // Convert proto format to ExecutionEvent
        let events = proto_events.into_iter().map(|e| crate::tui::app::ExecutionEvent {
            timestamp: e.timestamp,
            event_type: e.event_type,
            message: e.message,
            status: e.status,
        }).collect();

        tracing::debug!("Task {} execution complete", task_id);
        Ok(events)
    }

    /// Execute a workflow and return execution events
    pub async fn execute_workflow(&self, workflow_id: &str) -> Result<Vec<crate::tui::app::ExecutionEvent>> {
        tracing::debug!("Executing workflow {} on {}/grpc/workflows/{}/execute", workflow_id, self.server_url, workflow_id);

        let url = format!("{}/grpc/workflows/{}/execute", self.server_url, workflow_id);

        let response = self.client.post(&url)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to execute workflow: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Failed to execute workflow: {} - {}", status, error_text);
            return Err(AcoError::Connection(format!("Workflow execution failed: {} - {}", status, error_text)));
        }

        let proto_events: Vec<ExecutionEventProto> = response.json()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to parse execution events: {}", e)))?;

        // Convert proto format to ExecutionEvent
        let events = proto_events.into_iter().map(|e| crate::tui::app::ExecutionEvent {
            timestamp: e.timestamp,
            event_type: e.event_type,
            message: e.message,
            status: e.status,
        }).collect();

        tracing::debug!("Workflow {} execution complete", workflow_id);
        Ok(events)
    }

    /// Fetch bugs from server
    pub async fn fetch_bugs(&self) -> Result<Vec<BugInfo>> {
        tracing::debug!("Fetching bugs from {}/api/v1/bugs", self.server_url);

        let url = format!("{}/api/v1/bugs", self.server_url);

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to fetch bugs: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Failed to fetch bugs: {} - {}", status, error_text);
            return Ok(Vec::new());
        }

        let bugs: Vec<BugInfo> = response.json()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to parse bugs response: {}", e)))?;

        tracing::debug!("Fetched {} bugs from server", bugs.len());
        Ok(bugs)
    }

    /// Get a single bug by ID
    pub async fn get_bug(&self, bug_id: &str) -> Result<BugInfo> {
        tracing::debug!("Getting bug {} from {}/api/v1/bugs/{}", bug_id, self.server_url, bug_id);

        let url = format!("{}/api/v1/bugs/{}", self.server_url, bug_id);

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to get bug: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AcoError::Connection(format!("Failed to get bug: {} - {}", status, error_text)));
        }

        let bug: BugInfo = response.json()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to parse bug response: {}", e)))?;

        Ok(bug)
    }

    /// Create a new bug
    pub async fn create_bug(&self, request: CreateBugRequest) -> Result<BugInfo> {
        tracing::debug!("Creating bug: {:?}", request);

        let url = format!("{}/api/v1/bugs", self.server_url);

        let response = self.client.post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to create bug: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AcoError::Connection(format!("Failed to create bug: {} - {}", status, error_text)));
        }

        let bug: BugInfo = response.json()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to parse bug response: {}", e)))?;

        tracing::debug!("Created bug: {}", bug.id);
        Ok(bug)
    }

    /// Update a bug
    pub async fn update_bug(&self, bug_id: &str, request: UpdateBugRequest) -> Result<BugInfo> {
        tracing::debug!("Updating bug {}: {:?}", bug_id, request);

        let url = format!("{}/api/v1/bugs/{}", self.server_url, bug_id);

        let response = self.client.put(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to update bug: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AcoError::Connection(format!("Failed to update bug: {} - {}", status, error_text)));
        }

        let bug: BugInfo = response.json()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to parse bug response: {}", e)))?;

        tracing::debug!("Updated bug: {}", bug.id);
        Ok(bug)
    }

    /// Delete a bug
    pub async fn delete_bug(&self, bug_id: &str) -> Result<()> {
        tracing::debug!("Deleting bug {}", bug_id);

        let url = format!("{}/api/v1/bugs/{}", self.server_url, bug_id);

        let response = self.client.delete(&url)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to delete bug: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AcoError::Connection(format!("Failed to delete bug: {} - {}", status, error_text)));
        }

        tracing::debug!("Deleted bug: {}", bug_id);
        Ok(())
    }

    /// Get bug statistics
    pub async fn get_bug_stats(&self) -> Result<BugStats> {
        tracing::debug!("Getting bug stats from {}/api/v1/bugs/stats", self.server_url);

        let url = format!("{}/api/v1/bugs/stats", self.server_url);

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to get bug stats: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!("Failed to get bug stats: {} - {}", status, error_text);
            return Ok(BugStats::default());
        }

        let stats: BugStats = response.json()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to parse bug stats response: {}", e)))?;

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = TuiGrpcClient::new("http://localhost:50051".to_string());
        assert_eq!(client.server_url, "http://localhost:50051");
    }

    #[test]
    fn test_status_from_i32() {
        assert_eq!(status_from_i32(0), "pending");
        assert_eq!(status_from_i32(1), "running");
        assert_eq!(status_from_i32(2), "completed");
        assert_eq!(status_from_i32(3), "failed");
        assert_eq!(status_from_i32(4), "cancelled");
        assert_eq!(status_from_i32(99), "pending"); // Default
    }

    // Integration tests that require a running orchestrator server
    #[tokio::test]
    #[ignore = "requires running orchestrator server"]
    async fn test_fetch_tasks() {
        let client = TuiGrpcClient::new("http://localhost:50051".to_string());
        let result = client.fetch_tasks().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires running orchestrator server"]
    async fn test_fetch_workflows() {
        let client = TuiGrpcClient::new("http://localhost:50051".to_string());
        let result = client.fetch_workflows().await;
        assert!(result.is_ok());
    }
}
