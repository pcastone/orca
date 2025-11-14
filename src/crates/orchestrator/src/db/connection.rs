//! Database connection management
//!
//! Provides database connection pooling, health checks, and connection statistics.

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Type alias for the database connection pool
pub type DatabasePool = SqlitePool;

/// Database connection statistics
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    /// Number of currently idle connections
    pub idle_connections: u32,

    /// Number of currently active connections
    pub active_connections: u32,

    /// Maximum allowed connections
    pub max_connections: u32,

    /// Total number of connections acquired since pool creation
    pub total_connections_acquired: u64,

    /// Total number of connections released since pool creation
    pub total_connections_released: u64,

    /// Timestamp of the statistics collection (Unix timestamp in seconds)
    pub collected_at: u64,
}

/// Database connection wrapper
#[derive(Clone)]
pub struct DatabaseConnection {
    pool: Arc<DatabasePool>,
}

impl DatabaseConnection {
    /// Create a new database connection from a connection string
    ///
    /// # Arguments
    /// * `database_url` - SQLite connection string (e.g., "sqlite:db.db" or "sqlite::memory:")
    ///
    /// # Returns
    /// A new DatabaseConnection or an sqlx error
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Create a new database connection with custom pool size
    ///
    /// # Arguments
    /// * `database_url` - SQLite connection string
    /// * `max_connections` - Maximum number of concurrent connections
    ///
    /// # Returns
    /// A new DatabaseConnection or an sqlx error
    pub async fn with_max_connections(
        database_url: &str,
        max_connections: u32,
    ) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;

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
    /// # Returns
    /// Success or migration error
    pub async fn run_migrations(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("./migrations")
            .run(self.pool.as_ref())
            .await
    }

    /// Perform a health check by running a simple query
    ///
    /// # Returns
    /// Success (Ok(())) if database is healthy, error otherwise
    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1")
            .fetch_one(self.pool.as_ref())
            .await?;

        Ok(())
    }

    /// Get connection pool statistics
    ///
    /// # Returns
    /// PoolStatistics containing current pool state
    pub fn get_pool_statistics(&self) -> PoolStatistics {
        let pool_ref = self.pool.as_ref();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let idle = pool_ref.num_idle() as u32;
        let total = pool_ref.size() as u32;

        PoolStatistics {
            idle_connections: idle,
            active_connections: total.saturating_sub(idle),
            max_connections: total,
            total_connections_acquired: 0, // SQLite pool doesn't expose this metric
            total_connections_released: 0, // SQLite pool doesn't expose this metric
            collected_at: now,
        }
    }

    /// Check if the connection pool is healthy
    ///
    /// # Returns
    /// True if all connections are available, false otherwise
    pub fn is_pool_healthy(&self) -> bool {
        let stats = self.get_pool_statistics();
        stats.active_connections < stats.max_connections
    }

    /// Close the connection pool gracefully
    ///
    /// Closes all connections in the pool. After this is called,
    /// the connection cannot be used anymore.
    pub async fn close(self) {
        self.pool.close().await;
    }

    /// Wait for the pool to have at least one available connection
    ///
    /// # Arguments
    /// * `timeout_secs` - Maximum seconds to wait
    ///
    /// # Returns
    /// Success if a connection became available, error if timeout occurred
    pub async fn wait_for_connection(&self, timeout_secs: u64) -> Result<(), String> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        loop {
            if self.get_pool_statistics().idle_connections > 0 {
                return Ok(());
            }

            if start.elapsed() > timeout {
                return Err(format!(
                    "Timeout waiting for connection after {} seconds",
                    timeout_secs
                ));
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_connection() {
        let conn = DatabaseConnection::new("sqlite::memory:")
            .await
            .unwrap();

        assert!(conn.pool().acquire().await.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_success() {
        let conn = DatabaseConnection::new("sqlite::memory:")
            .await
            .unwrap();

        assert!(conn.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_get_pool_statistics() {
        let conn = DatabaseConnection::new("sqlite::memory:")
            .await
            .unwrap();

        let stats = conn.get_pool_statistics();
        assert!(stats.idle_connections > 0);
        assert_eq!(stats.max_connections, 5);
    }

    #[tokio::test]
    async fn test_is_pool_healthy() {
        let conn = DatabaseConnection::new("sqlite::memory:")
            .await
            .unwrap();

        assert!(conn.is_pool_healthy());
    }

    #[tokio::test]
    async fn test_wait_for_connection_success() {
        let conn = DatabaseConnection::new("sqlite::memory:")
            .await
            .unwrap();

        let result = conn.wait_for_connection(1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_close_connection() {
        let conn = DatabaseConnection::new("sqlite::memory:")
            .await
            .unwrap();

        conn.close().await;
        // After close, new operations should fail
    }

    #[tokio::test]
    async fn test_custom_max_connections() {
        let conn = DatabaseConnection::with_max_connections("sqlite::memory:", 10)
            .await
            .unwrap();

        let stats = conn.get_pool_statistics();
        assert_eq!(stats.max_connections, 10);
    }

    #[tokio::test]
    async fn test_pool_statistics_has_timestamp() {
        let conn = DatabaseConnection::new("sqlite::memory:")
            .await
            .unwrap();

        let stats = conn.get_pool_statistics();
        assert!(stats.collected_at > 0);
    }
}

