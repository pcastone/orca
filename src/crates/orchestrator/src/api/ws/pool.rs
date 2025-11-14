//! WebSocket connection pool management
//!
//! Manages active WebSocket connections with thread-safe operations and resource tracking.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Connection pool entry
#[derive(Debug, Clone)]
pub struct PoolEntry {
    /// Unique client ID
    pub client_id: String,
    /// Connection creation timestamp
    pub connected_at: i64,
    /// Last activity timestamp
    pub last_activity: Arc<AtomicU64>,
    /// Message count sent to this client
    pub messages_sent: Arc<AtomicU64>,
}

/// WebSocket connection pool
pub struct ConnectionPool {
    /// Active connections indexed by client_id
    connections: Arc<DashMap<String, PoolEntry>>,
    /// Maximum concurrent connections (default: 1000)
    max_connections: usize,
    /// Total connections ever created
    total_created: Arc<AtomicU64>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            max_connections,
            total_created: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a new connection pool with default settings (1000 max)
    pub fn new_default() -> Self {
        Self::new(1000)
    }

    /// Add a new connection to the pool
    pub fn connect(&self) -> Result<String, String> {
        if self.connections.len() >= self.max_connections {
            return Err(format!(
                "Connection limit reached: {} connections",
                self.max_connections
            ));
        }

        let client_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        let entry = PoolEntry {
            client_id: client_id.clone(),
            connected_at: now,
            last_activity: Arc::new(AtomicU64::new(now as u64)),
            messages_sent: Arc::new(AtomicU64::new(0)),
        };

        self.connections.insert(client_id.clone(), entry);
        self.total_created.fetch_add(1, Ordering::Relaxed);

        Ok(client_id)
    }

    /// Remove a connection from the pool
    pub fn disconnect(&self, client_id: &str) -> Option<PoolEntry> {
        self.connections.remove(client_id).map(|(_, entry)| entry)
    }

    /// Record activity for a connection
    pub fn record_activity(&self, client_id: &str) {
        if let Some(entry) = self.connections.get(client_id) {
            let now = chrono::Utc::now().timestamp() as u64;
            entry.last_activity.store(now, Ordering::Relaxed);
        }
    }

    /// Increment message count for a connection
    pub fn increment_message_count(&self, client_id: &str) {
        if let Some(entry) = self.connections.get(client_id) {
            entry.messages_sent.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get connection info
    pub fn get_connection(&self, client_id: &str) -> Option<PoolEntry> {
        self.connections.get(client_id).map(|entry| entry.clone())
    }

    /// Check if a connection exists
    pub fn exists(&self, client_id: &str) -> bool {
        self.connections.contains_key(client_id)
    }

    /// Get active connection count
    pub fn active_connections(&self) -> usize {
        self.connections.len()
    }

    /// Get total connections ever created
    pub fn total_connections(&self) -> u64 {
        self.total_created.load(Ordering::Relaxed)
    }

    /// Get all active client IDs
    pub fn get_all_clients(&self) -> Vec<String> {
        self.connections
            .iter()
            .map(|entry| entry.client_id.clone())
            .collect()
    }

    /// Remove stale connections (no activity for timeout_secs)
    pub fn cleanup_stale(&self, timeout_secs: i64) -> Vec<String> {
        let now = chrono::Utc::now().timestamp();
        let mut removed = Vec::new();

        self.connections.retain(|client_id, entry| {
            let last_activity = entry.last_activity.load(Ordering::Relaxed) as i64;
            if now - last_activity > timeout_secs {
                removed.push(client_id.clone());
                false
            } else {
                true
            }
        });

        removed
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            active_connections: self.connections.len(),
            max_connections: self.max_connections,
            total_created: self.total_created.load(Ordering::Relaxed),
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoolStats {
    /// Currently active connections
    pub active_connections: usize,
    /// Maximum allowed connections
    pub max_connections: usize,
    /// Total connections created since startup
    pub total_created: u64,
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = ConnectionPool::new(10);
        assert_eq!(pool.active_connections(), 0);
        assert_eq!(pool.total_connections(), 0);
    }

    #[test]
    fn test_connect() {
        let pool = ConnectionPool::new(10);
        let client_id = pool.connect().unwrap();
        assert!(pool.exists(&client_id));
        assert_eq!(pool.active_connections(), 1);
        assert_eq!(pool.total_connections(), 1);
    }

    #[test]
    fn test_disconnect() {
        let pool = ConnectionPool::new(10);
        let client_id = pool.connect().unwrap();
        assert_eq!(pool.active_connections(), 1);

        pool.disconnect(&client_id);
        assert_eq!(pool.active_connections(), 0);
    }

    #[test]
    fn test_max_connections() {
        let pool = ConnectionPool::new(2);
        let _c1 = pool.connect().unwrap();
        let _c2 = pool.connect().unwrap();

        assert!(pool.connect().is_err());
        assert_eq!(pool.active_connections(), 2);
    }

    #[test]
    fn test_record_activity() {
        let pool = ConnectionPool::new(10);
        let client_id = pool.connect().unwrap();

        pool.record_activity(&client_id);
        let entry = pool.get_connection(&client_id).unwrap();
        assert!(entry.last_activity.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_cleanup_stale() {
        let pool = ConnectionPool::new(10);
        let c1 = pool.connect().unwrap();
        let c2 = pool.connect().unwrap();

        // Manually set old timestamp for c1
        if let Some(entry) = pool.connections.get(&c1) {
            entry.last_activity.store(0, Ordering::Relaxed);
        }

        let removed = pool.cleanup_stale(100);
        assert_eq!(removed.len(), 1);
        assert_eq!(pool.active_connections(), 1);
        assert!(pool.exists(&c2));
    }

    #[test]
    fn test_stats() {
        let pool = ConnectionPool::new(10);
        let _c1 = pool.connect().unwrap();
        let _c2 = pool.connect().unwrap();

        let stats = pool.stats();
        assert_eq!(stats.active_connections, 2);
        assert_eq!(stats.max_connections, 10);
        assert_eq!(stats.total_created, 2);
    }
}
