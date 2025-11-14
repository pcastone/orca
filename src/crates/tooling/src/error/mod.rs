//! Error handling utilities
//!
//! This module provides utilities for error handling and context management
//! across the acolib workspace.
//!
//! # Features
//!
//! - `ErrorContext` trait for adding contextual information to errors
//! - Error chain formatting and analysis
//! - Root cause extraction
//!
//! # Example
//!
//! ```rust,ignore
//! use tooling::error::{ErrorContext, format_error_chain, root_cause};
//!
//! fn process_file(path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     std::fs::read_to_string(path)
//!         .context(format!("Failed to read file: {}", path))?;
//!
//!     // ... more operations ...
//!
//!     Ok(())
//! }
//!
//! // Error handling
//! match process_file("config.json") {
//!     Err(e) => {
//!         eprintln!("Error chain:\n{}", format_error_chain(&*e));
//!         eprintln!("Root cause: {}", root_cause(&*e));
//!     }
//!     Ok(_) => println!("Success!"),
//! }
//! ```

mod context;

pub use context::{error_chain_length, format_error_chain, root_cause, ErrorContext};
