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
    use tempfile::TempDir;

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

    // ===== Comprehensive Config Merging Tests =====

    #[tokio::test]
    async fn test_config_merging_priority_user_overrides_defaults() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[llm]
provider = "openai"
model = "gpt-4"
temperature = 0.5

[execution]
max_concurrent_tasks = 10
"#;
        fs::write(&user_config_path, user_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let config = loader.load().await.unwrap();

        // User config should override defaults
        assert_eq!(config.llm.provider, "openai");
        assert_eq!(config.llm.model, "gpt-4");
        assert_eq!(config.llm.temperature, 0.5);
        assert_eq!(config.execution.max_concurrent_tasks, 10);

        // Unspecified fields should remain defaults
        assert_eq!(config.database.path, "orca.db");
        assert_eq!(config.logging.level, "info");
    }

    #[tokio::test]
    async fn test_config_merging_priority_project_overrides_user() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");
        let project_config_path = temp_dir.path().join("project.toml");

        let user_toml = r#"
[llm]
provider = "openai"
model = "gpt-4"
temperature = 0.5

[execution]
max_concurrent_tasks = 10
"#;

        let project_toml = r#"
[llm]
model = "gpt-3.5-turbo"
temperature = 0.9

[database]
path = "/tmp/project.db"
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();
        fs::write(&project_config_path, project_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = project_config_path;

        let config = loader.load().await.unwrap();

        // Project config should override user config
        assert_eq!(config.llm.model, "gpt-3.5-turbo");
        assert_eq!(config.llm.temperature, 0.9);
        assert_eq!(config.database.path, "/tmp/project.db");

        // User-only fields preserved (provider wasn't in project config)
        assert_eq!(config.llm.provider, "openai");
        assert_eq!(config.execution.max_concurrent_tasks, 10);
    }

    #[tokio::test]
    async fn test_config_merging_three_levels() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");
        let project_config_path = temp_dir.path().join("project.toml");

        let user_toml = r#"
[llm]
provider = "openai"
model = "gpt-4"

[execution]
max_concurrent_tasks = 8
"#;

        let project_toml = r#"
[execution]
max_concurrent_tasks = 15
streaming = false
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();
        fs::write(&project_config_path, project_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = project_config_path;

        let config = loader.load().await.unwrap();

        // Priority: defaults < user < project
        assert_eq!(config.llm.provider, "openai"); // From user
        assert_eq!(config.llm.model, "gpt-4"); // From user
        assert_eq!(config.execution.max_concurrent_tasks, 15); // From project (overrides user)
        assert!(!config.execution.streaming); // From project
        assert_eq!(config.database.path, "orca.db"); // From defaults
        assert_eq!(config.logging.level, "info"); // From defaults
    }

    #[tokio::test]
    async fn test_config_merging_partial_sections() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        // Only override logging section
        let user_toml = r#"
[logging]
level = "debug"
format = "json"
colored = false
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let config = loader.load().await.unwrap();

        // Logging section from user config
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.logging.format, "json");
        assert!(!config.logging.colored);

        // Other sections from defaults
        assert_eq!(config.llm.provider, "anthropic");
        assert_eq!(config.database.path, "orca.db");
        assert_eq!(config.execution.max_concurrent_tasks, 5);
    }

    #[tokio::test]
    async fn test_config_merging_empty_user_config() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        // Empty config file (just whitespace/comments)
        let user_toml = r#"
# This is an empty config file
        "#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let config = loader.load().await.unwrap();

        // Should use all defaults
        assert_eq!(config.llm.provider, "anthropic");
        assert_eq!(config.database.path, "orca.db");
        assert_eq!(config.execution.max_concurrent_tasks, 5);
    }

    // ===== User-Level Config Loading Tests =====

    #[tokio::test]
    async fn test_load_user_config_success() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[llm]
provider = "anthropic"
model = "claude-3-opus"

[execution]
max_concurrent_tasks = 20
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;

        let config = loader.load_user_config().await.unwrap();

        assert_eq!(config.llm.provider, "anthropic");
        assert_eq!(config.llm.model, "claude-3-opus");
        assert_eq!(config.execution.max_concurrent_tasks, 20);
    }

    #[tokio::test]
    async fn test_load_user_config_file_not_found() {
        let mut loader = ConfigLoader::new();
        loader.user_config_path = PathBuf::from("/nonexistent/user.toml");

        let result = loader.load_user_config().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, OrcaError::Config(_)));
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_load_user_config_with_all_sections() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[database]
path = "custom.db"

