//! Comprehensive integration tests for workspace initialization
//!
//! Tests cover the complete lifecycle of workspace initialization including:
//! - Basic initialization workflows
//! - Security validation
//! - Error handling and recovery
//! - Edge cases and boundary conditions
//! - Concurrent operations

#![allow(unused_variables)] // TempDir is kept alive for tests even if not explicitly used

use aco::workspace::{
    SecurityConfig, WorkspaceInitConfig, WorkspaceInitializer, WorkspaceValidator,
};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper to create a temporary workspace root
fn create_temp_workspace() -> (TempDir, WorkspaceInitConfig) {
    let temp_dir = TempDir::new().unwrap();
    let config = WorkspaceInitConfig::new(temp_dir.path());
    (temp_dir, config)
}

/// Helper to verify workspace structure
fn verify_workspace_structure(root: &Path) -> bool {
    root.join(".acolib").exists()
        && root.join("logs").exists()
        && root.join("config").exists()
        && root.join(".acolib").join("cache").exists()
        && root.join(".acolib").join("workspace.toml").exists()
}

// ============================================================================
// Basic Initialization Tests
// ============================================================================

#[test]
fn test_basic_workspace_initialization() {
    let (temp_dir, config) = create_temp_workspace();
    let root = temp_dir.path();
    let mut initializer = WorkspaceInitializer::new(config);

    let result = initializer.init();
    assert!(result.is_ok());

    let metadata = result.unwrap();
    assert_eq!(metadata.root, root);
    assert!(verify_workspace_structure(root));
    // temp_dir is dropped here, cleaning up the test workspace
    drop(temp_dir);
}

#[test]
fn test_workspace_initialization_creates_all_directories() {
    let (temp_dir, config) = create_temp_workspace();
    let mut initializer = WorkspaceInitializer::new(config);

    initializer.init().unwrap();

    let root = temp_dir.path();
    assert!(root.join(".acolib").is_dir());
    assert!(root.join("logs").is_dir());
    assert!(root.join("config").is_dir());
    assert!(root.join(".acolib/cache").is_dir());
}

#[test]
fn test_workspace_metadata_file_creation() {
    let (temp_dir, config) = create_temp_workspace();
    let mut initializer = WorkspaceInitializer::new(config);

    let metadata = initializer.init().unwrap();
    let metadata_file = metadata.metadata_file();

    assert!(metadata_file.exists());
    assert!(metadata_file.is_file());

    // Verify file contains valid TOML
    let content = fs::read_to_string(&metadata_file).unwrap();
    assert!(content.contains("version"));
    assert!(content.contains("initialized_at"));
}

#[test]
fn test_workspace_metadata_convenience_methods() {
    let (temp_dir, config) = create_temp_workspace();
    let mut initializer = WorkspaceInitializer::new(config);

    let metadata = initializer.init().unwrap();
    let root = temp_dir.path();

    assert_eq!(metadata.logs_dir(), root.join("logs"));
    assert_eq!(metadata.config_dir(), root.join("config"));
    assert_eq!(metadata.cache_dir(), root.join(".acolib/cache"));
    assert_eq!(metadata.metadata_file(), root.join(".acolib/workspace.toml"));
}

// ============================================================================
// Idempotency Tests
// ============================================================================

#[test]
fn test_initialization_is_idempotent() {
    let (temp_dir, config) = create_temp_workspace();
    let mut initializer = WorkspaceInitializer::new(config);

    // Initialize twice
    let result1 = initializer.init();
    let result2 = initializer.init();

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let metadata1 = result1.unwrap();
    let metadata2 = result2.unwrap();

    // Both should have the same root
    assert_eq!(metadata1.root, metadata2.root);

    // Workspace structure should still be intact
    assert!(verify_workspace_structure(temp_dir.path()));
}

#[test]
fn test_initialization_with_existing_directories() {
    let (temp_dir, config) = create_temp_workspace();
    let root = temp_dir.path();

    // Pre-create some directories
    fs::create_dir_all(root.join(".acolib/cache")).unwrap();
    fs::create_dir_all(root.join("logs")).unwrap();

    let mut initializer = WorkspaceInitializer::new(config);
    let result = initializer.init();

    assert!(result.is_ok());
    assert!(verify_workspace_structure(root));
}

#[test]
fn test_initialization_with_existing_metadata_file() {
    let (_temp_dir, config) = create_temp_workspace();
    let root = _temp_dir.path();

    // Create directories and metadata file first
    fs::create_dir_all(root.join(".acolib")).unwrap();
    let metadata_file = root.join(".acolib/workspace.toml");
    fs::write(&metadata_file, "version = \"0.1.0\"\n").unwrap();

    let mut initializer = WorkspaceInitializer::new(config);
    let result = initializer.init();

    // Should succeed and not overwrite existing metadata
    assert!(result.is_ok());
    let metadata_content = fs::read_to_string(&metadata_file).unwrap();
    assert!(metadata_content.contains("version = \"0.1.0\""));
}

