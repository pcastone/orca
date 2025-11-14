//! WebSocket backpressure handling
//!
//! Handles slow consumers by buffering and managing message queue backpressure.

use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use serde::{Serialize, Deserialize};

/// Message in the backpressure queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedMessage {
    /// Message content
    pub data: String,
    /// Timestamp when queued
    pub timestamp: i64,
    /// Sequence number for tracking
    pub sequence: u64,
}

/// Backpressure manager for a client
#[derive(Clone)]
pub struct ClientBackpressure {
    /// Message queue
    queue: Arc<parking_lot::Mutex<VecDeque<QueuedMessage>>>,
    /// Max queue size
    max_size: usize,
    /// Message sequence counter
    sequence: Arc<std::sync::atomic::AtomicU64>,
    /// Dropped message count
    dropped: Arc<std::sync::atomic::AtomicU64>,
}

impl ClientBackpressure {
    /// Create new client backpressure handler
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: Arc::new(parking_lot::Mutex::new(VecDeque::new())),
            max_size,
            sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            dropped: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Try to enqueue a message
    pub fn enqueue(&self, data: String) -> Result<u64, BackpressureError> {
        let mut queue = self.queue.lock();

        if queue.len() >= self.max_size {
            // Remove oldest message (FIFO)
            queue.pop_front();
            self.dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        let sequence = self.sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let msg = QueuedMessage {
            data,
            timestamp: chrono::Utc::now().timestamp(),
            sequence,
        };

        queue.push_back(msg);
        Ok(sequence)
    }

    /// Dequeue a message
    pub fn dequeue(&self) -> Option<QueuedMessage> {
        let mut queue = self.queue.lock();
        queue.pop_front()
    }

    /// Get queue size
    pub fn queue_size(&self) -> usize {
        self.queue.lock().len()
    }

    /// Check if queue is full
    pub fn is_full(&self) -> bool {
        self.queue.lock().len() >= self.max_size
    }

    /// Get queue depth percentage
    pub fn queue_depth_percent(&self) -> f64 {
        let size = self.queue.lock().len();
        (size as f64 / self.max_size as f64) * 100.0
    }

    /// Get dropped message count
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Clear the queue
    pub fn clear(&self) {
        self.queue.lock().clear();
    }

    /// Get next sequence number
    pub fn next_sequence(&self) -> u64 {
        self.sequence.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Global backpressure manager
pub struct BackpressureManager {
    /// Per-client backpressure handlers
    clients: Arc<DashMap<String, ClientBackpressure>>,
    /// Default max queue size per client (100 messages)
    default_max_size: usize,
}

/// Backpressure error
#[derive(Debug, Clone)]
pub enum BackpressureError {
    /// Queue full and message was dropped
    Dropped,
}

impl std::fmt::Display for BackpressureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackpressureError::Dropped => write!(f, "Message dropped due to backpressure"),
        }
    }
}

impl std::error::Error for BackpressureError {}

impl BackpressureManager {
    /// Create new backpressure manager
    pub fn new(default_max_size: usize) -> Self {
        Self {
            clients: Arc::new(DashMap::new()),
            default_max_size,
        }
    }

    /// Create with default settings (100 messages per client)
    pub fn new_default() -> Self {
        Self::new(100)
    }

    /// Get or create backpressure handler for a client
    pub fn get_client(&self, client_id: &str) -> ClientBackpressure {
        self.clients
            .entry(client_id.to_string())
            .or_insert_with(|| ClientBackpressure::new(self.default_max_size))
            .clone()
    }

    /// Enqueue a message for a client
    pub fn enqueue(&self, client_id: &str, message: String) -> Result<(), String> {
        let handler = self.get_client(client_id);
        handler.enqueue(message).map(|_| ()).map_err(|e| e.to_string())
    }

    /// Dequeue a message for a client
    pub fn dequeue(&self, client_id: &str) -> Option<QueuedMessage> {
        self.clients
            .get(client_id)
            .and_then(|handler| handler.dequeue())
    }

    /// Get queue status for all clients
    pub fn get_all_queue_status(&self) -> Vec<ClientQueueStatus> {
        self.clients
            .iter()
            .map(|entry| ClientQueueStatus {
                client_id: entry.key().clone(),
                queue_size: entry.value().queue_size(),
                dropped: entry.value().dropped_count(),
                depth_percent: entry.value().queue_depth_percent(),
            })
            .collect()
    }

    /// Remove client from tracking
    pub fn remove_client(&self, client_id: &str) {
        self.clients.remove(client_id);
    }

    /// Get clients with full queues
    pub fn get_full_queues(&self) -> Vec<String> {
        self.clients
            .iter()
            .filter(|entry| entry.value().is_full())
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for BackpressureManager {
    fn default() -> Self {
        Self::new_default()
    }
}

/// Client queue status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientQueueStatus {
    /// Client ID
    pub client_id: String,
    /// Current queue size
    pub queue_size: usize,
    /// Messages dropped
    pub dropped: u64,
    /// Queue depth as percentage
    pub depth_percent: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_backpressure_creation() {
        let bp = ClientBackpressure::new(100);
        assert_eq!(bp.queue_size(), 0);
        assert_eq!(bp.dropped_count(), 0);
    }

    #[test]
    fn test_enqueue_dequeue() {
        let bp = ClientBackpressure::new(10);

        bp.enqueue("msg1".to_string()).unwrap();
        bp.enqueue("msg2".to_string()).unwrap();

        assert_eq!(bp.queue_size(), 2);

        let msg1 = bp.dequeue().unwrap();
        assert_eq!(msg1.data, "msg1");

        let msg2 = bp.dequeue().unwrap();
        assert_eq!(msg2.data, "msg2");

        assert!(bp.dequeue().is_none());
    }

    #[test]
    fn test_queue_overflow() {
        let bp = ClientBackpressure::new(3);

        for i in 0..5 {
            let _ = bp.enqueue(format!("msg{}", i));
        }

        // Queue should have dropped oldest 2 messages
        assert_eq!(bp.queue_size(), 3);
        assert_eq!(bp.dropped_count(), 2);

        let msg = bp.dequeue().unwrap();
        assert_eq!(msg.data, "msg2");
    }

    #[test]
    fn test_queue_full() {
        let bp = ClientBackpressure::new(3);

        bp.enqueue("msg1".to_string()).unwrap();
        bp.enqueue("msg2".to_string()).unwrap();
        bp.enqueue("msg3".to_string()).unwrap();

        assert!(bp.is_full());
        assert!(bp.queue_depth_percent() > 90.0);
    }

    #[test]
    fn test_backpressure_manager() {
        let mgr = BackpressureManager::new(5);

        mgr.enqueue("client1", "msg1".to_string()).unwrap();
        mgr.enqueue("client1", "msg2".to_string()).unwrap();

        assert_eq!(mgr.get_client("client1").queue_size(), 2);

        let msg = mgr.dequeue("client1").unwrap();
        assert_eq!(msg.data, "msg1");
    }

    #[test]
    fn test_queue_status() {
        let mgr = BackpressureManager::new(5);

        mgr.enqueue("client1", "msg1".to_string()).unwrap();
        mgr.enqueue("client2", "msg1".to_string()).unwrap();
        mgr.enqueue("client2", "msg2".to_string()).unwrap();

        let status = mgr.get_all_queue_status();
        assert_eq!(status.len(), 2);
    }

    #[test]
    fn test_sequence_numbers() {
        let bp = ClientBackpressure::new(10);

        let seq1 = bp.enqueue("msg1".to_string()).unwrap();
        let seq2 = bp.enqueue("msg2".to_string()).unwrap();

        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
    }
}
