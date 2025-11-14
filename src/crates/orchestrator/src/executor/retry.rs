//! Retry Logic for LLM Execution
//!
//! This module provides sophisticated retry strategies with exponential backoff,
//! error classification (transient vs permanent), and comprehensive logging.

use crate::{OrchestratorError, Result};
use std::future::Future;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Classification of errors for retry decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Transient errors that may succeed on retry (rate limits, timeouts, 5xx)
    Transient,

    /// Permanent errors that won't succeed on retry (4xx, invalid auth)
    Permanent,

    /// Unknown errors - treat as transient by default
    Unknown,
}

/// Retry strategy configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,

    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u64,

    /// Maximum backoff delay in milliseconds
    pub max_backoff_ms: u64,

    /// Multiplier for exponential backoff (typically 2.0)
    pub backoff_multiplier: f64,

    /// Whether to add random jitter to backoff delays
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,     // 1 second
            max_backoff_ms: 60_000,        // 60 seconds
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration with custom max retries
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Set initial backoff delay
    pub fn with_initial_backoff(mut self, ms: u64) -> Self {
        self.initial_backoff_ms = ms;
        self
    }

    /// Set maximum backoff delay
    pub fn with_max_backoff(mut self, ms: u64) -> Self {
        self.max_backoff_ms = ms;
        self
    }

    /// Set backoff multiplier
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Enable or disable jitter
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Calculate backoff delay for a given attempt
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        let delay_ms = (self.initial_backoff_ms as f64
            * self.backoff_multiplier.powi(attempt as i32)) as u64;

        let delay_ms = delay_ms.min(self.max_backoff_ms);

        let delay_ms = if self.jitter {
            // Add up to 25% random jitter
            let jitter_amount = (delay_ms as f64 * 0.25 * rand::random::<f64>()) as u64;
            delay_ms + jitter_amount
        } else {
            delay_ms
        };

        Duration::from_millis(delay_ms)
    }
}

/// Classify an error to determine if it should be retried
pub fn classify_error(error: &OrchestratorError) -> ErrorClass {
    match error {
        OrchestratorError::General(msg) => {
            let msg_lower = msg.to_lowercase();

            // Transient errors
            if msg_lower.contains("rate limit")
                || msg_lower.contains("too many requests")
                || msg_lower.contains("429") {
                return ErrorClass::Transient;
            }

            if msg_lower.contains("timeout")
                || msg_lower.contains("timed out")
                || msg_lower.contains("deadline exceeded") {
                return ErrorClass::Transient;
            }

            if msg_lower.contains("503")
                || msg_lower.contains("service unavailable")
                || msg_lower.contains("502")
                || msg_lower.contains("bad gateway")
                || msg_lower.contains("500")
                || msg_lower.contains("internal server error") {
                return ErrorClass::Transient;
            }

            if msg_lower.contains("connection")
                || msg_lower.contains("network")
                || msg_lower.contains("dns") {
                return ErrorClass::Transient;
            }

            // Permanent errors
            if msg_lower.contains("401")
                || msg_lower.contains("unauthorized")
                || msg_lower.contains("invalid api key")
                || msg_lower.contains("authentication failed") {
                return ErrorClass::Permanent;
            }

            if msg_lower.contains("403")
                || msg_lower.contains("forbidden")
                || msg_lower.contains("access denied") {
                return ErrorClass::Permanent;
            }

            if msg_lower.contains("404")
                || msg_lower.contains("not found") {
                return ErrorClass::Permanent;
            }

            if msg_lower.contains("400")
                || msg_lower.contains("bad request")
                || msg_lower.contains("invalid request") {
                return ErrorClass::Permanent;
            }

            // Default to unknown
            ErrorClass::Unknown
        }
        _ => ErrorClass::Unknown,
    }
}

