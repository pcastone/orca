//! Prompt repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::Prompt;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for prompt template database operations (user DB)
#[derive(Clone, Debug)]
pub struct PromptRepository {
    db: Arc<Database>,
}

impl PromptRepository {
    /// Create a new prompt repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save a prompt template to the database
    pub async fn save(&self, prompt: &Prompt) -> Result<()> {
        sqlx::query(
            "INSERT INTO prompts (id, name, description, template, category, variables, metadata,
                                  created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&prompt.id)
        .bind(&prompt.name)
        .bind(&prompt.description)
        .bind(&prompt.template)
        .bind(&prompt.category)
        .bind(&prompt.variables)
        .bind(&prompt.metadata)
        .bind(prompt.created_at)
        .bind(prompt.updated_at)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save prompt: {}", e)))?;

        Ok(())
    }

    /// Load a prompt by ID
    pub async fn find_by_id(&self, id: &str) -> Result<Prompt> {
        let row = sqlx::query(
            "SELECT id, name, description, template, category, variables, metadata,
                    created_at, updated_at
             FROM prompts WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load prompt: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Prompt not found: {}", id)))?;

        Ok(Prompt {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            template: row.get("template"),
            category: row.get("category"),
            variables: row.get("variables"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Find a prompt by name
    pub async fn find_by_name(&self, name: &str) -> Result<Prompt> {
        let row = sqlx::query(
            "SELECT id, name, description, template, category, variables, metadata,
                    created_at, updated_at
             FROM prompts WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load prompt: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Prompt not found: {}", name)))?;

        Ok(Prompt {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            template: row.get("template"),
            category: row.get("category"),
            variables: row.get("variables"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// List all prompts
    pub async fn list(&self) -> Result<Vec<Prompt>> {
        let rows = sqlx::query(
            "SELECT id, name, description, template, category, variables, metadata,
                    created_at, updated_at
             FROM prompts
             ORDER BY name ASC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list prompts: {}", e)))?;

        let prompts = rows
            .into_iter()
            .map(|row| Prompt {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                template: row.get("template"),
                category: row.get("category"),
                variables: row.get("variables"),
                metadata: row.get("metadata"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(prompts)
    }

    /// List prompts by category
    pub async fn list_by_category(&self, category: &str) -> Result<Vec<Prompt>> {
        let rows = sqlx::query(
            "SELECT id, name, description, template, category, variables, metadata,
                    created_at, updated_at
             FROM prompts
             WHERE category = ?
             ORDER BY name ASC"
        )
        .bind(category)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list prompts by category: {}", e)))?;

        let prompts = rows
            .into_iter()
            .map(|row| Prompt {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                template: row.get("template"),
                category: row.get("category"),
                variables: row.get("variables"),
                metadata: row.get("metadata"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(prompts)
    }

    /// Update a prompt
    pub async fn update(&self, prompt: &Prompt) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE prompts
             SET name = ?, description = ?, template = ?, category = ?, variables = ?,
                 metadata = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(&prompt.name)
        .bind(&prompt.description)
        .bind(&prompt.template)
        .bind(&prompt.category)
        .bind(&prompt.variables)
        .bind(&prompt.metadata)
        .bind(updated_at)
        .bind(&prompt.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update prompt: {}", e)))?;

        Ok(())
    }

    /// Delete a prompt
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM prompts WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete prompt: {}", e)))?;

        Ok(())
    }
}
