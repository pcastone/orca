//! Workspace Initialization
//!
//! This module handles initialization and validation of acolib workspaces.
//! A workspace is a directory containing configuration, logs, and other state.

use crate::error::{AcoError, Result};
use super::security::{PathValidator, SecurityConfig};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Configuration for workspace initialization
#[derive(Debug, Clone)]
pub struct WorkspaceInitConfig {
    /// Root directory of the workspace
    pub root: PathBuf,

    /// Whether to create directories if they don't exist
    pub create_if_missing: bool,

    /// Whether to validate write permissions
    pub validate_permissions: bool,

    /// Security configuration for path validation
    pub security_config: SecurityConfig,
}

impl WorkspaceInitConfig {
    /// Create a new configuration with the given root path
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            create_if_missing: true,
            validate_permissions: true,
            security_config: SecurityConfig::default(),
        }
    }

    /// Set whether to create directories if they don't exist
    pub fn with_create_if_missing(mut self, create: bool) -> Self {
        self.create_if_missing = create;
        self
    }

    /// Set whether to validate write permissions
    pub fn with_validate_permissions(mut self, validate: bool) -> Self {
        self.validate_permissions = validate;
        self
    }

    /// Set security configuration
    pub fn with_security_config(mut self, config: SecurityConfig) -> Self {
        self.security_config = config;
        self
    }
}

/// Workspace initializer
pub struct WorkspaceInitializer {
    config: WorkspaceInitConfig,
    path_validator: Option<PathValidator>,
}

impl WorkspaceInitializer {
    /// Create a new workspace initializer
    pub fn new(config: WorkspaceInitConfig) -> Self {
        Self {
            config,
            path_validator: None,
        }
    }

    /// Initialize a workspace
    ///
    /// This method:
    /// 1. Validates the root directory
    /// 2. Creates security validator
    /// 3. Creates necessary subdirectories with security validation
    /// 4. Initializes workspace metadata
    /// 5. Validates permissions
    pub fn init(&mut self) -> Result<WorkspaceMetadata> {
        debug!(
            "Initializing workspace at {}",
            self.config.root.display()
        );

        // Ensure root directory exists
        self.ensure_root_dir()?;

        // Validate root directory is accessible
        self.validate_root_dir()?;

        // Create security validator
        let validator = PathValidator::new(&self.config.root, self.config.security_config.clone())?;
        self.path_validator = Some(validator);

        // Create subdirectories with security validation
        self.create_subdirectories()?;

        // Validate write permissions
        if self.config.validate_permissions {
            self.validate_write_permissions()?;
        }

        // Create workspace metadata
        let metadata = self.create_workspace_metadata()?;

        info!(
            "Workspace initialized successfully at {}",
            self.config.root.display()
        );

        Ok(metadata)
    }

    /// Ensure the root directory exists
    fn ensure_root_dir(&self) -> Result<()> {
        if !self.config.root.exists() {
            if self.config.create_if_missing {
                debug!(
                    "Creating root directory: {}",
                    self.config.root.display()
                );
                fs::create_dir_all(&self.config.root)?;
            } else {
                return Err(AcoError::General(format!(
                    "Workspace root does not exist: {}",
                    self.config.root.display()
                )));
            }
        }

        Ok(())
    }

    /// Validate the root directory
    fn validate_root_dir(&self) -> Result<()> {
        if !self.config.root.is_dir() {
            return Err(AcoError::General(format!(
                "Workspace root is not a directory: {}",
                self.config.root.display()
            )));
        }

        Ok(())
    }

    /// Create required subdirectories
    fn create_subdirectories(&self) -> Result<()> {
        let subdirs = vec![
            ".acolib",      // Hidden config directory
            "logs",         // Log files
            "config",       // Configuration files
            ".acolib/cache", // Cache directory
        ];

        for subdir in subdirs {
            let path = self.config.root.join(subdir);

            // Validate path traversal before creating
            // (don't do full bounds check since path won't exist yet)
            if let Some(ref validator) = self.path_validator {
                // Only check for path traversal, not bounds
                validator.check_traversal_attempts(&path)?;
            }

            if !path.exists() {
                debug!("Creating directory: {}", path.display());
                fs::create_dir_all(&path)?;
            }
        }

        Ok(())
    }

