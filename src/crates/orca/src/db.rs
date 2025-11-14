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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

