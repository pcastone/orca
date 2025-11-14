//! Utility functions and helpers for acolib.
//!
//! This crate provides common utilities for HTTP servers, clients, and configuration
//! management. It's designed to complement the acolib ecosystem with reusable components.
//!
//! # Modules
//!
//! ## Server (`server`)
//!
//! HTTP server utilities including configuration builders and helpers:
//!
//! ```rust,ignore
//! use utils::server::{ServerConfig, ServerBuilder};
//! use std::time::Duration;
//!
//! let config = ServerBuilder::new()
//!     .bind("0.0.0.0", 8080)
//!     .timeout(Duration::from_secs(30))
//!     .with_logging()
//!     .with_cors()
//!     .build();
//! ```
//!
//! ## Client (`client`)
//!
//! HTTP client utilities with retry logic and authentication helpers:
//!
//! ```rust,ignore
//! use utils::client::{ClientConfig, HttpClient};
//! use std::time::Duration;
//!
//! let config = ClientConfig::new()
//!     .with_timeout(Duration::from_secs(30))
//!     .with_max_retries(3)
//!     .with_user_agent("my-app");
//!
//! let client = HttpClient::new(config)?;
//! let response = client.get("https://api.example.com").await?;
//! ```
//!
//! ## Config (`config`)
//!
//! Configuration management utilities for environment variables and file loading:
//!
//! ```rust,ignore
//! use utils::config::{get_env, get_env_parse, load_config_file};
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct AppConfig {
//!     api_key: String,
//!     port: u16,
//! }
//!
//! // Load from environment
//! let api_key = get_env("API_KEY")?;
//! let port = get_env_parse::<u16>("PORT")?;
//!
//! // Load from file
//! let config: AppConfig = load_config_file("config.yaml")?;
//! ```
//!
//! # Features
//!
//! - `server` - Server utilities (enabled by default)
//! - `client` - Client utilities (enabled by default)
//! - `config` - Configuration utilities (enabled by default)

pub mod error;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "config")]
pub mod config;

// Re-export commonly used types
pub use error::{Result, UtilsError};

#[cfg(feature = "server")]
pub use server::{ServerBuilder, ServerConfig};

#[cfg(feature = "client")]
pub use client::{AuthHelper, ClientConfig, HttpClient};

#[cfg(feature = "config")]
pub use config::{
    get_env, get_env_bool, get_env_bool_or, get_env_or, get_env_parse, get_env_parse_or,
    load_config_file, load_json_config, load_yaml_config, ConfigBuilder, FromEnv, ValidateConfig,
};

