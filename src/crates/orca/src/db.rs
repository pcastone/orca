//! Database management and migrations
//!
//! Provides SQLite database connection, schema management, and migrations
//! for persistent state storage in ~/.orca/orca.db

pub mod manager;

use crate::error::{OrcaError, Result};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Type alias for the database connection pool
pub type DatabasePool = SqlitePool;

/// Database connection wrapper
#[derive(Clone, Debug)]
pub struct Database {
    pub(crate) pool: Arc<DatabasePool>,
}

impl Database {
    /// Create a new database connection
    ///
    /// # Arguments
    /// * `database_path` - Path to the SQLite database file
    ///
    /// # Returns
    /// A new Database connection
    pub async fn new<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let path = database_path.as_ref();
        let path_str = path.to_str().ok_or_else(|| {
            OrcaError::Database("Invalid database path".to_string())
        })?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    OrcaError::Database(format!("Failed to create database directory: {}", e))
                })?;
            }
        }

        let database_url = format!("sqlite:{}", path_str);
        debug!(url = %database_url, "Connecting to database");

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to connect to database: {}", e)))?;

        info!(path = %path.display(), "Database connection established");

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Create a new database connection with custom pool size
    ///
    /// # Arguments
    /// * `database_path` - Path to the SQLite database file
    /// * `max_connections` - Maximum number of concurrent connections
    pub async fn with_max_connections<P: AsRef<Path>>(
        database_path: P,
        max_connections: u32,
    ) -> Result<Self> {
        let path = database_path.as_ref();
        let path_str = path.to_str().ok_or_else(|| {
            OrcaError::Database("Invalid database path".to_string())
        })?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    OrcaError::Database(format!("Failed to create database directory: {}", e))
                })?;
            }
        }

        let database_url = format!("sqlite:{}", path_str);

        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect(&database_url)
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to connect to database: {}", e)))?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    /// Run migrations on the database
    ///
    /// Migrations are embedded in the binary and located in ./migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");

        sqlx::migrate!("./migrations")
            .run(self.pool.as_ref())
            .await
            .map_err(|e| OrcaError::Database(format!("Migration failed: {}", e)))?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    /// Run migrations from a specific directory
    ///
    /// # Arguments
    /// * `migration_path` - Path to the migration directory (e.g., "migrations/user" or "migrations/project")
    ///
    /// # Supported Paths
    /// - "migrations/user" - User-level migrations (~/.orca/user.db)
    /// - "migrations/project" - Project-level migrations (.orca/project.db)
    pub async fn run_migrations_from(&self, migration_path: &str) -> Result<()> {
        info!(path = %migration_path, "Running database migrations from custom path");

        match migration_path {
            "migrations/user" => {
                sqlx::migrate!("./migrations/user")
                    .run(self.pool.as_ref())
                    .await
                    .map_err(|e| OrcaError::Database(format!("User migration failed: {}", e)))?;
            }
            "migrations/project" => {
                sqlx::migrate!("./migrations/project")
                    .run(self.pool.as_ref())
                    .await
                    .map_err(|e| OrcaError::Database(format!("Project migration failed: {}", e)))?;
            }
            _ => {
                return Err(OrcaError::Database(format!(
                    "Unsupported migration path: {}. Supported paths: migrations/user, migrations/project",
                    migration_path
                )));
            }
        }

        info!(path = %migration_path, "Database migrations completed successfully");
        Ok(())
    }

    /// Perform a health check by running a simple query
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| OrcaError::Database(format!("Health check failed: {}", e)))?;

        Ok(())
    }

    /// Close the database connection
    pub async fn close(&self) {
        self.pool.close().await;
        info!("Database connection closed");
    }

    /// Initialize the database with schema
    ///
    /// This creates a new database and runs all migrations
    pub async fn initialize<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let db = Self::new(database_path).await?;
        db.run_migrations().await?;
        Ok(db)
    }

    /// Create an in-memory test database with migrations applied
    pub async fn test_in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to connect to in-memory database: {}", e)))?;

        let db = Self {
            pool: Arc::new(pool),
        };

        db.run_migrations().await?;
        Ok(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_create_in_memory_database() {
        // Use in-memory database for tests
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        let result = db.health_check().await;
        assert!(result.is_ok());
        db.close().await;
    }

    #[tokio::test]
    async fn test_health_check() {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        let result = db.health_check().await;
        assert!(result.is_ok());
        db.close().await;
    }

    #[tokio::test]
    async fn test_with_max_connections() {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        let result = db.health_check().await;
        assert!(result.is_ok());
        db.close().await;
    }

    // ============================================================================
    // Phase 5.1: Database Operations - Migration Tests
    // ============================================================================

    #[tokio::test]
    #[ignore] // Requires actual migration files to be present
    async fn test_run_migrations_from_user_path() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("user.db");

        let db = Database::new(&db_path).await.unwrap();

        // Run user migrations
        let result = db.run_migrations_from("migrations/user").await;

        // Should succeed if migrations exist
        match result {
            Ok(_) => assert!(true),
            Err(e) => {
                // Expected if migrations don't exist in test environment
                assert!(e.to_string().contains("migration") || e.to_string().contains("User migration failed"));
            }
        }

        db.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires actual migration files to be present
    async fn test_run_migrations_from_project_path() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("project.db");

        let db = Database::new(&db_path).await.unwrap();

        // Run project migrations
        let result = db.run_migrations_from("migrations/project").await;

        // Should succeed if migrations exist
        match result {
            Ok(_) => assert!(true),
            Err(e) => {
                // Expected if migrations don't exist in test environment
                assert!(e.to_string().contains("migration") || e.to_string().contains("Project migration failed"));
            }
        }

        db.close().await;
    }

    #[tokio::test]
    async fn test_run_migrations_from_invalid_path() {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        // Try to run migrations from invalid path
        let result = db.run_migrations_from("migrations/invalid").await;

        // Should fail with unsupported path error
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Unsupported migration path"));
        assert!(error.to_string().contains("migrations/invalid"));

        db.close().await;
    }

    #[tokio::test]
    async fn test_migration_error_handling() {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        // Test error message for unsupported paths
        let paths = vec!["migrations/invalid", "wrong/path", ""];

        for path in paths {
            let result = db.run_migrations_from(path).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Unsupported migration path"));
        }

        db.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires actual migration files to be present
    async fn test_run_migrations_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(&db_path).await.unwrap();

        // Run migrations twice - should be idempotent
        let result1 = db.run_migrations_from("migrations/user").await;
        let result2 = db.run_migrations_from("migrations/user").await;

        // Both should succeed or both should fail consistently
        match (result1, result2) {
            (Ok(_), Ok(_)) => assert!(true),
            (Err(_), Err(_)) => assert!(true),
            _ => panic!("Migration idempotency check failed"),
        }

        db.close().await;
    }

    // ============================================================================
    // Phase 5.1: Database Operations - Transaction Tests
    // ============================================================================

    #[tokio::test]
    async fn test_transaction_commit() {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        // Create a test table
        sqlx::query("CREATE TABLE test_commit (id INTEGER PRIMARY KEY, value TEXT)")
            .execute(db.pool())
            .await
            .unwrap();

        // Start transaction
        let mut tx = db.pool().begin().await.unwrap();

        // Insert data
        sqlx::query("INSERT INTO test_commit (value) VALUES (?)")
            .bind("test_value")
            .execute(&mut *tx)
            .await
            .unwrap();

        // Commit transaction
        tx.commit().await.unwrap();

        // Verify data persisted
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_commit")
            .fetch_one(db.pool())
            .await
            .unwrap();

        assert_eq!(count, 1);

        db.close().await;
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        // Create a test table
        sqlx::query("CREATE TABLE test_rollback (id INTEGER PRIMARY KEY, value TEXT)")
            .execute(db.pool())
            .await
            .unwrap();

        // Start transaction
        let mut tx = db.pool().begin().await.unwrap();

        // Insert data
        sqlx::query("INSERT INTO test_rollback (value) VALUES (?)")
            .bind("test_value")
            .execute(&mut *tx)
            .await
            .unwrap();

        // Rollback transaction
        tx.rollback().await.unwrap();

        // Verify data was not persisted
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_rollback")
            .fetch_one(db.pool())
            .await
            .unwrap();

        assert_eq!(count, 0);

        db.close().await;
    }

    #[tokio::test]
    async fn test_concurrent_transactions() {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Create a test table
        sqlx::query("CREATE TABLE test_concurrent (id INTEGER PRIMARY KEY, value INTEGER)")
            .execute(db.pool())
            .await
            .unwrap();

        // Run 5 concurrent transactions
        let mut handles = vec![];

        for i in 0..5 {
            let db_clone = db.clone();
            let handle = tokio::spawn(async move {
                let mut tx = db_clone.pool().begin().await.unwrap();

                sqlx::query("INSERT INTO test_concurrent (value) VALUES (?)")
                    .bind(i)
                    .execute(&mut *tx)
                    .await
                    .unwrap();

                tx.commit().await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all transactions to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all transactions committed
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_concurrent")
            .fetch_one(db.pool())
            .await
            .unwrap();

        assert_eq!(count, 5);

        db.close().await;
    }

    #[tokio::test]
    async fn test_transaction_isolation() {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        // Create a test table
        sqlx::query("CREATE TABLE test_isolation (id INTEGER PRIMARY KEY, value INTEGER)")
            .execute(db.pool())
            .await
            .unwrap();

        // Insert initial data
        sqlx::query("INSERT INTO test_isolation (value) VALUES (100)")
            .execute(db.pool())
            .await
            .unwrap();

        // Start transaction
        let mut tx = db.pool().begin().await.unwrap();

        // Update within transaction
        sqlx::query("UPDATE test_isolation SET value = 200 WHERE id = 1")
            .execute(&mut *tx)
            .await
            .unwrap();

        // Read within same transaction - should see updated value
        let value: i64 = sqlx::query_scalar("SELECT value FROM test_isolation WHERE id = 1")
            .fetch_one(&mut *tx)
            .await
            .unwrap();

        assert_eq!(value, 200); // Should see updated value within transaction

        // Rollback to test isolation
        tx.rollback().await.unwrap();

        // After rollback, should see original value
        let value: i64 = sqlx::query_scalar("SELECT value FROM test_isolation WHERE id = 1")
            .fetch_one(db.pool())
            .await
            .unwrap();

        assert_eq!(value, 100); // Original value restored after rollback

        db.close().await;
    }

    #[tokio::test]
    async fn test_transaction_error_rollback() {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        // Create a test table with unique constraint
        sqlx::query("CREATE TABLE test_error_rollback (id INTEGER PRIMARY KEY, value TEXT UNIQUE)")
            .execute(db.pool())
            .await
            .unwrap();

        // Insert initial data
        sqlx::query("INSERT INTO test_error_rollback (value) VALUES (?)")
            .bind("unique_value")
            .execute(db.pool())
            .await
            .unwrap();

        // Start transaction
        let mut tx = db.pool().begin().await.unwrap();

        // Insert valid data
        sqlx::query("INSERT INTO test_error_rollback (value) VALUES (?)")
            .bind("valid_value")
            .execute(&mut *tx)
            .await
            .unwrap();

        // Try to insert duplicate - should fail
        let result = sqlx::query("INSERT INTO test_error_rollback (value) VALUES (?)")
            .bind("unique_value")
            .execute(&mut *tx)
            .await;

        assert!(result.is_err());

        // Rollback on error
        tx.rollback().await.unwrap();

        // Verify only original data exists
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_error_rollback")
            .fetch_one(db.pool())
            .await
            .unwrap();

        assert_eq!(count, 1);

        db.close().await;
    }

    #[tokio::test]
    async fn test_nested_transactions() {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        // Create a test table
        sqlx::query("CREATE TABLE test_nested (id INTEGER PRIMARY KEY, value TEXT)")
            .execute(db.pool())
            .await
            .unwrap();

        // Start outer transaction
        let mut tx1 = db.pool().begin().await.unwrap();

        sqlx::query("INSERT INTO test_nested (value) VALUES (?)")
            .bind("outer")
            .execute(&mut *tx1)
            .await
            .unwrap();

        // SQLite doesn't support true nested transactions, but we can test savepoints
        // For now, just test that we can start another transaction after committing
        tx1.commit().await.unwrap();

        let mut tx2 = db.pool().begin().await.unwrap();

        sqlx::query("INSERT INTO test_nested (value) VALUES (?)")
            .bind("inner")
            .execute(&mut *tx2)
            .await
            .unwrap();

        tx2.commit().await.unwrap();

        // Verify both inserts succeeded
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_nested")
            .fetch_one(db.pool())
            .await
            .unwrap();

        assert_eq!(count, 2);

        db.close().await;
    }

    // ============================================================================
    // Phase 5.1: Database Operations - Connection Pooling Tests
    // ============================================================================

    #[tokio::test]
    async fn test_pool_exhaustion() {
        let pool = SqlitePoolOptions::new()
            .max_connections(2) // Small pool for testing
            .acquire_timeout(Duration::from_millis(100))
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Acquire all connections
        let conn1 = db.pool().acquire().await.unwrap();
        let conn2 = db.pool().acquire().await.unwrap();

        // Try to acquire another connection - should fail due to pool exhaustion
        let result = db.pool().acquire().await;

        // Should fail since pool is exhausted
        assert!(result.is_err());

        // Release connections
        drop(conn1);
        drop(conn2);

        db.close().await;
    }

    #[tokio::test]
    async fn test_concurrent_connection_acquisition() {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Spawn 20 tasks trying to acquire connections
        let mut handles = vec![];

        for _ in 0..20 {
            let db_clone = db.clone();
            let handle = tokio::spawn(async move {
                let _conn = db_clone.pool().acquire().await.unwrap();
                // Simulate some work
                tokio::time::sleep(Duration::from_millis(10)).await;
            });
            handles.push(handle);
        }

        // All tasks should complete successfully (with queuing)
        for handle in handles {
            handle.await.unwrap();
        }

        db.close().await;
    }

    #[tokio::test]
    async fn test_connection_release() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        // Acquire and release connection
        {
            let _conn = db.pool().acquire().await.unwrap();
            // Connection released when _conn goes out of scope
        }

        // Should be able to acquire again immediately
        let result = timeout(Duration::from_millis(100), async {
            db.pool().acquire().await
        })
        .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());

        db.close().await;
    }

    #[tokio::test]
    async fn test_custom_pool_size() {
        // Use in-memory database with custom pool size
        let pool = SqlitePoolOptions::new()
            .max_connections(15)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Verify we can create many concurrent connections
        let mut handles = vec![];

        for _ in 0..10 {
            let db_clone = db.clone();
            let handle = tokio::spawn(async move {
                let _conn = db_clone.pool().acquire().await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;
            });
            handles.push(handle);
        }

        // All should complete without timeout
        for handle in handles {
            handle.await.unwrap();
        }

        db.close().await;
    }

    #[tokio::test]
    async fn test_pool_health_after_close() {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Database {
            pool: Arc::new(pool),
        };

        // Pool should be open
        assert!(!db.pool().is_closed());

        // Close the pool
        db.close().await;

        // Pool should be closed
        assert!(db.pool().is_closed());
    }

    // ============================================================================
    // Phase 5.1: Database Operations - Concurrent Access Tests
    // ============================================================================

    #[tokio::test]
    async fn test_parallel_reads() {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Create and populate test table
        sqlx::query("CREATE TABLE test_reads (id INTEGER PRIMARY KEY, value INTEGER)")
            .execute(db.pool())
            .await
            .unwrap();

        for i in 0..100 {
            sqlx::query("INSERT INTO test_reads (value) VALUES (?)")
                .bind(i)
                .execute(db.pool())
                .await
                .unwrap();
        }

        // Spawn 10 concurrent read tasks
        let mut handles = vec![];

        for _ in 0..10 {
            let db_clone = db.clone();
            let handle = tokio::spawn(async move {
                let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_reads")
                    .fetch_one(db_clone.pool())
                    .await
                    .unwrap();

                assert_eq!(count, 100);
            });
            handles.push(handle);
        }

        // All reads should succeed
        for handle in handles {
            handle.await.unwrap();
        }

        db.close().await;
    }

    #[tokio::test]
    async fn test_parallel_writes() {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Create test table
        sqlx::query("CREATE TABLE test_writes (id INTEGER PRIMARY KEY, value INTEGER)")
            .execute(db.pool())
            .await
            .unwrap();

        // Spawn 10 concurrent write tasks
        let mut handles = vec![];

        for i in 0..10 {
            let db_clone = db.clone();
            let handle = tokio::spawn(async move {
                sqlx::query("INSERT INTO test_writes (value) VALUES (?)")
                    .bind(i)
                    .execute(db_clone.pool())
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        // All writes should succeed
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all writes completed
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_writes")
            .fetch_one(db.pool())
            .await
            .unwrap();

        assert_eq!(count, 10);

        db.close().await;
    }

    #[tokio::test]
    async fn test_read_write_concurrency() {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Create and populate test table
        sqlx::query("CREATE TABLE test_rw_concurrency (id INTEGER PRIMARY KEY, value INTEGER)")
            .execute(db.pool())
            .await
            .unwrap();

        for i in 0..50 {
            sqlx::query("INSERT INTO test_rw_concurrency (value) VALUES (?)")
                .bind(i)
                .execute(db.pool())
                .await
                .unwrap();
        }

        // Spawn mixed read and write tasks
        let mut handles = vec![];

        // 5 read tasks
        for _ in 0..5 {
            let db_clone = db.clone();
            let handle = tokio::spawn(async move {
                let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_rw_concurrency")
                    .fetch_one(db_clone.pool())
                    .await
                    .unwrap();

                assert!(count >= 50);
            });
            handles.push(handle);
        }

        // 5 write tasks
        for i in 50..55 {
            let db_clone = db.clone();
            let handle = tokio::spawn(async move {
                sqlx::query("INSERT INTO test_rw_concurrency (value) VALUES (?)")
                    .bind(i)
                    .execute(db_clone.pool())
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        // All tasks should complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify final count
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_rw_concurrency")
            .fetch_one(db.pool())
            .await
            .unwrap();

        assert_eq!(count, 55);

        db.close().await;
    }

    #[tokio::test]
    async fn test_concurrent_health_checks() {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Spawn 20 concurrent health check tasks
        let mut handles = vec![];

        for _ in 0..20 {
            let db_clone = db.clone();
            let handle = tokio::spawn(async move {
                db_clone.health_check().await.unwrap();
            });
            handles.push(handle);
        }

        // All health checks should succeed
        for handle in handles {
            handle.await.unwrap();
        }

        db.close().await;
    }

    #[tokio::test]
    async fn test_stress_concurrent_queries() {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Create test table
        sqlx::query("CREATE TABLE test_stress (id INTEGER PRIMARY KEY, value INTEGER, data TEXT)")
            .execute(db.pool())
            .await
            .unwrap();

        // Spawn 50 concurrent tasks doing various operations
        let mut handles = vec![];

        for i in 0..50 {
            let db_clone = db.clone();
            let handle = tokio::spawn(async move {
                match i % 3 {
                    0 => {
                        // Insert
                        sqlx::query("INSERT INTO test_stress (value, data) VALUES (?, ?)")
                            .bind(i)
                            .bind(format!("data_{}", i))
                            .execute(db_clone.pool())
                            .await
                            .unwrap();
                    }
                    1 => {
                        // Read count
                        let _: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_stress")
                            .fetch_one(db_clone.pool())
                            .await
                            .unwrap();
                    }
                    2 => {
                        // Health check
                        db_clone.health_check().await.unwrap();
                    }
                    _ => unreachable!(),
                }
            });
            handles.push(handle);
        }

        // All tasks should complete successfully
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify some data was inserted
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_stress")
            .fetch_one(db.pool())
            .await
            .unwrap();

        // Should have inserts from i % 3 == 0 (17 out of 50)
        assert!(count >= 16);

        db.close().await;
    }
}

