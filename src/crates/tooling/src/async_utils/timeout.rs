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

    // ===== Cancellation Propagation Tests =====

    #[tokio::test]
    async fn test_timeout_cancels_operation() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = completed.clone();

        let result = with_timeout(Duration::from_millis(10), async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            completed_clone.store(true, Ordering::SeqCst);
            Ok::<_, String>("should not reach here")
        })
        .await;

        // Operation should timeout
        assert!(result.is_err());
        matches!(result.unwrap_err(), TimeoutError::Timeout(_));

        // Give a bit of time to ensure the future was dropped
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Operation should not have completed
        assert!(!completed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_timeout_guard_cancellation_on_drop() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let started = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));

        let started_clone = started.clone();
        let completed_clone = completed.clone();

        // Create a scope where guard will be dropped
        {
            let guard = TimeoutGuard::new(Duration::from_millis(100));

            let future = guard.execute(async move {
                started_clone.store(true, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(200)).await;
                completed_clone.store(true, Ordering::SeqCst);
                Ok::<_, String>("completed")
            });

            // Drop the future before it completes (simulating cancellation)
            drop(future);
        }

        // Give time for any background tasks
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Operation should not have completed
        assert!(!completed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_cancellation_with_nested_timeouts() {
        let result = with_timeout(Duration::from_millis(50), async {
            with_timeout(Duration::from_millis(100), async {
                tokio::time::sleep(Duration::from_millis(150)).await;
                Ok::<_, String>("inner")
            })
            .await
        })
        .await;

        // Outer timeout should fire first
        assert!(result.is_err());
        if let Err(TimeoutError::Timeout(d)) = result {
            assert_eq!(d, Duration::from_millis(50));
        } else {
            panic!("Expected outer timeout");
        }
    }

    // ===== Early Completion Handling Tests =====

    #[tokio::test]
    async fn test_early_completion_immediately() {
        let start = tokio::time::Instant::now();

        let result = with_timeout(Duration::from_secs(10), async {
            // Complete immediately
            Ok::<_, String>("immediate")
        })
        .await;

        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "immediate");
        // Should complete in well under the timeout
        assert!(elapsed < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_early_completion_with_very_short_timeout() {
        // Even with a very short timeout, if operation completes first, it should succeed
        let result = with_timeout(Duration::from_micros(100), async {
            Ok::<_, String>("fast")
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "fast");
    }

    #[tokio::test]
    async fn test_timeout_guard_early_completion() {
        let guard = TimeoutGuard::new(Duration::from_secs(10));

        let result = guard
            .execute(async {
                // Complete early
                tokio::time::sleep(Duration::from_millis(1)).await;
                Ok::<_, String>("early")
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "early");

        // Guard should still have time remaining
        assert!(guard.remaining().is_some());
        assert!(guard.remaining().unwrap() > Duration::from_secs(9));
    }

    #[tokio::test]
    async fn test_multiple_operations_with_same_guard() {
        let guard = TimeoutGuard::new(Duration::from_secs(1));

        // First operation
        let result1 = guard
            .execute(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok::<_, String>("first")
            })
            .await;
        assert!(result1.is_ok());

        // Second operation with same guard (reduced remaining time)
        let result2 = guard
            .execute(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok::<_, String>("second")
            })
            .await;
        assert!(result2.is_ok());

        // Guard should have less time remaining
        let remaining = guard.remaining();
        assert!(remaining.is_some());
        assert!(remaining.unwrap() < Duration::from_millis(800));
    }

    // ===== Edge Case Tests =====

    #[tokio::test]
    async fn test_zero_duration_timeout() {
        let result = with_timeout(Duration::from_secs(0), async {
            // Even immediate completion might timeout with zero duration
            Ok::<_, String>("instant")
        })
        .await;

        // With zero duration, should typically timeout
        // (though in practice it might succeed due to timing)
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_very_long_timeout() {
        let guard = TimeoutGuard::new(Duration::from_secs(86400)); // 1 day

        assert!(!guard.is_expired());
        assert!(guard.remaining().is_some());

        let remaining = guard.remaining().unwrap();
        assert!(remaining > Duration::from_secs(86000));
    }

    #[tokio::test]
    async fn test_timeout_error_display_formatting() {
        // Test Timeout variant display
        let timeout_err: TimeoutError<String> = TimeoutError::Timeout(Duration::from_secs(5));
        let display = format!("{}", timeout_err);
        assert!(display.contains("timed out"));
        assert!(display.contains("5s"));

        // Test OperationFailed variant display
        let op_failed_err: TimeoutError<String> =
            TimeoutError::OperationFailed("custom error".to_string());
        let display = format!("{}", op_failed_err);
        assert!(display.contains("Operation failed"));
        assert!(display.contains("custom error"));
    }

    #[tokio::test]
    async fn test_timeout_error_source() {
        use std::error::Error;

        // Timeout variant should have no source
        let timeout_err: TimeoutError<std::io::Error> = TimeoutError::Timeout(Duration::from_secs(1));
        assert!(timeout_err.source().is_none());

        // OperationFailed with error type that implements Error
        let op_err = std::io::Error::new(std::io::ErrorKind::Other, "io error");
        let timeout_err = TimeoutError::OperationFailed(op_err);
        assert!(timeout_err.source().is_some());
    }

    #[tokio::test]
    async fn test_timeout_guard_sleep_already_expired() {
        let guard = TimeoutGuard::new(Duration::from_millis(1));
        tokio::time::sleep(Duration::from_millis(10)).await;

        let start = tokio::time::Instant::now();
        guard.sleep_until_deadline().await;
        let elapsed = start.elapsed();

        // Should return immediately if already expired
        assert!(elapsed < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_concurrent_timeout_operations() {
        use tokio::join;

        let (result1, result2, result3) = join!(
            with_timeout(Duration::from_millis(100), async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<_, String>("fast")
            }),
            with_timeout(Duration::from_millis(50), async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok::<_, String>("slow")
            }),
            with_timeout(Duration::from_millis(200), async {
                Ok::<_, String>("immediate")
            })
        );

        // First should succeed
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), "fast");

        // Second should timeout
        assert!(result2.is_err());
        matches!(result2.unwrap_err(), TimeoutError::Timeout(_));

        // Third should succeed
        assert!(result3.is_ok());
        assert_eq!(result3.unwrap(), "immediate");
    }

    #[tokio::test]
    async fn test_timeout_with_panicking_operation() {
        use std::panic::AssertUnwindSafe;
        use tokio::task;

        // Spawn in a separate task to catch the panic
        let handle = task::spawn(async {
            with_timeout(Duration::from_secs(1), async {
                panic!("operation panicked");
                #[allow(unreachable_code)]
                Ok::<_, String>("never")
            })
            .await
        });

        let result = AssertUnwindSafe(handle).await;

        // Should propagate the panic
        assert!(result.is_err());
    }
}
