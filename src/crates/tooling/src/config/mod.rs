//! Configuration management utilities
//!
//! This module provides common patterns for configuration management across
//! the acolib workspace, including:
//!
//! - `ConfigBuilder` trait for consistent configuration APIs
//! - Environment variable loading with proper error handling
//! - Configuration validation helpers
//!
//! # Example
//!
//! ```rust,ignore
//! use tooling::config::{ConfigBuilder, get_env_parse};
//!
//! #[derive(Clone, Default)]
//! struct AppConfig {
//!     pub port: u16,
//!     pub host: String,
//!     pub debug: bool,
//! }
//!
//! impl ConfigBuilder for AppConfig {
//!     fn validate(&self) -> tooling::Result<()> {
//!         if self.port == 0 {
//!             return Err(tooling::ToolingError::General(
//!                 "Port must be non-zero".into()
//!             ));
//!         }
//!         Ok(())
//!     }
//!
//!     fn from_env(prefix: &str) -> tooling::Result<Self> {
//!         use tooling::config::{get_env_parse_or, get_env_or, get_env_bool};
//!
//!         Ok(Self {
//!             port: get_env_parse_or(&format!("{}PORT", prefix), 8080)?,
//!             host: get_env_or(&format!("{}HOST", prefix), "localhost")?,
//!             debug: get_env_bool(&format!("{}DEBUG", prefix))?.unwrap_or(false),
//!         })
//!     }
//!
//!     fn merge(&mut self, other: Self) -> &mut Self {
//!         if other.port != 0 {
//!             self.port = other.port;
//!         }
//!         if !other.host.is_empty() {
//!             self.host = other.host;
//!         }
//!         self
//!     }
//! }
//!
//! // Usage
//! let config = AppConfig::from_env_with_defaults("APP_")?;
//! ```

mod builder;
mod env;

pub use builder::ConfigBuilder;
pub use env::{
    build_env_key, get_env, get_env_bool, get_env_or, get_env_parse, get_env_parse_or,
};
