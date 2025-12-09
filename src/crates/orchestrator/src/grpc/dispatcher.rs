//! Tool dispatcher for routing tool execution requests to workers
//!
//! Dispatches tool requests to available workers and waits for results.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use uuid::Uuid;
use serde_json::Value;
use thiserror::Error;

use super::messages::{ToolRequest, ToolResult, WorkerEvent};
use super::workers::WorkerRegistry;

/// Errors that can occur during tool dispatch
#[derive(Debug, Error)]
pub enum DispatchError {
    /// No worker available for the tool
    #[error("No worker available for tool: {0}")]
    NoWorkerAvailable(String),

    /// Worker not found
    #[error("Worker not found: {0}")]
    WorkerNotFound(String),

    /// Failed to send request to worker
    #[error("Failed to send request: {0}")]
    SendFailed(String),

    /// Request timed out
    #[error("Tool execution timed out after {0:?}")]
    Timeout(Duration),

    /// Result channel closed
    #[error("Result channel closed")]
    ChannelClosed,

    /// Tool execution failed
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),
}

/// Result type for dispatcher operations
pub type DispatchResult<T> = Result<T, DispatchError>;

/// Tool dispatcher for routing requests to workers
pub struct ToolDispatcher {
    /// Worker registry for finding available workers
    registry: Arc<WorkerRegistry>,
}

impl ToolDispatcher {
    /// Create a new tool dispatcher
    pub fn new(registry: Arc<WorkerRegistry>) -> Self {
        Self { registry }
    }

    /// Execute a tool on an available worker
    ///
    /// Finds a worker with the capability, sends the request, and waits for the result.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        timeout: Duration,
    ) -> DispatchResult<ToolResult> {
        // Find a worker with the capability
        let worker = self.registry.find_worker_for_tool(tool_name)
            .ok_or_else(|| DispatchError::NoWorkerAvailable(tool_name.to_string()))?;

        let worker_id = worker.id.clone();
        let request_id = Uuid::new_v4().to_string();

        tracing::info!(
            "Dispatching tool {} to worker {} (request {})",
            tool_name, worker_id, request_id
        );

        // Create result channel
        let (result_tx, result_rx) = oneshot::channel();

        // Create tool request
        let request = ToolRequest {
            request_id: request_id.clone(),
            worker_id: worker_id.clone(),
            tool_name: tool_name.to_string(),
            arguments,
            timeout_ms: timeout.as_millis() as u64,
        };

        // Store pending request
        self.registry.add_pending_request(request.clone(), result_tx);

        // Send request to worker via SSE
        let event = WorkerEvent::ToolRequest(request);
        self.registry.send_to_worker(&worker_id, event)
            .map_err(|e| DispatchError::SendFailed(e))?;

        // Wait for result with timeout
        let result = tokio::time::timeout(timeout, result_rx)
            .await
            .map_err(|_| DispatchError::Timeout(timeout))?
            .map_err(|_| DispatchError::ChannelClosed)?;

        if !result.success {
            if let Some(error) = &result.error {
                tracing::warn!("Tool {} execution failed: {}", tool_name, error);
            }
        } else {
            tracing::info!(
                "Tool {} completed successfully in {}ms",
                tool_name, result.duration_ms
            );
        }

        Ok(result)
    }

    /// Execute a tool with a specific worker
    pub async fn execute_tool_on_worker(
        &self,
        worker_id: &str,
        tool_name: &str,
        arguments: Value,
        timeout: Duration,
    ) -> DispatchResult<ToolResult> {
        // Verify worker exists
        let _worker = self.registry.get_worker(worker_id)
            .ok_or_else(|| DispatchError::WorkerNotFound(worker_id.to_string()))?;

        let request_id = Uuid::new_v4().to_string();

        tracing::info!(
            "Dispatching tool {} to specific worker {} (request {})",
            tool_name, worker_id, request_id
        );

        // Create result channel
        let (result_tx, result_rx) = oneshot::channel();

        // Create tool request
        let request = ToolRequest {
            request_id: request_id.clone(),
            worker_id: worker_id.to_string(),
            tool_name: tool_name.to_string(),
            arguments,
            timeout_ms: timeout.as_millis() as u64,
        };

        // Store pending request
        self.registry.add_pending_request(request.clone(), result_tx);

        // Send request to worker via SSE
        let event = WorkerEvent::ToolRequest(request);
        self.registry.send_to_worker(worker_id, event)
            .map_err(|e| DispatchError::SendFailed(e))?;

        // Wait for result with timeout
        let result = tokio::time::timeout(timeout, result_rx)
            .await
            .map_err(|_| DispatchError::Timeout(timeout))?
            .map_err(|_| DispatchError::ChannelClosed)?;

        Ok(result)
    }

    /// Check if any worker can execute the given tool
    pub fn can_execute(&self, tool_name: &str) -> bool {
        self.registry.find_worker_for_tool(tool_name).is_some()
    }

    /// Get the list of all available tools across all workers
    pub fn available_tools(&self) -> Vec<String> {
        let mut tools = std::collections::HashSet::new();
        for worker in self.registry.list_workers() {
            for cap in worker.capabilities {
                tools.insert(cap);
            }
        }
        tools.into_iter().collect()
    }

    /// Get worker count
    pub fn worker_count(&self) -> usize {
        self.registry.worker_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher_creation() {
        let registry = Arc::new(WorkerRegistry::new());
        let dispatcher = ToolDispatcher::new(registry);
        assert_eq!(dispatcher.worker_count(), 0);
    }

    #[test]
    fn test_can_execute_no_workers() {
        let registry = Arc::new(WorkerRegistry::new());
        let dispatcher = ToolDispatcher::new(registry);
        assert!(!dispatcher.can_execute("file_read"));
    }

    #[test]
    fn test_can_execute_with_worker() {
        let registry = Arc::new(WorkerRegistry::new());
        registry.register_worker(
            "test-worker".to_string(),
            vec!["file_read".to_string(), "shell_exec".to_string()],
            "/tmp".to_string(),
        );

        let dispatcher = ToolDispatcher::new(registry);
        assert!(dispatcher.can_execute("file_read"));
        assert!(dispatcher.can_execute("shell_exec"));
        assert!(!dispatcher.can_execute("unknown_tool"));
    }

    #[test]
    fn test_available_tools() {
        let registry = Arc::new(WorkerRegistry::new());
        registry.register_worker(
            "worker1".to_string(),
            vec!["file_read".to_string()],
            "/tmp".to_string(),
        );
        registry.register_worker(
            "worker2".to_string(),
            vec!["shell_exec".to_string(), "file_read".to_string()],
            "/tmp".to_string(),
        );

        let dispatcher = ToolDispatcher::new(registry);
        let tools = dispatcher.available_tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.contains(&"file_read".to_string()));
        assert!(tools.contains(&"shell_exec".to_string()));
    }
}
