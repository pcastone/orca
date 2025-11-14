//! AST cache repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::AstCache;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for AST cache database operations (project DB)
#[derive(Clone, Debug)]
pub struct AstCacheRepository {
    db: Arc<Database>,
}

impl AstCacheRepository {
    /// Create a new AST cache repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save an AST cache entry to the database
    pub async fn save(&self, ast: &AstCache) -> Result<()> {
        sqlx::query(
            "INSERT INTO ast_cache (id, file_path, language, content_hash, ast_data, symbols,
                                    imports, file_size, parse_duration_ms, created_at,
                                    updated_at, accessed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&ast.id)
        .bind(&ast.file_path)
        .bind(&ast.language)
        .bind(&ast.content_hash)
        .bind(&ast.ast_data)
        .bind(&ast.symbols)
        .bind(&ast.imports)
        .bind(ast.file_size)
        .bind(ast.parse_duration_ms)
        .bind(ast.created_at)
        .bind(ast.updated_at)
        .bind(ast.accessed_at)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save AST cache: {}", e)))?;

        Ok(())
    }

    /// Load an AST cache entry by ID
    pub async fn find_by_id(&self, id: &str) -> Result<AstCache> {
        let row = sqlx::query(
            "SELECT id, file_path, language, content_hash, ast_data, symbols, imports,
                    file_size, parse_duration_ms, created_at, updated_at, accessed_at
             FROM ast_cache WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load AST cache: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("AST cache entry not found: {}", id)))?;

        Ok(AstCache {
            id: row.get("id"),
            file_path: row.get("file_path"),
            language: row.get("language"),
            content_hash: row.get("content_hash"),
            ast_data: row.get("ast_data"),
            symbols: row.get("symbols"),
            imports: row.get("imports"),
            file_size: row.get("file_size"),
            parse_duration_ms: row.get("parse_duration_ms"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            accessed_at: row.get("accessed_at"),
        })
    }

    /// Find an AST cache entry by file path
    pub async fn find_by_file_path(&self, file_path: &str) -> Result<AstCache> {
        let row = sqlx::query(
            "SELECT id, file_path, language, content_hash, ast_data, symbols, imports,
                    file_size, parse_duration_ms, created_at, updated_at, accessed_at
             FROM ast_cache WHERE file_path = ?"
        )
        .bind(file_path)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load AST cache: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("AST cache entry not found for: {}", file_path)))?;

        // Update access timestamp
        self.touch_by_path(file_path).await?;

        Ok(AstCache {
            id: row.get("id"),
            file_path: row.get("file_path"),
            language: row.get("language"),
            content_hash: row.get("content_hash"),
            ast_data: row.get("ast_data"),
            symbols: row.get("symbols"),
            imports: row.get("imports"),
            file_size: row.get("file_size"),
            parse_duration_ms: row.get("parse_duration_ms"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            accessed_at: row.get("accessed_at"),
        })
    }

    /// List all AST cache entries
    pub async fn list(&self) -> Result<Vec<AstCache>> {
        let rows = sqlx::query(
            "SELECT id, file_path, language, content_hash, ast_data, symbols, imports,
                    file_size, parse_duration_ms, created_at, updated_at, accessed_at
             FROM ast_cache
             ORDER BY accessed_at DESC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list AST cache: {}", e)))?;

        let entries = rows
            .into_iter()
            .map(|row| AstCache {
                id: row.get("id"),
                file_path: row.get("file_path"),
                language: row.get("language"),
                content_hash: row.get("content_hash"),
                ast_data: row.get("ast_data"),
                symbols: row.get("symbols"),
                imports: row.get("imports"),
                file_size: row.get("file_size"),
                parse_duration_ms: row.get("parse_duration_ms"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                accessed_at: row.get("accessed_at"),
            })
            .collect();

        Ok(entries)
    }

    /// List AST cache entries by language
    pub async fn list_by_language(&self, language: &str) -> Result<Vec<AstCache>> {
        let rows = sqlx::query(
            "SELECT id, file_path, language, content_hash, ast_data, symbols, imports,
                    file_size, parse_duration_ms, created_at, updated_at, accessed_at
             FROM ast_cache
             WHERE language = ?
             ORDER BY accessed_at DESC"
        )
        .bind(language)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list AST cache by language: {}", e)))?;

        let entries = rows
            .into_iter()
            .map(|row| AstCache {
                id: row.get("id"),
                file_path: row.get("file_path"),
                language: row.get("language"),
                content_hash: row.get("content_hash"),
                ast_data: row.get("ast_data"),
                symbols: row.get("symbols"),
                imports: row.get("imports"),
                file_size: row.get("file_size"),
                parse_duration_ms: row.get("parse_duration_ms"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                accessed_at: row.get("accessed_at"),
            })
            .collect();

        Ok(entries)
    }

    /// Update an AST cache entry
    pub async fn update(&self, ast: &AstCache) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE ast_cache
             SET file_path = ?, language = ?, content_hash = ?, ast_data = ?, symbols = ?,
                 imports = ?, file_size = ?, parse_duration_ms = ?, updated_at = ?, accessed_at = ?
             WHERE id = ?"
        )
        .bind(&ast.file_path)
        .bind(&ast.language)
        .bind(&ast.content_hash)
        .bind(&ast.ast_data)
        .bind(&ast.symbols)
        .bind(&ast.imports)
        .bind(ast.file_size)
        .bind(ast.parse_duration_ms)
        .bind(updated_at)
        .bind(ast.accessed_at)
        .bind(&ast.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update AST cache: {}", e)))?;

        Ok(())
    }

    /// Delete an AST cache entry
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM ast_cache WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete AST cache: {}", e)))?;

        Ok(())
    }

    /// Delete AST cache entry by file path
    pub async fn delete_by_path(&self, file_path: &str) -> Result<()> {
        sqlx::query("DELETE FROM ast_cache WHERE file_path = ?")
            .bind(file_path)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete AST cache by path: {}", e)))?;

        Ok(())
    }

    /// Update access timestamp for a file
    async fn touch_by_path(&self, file_path: &str) -> Result<()> {
        let accessed_at = Utc::now().timestamp();

        sqlx::query("UPDATE ast_cache SET accessed_at = ? WHERE file_path = ?")
            .bind(accessed_at)
            .bind(file_path)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to update AST cache access time: {}", e)))?;

        Ok(())
    }

    /// Clear stale cache entries (older than specified days)
    pub async fn clear_stale(&self, days: i64) -> Result<usize> {
        let cutoff = Utc::now().timestamp() - (days * 86400); // 86400 seconds per day

        let result = sqlx::query("DELETE FROM ast_cache WHERE accessed_at < ?")
            .bind(cutoff)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to clear stale AST cache: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }

    /// Check if file has cached AST
    pub async fn has_cache_for_file(&self, file_path: &str) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM ast_cache WHERE file_path = ?")
            .bind(file_path)
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to check AST cache: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }
}
