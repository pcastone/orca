//! Bug repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::{Bug, BugStats};
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for bug tracking database operations (project DB)
#[derive(Clone, Debug)]
pub struct BugRepository {
    db: Arc<Database>,
}

impl BugRepository {
    /// Create a new bug repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save a bug to the database
    pub async fn save(&self, bug: &Bug) -> Result<()> {
        sqlx::query(
            "INSERT INTO bugs (id, title, description, status, priority, severity, assignee,
                               reporter, labels, related_files, created_at, updated_at,
                               resolved_at, metadata)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&bug.id)
        .bind(&bug.title)
        .bind(&bug.description)
        .bind(&bug.status)
        .bind(bug.priority)
        .bind(&bug.severity)
        .bind(&bug.assignee)
        .bind(&bug.reporter)
        .bind(&bug.labels)
        .bind(&bug.related_files)
        .bind(bug.created_at)
        .bind(bug.updated_at)
        .bind(bug.resolved_at)
        .bind(&bug.metadata)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save bug: {}", e)))?;

        Ok(())
    }

    /// Load a bug by ID
    pub async fn find_by_id(&self, id: &str) -> Result<Bug> {
        let row = sqlx::query(
            "SELECT id, title, description, status, priority, severity, assignee, reporter,
                    labels, related_files, created_at, updated_at, resolved_at, metadata
             FROM bugs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load bug: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Bug not found: {}", id)))?;

        Ok(Bug {
            id: row.get("id"),
            title: row.get("title"),
            description: row.get("description"),
            status: row.get("status"),
            priority: row.get("priority"),
            severity: row.get("severity"),
            assignee: row.get("assignee"),
            reporter: row.get("reporter"),
            labels: row.get("labels"),
            related_files: row.get("related_files"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            resolved_at: row.get("resolved_at"),
            metadata: row.get("metadata"),
        })
    }

    /// List all bugs
    pub async fn list(&self) -> Result<Vec<Bug>> {
        let rows = sqlx::query(
            "SELECT id, title, description, status, priority, severity, assignee, reporter,
                    labels, related_files, created_at, updated_at, resolved_at, metadata
             FROM bugs
             ORDER BY priority ASC, created_at DESC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list bugs: {}", e)))?;

        let bugs = rows
            .into_iter()
            .map(|row| Bug {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                status: row.get("status"),
                priority: row.get("priority"),
                severity: row.get("severity"),
                assignee: row.get("assignee"),
                reporter: row.get("reporter"),
                labels: row.get("labels"),
                related_files: row.get("related_files"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                resolved_at: row.get("resolved_at"),
                metadata: row.get("metadata"),
            })
            .collect();

        Ok(bugs)
    }

    /// List bugs by status
    pub async fn list_by_status(&self, status: &str) -> Result<Vec<Bug>> {
        let rows = sqlx::query(
            "SELECT id, title, description, status, priority, severity, assignee, reporter,
                    labels, related_files, created_at, updated_at, resolved_at, metadata
             FROM bugs
             WHERE status = ?
             ORDER BY priority ASC, created_at DESC"
        )
        .bind(status)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list bugs by status: {}", e)))?;

        let bugs = rows
            .into_iter()
            .map(|row| Bug {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                status: row.get("status"),
                priority: row.get("priority"),
                severity: row.get("severity"),
                assignee: row.get("assignee"),
                reporter: row.get("reporter"),
                labels: row.get("labels"),
                related_files: row.get("related_files"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                resolved_at: row.get("resolved_at"),
                metadata: row.get("metadata"),
            })
            .collect();

        Ok(bugs)
    }

    /// List bugs assigned to a specific user
    pub async fn list_by_assignee(&self, assignee: &str) -> Result<Vec<Bug>> {
        let rows = sqlx::query(
            "SELECT id, title, description, status, priority, severity, assignee, reporter,
                    labels, related_files, created_at, updated_at, resolved_at, metadata
             FROM bugs
             WHERE assignee = ?
             ORDER BY priority ASC, created_at DESC"
        )
        .bind(assignee)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list bugs by assignee: {}", e)))?;

        let bugs = rows
            .into_iter()
            .map(|row| Bug {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                status: row.get("status"),
                priority: row.get("priority"),
                severity: row.get("severity"),
                assignee: row.get("assignee"),
                reporter: row.get("reporter"),
                labels: row.get("labels"),
                related_files: row.get("related_files"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                resolved_at: row.get("resolved_at"),
                metadata: row.get("metadata"),
            })
            .collect();

        Ok(bugs)
    }

    /// Update a bug
    pub async fn update(&self, bug: &Bug) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE bugs
             SET title = ?, description = ?, status = ?, priority = ?, severity = ?,
                 assignee = ?, reporter = ?, labels = ?, related_files = ?, updated_at = ?,
                 resolved_at = ?, metadata = ?
             WHERE id = ?"
        )
        .bind(&bug.title)
        .bind(&bug.description)
        .bind(&bug.status)
        .bind(bug.priority)
        .bind(&bug.severity)
        .bind(&bug.assignee)
        .bind(&bug.reporter)
        .bind(&bug.labels)
        .bind(&bug.related_files)
        .bind(updated_at)
        .bind(bug.resolved_at)
        .bind(&bug.metadata)
        .bind(&bug.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update bug: {}", e)))?;

        Ok(())
    }

    /// Delete a bug
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM bugs WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete bug: {}", e)))?;

        Ok(())
    }

    /// Count bugs by status
    pub async fn count_by_status(&self, status: &str) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM bugs WHERE status = ?")
            .bind(status)
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to count bugs: {}", e)))?;

        Ok(row.get("count"))
    }

    /// Get bug statistics
    pub async fn get_stats(&self) -> Result<BugStats> {
        // Count by status
        let status_row = sqlx::query(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'open' THEN 1 ELSE 0 END) as open,
                SUM(CASE WHEN status = 'in_progress' THEN 1 ELSE 0 END) as in_progress,
                SUM(CASE WHEN status = 'fixed' THEN 1 ELSE 0 END) as fixed,
                SUM(CASE WHEN status = 'wontfix' THEN 1 ELSE 0 END) as wontfix,
                SUM(CASE WHEN status = 'duplicate' THEN 1 ELSE 0 END) as duplicate
             FROM bugs"
        )
        .fetch_one(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get bug stats: {}", e)))?;

        // Count by priority
        let priority_row = sqlx::query(
            "SELECT
                SUM(CASE WHEN priority = 1 THEN 1 ELSE 0 END) as critical,
                SUM(CASE WHEN priority = 2 THEN 1 ELSE 0 END) as high,
                SUM(CASE WHEN priority = 3 THEN 1 ELSE 0 END) as medium,
                SUM(CASE WHEN priority = 4 THEN 1 ELSE 0 END) as low,
                SUM(CASE WHEN priority = 5 THEN 1 ELSE 0 END) as trivial
             FROM bugs"
        )
        .fetch_one(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get bug priority stats: {}", e)))?;

        Ok(BugStats {
            total: status_row.get("total"),
            open: status_row.get("open"),
            in_progress: status_row.get("in_progress"),
            fixed: status_row.get("fixed"),
            wontfix: status_row.get("wontfix"),
            duplicate: status_row.get("duplicate"),
            critical: priority_row.get("critical"),
            high: priority_row.get("high"),
            medium: priority_row.get("medium"),
            low: priority_row.get("low"),
            trivial: priority_row.get("trivial"),
        })
    }
}
