//! aco client application library
//!
//! This crate provides the aco client application that hosts the Tool Runtime
//! and communicates with the orchestrator via WebSocket.

pub mod client;
pub mod config;
pub mod error;
pub mod server;
pub mod session;
pub mod tui;
pub mod version;
pub mod workspace;

pub use config::{AcoConfig, ConfigLoader};
pub use error::{AcoError, Result};
pub use server::AcoServer;
pub use session::SessionManager;
pub use tui::{App, TuiConfig};
pub use workspace::{
    PathValidator, SecurityConfig, WorkspaceInitConfig, WorkspaceInitializer, WorkspaceMetadata,
    WorkspaceValidator,
};