[llm]
provider = "openai"
model = "gpt-4-turbo"
temperature = 0.3
max_tokens = 8000

[execution]
max_concurrent_tasks = 12
task_timeout = 600
streaming = false
max_iterations = 20

[logging]
level = "trace"
format = "pretty"
colored = true
timestamps = false
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;

        let config = loader.load_user_config().await.unwrap();

        assert_eq!(config.database.path, "custom.db");
        assert_eq!(config.llm.provider, "openai");
        assert_eq!(config.llm.model, "gpt-4-turbo");
        assert_eq!(config.llm.temperature, 0.3);
        assert_eq!(config.llm.max_tokens, 8000);
        assert_eq!(config.execution.max_concurrent_tasks, 12);
        assert_eq!(config.execution.task_timeout, 600);
        assert!(!config.execution.streaming);
        assert_eq!(config.execution.max_iterations, 20);
        assert_eq!(config.logging.level, "trace");
        assert_eq!(config.logging.format, "pretty");
        assert!(config.logging.colored);
        assert!(!config.logging.timestamps);
    }

    // ===== Project-Level Config Loading Tests =====

    #[tokio::test]
    async fn test_load_project_config_success() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let project_config_path = temp_dir.path().join("project.toml");

        let project_toml = r#"
[llm]
provider = "gemini"
model = "gemini-pro"

[database]
path = "/project/data.db"
"#;

        fs::write(&project_config_path, project_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.project_config_path = project_config_path;

        let config = loader.load_project_config().await.unwrap();

        assert_eq!(config.llm.provider, "gemini");
        assert_eq!(config.llm.model, "gemini-pro");
        assert_eq!(config.database.path, "/project/data.db");
    }

    #[tokio::test]
    async fn test_load_project_config_file_not_found() {
        let mut loader = ConfigLoader::new();
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let result = loader.load_project_config().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, OrcaError::Config(_)));
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_load_project_config_overrides_for_ci() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let project_config_path = temp_dir.path().join("project.toml");

        // Typical CI/CD overrides
        let project_toml = r#"
[execution]
max_concurrent_tasks = 1
streaming = false

[logging]
level = "warn"
format = "json"
colored = false
"#;

        fs::write(&project_config_path, project_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.project_config_path = project_config_path;

        let config = loader.load_project_config().await.unwrap();

        assert_eq!(config.execution.max_concurrent_tasks, 1);
        assert!(!config.execution.streaming);
        assert_eq!(config.logging.level, "warn");
        assert_eq!(config.logging.format, "json");
        assert!(!config.logging.colored);
    }

    // ===== File Not Found Handling Tests =====

    #[tokio::test]
    async fn test_load_from_path_nonexistent_file() {
        let loader = ConfigLoader::new();
        let nonexistent = PathBuf::from("/this/path/does/not/exist/config.toml");

        let result = loader.load_from_path(&nonexistent).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, OrcaError::Config(_)));
        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("config.toml"));
    }

    #[tokio::test]
    async fn test_load_gracefully_handles_missing_configs() {
        let mut loader = ConfigLoader::new();
        loader.user_config_path = PathBuf::from("/nonexistent/user.toml");
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        // Should not error, should return defaults
        let config = loader.load().await.unwrap();

        assert_eq!(config.llm.provider, "anthropic");
        assert_eq!(config.database.path, "orca.db");
    }

    #[tokio::test]
    async fn test_load_with_invalid_toml_syntax() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        // Invalid TOML syntax
        let invalid_toml = r#"
[llm
provider = "openai"
"#;

        fs::write(&user_config_path, invalid_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let result = loader.load().await;

        // Should fail to parse and fall back to defaults gracefully
        // or return an error - current implementation returns defaults
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_load_with_malformed_toml_content() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let project_config_path = temp_dir.path().join("project.toml");

        // Syntactically valid TOML but wrong types
        let malformed_toml = r#"
[llm]
max_tokens = "should be number not string"
"#;

        fs::write(&project_config_path, malformed_toml).await.unwrap();

        let loader = ConfigLoader::new();
        let result = loader.load_from_path(&project_config_path).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, OrcaError::Config(_)));
        assert!(err.to_string().contains("Failed to parse"));
    }

    #[tokio::test]
    async fn test_user_config_exists_when_file_present() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        fs::write(&user_config_path, "[llm]\n").await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path.clone();

        assert!(loader.user_config_exists());
        assert!(user_config_path.exists());
    }

    #[tokio::test]
    async fn test_project_config_exists_when_file_present() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let project_config_path = temp_dir.path().join("project.toml");

        fs::write(&project_config_path, "[database]\n").await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.project_config_path = project_config_path.clone();

        assert!(loader.project_config_exists());
        assert!(project_config_path.exists());
    }

    // ===== Environment Variable Expansion Tests =====

    #[tokio::test]
    async fn test_env_var_expansion_in_api_key() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[llm]
