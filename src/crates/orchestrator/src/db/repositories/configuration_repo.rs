//! Configuration repository for database operations

use crate::db::connection::DatabasePool;
use crate::db::models::Configuration;
use chrono::Utc;

/// Configuration repository for managing configuration database operations
pub struct ConfigurationRepository;

impl ConfigurationRepository {
    /// Create or update a configuration entry
    pub async fn set(
        pool: &DatabasePool,
        key: String,
        value: String,
        value_type: String,
    ) -> Result<Configuration, sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        // Check if exists
        let exists = sqlx::query_as::<_, Configuration>(
            "SELECT key, value, value_type, description, is_secret, created_at, updated_at FROM configurations WHERE key = ?"
        )
        .bind(&key)
        .fetch_optional(pool)
        .await?;

        if exists.is_some() {
            // Update
            sqlx::query(
                "UPDATE configurations SET value = ?, value_type = ?, updated_at = ? WHERE key = ?"
            )
            .bind(&value)
            .bind(&value_type)
            .bind(&now)
            .bind(&key)
            .execute(pool)
            .await?;
        } else {
            // Insert
            sqlx::query(
                "INSERT INTO configurations (key, value, value_type, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(&key)
            .bind(&value)
            .bind(&value_type)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
        }

        sqlx::query_as::<_, Configuration>(
            "SELECT key, value, value_type, description, is_secret, created_at, updated_at FROM configurations WHERE key = ?"
        )
        .bind(&key)
        .fetch_one(pool)
        .await
    }

