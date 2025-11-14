//! Project rule repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::ProjectRule;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for project rule database operations (project DB)
#[derive(Clone, Debug)]
pub struct ProjectRuleRepository {
    db: Arc<Database>,
}

impl ProjectRuleRepository {
    /// Create a new project rule repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save a project rule to the database
    pub async fn save(&self, rule: &ProjectRule) -> Result<()> {
        sqlx::query(
            "INSERT INTO project_rules (id, name, rule_type, description, config, severity,
                                        enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(&rule.rule_type)
        .bind(&rule.description)
        .bind(&rule.config)
        .bind(&rule.severity)
        .bind(rule.enabled)
        .bind(rule.created_at)
        .bind(rule.updated_at)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save project rule: {}", e)))?;

        Ok(())
    }

    /// Load a project rule by ID
    pub async fn find_by_id(&self, id: &str) -> Result<ProjectRule> {
        let row = sqlx::query(
            "SELECT id, name, rule_type, description, config, severity, enabled,
                    created_at, updated_at
             FROM project_rules WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load project rule: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Project rule not found: {}", id)))?;

        Ok(ProjectRule {
            id: row.get("id"),
            name: row.get("name"),
            rule_type: row.get("rule_type"),
            description: row.get("description"),
            config: row.get("config"),
            severity: row.get("severity"),
            enabled: row.get("enabled"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// List all project rules
    pub async fn list(&self) -> Result<Vec<ProjectRule>> {
        let rows = sqlx::query(
            "SELECT id, name, rule_type, description, config, severity, enabled,
                    created_at, updated_at
             FROM project_rules
             ORDER BY rule_type ASC, name ASC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list project rules: {}", e)))?;

        let rules = rows
            .into_iter()
            .map(|row| ProjectRule {
                id: row.get("id"),
                name: row.get("name"),
                rule_type: row.get("rule_type"),
                description: row.get("description"),
                config: row.get("config"),
                severity: row.get("severity"),
                enabled: row.get("enabled"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(rules)
    }

    /// List enabled project rules
    pub async fn list_enabled(&self) -> Result<Vec<ProjectRule>> {
        let rows = sqlx::query(
            "SELECT id, name, rule_type, description, config, severity, enabled,
                    created_at, updated_at
             FROM project_rules
             WHERE enabled = 1
             ORDER BY rule_type ASC, name ASC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list enabled project rules: {}", e)))?;

        let rules = rows
            .into_iter()
            .map(|row| ProjectRule {
                id: row.get("id"),
                name: row.get("name"),
                rule_type: row.get("rule_type"),
                description: row.get("description"),
                config: row.get("config"),
                severity: row.get("severity"),
                enabled: row.get("enabled"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(rules)
    }

    /// List rules by type
    pub async fn list_by_type(&self, rule_type: &str) -> Result<Vec<ProjectRule>> {
        let rows = sqlx::query(
            "SELECT id, name, rule_type, description, config, severity, enabled,
                    created_at, updated_at
             FROM project_rules
             WHERE rule_type = ?
             ORDER BY name ASC"
        )
        .bind(rule_type)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list project rules by type: {}", e)))?;

        let rules = rows
            .into_iter()
            .map(|row| ProjectRule {
                id: row.get("id"),
                name: row.get("name"),
                rule_type: row.get("rule_type"),
                description: row.get("description"),
                config: row.get("config"),
                severity: row.get("severity"),
                enabled: row.get("enabled"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(rules)
    }

    /// Update a project rule
    pub async fn update(&self, rule: &ProjectRule) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE project_rules
             SET name = ?, rule_type = ?, description = ?, config = ?, severity = ?,
                 enabled = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(&rule.name)
        .bind(&rule.rule_type)
        .bind(&rule.description)
        .bind(&rule.config)
        .bind(&rule.severity)
        .bind(rule.enabled)
        .bind(updated_at)
        .bind(&rule.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update project rule: {}", e)))?;

        Ok(())
    }

    /// Delete a project rule
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM project_rules WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete project rule: {}", e)))?;

        Ok(())
    }

    /// Enable a project rule
    pub async fn enable(&self, id: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query("UPDATE project_rules SET enabled = 1, updated_at = ? WHERE id = ?")
            .bind(updated_at)
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to enable project rule: {}", e)))?;

        Ok(())
    }

    /// Disable a project rule
    pub async fn disable(&self, id: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query("UPDATE project_rules SET enabled = 0, updated_at = ? WHERE id = ?")
            .bind(updated_at)
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to disable project rule: {}", e)))?;

        Ok(())
    }
}