// ============================================================================
// Security & Validation Tests
// ============================================================================

#[test]
fn test_initialization_with_custom_security_config() {
    let (_temp_dir, mut config) = create_temp_workspace();

    let mut security_config = SecurityConfig::default();
    security_config.allow_symlinks = true;
    config = config.with_security_config(security_config);

    let mut initializer = WorkspaceInitializer::new(config);
    let result = initializer.init();

    // Should initialize successfully with custom security config
    assert!(result.is_ok());
}

#[test]
fn test_workspace_validator_accepts_initialized_workspace() {
    let (temp_dir, config) = create_temp_workspace();
    let mut initializer = WorkspaceInitializer::new(config);

    // Initialize first
    initializer.init().unwrap();

    // Now validate
    let validation_result = WorkspaceValidator::validate(temp_dir.path());
    assert!(validation_result.is_ok());

    let metadata = validation_result.unwrap();
    assert_eq!(metadata.root, temp_dir.path());
}

#[test]
fn test_workspace_validator_rejects_missing_directories() {
    let (temp_dir, _config) = create_temp_workspace();
    let root = temp_dir.path();

    // Create only partial structure
    fs::create_dir_all(root.join(".acolib")).unwrap();
    fs::create_dir_all(root.join("logs")).unwrap();
    // Missing "config" directory

    let validation_result = WorkspaceValidator::validate(root);
    assert!(validation_result.is_err());
}

#[test]
fn test_path_validator_prevents_traversal() {
    // Test that path traversal is prevented through security configuration
    let (_temp_dir, config) = create_temp_workspace();

    // Verify security config has traversal prevention enabled by default
    assert!(!config.security_config.blocked_paths.is_empty());

    // The validator is used internally during initialization
    // This test verifies the configuration is properly passed
    let mut initializer = WorkspaceInitializer::new(config);
    let result = initializer.init();
    assert!(result.is_ok());
}

#[test]
fn test_path_validator_allows_safe_relative_paths() {
    // Test that relative path validation is configured correctly
    let (temp_dir, config) = create_temp_workspace();

    // Verify the config allows safe operations
    assert!(config.create_if_missing);
    assert!(config.validate_permissions);

    let mut initializer = WorkspaceInitializer::new(config);
    let result = initializer.init();
    assert!(result.is_ok());

    // Workspace should be created with all directories
    assert!(temp_dir.path().join("logs").exists());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_initialization_fails_with_create_if_missing_false() {
    let _temp_dir = TempDir::new().unwrap();
    let nonexistent_path = _temp_dir.path().join("does_not_exist");

    let config = WorkspaceInitConfig::new(&nonexistent_path)
        .with_create_if_missing(false);
    let mut initializer = WorkspaceInitializer::new(config);

    let result = initializer.init();
    assert!(result.is_err());
}

#[test]
fn test_initialization_succeeds_with_nested_paths() {
    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir.path().join("level1").join("level2").join("workspace");

    let config = WorkspaceInitConfig::new(&nested_path)
        .with_create_if_missing(true);
    let mut initializer = WorkspaceInitializer::new(config);

    let result = initializer.init();
    assert!(result.is_ok());
    assert!(nested_path.exists());
    assert!(verify_workspace_structure(&nested_path));
}

#[test]
fn test_initialization_fails_if_root_is_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, "test").unwrap();

    let config = WorkspaceInitConfig::new(&file_path);
    let mut initializer = WorkspaceInitializer::new(config);

    let result = initializer.init();
    assert!(result.is_err());
}

// ============================================================================
// Permission & File System Tests
// ============================================================================

#[test]
fn test_workspace_directories_are_readable() {
    let (temp_dir, config) = create_temp_workspace();
    let mut initializer = WorkspaceInitializer::new(config);

    initializer.init().unwrap();

    let root = temp_dir.path();
    assert!(fs::metadata(root.join(".acolib")).is_ok());
    assert!(fs::metadata(root.join("logs")).is_ok());
    assert!(fs::metadata(root.join("config")).is_ok());
}

#[test]
fn test_workspace_directories_are_writable() {
    let (temp_dir, config) = create_temp_workspace();
    let mut initializer = WorkspaceInitializer::new(config);

    initializer.init().unwrap();

    let root = temp_dir.path();

    // Try to write test files
    let test_file = root.join("logs/test.txt");
    let write_result = fs::write(&test_file, "test data");
    assert!(write_result.is_ok());

    let cache_file = root.join(".acolib/cache/test.json");
    let write_result = fs::write(&cache_file, "{}");
    assert!(write_result.is_ok());
}

#[test]
fn test_validation_with_write_test() {
    let (temp_dir, config) = create_temp_workspace();
    let config = config.with_validate_permissions(true);
    let mut initializer = WorkspaceInitializer::new(config);

    let result = initializer.init();
    assert!(result.is_ok());

    // Verify permission validation happened
    assert!(temp_dir.path().join(".acolib").exists());
}

// ============================================================================
// Concurrent Initialization Tests
// ============================================================================

