//! WebSocket connection timeout management
//!
//! Handles connection timeouts and heartbeat monitoring.

use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Timeout configuration
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Idle timeout in seconds (default: 120)
    pub idle_timeout_secs: u64,
    /// Heartbeat interval in seconds (default: 30)
    pub heartbeat_interval_secs: u64,
    /// Max allowed time without heartbeat response (default: 60)
    pub heartbeat_timeout_secs: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 120,
            heartbeat_interval_secs: 30,
            heartbeat_timeout_secs: 60,
        }
    }
}

/// Client timeout tracker
#[derive(Debug, Clone)]
pub struct ClientTimeout {
    /// Client ID
    pub client_id: String,
    /// Last activity timestamp (seconds)
    pub last_activity: Arc<AtomicU64>,
    /// Last heartbeat sent timestamp (seconds)
    pub last_heartbeat_sent: Arc<AtomicU64>,
    /// Last heartbeat received timestamp (seconds)
    pub last_heartbeat_received: Arc<AtomicU64>,
    /// Connected at timestamp (seconds)
    pub connected_at: u64,
}

impl ClientTimeout {
    /// Create new client timeout tracker
    pub fn new(client_id: String) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            client_id,
            last_activity: Arc::new(AtomicU64::new(now)),
            last_heartbeat_sent: Arc::new(AtomicU64::new(now)),
            last_heartbeat_received: Arc::new(AtomicU64::new(now)),
            connected_at: now,
        }
    }

    /// Update last activity timestamp
    pub fn record_activity(&self) {
        let now = chrono::Utc::now().timestamp() as u64;
        self.last_activity.store(now, Ordering::Relaxed);
    }

    /// Record heartbeat sent
    pub fn record_heartbeat_sent(&self) {
        let now = chrono::Utc::now().timestamp() as u64;
        self.last_heartbeat_sent.store(now, Ordering::Relaxed);
    }

    /// Record heartbeat received
    pub fn record_heartbeat_received(&self) {
        let now = chrono::Utc::now().timestamp() as u64;
        self.last_heartbeat_received.store(now, Ordering::Relaxed);
    }

    /// Get idle time in seconds
    pub fn idle_seconds(&self) -> u64 {
        let now = chrono::Utc::now().timestamp() as u64;
        let last = self.last_activity.load(Ordering::Relaxed);
        now.saturating_sub(last)
    }

    /// Check if connection is idle
    pub fn is_idle(&self, timeout_secs: u64) -> bool {
        self.idle_seconds() > timeout_secs
    }

    /// Check if heartbeat is missing
    pub fn is_heartbeat_missing(&self, timeout_secs: u64) -> bool {
        let now = chrono::Utc::now().timestamp() as u64;
        let last_received = self.last_heartbeat_received.load(Ordering::Relaxed);
        now.saturating_sub(last_received) > timeout_secs
    }

    /// Get connection uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        let now = chrono::Utc::now().timestamp() as u64;
        now.saturating_sub(self.connected_at)
    }

    /// Check if should send heartbeat
    pub fn should_send_heartbeat(&self, interval_secs: u64) -> bool {
        let now = chrono::Utc::now().timestamp() as u64;
        let last_sent = self.last_heartbeat_sent.load(Ordering::Relaxed);
        now.saturating_sub(last_sent) >= interval_secs
    }
}

/// Global timeout manager
pub struct TimeoutManager {
    /// Per-client timeout trackers
    clients: Arc<DashMap<String, ClientTimeout>>,
    /// Configuration
    config: TimeoutConfig,
}

