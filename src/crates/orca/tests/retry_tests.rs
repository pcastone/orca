//! Integration tests for retry logic with exponential backoff (ORCA-035)

use orca::executor::retry::{RetryConfig, with_retry};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[tokio::test]
async fn test_retry_eventually_succeeds() {
    let attempt_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = attempt_counter.clone();

    let config = RetryConfig::default();

    let result = with_retry(&config, "test-task", || {
        let counter = counter_clone.clone();
        async move {
            let attempt = counter.fetch_add(1, Ordering::SeqCst);

            // Fail first 2 attempts, succeed on 3rd
            if attempt < 2 {
                Err("Transient failure")
            } else {
                Ok("Success")
            }
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(attempt_counter.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_retry_exhausts_attempts() {
    let attempt_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = attempt_counter.clone();

    let config = RetryConfig {
        max_retries: 3,
        ..Default::default()
    };

    let result = with_retry(&config, "test-task", || {
        let counter = counter_clone.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Err::<&str, &str>("Persistent failure")
        }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Persistent failure");
    // Should try: initial + 3 retries = 4 total attempts
    assert_eq!(attempt_counter.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn test_retry_immediate_success() {
    let attempt_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = attempt_counter.clone();

    let config = RetryConfig::default();

    let result: Result<&str, &str> = with_retry(&config, "test-task", || {
        let counter = counter_clone.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok("Success")
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    // Should only try once
    assert_eq!(attempt_counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_retry_exponential_backoff() {
    let config = RetryConfig {
        max_retries: 3,
        initial_delay_secs: 1,
        max_delay_secs: 60,
        multiplier: 2.0,
    };

    let attempt_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = attempt_counter.clone();

    let start_time = Instant::now();

    let _result = with_retry(&config, "test-task", || {
        let counter = counter_clone.clone();
        async move {
            let attempt = counter.fetch_add(1, Ordering::SeqCst);

            // Always fail to test full backoff sequence
            if attempt < 3 {
                Err("Failure")
            } else {
                Ok("Success")
            }
        }
    })
    .await;

    let duration = start_time.elapsed();

    // Expected delays: 0s (attempt 0), 1s (attempt 1), 2s (attempt 2), 4s (attempt 3)
    // Total: ~7 seconds
    // Allow some tolerance for timing
    assert!(duration.as_secs() >= 6, "Backoff too fast: {:?}", duration);
    assert!(duration.as_secs() <= 10, "Backoff too slow: {:?}", duration);
}

#[test]
fn test_retry_config_defaults() {
    let config = RetryConfig::default();

    assert_eq!(config.max_retries, 3);
    assert_eq!(config.initial_delay_secs, 1);
    assert_eq!(config.max_delay_secs, 60);
    assert_eq!(config.multiplier, 2.0);
}

#[test]
fn test_retry_config_custom_values() {
    let config = RetryConfig {
        max_retries: 5,
        initial_delay_secs: 2,
        max_delay_secs: 120,
        multiplier: 3.0,
    };

    assert_eq!(config.max_retries, 5);
    assert_eq!(config.initial_delay_secs, 2);
    assert_eq!(config.max_delay_secs, 120);
    assert_eq!(config.multiplier, 3.0);
}

#[test]
fn test_delay_calculation() {
    let config = RetryConfig {
        max_retries: 5,
        initial_delay_secs: 1,
        max_delay_secs: 60,
        multiplier: 2.0,
    };

    // Test delay calculation for various attempts
    assert_eq!(config.calculate_delay(0).as_secs(), 1);  // 1 * 2^0 = 1
    assert_eq!(config.calculate_delay(1).as_secs(), 2);  // 1 * 2^1 = 2
    assert_eq!(config.calculate_delay(2).as_secs(), 4);  // 1 * 2^2 = 4
    assert_eq!(config.calculate_delay(3).as_secs(), 8);  // 1 * 2^3 = 8
    assert_eq!(config.calculate_delay(4).as_secs(), 16); // 1 * 2^4 = 16
    assert_eq!(config.calculate_delay(5).as_secs(), 32); // 1 * 2^5 = 32
    assert_eq!(config.calculate_delay(6).as_secs(), 60); // Capped at max_delay_secs
    assert_eq!(config.calculate_delay(10).as_secs(), 60); // Still capped
}

#[test]
fn test_delay_calculation_with_different_multiplier() {
    let config = RetryConfig {
        max_retries: 3,
        initial_delay_secs: 2,
        max_delay_secs: 100,
        multiplier: 3.0,
    };

    assert_eq!(config.calculate_delay(0).as_secs(), 2);  // 2 * 3^0 = 2
    assert_eq!(config.calculate_delay(1).as_secs(), 6);  // 2 * 3^1 = 6
    assert_eq!(config.calculate_delay(2).as_secs(), 18); // 2 * 3^2 = 18
    assert_eq!(config.calculate_delay(3).as_secs(), 54); // 2 * 3^3 = 54
    assert_eq!(config.calculate_delay(4).as_secs(), 100); // Capped
}

#[tokio::test]
async fn test_retry_with_zero_retries() {
    let attempt_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = attempt_counter.clone();

    let config = RetryConfig {
        max_retries: 0,
        ..Default::default()
    };

    let result = with_retry(&config, "test-task", || {
        let counter = counter_clone.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Err::<&str, &str>("Failure")
        }
    })
    .await;

    assert!(result.is_err());
    // Should only try once (no retries)
    assert_eq!(attempt_counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_retry_with_flaky_operation() {
    // Simulate a flaky operation that fails randomly
    let attempt_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = attempt_counter.clone();

    let config = RetryConfig {
        max_retries: 5,
        initial_delay_secs: 0, // No delay for faster test
        ..Default::default()
    };

    let result = with_retry(&config, "test-task", || {
        let counter = counter_clone.clone();
        async move {
            let attempt = counter.fetch_add(1, Ordering::SeqCst);

            // Fail attempts 0, 1, 3 but succeed on attempt 2
            if attempt == 2 {
                Ok("Success")
            } else if attempt < 4 {
                Err("Transient failure")
            } else {
                Ok("Later success")
            }
        }
    })
    .await;

    assert!(result.is_ok());
    // Should succeed on attempt 2 (index 2)
    assert_eq!(attempt_counter.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_retry_preserves_success_value() {
    let config = RetryConfig::default();

    let result: Result<i32, &str> = with_retry(&config, "test-task", || async {
        Ok(42)
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_retry_preserves_error_type() {
    let config = RetryConfig {
        max_retries: 2,
        ..Default::default()
    };

    let result: Result<i32, String> = with_retry(&config, "test-task", || async {
        Err::<i32, String>("Custom error message".to_string())
    })
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Custom error message");
}
