//! Configuration loader with dual-location support
//!
//! Loads configuration from:
//! 1. Default values
//! 2. User-level config: ~/.orca/orca.toml
//! 3. Project-level config: ./.orca/orca.toml
//!
//! Later configs override earlier ones.

use crate::config::schema::OrcaConfig;
use crate::error::{OrcaError, Result};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info};

/// Configuration loader that handles both user and project configs
pub struct ConfigLoader {
    user_config_path: PathBuf,
    project_config_path: PathBuf,
}

impl ConfigLoader {
    /// Create a new config loader
    pub fn new() -> Self {
        Self {
            user_config_path: Self::user_config_path(),
            project_config_path: Self::project_config_path(),
        }
    }

    /// Get user-level config path (~/.orca/orca.toml)
    fn user_config_path() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to get home directory")
            .join(".orca")
            .join("orca.toml")
    }

    /// Get project-level config path (./.orca/orca.toml)
    fn project_config_path() -> PathBuf {
        std::env::current_dir()
            .expect("Failed to get current directory")
            .join(".orca")
            .join("orca.toml")
    }

    /// Load configuration from both locations with project taking precedence
    ///
    /// Priority order:
    /// 1. Default values
    /// 2. User-level config (~/.orca/orca.toml)
    /// 3. Project-level config (./.orca/orca.toml)
    pub async fn load(&self) -> Result<OrcaConfig> {
        // Start with defaults
        let mut config = OrcaConfig::default();
        info!("Loading configuration with defaults");

        // Load user-level config if it exists
        match self.load_from_path(&self.user_config_path).await {
            Ok(user_config) => {
                debug!(path = %self.user_config_path.display(), "Loaded user-level config");
                config.merge(user_config);
            }
            Err(e) => {
                debug!(
                    path = %self.user_config_path.display(),
                    error = %e,
                    "User-level config not found, using defaults"
                );
            }
        }

        // Load project-level config if it exists (overrides user config)
        match self.load_from_path(&self.project_config_path).await {
            Ok(project_config) => {
                debug!(path = %self.project_config_path.display(), "Loaded project-level config");
                config.merge(project_config);
            }
            Err(e) => {
                debug!(
                    path = %self.project_config_path.display(),
                    error = %e,
                    "Project-level config not found"
                );
            }
        }

        // Resolve environment variables
        config.resolve_env_vars();

        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Load configuration from a specific path
    async fn load_from_path(&self, path: &PathBuf) -> Result<OrcaConfig> {
        if !path.exists() {
            return Err(OrcaError::Config(format!(
                "Config file not found: {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| OrcaError::Config(format!("Failed to read config: {}", e)))?;

        let config: OrcaConfig = toml::from_str(&content)
            .map_err(|e| OrcaError::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    /// Load only user-level config
    pub async fn load_user_config(&self) -> Result<OrcaConfig> {
        self.load_from_path(&self.user_config_path).await
    }

    /// Load only project-level config
    pub async fn load_project_config(&self) -> Result<OrcaConfig> {
        self.load_from_path(&self.project_config_path).await
    }

    /// Get user config path
    pub fn get_user_config_path(&self) -> &PathBuf {
        &self.user_config_path
    }

    /// Get project config path
    pub fn get_project_config_path(&self) -> &PathBuf {
        &self.project_config_path
    }

    /// Check if user config exists
    pub fn user_config_exists(&self) -> bool {
        self.user_config_path.exists()
    }

    /// Check if project config exists
    pub fn project_config_exists(&self) -> bool {
        self.project_config_path.exists()
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_paths() {
        let loader = ConfigLoader::new();

        let user_path = loader.get_user_config_path();
        assert!(user_path.ends_with(".orca/orca.toml"));

        let project_path = loader.get_project_config_path();
        assert!(project_path.ends_with(".orca/orca.toml"));
    }

    #[tokio::test]
    async fn test_load_returns_defaults_when_no_files() {
        let loader = ConfigLoader::new();
        let config = loader.load().await.unwrap();

        // Should return defaults even if files don't exist
        assert_eq!(config.database.path, "orca.db");
        assert_eq!(config.llm.provider, "anthropic");
    }

    #[test]
    fn test_config_existence_checks() {
        let loader = ConfigLoader::new();

        // These will be false unless the user has actually initialized orca
        let _user_exists = loader.user_config_exists();
        let _project_exists = loader.project_config_exists();

        // Just verify the methods don't panic
        assert!(true);
    }
}
