//! Retry logic with exponential backoff
//!
//! Provides configurable retry mechanism for failed task executions.

use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: usize,

    /// Initial delay before first retry (in seconds)
    pub initial_delay_secs: u64,

    /// Maximum delay between retries (in seconds)
    pub max_delay_secs: u64,

    /// Multiplier for exponential backoff (e.g., 2.0 for doubling)
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_secs: 1,
            max_delay_secs: 60,
            multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(max_retries: usize, initial_delay_secs: u64, max_delay_secs: u64, multiplier: f64) -> Self {
        Self {
            max_retries,
            initial_delay_secs,
            max_delay_secs,
            multiplier,
        }
    }

    /// Calculate delay for a given attempt number (0-indexed)
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        let delay_secs = (self.initial_delay_secs as f64) * self.multiplier.powi(attempt as i32);
        let capped_delay = delay_secs.min(self.max_delay_secs as f64);
        Duration::from_secs(capped_delay as u64)
    }
}

/// Execute a function with retry logic
///
/// # Arguments
/// * `config` - Retry configuration
/// * `task_id` - Task identifier for logging
/// * `operation` - Async function to execute
///
/// # Returns
/// Result of the operation after all retries exhausted or success
pub async fn with_retry<F, Fut, T, E>(
    config: &RetryConfig,
    task_id: &str,
    mut operation: F,
) -> std::result::Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error = None;

    for attempt in 0..=config.max_retries {
        if attempt > 0 {
            let delay = config.calculate_delay(attempt - 1);
            debug!(
                task_id = %task_id,
                attempt = attempt,
                delay_secs = delay.as_secs(),
                "Retrying after delay"
            );
            sleep(delay).await;
        }

        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!(
                        task_id = %task_id,
                        attempt = attempt,
                        "Retry succeeded"
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt < config.max_retries {
                    warn!(
                        task_id = %task_id,
                        attempt = attempt + 1,
                        max_retries = config.max_retries,
                        error = %e,
                        "Operation failed, will retry"
                    );
                } else {
                    warn!(
                        task_id = %task_id,
                        attempt = attempt + 1,
                        error = %e,
                        "Operation failed, max retries exhausted"
                    );
                }
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_secs, 1);
        assert_eq!(config.max_delay_secs, 60);
        assert_eq!(config.multiplier, 2.0);
    }

    #[test]
    fn test_calculate_delay_exponential() {
        let config = RetryConfig::new(3, 1, 60, 2.0);

        // Attempt 0: 1 * 2^0 = 1 second
        assert_eq!(config.calculate_delay(0).as_secs(), 1);

        // Attempt 1: 1 * 2^1 = 2 seconds
        assert_eq!(config.calculate_delay(1).as_secs(), 2);

        // Attempt 2: 1 * 2^2 = 4 seconds
        assert_eq!(config.calculate_delay(2).as_secs(), 4);

        // Attempt 3: 1 * 2^3 = 8 seconds
        assert_eq!(config.calculate_delay(3).as_secs(), 8);
    }

    #[test]
    fn test_calculate_delay_capped() {
        let config = RetryConfig::new(10, 10, 30, 2.0);

        // Attempt 0: 10 * 2^0 = 10 seconds
        assert_eq!(config.calculate_delay(0).as_secs(), 10);

        // Attempt 1: 10 * 2^1 = 20 seconds
        assert_eq!(config.calculate_delay(1).as_secs(), 20);

        // Attempt 2: 10 * 2^2 = 40 seconds, capped at 30
        assert_eq!(config.calculate_delay(2).as_secs(), 30);

        // Attempt 3: 10 * 2^3 = 80 seconds, capped at 30
        assert_eq!(config.calculate_delay(3).as_secs(), 30);
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_immediately() {
        let config = RetryConfig::new(3, 1, 60, 2.0);
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();

        let result = with_retry(&config, "test-task", || {
            let attempts = attempts_clone.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Ok::<i32, String>(42)
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_after_failures() {
        let config = RetryConfig::new(3, 0, 60, 2.0); // 0 delay for fast test
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();

        let result = with_retry(&config, "test-task", || {
            let attempts = attempts_clone.clone();
            async move {
                let count = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                if count < 3 {
                    Err("Temporary failure".to_string())
                } else {
                    Ok::<i32, String>(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_with_retry_exhausts_retries() {
        let config = RetryConfig::new(2, 0, 60, 2.0); // 0 delay for fast test
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();

        let result = with_retry(&config, "test-task", || {
            let attempts = attempts_clone.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err::<i32, String>("Permanent failure".to_string())
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Permanent failure");
        assert_eq!(attempts.load(Ordering::SeqCst), 3); // Initial attempt + 2 retries
    }
}
