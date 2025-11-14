//! WebSocket rate limiting
//!
//! Implements token bucket algorithm for per-client rate limiting.

use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current token count
    tokens: Arc<AtomicU64>,
    /// Last refill timestamp (in milliseconds)
    last_refill: Arc<AtomicU64>,
    /// Capacity (tokens)
    capacity: u64,
    /// Refill rate (tokens per second)
    refill_rate: u64,
}

impl TokenBucket {
    /// Create new token bucket
    fn new(capacity: u64, refill_rate: u64) -> Self {
        let now_millis = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            tokens: Arc::new(AtomicU64::new(capacity)),
            last_refill: Arc::new(AtomicU64::new(now_millis)),
            capacity,
            refill_rate,
        }
    }

    /// Try to consume a token
    fn try_consume(&self, amount: u64) -> bool {
        let now_millis = chrono::Utc::now().timestamp_millis() as u64;
        let last_refill = self.last_refill.load(Ordering::Relaxed);

        // Calculate tokens to add
        let elapsed_ms = now_millis.saturating_sub(last_refill);
        let tokens_to_add = (elapsed_ms * self.refill_rate) / 1000;

        // Refill tokens up to capacity
        let current = self.tokens.load(Ordering::Relaxed);
        let new_tokens = std::cmp::min(current + tokens_to_add, self.capacity);

        // Update refill time
        self.last_refill.store(now_millis, Ordering::Relaxed);

        // Try to consume
        if new_tokens >= amount {
            self.tokens.store(new_tokens - amount, Ordering::Relaxed);
            true
        } else {
            self.tokens.store(new_tokens, Ordering::Relaxed);
            false
        }
    }

    /// Get current token count
    fn tokens(&self) -> u64 {
        self.tokens.load(Ordering::Relaxed)
    }
}

/// Rate limiter for WebSocket connections
pub struct RateLimiter {
    /// Token buckets per client
    buckets: Arc<DashMap<String, TokenBucket>>,
    /// Default capacity (tokens)
    capacity: u64,
    /// Default refill rate (tokens per second)
    refill_rate: u64,
    /// Violation count per client
    violations: Arc<DashMap<String, u64>>,
}

impl RateLimiter {
    /// Create new rate limiter
    ///
    /// # Arguments
    /// * `messages_per_second` - Maximum messages per second per client
    pub fn new(messages_per_second: u64) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            capacity: messages_per_second * 10, // Allow burst of 10 seconds worth
            refill_rate: messages_per_second,
            violations: Arc::new(DashMap::new()),
        }
    }

    /// Create with 100 messages/second limit
    pub fn default_limit() -> Self {
        Self::new(100)
    }

    /// Check if a message is allowed
    pub fn allow_message(&self, client_id: &str) -> bool {
        let bucket = self
            .buckets
            .entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));

        bucket.try_consume(1)
    }

    /// Check if multiple messages are allowed
    pub fn allow_messages(&self, client_id: &str, count: u64) -> bool {
        let bucket = self
            .buckets
            .entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));

        bucket.try_consume(count)
    }

    /// Record a rate limit violation
    pub fn record_violation(&self, client_id: &str) {
        self.violations
            .entry(client_id.to_string())
            .and_modify(|v| *v += 1)
            .or_insert(1);
    }

    /// Get violation count for a client
    pub fn get_violations(&self, client_id: &str) -> u64 {
        self.violations
            .get(client_id)
            .map(|v| *v)
            .unwrap_or(0)
    }

    /// Reset violations for a client
    pub fn reset_violations(&self, client_id: &str) {
        self.violations.remove(client_id);
    }

    /// Get token bucket status for a client
    pub fn get_tokens(&self, client_id: &str) -> Option<u64> {
        self.buckets.get(client_id).map(|b| b.tokens())
    }

    /// Get all clients with violations
    pub fn get_violating_clients(&self) -> Vec<(String, u64)> {
        self.violations
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect()
    }

    /// Clear all rate limit state for a client
    pub fn clear_client(&self, client_id: &str) {
        self.buckets.remove(client_id);
        self.violations.remove(client_id);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::default_limit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(100);
        assert!(limiter.allow_message("client1"));
    }

    #[test]
    fn test_rate_limit_enforcement() {
        let limiter = RateLimiter::new(1); // 1 msg/sec
        let client = "test_client";

        // Should allow first message
        assert!(limiter.allow_message(client));

        // Should block subsequent messages
        for _ in 0..10 {
            if !limiter.allow_message(client) {
                break;
            }
        }

        // At least one should fail
        let mut blocked = false;
        for _ in 0..100 {
            if !limiter.allow_message(client) {
                blocked = true;
                break;
            }
        }
        assert!(blocked);
    }

    #[test]
    fn test_multiple_clients() {
        let limiter = RateLimiter::new(1);

        let c1 = "client1";
        let c2 = "client2";

        assert!(limiter.allow_message(c1));
        assert!(limiter.allow_message(c2));

        // Each client has independent limit
        let block_c1 = (0..100).any(|_| !limiter.allow_message(c1));
        let block_c2 = (0..100).any(|_| !limiter.allow_message(c2));

        assert!(block_c1);
        assert!(block_c2);
    }

    #[test]
    fn test_violations() {
        let limiter = RateLimiter::new(1);
        let client = "test";

        assert_eq!(limiter.get_violations(client), 0);

        limiter.record_violation(client);
        limiter.record_violation(client);

        assert_eq!(limiter.get_violations(client), 2);

        limiter.reset_violations(client);
        assert_eq!(limiter.get_violations(client), 0);
    }

    #[test]
    fn test_bulk_messages() {
        let limiter = RateLimiter::new(100);
        let client = "test";

        // Should allow up to capacity
        assert!(limiter.allow_messages(client, 50));

        // Eventually should fail with enough attempts
        let mut blocked = false;
        for _ in 0..1000 {
            if !limiter.allow_messages(client, 100) {
                blocked = true;
                break;
            }
        }
        assert!(blocked);
    }

    #[test]
    fn test_clear_client() {
        let limiter = RateLimiter::new(1);
        let client = "test";

        limiter.allow_message(client);
        limiter.record_violation(client);

        assert_eq!(limiter.get_violations(client), 1);

        limiter.clear_client(client);
        assert_eq!(limiter.get_violations(client), 0);
    }
}
