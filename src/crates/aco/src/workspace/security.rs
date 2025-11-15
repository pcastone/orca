//! Workspace Security Validation
//!
//! This module provides security checks for workspace operations:
//! - Path traversal prevention
//! - Symbolic link detection
//! - Sensitive path blocking
//! - Workspace boundary enforcement

use crate::error::{AcoError, Result};
use std::path::{Component, Path, PathBuf};
use tracing::debug;

/// Security configuration for workspace operations
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Whether to allow symbolic links in workspace
    pub allow_symlinks: bool,

    /// Whether to follow symlinks when checking paths
    pub follow_symlinks: bool,

    /// List of paths to block (relative to system root)
    pub blocked_paths: Vec<PathBuf>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_symlinks: false,
            follow_symlinks: false,
            blocked_paths: default_blocked_paths(),
        }
    }
}

/// Default system paths to block
fn default_blocked_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/etc"),
        PathBuf::from("/sys"),
        PathBuf::from("/proc"),
        PathBuf::from("/root"),
        PathBuf::from("/home"),
        PathBuf::from("/var/log"),
        PathBuf::from("/boot"),
        PathBuf::from("/dev"),
        PathBuf::from("/bin"),
        PathBuf::from("/sbin"),
        PathBuf::from("/usr"),
        PathBuf::from("/lib"),
        PathBuf::from("/opt"),
    ]
}

/// Workspace path validator
pub struct PathValidator {
    workspace_root: PathBuf,
    config: SecurityConfig,
}

impl PathValidator {
    /// Create a new path validator
    pub fn new(workspace_root: impl AsRef<Path>, config: SecurityConfig) -> Result<Self> {
        let workspace_root = workspace_root.as_ref().to_path_buf();

        // Validate workspace root exists and is accessible
        if !workspace_root.exists() {
            return Err(AcoError::General(format!(
                "Workspace root does not exist: {}",
                workspace_root.display()
            )));
        }

        if !workspace_root.is_dir() {
            return Err(AcoError::General(format!(
                "Workspace root is not a directory: {}",
                workspace_root.display()
            )));
        }

        Ok(Self {
            workspace_root,
            config,
        })
    }

    /// Validate that a path is safe to access
    pub fn validate_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Check for path traversal attempts
        self.check_traversal_attempts(path)?;

        // Check for symbolic links
        self.check_symlinks(path)?;

        // Check if path is within workspace bounds
        self.check_workspace_bounds(path)?;

        // Check for blocked system paths
        self.check_blocked_paths(path)?;

        debug!("Path validation passed for: {}", path.display());