    /// Validate write permissions
    fn validate_write_permissions(&self) -> Result<()> {
        // Try to write a test file to verify permissions
        let test_file = self.config.root.join(".acolib").join(".write_test");

        match fs::write(&test_file, b"test") {
            Ok(_) => {
                // Clean up test file
                let _ = fs::remove_file(&test_file);
                debug!("Write permissions validated");
                Ok(())
            }
            Err(e) => Err(AcoError::General(format!(
                "No write permission in workspace {}: {}",
                self.config.root.display(),
                e
            ))),
        }
    }

    /// Create workspace metadata
    fn create_workspace_metadata(&self) -> Result<WorkspaceMetadata> {
        let metadata = WorkspaceMetadata {
            root: self.config.root.clone(),
            initialized_at: chrono::Utc::now(),
            version: "0.1.0".to_string(),
        };

        // Write metadata to file (if needed in the future)
        let metadata_path = self.config.root.join(".acolib").join("workspace.toml");
        if !metadata_path.exists() {
            let toml_content = format!(
                r#"# acolib Workspace Configuration
# This file is auto-generated

version = "{}"
initialized_at = "{}"
"#,
                metadata.version, metadata.initialized_at
            );

            fs::write(&metadata_path, toml_content)?;
        }

        Ok(metadata)
    }
}

/// Workspace metadata
#[derive(Debug, Clone)]
pub struct WorkspaceMetadata {
    /// Root directory of the workspace
    pub root: PathBuf,

    /// When the workspace was initialized
    pub initialized_at: chrono::DateTime<chrono::Utc>,

    /// Version of the workspace format
    pub version: String,
}

impl WorkspaceMetadata {
    /// Get the logs directory path
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> PathBuf {
        self.root.join(".acolib").join("cache")
    }

    /// Get the workspace metadata file path
    pub fn metadata_file(&self) -> PathBuf {
        self.root.join(".acolib").join("workspace.toml")
    }
}

/// Workspace validator
pub struct WorkspaceValidator;

impl WorkspaceValidator {
    /// Validate an existing workspace
    pub fn validate(root: impl AsRef<Path>) -> Result<WorkspaceMetadata> {
        let root = root.as_ref();

        debug!("Validating workspace at {}", root.display());

        // Check root exists and is directory
        if !root.exists() {
            return Err(AcoError::General(format!(
                "Workspace root does not exist: {}",
                root.display()
            )));
        }

        if !root.is_dir() {
            return Err(AcoError::General(format!(
                "Workspace root is not a directory: {}",
                root.display()
            )));
        }

        // Check required subdirectories exist
        let required_dirs = vec![".acolib", "logs", "config"];
        for dir in required_dirs {
            let path = root.join(dir);
            if !path.exists() {
                return Err(AcoError::General(format!(
                    "Workspace missing required directory: {}",
                    dir
                )));
            }
        }

        // Check write permission
        let test_file = root.join(".acolib").join(".permission_test");
        match fs::write(&test_file, b"") {
            Ok(_) => {
                let _ = fs::remove_file(&test_file);
            }
            Err(e) => {
                return Err(AcoError::General(format!(
                    "No write permission in workspace {}: {}",
                    root.display(),
                    e
                )))
            }
        }

        info!("Workspace validated successfully at {}", root.display());

        Ok(WorkspaceMetadata {
            root: root.to_path_buf(),
            initialized_at: chrono::Utc::now(),
            version: "0.1.0".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = WorkspaceInitConfig::new(root);
        let mut initializer = WorkspaceInitializer::new(config);
        let metadata = initializer.init().unwrap();

        // Check root is set correctly
        assert_eq!(metadata.root, root);

        // Check subdirectories were created
        assert!(root.join(".acolib").exists());
        assert!(root.join("logs").exists());
        assert!(root.join("config").exists());
        assert!(root.join(".acolib").join("cache").exists());

        // Check metadata file was created
        assert!(metadata.metadata_file().exists());
    }

    #[test]
    fn test_workspace_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = WorkspaceInitConfig::new(root);
        let mut initializer = WorkspaceInitializer::new(config);

        // Initialize twice
        let metadata1 = initializer.init().unwrap();
        let metadata2 = initializer.init().unwrap();

        // Both should succeed and have same root
        assert_eq!(metadata1.root, metadata2.root);

        // Directories should still exist
        assert!(root.join(".acolib").exists());
        assert!(root.join("logs").exists());
    }

    #[test]
    fn test_workspace_creation_with_missing_parent() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("workspace");

        let config = WorkspaceInitConfig::new(&nested_path).with_create_if_missing(true);
        let mut initializer = WorkspaceInitializer::new(config);
        let metadata = initializer.init().unwrap();

        // Check nested directory was created
        assert!(nested_path.exists());
        assert_eq!(metadata.root, nested_path);
    }

