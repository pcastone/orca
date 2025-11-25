//! Pattern configuration repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::PatternConfig;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for pattern configuration database operations
#[derive(Clone, Debug)]
pub struct PatternConfigRepository {
    db: Arc<Database>,
}

impl PatternConfigRepository {
    /// Create a new pattern config repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save a pattern configuration to the database
    pub async fn save(&self, config: &PatternConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO pattern_configs (id, name, pattern_type, config, tools, system_prompt, max_iterations, is_default, usage_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                pattern_type = excluded.pattern_type,
                config = excluded.config,
                tools = excluded.tools,
                system_prompt = excluded.system_prompt,
                max_iterations = excluded.max_iterations,
                is_default = excluded.is_default,
                usage_count = excluded.usage_count,
                updated_at = excluded.updated_at"
        )
        .bind(&config.id)
        .bind(&config.name)
        .bind(&config.pattern_type)
        .bind(&config.config)
        .bind(&config.tools)
        .bind(&config.system_prompt)
        .bind(config.max_iterations)
        .bind(config.is_default)
        .bind(config.usage_count)
        .bind(config.created_at)
        .bind(config.updated_at)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save pattern config: {}", e)))?;

        Ok(())
    }

    /// Find a pattern configuration by ID
    pub async fn find_by_id(&self, id: &str) -> Result<PatternConfig> {
        let row = sqlx::query(
            "SELECT id, name, pattern_type, config, tools, system_prompt, max_iterations, is_default, usage_count, created_at, updated_at
             FROM pattern_configs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load pattern config: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Pattern config not found: {}", id)))?;

        Ok(PatternConfig {
            id: row.get("id"),
            name: row.get("name"),
            pattern_type: row.get("pattern_type"),
            config: row.get("config"),
            tools: row.get("tools"),
            system_prompt: row.get("system_prompt"),
            max_iterations: row.get("max_iterations"),
            is_default: row.get("is_default"),
            usage_count: row.get("usage_count"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Find a pattern configuration by name
    pub async fn find_by_name(&self, name: &str) -> Result<PatternConfig> {
        let row = sqlx::query(
            "SELECT id, name, pattern_type, config, tools, system_prompt, max_iterations, is_default, usage_count, created_at, updated_at
             FROM pattern_configs WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load pattern config: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Pattern config not found: {}", name)))?;

        Ok(PatternConfig {
            id: row.get("id"),
            name: row.get("name"),
            pattern_type: row.get("pattern_type"),
            config: row.get("config"),
            tools: row.get("tools"),
            system_prompt: row.get("system_prompt"),
            max_iterations: row.get("max_iterations"),
            is_default: row.get("is_default"),
            usage_count: row.get("usage_count"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Get the default pattern configuration
    pub async fn find_default(&self) -> Result<PatternConfig> {
        let row = sqlx::query(
            "SELECT id, name, pattern_type, config, tools, system_prompt, max_iterations, is_default, usage_count, created_at, updated_at
             FROM pattern_configs WHERE is_default = 1 LIMIT 1"
        )
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load default pattern config: {}", e)))?
        .ok_or_else(|| OrcaError::Database("No default pattern config found".to_string()))?;

        Ok(PatternConfig {
            id: row.get("id"),
            name: row.get("name"),
            pattern_type: row.get("pattern_type"),
            config: row.get("config"),
            tools: row.get("tools"),
            system_prompt: row.get("system_prompt"),
            max_iterations: row.get("max_iterations"),
            is_default: row.get("is_default"),
            usage_count: row.get("usage_count"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// List all pattern configurations
    pub async fn list(&self) -> Result<Vec<PatternConfig>> {
        let rows = sqlx::query(
            "SELECT id, name, pattern_type, config, tools, system_prompt, max_iterations, is_default, usage_count, created_at, updated_at
             FROM pattern_configs
             ORDER BY is_default DESC, usage_count DESC, name ASC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list pattern configs: {}", e)))?;

        let configs = rows
            .into_iter()
            .map(|row| PatternConfig {
                id: row.get("id"),
                name: row.get("name"),
                pattern_type: row.get("pattern_type"),
                config: row.get("config"),
                tools: row.get("tools"),
                system_prompt: row.get("system_prompt"),
                max_iterations: row.get("max_iterations"),
                is_default: row.get("is_default"),
                usage_count: row.get("usage_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(configs)
    }

    /// List pattern configurations by pattern type
    pub async fn list_by_type(&self, pattern_type: &str) -> Result<Vec<PatternConfig>> {
        let rows = sqlx::query(
            "SELECT id, name, pattern_type, config, tools, system_prompt, max_iterations, is_default, usage_count, created_at, updated_at
             FROM pattern_configs
             WHERE pattern_type = ?
             ORDER BY is_default DESC, usage_count DESC, name ASC"
        )
        .bind(pattern_type)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list pattern configs by type: {}", e)))?;

        let configs = rows
            .into_iter()
            .map(|row| PatternConfig {
                id: row.get("id"),
                name: row.get("name"),
                pattern_type: row.get("pattern_type"),
                config: row.get("config"),
                tools: row.get("tools"),
                system_prompt: row.get("system_prompt"),
                max_iterations: row.get("max_iterations"),
                is_default: row.get("is_default"),
                usage_count: row.get("usage_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(configs)
    }

    /// Update a pattern configuration
    pub async fn update(&self, config: &PatternConfig) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE pattern_configs
             SET name = ?, pattern_type = ?, config = ?, tools = ?, system_prompt = ?,
                 max_iterations = ?, is_default = ?, usage_count = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(&config.name)
        .bind(&config.pattern_type)
        .bind(&config.config)
        .bind(&config.tools)
        .bind(&config.system_prompt)
        .bind(config.max_iterations)
        .bind(config.is_default)
        .bind(config.usage_count)
        .bind(updated_at)
        .bind(&config.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update pattern config: {}", e)))?;

        Ok(())
    }

    /// Delete a pattern configuration
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM pattern_configs WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete pattern config: {}", e)))?;

        Ok(())
    }

    /// Increment the usage count for a pattern configuration
    pub async fn increment_usage(&self, id: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE pattern_configs SET usage_count = usage_count + 1, updated_at = ? WHERE id = ?"
        )
        .bind(updated_at)
        .bind(id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to increment usage count: {}", e)))?;

        Ok(())
    }

    /// Set a configuration as the default (unsets others)
    pub async fn set_default(&self, id: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        // Unset all defaults
        sqlx::query("UPDATE pattern_configs SET is_default = 0, updated_at = ?")
            .bind(updated_at)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to clear defaults: {}", e)))?;

        // Set the new default
        sqlx::query("UPDATE pattern_configs SET is_default = 1, updated_at = ? WHERE id = ?")
            .bind(updated_at)
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to set default: {}", e)))?;

        Ok(())
    }

    /// Check if a pattern configuration exists
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM pattern_configs WHERE id = ?")
            .bind(id)
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to check existence: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }

    /// Count pattern configurations by type
    pub async fn count_by_type(&self, pattern_type: &str) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM pattern_configs WHERE pattern_type = ?")
            .bind(pattern_type)
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to count by type: {}", e)))?;

        Ok(row.get("count"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::pattern_config::PatternType;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> Arc<Database> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        // Run migrations
        db.run_migrations().await.unwrap();

        db
    }

    #[tokio::test]
    async fn test_save_and_find() {
        let db = setup_test_db().await;
        let repo = PatternConfigRepository::new(db);

        let config = PatternConfig::new("Test Config", PatternType::React)
            .with_tools(vec!["read_file", "write_file"])
            .with_system_prompt("Test prompt");

        repo.save(&config).await.unwrap();

        let loaded = repo.find_by_id(&config.id).await.unwrap();
        assert_eq!(loaded.id, config.id);
        assert_eq!(loaded.name, "Test Config");
        assert_eq!(loaded.pattern_type, "react");
    }

    #[tokio::test]
    async fn test_find_by_name() {
        let db = setup_test_db().await;
        let repo = PatternConfigRepository::new(db);

        let config = PatternConfig::new("Unique Name", PatternType::Reflection);
        repo.save(&config).await.unwrap();

        let loaded = repo.find_by_name("Unique Name").await.unwrap();
        assert_eq!(loaded.id, config.id);
    }

    #[tokio::test]
    async fn test_list_configs() {
        let db = setup_test_db().await;
        let repo = PatternConfigRepository::new(db);

        // The migration inserts 4 default configs
        let configs = repo.list().await.unwrap();
        assert!(configs.len() >= 4);
    }

    #[tokio::test]
    async fn test_list_by_type() {
        let db = setup_test_db().await;
        let repo = PatternConfigRepository::new(db);

        let react_configs = repo.list_by_type("react").await.unwrap();
        assert!(react_configs.len() >= 2); // default_react and default_react_simple

        for config in react_configs {
            assert_eq!(config.pattern_type, "react");
        }
    }

    #[tokio::test]
    async fn test_find_default() {
        let db = setup_test_db().await;
        let repo = PatternConfigRepository::new(db);

        let default = repo.find_default().await.unwrap();
        assert!(default.is_default);
        assert_eq!(default.id, "default_react");
    }

    #[tokio::test]
    async fn test_increment_usage() {
        let db = setup_test_db().await;
        let repo = PatternConfigRepository::new(db);

        let config = PatternConfig::new("Usage Test", PatternType::React);
        repo.save(&config).await.unwrap();

        assert_eq!(repo.find_by_id(&config.id).await.unwrap().usage_count, 0);

        repo.increment_usage(&config.id).await.unwrap();
        assert_eq!(repo.find_by_id(&config.id).await.unwrap().usage_count, 1);

        repo.increment_usage(&config.id).await.unwrap();
        assert_eq!(repo.find_by_id(&config.id).await.unwrap().usage_count, 2);
    }

    #[tokio::test]
    async fn test_set_default() {
        let db = setup_test_db().await;
        let repo = PatternConfigRepository::new(db);

        let config = PatternConfig::new("New Default", PatternType::Reflection);
        repo.save(&config).await.unwrap();

        repo.set_default(&config.id).await.unwrap();

        let new_default = repo.find_default().await.unwrap();
        assert_eq!(new_default.id, config.id);

        // Old default should no longer be default
        let old_default = repo.find_by_id("default_react").await.unwrap();
        assert!(!old_default.is_default);
    }

    #[tokio::test]
    async fn test_delete() {
        let db = setup_test_db().await;
        let repo = PatternConfigRepository::new(db);

        let config = PatternConfig::new("To Delete", PatternType::React);
        repo.save(&config).await.unwrap();

        assert!(repo.exists(&config.id).await.unwrap());

        repo.delete(&config.id).await.unwrap();

        assert!(!repo.exists(&config.id).await.unwrap());
    }

    #[tokio::test]
    async fn test_update() {
        let db = setup_test_db().await;
        let repo = PatternConfigRepository::new(db);

        let mut config = PatternConfig::new("Original", PatternType::React);
        repo.save(&config).await.unwrap();

        config.name = "Updated".to_string();
        config.max_iterations = 20;
        repo.update(&config).await.unwrap();

        let loaded = repo.find_by_id(&config.id).await.unwrap();
        assert_eq!(loaded.name, "Updated");
        assert_eq!(loaded.max_iterations, 20);
    }
}