provider = "openai"
api_key = "${ORCA_TEST_API_KEY}"
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        std::env::set_var("ORCA_TEST_API_KEY", "sk-test-123456");

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let config = loader.load().await.unwrap();

        assert_eq!(config.llm.api_key, Some("sk-test-123456".to_string()));

        std::env::remove_var("ORCA_TEST_API_KEY");
    }

    #[tokio::test]
    async fn test_env_var_expansion_in_api_base() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[llm]
provider = "openai"
api_base = "${ORCA_TEST_API_BASE}"
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        std::env::set_var("ORCA_TEST_API_BASE", "https://custom.api.com/v1");

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let config = loader.load().await.unwrap();

        assert_eq!(config.llm.api_base, Some("https://custom.api.com/v1".to_string()));

        std::env::remove_var("ORCA_TEST_API_BASE");
    }

    #[tokio::test]
    async fn test_env_var_expansion_missing_var_keeps_placeholder() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[llm]
provider = "anthropic"
api_key = "${MISSING_VAR_THAT_DOES_NOT_EXIST}"
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let config = loader.load().await.unwrap();

        // Should keep the placeholder when var doesn't exist
        assert_eq!(config.llm.api_key, Some("${MISSING_VAR_THAT_DOES_NOT_EXIST}".to_string()));
    }

    #[tokio::test]
    async fn test_env_var_expansion_literal_value_unchanged() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[llm]
provider = "openai"
api_key = "sk-literal-key-12345"
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let config = loader.load().await.unwrap();

        // Literal values should not be expanded
        assert_eq!(config.llm.api_key, Some("sk-literal-key-12345".to_string()));
    }

    #[tokio::test]
    async fn test_env_var_expansion_multiple_vars() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[llm]
provider = "openai"
api_key = "${ORCA_TEST_KEY_1}"
api_base = "${ORCA_TEST_BASE_1}"
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        std::env::set_var("ORCA_TEST_KEY_1", "key-value-1");
        std::env::set_var("ORCA_TEST_BASE_1", "https://base-1.com");

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let config = loader.load().await.unwrap();

        assert_eq!(config.llm.api_key, Some("key-value-1".to_string()));
        assert_eq!(config.llm.api_base, Some("https://base-1.com".to_string()));

        std::env::remove_var("ORCA_TEST_KEY_1");
        std::env::remove_var("ORCA_TEST_BASE_1");
    }

    #[tokio::test]
    async fn test_env_var_expansion_project_overrides_user() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");
        let project_config_path = temp_dir.path().join("project.toml");

        let user_toml = r#"
[llm]
api_key = "${USER_API_KEY}"
"#;

        let project_toml = r#"
[llm]
api_key = "${PROJECT_API_KEY}"
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();
        fs::write(&project_config_path, project_toml).await.unwrap();

        std::env::set_var("USER_API_KEY", "user-key-123");
        std::env::set_var("PROJECT_API_KEY", "project-key-456");

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = project_config_path;

        let config = loader.load().await.unwrap();

        // Project env var should take precedence
        assert_eq!(config.llm.api_key, Some("project-key-456".to_string()));

        std::env::remove_var("USER_API_KEY");
        std::env::remove_var("PROJECT_API_KEY");
    }

    #[tokio::test]
    async fn test_env_var_expansion_empty_string() {
        use tokio::fs;
        let temp_dir = TempDir::new().unwrap();
        let user_config_path = temp_dir.path().join("user.toml");

        let user_toml = r#"
[llm]
api_key = "${ORCA_EMPTY_VAR}"
"#;

        fs::write(&user_config_path, user_toml).await.unwrap();

        std::env::set_var("ORCA_EMPTY_VAR", "");

        let mut loader = ConfigLoader::new();
        loader.user_config_path = user_config_path;
        loader.project_config_path = PathBuf::from("/nonexistent/project.toml");

        let config = loader.load().await.unwrap();

        // Empty env var should expand to empty string
        assert_eq!(config.llm.api_key, Some("".to_string()));

        std::env::remove_var("ORCA_EMPTY_VAR");
    }
}
