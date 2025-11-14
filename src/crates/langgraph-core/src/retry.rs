//! Retry Policies - Exponential backoff for transient failures
//!
//! This module provides configurable retry policies with exponential backoff and jitter
//! for handling transient failures during graph execution. Retry policies are used to
//! automatically recover from temporary errors like:
//! - Network timeouts and connection failures
//! - Rate limit errors from external APIs
//! - Temporary service unavailability
//! - Deadlock or resource contention
//!
//! # Overview
//!
//! **Retry policies** define how many times an operation should be retried and how long
//! to wait between attempts. The module provides:
//! - **Exponential backoff** - Progressively longer delays between retries
//! - **Jitter** - Random variation to prevent thundering herd
//! - **Configurable limits** - Max attempts, min/max intervals
//! - **Retry state tracking** - Track attempts and errors for debugging
//!
//! **Use retry policies when:**
//! - Calling external APIs with transient failures
//! - Executing LLM calls with rate limits
//! - Accessing databases with connection issues
//! - Performing network operations
//!
//! **Don't retry when:**
//! - Errors are permanent (validation errors, 404s)
//! - Operations have side effects (non-idempotent writes)
//! - Errors require user intervention
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  Graph Execution / Node Execution                        │
//! │  • Attempts operation                                    │
//! │  • On failure, consults retry policy                     │
//! └─────────────┬────────────────────────────────────────────┘
//!               │
//!               ↓ Check if retry allowed
//! ┌──────────────────────────────────────────────────────────┐
//! │  RetryPolicy                                             │
//! │  • max_attempts: 3 (default)                             │
//! │  • initial_interval: 0.5s                                │
//! │  • backoff_factor: 2.0 (doubles each attempt)            │
//! │  • max_interval: 128s                                    │
//! │  • jitter: true (randomize timing)                       │
//! └─────────────┬────────────────────────────────────────────┘
//!               │
//!               ↓ Calculate delay
//! ┌──────────────────────────────────────────────────────────┐
//! │  Exponential Backoff with Jitter                         │
//! │                                                          │
//! │  delay = initial × (backoff_factor ^ attempt)            │
//! │  delay = min(delay, max_interval)                        │
//! │  if jitter: delay *= random(0.5..1.5)                    │
//! └─────────────┬────────────────────────────────────────────┘
//!               │
//!               ↓ Sleep then retry
//! ┌──────────────────────────────────────────────────────────┐
//! │  RetryState (tracks attempts and errors)                 │
//! │  • attempts: 0 → 1 → 2                                   │
//! │  • last_error: "Connection timeout"                      │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Default Retry Policy
//!
//! ```rust
//! use langgraph_core::retry::RetryPolicy;
//!
//! // Default: 3 attempts, exponential backoff with jitter
//! let policy = RetryPolicy::default();
//!
//! // Check if retry is allowed
//! for attempt in 0..5 {
//!     if !policy.should_retry(attempt) {
//!         println!("Max retries exceeded at attempt {}", attempt);
//!         break;
//!     }
//!
//!     let delay = policy.calculate_delay(attempt);
//!     println!("Attempt {}: waiting {:?}", attempt, delay);
//! }
//! ```
//!
//! ## Custom Retry Policy
//!
//! ```rust
//! use langgraph_core::retry::RetryPolicy;
//!
//! // Aggressive retry: 5 attempts, faster backoff
//! let policy = RetryPolicy::new(5)
//!     .with_initial_interval(1.0)      // Start with 1 second
//!     .with_backoff_factor(3.0)        // Triple each time (1s, 3s, 9s, ...)
//!     .with_max_interval(60.0)         // Cap at 60 seconds
//!     .with_jitter(true);              // Add randomness
//! ```
//!
//! ## Retry State Tracking
//!
//! ```rust
//! use langgraph_core::retry::RetryState;
//!
//! let mut state = RetryState::new();
//!
//! // Record failed attempts
//! state.record_attempt(Some("Connection timeout".to_string()));
//! state.record_attempt(Some("Rate limit exceeded".to_string()));
//!
//! println!("Attempts: {}", state.attempts);
//! println!("Last error: {:?}", state.last_error);
//!
//! // Reset after success
//! state.reset();
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Retry Loop for External API
//!
//! ```rust,ignore
//! use langgraph_core::retry::{RetryPolicy, RetryState};
//! use tokio::time::sleep;
//!
//! async fn call_api_with_retry<T>(
//!     policy: &RetryPolicy,
//!     api_call: impl Fn() -> Result<T, String>
//! ) -> Result<T, String> {
//!     let mut state = RetryState::new();
//!
//!     loop {
//!         match api_call() {
//!             Ok(result) => return Ok(result),
//!             Err(error) => {
//!                 state.record_attempt(Some(error.clone()));
//!
//!                 if !policy.should_retry(state.attempts) {
//!                     return Err(format!(
//!                         "Failed after {} attempts: {}",
//!                         state.attempts, error
//!                     ));
//!                 }
//!
//!                 let delay = policy.calculate_delay(state.attempts - 1);
//!                 eprintln!("Retry attempt {} after {:?}", state.attempts, delay);
//!                 sleep(delay).await;
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Pattern 2: LLM Call with Rate Limit Handling
//!
//! ```rust,ignore
//! use langgraph_core::retry::RetryPolicy;
//!
//! async fn call_llm_with_retry(prompt: &str) -> Result<String, String> {
//!     let policy = RetryPolicy::new(5)
//!         .with_initial_interval(2.0)   // Start with 2 seconds
//!         .with_backoff_factor(2.0)     // Double each time
//!         .with_max_interval(60.0);     // Max 1 minute
//!
//!     for attempt in 0..policy.max_attempts {
//!         match call_llm_api(prompt).await {
//!             Ok(response) => return Ok(response),
//!             Err(error) if error.is_rate_limit() => {
//!                 if !policy.should_retry(attempt + 1) {
//!                     return Err("Rate limit retry exhausted".to_string());
//!                 }
//!                 let delay = policy.calculate_delay(attempt);
//!                 tokio::time::sleep(delay).await;
//!             }
//!             Err(error) => return Err(error.to_string()), // Don't retry other errors
//!         }
//!     }
//!
//!     Err("Unexpected retry exhaustion".to_string())
//! }
//! ```
//!
//! ## Pattern 3: Selective Retry by Error Type
//!
//! ```rust,ignore
//! async fn operation_with_selective_retry(
//!     policy: &RetryPolicy
//! ) -> Result<String, String> {
//!     for attempt in 0..policy.max_attempts {
//!         match perform_operation().await {
//!             Ok(result) => return Ok(result),
//!             Err(error) => {
//!                 // Only retry on transient errors
//!                 if is_transient_error(&error) {
//!                     if policy.should_retry(attempt + 1) {
//!                         let delay = policy.calculate_delay(attempt);
//!                         tokio::time::sleep(delay).await;
//!                         continue;
//!                     }
//!                 }
//!                 // Permanent error or retries exhausted
//!                 return Err(error);
//!             }
//!         }
//!     }
//!     Err("Unexpected path".to_string())
//! }
//!
//! fn is_transient_error(error: &str) -> bool {
//!     error.contains("timeout") ||
//!     error.contains("connection") ||
//!     error.contains("rate limit") ||
//!     error.contains("503") ||
//!     error.contains("504")
//! }
//! ```
//!
//! # Exponential Backoff Explained
//!
//! ## Default Policy Timing
//!
//! With default settings (initial: 0.5s, factor: 2.0):
//! - Attempt 0: 0.5s × 2^0 = **0.5s**
//! - Attempt 1: 0.5s × 2^1 = **1.0s**
//! - Attempt 2: 0.5s × 2^2 = **2.0s**
//! - Attempt 3: 0.5s × 2^3 = **4.0s**
//!
//! ## Why Jitter Matters
//!
//! Without jitter, if 1000 requests fail simultaneously (e.g., server restart),
//! they'll all retry at the same time, causing a **thundering herd**:
//!
//! ```text
//! Without Jitter:           With Jitter:
//! All retry at 0.5s         Spread between 0.25s-0.75s
//! ▼▼▼▼▼▼▼▼▼▼               ▼ ▼  ▼ ▼   ▼ ▼  ▼
//! Server overloaded again   Load distributed smoothly
//! ```
//!
//! Jitter multiplies delay by random factor (0.5x to 1.5x), spreading load.
//!
//! # Performance Considerations
//!
//! ## Choosing Backoff Parameters
//!
//! | Use Case | Max Attempts | Initial | Factor | Max Interval |
//! |----------|--------------|---------|--------|--------------|
//! | Fast API (low latency) | 3-5 | 0.5s | 2.0 | 10s |
//! | LLM calls (rate limits) | 5-7 | 2.0s | 2.0 | 60s |
//! | Database operations | 3-4 | 0.1s | 3.0 | 5s |
//! | Long-running jobs | 10+ | 5.0s | 1.5 | 300s |
//!
//! ## Total Time Calculation
//!
//! Total time for N retries with exponential backoff:
//! ```text
//! Total = initial × (factor^N - 1) / (factor - 1)
//!
//! Example (initial=1s, factor=2, N=5):
//! Total = 1 × (2^5 - 1) / (2 - 1) = 31 seconds
//! ```
//!
//! ## Best Practices
//!
//! 1. **Enable jitter in production** - Prevents thundering herd
//! 2. **Set reasonable max_interval** - Don't wait too long between retries
//! 3. **Track retry state** - Log failures for debugging
//! 4. **Don't retry permanent errors** - Check error type before retrying
//! 5. **Consider circuit breakers** - Stop retrying if service is consistently down
//!
//! # Python LangGraph Comparison
//!
//! Python LangGraph uses **tenacity** library for retries:
//!
//! ```python
//! from tenacity import retry, stop_after_attempt, wait_exponential
//!
//! @retry(stop=stop_after_attempt(3),
//!        wait=wait_exponential(multiplier=1, min=0.5, max=60))
//! def call_api():
//!     # Operation that may fail
//!     pass
//! ```
//!
//! **Rust Equivalent:**
//! ```rust,ignore
//! use langgraph_core::retry::RetryPolicy;
//!
//! let policy = RetryPolicy::new(3)
//!     .with_initial_interval(0.5)
//!     .with_backoff_factor(2.0)
//!     .with_max_interval(60.0);
//!
//! // Manual retry loop (no decorator syntax in Rust)
//! for attempt in 0..policy.max_attempts {
//!     match call_api().await {
//!         Ok(result) => return Ok(result),
//!         Err(_) if policy.should_retry(attempt + 1) => {
//!             tokio::time::sleep(policy.calculate_delay(attempt)).await;
//!         }
//!         Err(e) => return Err(e),
//!     }
//! }
//! ```
//!
//! **Key Differences:**
//! - Python uses decorators, Rust requires explicit loops
//! - Both support exponential backoff and jitter
//! - Rust provides more control over retry logic
//! - Python tenacity has more built-in predicates
//!
//! # See Also
//!
//! - [`crate::compiled::CompiledGraph`] - Can use retry policies for node execution
//! - [`crate::tool`] - Tool calls can benefit from retry policies
//! - [`crate::error`] - Error types that may be retryable
//! - [tokio-retry](https://crates.io/crates/tokio-retry) - Alternative retry library
//! - [backoff](https://crates.io/crates/backoff) - Another exponential backoff library

