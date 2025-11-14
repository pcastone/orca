//! Task executor for Pregel tasks.

use crate::error::Result;
use super::types::{PregelExecutableTask, RetryPolicy};
use std::time::Duration;

/// Executor for Pregel tasks with retry logic.
pub struct TaskExecutor {
    retry_policies: Vec<RetryPolicy>,
}

impl TaskExecutor {
    pub fn new(retry_policies: Vec<RetryPolicy>) -> Self {
        Self { retry_policies }
    }

    /// Execute a task with retries.
    ///
    /// Implements exponential backoff with optional jitter for retry delays.
    /// Uses the first retry policy if multiple are configured, or defaults to single attempt.
    pub async fn execute(&self, task: &PregelExecutableTask) -> Result<serde_json::Value> {
        let policy = self.retry_policies.first();
        let max_attempts = policy.map(|p| p.max_attempts).unwrap_or(1);

        let mut attempts = 0;
        let mut last_error = None;

        while attempts < max_attempts {
            attempts += 1;

            tracing::debug!(
                task = ?task.name,
                attempt = attempts,
                max_attempts = max_attempts,
                "Executing task"
            );

            match task.proc.execute(task.input.clone()).await {
                Ok(result) => {
                    if attempts > 1 {
                        tracing::info!(
                            task = ?task.name,
                            attempts = attempts,
                            "Task succeeded after retry"
                        );
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);

                    // If we haven't exhausted attempts, wait before retrying
                    if attempts < max_attempts {
                        if let Some(policy) = policy {
                            let delay = self.calculate_delay(policy, attempts);
                            tracing::warn!(
                                task = ?task.name,
                                attempt = attempts,
                                max_attempts = max_attempts,
                                delay_ms = delay.as_millis(),
                                error = ?last_error,
                                "Task failed, retrying after delay"
                            );
                            tokio::time::sleep(delay).await;
                        }
                    } else {
                        tracing::error!(
                            task = ?task.name,
                            attempts = attempts,
                            error = ?last_error,
                            "Task failed after all retry attempts"
                        );
                    }
                }
            }
        }

        // All attempts exhausted
        Err(last_error.unwrap())
    }

    /// Calculate retry delay with exponential backoff and optional jitter.
    ///
    /// # Arguments
    ///
    /// * `policy` - The retry policy with backoff configuration
    /// * `attempt` - The current attempt number (1-indexed)
    ///
    /// # Returns
    ///
    /// Duration to wait before the next retry attempt
    fn calculate_delay(&self, policy: &RetryPolicy, attempt: usize) -> Duration {
        let base = policy.initial_interval;
        let multiplier = policy.backoff_factor.powi((attempt - 1) as i32);
        let delay = base * multiplier;
        let capped = delay.min(policy.max_interval);

        // Add jitter if enabled
        let final_delay = if policy.jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            // Add random jitter between 0% and 25% of the delay
            let jitter_factor = rng.gen_range(0.0..0.25);
            capped * (1.0 + jitter_factor)
        } else {
            capped
        };