    /// Get a configuration by key
    pub async fn get(
        pool: &DatabasePool,
        key: &str,
    ) -> Result<Option<Configuration>, sqlx::Error> {
        sqlx::query_as::<_, Configuration>(
            "SELECT key, value, value_type, description, is_secret, created_at, updated_at FROM configurations WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(pool)
        .await
    }

    /// Get all configurations
    pub async fn list(pool: &DatabasePool) -> Result<Vec<Configuration>, sqlx::Error> {
        sqlx::query_as::<_, Configuration>(
            "SELECT key, value, value_type, description, is_secret, created_at, updated_at FROM configurations ORDER BY key ASC"
        )
        .fetch_all(pool)
        .await
    }

    /// Get configurations by type
    pub async fn list_by_type(
        pool: &DatabasePool,
        value_type: &str,
    ) -> Result<Vec<Configuration>, sqlx::Error> {
        sqlx::query_as::<_, Configuration>(
            "SELECT key, value, value_type, description, is_secret, created_at, updated_at FROM configurations WHERE value_type = ? ORDER BY key ASC"
        )
        .bind(value_type)
        .fetch_all(pool)
        .await
    }

    /// Get all secret configurations
    pub async fn list_secrets(pool: &DatabasePool) -> Result<Vec<Configuration>, sqlx::Error> {
        sqlx::query_as::<_, Configuration>(
            "SELECT key, value, value_type, description, is_secret, created_at, updated_at FROM configurations WHERE is_secret = 1 ORDER BY key ASC"
        )
        .fetch_all(pool)
        .await
    }

    /// Update configuration value only
    pub async fn update_value(
        pool: &DatabasePool,
        key: &str,
        value: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE configurations SET value = ?, updated_at = ? WHERE key = ?")
            .bind(value)
            .bind(&now)
            .bind(key)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Update configuration description
    pub async fn update_description(
        pool: &DatabasePool,
        key: &str,
        description: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE configurations SET description = ?, updated_at = ? WHERE key = ?"
        )
        .bind(description)
        .bind(&now)
        .bind(key)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Mark a configuration as secret
    pub async fn mark_secret(pool: &DatabasePool, key: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE configurations SET is_secret = ?, updated_at = ? WHERE key = ?")
            .bind(1)
            .bind(&now)
            .bind(key)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Unmark a configuration as secret
    pub async fn unmark_secret(pool: &DatabasePool, key: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE configurations SET is_secret = ?, updated_at = ? WHERE key = ?")
            .bind(0)
            .bind(&now)
            .bind(key)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete a configuration
    pub async fn delete(pool: &DatabasePool, key: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM configurations WHERE key = ?")
            .bind(key)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete all configurations (use with caution)
    pub async fn delete_all(pool: &DatabasePool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM configurations")
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Count total configurations
    pub async fn count(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM configurations")
            .fetch_one(pool)
            .await?;

        Ok(result.0)
    }

    /// Check if a configuration key exists
    pub async fn exists(pool: &DatabasePool, key: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("SELECT 1 FROM configurations WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;

        Ok(result.is_some())
    }

    /// Get configuration keys matching a prefix
    pub async fn list_by_prefix(
        pool: &DatabasePool,
        prefix: &str,
    ) -> Result<Vec<Configuration>, sqlx::Error> {
        let pattern = format!("{}%", prefix);
        sqlx::query_as::<_, Configuration>(
            "SELECT key, value, value_type, description, is_secret, created_at, updated_at FROM configurations WHERE key LIKE ? ORDER BY key ASC"
        )
        .bind(&pattern)
        .fetch_all(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> sqlx::sqlite::SqlitePool {
        let pool = sqlx::sqlite::SqlitePool::connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE configurations (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL,
                value_type TEXT NOT NULL DEFAULT 'string',
                description TEXT,
                is_secret INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                CHECK (is_secret IN (0, 1)),
                CHECK (value_type IN ('string', 'integer', 'float', 'boolean', 'json'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_set_new_configuration() {
        let pool = setup_db().await;

        let config = ConfigurationRepository::set(
            &pool,
            "app.name".to_string(),
            "Orchestrator".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(config.key, "app.name");
        assert_eq!(config.value, "Orchestrator");
    }

    #[tokio::test]
    async fn test_get_configuration() {
        let pool = setup_db().await;

        ConfigurationRepository::set(
            &pool,
            "app.name".to_string(),
            "Orchestrator".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        let config = ConfigurationRepository::get(&pool, "app.name")
            .await
            .unwrap();

        assert!(config.is_some());
        assert_eq!(config.unwrap().value, "Orchestrator");
    }

    #[tokio::test]
    async fn test_update_value() {
        let pool = setup_db().await;

        ConfigurationRepository::set(
            &pool,
            "app.name".to_string(),
            "Orchestrator".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        ConfigurationRepository::update_value(&pool, "app.name", "Updated Name")
            .await
            .unwrap();

        let config = ConfigurationRepository::get(&pool, "app.name")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(config.value, "Updated Name");
    }

    #[tokio::test]
    async fn test_list_configurations() {
        let pool = setup_db().await;

        ConfigurationRepository::set(
            &pool,
            "app.name".to_string(),
            "Orchestrator".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        ConfigurationRepository::set(
            &pool,
            "app.version".to_string(),
            "1.0.0".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        let configs = ConfigurationRepository::list(&pool).await.unwrap();
        assert_eq!(configs.len(), 2);
    }

    #[tokio::test]
    async fn test_list_by_type() {
        let pool = setup_db().await;

        ConfigurationRepository::set(
            &pool,
            "app.name".to_string(),
            "Orchestrator".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        ConfigurationRepository::set(
            &pool,
            "app.port".to_string(),
            "8080".to_string(),
            "integer".to_string(),
        )
        .await
        .unwrap();

        let strings = ConfigurationRepository::list_by_type(&pool, "string")
            .await
            .unwrap();
        let integers = ConfigurationRepository::list_by_type(&pool, "integer")
            .await
            .unwrap();

        assert_eq!(strings.len(), 1);
        assert_eq!(integers.len(), 1);
    }

    #[tokio::test]
    async fn test_mark_secret() {
        let pool = setup_db().await;

        ConfigurationRepository::set(
            &pool,
            "db.password".to_string(),
            "secret123".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        ConfigurationRepository::mark_secret(&pool, "db.password")
            .await
            .unwrap();

        let config = ConfigurationRepository::get(&pool, "db.password")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(config.is_secret, 1);
    }

    #[tokio::test]
    async fn test_delete_configuration() {
        let pool = setup_db().await;

        ConfigurationRepository::set(
            &pool,
            "app.name".to_string(),
            "Orchestrator".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        ConfigurationRepository::delete(&pool, "app.name")
            .await
            .unwrap();

        let config = ConfigurationRepository::get(&pool, "app.name")
            .await
            .unwrap();

        assert!(config.is_none());
    }

    #[tokio::test]
    async fn test_exists() {
        let pool = setup_db().await;

        ConfigurationRepository::set(
            &pool,
            "app.name".to_string(),
            "Orchestrator".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        let exists = ConfigurationRepository::exists(&pool, "app.name")
            .await
            .unwrap();
        let not_exists = ConfigurationRepository::exists(&pool, "nonexistent")
            .await
            .unwrap();

        assert!(exists);
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_list_by_prefix() {
        let pool = setup_db().await;

        ConfigurationRepository::set(
            &pool,
            "app.name".to_string(),
            "Orchestrator".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        ConfigurationRepository::set(
            &pool,
            "app.version".to_string(),
            "1.0.0".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        ConfigurationRepository::set(
            &pool,
            "db.host".to_string(),
            "localhost".to_string(),
            "string".to_string(),
        )
        .await
        .unwrap();

        let app_configs = ConfigurationRepository::list_by_prefix(&pool, "app.")
            .await
            .unwrap();

        assert_eq!(app_configs.len(), 2);
    }
}
