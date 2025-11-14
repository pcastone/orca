//! Async utilities for common async patterns
//!
//! This module provides utilities for working with async operations:
//! - Retry policies with exponential backoff
//! - Timeout wrappers and guards
//!
//! # Example
//!
//! ```rust,ignore
//! use tooling::async_utils::retry::{RetryPolicy, with_retry};
//! use tooling::async_utils::timeout::{with_timeout, TimeoutGuard};
//! use std::time::Duration;
//!
//! // Retry with exponential backoff
//! async fn call_api_with_retry() -> Result<String, String> {
//!     let policy = RetryPolicy::new(3)
//!         .with_initial_interval(1.0)
//!         .with_backoff_factor(2.0);
//!
//!     with_retry(&policy, || async {
//!         // API call that may fail transiently
//!         Ok("success".to_string())
//!     }).await
//! }
//!
//! // Timeout for slow operations
//! async fn call_with_timeout() -> Result<String, String> {
//!     with_timeout(
//!         Duration::from_secs(30),
//!         async {
//!             // Slow operation
//!             Ok("done".to_string())
//!         }
//!     ).await
//!     .map_err(|e| e.to_string())
//! }
//!
//! // Combined retry + timeout
//! async fn robust_call() -> Result<String, String> {
//!     let policy = RetryPolicy::new(3);
//!
//!     with_retry(&policy, || async {
//!         with_timeout(
//!             Duration::from_secs(10),
//!             async {
//!                 // API call with timeout
//!                 Ok("success".to_string())
//!             }
//!         ).await
//!         .map_err(|e| e.to_string())
//!     }).await
//! }
//! ```

pub mod retry;
pub mod timeout;
