//! Worker registry for managing connected ACO clients
//!
//! Tracks connected workers, their capabilities, and routes tool
//! execution requests to appropriate workers.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot};
use uuid::Uuid;

use super::messages::{ToolRequest, ToolResult, WorkerEvent};

/// Information about a connected worker
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// Unique worker ID
    pub id: String,
    /// Human-readable worker name
    pub name: String,
    /// List of tool capabilities
    pub capabilities: Vec<String>,
    /// Workspace root path
    pub workspace_path: String,
    /// When the worker connected
    pub connected_at: DateTime<Utc>,
    /// Last heartbeat received
    pub last_heartbeat: DateTime<Utc>,
    /// Number of pending requests
    pub pending_count: u32,
}

/// Pending tool request awaiting result
pub struct PendingRequest {
    /// The tool request
    pub request: ToolRequest,
    /// Channel to send result back
    pub result_tx: oneshot::Sender<ToolResult>,
    /// When the request was created
    pub created_at: DateTime<Utc>,
}

/// Registry for managing connected workers
pub struct WorkerRegistry {
    /// Connected workers by ID
    workers: DashMap<String, WorkerInfo>,
    /// Pending tool requests by request ID
    pending_requests: DashMap<String, PendingRequest>,
    /// Broadcast channel for sending events to workers
    event_tx: broadcast::Sender<WorkerEvent>,
    /// Per-worker event channels
    worker_channels: DashMap<String, broadcast::Sender<WorkerEvent>>,
}

impl WorkerRegistry {
    /// Create a new worker registry
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            workers: DashMap::new(),
            pending_requests: DashMap::new(),
            event_tx,
            worker_channels: DashMap::new(),
        }
    }

    /// Register a new worker
    pub fn register_worker(
        &self,
        name: String,
        capabilities: Vec<String>,
        workspace_path: String,
    ) -> String {
        let worker_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let worker = WorkerInfo {
            id: worker_id.clone(),
            name,
            capabilities,
            workspace_path,
            connected_at: now,
            last_heartbeat: now,
            pending_count: 0,
        };

        // Create per-worker channel
        let (tx, _) = broadcast::channel(256);
        self.worker_channels.insert(worker_id.clone(), tx);

        self.workers.insert(worker_id.clone(), worker);
        tracing::info!("Registered worker: {}", worker_id);

        worker_id
    }

    /// Unregister a worker
    pub fn unregister_worker(&self, worker_id: &str) {
        if let Some((_, worker)) = self.workers.remove(worker_id) {
            tracing::info!("Unregistered worker: {} ({})", worker.name, worker_id);
        }
        self.worker_channels.remove(worker_id);
    }

    /// Get worker info
    pub fn get_worker(&self, worker_id: &str) -> Option<WorkerInfo> {
        self.workers.get(worker_id).map(|w| w.clone())
    }

    /// Update worker heartbeat
    pub fn update_heartbeat(&self, worker_id: &str) {
        if let Some(mut worker) = self.workers.get_mut(worker_id) {
            worker.last_heartbeat = Utc::now();
        }
    }

    /// Find a worker with the given capability
    pub fn find_worker_for_tool(&self, tool_name: &str) -> Option<WorkerInfo> {
        // Find workers with matching capability, prefer one with lowest pending count
        self.workers
            .iter()
            .filter(|w| w.capabilities.contains(&tool_name.to_string()))
            .min_by_key(|w| w.pending_count)
            .map(|w| w.clone())
    }

    /// Subscribe to events for a specific worker
    pub fn subscribe(&self, worker_id: &str) -> Option<broadcast::Receiver<WorkerEvent>> {
        self.worker_channels
            .get(worker_id)
            .map(|tx| tx.subscribe())
    }

    /// Send event to a specific worker
    pub fn send_to_worker(&self, worker_id: &str, event: WorkerEvent) -> Result<(), String> {
        if let Some(tx) = self.worker_channels.get(worker_id) {
            tx.send(event)
                .map_err(|e| format!("Failed to send event: {}", e))?;
            Ok(())
        } else {
            Err(format!("Worker not found: {}", worker_id))
        }
    }

    /// Store a pending request
    pub fn add_pending_request(&self, request: ToolRequest, result_tx: oneshot::Sender<ToolResult>) {
        let request_id = request.request_id.clone();
        let worker_id = request.worker_id.clone();

        // Increment pending count for worker
        if let Some(mut worker) = self.workers.get_mut(&worker_id) {
            worker.pending_count += 1;
        }

        self.pending_requests.insert(
            request_id,
            PendingRequest {
                request,
                result_tx,
                created_at: Utc::now(),
            },
        );
    }

    /// Complete a pending request with result
    pub fn complete_request(&self, result: ToolResult) -> Result<(), String> {
        let request_id = result.request_id.clone();
        let worker_id = result.worker_id.clone();

        // Decrement pending count for worker
        if let Some(mut worker) = self.workers.get_mut(&worker_id) {
            worker.pending_count = worker.pending_count.saturating_sub(1);
        }

        if let Some((_, pending)) = self.pending_requests.remove(&request_id) {
            pending
                .result_tx
                .send(result)
                .map_err(|_| "Result channel closed".to_string())
        } else {
            Err(format!("Request not found: {}", request_id))
        }
    }

    /// Get count of connected workers
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Get all connected workers
    pub fn list_workers(&self) -> Vec<WorkerInfo> {
        self.workers.iter().map(|w| w.clone()).collect()
    }

    /// Clean up stale workers (no heartbeat for duration)
    pub fn cleanup_stale_workers(&self, max_age: chrono::Duration) {
        let cutoff = Utc::now() - max_age;
        let stale: Vec<_> = self
            .workers
            .iter()
            .filter(|w| w.last_heartbeat < cutoff)
            .map(|w| w.id.clone())
            .collect();

        for worker_id in stale {
            tracing::warn!("Removing stale worker: {}", worker_id);
            self.unregister_worker(&worker_id);
        }
    }
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_worker() {
        let registry = WorkerRegistry::new();
        let worker_id = registry.register_worker(
            "test-worker".to_string(),
            vec!["file_read".to_string(), "shell_exec".to_string()],
            "/tmp/workspace".to_string(),
        );

        assert!(!worker_id.is_empty());
        assert!(registry.get_worker(&worker_id).is_some());
        assert_eq!(registry.worker_count(), 1);
    }

    #[test]
    fn test_find_worker_for_tool() {
        let registry = WorkerRegistry::new();
        registry.register_worker(
            "worker1".to_string(),
            vec!["file_read".to_string()],
            "/tmp/w1".to_string(),
        );
        registry.register_worker(
            "worker2".to_string(),
            vec!["shell_exec".to_string()],
            "/tmp/w2".to_string(),
        );

        let worker = registry.find_worker_for_tool("file_read");
        assert!(worker.is_some());
        assert!(worker.unwrap().capabilities.contains(&"file_read".to_string()));

        let worker = registry.find_worker_for_tool("unknown_tool");
        assert!(worker.is_none());
    }

    #[test]
    fn test_unregister_worker() {
        let registry = WorkerRegistry::new();
        let worker_id = registry.register_worker(
            "test-worker".to_string(),
            vec!["file_read".to_string()],
            "/tmp/workspace".to_string(),
        );

        assert_eq!(registry.worker_count(), 1);
        registry.unregister_worker(&worker_id);
        assert_eq!(registry.worker_count(), 0);
    }
}