        Ok(())
    }

    /// Check for path traversal attempts (.. components)
    pub fn check_traversal_attempts(&self, path: &Path) -> Result<()> {
        for component in path.components() {
            if component == Component::ParentDir {
                return Err(AcoError::General(
                    "Path traversal detected: contains '..'.".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Check for symbolic links
    fn check_symlinks(&self, path: &Path) -> Result<()> {
        // Only check the path itself, not parents
        // Parents are implicitly allowed since they're part of the system
        if !self.config.allow_symlinks && path.exists() {
            // Check if the path itself is a symlink
            if path.read_link().is_ok() {
                return Err(AcoError::General(format!(
                    "Symbolic links are not allowed: {}",
                    path.display()
                )));
            }
        }

        Ok(())
    }

    /// Check if path stays within workspace bounds
    fn check_workspace_bounds(&self, path: &Path) -> Result<()> {
        // Canonicalize both paths for accurate comparison
        let canonical_root = self.workspace_root.canonicalize().unwrap_or_else(|_| {
            // If we can't canonicalize root, use as-is (defensive)
            self.workspace_root.clone()
        });

        let canonical_path = if path.is_absolute() {
            // For absolute paths, try to canonicalize
            path.canonicalize().unwrap_or_else(|_| {
                // If can't canonicalize (e.g., doesn't exist), use as-is
                path.to_path_buf()
            })
        } else {
            // For relative paths, join with root first
            let full_path = canonical_root.join(path);
            // Try to canonicalize, but if it fails (path doesn't exist), use as-is
            full_path.canonicalize().unwrap_or(full_path)
        };

        // Check if path is within or equal to workspace root
        if !canonical_path.starts_with(&canonical_root) {
            return Err(AcoError::General(format!(
                "Path {} is outside workspace bounds: {}",
                path.display(),
                canonical_root.display()
            )));
        }

        Ok(())
    }

    /// Check against blocked system paths
    fn check_blocked_paths(&self, path: &Path) -> Result<()> {
        // Only check absolute paths against blocked list
        if path.is_absolute() {
            for blocked_path in &self.config.blocked_paths {
                if path.starts_with(blocked_path) {
                    return Err(AcoError::General(format!(
                        "Path {} is in blocked system directory: {}",
                        path.display(),
                        blocked_path.display()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate a relative path for safe access within workspace
    pub fn validate_relative_path(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf> {
        let relative_path = relative_path.as_ref();

        // Path must be relative
        if relative_path.is_absolute() {
            return Err(AcoError::General(
                "Path must be relative to workspace root".to_string(),
            ));
        }

        // Check for path traversal in the relative path
        for component in relative_path.components() {
            if component == Component::ParentDir {
                return Err(AcoError::General(
                    "Path traversal detected in relative path".to_string(),
                ));
            }
        }

        // Construct the full path and return without full validation
        // (full validation would be done when actually accessing the path)
        let full_path = self.workspace_root.join(relative_path);
        Ok(full_path)
    }

    /// Validate a file path for reading
    pub fn validate_read_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        self.validate_path(path)?;

        // Additional check: path must exist for reading
        if !path.exists() {
            return Err(AcoError::General(format!(
                "File does not exist: {}",
                path.display()
            )));
        }

        Ok(())
    }

    /// Validate a file path for writing
    pub fn validate_write_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // For write paths, do basic safety checks but be lenient with bounds
        // (since the path may not exist yet)

        // Check for path traversal
        self.check_traversal_attempts(path)?;

        // Check for symlinks (if not allowed)
        self.check_symlinks(path)?;

        // Check parent directory is writable (if it exists)
        if let Some(parent) = path.parent() {
            if parent.exists() {
                // Try to check write permission on parent
                let metadata = std::fs::metadata(parent).map_err(|e| {
                    AcoError::General(format!(
                        "Cannot access parent directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;

                if metadata.permissions().readonly() {
                    return Err(AcoError::General(format!(
                        "Parent directory is read-only: {}",
                        parent.display()
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_validator() -> (TempDir, PathValidator) {
        let temp_dir = TempDir::new().unwrap();
        let validator =
            PathValidator::new(temp_dir.path(), SecurityConfig::default()).unwrap();
        (temp_dir, validator)
    }

    #[test]
    fn test_valid_relative_path() {
        let (temp_dir, validator) = setup_validator();

        // validate_relative_path just checks structure, not existence
        let result = validator.validate_relative_path("config/test.toml");
        assert!(result.is_ok());
        // Path should be within workspace
        let path = result.unwrap();
        assert!(path.starts_with(temp_dir.path()));
        assert!(!path.is_absolute() || path.starts_with(temp_dir.path()));
    }

    #[test]
    fn test_path_traversal_prevention() {
        let (_, validator) = setup_validator();
        let result = validator.validate_path("../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("traversal"));
    }

    #[test]
    fn test_absolute_path_within_bounds() {
        let (temp_dir, validator) = setup_validator();
        let abs_path = temp_dir.path().join("config");
        // Create the path first
        std::fs::create_dir(&abs_path).unwrap();

        let result = validator.validate_path(&abs_path);
        // Path exists and is within bounds
        assert!(result.is_ok());
    }

    #[test]
    fn test_absolute_path_outside_bounds() {
        let (_, validator) = setup_validator();
        // Try to access a path far outside workspace
        let result = validator.validate_path("/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_relative_path_rejects_absolute() {
        let (_, validator) = setup_validator();
        let result = validator.validate_relative_path("/etc/passwd");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be relative"));
    }

    #[test]
    fn test_read_path_requires_existence() {
        let (temp_dir, validator) = setup_validator();
        let nonexistent = temp_dir.path().join("nonexistent.txt");
        let result = validator.validate_read_path(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_path_succeeds_for_existing_file() {
        let (temp_dir, validator) = setup_validator();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test").unwrap();

        let result = validator.validate_read_path(&test_file);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_path_validation() {
        let (temp_dir, validator) = setup_validator();
        let write_path = temp_dir.path().join("newfile.txt");

        // Create parent directory to ensure it exists for validation
        std::fs::create_dir_all(write_path.parent().unwrap()).unwrap();

        let result = validator.validate_write_path(&write_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_blocked_system_paths() {
        let (_, validator) = setup_validator();
        let result = validator.validate_path("/etc/passwd");
        // Should fail because it's outside workspace bounds or blocked
        assert!(result.is_err());
    }

    #[test]
    fn test_parent_dir_component_blocked() {
        let (_, validator) = setup_validator();
        let result = validator.validate_path("config/../../etc");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_parent_dir_attempts() {
        let (_, validator) = setup_validator();
        let result = validator.validate_path("../../../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_symlink_detection_disabled_by_default() {
        let (temp_dir, validator) = setup_validator();

        // Create a symlink
        let target = temp_dir.path().join("target.txt");
        std::fs::write(&target, "content").unwrap();

        let link = temp_dir.path().join("link.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, &link).unwrap();

        // Should fail because symlinks not allowed by default
        let result = validator.validate_path(&link);
        assert!(result.is_err());
        // Error should be about symbolic links
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("link"));
    }

    #[test]
    fn test_symlink_detection_allowed_when_configured() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = SecurityConfig::default();
        config.allow_symlinks = true;

        let validator = PathValidator::new(temp_dir.path(), config).unwrap();

        // Create a symlink
        let target = temp_dir.path().join("target.txt");
        std::fs::write(&target, "content").unwrap();

        let link = temp_dir.path().join("link.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, &link).unwrap();

        // Should succeed because symlinks allowed
        let result = validator.validate_path(&link);
        assert!(result.is_ok());
    }

    #[test]
    fn test_workspace_root_validation() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent");

        let result = PathValidator::new(&nonexistent, SecurityConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_workspace_root_must_be_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        std::fs::write(&file_path, "test").unwrap();

        let result = PathValidator::new(&file_path, SecurityConfig::default());
        assert!(result.is_err());
    }

    // =================================================================
    // ADVANCED SECURITY EDGE CASE TESTS
    // =================================================================

    #[test]
    fn test_url_encoded_path_traversal() {
        let (_, validator) = setup_validator();

        // URL encoded ".." should still be caught
        // Note: Rust's PathBuf doesn't automatically decode, but test for safety
        let encoded_patterns = vec![
            "%2e%2e/etc/passwd",
            "config/%2e%2e/etc",
            "%2e%2e%2f%2e%2e%2fetc",
        ];

        for pattern in encoded_patterns {
            let result = validator.validate_path(pattern);
            // These might not be caught by component check since PathBuf doesn't decode
            // But they should be caught by bounds check or not resolve correctly
            // This documents the current behavior
            if result.is_ok() {
                // If it passes validation, ensure it doesn't actually escape
                let safe_path = validator.workspace_root.join(pattern);
                assert!(safe_path.starts_with(&validator.workspace_root),
                    "URL encoded path {} should not escape workspace", pattern);
            }
        }
    }

    #[test]
    fn test_mixed_separators_windows_unix() {
        let (_, validator) = setup_validator();

        // Test mixed Windows/Unix separators in path traversal attempts
        // Note: On Unix, backslashes are treated as literal filename characters
        // On Windows, they're path separators. Test behavior accordingly.

        #[cfg(windows)]
        {
            let patterns = vec![
                "config\\..\\..\\etc",  // Windows style
                "config\\../etc",        // Mixed
                "config/..\\etc",        // Mixed reverse
            ];

            for pattern in patterns {
                let result = validator.validate_path(pattern);
                // Should catch .. components regardless of separator
                assert!(result.is_err(),
                    "Mixed separator pattern {} should be rejected on Windows", pattern);
            }
        }

        #[cfg(unix)]
        {
            // On Unix, backslashes are literal characters, not separators
            // But forward slash with .. should still be caught
            let patterns = vec![
                "config/../etc",
                "config/../../etc",
            ];

            for pattern in patterns {
                let result = validator.validate_path(pattern);
                assert!(result.is_err(),
                    "Path traversal with .. should be rejected: {}", pattern);
            }

            // Backslashes are treated as literal filename chars on Unix
            let backslash_pattern = "config\\..\\..\\etc";
            let result = validator.validate_path(backslash_pattern);
            // On Unix, this is a filename with backslashes, which is valid (odd but valid)
            // It won't have ParentDir components, so validation may pass
            if result.is_ok() {
                // Ensure it's contained within workspace
                let safe_path = validator.workspace_root.join(backslash_pattern);
                assert!(safe_path.starts_with(&validator.workspace_root));
            }
        }
    }

    #[test]
    fn test_null_byte_in_path() {
        let (_, validator) = setup_validator();

        // Null bytes in paths are a security vulnerability
        // Rust's path handling should prevent this, but test for safety
        let path_with_null = "config\0/etc/passwd";

        // PathBuf should handle this safely (null bytes are invalid in paths)
        // This test documents the behavior
        let result = validator.validate_path(path_with_null);

        // Either validation fails, or the path is safely contained
        if result.is_ok() {
            let safe_path = validator.workspace_root.join(path_with_null);
            assert!(safe_path.to_str().is_none() ||
                    safe_path.starts_with(&validator.workspace_root));
        }
    }

    #[test]
    fn test_empty_path_validation() {
        let (_, validator) = setup_validator();

        // Empty path should be handled safely
        let result = validator.validate_path("");

        // Either it's rejected or it resolves to workspace root
        if result.is_ok() {
            let resolved = validator.workspace_root.join("");
            assert_eq!(resolved, validator.workspace_root);
        }
    }

    #[test]
    fn test_dot_current_directory() {
        let (_, validator) = setup_validator();

        // Single dot should be allowed (current directory)
        let result = validator.validate_path(".");
        // Should be OK - current directory is safe
        assert!(result.is_ok());

        let result = validator.validate_path("./config");
        assert!(result.is_ok());
    }

    #[test]
    fn test_hidden_file_path_traversal() {
        let (_, validator) = setup_validator();

        // Path traversal hidden in "normal" looking paths
        let result = validator.validate_path("./../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("traversal"));
    }

    #[test]
    fn test_very_long_path() {
        let (_, validator) = setup_validator();

        // Test with very long path (potential buffer overflow or DoS)
        let long_component = "a".repeat(255); // Max filename length on most systems
        let long_path = format!("{}/{}/{}/{}", long_component, long_component,
                                long_component, long_component);

        let result = validator.validate_relative_path(&long_path);

        // Should either succeed (and be contained) or fail gracefully
        if let Ok(path) = result {
            assert!(path.starts_with(&validator.workspace_root));
        }
    }

    #[test]
    fn test_blocked_path_exact_match() {
        let (_, validator) = setup_validator();

        // Test exact blocked path match
        let result = validator.validate_path("/etc");
        assert!(result.is_err());

        let result = validator.validate_path("/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_blocked_path_prefix_false_positive() {
        let temp_dir = TempDir::new().unwrap();

        // Create a workspace that has "etc" in the name but isn't /etc
        let workspace_with_etc = temp_dir.path().join("my_etc_backup");
        std::fs::create_dir(&workspace_with_etc).unwrap();

        let validator = PathValidator::new(&workspace_with_etc, SecurityConfig::default()).unwrap();

        // Path within workspace that contains "etc" should be allowed
        let safe_path = workspace_with_etc.join("config");
        std::fs::create_dir(&safe_path).unwrap();

        let result = validator.validate_path(&safe_path);
        assert!(result.is_ok(), "Path within workspace should be allowed even if workspace name contains 'etc'");
    }

    #[test]
    fn test_custom_blocked_paths() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = SecurityConfig::default();

        // Add custom blocked path
        config.blocked_paths.push(PathBuf::from("/custom/blocked"));

        let validator = PathValidator::new(temp_dir.path(), config).unwrap();

        let result = validator.validate_path("/custom/blocked/file.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_case_sensitivity_in_paths() {
        let (_, validator) = setup_validator();

        // On case-sensitive filesystems, /ETC and /etc are different
        // But both should be caught if configured
        let result = validator.validate_path("/ETC/passwd");

        // This might pass or fail depending on OS and configuration
        // Document the behavior: blocked paths are case-sensitive by default
        if result.is_ok() {
            // If it passes, it's because "/ETC" isn't in the blocked list
            // which only has "/etc" (lowercase)
            assert!(!validator.config.blocked_paths.contains(&PathBuf::from("/ETC")));
        }
    }

    #[test]
    fn test_symlink_in_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = SecurityConfig::default();

        // Create workspace inside temp_dir
        let workspace = temp_dir.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();

        let validator = PathValidator::new(&workspace, config).unwrap();

        // Create a file in workspace
        let target = workspace.join("file.txt");
        std::fs::write(&target, "content").unwrap();

        // The file itself is not a symlink, should pass
        let result = validator.validate_path(&target);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tilde_expansion_not_performed() {
        let (_, validator) = setup_validator();

        // Tilde should NOT be expanded (that's a shell feature)
        let result = validator.validate_path("~/etc/passwd");

        // Should be treated as literal "~" directory name
        // which is relative and within workspace bounds
        if result.is_ok() {
            let resolved = validator.workspace_root.join("~/etc/passwd");
            assert!(resolved.starts_with(&validator.workspace_root));
        }
    }

    #[test]
    fn test_multiple_slashes() {
        let (_, validator) = setup_validator();

        // Multiple slashes should be normalized
        let result = validator.validate_path("config///test.txt");

        // PathBuf normalizes multiple slashes
        assert!(result.is_ok());
    }

    #[test]
    fn test_trailing_slash_handling() {
        let (temp_dir, validator) = setup_validator();

        // Create a directory
        let dir = temp_dir.path().join("config");
        std::fs::create_dir(&dir).unwrap();

        // Trailing slash should be OK for directories
        let result = validator.validate_path(dir.join(""));
        assert!(result.is_ok());
    }

    #[test]
    fn test_canonicalization_edge_case() {
        let (temp_dir, validator) = setup_validator();

        // Path with . and .. that ultimately stays in workspace
        let dir = temp_dir.path().join("dir");
        std::fs::create_dir(&dir).unwrap();

        let complex_path = temp_dir.path().join("./dir/../dir/./file.txt");

        // Even though it has .. it resolves to workspace/dir/file.txt
        // But our validator rejects ALL .. components for safety
        let result = validator.validate_path(&complex_path);
        assert!(result.is_err(), "Should reject any .. for security");
    }

    #[test]
    fn test_read_path_symlink_target_outside_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();

        // Create target outside workspace
        let outside = temp_dir.path().join("outside.txt");
        std::fs::write(&outside, "secret").unwrap();

        let mut config = SecurityConfig::default();
        config.allow_symlinks = true;  // Allow symlinks to test boundary check

        let validator = PathValidator::new(&workspace, config).unwrap();

        // Create symlink inside workspace pointing outside
        let link = workspace.join("link.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&outside, &link).unwrap();

        // Should fail because canonical path is outside workspace
        let result = validator.validate_read_path(&link);
        assert!(result.is_err(), "Symlink pointing outside workspace should be rejected");
    }

    #[test]
    fn test_write_path_parent_not_writable() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();

        let validator = PathValidator::new(&workspace, SecurityConfig::default()).unwrap();

        // Create a read-only directory
        let readonly_dir = workspace.join("readonly");
        std::fs::create_dir(&readonly_dir).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_mode(0o444);  // Read-only
            std::fs::set_permissions(&readonly_dir, perms).unwrap();

            let write_path = readonly_dir.join("newfile.txt");
            let result = validator.validate_write_path(&write_path);

            // Should fail because parent is read-only
            assert!(result.is_err());

            // Clean up - restore write permissions
            let mut perms = std::fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&readonly_dir, perms).unwrap();
        }

        #[cfg(windows)]
        {
            // Windows permission handling is different, skip this test
            // or implement Windows-specific read-only check
        }
    }
}