use std::time::Duration;
use rand::Rng;

/// Configuration for retrying failed node executions
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
        let delay0 = policy.calculate_delay(0);
        assert_eq!(delay0.as_secs_f64(), 1.0);

        // Attempt 1: 1.0 * 2^1 = 2.0
        let delay1 = policy.calculate_delay(1);
        assert_eq!(delay1.as_secs_f64(), 2.0);

        // Attempt 2: 1.0 * 2^2 = 4.0
        let delay2 = policy.calculate_delay(2);
        assert_eq!(delay2.as_secs_f64(), 4.0);

        // Attempt 3: 1.0 * 2^3 = 8.0
        let delay3 = policy.calculate_delay(3);
        assert_eq!(delay3.as_secs_f64(), 8.0);
    }

    #[test]
    fn test_max_interval_cap() {
        let policy = RetryPolicy::new(10)
            .with_initial_interval(10.0)
            .with_backoff_factor(2.0)
            .with_max_interval(50.0)
            .with_jitter(false);

        // Attempt 5: 10.0 * 2^5 = 320.0, but capped at 50.0
        let delay = policy.calculate_delay(5);
        assert_eq!(delay.as_secs_f64(), 50.0);
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

        // Check that not all delays are identical (very unlikely with jitter)
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

        assert!(policy.should_retry(0)); // First attempt
        assert!(policy.should_retry(1)); // Second attempt
        assert!(policy.should_retry(2)); // Third attempt
        assert!(!policy.should_retry(3)); // Exceeded max_attempts
        assert!(!policy.should_retry(4)); // Way over
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
}
