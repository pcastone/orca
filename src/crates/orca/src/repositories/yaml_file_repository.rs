//! YAML file repository for database operations
//!
//! Handles CRUD operations for yaml_files table in user database.

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::YamlFile;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for YAML file tracking database operations (user DB)
#[derive(Clone, Debug)]
pub struct YamlFileRepository {
    db: Arc<Database>,
}

impl YamlFileRepository {
    /// Create a new YAML file repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save a YAML file entry to the database
    pub async fn save(&self, yaml_file: &YamlFile) -> Result<()> {
        sqlx::query(
            "INSERT INTO yaml_files (id, file_path, file_type, content_hash, target_table,
                                     target_id, file_size, last_synced_at, sync_status,
                                     sync_error, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&yaml_file.id)
        .bind(&yaml_file.file_path)
        .bind(&yaml_file.file_type)
        .bind(&yaml_file.content_hash)
        .bind(&yaml_file.target_table)
        .bind(&yaml_file.target_id)
        .bind(yaml_file.file_size)
        .bind(yaml_file.last_synced_at)
        .bind(&yaml_file.sync_status)
        .bind(&yaml_file.sync_error)
        .bind(yaml_file.created_at)
        .bind(yaml_file.updated_at)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save YAML file entry: {}", e)))?;

        Ok(())
    }

    /// Find a YAML file entry by ID
    pub async fn find_by_id(&self, id: &str) -> Result<YamlFile> {
        let row = sqlx::query(
            "SELECT id, file_path, file_type, content_hash, target_table, target_id,
                    file_size, last_synced_at, sync_status, sync_error, created_at, updated_at
             FROM yaml_files WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load YAML file entry: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("YAML file entry not found: {}", id)))?;

