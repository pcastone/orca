//! Real-time statistics API handler
//!
//! Provides endpoints for monitoring real-time system metrics.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::api::ws::{
    ConnectionPool, WebSocketMetrics, RateLimiter, BackpressureManager,
    TimeoutManager, FilterManager, BatchingManager, EventHistory,
};

/// Real-time statistics
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RealtimeStats {
    /// Connection pool statistics
    pub connections: serde_json::Value,
    /// WebSocket metrics
    pub metrics: serde_json::Value,
    /// Rate limit violations
    pub rate_limit_violations: usize,
    /// Clients with backpressure
    pub backpressure_clients: usize,
    /// Average queue depth
    pub avg_queue_depth: f64,
    /// Event history size
    pub event_history_size: usize,
    /// Timestamp
    pub timestamp: String,
}

/// Get real-time statistics
///
/// GET /api/v1/realtime/stats
pub async fn get_realtime_stats(
    State(_pool): State<Arc<ConnectionPool>>,
    State(_metrics): State<Arc<WebSocketMetrics>>,
    State(_limiter): State<Arc<RateLimiter>>,
    State(_backpressure): State<Arc<BackpressureManager>>,
    State(_timeout): State<Arc<TimeoutManager>>,
    State(_filters): State<Arc<FilterManager>>,
    State(_batching): State<Arc<BatchingManager>>,
    State(_history): State<Arc<EventHistory>>,
) -> (StatusCode, Json<Value>) {
    let pool_stats = _pool.stats();
    let metrics_snapshot = _metrics.snapshot();
    let backpressure_status = _backpressure.get_all_queue_status();
    let history_size = _history.size();

    // Calculate average queue depth
    let avg_queue_depth = if !backpressure_status.is_empty() {
        let total: usize = backpressure_status.iter().map(|s| s.queue_size).sum();
        total as f64 / backpressure_status.len() as f64
    } else {
        0.0
    };

    let response = json!({
        "status": "ok",
        "data": {
            "connections": {
                "active": pool_stats.active_connections,
                "total": pool_stats.total_created,
                "max": pool_stats.max_connections,
            },
            "metrics": {
                "messages_sent": metrics_snapshot.messages_sent,
                "messages_received": metrics_snapshot.messages_received,
                "bytes_sent": metrics_snapshot.bytes_sent,
                "bytes_received": metrics_snapshot.bytes_received,
                "errors": metrics_snapshot.error_count,
            },
            "rate_limit_violations": _limiter.get_violating_clients().len(),
            "backpressure_clients": backpressure_status.len(),
            "avg_queue_depth": avg_queue_depth,
            "event_history_size": history_size,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }
    });

    (StatusCode::OK, Json(response))
}

/// Connection status
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ConnectionStatus {
    /// Total active connections
    pub active_connections: usize,
    /// Maximum allowed connections
    pub max_connections: usize,
    /// Connections with pending messages
    pub backpressure_count: usize,
    /// Average message age in queue (milliseconds)
    pub avg_queue_age_ms: u64,
}

/// Get connection status
///
/// GET /api/v1/realtime/connections
pub async fn get_connection_status(
    State(pool): State<Arc<ConnectionPool>>,
    State(backpressure): State<Arc<BackpressureManager>>,
) -> (StatusCode, Json<Value>) {
    let pool_stats = pool.stats();
    let backpressure_status = backpressure.get_all_queue_status();

    let response = json!({
        "status": "ok",
        "data": {
            "active_connections": pool_stats.active_connections,
            "max_connections": pool_stats.max_connections,
            "backpressure_count": backpressure_status.iter().filter(|s| s.queue_size > 0).count(),
            "avg_queue_age_ms": 0,
        }
    });

    (StatusCode::OK, Json(response))
}

/// Performance metrics
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct PerformanceMetrics {
    /// Messages per second
    pub messages_per_second: f64,
    /// Average message size in bytes
    pub avg_message_size: f64,
    /// Error rate (percent)
    pub error_rate: f64,
    /// Compression savings in bytes
    pub compression_savings: u64,
}

/// Get performance metrics
///
/// GET /api/v1/realtime/performance
pub async fn get_performance_metrics(
    State(metrics): State<Arc<WebSocketMetrics>>,
) -> (StatusCode, Json<Value>) {
    let snapshot = metrics.snapshot();

    let total_messages = snapshot.total_messages();
    let messages_per_second = if total_messages > 0 {
        total_messages as f64 / 60.0 // Approximate per second
    } else {
        0.0
    };

    let avg_message_size = if total_messages > 0 {
        snapshot.total_bytes() as f64 / total_messages as f64
    } else {
        0.0
    };

    let error_rate = if total_messages > 0 {
        (snapshot.error_count as f64 / total_messages as f64) * 100.0
    } else {
        0.0
    };

    let response = json!({
        "status": "ok",
        "data": {
            "messages_per_second": messages_per_second,
            "avg_message_size": avg_message_size,
            "error_rate": error_rate,
            "compression_savings": 0,
        }
    });

    (StatusCode::OK, Json(response))
}
