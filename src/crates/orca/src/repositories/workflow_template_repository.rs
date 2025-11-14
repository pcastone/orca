//! Workflow template repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::WorkflowTemplate;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for workflow template database operations (user DB)
#[derive(Clone, Debug)]
pub struct WorkflowTemplateRepository {
    db: Arc<Database>,
}

impl WorkflowTemplateRepository {
    /// Create a new workflow template repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save a workflow template to the database
    pub async fn save(&self, template: &WorkflowTemplate) -> Result<()> {
        sqlx::query(
            "INSERT INTO workflow_templates (id, name, description, pattern, definition, tags,
                                             is_public, usage_count, metadata, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&template.id)
        .bind(&template.name)
        .bind(&template.description)
        .bind(&template.pattern)
        .bind(&template.definition)
        .bind(&template.tags)
        .bind(template.is_public)
        .bind(template.usage_count)
        .bind(&template.metadata)
        .bind(template.created_at)
        .bind(template.updated_at)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save workflow template: {}", e)))?;

        Ok(())
    }

    /// Load a workflow template by ID
    pub async fn find_by_id(&self, id: &str) -> Result<WorkflowTemplate> {
        let row = sqlx::query(
            "SELECT id, name, description, pattern, definition, tags, is_public, usage_count,
                    metadata, created_at, updated_at
             FROM workflow_templates WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load workflow template: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Workflow template not found: {}", id)))?;

        Ok(WorkflowTemplate {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            pattern: row.get("pattern"),
            definition: row.get("definition"),
            tags: row.get("tags"),
            is_public: row.get("is_public"),
            usage_count: row.get("usage_count"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Find a workflow template by name
    pub async fn find_by_name(&self, name: &str) -> Result<WorkflowTemplate> {
        let row = sqlx::query(
            "SELECT id, name, description, pattern, definition, tags, is_public, usage_count,
                    metadata, created_at, updated_at
             FROM workflow_templates WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load workflow template: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Workflow template not found: {}", name)))?;

        Ok(WorkflowTemplate {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            pattern: row.get("pattern"),
            definition: row.get("definition"),
            tags: row.get("tags"),
            is_public: row.get("is_public"),
            usage_count: row.get("usage_count"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// List all workflow templates
    pub async fn list(&self) -> Result<Vec<WorkflowTemplate>> {
        let rows = sqlx::query(
            "SELECT id, name, description, pattern, definition, tags, is_public, usage_count,
                    metadata, created_at, updated_at
             FROM workflow_templates
             ORDER BY usage_count DESC, name ASC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list workflow templates: {}", e)))?;

        let templates = rows
            .into_iter()
            .map(|row| WorkflowTemplate {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                pattern: row.get("pattern"),
                definition: row.get("definition"),
                tags: row.get("tags"),
                is_public: row.get("is_public"),
                usage_count: row.get("usage_count"),
                metadata: row.get("metadata"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(templates)
    }

    /// List templates by pattern
    pub async fn list_by_pattern(&self, pattern: &str) -> Result<Vec<WorkflowTemplate>> {
        let rows = sqlx::query(
            "SELECT id, name, description, pattern, definition, tags, is_public, usage_count,
                    metadata, created_at, updated_at
             FROM workflow_templates
             WHERE pattern = ?
             ORDER BY usage_count DESC, name ASC"
        )
        .bind(pattern)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list workflow templates by pattern: {}", e)))?;

        let templates = rows
            .into_iter()
            .map(|row| WorkflowTemplate {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                pattern: row.get("pattern"),
                definition: row.get("definition"),
                tags: row.get("tags"),
                is_public: row.get("is_public"),
                usage_count: row.get("usage_count"),
                metadata: row.get("metadata"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(templates)
    }

    /// Update a workflow template
    pub async fn update(&self, template: &WorkflowTemplate) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE workflow_templates
             SET name = ?, description = ?, pattern = ?, definition = ?, tags = ?,
                 is_public = ?, usage_count = ?, metadata = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(&template.name)
        .bind(&template.description)
        .bind(&template.pattern)
        .bind(&template.definition)
        .bind(&template.tags)
        .bind(template.is_public)
        .bind(template.usage_count)
        .bind(&template.metadata)
        .bind(updated_at)
        .bind(&template.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update workflow template: {}", e)))?;

        Ok(())
    }

    /// Delete a workflow template
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM workflow_templates WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete workflow template: {}", e)))?;

        Ok(())
    }

    /// Increment usage count for a template
    pub async fn increment_usage(&self, id: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE workflow_templates
             SET usage_count = usage_count + 1, updated_at = ?
             WHERE id = ?"
        )
        .bind(updated_at)
        .bind(id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to increment template usage: {}", e)))?;

        Ok(())
    }
}
