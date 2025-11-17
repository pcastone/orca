//! Common test utilities and setup

use orca::db::Database;
use orca::DatabaseManager;
use std::sync::Arc;
use tempfile::TempDir;

/// Create a test database with a unique in-memory instance
pub async fn setup_test_db() -> (TempDir, Arc<Database>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Use in-memory database for testing - much simpler and more reliable
    let db = Arc::new(
        Database::test_in_memory()
            .await
            .expect("Failed to create test database"),
    );

    (temp_dir, db)
}

/// Create a test DatabaseManager with a unique directory
pub async fn setup_test_manager() -> (TempDir, Arc<DatabaseManager>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create orca subdirectory
    let orca_dir = temp_dir.path().join(".orca");
    std::fs::create_dir_all(&orca_dir).expect("Failed to create orca dir");

    let manager = DatabaseManager::new(temp_dir.path().to_str().unwrap())
        .await
        .expect("Failed to create test manager");

    (temp_dir, Arc::new(manager))
}