        Ok(self.row_to_yaml_file(&row))
    }

    /// Find a YAML file entry by file path
    pub async fn find_by_file_path(&self, file_path: &str) -> Result<YamlFile> {
        let row = sqlx::query(
            "SELECT id, file_path, file_type, content_hash, target_table, target_id,
                    file_size, last_synced_at, sync_status, sync_error, created_at, updated_at
             FROM yaml_files WHERE file_path = ?"
        )
        .bind(file_path)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load YAML file entry: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("YAML file entry not found for: {}", file_path)))?;

        Ok(self.row_to_yaml_file(&row))
    }

    /// List all YAML file entries
    pub async fn list(&self) -> Result<Vec<YamlFile>> {
        let rows = sqlx::query(
            "SELECT id, file_path, file_type, content_hash, target_table, target_id,
                    file_size, last_synced_at, sync_status, sync_error, created_at, updated_at
             FROM yaml_files
             ORDER BY last_synced_at DESC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list YAML files: {}", e)))?;

        Ok(rows.iter().map(|row| self.row_to_yaml_file(row)).collect())
    }

    /// List YAML file entries by file type
    pub async fn list_by_type(&self, file_type: &str) -> Result<Vec<YamlFile>> {
        let rows = sqlx::query(
            "SELECT id, file_path, file_type, content_hash, target_table, target_id,
                    file_size, last_synced_at, sync_status, sync_error, created_at, updated_at
             FROM yaml_files
             WHERE file_type = ?
             ORDER BY last_synced_at DESC"
        )
        .bind(file_type)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list YAML files by type: {}", e)))?;

        Ok(rows.iter().map(|row| self.row_to_yaml_file(row)).collect())
    }

    /// List YAML file entries with pending sync status
    pub async fn list_pending(&self) -> Result<Vec<YamlFile>> {
        let rows = sqlx::query(
            "SELECT id, file_path, file_type, content_hash, target_table, target_id,
                    file_size, last_synced_at, sync_status, sync_error, created_at, updated_at
             FROM yaml_files
             WHERE sync_status = 'pending'
             ORDER BY updated_at ASC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list pending YAML files: {}", e)))?;

        Ok(rows.iter().map(|row| self.row_to_yaml_file(row)).collect())
    }

    /// List YAML file entries with error status
    pub async fn list_errors(&self) -> Result<Vec<YamlFile>> {
        let rows = sqlx::query(
            "SELECT id, file_path, file_type, content_hash, target_table, target_id,
                    file_size, last_synced_at, sync_status, sync_error, created_at, updated_at
             FROM yaml_files
             WHERE sync_status = 'error'
             ORDER BY updated_at DESC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list error YAML files: {}", e)))?;

        Ok(rows.iter().map(|row| self.row_to_yaml_file(row)).collect())
    }

    /// Update a YAML file entry
    pub async fn update(&self, yaml_file: &YamlFile) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE yaml_files
             SET file_path = ?, file_type = ?, content_hash = ?, target_table = ?,
                 target_id = ?, file_size = ?, last_synced_at = ?, sync_status = ?,
                 sync_error = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(&yaml_file.file_path)
        .bind(&yaml_file.file_type)
        .bind(&yaml_file.content_hash)
        .bind(&yaml_file.target_table)
        .bind(&yaml_file.target_id)
        .bind(yaml_file.file_size)
        .bind(yaml_file.last_synced_at)
        .bind(&yaml_file.sync_status)
        .bind(&yaml_file.sync_error)
        .bind(updated_at)
        .bind(&yaml_file.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update YAML file entry: {}", e)))?;

        Ok(())
    }

    /// Delete a YAML file entry by ID
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM yaml_files WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete YAML file entry: {}", e)))?;

        Ok(())
    }

    /// Delete a YAML file entry by file path
    pub async fn delete_by_path(&self, file_path: &str) -> Result<()> {
        sqlx::query("DELETE FROM yaml_files WHERE file_path = ?")
            .bind(file_path)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete YAML file by path: {}", e)))?;

        Ok(())
    }

    /// Check if a file path has a tracking entry
    pub async fn has_file(&self, file_path: &str) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM yaml_files WHERE file_path = ?")
            .bind(file_path)
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to check YAML file: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }

    /// Update content hash for a file (marks as pending)
    pub async fn update_hash(&self, file_path: &str, new_hash: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE yaml_files SET content_hash = ?, sync_status = 'pending', updated_at = ?
             WHERE file_path = ?"
        )
        .bind(new_hash)
        .bind(updated_at)
        .bind(file_path)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update YAML file hash: {}", e)))?;

        Ok(())
    }

    /// Mark a file as synced
    pub async fn mark_synced(&self, file_path: &str, target_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();

        sqlx::query(
            "UPDATE yaml_files SET sync_status = 'synced', sync_error = NULL,
                                   target_id = ?, last_synced_at = ?, updated_at = ?
             WHERE file_path = ?"
        )
        .bind(target_id)
        .bind(now)
        .bind(now)
        .bind(file_path)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to mark YAML file as synced: {}", e)))?;

        Ok(())
    }

    /// Mark a file as error
    pub async fn mark_error(&self, file_path: &str, error: &str) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE yaml_files SET sync_status = 'error', sync_error = ?, updated_at = ?
             WHERE file_path = ?"
        )
        .bind(error)
        .bind(updated_at)
        .bind(file_path)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to mark YAML file as error: {}", e)))?;

        Ok(())
    }

    /// Get sync statistics
    pub async fn get_stats(&self) -> Result<YamlFileStats> {
        let row = sqlx::query(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN sync_status = 'synced' THEN 1 ELSE 0 END) as synced,
                SUM(CASE WHEN sync_status = 'pending' THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN sync_status = 'error' THEN 1 ELSE 0 END) as errors
             FROM yaml_files"
        )
        .fetch_one(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get YAML file stats: {}", e)))?;

        Ok(YamlFileStats {
            total: row.get::<i64, _>("total") as usize,
            synced: row.get::<i64, _>("synced") as usize,
            pending: row.get::<i64, _>("pending") as usize,
            errors: row.get::<i64, _>("errors") as usize,
        })
    }

    /// Helper to convert a database row to YamlFile
    fn row_to_yaml_file(&self, row: &sqlx::sqlite::SqliteRow) -> YamlFile {
        YamlFile {
            id: row.get("id"),
            file_path: row.get("file_path"),
            file_type: row.get("file_type"),
            content_hash: row.get("content_hash"),
            target_table: row.get("target_table"),
            target_id: row.get("target_id"),
            file_size: row.get("file_size"),
            last_synced_at: row.get("last_synced_at"),
            sync_status: row.get("sync_status"),
            sync_error: row.get("sync_error"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// Statistics for YAML file tracking
#[derive(Debug, Clone, Default)]
pub struct YamlFileStats {
    pub total: usize,
    pub synced: usize,
    pub pending: usize,
    pub errors: usize,
}
