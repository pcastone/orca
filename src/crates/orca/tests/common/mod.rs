//! Common test utilities and setup

use orca::db::Database;
use orca::DatabaseManager;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Create a test database with a unique name
pub async fn setup_test_db() -> (TempDir, Arc<Database>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a unique database name to avoid conflicts
    let counter = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!("test_{}.db", counter);
    let db_path = temp_dir.path().join(&db_name);

    // Create parent directory
    std::fs::create_dir_all(temp_dir.path()).expect("Failed to create temp dir");

    let db = Database::new(db_path.to_str().unwrap())
        .await
        .expect("Failed to create test database");

    (temp_dir, Arc::new(db))
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
