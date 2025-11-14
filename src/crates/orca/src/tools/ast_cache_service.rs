//! AST Cache Service
//!
//! High-level service for managing cached Abstract Syntax Trees with automatic
//! validation, cache invalidation, and cleanup utilities.

use crate::DatabaseManager;
use crate::error::{OrcaError, Result};
use crate::models::AstCache;
use crate::repositories::AstCacheRepository;
use sha2::{Digest, Sha256};
use std::fs;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Cache hit/miss statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: usize,
    /// Number of cache misses
    pub misses: usize,
    /// Number of stale entries (invalidated)
    pub stale: usize,
    /// Total cache queries
    pub queries: usize,
}

impl CacheStats {
    /// Calculate cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        if self.queries == 0 {
            0.0
        } else {
            self.hits as f64 / self.queries as f64
        }
    }
}

/// AST cache service
///
/// Provides high-level caching operations with automatic validation,
/// content hash checking, and cache maintenance.
pub struct AstCacheService {
    /// Database manager for accessing cache
    db_manager: Arc<DatabaseManager>,

    /// Cache statistics
    stats: std::sync::Mutex<CacheStats>,
}

impl AstCacheService {
    /// Create a new AST cache service
    ///
    /// # Arguments
    /// * `db_manager` - Database manager with project database access
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            db_manager,
            stats: std::sync::Mutex::new(CacheStats::default()),
        }
    }

    /// Get cached AST for a file
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    ///
    /// # Returns
    /// Cached AST if valid, None if cache miss or stale
    pub async fn get(&self, file_path: &str) -> Result<Option<AstCache>> {
        let start_time = Instant::now();

        // Get project database
        let project_db = match self.db_manager.project_db() {
            Some(db) => db,
            None => {
                debug!("No project database available for AST cache");
                self.record_miss();
                return Ok(None);
            }
        };

        let repo = AstCacheRepository::new(project_db.clone());

        // Try to find cached entry
        let cached = match repo.find_by_file_path(file_path).await {
            Ok(entry) => entry,
            Err(_) => {
                debug!(file = file_path, "AST cache miss");
                self.record_miss();
                return Ok(None);
            }
        };

        // Validate cache entry (check if file has changed)
        let current_hash = self.compute_file_hash(file_path)?;

        if cached.is_stale(&current_hash) {
            debug!(file = file_path, "AST cache stale (file modified)");
            self.record_stale();

            // Delete stale entry
            let _ = repo.delete_by_path(file_path).await;

            return Ok(None);
        }

        let duration = start_time.elapsed();
        debug!(
            file = file_path,
            duration_ms = duration.as_millis(),
            "AST cache hit"
        );

        self.record_hit();
        Ok(Some(cached))
    }

    /// Store AST in cache
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    /// * `language` - Programming language
    /// * `ast_data` - Serialized AST data (JSON)
    /// * `symbols` - Optional extracted symbols (JSON array)
    /// * `imports` - Optional extracted imports (JSON array)
    /// * `parse_duration_ms` - Time taken to parse
    pub async fn put(
        &self,
        file_path: &str,
        language: &str,
        ast_data: String,
        symbols: Option<String>,
        imports: Option<String>,
        parse_duration_ms: i64,
    ) -> Result<()> {
        let start_time = Instant::now();

        // Get project database
        let project_db = match self.db_manager.project_db() {
            Some(db) => db,
            None => {
                warn!("No project database available for AST cache");
                return Ok(());
            }
        };

        let repo = AstCacheRepository::new(project_db.clone());

        // Compute file hash
        let content_hash = self.compute_file_hash(file_path)?;

        // Get file size
        let file_size = fs::metadata(file_path)
            .map(|m| m.len() as i64)
            .ok();

        // Check if entry already exists
        let exists = repo.has_cache_for_file(file_path).await?;

        if exists {
            // Update existing entry
            let mut cached = repo.find_by_file_path(file_path).await?;
            cached.language = language.to_string();
            cached.content_hash = content_hash;
            cached.ast_data = ast_data;
            cached.symbols = symbols;
            cached.imports = imports;
            cached.file_size = file_size;
            cached.parse_duration_ms = Some(parse_duration_ms);
            cached.touch();

            repo.update(&cached).await?;
        } else {
            // Create new entry
            let mut cached = AstCache::new(
                file_path.to_string(),
                language.to_string(),
                content_hash,
                ast_data,
            );

            if let Some(syms) = symbols {
                cached = cached.with_symbols(syms);
            }

            if let Some(imps) = imports {
                cached = cached.with_imports(imps);
            }

            if let Some(size) = file_size {
                cached = cached.with_file_size(size);
            }

            cached = cached.with_parse_duration(parse_duration_ms);

            repo.save(&cached).await?;
        }

        let duration = start_time.elapsed();
        debug!(
            file = file_path,
            duration_ms = duration.as_millis(),
            "AST cached"
        );

        Ok(())
    }

    /// Invalidate cache for a file
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    pub async fn invalidate(&self, file_path: &str) -> Result<()> {
        let project_db = match self.db_manager.project_db() {
            Some(db) => db,
            None => return Ok(()),
        };

        let repo = AstCacheRepository::new(project_db.clone());
        repo.delete_by_path(file_path).await?;

        debug!(file = file_path, "AST cache invalidated");
        Ok(())
    }

    /// Clear stale cache entries (older than specified days)
    ///
    /// # Arguments
    /// * `days` - Age threshold in days
    ///
    /// # Returns
    /// Number of entries cleared
    pub async fn clear_stale(&self, days: i64) -> Result<usize> {
        let project_db = match self.db_manager.project_db() {
            Some(db) => db,
            None => return Ok(0),
        };

        let repo = AstCacheRepository::new(project_db.clone());
        let count = repo.clear_stale(days).await?;

        info!(count = count, days = days, "Cleared stale AST cache entries");
        Ok(count)
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    /// Reset cache statistics
    pub fn reset_stats(&self) {
        *self.stats.lock().unwrap() = CacheStats::default();
    }

    /// Get all cached entries
    pub async fn list_all(&self) -> Result<Vec<AstCache>> {
        let project_db = match self.db_manager.project_db() {
            Some(db) => db,
            None => return Ok(Vec::new()),
        };

        let repo = AstCacheRepository::new(project_db.clone());
        repo.list().await
    }

    /// Get cached entries by language
    pub async fn list_by_language(&self, language: &str) -> Result<Vec<AstCache>> {
        let project_db = match self.db_manager.project_db() {
            Some(db) => db,
            None => return Ok(Vec::new()),
        };

        let repo = AstCacheRepository::new(project_db.clone());
        repo.list_by_language(language).await
    }

    /// Compute SHA-256 hash of file content
    fn compute_file_hash(&self, file_path: &str) -> Result<String> {
        let content = fs::read(file_path)
            .map_err(|e| OrcaError::Other(format!("Failed to read file {}: {}", file_path, e)))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();

        Ok(format!("{:x}", hash))
    }

    /// Record cache hit
    fn record_hit(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.hits += 1;
        stats.queries += 1;
    }

    /// Record cache miss
    fn record_miss(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.misses += 1;
        stats.queries += 1;
    }

    /// Record stale entry
    fn record_stale(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.stale += 1;
        stats.queries += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats {
            hits: 75,
            misses: 25,
            stale: 5,
            queries: 100,
        };

        assert_eq!(stats.hit_rate(), 0.75);
    }

    #[test]
    fn test_cache_stats_hit_rate_zero_queries() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.stale, 0);
        assert_eq!(stats.queries, 0);
    }
}
