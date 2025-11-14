//! LLM Provider repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::LlmProviderConfig;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for LLM provider database operations (user DB)
#[derive(Clone, Debug)]
pub struct LlmProviderRepository {
    db: Arc<Database>,
}

impl LlmProviderRepository {
    /// Create a new LLM provider repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save an LLM provider to the database
    pub async fn save(&self, provider: &LlmProviderConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO llm_providers (id, name, provider_type, model, api_key, api_base,
                                        temperature, max_tokens, settings, is_default,
                                        created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.provider_type)
        .bind(&provider.model)
        .bind(&provider.api_key)
        .bind(&provider.api_base)
        .bind(provider.temperature)
        .bind(provider.max_tokens)
        .bind(&provider.settings)
        .bind(provider.is_default)
        .bind(provider.created_at)
        .bind(provider.updated_at)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save LLM provider: {}", e)))?;

        Ok(())
    }

    /// Load an LLM provider by ID
    pub async fn find_by_id(&self, id: &str) -> Result<LlmProviderConfig> {
        let row = sqlx::query(
            "SELECT id, name, provider_type, model, api_key, api_base, temperature, max_tokens,
                    settings, is_default, created_at, updated_at
             FROM llm_providers WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load LLM provider: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("LLM provider not found: {}", id)))?;

        Ok(LlmProviderConfig {
            id: row.get("id"),
            name: row.get("name"),
            provider_type: row.get("provider_type"),
            model: row.get("model"),
            api_key: row.get("api_key"),
            api_base: row.get("api_base"),
            temperature: row.get("temperature"),
            max_tokens: row.get("max_tokens"),
            settings: row.get("settings"),
            is_default: row.get("is_default"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// List all LLM providers
    pub async fn list(&self) -> Result<Vec<LlmProviderConfig>> {
        let rows = sqlx::query(
            "SELECT id, name, provider_type, model, api_key, api_base, temperature, max_tokens,
                    settings, is_default, created_at, updated_at
             FROM llm_providers
             ORDER BY is_default DESC, name ASC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list LLM providers: {}", e)))?;

        let providers = rows
            .into_iter()
            .map(|row| LlmProviderConfig {
                id: row.get("id"),
                name: row.get("name"),
                provider_type: row.get("provider_type"),
                model: row.get("model"),
                api_key: row.get("api_key"),
                api_base: row.get("api_base"),
                temperature: row.get("temperature"),
                max_tokens: row.get("max_tokens"),
                settings: row.get("settings"),
                is_default: row.get("is_default"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(providers)
    }

    /// Get the default LLM provider
    pub async fn get_default(&self) -> Result<LlmProviderConfig> {
        let row = sqlx::query(
            "SELECT id, name, provider_type, model, api_key, api_base, temperature, max_tokens,
                    settings, is_default, created_at, updated_at
             FROM llm_providers
             WHERE is_default = 1
             LIMIT 1"
        )
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get default LLM provider: {}", e)))?
        .ok_or_else(|| OrcaError::Database("No default LLM provider configured".to_string()))?;

        Ok(LlmProviderConfig {
            id: row.get("id"),
            name: row.get("name"),
            provider_type: row.get("provider_type"),
            model: row.get("model"),
            api_key: row.get("api_key"),
            api_base: row.get("api_base"),
            temperature: row.get("temperature"),
            max_tokens: row.get("max_tokens"),
            settings: row.get("settings"),
            is_default: row.get("is_default"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Update an LLM provider
    pub async fn update(&self, provider: &LlmProviderConfig) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE llm_providers
             SET name = ?, provider_type = ?, model = ?, api_key = ?, api_base = ?,
                 temperature = ?, max_tokens = ?, settings = ?, is_default = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(&provider.name)
        .bind(&provider.provider_type)
        .bind(&provider.model)
        .bind(&provider.api_key)
        .bind(&provider.api_base)
        .bind(provider.temperature)
        .bind(provider.max_tokens)
        .bind(&provider.settings)
        .bind(provider.is_default)
        .bind(updated_at)
        .bind(&provider.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update LLM provider: {}", e)))?;

        Ok(())
    }

    /// Delete an LLM provider
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM llm_providers WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete LLM provider: {}", e)))?;

        Ok(())
    }

    /// Set a provider as the default (clears other defaults)
    pub async fn set_default(&self, id: &str) -> Result<()> {
        // First, clear all defaults
        sqlx::query("UPDATE llm_providers SET is_default = 0")
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to clear default providers: {}", e)))?;

        // Then set the specified provider as default
        sqlx::query("UPDATE llm_providers SET is_default = 1, updated_at = ? WHERE id = ?")
            .bind(Utc::now().timestamp())
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to set default provider: {}", e)))?;

        Ok(())
    }
}
