//! Initialization module for Orca
//!
//! Handles first-time setup including directory creation, database initialization,
//! and configuration file generation.

use crate::error::{OrcaError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Default configuration directory name
pub const CONFIG_DIR: &str = ".orca";

/// Default configuration file name
pub const CONFIG_FILE: &str = "orca.toml";

/// Default database file name
pub const DATABASE_FILE: &str = "orca.db";

/// Get the Orca home directory (~/.orca)
///
/// # Returns
///
/// Path to the user's Orca configuration directory
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined
pub fn get_orca_home() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(CONFIG_DIR))
        .ok_or_else(|| OrcaError::Config("Could not determine home directory".to_string()))
}

/// Get the path to the user-level configuration file
pub fn get_user_config_path() -> Result<PathBuf> {
    Ok(get_orca_home()?.join(CONFIG_FILE))
}

/// Get the path to the project-level configuration file
pub fn get_project_config_path() -> Result<PathBuf> {
    Ok(PathBuf::from(".").join(CONFIG_DIR).join(CONFIG_FILE))
}

/// Get the path to the database file
pub fn get_database_path() -> Result<PathBuf> {
    Ok(get_orca_home()?.join(DATABASE_FILE))
}

/// Check if Orca is initialized
///
/// Returns true if the ~/.orca directory exists and contains a database file
pub fn is_initialized() -> bool {
    get_orca_home()
        .and_then(|home| Ok(home.exists() && home.join(DATABASE_FILE).exists()))
        .unwrap_or(false)
}

/// Initialize Orca directories and configuration
///
/// Creates:
/// - ~/.orca directory
/// - ~/.orca/orca.toml with default configuration
/// - ~/.orca/orca.db (database initialization happens separately)
///
/// # Arguments
///
/// * `force` - If true, overwrite existing configuration
///
/// # Errors
///
/// Returns an error if:
/// - Directory creation fails
/// - File write fails
/// - Permissions are insufficient
pub fn initialize(force: bool) -> Result<()> {
    let orca_home = get_orca_home()?;

    info!(path = %orca_home.display(), "Initializing Orca");

    // Create ~/.orca directory
    if !orca_home.exists() {
        fs::create_dir_all(&orca_home)
            .map_err(|e| OrcaError::Config(format!("Failed to create directory: {}", e)))?;
        info!(path = %orca_home.display(), "Created Orca home directory");
    } else {
        info!(path = %orca_home.display(), "Orca home directory already exists");
    }

    // Create default configuration if it doesn't exist or force is true
    let config_path = orca_home.join(CONFIG_FILE);
    if !config_path.exists() || force {
        create_default_config(&config_path)?;
        info!(path = %config_path.display(), "Created default configuration");
    } else {
        warn!(path = %config_path.display(), "Configuration already exists (use --force to overwrite)");
    }

    // Database initialization will be handled by the db module
    let db_path = orca_home.join(DATABASE_FILE);
    if !db_path.exists() {
        info!(path = %db_path.display(), "Database will be created on first use");
    } else {
        info!(path = %db_path.display(), "Database already exists");
    }

    Ok(())
}

/// Create default configuration file
fn create_default_config(path: &Path) -> Result<()> {
    let default_config = r#"# Orca Configuration
#
# This is the user-level configuration file for Orca.
# Project-specific settings can be placed in ./.orca/orca.toml

[database]
# Database file path (relative to ~/.orca)
path = "orca.db"

[llm]
# LLM provider: "openai", "anthropic", "gemini", "ollama"
provider = "anthropic"

# Model name
model = "claude-3-sonnet"

# API key (can use environment variables like ${ANTHROPIC_API_KEY})
# api_key = "${ANTHROPIC_API_KEY}"

# Temperature for generation (0.0-1.0)
temperature = 0.7

# Maximum tokens to generate
max_tokens = 4096

[execution]
# Maximum concurrent tasks
max_concurrent_tasks = 5

# Task timeout in seconds
task_timeout = 300

# Enable streaming output
streaming = true

[logging]
# Log level: "trace", "debug", "info", "warn", "error"
level = "info"

# Log format: "compact", "pretty", "json"
format = "compact"

[budget]
# Default budget to use for workflows (by name, optional)
# default_budget = "api-budget"

# Enable automatic budget enforcement during execution (true/false)
enforce_budgets = true

# Log budget usage details (true/false)
log_usage = true

# Alert threshold percentage (0.0-100.0) - warn when usage exceeds this
alert_threshold = 80.0

[workflow]
# Default LLM profile to use for all workflows (by name, optional)
# default_llm_profile = "multi-llm"

# Default planner LLM for workflows (format: provider:model, optional)
# default_planner_llm = "anthropic:claude-3-sonnet"

# Default worker LLM for workflows (format: provider:model, optional)
# default_worker_llm = "openai:gpt-4"

# Enable workflow caching (true/false)
enable_caching = false

# Cache time-to-live in seconds
cache_ttl_secs = 3600

# Maximum workflow execution duration in seconds
max_duration_secs = 3600
"#;

    fs::write(path, default_config)
        .map_err(|e| OrcaError::Config(format!("Failed to write configuration: {}", e)))?;

    Ok(())
}

/// Clean up Orca directories (for testing or uninstall)
///
/// # Safety
///
/// This removes the entire ~/.orca directory and all its contents.
/// Use with caution!
#[cfg(test)]
pub fn cleanup() -> Result<()> {
    let orca_home = get_orca_home()?;

    if orca_home.exists() {
        fs::remove_dir_all(&orca_home)
            .map_err(|e| OrcaError::Config(format!("Failed to remove directory: {}", e)))?;
        info!(path = %orca_home.display(), "Cleaned up Orca directory");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_orca_home() {
        let home = get_orca_home();
        assert!(home.is_ok());
        let home_path = home.unwrap();
        assert!(home_path.to_string_lossy().contains(CONFIG_DIR));
    }

    #[test]
    fn test_config_paths() {
        let user_config = get_user_config_path();
        assert!(user_config.is_ok());
        assert!(user_config.unwrap().to_string_lossy().contains(CONFIG_FILE));

        let project_config = get_project_config_path();
        assert!(project_config.is_ok());
        assert!(project_config.unwrap().to_string_lossy().contains(CONFIG_FILE));
    }

    #[test]
    fn test_database_path() {
        let db_path = get_database_path();
        assert!(db_path.is_ok());
        assert!(db_path.unwrap().to_string_lossy().contains(DATABASE_FILE));
    }

    #[test]
    fn test_create_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.toml");

        let result = create_default_config(&config_path);
        assert!(result.is_ok());
        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[database]"));
        assert!(content.contains("[llm]"));
        assert!(content.contains("[execution]"));
        assert!(content.contains("[logging]"));
        assert!(content.contains("[budget]"));
        assert!(content.contains("[workflow]"));
    }
}
