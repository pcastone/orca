//! Retry utilities for async operations
//!
//! Provides configurable retry policies with exponential backoff and jitter
//! for handling transient failures in async operations.

use rand::Rng;
use std::future::Future;
use std::time::Duration;

/// Configuration for retrying failed operations
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including the first)
    pub max_attempts: usize,

    /// Initial interval between retries in seconds
    pub initial_interval: f64,

    /// Multiplier for the interval after each retry
    pub backoff_factor: f64,

    /// Maximum interval between retries in seconds
    pub max_interval: f64,

    /// Whether to add random jitter to intervals
    pub jitter: bool,
}

impl RetryPolicy {
    /// Create a new retry policy with the given max attempts
    ///
    /// # Arguments
    ///
    /// * `max_attempts` - Maximum number of attempts (including first)
    ///
    /// # Example
    ///
    /// ```rust
    /// use tooling::async_utils::retry::RetryPolicy;
    ///
    /// let policy = RetryPolicy::new(3);
    /// assert_eq!(policy.max_attempts, 3);
    /// ```
    pub fn new(max_attempts: usize) -> Self {
        Self {
            max_attempts,
            initial_interval: 0.5,
            backoff_factor: 2.0,
            max_interval: 128.0,
            jitter: true,
        }
    }

    /// Set the initial interval between retries
    pub fn with_initial_interval(mut self, seconds: f64) -> Self {
        self.initial_interval = seconds;
        self
    }

    /// Set the backoff factor
    pub fn with_backoff_factor(mut self, factor: f64) -> Self {
        self.backoff_factor = factor;
        self
    }

    /// Set the maximum interval between retries
    pub fn with_max_interval(mut self, seconds: f64) -> Self {
        self.max_interval = seconds;
        self
    }

    /// Enable or disable jitter
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Calculate the delay for a given attempt number (0-indexed)
    ///
    /// Uses exponential backoff: initial_interval * (backoff_factor ^ attempt)
    /// Capped at max_interval, with optional jitter.
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        if attempt >= self.max_attempts {
            return Duration::from_secs(0);
        }

        // Calculate base delay with exponential backoff
        let base_delay = self.initial_interval * self.backoff_factor.powi(attempt as i32);

        // Cap at max_interval
        let capped_delay = base_delay.min(self.max_interval);

        // Add jitter if enabled (random factor between 0.5 and 1.5)
        let final_delay = if self.jitter {
            let mut rng = rand::thread_rng();
            let jitter_factor = rng.gen_range(0.5..=1.5);
            capped_delay * jitter_factor
        } else {
            capped_delay
        };

        Duration::from_secs_f64(final_delay)
    }

    /// Check if more retries are allowed
    pub fn should_retry(&self, attempt: usize) -> bool {
        attempt < self.max_attempts
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(3)
    }
}

/// Retry state for tracking retry attempts
#[derive(Debug, Clone)]
pub struct RetryState {
    /// Number of attempts made so far
    pub attempts: usize,

    /// Last error message
    pub last_error: Option<String>,
}

impl RetryState {
    /// Create a new retry state
    pub fn new() -> Self {
        Self {
            attempts: 0,
            last_error: None,
        }
    }

    /// Record an attempt
    pub fn record_attempt(&mut self, error: Option<String>) {
        self.attempts += 1;
        self.last_error = error;
    }

    /// Reset the retry state
    pub fn reset(&mut self) {
        self.attempts = 0;
        self.last_error = None;
    }
}

