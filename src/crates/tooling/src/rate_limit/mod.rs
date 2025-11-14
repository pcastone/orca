//! Rate limiting utilities
//!
//! Provides simple rate limiting for controlling operation frequency.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Simple token bucket rate limiter
///
/// Implements the token bucket algorithm for rate limiting.
/// Tokens are added at a constant rate up to a maximum capacity.
///
/// # Example
///
/// ```rust,ignore
/// use tooling::rate_limit::RateLimiter;
/// use std::time::Duration;
///
/// // Allow 10 operations per second
/// let limiter = RateLimiter::new(10, Duration::from_secs(1));
///
/// // Check if operation is allowed
/// if limiter.check().await {
///     // Perform operation
/// } else {
///     // Rate limited
/// }
/// ```
#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<RateLimiterState>>,
}

struct RateLimiterState {
    /// Maximum number of tokens
    capacity: usize,

    /// Current number of tokens
    tokens: f64,

    /// Time period for refill
    refill_period: Duration,

    /// Last refill time
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    ///
    /// * `max_operations` - Maximum number of operations allowed
    /// * `period` - Time period for the limit
    ///
    /// # Example
    ///
    /// ```rust
    /// use tooling::rate_limit::RateLimiter;
    /// use std::time::Duration;
    ///
    /// // 100 requests per minute
    /// let limiter = RateLimiter::new(100, Duration::from_secs(60));
    /// ```
    pub fn new(max_operations: usize, period: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimiterState {
                capacity: max_operations,
                tokens: max_operations as f64,
                refill_period: period,
                last_refill: Instant::now(),
            })),
        }
    }

    /// Check if an operation is allowed (non-blocking)
    ///
    /// # Returns
    ///
    /// `true` if operation is allowed, `false` if rate limited
    pub async fn check(&self) -> bool {
        let mut state = self.state.lock().await;
        state.refill();

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Wait until an operation is allowed (blocking)
    ///
    /// This method will sleep until a token is available.
    pub async fn acquire(&self) {
        loop {
            if self.check().await {
                return;
            }

            // Sleep for a short duration before checking again
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Check remaining capacity
    ///
    /// # Returns
    ///
    /// Number of operations that can be performed immediately
    pub async fn available(&self) -> usize {
        let mut state = self.state.lock().await;
        state.refill();
        state.tokens.floor() as usize
    }

    /// Reset the rate limiter
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        state.tokens = state.capacity as f64;
        state.last_refill = Instant::now();
    }
}

impl RateLimiterState {
    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);

        if elapsed >= self.refill_period {
            // Full refill
            self.tokens = self.capacity as f64;
            self.last_refill = now;
        } else {
            // Partial refill based on time elapsed
            let refill_ratio = elapsed.as_secs_f64() / self.refill_period.as_secs_f64();
            let tokens_to_add = (self.capacity as f64) * refill_ratio;
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity as f64);

            if tokens_to_add > 0.0 {
                self.last_refill = now;
            }
        }
    }
}

/// Sliding window rate limiter
///
/// Tracks operations in a sliding time window.
///
/// # Example
///
/// ```rust,ignore
/// use tooling::rate_limit::SlidingWindowLimiter;
/// use std::time::Duration;
///
/// // Allow 100 operations per minute
/// let limiter = SlidingWindowLimiter::new(100, Duration::from_secs(60));
///
/// if limiter.check().await {
///     // Operation allowed
/// }
/// ```
#[derive(Clone)]
pub struct SlidingWindowLimiter {
    state: Arc<Mutex<SlidingWindowState>>,
}

struct SlidingWindowState {
    /// Maximum operations in window
    max_operations: usize,

    /// Window duration
    window: Duration,

    /// Timestamps of recent operations
    operations: Vec<Instant>,
}

impl SlidingWindowLimiter {
    /// Create a new sliding window rate limiter
    ///
    /// # Arguments
    ///
    /// * `max_operations` - Maximum operations in window
    /// * `window` - Time window duration
    pub fn new(max_operations: usize, window: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(SlidingWindowState {
                max_operations,
                window,
                operations: Vec::new(),
            })),
        }
    }

    /// Check if an operation is allowed
    ///
    /// # Returns
    ///
    /// `true` if operation is allowed, `false` if rate limited
    pub async fn check(&self) -> bool {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let window = state.window;

        // Remove operations outside the window
        state
            .operations
            .retain(|&time| now.duration_since(time) < window);

        if state.operations.len() < state.max_operations {
            state.operations.push(now);
            true
        } else {
            false
        }
    }

    /// Get count of operations in current window
    pub async fn count(&self) -> usize {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let window = state.window;

        state
            .operations
            .retain(|&time| now.duration_since(time) < window);
        state.operations.len()
    }

    /// Reset the limiter
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        state.operations.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_operations() {
        let limiter = RateLimiter::new(5, Duration::from_secs(1));

        // Should allow 5 operations
        for _ in 0..5 {
            assert!(limiter.check().await);
        }

        // 6th operation should be denied
        assert!(!limiter.check().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_refills() {
        let limiter = RateLimiter::new(2, Duration::from_millis(100));

        // Use up tokens
        assert!(limiter.check().await);
        assert!(limiter.check().await);
        assert!(!limiter.check().await);

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(110)).await;

        // Should have new tokens
        assert!(limiter.check().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_available() {
        let limiter = RateLimiter::new(5, Duration::from_secs(1));

        assert_eq!(limiter.available().await, 5);

        limiter.check().await;
        assert_eq!(limiter.available().await, 4);

        limiter.check().await;
        limiter.check().await;
        assert_eq!(limiter.available().await, 2);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));

        limiter.check().await;
        limiter.check().await;
        limiter.check().await;

        assert_eq!(limiter.available().await, 0);

        limiter.reset().await;
        assert_eq!(limiter.available().await, 3);
    }

    #[tokio::test]
    async fn test_sliding_window_limiter() {
        let limiter = SlidingWindowLimiter::new(3, Duration::from_millis(100));

        // Should allow 3 operations
        assert!(limiter.check().await);
        assert!(limiter.check().await);
        assert!(limiter.check().await);

        // 4th should be denied
        assert!(!limiter.check().await);

        assert_eq!(limiter.count().await, 3);
    }

    #[tokio::test]
    async fn test_sliding_window_expires() {
        let limiter = SlidingWindowLimiter::new(2, Duration::from_millis(50));

        limiter.check().await;
        limiter.check().await;

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should allow new operations
        assert!(limiter.check().await);
        assert_eq!(limiter.count().await, 1);
    }

    #[tokio::test]
    async fn test_sliding_window_reset() {
        let limiter = SlidingWindowLimiter::new(3, Duration::from_secs(1));

        limiter.check().await;
        limiter.check().await;

        assert_eq!(limiter.count().await, 2);

        limiter.reset().await;
        assert_eq!(limiter.count().await, 0);
    }

    #[tokio::test]
    async fn test_rate_limiter_acquire() {
        let limiter = RateLimiter::new(1, Duration::from_millis(50));

        // Use up token
        limiter.check().await;

        let start = Instant::now();
        limiter.acquire().await;
        let elapsed = start.elapsed();

        // Should have waited for refill
        assert!(elapsed >= Duration::from_millis(40));
    }
}