/// Execute a function with retry logic
///
/// # Arguments
/// * `config` - Retry configuration
/// * `operation_name` - Name of the operation for logging
/// * `f` - Async function to execute
///
/// # Returns
/// Result of the operation or the last error encountered
pub async fn retry_with_backoff<F, Fut, T>(
    config: &RetryConfig,
    operation_name: &str,
    mut f: F,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 0..=config.max_retries {
        if attempt > 0 {
            info!(
                operation = operation_name,
                attempt = attempt,
                max_retries = config.max_retries,
                "Retrying operation"
            );
        }

        match f().await {
            Ok(result) => {
                if attempt > 0 {
                    info!(
                        operation = operation_name,
                        attempt = attempt,
                        "Operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                let error_class = classify_error(&e);

                debug!(
                    operation = operation_name,
                    attempt = attempt,
                    error = ?e,
                    classification = ?error_class,
                    "Operation failed"
                );

                // Don't retry permanent errors
                if error_class == ErrorClass::Permanent {
                    error!(
                        operation = operation_name,
                        error = ?e,
                        "Permanent error detected, aborting retries"
                    );
                    return Err(e);
                }

                last_error = Some(e);

                // Don't sleep after the last attempt
                if attempt < config.max_retries {
                    let delay = config.backoff_delay(attempt);

                    warn!(
                        operation = operation_name,
                        attempt = attempt,
                        delay_ms = delay.as_millis(),
                        "Transient error, will retry after delay"
                    );

                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    // All retries exhausted
    let final_error = last_error.unwrap_or_else(|| {
        OrchestratorError::General(format!(
            "Operation '{}' failed after {} retries",
            operation_name, config.max_retries
        ))
    });

    error!(
        operation = operation_name,
        max_retries = config.max_retries,
        error = ?final_error,
        "All retry attempts exhausted"
    );

    Err(final_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_ms, 1000);
        assert_eq!(config.max_backoff_ms, 60_000);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!(config.jitter);
    }

    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::new(5)
            .with_initial_backoff(500)
            .with_max_backoff(30_000)
            .with_multiplier(1.5)
            .with_jitter(false);

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_backoff_ms, 500);
        assert_eq!(config.max_backoff_ms, 30_000);
        assert_eq!(config.backoff_multiplier, 1.5);
        assert!(!config.jitter);
    }

    #[test]
    fn test_backoff_delay_exponential() {
        let config = RetryConfig::new(3)
            .with_initial_backoff(1000)
            .with_jitter(false);

        let delay0 = config.backoff_delay(0);
        let delay1 = config.backoff_delay(1);
        let delay2 = config.backoff_delay(2);

        assert_eq!(delay0.as_millis(), 1000);  // 1000 * 2^0
        assert_eq!(delay1.as_millis(), 2000);  // 1000 * 2^1
        assert_eq!(delay2.as_millis(), 4000);  // 1000 * 2^2
    }

    #[test]
    fn test_backoff_delay_max_cap() {
        let config = RetryConfig::new(10)
            .with_initial_backoff(1000)
            .with_max_backoff(5000)
            .with_jitter(false);

        let delay5 = config.backoff_delay(5);

        // Would be 32000 without cap, should be capped at 5000
        assert_eq!(delay5.as_millis(), 5000);
    }

    #[test]
    fn test_classify_error_rate_limit() {
        let error = OrchestratorError::General("Rate limit exceeded (429)".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Transient);

        let error = OrchestratorError::General("Too many requests".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Transient);
    }

    #[test]
    fn test_classify_error_timeout() {
        let error = OrchestratorError::General("Request timed out".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Transient);

        let error = OrchestratorError::General("Deadline exceeded".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Transient);
    }

    #[test]
    fn test_classify_error_server_errors() {
        let error = OrchestratorError::General("500 Internal Server Error".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Transient);

        let error = OrchestratorError::General("503 Service Unavailable".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Transient);

        let error = OrchestratorError::General("502 Bad Gateway".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Transient);
    }

    #[test]
    fn test_classify_error_auth_failures() {
        let error = OrchestratorError::General("401 Unauthorized".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Permanent);

        let error = OrchestratorError::General("Invalid API key".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Permanent);

        let error = OrchestratorError::General("Authentication failed".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Permanent);
    }

    #[test]
    fn test_classify_error_bad_request() {
        let error = OrchestratorError::General("400 Bad Request".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Permanent);

        let error = OrchestratorError::General("404 Not Found".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Permanent);

        let error = OrchestratorError::General("403 Forbidden".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Permanent);
    }

    #[test]
    fn test_classify_error_network() {
        let error = OrchestratorError::General("Connection refused".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Transient);

        let error = OrchestratorError::General("Network error".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Transient);
    }

    #[test]
    fn test_classify_error_unknown() {
        let error = OrchestratorError::General("Some other error".to_string());
        assert_eq!(classify_error(&error), ErrorClass::Unknown);
    }

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        use std::sync::{Arc, Mutex};

        let config = RetryConfig::new(3);
        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let result = retry_with_backoff(&config, "test_operation", move || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                *count.lock().unwrap() += 1;
                Ok::<i32, OrchestratorError>(42)
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*attempt_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_transient_errors() {
        use std::sync::{Arc, Mutex};

        let config = RetryConfig::new(3)
            .with_initial_backoff(10) // Fast for testing
            .with_jitter(false);

        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let result = retry_with_backoff(&config, "test_operation", move || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                let mut c = count.lock().unwrap();
                *c += 1;
                let current = *c;
                drop(c);

                if current < 3 {
                    Err(OrchestratorError::General("503 Service Unavailable".to_string()))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*attempt_count.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_retry_abort_on_permanent_error() {
        use std::sync::{Arc, Mutex};

        let config = RetryConfig::new(3)
            .with_initial_backoff(10);

        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let result: Result<()> = retry_with_backoff(&config, "test_operation", move || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                *count.lock().unwrap() += 1;
                Err(OrchestratorError::General("401 Unauthorized".to_string()))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(*attempt_count.lock().unwrap(), 1); // Should not retry permanent errors
    }

    #[tokio::test]
    async fn test_retry_exhaust_all_attempts() {
        use std::sync::{Arc, Mutex};

        let config = RetryConfig::new(2)
            .with_initial_backoff(10)
            .with_jitter(false);

        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let result: Result<()> = retry_with_backoff(&config, "test_operation", move || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                *count.lock().unwrap() += 1;
                Err(OrchestratorError::General("Timeout".to_string()))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(*attempt_count.lock().unwrap(), 3); // Initial + 2 retries
    }
}