impl Default for RetryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute an async operation with retry logic
///
/// # Arguments
///
/// * `policy` - Retry policy to use
/// * `operation` - Async operation to execute (must be retryable/idempotent)
///
/// # Returns
///
/// Result of the operation after all retries
///
/// # Example
///
/// ```rust,ignore
/// use tooling::async_utils::retry::{RetryPolicy, with_retry};
///
/// async fn call_api() -> Result<String, String> {
///     // API call that may fail
///     Ok("success".to_string())
/// }
///
/// let policy = RetryPolicy::new(3);
/// let result = with_retry(&policy, || call_api()).await?;
/// ```
pub async fn with_retry<F, Fut, T, E>(
    policy: &RetryPolicy,
    operation: F,
) -> std::result::Result<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = std::result::Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error = None;

    for attempt in 0..policy.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                tracing::debug!(
                    "Attempt {} failed: {}. Retrying...",
                    attempt + 1,
                    error
                );

                last_error = Some(error);

                // Check if we should retry
                if !policy.should_retry(attempt + 1) {
                    break;
                }

                // Calculate and sleep for the delay
                let delay = policy.calculate_delay(attempt);
                tracing::debug!("Waiting {:?} before retry", delay);
                tokio::time::sleep(delay).await;
            }
        }
    }

    // All retries exhausted, return last error
    Err(last_error.expect("Should have error after exhausting retries"))
}

/// Check if an error message indicates a transient error that should be retried
///
/// Recognizes common transient error patterns:
/// - Connection errors
/// - Timeouts
/// - Rate limits
/// - 5xx HTTP status codes
/// - Service unavailable
///
/// # Arguments
///
/// * `error_msg` - Error message to check
///
/// # Returns
///
/// `true` if the error appears to be transient
///
/// # Example
///
/// ```rust
/// use tooling::async_utils::retry::is_retryable_error;
///
/// assert!(is_retryable_error("Connection timeout"));
/// assert!(is_retryable_error("503 Service Unavailable"));
/// assert!(!is_retryable_error("404 Not Found"));
/// ```
pub fn is_retryable_error(error_msg: &str) -> bool {
    let lower = error_msg.to_lowercase();

    lower.contains("timeout")
        || lower.contains("connection")
        || lower.contains("rate limit")
        || lower.contains("503")
        || lower.contains("504")
        || lower.contains("502")
        || lower.contains("500")
        || lower.contains("unavailable")
        || lower.contains("too many requests")
        || lower.contains("retry")
}

