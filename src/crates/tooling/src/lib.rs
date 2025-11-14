//! Tooling utilities and helpers for acolib
//!
//! This crate provides common tooling functionality that can be used
//! across the acolib workspace.
//!
//! # Modules
//!
//! - `config` - Configuration management with environment variable loading
//! - `error` - Error handling utilities with context and chain formatting
//! - `async_utils` - Retry policies and timeout utilities for async operations
//! - `validation` - Fluent validation API for type-safe data validation
//! - `serialization` - Stable JSON serialization and hashing utilities
//! - `rate_limit` - Token bucket and sliding window rate limiters
//! - `logging` - Structured logging helpers and formatters

pub mod async_utils;
pub mod config;
pub mod error;
pub mod logging;
pub mod rate_limit;
pub mod serialization;
pub mod validation;
pub mod tools;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur in the tooling crate
#[derive(Debug, Error)]
pub enum ToolingError {
    /// General error with message
    #[error("Tooling error: {0}")]
    General(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for tooling operations
pub type Result<T> = std::result::Result<T, ToolingError>;

/// Configuration structure for tooling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolingConfig {
    /// Enable verbose logging
    pub verbose: bool,
    /// Output directory
    pub output_dir: Option<String>,
}

impl Default for ToolingConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            output_dir: None,
        }
    }
}

impl ToolingConfig {
    /// Create a new tooling configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set verbose mode
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set output directory
    pub fn with_output_dir(mut self, dir: impl Into<String>) -> Self {
        self.output_dir = Some(dir.into());
        self
    }
}

/// Initialize the tooling system
pub fn init() -> Result<()> {
    tracing::debug!("Initializing tooling system");
    Ok(())
}

/// Get version information
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ToolingConfig::default();
        assert!(!config.verbose);
        assert!(config.output_dir.is_none());
    }

    #[test]
    fn test_config_builder() {
        let config = ToolingConfig::new()
            .with_verbose(true)
            .with_output_dir("/tmp");

        assert!(config.verbose);
        assert_eq!(config.output_dir, Some("/tmp".to_string()));
    }

    #[test]
    fn test_version() {
        let v = version();
        assert!(!v.is_empty());
    }

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }
}
