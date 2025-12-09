//! Configuration management for Orca
//!
//! Supports dual-location configuration:
//! - User-level: ~/.orca/orca.toml
//! - Project-level: ./.orca/orca.toml
//!
//! Project-level config overrides user-level config.

mod schema;
mod loader;
pub mod deep_merge;

pub use schema::{OrcaConfig, DatabaseConfig, ExecutionConfig, LoggingConfig, BudgetConfig, WorkflowConfig, BackupConfig};
pub use loader::ConfigLoader;
pub use deep_merge::{deep_merge, deep_merge_with_options, MergeOptions, ArrayMergeStrategy, NullHandling};

use crate::Result;

/// Load configuration from both locations with project config taking precedence
///
/// Priority order:
/// 1. Default values
/// 2. User-level config (~/.orca/orca.toml)
/// 3. Project-level config (./.orca/orca.toml)
pub async fn load_config() -> Result<OrcaConfig> {
    let loader = ConfigLoader::new();
    loader.load().await
}
