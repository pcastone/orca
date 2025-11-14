//! Configuration initialization and directory setup

use crate::Result;
use std::path::PathBuf;
use tokio::fs;

/// Initialize both user and project .aco directories
pub async fn init_config_directories() -> Result<()> {
    // Create user-level .aco directory (~/.aco)
    let user_aco_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".aco");

    if !user_aco_dir.exists() {
        fs::create_dir_all(&user_aco_dir).await?;
        tracing::info!("Created user config directory: {}", user_aco_dir.display());
    }

    // Create project-level .aco directory (./.aco)
    let project_aco_dir = std::env::current_dir()
        .expect("Failed to get current directory")
        .join(".aco");

    if !project_aco_dir.exists() {
        fs::create_dir_all(&project_aco_dir).await?;
        tracing::info!("Created project config directory: {}", project_aco_dir.display());
    }

    Ok(())
}

/// Ensure config files exist (create defaults if not)
pub async fn ensure_config_files() -> Result<()> {
    // Ensure user config file
    let user_config_path = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".aco")
        .join("aco.toml");

    if !user_config_path.exists() {
        create_default_config_file(&user_config_path).await?;
        tracing::info!("Created user config file: {}", user_config_path.display());
    }

    // Ensure project config file
    let project_config_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join(".aco")
        .join("aco.toml");

    if !project_config_path.exists() {
        create_default_config_file(&project_config_path).await?;
        tracing::info!("Created project config file: {}", project_config_path.display());
    }

    Ok(())
}

/// Create a default config file at the specified path
async fn create_default_config_file(path: &PathBuf) -> Result<()> {
    let default_config = r#"# aco Configuration File
#
# This file configures the aco client application.
# Project-level config (./.aco/aco.toml) overrides user-level config (~/.aco/aco.toml)

[server]
host = "127.0.0.1"
port = 8080
ws_path = "/ws"
enable_tls = false

[client]
orchestrator_url = "ws://127.0.0.1:8080/ws"
session_timeout = 3600        # seconds
reconnect_attempts = 5
reconnect_delay_ms = 1000     # milliseconds

[tools]
enabled_tools = []
execution_timeout = 300       # seconds

# Example tool-specific settings:
# [tools.tool_settings.file_reader]
# max_file_size = 10485760    # 10MB
# allowed_extensions = [".txt", ".md", ".json"]

[ui]
enable_tui = false
log_level = "info"            # trace, debug, info, warn, error
colored_output = true
show_timestamps = true
"#;

    fs::write(path, default_config).await?;
    Ok(())
}

/// Initialize config for a new project (creates .aco directory and config file)
pub async fn init_project_config() -> Result<PathBuf> {
    let project_aco_dir = std::env::current_dir()
        .expect("Failed to get current directory")
        .join(".aco");

    fs::create_dir_all(&project_aco_dir).await?;

    let config_path = project_aco_dir.join("aco.toml");
    create_default_config_file(&config_path).await?;

    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("aco.toml");

        create_default_config_file(&config_path).await.unwrap();

        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).await.unwrap();
        assert!(content.contains("[server]"));
        assert!(content.contains("[client]"));
        assert!(content.contains("[tools]"));
        assert!(content.contains("[ui]"));
    }
}