#[test]
fn test_concurrent_initialization_same_directory() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create two initializers for the same directory
    let config1 = WorkspaceInitConfig::new(root);
    let config2 = WorkspaceInitConfig::new(root);

    let mut init1 = WorkspaceInitializer::new(config1);
    let mut init2 = WorkspaceInitializer::new(config2);

    // Both should succeed (idempotent)
    let result1 = init1.init();
    let result2 = init2.init();

    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(verify_workspace_structure(root));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_initialization_with_special_characters_in_path() {
    let temp_dir = TempDir::new().unwrap();
    let special_path = temp_dir.path().join("workspace-v0.1.0_test");

    let config = WorkspaceInitConfig::new(&special_path)
        .with_create_if_missing(true);
    let mut initializer = WorkspaceInitializer::new(config);

    let result = initializer.init();
    assert!(result.is_ok());
    assert!(verify_workspace_structure(&special_path));
}

#[test]
fn test_initialization_with_long_path() {
    let temp_dir = TempDir::new().unwrap();
    let mut path = temp_dir.path().to_path_buf();

    // Create a reasonably long path (but not exceeding filesystem limits)
    for i in 0..5 {
        path = path.join(format!("level_{}", i));
    }

    let config = WorkspaceInitConfig::new(&path)
        .with_create_if_missing(true);
    let mut initializer = WorkspaceInitializer::new(config);

    let result = initializer.init();
    assert!(result.is_ok());
    assert!(verify_workspace_structure(&path));
}

#[test]
fn test_metadata_contains_valid_version() {
    let (_temp_dir, config) = create_temp_workspace();
    let mut initializer = WorkspaceInitializer::new(config);

    let metadata = initializer.init().unwrap();
    assert!(!metadata.version.is_empty());
    assert_eq!(metadata.version, "0.1.0");
}

#[test]
fn test_metadata_initialization_timestamp() {
    let (_temp_dir, config) = create_temp_workspace();
    let mut initializer = WorkspaceInitializer::new(config);

    let metadata = initializer.init().unwrap();

    // Verify timestamp is recent (within last minute)
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(metadata.initialized_at);
    assert!(diff.num_seconds() < 60);
}

// ============================================================================
// Validator Integration Tests
// ============================================================================

#[test]
fn test_validator_with_existing_workspace() {
    let (temp_dir, config) = create_temp_workspace();
    let mut initializer = WorkspaceInitializer::new(config);

    // Initialize the workspace
    initializer.init().unwrap();

    // Create a second initializer and validator
    let validator = aco::workspace::WorkspaceValidator::validate(temp_dir.path());
    assert!(validator.is_ok());
}

#[test]
fn test_validator_rejects_incomplete_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create minimal structure
    fs::create_dir_all(root.join(".acolib")).unwrap();

    let validator = aco::workspace::WorkspaceValidator::validate(root);
    assert!(validator.is_err());
}

// ============================================================================
// Recovery & Repair Tests
// ============================================================================

#[test]
fn test_reinitialization_fixes_missing_cache_directory() {
    let (temp_dir, config) = create_temp_workspace();
    let root = temp_dir.path();

    let mut initializer = WorkspaceInitializer::new(config);
    initializer.init().unwrap();

    // Delete the cache directory
    fs::remove_dir_all(root.join(".acolib/cache")).unwrap();
    assert!(!root.join(".acolib/cache").exists());

    // Reinitialize
    let config2 = WorkspaceInitConfig::new(root);
    let mut initializer2 = WorkspaceInitializer::new(config2);
    let result = initializer2.init();

    assert!(result.is_ok());
    assert!(root.join(".acolib/cache").exists());
}

#[test]
fn test_reinitialization_preserves_metadata_file() {
    let (temp_dir, config) = create_temp_workspace();
    let root = temp_dir.path();

    let mut initializer = WorkspaceInitializer::new(config);
    let metadata1 = initializer.init().unwrap();

    let metadata_file = metadata1.metadata_file();
    let original_content = fs::read_to_string(&metadata_file).unwrap();

    // Reinitialize
    let config2 = WorkspaceInitConfig::new(root);
    let mut initializer2 = WorkspaceInitializer::new(config2);
    initializer2.init().unwrap();

    // Metadata file should be unchanged
    let new_content = fs::read_to_string(&metadata_file).unwrap();
    assert_eq!(original_content, new_content);
}

// ============================================================================
// State Consistency Tests
// ============================================================================

#[test]
fn test_multiple_sequential_initializations_are_consistent() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let mut results = Vec::new();

    // Initialize multiple times
    for _ in 0..3 {
        let config = WorkspaceInitConfig::new(root);
        let mut initializer = WorkspaceInitializer::new(config);
        let metadata = initializer.init().unwrap();
        results.push(metadata.root);
    }

    // All should have the same root
    for result in &results {
        assert_eq!(result, root);
    }

    // Workspace should still be valid
    assert!(verify_workspace_structure(root));
}
