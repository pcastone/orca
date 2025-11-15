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
    use tempfile::TempDir;

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

    // ===== Config Merging Priority Tests =====

    #[tokio::test]
    async fn test_config_merging_user_overrides_defaults() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[server]
host = "0.0.0.0"
port = 9090

[client]
session_timeout = 7200
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let config = loader.load().await.unwrap();

        // User config should override defaults
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.client.session_timeout, 7200);

        // Unspecified fields should remain defaults
        assert_eq!(config.server.ws_path, "/ws");
        assert_eq!(config.ui.log_level, "info");
    }

    #[tokio::test]
    async fn test_config_merging_project_overrides_user() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");
        let project_config_path = temp_dir.path().join("project.toml");

        let user_toml = r#"
[server]
host = "0.0.0.0"
port = 9090

[client]
session_timeout = 7200
"#;

        let project_toml = r#"
[server]
port = 8888

[ui]
log_level = "debug"
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();
        fs::write(&project_config_path, project_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = project_config_path;

        let config = loader.load().await.unwrap();

        // Project config should override user config
        assert_eq!(config.server.port, 8888);
        assert_eq!(config.ui.log_level, "debug");

        // User-only fields preserved
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.client.session_timeout, 7200);

        // Defaults for unspecified fields
        assert_eq!(config.server.ws_path, "/ws");
    }

    #[tokio::test]
    async fn test_config_merging_three_levels() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");
        let project_config_path = temp_dir.path().join("project.toml");

        let user_toml = r#"
[server]
host = "0.0.0.0"
port = 9000

[client]
reconnect_attempts = 10
"#;

        let project_toml = r#"
[server]
port = 9999

[ui]
enable_tui = true
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();
        fs::write(&project_config_path, project_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = project_config_path;

        let config = loader.load().await.unwrap();

        // Priority: defaults < user < project
        assert_eq!(config.server.host, "0.0.0.0"); // From user
        assert_eq!(config.server.port, 9999); // From project (overrides user)
        assert_eq!(config.client.reconnect_attempts, 10); // From user
        assert!(config.ui.enable_tui); // From project
        assert_eq!(config.server.ws_path, "/ws"); // From defaults
    }

    #[tokio::test]
    async fn test_config_merging_tools_list_union() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");
        let project_config_path = temp_dir.path().join("project.toml");

        let user_toml = r#"
[tools]
enabled_tools = ["shell", "git"]
execution_timeout = 600
"#;

        let project_toml = r#"
[tools]
enabled_tools = ["git", "filesystem", "http"]
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();
        fs::write(&project_config_path, project_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = project_config_path;

        let config = loader.load().await.unwrap();

        // enabled_tools should be union of both lists
        assert_eq!(config.tools.enabled_tools.len(), 4); // shell, git, filesystem, http
        assert!(config.tools.enabled_tools.contains(&"shell".to_string()));
        assert!(config.tools.enabled_tools.contains(&"git".to_string()));
        assert!(config.tools.enabled_tools.contains(&"filesystem".to_string()));
        assert!(config.tools.enabled_tools.contains(&"http".to_string()));

        // User's timeout should be preserved
        assert_eq!(config.tools.execution_timeout, 600);
    }

    // ===== User-Level Config Loading Tests =====

    #[tokio::test]
    async fn test_load_user_config_success() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[server]
host = "localhost"
port = 3000