impl TimeoutManager {
    /// Create new timeout manager
    pub fn new(config: TimeoutConfig) -> Self {
        Self {
            clients: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Create with default configuration
    pub fn new_default() -> Self {
        Self::new(TimeoutConfig::default())
    }

    /// Register a new client
    pub fn register_client(&self, client_id: String) -> ClientTimeout {
        let timeout = ClientTimeout::new(client_id.clone());
        self.clients.insert(client_id, timeout.clone());
        timeout
    }

    /// Unregister a client
    pub fn unregister_client(&self, client_id: &str) {
        self.clients.remove(client_id);
    }

    /// Get timeout tracker for a client
    pub fn get_client(&self, client_id: &str) -> Option<ClientTimeout> {
        self.clients.get(client_id).map(|entry| entry.clone())
    }

    /// Record activity for a client
    pub fn record_activity(&self, client_id: &str) {
        if let Some(client) = self.clients.get(client_id) {
            client.record_activity();
        }
    }

    /// Check for idle connections
    pub fn get_idle_clients(&self) -> Vec<String> {
        self.clients
            .iter()
            .filter(|entry| entry.value().is_idle(self.config.idle_timeout_secs))
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Check for missing heartbeats
    pub fn get_missing_heartbeat_clients(&self) -> Vec<String> {
        self.clients
            .iter()
            .filter(|entry| entry.value().is_heartbeat_missing(self.config.heartbeat_timeout_secs))
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get clients that need heartbeat
    pub fn get_heartbeat_needed_clients(&self) -> Vec<String> {
        self.clients
            .iter()
            .filter(|entry| entry.value().should_send_heartbeat(self.config.heartbeat_interval_secs))
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Record heartbeat sent for a client
    pub fn record_heartbeat_sent(&self, client_id: &str) {
        if let Some(client) = self.clients.get(client_id) {
            client.record_heartbeat_sent();
        }
    }

    /// Record heartbeat received for a client
    pub fn record_heartbeat_received(&self, client_id: &str) {
        if let Some(client) = self.clients.get(client_id) {
            client.record_heartbeat_received();
        }
    }

    /// Get timeout configuration
    pub fn config(&self) -> &TimeoutConfig {
        &self.config
    }

    /// Get all clients
    pub fn get_all_clients(&self) -> Vec<ClientTimeout> {
        self.clients
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Cleanup all resources
    pub fn cleanup(&self) {
        self.clients.clear();
    }
}

impl Default for TimeoutManager {
    fn default() -> Self {
        Self::new_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_client_timeout_creation() {
        let timeout = ClientTimeout::new("client1".to_string());
        assert_eq!(timeout.idle_seconds(), 0);
    }

    #[test]
    fn test_record_activity() {
        let timeout = ClientTimeout::new("client1".to_string());
        timeout.record_activity();
        assert_eq!(timeout.idle_seconds(), 0);
    }

    #[test]
    fn test_idle_check() {
        let timeout = ClientTimeout::new("client1".to_string());
        // Manually set old timestamp
        timeout.last_activity.store(0, Ordering::Relaxed);
        assert!(timeout.is_idle(1));
    }

    #[test]
    fn test_heartbeat_tracking() {
        let timeout = ClientTimeout::new("client1".to_string());
        timeout.record_heartbeat_sent();
        timeout.record_heartbeat_received();

        let sent = timeout.last_heartbeat_sent.load(Ordering::Relaxed);
        let received = timeout.last_heartbeat_received.load(Ordering::Relaxed);

        assert!(sent > 0);
        assert!(received > 0);
    }

    #[test]
    fn test_timeout_manager_register() {
        let manager = TimeoutManager::new_default();
        let timeout = manager.register_client("client1".to_string());
        assert!(manager.get_client("client1").is_some());
        assert_eq!(timeout.client_id, "client1");
    }

    #[test]
    fn test_timeout_manager_unregister() {
        let manager = TimeoutManager::new_default();
        manager.register_client("client1".to_string());
        assert!(manager.get_client("client1").is_some());

        manager.unregister_client("client1");
        assert!(manager.get_client("client1").is_none());
    }

    #[test]
    fn test_uptime_calculation() {
        let timeout = ClientTimeout::new("client1".to_string());
        let uptime = timeout.uptime_seconds();
        assert_eq!(uptime, 0);
    }

    #[test]
    fn test_should_send_heartbeat() {
        let timeout = ClientTimeout::new("client1".to_string());
        // Immediately should need heartbeat (interval exceeded)
        assert!(timeout.should_send_heartbeat(0));
    }
}
