//! Timeout utilities for async operations
//!
//! Provides timeout wrappers and RAII timeout guards for async operations.

use std::future::Future;
use std::time::Duration;
use tokio::time::timeout as tokio_timeout;

/// Execute an async operation with a timeout
///
/// # Arguments
///
/// * `duration` - Maximum duration to wait
/// * `operation` - Async operation to execute
///
/// # Returns
///
/// Result of the operation, or error if timeout exceeded
///
/// # Example
///
/// ```rust,ignore
/// use tooling::async_utils::timeout::with_timeout;
/// use std::time::Duration;
///
/// async fn slow_operation() -> Result<String, String> {
///     tokio::time::sleep(Duration::from_secs(10)).await;
///     Ok("done".to_string())
/// }
///
/// let result = with_timeout(
///     Duration::from_secs(1),
///     slow_operation()
/// ).await;
///
/// assert!(result.is_err()); // Timeout
/// ```
pub async fn with_timeout<F, T, E>(
    duration: Duration,
    operation: F,
) -> std::result::Result<T, TimeoutError<E>>
where
    F: Future<Output = std::result::Result<T, E>>,
{
    match tokio_timeout(duration, operation).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(error)) => Err(TimeoutError::OperationFailed(error)),
        Err(_elapsed) => Err(TimeoutError::Timeout(duration)),
    }
}

/// Error type for timeout operations
#[derive(Debug)]
pub enum TimeoutError<E> {
    /// Operation completed but failed
    OperationFailed(E),
    /// Operation timed out
    Timeout(Duration),
}

impl<E: std::fmt::Display> std::fmt::Display for TimeoutError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutError::OperationFailed(e) => write!(f, "Operation failed: {}", e),
            TimeoutError::Timeout(d) => write!(f, "Operation timed out after {:?}", d),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for TimeoutError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TimeoutError::OperationFailed(e) => Some(e),
            TimeoutError::Timeout(_) => None,
        }
    }
}

/// RAII timeout guard that ensures an operation completes within a duration
///
/// Automatically starts a timer on creation and can be checked or awaited.
///
/// # Example
///
/// ```rust,ignore
/// use tooling::async_utils::timeout::TimeoutGuard;
/// use std::time::Duration;
///
/// async fn monitored_operation() -> Result<(), String> {
///     let guard = TimeoutGuard::new(Duration::from_secs(30));
///
///     // Do work...
///     some_async_work().await?;
///
///     // Check if timeout exceeded
///     if guard.is_expired() {
///         return Err("Operation took too long".to_string());
///     }
///
///     Ok(())
/// }
/// ```
pub struct TimeoutGuard {
    deadline: tokio::time::Instant,
    duration: Duration,
}

impl TimeoutGuard {
    /// Create a new timeout guard with the specified duration
    pub fn new(duration: Duration) -> Self {
        Self {
            deadline: tokio::time::Instant::now() + duration,
            duration,
        }
    }

    /// Check if the timeout has been exceeded
    pub fn is_expired(&self) -> bool {
        tokio::time::Instant::now() >= self.deadline
    }

    /// Get the remaining time until timeout
    ///
    /// Returns None if timeout has already expired
    pub fn remaining(&self) -> Option<Duration> {
        let now = tokio::time::Instant::now();
        if now >= self.deadline {
            None
        } else {
            Some(self.deadline.duration_since(now))
        }
    }

    /// Get the original timeout duration
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Sleep until the timeout deadline
    ///
    /// Returns immediately if deadline has passed
    pub async fn sleep_until_deadline(&self) {
        if let Some(remaining) = self.remaining() {
            tokio::time::sleep(remaining).await;
        }
    }

    /// Execute an operation with the remaining time as timeout
    ///
    /// # Arguments
    ///
    /// * `operation` - Async operation to execute
    ///
    /// # Returns
    ///
    /// Result of the operation, or timeout error if remaining time exceeded
    pub async fn execute<F, T, E>(
        &self,
        operation: F,
    ) -> std::result::Result<T, TimeoutError<E>>
    where
        F: Future<Output = std::result::Result<T, E>>,
    {
        match self.remaining() {
            Some(remaining) => with_timeout(remaining, operation).await,
            None => Err(TimeoutError::Timeout(self.duration)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_with_timeout_success() {
        let result = with_timeout(Duration::from_millis(100), async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<_, String>("success")
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_with_timeout_exceeded() {
        let result = with_timeout(Duration::from_millis(10), async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<_, String>("should not reach here")
        })
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TimeoutError::Timeout(d) => {
                assert_eq!(d, Duration::from_millis(10));
            }
            _ => panic!("Expected timeout error"),
        }
    }

    #[tokio::test]
    async fn test_with_timeout_operation_fails() {
        let result = with_timeout(Duration::from_millis(100), async {
            Err::<String, _>("operation error")
        })
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TimeoutError::OperationFailed(e) => {
                assert_eq!(e, "operation error");
            }
            _ => panic!("Expected operation failed error"),
        }
    }

    #[tokio::test]
    async fn test_timeout_guard_not_expired() {
        let guard = TimeoutGuard::new(Duration::from_secs(10));

        assert!(!guard.is_expired());
        assert!(guard.remaining().is_some());

        let remaining = guard.remaining().unwrap();
        assert!(remaining <= Duration::from_secs(10));
        assert!(remaining > Duration::from_secs(9));
    }

    #[tokio::test]
    async fn test_timeout_guard_expired() {
        let guard = TimeoutGuard::new(Duration::from_millis(1));

        tokio::time::sleep(Duration::from_millis(10)).await;

        assert!(guard.is_expired());
        assert!(guard.remaining().is_none());
    }

    #[tokio::test]
    async fn test_timeout_guard_duration() {
        let guard = TimeoutGuard::new(Duration::from_secs(5));
        assert_eq!(guard.duration(), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_timeout_guard_execute_success() {
        let guard = TimeoutGuard::new(Duration::from_secs(1));

        let result = guard
            .execute(async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<_, String>("success")
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_timeout_guard_execute_timeout() {
        let guard = TimeoutGuard::new(Duration::from_millis(10));

        let result = guard
            .execute(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok::<_, String>("should not reach")
            })
            .await;

        assert!(result.is_err());
        matches!(result.unwrap_err(), TimeoutError::Timeout(_));
    }

    #[tokio::test]
    async fn test_timeout_guard_execute_already_expired() {
        let guard = TimeoutGuard::new(Duration::from_millis(1));
        tokio::time::sleep(Duration::from_millis(10)).await;

        let result = guard
            .execute(async { Ok::<_, String>("should not execute") })
            .await;

        assert!(result.is_err());
        matches!(result.unwrap_err(), TimeoutError::Timeout(_));
    }

    #[tokio::test]
    async fn test_timeout_guard_sleep_until_deadline() {
        let guard = TimeoutGuard::new(Duration::from_millis(50));

        let start = tokio::time::Instant::now();
        guard.sleep_until_deadline().await;
        let elapsed = start.elapsed();

        // Should sleep approximately 50ms
        assert!(elapsed >= Duration::from_millis(45));
        assert!(elapsed <= Duration::from_millis(100));
    }
}
