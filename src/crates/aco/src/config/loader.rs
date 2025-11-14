//! Configuration loader with dual-location support

use crate::config::schema::AcoConfig;
use crate::{AcoError, Result};
use std::path::PathBuf;
use tokio::fs;

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

    /// Get user-level config path (~/.aco/aco.toml)
    fn user_config_path() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to get home directory")
            .join(".aco")
            .join("aco.toml")
    }

    /// Get project-level config path (./.aco/aco.toml)
    fn project_config_path() -> PathBuf {
        std::env::current_dir()
            .expect("Failed to get current directory")
            .join(".aco")
            .join("aco.toml")
    }

    /// Load configuration from both locations with project taking precedence
    pub async fn load(&self) -> Result<AcoConfig> {
        // Start with defaults
        let mut config = AcoConfig::default();

        // Load user-level config if it exists
        if let Ok(user_config) = self.load_from_path(&self.user_config_path).await {
            config.merge(user_config);
        }

        // Load project-level config if it exists (overrides user config)
        if let Ok(project_config) = self.load_from_path(&self.project_config_path).await {
            config.merge(project_config);
        }

        Ok(config)
    }

    /// Load configuration from a specific path
    async fn load_from_path(&self, path: &PathBuf) -> Result<AcoConfig> {
        if !path.exists() {
            return Err(AcoError::Config(format!(
                "Config file not found: {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(path).await?;
        let config: AcoConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load only user-level config
    pub async fn load_user_config(&self) -> Result<AcoConfig> {
        self.load_from_path(&self.user_config_path).await
    }

    /// Load only project-level config
    pub async fn load_project_config(&self) -> Result<AcoConfig> {
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
        assert!(user_path.ends_with(".aco/aco.toml"));

        let project_path = loader.get_project_config_path();
        assert!(project_path.ends_with(".aco/aco.toml"));
    }

    #[tokio::test]
    async fn test_load_returns_defaults_when_no_files() {
        let loader = ConfigLoader::new();
        let config = loader.load().await.unwrap();

        // Should return defaults even if files don't exist
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
    }
}