[ui]
log_level = "trace"
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;

        let config = loader.load_user_config().await.unwrap();

        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.ui.log_level, "trace");
    }

    #[tokio::test]
    async fn test_load_user_config_file_not_found() {
        let mut loader = ConfigLoader::new();
        loader.user_config_path = PathBuf::from("/nonexistent/user.toml");

        let result = loader.load_user_config().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_load_user_config_with_all_sections() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[server]
host = "0.0.0.0"
port = 8081
ws_path = "/websocket"
enable_tls = true

[client]
orchestrator_url = "wss://example.com/ws"
session_timeout = 3600
reconnect_attempts = 3
reconnect_delay_ms = 2000

[tools]
enabled_tools = ["shell", "git", "http"]
execution_timeout = 120

[ui]
enable_tui = true
log_level = "debug"
colored_output = false
show_timestamps = false
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;

        let config = loader.load_user_config().await.unwrap();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8081);
        assert_eq!(config.server.ws_path, "/websocket");
        assert!(config.server.enable_tls);
        assert_eq!(config.client.orchestrator_url, "wss://example.com/ws");
        assert_eq!(config.client.session_timeout, 3600);
        assert_eq!(config.client.reconnect_attempts, 3);
        assert_eq!(config.client.reconnect_delay_ms, 2000);
        assert_eq!(config.tools.enabled_tools.len(), 3);
        assert_eq!(config.tools.execution_timeout, 120);
        assert!(config.ui.enable_tui);
        assert_eq!(config.ui.log_level, "debug");
        assert!(!config.ui.colored_output);
        assert!(!config.ui.show_timestamps);
    }

    // ===== Project-Level Config Loading Tests =====

    #[tokio::test]
    async fn test_load_project_config_success() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let project_config_path = temp_dir.path().join("project.toml");

        let project_toml = r#"
[server]
port = 4000

[ui]
log_level = "warn"
"#;

        fs::write(&project_config_path, project_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.project_config_path = project_config_path;

        let config = loader.load_project_config().await.unwrap();

        assert_eq!(config.server.port, 4000);
        assert_eq!(config.ui.log_level, "warn");
    }

    #[tokio::test]
    async fn test_load_project_config_file_not_found() {
        let mut loader = ConfigLoader::new();
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let result = loader.load_project_config().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_load_project_config_minimal() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let project_config_path = temp_dir.path().join("project.toml");

        // Just override log level for CI
        let project_toml = r#"
[ui]
log_level = "error"
colored_output = false
"#;

        fs::write(&project_config_path, project_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.project_config_path = project_config_path;

        let config = loader.load_project_config().await.unwrap();

        assert_eq!(config.ui.log_level, "error");
        assert!(!config.ui.colored_output);
    }

    // ===== File Not Found Handling Tests =====

    #[tokio::test]
    async fn test_load_gracefully_handles_missing_configs() {
        let mut loader = ConfigLoader::new();
        loader.user_config_path = PathBuf::from("/nonexistent/user.toml");
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        // Should not error, should return defaults
        let config = loader.load().await.unwrap();

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
    }

    #[tokio::test]
    async fn test_load_with_malformed_toml() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let project_config_path = temp_dir.path().join("project.toml");

        // Invalid TOML
        let invalid_toml = r#"
[server
port = 8080
"#;

        fs::write(&project_config_path, invalid_toml).await.unwrap();

        let loader = ConfigLoader::new();
        let result = loader.load_from_path(&project_config_path).await;

        assert!(result.is_err());
    }

    // ===== Validation Rules Tests =====

    #[test]
    fn test_validate_success() {
        let config = AcoConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_port_zero() {
        let mut config = AcoConfig::default();
        config.server.port = 0;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("port must be greater than 0"));
    }

    #[test]
    fn test_validate_invalid_session_timeout_zero() {
        let mut config = AcoConfig::default();
        config.client.session_timeout = 0;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Session timeout must be greater than 0"));
    }

    #[test]
    fn test_validate_invalid_reconnect_attempts_zero() {
        let mut config = AcoConfig::default();
        config.client.reconnect_attempts = 0;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Reconnect attempts must be at least 1"));
    }

    #[test]
    fn test_validate_invalid_log_level() {
        let mut config = AcoConfig::default();
        config.ui.log_level = "invalid_level".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid log level"));
    }

    #[test]
    fn test_validate_all_valid_log_levels() {
        let valid_levels = vec!["trace", "debug", "info", "warn", "error"];

        for level in valid_levels {
            let mut config = AcoConfig::default();
            config.ui.log_level = level.to_string();
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn test_validate_port_in_valid_range() {
        let mut config = AcoConfig::default();
        config.server.port = 65535; // Max valid port
        assert!(config.validate().is_ok());

        config.server.port = 1; // Min valid port
        assert!(config.validate().is_ok());
    }
}