    #[test]
    fn test_workspace_validation_fails_without_create() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist");

        let config = WorkspaceInitConfig::new(&nonexistent).with_create_if_missing(false);
        let mut initializer = WorkspaceInitializer::new(config);

        let result = initializer.init();
        assert!(result.is_err());
    }

    #[test]
    fn test_workspace_validator() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Initialize workspace first
        let config = WorkspaceInitConfig::new(root);
        let mut initializer = WorkspaceInitializer::new(config);
        initializer.init().unwrap();

        // Now validate it
        let validation = WorkspaceValidator::validate(root);
        assert!(validation.is_ok());
    }

    #[test]
    fn test_workspace_validator_missing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create partial workspace (missing required directories)
        fs::create_dir(root.join(".acolib")).unwrap();

        // Validation should fail
        let validation = WorkspaceValidator::validate(root);
        assert!(validation.is_err());
    }

    #[test]
    fn test_workspace_metadata_paths() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = WorkspaceInitConfig::new(root);
        let mut initializer = WorkspaceInitializer::new(config);
        let metadata = initializer.init().unwrap();

        // Check convenience methods
        assert_eq!(metadata.logs_dir(), root.join("logs"));
        assert_eq!(metadata.config_dir(), root.join("config"));
        assert_eq!(metadata.cache_dir(), root.join(".acolib").join("cache"));
        assert_eq!(metadata.metadata_file(), root.join(".acolib").join("workspace.toml"));
    }

    #[test]
    fn test_workspace_not_a_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "test").unwrap();

        let config = WorkspaceInitConfig::new(&file_path);
        let mut initializer = WorkspaceInitializer::new(config);

        let result = initializer.init();
        assert!(result.is_err());
    }

    #[test]
    fn test_path_traversal_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = WorkspaceInitConfig::new(root);
        let mut initializer = WorkspaceInitializer::new(config);

        // This should fail during init because path traversal is blocked
        let result = initializer.init();
        assert!(result.is_ok()); // init succeeds, but paths are validated

        // Now try to access a path with traversal
        if let Some(ref validator) = initializer.path_validator {
            let result = validator.validate_path("../../../etc/passwd");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("traversal"));
        }
    }

    #[test]
    fn test_security_config_customization() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let mut security_config = SecurityConfig::default();
        security_config.allow_symlinks = true;

        let config = WorkspaceInitConfig::new(root).with_security_config(security_config);
        let mut initializer = WorkspaceInitializer::new(config);

        let result = initializer.init();
        assert!(result.is_ok());
        assert!(initializer.path_validator.is_some());
    }

    #[test]
    fn test_blocked_system_paths() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = WorkspaceInitConfig::new(root);
        let mut initializer = WorkspaceInitializer::new(config);
        let _ = initializer.init();

        // Check that blocked paths are rejected
        if let Some(ref validator) = initializer.path_validator {
            let result = validator.validate_path("/etc/passwd");
            // Should fail because it's outside workspace
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_validate_relative_path_helper() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let config = WorkspaceInitConfig::new(root);
        let mut initializer = WorkspaceInitializer::new(config);
        let _ = initializer.init();

        if let Some(ref validator) = initializer.path_validator {
            // Valid relative path
            let result = validator.validate_relative_path("config/app.toml");
            assert!(result.is_ok());

            // Absolute path should fail
            let result = validator.validate_relative_path("/etc/passwd");
            assert!(result.is_err());
        }
    }
}
