//! WebSocket metrics tracking
//!
//! Tracks WebSocket connection and message metrics for monitoring and diagnostics.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// WebSocket metrics
#[derive(Clone)]
pub struct WebSocketMetrics {
    /// Total connections established
    total_connections: Arc<AtomicU64>,
    /// Currently active connections
    active_connections: Arc<AtomicU64>,
    /// Total messages sent
    messages_sent: Arc<AtomicU64>,
    /// Total messages received
    messages_received: Arc<AtomicU64>,
    /// Total errors occurred
    error_count: Arc<AtomicU64>,
    /// Total bytes sent
    bytes_sent: Arc<AtomicU64>,
    /// Total bytes received
    bytes_received: Arc<AtomicU64>,
}

impl WebSocketMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            total_connections: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU64::new(0)),
            messages_sent: Arc::new(AtomicU64::new(0)),
            messages_received: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a new connection
    pub fn record_connection(&self) {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a disconnection
    pub fn record_disconnection(&self) {
        let current = self.active_connections.load(Ordering::Relaxed);
        if current > 0 {
            self.active_connections.store(current - 1, Ordering::Relaxed);
        }
    }

    /// Record a sent message
    pub fn record_message_sent(&self, bytes: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a received message
    pub fn record_message_received(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record an error
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_connections: self.total_connections.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
        }
    }
}

impl Default for WebSocketMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics snapshot for a point in time
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    /// Total connections established
    pub total_connections: u64,
    /// Currently active connections
    pub active_connections: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total errors
    pub error_count: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
}

impl MetricsSnapshot {
    /// Calculate average message size sent
    pub fn avg_message_size_sent(&self) -> f64 {
        if self.messages_sent > 0 {
            self.bytes_sent as f64 / self.messages_sent as f64
        } else {
            0.0
        }
    }

    /// Calculate average message size received
    pub fn avg_message_size_received(&self) -> f64 {
        if self.messages_received > 0 {
            self.bytes_received as f64 / self.messages_received as f64
        } else {
            0.0
        }
    }

    /// Calculate total throughput in bytes
    pub fn total_bytes(&self) -> u64 {
        self.bytes_sent + self.bytes_received
    }

    /// Calculate total messages
    pub fn total_messages(&self) -> u64 {
        self.messages_sent + self.messages_received
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = WebSocketMetrics::new();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_connections, 0);
        assert_eq!(snapshot.active_connections, 0);
    }

    #[test]
    fn test_record_connection() {
        let metrics = WebSocketMetrics::new();
        metrics.record_connection();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_connections, 1);
        assert_eq!(snapshot.active_connections, 1);
    }

    #[test]
    fn test_record_disconnection() {
        let metrics = WebSocketMetrics::new();
        metrics.record_connection();
        metrics.record_disconnection();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.active_connections, 0);
    }

    #[test]
    fn test_record_message() {
        let metrics = WebSocketMetrics::new();
        metrics.record_message_sent(100);
        metrics.record_message_received(50);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.messages_sent, 1);
        assert_eq!(snapshot.messages_received, 1);
        assert_eq!(snapshot.bytes_sent, 100);
        assert_eq!(snapshot.bytes_received, 50);
    }

    #[test]
    fn test_metrics_snapshot_calculations() {
        let snapshot = MetricsSnapshot {
            total_connections: 10,
            active_connections: 5,
            messages_sent: 100,
            messages_received: 80,
            error_count: 2,
            bytes_sent: 10000,
            bytes_received: 8000,
        };

        assert_eq!(snapshot.total_bytes(), 18000);
        assert_eq!(snapshot.total_messages(), 180);
        assert!((snapshot.avg_message_size_sent() - 100.0).abs() < 0.01);
        assert!((snapshot.avg_message_size_received() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_record_error() {
        let metrics = WebSocketMetrics::new();
        metrics.record_error();
        metrics.record_error();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.error_count, 2);
    }
}