/// Extract retry-after duration from error message or headers
///
/// Looks for common patterns like:
/// - "Retry after 30 seconds"
/// - "retry-after: 60"
/// - "Rate limit reset in 120s"
///
/// # Arguments
///
/// * `error_msg` - Error message or header value
///
/// # Returns
///
/// Parsed duration if found, None otherwise
///
/// # Example
///
/// ```rust
/// use tooling::async_utils::retry::extract_retry_after;
/// use std::time::Duration;
///
/// let duration = extract_retry_after("Retry after 30 seconds");
/// assert_eq!(duration, Some(Duration::from_secs(30)));
///
/// let duration = extract_retry_after("retry-after: 60");
/// assert_eq!(duration, Some(Duration::from_secs(60)));
/// ```
pub fn extract_retry_after(error_msg: &str) -> Option<Duration> {
    let lower = error_msg.to_lowercase();

    // Try to find patterns like "retry after 30" or "retry-after: 30"
    if let Some(pos) = lower.find("retry") {
        if let Some(after_pos) = lower[pos..].find("after") {
            let rest = &lower[pos + after_pos + 5..];

            // Extract first number we find
            let num_str: String = rest
                .chars()
                .skip_while(|c| !c.is_numeric())
                .take_while(|c| c.is_numeric())
                .collect();

            if let Ok(seconds) = num_str.parse::<u64>() {
                return Some(Duration::from_secs(seconds));
            }
        }
    }

    // Try pattern like "reset in 120s"
    if let Some(pos) = lower.find("reset in") {
        let rest = &lower[pos + 8..];
        let num_str: String = rest
            .chars()
            .skip_while(|c| !c.is_numeric())
            .take_while(|c| c.is_numeric())
            .collect();

        if let Ok(seconds) = num_str.parse::<u64>() {
            return Some(Duration::from_secs(seconds));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_interval, 0.5);
        assert_eq!(policy.backoff_factor, 2.0);
        assert_eq!(policy.max_interval, 128.0);
        assert!(policy.jitter);
    }

    #[test]
    fn test_retry_policy_builder() {
        let policy = RetryPolicy::new(5)
            .with_initial_interval(1.0)
            .with_backoff_factor(3.0)
            .with_max_interval(60.0)
            .with_jitter(false);

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_interval, 1.0);
        assert_eq!(policy.backoff_factor, 3.0);
        assert_eq!(policy.max_interval, 60.0);
        assert!(!policy.jitter);
    }

    #[test]
    fn test_exponential_backoff() {
        let policy = RetryPolicy::new(5)
            .with_initial_interval(1.0)
            .with_backoff_factor(2.0)
            .with_max_interval(100.0)
            .with_jitter(false);

        // Attempt 0: 1.0 * 2^0 = 1.0
        assert_eq!(policy.calculate_delay(0).as_secs_f64(), 1.0);

        // Attempt 1: 1.0 * 2^1 = 2.0
        assert_eq!(policy.calculate_delay(1).as_secs_f64(), 2.0);

        // Attempt 2: 1.0 * 2^2 = 4.0
        assert_eq!(policy.calculate_delay(2).as_secs_f64(), 4.0);

        // Attempt 3: 1.0 * 2^3 = 8.0
        assert_eq!(policy.calculate_delay(3).as_secs_f64(), 8.0);
    }

    #[test]
    fn test_max_interval_cap() {
        let policy = RetryPolicy::new(10)
            .with_initial_interval(10.0)
            .with_backoff_factor(2.0)
            .with_max_interval(50.0)
            .with_jitter(false);

        // Attempt 5: 10.0 * 2^5 = 320.0, but capped at 50.0
        assert_eq!(policy.calculate_delay(5).as_secs_f64(), 50.0);
    }

    #[test]
    fn test_jitter_adds_randomness() {
        let policy = RetryPolicy::new(5)
            .with_initial_interval(1.0)
            .with_backoff_factor(2.0)
            .with_jitter(true);

        // With jitter, the delay should vary between runs
        let delays: Vec<f64> = (0..10)
            .map(|_| policy.calculate_delay(2).as_secs_f64())
            .collect();

        // Check that not all delays are identical
        let first_delay = delays[0];
        let has_variation = delays.iter().any(|&d| (d - first_delay).abs() > 0.01);
        assert!(has_variation, "Jitter should produce varied delays");

        // Check that delays are within the jitter range (0.5x to 1.5x base)
        let base_delay = 4.0; // 1.0 * 2^2
        for delay in delays {
            assert!(delay >= base_delay * 0.5);
            assert!(delay <= base_delay * 1.5);
        }
    }

    #[test]
    fn test_should_retry() {
        let policy = RetryPolicy::new(3);

        assert!(policy.should_retry(0));
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
        assert!(!policy.should_retry(4));
    }

    #[test]
    fn test_retry_state() {
        let mut state = RetryState::new();

        assert_eq!(state.attempts, 0);
        assert!(state.last_error.is_none());

        state.record_attempt(Some("Error 1".to_string()));
        assert_eq!(state.attempts, 1);
        assert_eq!(state.last_error, Some("Error 1".to_string()));

        state.record_attempt(Some("Error 2".to_string()));
        assert_eq!(state.attempts, 2);
        assert_eq!(state.last_error, Some("Error 2".to_string()));

        state.reset();
        assert_eq!(state.attempts, 0);
        assert!(state.last_error.is_none());
    }

    #[test]
    fn test_is_retryable_error() {
        // Transient errors
        assert!(is_retryable_error("Connection timeout"));
        assert!(is_retryable_error("503 Service Unavailable"));
        assert!(is_retryable_error("504 Gateway Timeout"));
        assert!(is_retryable_error("502 Bad Gateway"));
        assert!(is_retryable_error("500 Internal Server Error"));
        assert!(is_retryable_error("Rate limit exceeded"));
        assert!(is_retryable_error("Too many requests"));
        assert!(is_retryable_error("Service temporarily unavailable"));

        // Permanent errors
        assert!(!is_retryable_error("404 Not Found"));
        assert!(!is_retryable_error("401 Unauthorized"));
        assert!(!is_retryable_error("403 Forbidden"));
        assert!(!is_retryable_error("400 Bad Request"));
        assert!(!is_retryable_error("Validation error"));
    }

    #[test]
    fn test_extract_retry_after() {
        // Various retry-after formats
        assert_eq!(
            extract_retry_after("Retry after 30 seconds"),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            extract_retry_after("retry-after: 60"),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            extract_retry_after("Rate limit reset in 120s"),
            Some(Duration::from_secs(120))
        );
        assert_eq!(
            extract_retry_after("Please retry after 45 seconds"),
            Some(Duration::from_secs(45))
        );

        // No retry-after information
        assert_eq!(extract_retry_after("Server error"), None);
        assert_eq!(extract_retry_after("Connection failed"), None);
    }

    #[tokio::test]
    async fn test_with_retry_success_first_attempt() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let policy = RetryPolicy::new(3);
        let call_count = Arc::new(AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let result = with_retry(&policy, move || {
            let count = count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<_, String>("success")
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_with_retry_success_after_failures() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let policy = RetryPolicy::new(3)
            .with_initial_interval(0.01)
            .with_jitter(false);
        let call_count = Arc::new(AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let result = with_retry(&policy, move || {
            let count = count_clone.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 3 {
                    Err("transient error")
                } else {
                    Ok::<_, &str>("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_with_retry_all_attempts_fail() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let policy = RetryPolicy::new(3)
            .with_initial_interval(0.01)
            .with_jitter(false);
        let call_count = Arc::new(AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let result = with_retry(&policy, move || {
            let count = count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<String, _>("persistent error")
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "persistent error");
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    // ===== Additional Jitter Calculation Edge Case Tests =====

    #[test]
    fn test_jitter_with_very_small_interval() {
        let policy = RetryPolicy::new(5)
            .with_initial_interval(0.001) // 1ms
            .with_backoff_factor(2.0)
            .with_jitter(true);

        let delay = policy.calculate_delay(0).as_secs_f64();

        // Should be between 0.5ms and 1.5ms
        assert!(delay >= 0.0005);
        assert!(delay <= 0.0015);
    }

    #[test]
    fn test_jitter_with_very_large_interval() {
        let policy = RetryPolicy::new(10)
            .with_initial_interval(100.0)
            .with_backoff_factor(2.0)
            .with_max_interval(200.0)
            .with_jitter(true);

        // Attempt 4: 100 * 2^4 = 1600, capped at 200
        let delay = policy.calculate_delay(4).as_secs_f64();

        // Should be between 100 (200 * 0.5) and 300 (200 * 1.5)
        assert!(delay >= 100.0);
        assert!(delay <= 300.0);
    }

    #[test]
    fn test_jitter_disabled_produces_consistent_delays() {
        let policy = RetryPolicy::new(5)
            .with_initial_interval(2.0)
            .with_backoff_factor(2.0)
            .with_jitter(false);

        // Run multiple times and verify delays are identical
        let delays: Vec<f64> = (0..10)
            .map(|_| policy.calculate_delay(2).as_secs_f64())
            .collect();

        // All delays should be exactly the same when jitter is disabled
        let first_delay = delays[0];
        for delay in delays {
            assert_eq!(delay, first_delay);
        }

        // Should be exactly 2.0 * 2^2 = 8.0
        assert_eq!(first_delay, 8.0);
    }

    #[test]
    fn test_jitter_range_boundaries_multiple_samples() {
        let policy = RetryPolicy::new(10)
            .with_initial_interval(10.0)
            .with_backoff_factor(2.0)
            .with_max_interval(200.0) // Set high max to avoid capping
            .with_jitter(true);

        // Test multiple attempts to ensure jitter stays within bounds
        for attempt in 0..5 {
            let base_delay = 10.0 * 2.0_f64.powi(attempt);
            let capped_delay = base_delay.min(200.0);

            // Sample 20 times to verify range
            for _ in 0..20 {
                let delay = policy.calculate_delay(attempt as usize).as_secs_f64();
                assert!(delay >= capped_delay * 0.5,
                    "Delay {} should be >= {} (capped_delay * 0.5) for attempt {}",
                    delay, capped_delay * 0.5, attempt);
                assert!(delay <= capped_delay * 1.5,
                    "Delay {} should be <= {} (capped_delay * 1.5) for attempt {}",
                    delay, capped_delay * 1.5, attempt);
            }
        }
    }

    #[test]
    fn test_jitter_calculation_with_zero_initial_interval() {
        let policy = RetryPolicy::new(3)
            .with_initial_interval(0.0)
            .with_jitter(true);

        let delay = policy.calculate_delay(0).as_secs_f64();

        // With 0 initial interval, delay should be 0 even with jitter
        assert_eq!(delay, 0.0);
    }

    #[test]
    fn test_calculate_delay_beyond_max_attempts() {
        let policy = RetryPolicy::new(3)
            .with_initial_interval(1.0)
            .with_backoff_factor(2.0);

        // Attempts beyond max should return 0
        assert_eq!(policy.calculate_delay(3).as_secs(), 0);
        assert_eq!(policy.calculate_delay(4).as_secs(), 0);
        assert_eq!(policy.calculate_delay(100).as_secs(), 0);
    }

    #[test]
    fn test_jitter_with_max_interval_cap() {
        let policy = RetryPolicy::new(10)
            .with_initial_interval(5.0)
            .with_backoff_factor(3.0)
            .with_max_interval(20.0)
            .with_jitter(true);

        // Attempt 3: 5.0 * 3^3 = 135, capped at 20
        // With jitter: should be between 10 (20 * 0.5) and 30 (20 * 1.5)
        let delay = policy.calculate_delay(3).as_secs_f64();
        assert!(delay >= 10.0);
        assert!(delay <= 30.0);
    }

    // ===== Additional Retry Predicate Logic Edge Case Tests =====

    #[test]
    fn test_is_retryable_error_case_insensitive() {
        // Test various case combinations
        assert!(is_retryable_error("CONNECTION TIMEOUT"));
        assert!(is_retryable_error("Connection Timeout"));
        assert!(is_retryable_error("connection timeout"));
        assert!(is_retryable_error("CoNnEcTiOn TiMeOuT"));
    }

    #[test]
    fn test_is_retryable_error_partial_matches() {
        // Should match partial occurrences
        assert!(is_retryable_error("The connection to the server timed out"));
        assert!(is_retryable_error("Error 503: Service Unavailable for maintenance"));
        assert!(is_retryable_error("Rate limit exceeded, please try again"));
    }

    #[test]
    fn test_is_retryable_error_edge_cases() {
        // Empty string
        assert!(!is_retryable_error(""));

        // Just whitespace
        assert!(!is_retryable_error("   "));

        // Status codes without keywords
        assert!(is_retryable_error("500"));
        assert!(is_retryable_error("502"));
        assert!(is_retryable_error("503"));
        assert!(is_retryable_error("504"));

        // Non-retryable codes
        assert!(!is_retryable_error("200 OK"));
        assert!(!is_retryable_error("201 Created"));
        assert!(!is_retryable_error("404"));
    }

    #[test]
    fn test_is_retryable_error_compound_messages() {
        // Messages with multiple keywords
        assert!(is_retryable_error("Connection timeout after 30s, service unavailable"));
        assert!(is_retryable_error("503 Service Unavailable: Connection timeout"));

        // Mixed retryable and non-retryable (should be retryable if any match)
        assert!(is_retryable_error("404 Not Found, but service temporarily unavailable"));
    }

    #[test]
    fn test_extract_retry_after_edge_cases() {
        // No numbers
        assert_eq!(extract_retry_after("Retry after soon"), None);

        // Numbers but no retry keyword
        assert_eq!(extract_retry_after("Wait 30 seconds"), None);

        // Multiple numbers (should take first)
        assert_eq!(
            extract_retry_after("Retry after 30 or 60 seconds"),
            Some(Duration::from_secs(30))
        );

        // Very large numbers
        assert_eq!(
            extract_retry_after("Retry after 86400 seconds"),
            Some(Duration::from_secs(86400))
        );

        // Zero seconds
        assert_eq!(
            extract_retry_after("Retry after 0 seconds"),
            Some(Duration::from_secs(0))
        );
    }

    #[test]
    fn test_extract_retry_after_various_formats() {
        // Different separators
        assert_eq!(
            extract_retry_after("Retry-After: 45"),
            Some(Duration::from_secs(45))
        );
        assert_eq!(
            extract_retry_after("RetryAfter=90"),
            Some(Duration::from_secs(90))
        );

        // With units mentioned
        assert_eq!(
            extract_retry_after("retry after 15 sec"),
            Some(Duration::from_secs(15))
        );
        assert_eq!(
            extract_retry_after("retry after 120 secs"),
            Some(Duration::from_secs(120))
        );
    }

    #[test]
    fn test_extract_retry_after_reset_in_format() {
        // "reset in" format variations
        assert_eq!(
            extract_retry_after("Rate limit reset in 300s"),
            Some(Duration::from_secs(300))
        );
        assert_eq!(
            extract_retry_after("Quota reset in 3600 seconds"),
            Some(Duration::from_secs(3600))
        );
        assert_eq!(
            extract_retry_after("limit reset in 60"),
            Some(Duration::from_secs(60))
        );
    }

    #[test]
    fn test_should_retry_boundary_conditions() {
        let policy = RetryPolicy::new(5);

        // Boundary: exactly at max_attempts
        assert!(!policy.should_retry(5));

        // One before max
        assert!(policy.should_retry(4));

        // Well beyond max
        assert!(!policy.should_retry(100));
    }

    #[test]
    fn test_retry_state_multiple_operations() {
        let mut state = RetryState::new();

        // Simulate multiple retry cycles
        for i in 1..=3 {
            state.record_attempt(Some(format!("Error {}", i)));
            assert_eq!(state.attempts, i);
            assert_eq!(state.last_error, Some(format!("Error {}", i)));
        }

        state.reset();

        // Should be ready for new retry cycle
        state.record_attempt(Some("New error".to_string()));
        assert_eq!(state.attempts, 1);
        assert_eq!(state.last_error, Some("New error".to_string()));
    }

    #[test]
    fn test_retry_state_with_none_error() {
        let mut state = RetryState::new();

        // Recording attempt with no error
        state.record_attempt(None);
        assert_eq!(state.attempts, 1);
        assert!(state.last_error.is_none());

        // Recording attempt with error after none
        state.record_attempt(Some("Error occurred".to_string()));
        assert_eq!(state.attempts, 2);
        assert_eq!(state.last_error, Some("Error occurred".to_string()));
    }

    #[test]
    fn test_backoff_factor_edge_cases() {
        // Backoff factor of 1.0 (no exponential growth)
        let policy = RetryPolicy::new(5)
            .with_initial_interval(2.0)
            .with_backoff_factor(1.0)
            .with_jitter(false);

        for attempt in 0..5 {
            assert_eq!(policy.calculate_delay(attempt).as_secs_f64(), 2.0);
        }

        // Backoff factor > 2.0 (faster growth)
        let policy = RetryPolicy::new(5)
            .with_initial_interval(1.0)
            .with_backoff_factor(3.0)
            .with_jitter(false);

        assert_eq!(policy.calculate_delay(0).as_secs_f64(), 1.0);   // 1.0 * 3^0
        assert_eq!(policy.calculate_delay(1).as_secs_f64(), 3.0);   // 1.0 * 3^1
        assert_eq!(policy.calculate_delay(2).as_secs_f64(), 9.0);   // 1.0 * 3^2
        assert_eq!(policy.calculate_delay(3).as_secs_f64(), 27.0);  // 1.0 * 3^3
    }
}
