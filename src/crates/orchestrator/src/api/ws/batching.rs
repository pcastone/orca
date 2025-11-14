//! Event batching for efficient delivery
//!
//! Batches multiple events together to reduce message overhead.

use crate::api::ws::events::RealtimeEvent;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Batched events message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBatch {
    /// Batch ID
    pub batch_id: String,
    /// Events in batch
    pub events: Vec<RealtimeEvent>,
    /// Batch creation timestamp
    pub timestamp: String,
}

impl EventBatch {
    /// Create new event batch
    pub fn new() -> Self {
        Self {
            batch_id: uuid::Uuid::new_v4().to_string(),
            events: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Add event to batch
    pub fn add_event(&mut self, event: RealtimeEvent) {
        self.events.push(event);
    }

    /// Get batch size
    pub fn size(&self) -> usize {
        self.events.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl Default for EventBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Event batcher for a client
pub struct ClientBatcher {
    /// Current batch
    batch: Arc<parking_lot::Mutex<EventBatch>>,
    /// Last flush timestamp (millis)
    last_flush: Arc<AtomicU64>,
    /// Max batch size
    max_size: usize,
    /// Flush interval in millis
    flush_interval_ms: u64,
}

impl ClientBatcher {
    /// Create new client batcher
    pub fn new(max_size: usize, flush_interval_ms: u64) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            batch: Arc::new(parking_lot::Mutex::new(EventBatch::new())),
            last_flush: Arc::new(AtomicU64::new(now)),
            max_size,
            flush_interval_ms,
        }
    }

    /// Create with defaults (10 events max, 100ms interval)
    pub fn new_default() -> Self {
        Self::new(10, 100)
    }

    /// Add event to batch
    pub fn add_event(&self, event: RealtimeEvent) -> Option<EventBatch> {
        let mut batch = self.batch.lock();
        batch.add_event(event);

        // Check if should flush
        if batch.size() >= self.max_size {
            let mut new_batch = EventBatch::new();
            std::mem::swap(&mut *batch, &mut new_batch);
            self.last_flush.store(
                chrono::Utc::now().timestamp_millis() as u64,
                Ordering::Relaxed,
            );
            drop(batch);
            return Some(new_batch);
        }

        None
    }

    /// Check if batch should flush by timeout
    pub fn should_flush(&self) -> bool {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let last_flush = self.last_flush.load(Ordering::Relaxed);
        now - last_flush >= self.flush_interval_ms
    }

    /// Flush batch (may be partial)
    pub fn flush(&self) -> Option<EventBatch> {
        let mut batch = self.batch.lock();

        if batch.is_empty() {
            return None;
        }

        let mut new_batch = EventBatch::new();
        std::mem::swap(&mut *batch, &mut new_batch);
        self.last_flush.store(
            chrono::Utc::now().timestamp_millis() as u64,
            Ordering::Relaxed,
        );
        drop(batch);

        Some(new_batch)
    }

    /// Get pending batch size
    pub fn pending_size(&self) -> usize {
        self.batch.lock().size()
    }

    /// Get time since last flush
    pub fn time_since_last_flush_ms(&self) -> u64 {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let last_flush = self.last_flush.load(Ordering::Relaxed);
        now.saturating_sub(last_flush)
    }
}

impl Clone for ClientBatcher {
    fn clone(&self) -> Self {
        Self {
            batch: Arc::clone(&self.batch),
            last_flush: Arc::clone(&self.last_flush),
            max_size: self.max_size,
            flush_interval_ms: self.flush_interval_ms,
        }
    }
}

/// Global batching manager
pub struct BatchingManager {
    /// Per-client batchers
    batchers: Arc<dashmap::DashMap<String, ClientBatcher>>,
    /// Default max batch size
    default_max_size: usize,
    /// Default flush interval (ms)
    default_flush_interval_ms: u64,
}

impl BatchingManager {
    /// Create new batching manager
    pub fn new(default_max_size: usize, default_flush_interval_ms: u64) -> Self {
        Self {
            batchers: Arc::new(dashmap::DashMap::new()),
            default_max_size,
            default_flush_interval_ms,
        }
    }

    /// Create with defaults
    pub fn new_default() -> Self {
        Self::new(10, 100)
    }

    /// Get or create batcher for a client
    pub fn get_batcher(&self, client_id: &str) -> ClientBatcher {
        self.batchers
            .entry(client_id.to_string())
            .or_insert_with(|| {
                ClientBatcher::new(self.default_max_size, self.default_flush_interval_ms)
            })
            .clone()
    }

    /// Add event to client batch
    pub fn add_event(&self, client_id: &str, event: RealtimeEvent) -> Option<EventBatch> {
        let batcher = self.get_batcher(client_id);
        batcher.add_event(event)
    }

    /// Get clients that need flushing
    pub fn get_clients_needing_flush(&self) -> Vec<String> {
        self.batchers
            .iter()
            .filter(|entry| entry.value().should_flush())
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Flush a client's batch
    pub fn flush_client(&self, client_id: &str) -> Option<EventBatch> {
        self.batchers
            .get(client_id)
            .and_then(|batcher| batcher.flush())
    }

    /// Get pending batch info for all clients
    pub fn get_all_pending_batches(&self) -> Vec<(String, usize, u64)> {
        self.batchers
            .iter()
            .map(|entry| {
                (
                    entry.key().clone(),
                    entry.value().pending_size(),
                    entry.value().time_since_last_flush_ms(),
                )
            })
            .collect()
    }

    /// Remove client batcher
    pub fn remove_client(&self, client_id: &str) {
        self.batchers.remove(client_id);
    }
}

impl Default for BatchingManager {
    fn default() -> Self {
        Self::new_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_batch_creation() {
        let batch = EventBatch::new();
        assert!(batch.is_empty());
        assert_eq!(batch.size(), 0);
    }

    #[test]
    fn test_add_event_to_batch() {
        let mut batch = EventBatch::new();
        let event = RealtimeEvent::Heartbeat {
            timestamp: "2025-11-10T00:00:00Z".to_string(),
        };

        batch.add_event(event);
        assert_eq!(batch.size(), 1);
    }

    #[test]
    fn test_client_batcher() {
        let batcher = ClientBatcher::new(3, 100);

        let event = RealtimeEvent::Heartbeat {
            timestamp: "2025-11-10T00:00:00Z".to_string(),
        };

        assert!(batcher.add_event(event.clone()).is_none());
        assert!(batcher.add_event(event.clone()).is_none());
        let flushed = batcher.add_event(event);
        assert!(flushed.is_some());
    }

    #[test]
    fn test_batcher_pending_size() {
        let batcher = ClientBatcher::new_default();

        let event = RealtimeEvent::Heartbeat {
            timestamp: "2025-11-10T00:00:00Z".to_string(),
        };

        batcher.add_event(event);
        assert_eq!(batcher.pending_size(), 1);
    }

    #[test]
    fn test_batching_manager() {
        let manager = BatchingManager::new_default();

        let event = RealtimeEvent::Heartbeat {
            timestamp: "2025-11-10T00:00:00Z".to_string(),
        };

        manager.add_event("client1", event);
        let batcher = manager.get_batcher("client1");
        assert_eq!(batcher.pending_size(), 1);
    }

    #[test]
    fn test_flush_batch() {
        let batcher = ClientBatcher::new(10, 100);

        let event = RealtimeEvent::Heartbeat {
            timestamp: "2025-11-10T00:00:00Z".to_string(),
        };

        batcher.add_event(event);
        let flushed = batcher.flush();
        assert!(flushed.is_some());
        assert_eq!(batcher.pending_size(), 0);
    }
}
