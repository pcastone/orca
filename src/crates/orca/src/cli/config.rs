//! Configuration helpers for CLI
//!
//! Provides utilities for loading configuration and creating execution contexts.

use crate::config::ConfigLoader;
use crate::context::{ContextBuilder, ExecutionContext};
use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::init;
use std::sync::Arc;
use tracing::info;

/// Get or create an execution context from user configuration
///
/// This loads the configuration, initializes the database, and creates
/// a fully configured execution context ready for use.
///
/// # Returns
/// An initialized ExecutionContext
///
/// # Errors
/// Returns error if configuration is invalid or context cannot be created
pub async fn get_or_create_context() -> Result<ExecutionContext> {
    // Load configuration
    info!("Loading configuration");
    let loader = ConfigLoader::new();
    let config = loader.load().await?;

    // Get database path
    let db_path = config.database_path();

    // Ensure database parent directory exists
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| OrcaError::Database(format!("Failed to create database directory: {}", e)))?;
        }
    }

    // Initialize database
    info!(path = %db_path.display(), "Initializing database");
    let database = Arc::new(Database::initialize(db_path).await?);

    // Get workspace root
    let workspace_root = config.execution.workspace_root.clone()
        .or_else(|| std::env::current_dir().ok())
        .ok_or_else(|| OrcaError::Config("Unable to determine workspace root".to_string()))?;

    // Build execution context
    info!("Building execution context");
    let context = ContextBuilder::new()
        .with_database(database)
        .with_config(config)
        .with_workspace_root(workspace_root)
        .build()
        .await?;

    info!("Execution context ready");
    Ok(context)
}

/// Check if orca is initialized
///
/// Returns true if the configuration file exists
pub fn is_initialized() -> bool {
    init::get_user_config_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

/// Get initialization instructions
pub fn get_init_instructions() -> String {
    "Orca is not initialized. Run 'orca init' to get started.".to_string()
}
