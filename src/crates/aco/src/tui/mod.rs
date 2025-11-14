//! TUI module for ratatui-based terminal interface
//!
//! Provides terminal UI with event loop, app state, and keyboard handling.

use crate::error::Result;
use std::path::PathBuf;

pub mod app;
pub mod events;
pub mod ui;

pub use app::{App, AppState, View, TaskItem, WorkflowItem};
pub use events::{EventHandler, Event};

/// TUI configuration
#[derive(Debug, Clone)]
pub struct TuiConfig {
    pub server_url: String,
    pub workspace: PathBuf,
    pub verbose: bool,
}

impl TuiConfig {
    /// Create TUI config from environment variables
    pub fn from_env(server_url: String, workspace: PathBuf, verbose: bool) -> Self {
        Self {
            server_url,
            workspace,
            verbose,
        }
    }
}

/// Run the TUI application (stub implementation)
pub async fn run(_config: TuiConfig) -> Result<()> {
    // Stub implementation - will be implemented in future tasks
    Ok(())
}