        Duration::from_secs_f64(final_delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GraphError;
    use crate::pregel::types::NodeExecutor;
    use serde_json::json;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Mock executor that fails a certain number of times before succeeding
    struct FailingExecutor {
        fail_count: Arc<AtomicUsize>,
        attempts: Arc<AtomicUsize>,
    }

    impl NodeExecutor for FailingExecutor {
        fn execute(
            &self,
            _input: serde_json::Value,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + '_>> {
            let fail_count = self.fail_count.clone();
            let attempts = self.attempts.clone();

            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                let failures_needed = fail_count.load(Ordering::SeqCst);

                if attempt < failures_needed {
                    Err(GraphError::Execution(format!("Simulated failure {}/{}", attempt + 1, failures_needed)))
                } else {
                    Ok(json!({"success": true, "attempts": attempt + 1}))
                }
            })
        }
    }

    #[tokio::test]
    async fn test_executor_succeeds_without_retry() {
        let executor = TaskExecutor::new(vec![]);
        let attempts = Arc::new(AtomicUsize::new(0));

        let task = PregelExecutableTask {
            name: "test_task".to_string(),
            input: json!({}),
            proc: Arc::new(FailingExecutor {
                fail_count: Arc::new(AtomicUsize::new(0)),
                attempts: attempts.clone(),
            }),
            writes: VecDeque::new(),
            config: json!(null),
            triggers: vec![],
            write_channels: vec![],
            retry_policy: vec![],
            cache_key: None,
            id: "task-1".to_string(),
            path: vec![],
            writers: vec![],
        };

        let result = executor.execute(&task).await;
        assert!(result.is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 1, "Should execute exactly once");
    }

    #[tokio::test]
    async fn test_executor_retries_on_failure() {
        let policy = RetryPolicy {
            initial_interval: 0.001, // 1ms for fast tests
            backoff_factor: 2.0,
            max_interval: 0.01,
            max_attempts: 3,
            jitter: false, // Disable jitter for deterministic tests
        };

        let executor = TaskExecutor::new(vec![policy]);
        let attempts = Arc::new(AtomicUsize::new(0));
        let fail_count = Arc::new(AtomicUsize::new(2)); // Fail first 2 attempts

        let task = PregelExecutableTask {
            name: "test_task".to_string(),
            input: json!({}),
            proc: Arc::new(FailingExecutor {
                fail_count: fail_count.clone(),
                attempts: attempts.clone(),
            }),
            writes: VecDeque::new(),
            config: json!(null),
            triggers: vec![],
            write_channels: vec![],
            retry_policy: vec![],
            cache_key: None,
            id: "task-2".to_string(),
            path: vec![],
            writers: vec![],
        };

        let result = executor.execute(&task).await;
        assert!(result.is_ok(), "Should succeed after retries");
        assert_eq!(attempts.load(Ordering::SeqCst), 3, "Should attempt 3 times (fail, fail, succeed)");
    }

    #[tokio::test]
    async fn test_executor_fails_after_max_attempts() {
        let policy = RetryPolicy {
            initial_interval: 0.001,
            backoff_factor: 2.0,
            max_interval: 0.01,
            max_attempts: 3,
            jitter: false,
        };

        let executor = TaskExecutor::new(vec![policy]);
        let attempts = Arc::new(AtomicUsize::new(0));
        let fail_count = Arc::new(AtomicUsize::new(10)); // Always fail

        let task = PregelExecutableTask {
            name: "test_task".to_string(),
            input: json!({}),
            proc: Arc::new(FailingExecutor {
                fail_count: fail_count.clone(),
                attempts: attempts.clone(),
            }),
            writes: VecDeque::new(),
            config: json!(null),
            triggers: vec![],
            write_channels: vec![],
            retry_policy: vec![],
            cache_key: None,
            id: "task-3".to_string(),
            path: vec![],
            writers: vec![],
        };

        let result = executor.execute(&task).await;
        assert!(result.is_err(), "Should fail after max attempts");
        assert_eq!(attempts.load(Ordering::SeqCst), 3, "Should attempt exactly 3 times");
    }

    #[tokio::test]
    async fn test_calculate_delay_exponential_backoff() {
        let policy = RetryPolicy {
            initial_interval: 1.0,
            backoff_factor: 2.0,
            max_interval: 10.0,
            max_attempts: 5,
            jitter: false,
        };

        let executor = TaskExecutor::new(vec![policy.clone()]);

        // Attempt 1: 1.0 * 2^0 = 1.0
        let delay1 = executor.calculate_delay(&policy, 1);
        assert_eq!(delay1.as_secs_f64(), 1.0);

        // Attempt 2: 1.0 * 2^1 = 2.0
        let delay2 = executor.calculate_delay(&policy, 2);
        assert_eq!(delay2.as_secs_f64(), 2.0);

        // Attempt 3: 1.0 * 2^2 = 4.0
        let delay3 = executor.calculate_delay(&policy, 3);
        assert_eq!(delay3.as_secs_f64(), 4.0);

        // Attempt 4: 1.0 * 2^3 = 8.0
        let delay4 = executor.calculate_delay(&policy, 4);
        assert_eq!(delay4.as_secs_f64(), 8.0);

        // Attempt 5: 1.0 * 2^4 = 16.0, but capped at max_interval = 10.0
        let delay5 = executor.calculate_delay(&policy, 5);
        assert_eq!(delay5.as_secs_f64(), 10.0);
    }

    #[tokio::test]
    async fn test_calculate_delay_with_jitter() {
        let policy = RetryPolicy {
            initial_interval: 1.0,
            backoff_factor: 1.0, // No backoff for simpler testing
            max_interval: 10.0,
            max_attempts: 5,
            jitter: true,
        };

        let executor = TaskExecutor::new(vec![policy.clone()]);

        // With jitter, delay should be between base and base * 1.25
        let delay = executor.calculate_delay(&policy, 1);
        let delay_secs = delay.as_secs_f64();

        assert!(delay_secs >= 1.0, "Delay with jitter should be at least the base");
        assert!(delay_secs <= 1.25, "Delay with jitter should be at most base * 1.25");
    }
}
