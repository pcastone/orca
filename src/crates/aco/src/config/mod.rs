//! Configuration management for aco client
//!
//! Supports dual-location configuration:
//! - User-level: ~/.aco/aco.toml
//! - Project-level: ./.aco/aco.toml
//!
//! Project-level config overrides user-level config.

mod schema;
mod loader;
mod init;

pub use schema::{AcoConfig, ServerConfig, ClientConfig, ToolsConfig, UiConfig};
pub use loader::ConfigLoader;
pub use init::{init_config_directories, ensure_config_files, init_project_config};

use crate::Result;

/// Load configuration from both locations with project config taking precedence
pub async fn load_config() -> Result<AcoConfig> {
    let loader = ConfigLoader::new();
    loader.load().await
}

/// Initialize configuration directories and files if they don't exist
pub async fn init_config() -> Result<()> {
    init_config_directories().await?;
    ensure_config_files().await?;
    Ok(())
}
