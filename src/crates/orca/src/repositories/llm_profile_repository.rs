//! LLM Profile repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::LlmProfile;
use chrono::Utc;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, FromRow};
use std::sync::Arc;
use uuid::Uuid;

/// Repository for LLM profile database operations
#[derive(Clone, Debug)]
pub struct LlmProfileRepository {
    db: Arc<Database>,
}

impl LlmProfileRepository {
    /// Create a new LLM profile repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Create a new LLM profile
    pub async fn create(&self, mut profile: LlmProfile) -> Result<LlmProfile> {
        let now = Utc::now().timestamp();
        profile.id = Uuid::new_v4().to_string();
        profile.created_at = now;
        profile.updated_at = now;

        sqlx::query(
            "INSERT INTO llm_profiles (id, name, planner_provider, planner_model,
                                     worker_provider, worker_model, active, description,
                                     created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&profile.id)
        .bind(&profile.name)
        .bind(&profile.planner_provider)
        .bind(&profile.planner_model)
        .bind(&profile.worker_provider)
        .bind(&profile.worker_model)
        .bind(profile.active)
        .bind(&profile.description)
        .bind(now)
        .bind(now)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to create LLM profile: {}", e)))?;

        Ok(profile)
    }

    /// Get profile by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<LlmProfile>> {
        let row = sqlx::query(
            "SELECT id, name, planner_provider, planner_model, worker_provider, worker_model,
                    active, description, created_at, updated_at
             FROM llm_profiles WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get LLM profile: {}", e)))?;

        Ok(row.map(|r| self.row_to_profile(r)))
    }

    /// Get profile by name
    pub async fn get_by_name(&self, name: &str) -> Result<Option<LlmProfile>> {
        let row = sqlx::query(
            "SELECT id, name, planner_provider, planner_model, worker_provider, worker_model,
                    active, description, created_at, updated_at
             FROM llm_profiles WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get LLM profile by name: {}", e)))?;

        Ok(row.map(|r| self.row_to_profile(r)))
    }

    /// List all profiles
    pub async fn list_all(&self) -> Result<Vec<LlmProfile>> {
        let rows = sqlx::query(
            "SELECT id, name, planner_provider, planner_model, worker_provider, worker_model,
                    active, description, created_at, updated_at
             FROM llm_profiles ORDER BY created_at DESC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list LLM profiles: {}", e)))?;

        Ok(rows.into_iter().map(|r| self.row_to_profile(r)).collect())
    }

    /// Get active profile
    pub async fn get_active(&self) -> Result<Option<LlmProfile>> {
        let row = sqlx::query(
            "SELECT id, name, planner_provider, planner_model, worker_provider, worker_model,
                    active, description, created_at, updated_at
             FROM llm_profiles WHERE active = 1 LIMIT 1"
        )
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get active LLM profile: {}", e)))?;

        Ok(row.map(|r| self.row_to_profile(r)))
    }

    /// Update profile
    pub async fn update(&self, profile: &LlmProfile) -> Result<()> {
        let now = Utc::now().timestamp();

        sqlx::query(
            "UPDATE llm_profiles SET name = ?, planner_provider = ?, planner_model = ?,
                                    worker_provider = ?, worker_model = ?, active = ?,
                                    description = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(&profile.name)
        .bind(&profile.planner_provider)
        .bind(&profile.planner_model)
        .bind(&profile.worker_provider)
        .bind(&profile.worker_model)
        .bind(profile.active)
        .bind(&profile.description)
        .bind(now)
        .bind(&profile.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update LLM profile: {}", e)))?;

        Ok(())
    }

    /// Delete profile
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM llm_profiles WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete LLM profile: {}", e)))?;

        Ok(())
    }

    /// Activate a profile and deactivate others
    pub async fn activate(&self, id: &str) -> Result<()> {
        let mut tx = self.db.pool().begin().await
            .map_err(|e| OrcaError::Database(format!("Failed to start transaction: {}", e)))?;

        sqlx::query("UPDATE llm_profiles SET active = 0")
            .execute(&mut *tx)
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to deactivate profiles: {}", e)))?;

        sqlx::query("UPDATE llm_profiles SET active = 1 WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to activate profile: {}", e)))?;

        tx.commit().await
            .map_err(|e| OrcaError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    // Helper function to convert database row to LlmProfile
    fn row_to_profile(&self, row: sqlx::sqlite::SqliteRow) -> LlmProfile {
        LlmProfile {
            id: row.get("id"),
            name: row.get("name"),
            planner_provider: row.get("planner_provider"),
            planner_model: row.get("planner_model"),
            worker_provider: row.get("worker_provider"),
            worker_model: row.get("worker_model"),
            description: row.get("description"),
            active: row.get::<bool, _>("active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
