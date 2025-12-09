//! YAML synchronization service
//!
//! Orchestrates the sync process between YAML files and database.
//! Handles startup sync, on-demand sync, and change detection.

use crate::config::deep_merge::deep_merge;
use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::{YamlFile, YamlFileType};
use crate::repositories::YamlFileRepository;
use crate::services::yaml_loader_service::{LoadedYaml, YamlLoaderService};
use crate::services::yaml_validator::YamlValidator;
use serde_json::Value;
use sqlx::Row;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

/// Result of syncing a single file
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// File path that was processed
    pub file_path: String,
    /// Whether file was synced (vs skipped due to no changes)
    pub synced: bool,
    /// Target ID in database
    pub target_id: Option<String>,
    /// Error message if sync failed
    pub error: Option<String>,
}

impl SyncResult {
    pub fn synced(file_path: String, target_id: String) -> Self {
        Self {
            file_path,
            synced: true,
            target_id: Some(target_id),
            error: None,
        }
    }

    pub fn skipped(file_path: String) -> Self {
        Self {
            file_path,
            synced: false,
            target_id: None,
            error: None,
        }
    }

    pub fn error(file_path: String, error: String) -> Self {
        Self {
            file_path,
            synced: false,
            target_id: None,
            error: Some(error),
        }
    }
}

/// Summary report of a sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncReport {
    /// Total files scanned
    pub total_scanned: usize,
    /// Files synced (new or changed)
    pub synced: usize,
    /// Files skipped (unchanged)
    pub skipped: usize,
    /// Files with errors
    pub errors: usize,
    /// Individual results
    pub results: Vec<SyncResult>,
}

impl SyncReport {
    pub fn add_result(&mut self, result: SyncResult) {
        self.total_scanned += 1;
        if result.error.is_some() {
            self.errors += 1;
        } else if result.synced {
            self.synced += 1;
        } else {
            self.skipped += 1;
        }
        self.results.push(result);
    }
}

/// YAML synchronization service
pub struct YamlSyncService {
    db: Arc<Database>,
    yaml_repo: YamlFileRepository,
    loader: YamlLoaderService,
}

impl YamlSyncService {
    /// Create a new sync service
    pub fn new(db: Arc<Database>) -> Self {
        let yaml_repo = YamlFileRepository::new(db.clone());
        let loader = YamlLoaderService::new();
        Self {
            db,
            yaml_repo,
            loader,
        }
    }

    /// Sync all YAML files from specified base directories
    ///
    /// Scans directories recursively, checks checksums, and syncs changed files.
    pub async fn sync_all(&self, base_dirs: &[&str]) -> Result<SyncReport> {
        let mut report = SyncReport::default();

        for base_dir in base_dirs {
            let base_path = Path::new(base_dir);
            if !base_path.exists() {
                warn!(dir = %base_dir, "Skipping non-existent directory");
                continue;
            }

            info!(dir = %base_dir, "Scanning directory for YAML files");

            for entry in WalkDir::new(base_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if !self.loader.is_yaml_file(path) {
                    continue;
                }

                let result = self.sync_file(path).await;
                report.add_result(result);
            }
        }

        info!(
            scanned = report.total_scanned,
            synced = report.synced,
            skipped = report.skipped,
            errors = report.errors,
            "YAML sync complete"
        );

        Ok(report)
    }

    /// Sync a single YAML file
    ///
    /// Checks if file has changed and syncs to database if needed.
    pub async fn sync_file(&self, path: &Path) -> SyncResult {
        let file_path = path.to_string_lossy().to_string();

        // Load the YAML file
        let loaded = match self.loader.load_file(path) {
            Ok(l) => l,
            Err(e) => {
                error!(path = %file_path, error = %e, "Failed to load YAML file");
                return SyncResult::error(file_path.clone(), e.to_string());
            }
        };

        // Check if file exists in tracking database
        match self.yaml_repo.find_by_file_path(&file_path).await {
            Ok(existing) => {
                // File is tracked - check if changed
                if !existing.is_stale(&loaded.content_hash) {
                    debug!(path = %file_path, "File unchanged, skipping");
                    return SyncResult::skipped(file_path);
                }

                // File changed - sync it
                debug!(path = %file_path, "File changed, syncing");
                self.sync_changed_file(&existing, &loaded).await
            }
            Err(_) => {
                // File not tracked - new file
                debug!(path = %file_path, "New file, syncing");
                self.sync_new_file(&loaded).await
            }
        }
    }

    /// Sync a new file (not previously tracked)
    async fn sync_new_file(&self, loaded: &LoadedYaml) -> SyncResult {
        // Validate and apply to target table
        let target_id = match self.apply_to_database(loaded, None).await {
            Ok(id) => id,
            Err(e) => {
                error!(path = %loaded.file_path, error = %e, "Failed to apply new file to database");
                return SyncResult::error(loaded.file_path.clone(), e.to_string());
            }
        };

        // Create tracking entry
        let yaml_file = YamlFile::new(
            loaded.file_path.clone(),
            loaded.file_type,
            loaded.content_hash.clone(),
        )
        .with_file_size(loaded.file_size)
        .with_target_id(target_id.clone());

        if let Err(e) = self.yaml_repo.save(&yaml_file).await {
            error!(path = %loaded.file_path, error = %e, "Failed to save tracking entry");
            return SyncResult::error(loaded.file_path.clone(), e.to_string());
        }

        // Mark as synced
        if let Err(e) = self
            .yaml_repo
            .mark_synced(&loaded.file_path, &target_id)
            .await
        {
            warn!(path = %loaded.file_path, error = %e, "Failed to mark as synced");
        }

        SyncResult::synced(loaded.file_path.clone(), target_id)
    }

    /// Sync a changed file (already tracked)
    async fn sync_changed_file(&self, existing: &YamlFile, loaded: &LoadedYaml) -> SyncResult {
        // Update hash first to mark as pending
        if let Err(e) = self
            .yaml_repo
            .update_hash(&loaded.file_path, &loaded.content_hash)
            .await
        {
            return SyncResult::error(loaded.file_path.clone(), e.to_string());
        }

        // Apply to database with merge
        let target_id = match self
            .apply_to_database(loaded, existing.target_id.as_deref())
            .await
        {
            Ok(id) => id,
            Err(e) => {
                // Mark as error
                let _ = self
                    .yaml_repo
                    .mark_error(&loaded.file_path, &e.to_string())
                    .await;
                return SyncResult::error(loaded.file_path.clone(), e.to_string());
            }
        };

        // Mark as synced
        if let Err(e) = self
            .yaml_repo
            .mark_synced(&loaded.file_path, &target_id)
            .await
        {
            warn!(path = %loaded.file_path, error = %e, "Failed to mark as synced");
        }

        SyncResult::synced(loaded.file_path.clone(), target_id)
    }

    /// Apply YAML content to target database table
    ///
    /// If existing_id is provided, performs a deep merge with existing data.
    async fn apply_to_database(
        &self,
        loaded: &LoadedYaml,
        existing_id: Option<&str>,
    ) -> Result<String> {
        match loaded.file_type {
            YamlFileType::Workflow => self.apply_workflow(loaded, existing_id).await,
            YamlFileType::Pattern => self.apply_pattern(loaded, existing_id).await,
            YamlFileType::Prompt => self.apply_prompt(loaded, existing_id).await,
            YamlFileType::Template => self.apply_workflow(loaded, existing_id).await,
            YamlFileType::Tool => self.apply_tool(loaded, existing_id).await,
        }
    }

    /// Apply workflow YAML to workflow_templates table
    async fn apply_workflow(&self, loaded: &LoadedYaml, existing_id: Option<&str>) -> Result<String> {
        let validated = YamlValidator::validate_workflow(&loaded.content)?;

        // Determine final content (merge if existing)
        let final_content = if let Some(id) = existing_id {
            self.merge_with_existing("workflow_templates", id, &loaded.content)
                .await?
        } else {
            loaded.content.clone()
        };

        let definition_json = serde_json::to_string(&final_content)
            .map_err(|e| OrcaError::Other(format!("Failed to serialize workflow: {}", e)))?;

        let tags_json = serde_json::to_string(&validated.tags)
            .map_err(|e| OrcaError::Other(format!("Failed to serialize tags: {}", e)))?;

        let now = chrono::Utc::now().timestamp();

        if let Some(id) = existing_id {
            // Update existing
            sqlx::query(
                "UPDATE workflow_templates
                 SET name = ?, description = ?, pattern = ?, definition = ?, tags = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(&validated.name)
            .bind(&validated.description)
            .bind(&validated.pattern)
            .bind(&definition_json)
            .bind(&tags_json)
            .bind(now)
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to update workflow: {}", e)))?;

            Ok(id.to_string())
        } else {
            // Insert new
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO workflow_templates (id, name, description, pattern, definition, tags, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&validated.name)
            .bind(&validated.description)
            .bind(&validated.pattern)
            .bind(&definition_json)
            .bind(&tags_json)
            .bind(now)
            .bind(now)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to insert workflow: {}", e)))?;

            Ok(id)
        }
    }

    /// Apply pattern YAML to pattern_configs table
    async fn apply_pattern(&self, loaded: &LoadedYaml, existing_id: Option<&str>) -> Result<String> {
        let validated = YamlValidator::validate_pattern(&loaded.content)?;

        let _final_content = if let Some(id) = existing_id {
            self.merge_with_existing("pattern_configs", id, &loaded.content)
                .await?
        } else {
            loaded.content.clone()
        };

        let config_json = serde_json::to_string(&validated.config)
            .map_err(|e| OrcaError::Other(format!("Failed to serialize config: {}", e)))?;

        let tools_json = serde_json::to_string(&validated.tools)
            .map_err(|e| OrcaError::Other(format!("Failed to serialize tools: {}", e)))?;

        let now = chrono::Utc::now().timestamp();

        if let Some(id) = existing_id {
            sqlx::query(
                "UPDATE pattern_configs
                 SET name = ?, pattern_type = ?, config = ?, tools = ?, system_prompt = ?,
                     max_iterations = ?, is_default = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(&validated.name)
            .bind(&validated.pattern_type)
            .bind(&config_json)
            .bind(&tools_json)
            .bind(&validated.system_prompt)
            .bind(validated.max_iterations)
            .bind(validated.is_default)
            .bind(now)
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to update pattern: {}", e)))?;

            Ok(id.to_string())
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO pattern_configs (id, name, pattern_type, config, tools, system_prompt, max_iterations, is_default, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&validated.name)
            .bind(&validated.pattern_type)
            .bind(&config_json)
            .bind(&tools_json)
            .bind(&validated.system_prompt)
            .bind(validated.max_iterations)
            .bind(validated.is_default)
            .bind(now)
            .bind(now)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to insert pattern: {}", e)))?;

            Ok(id)
        }
    }

    /// Apply prompt YAML to prompts table
    async fn apply_prompt(&self, loaded: &LoadedYaml, existing_id: Option<&str>) -> Result<String> {
        let validated = YamlValidator::validate_prompt(&loaded.content)?;

        let variables_json = serde_json::to_string(&validated.variables)
            .map_err(|e| OrcaError::Other(format!("Failed to serialize variables: {}", e)))?;

        let now = chrono::Utc::now().timestamp();

        if let Some(id) = existing_id {
            sqlx::query(
                "UPDATE prompts
                 SET name = ?, template = ?, category = ?, description = ?, variables = ?, updated_at = ?
                 WHERE id = ?",
            )
            .bind(&validated.name)
            .bind(&validated.template)
            .bind(&validated.category)
            .bind(&validated.description)
            .bind(&variables_json)
            .bind(now)
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to update prompt: {}", e)))?;

            Ok(id.to_string())
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO prompts (id, name, template, category, description, variables, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&validated.name)
            .bind(&validated.template)
            .bind(&validated.category)
            .bind(&validated.description)
            .bind(&variables_json)
            .bind(now)
            .bind(now)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to insert prompt: {}", e)))?;

            Ok(id)
        }
    }

    /// Apply tool YAML to tools table
    async fn apply_tool(&self, loaded: &LoadedYaml, existing_id: Option<&str>) -> Result<String> {
        let validated = YamlValidator::validate_tool(&loaded.content)?;

        let _final_content = if let Some(id) = existing_id {
            self.merge_with_existing("tools", id, &loaded.content)
                .await?
        } else {
            loaded.content.clone()
        };

        let config_json = serde_json::to_string(&validated.config)
            .map_err(|e| OrcaError::Other(format!("Failed to serialize tool config: {}", e)))?;

        let now = chrono::Utc::now().timestamp();

        if let Some(id) = existing_id {
            sqlx::query(
                "UPDATE tools SET name = ?, description = ?, config = ?, updated_at = ? WHERE id = ?",
            )
            .bind(&validated.name)
            .bind(&validated.description)
            .bind(&config_json)
            .bind(now)
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to update tool: {}", e)))?;

            Ok(id.to_string())
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO tools (id, name, description, config, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&validated.name)
            .bind(&validated.description)
            .bind(&config_json)
            .bind(now)
            .bind(now)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to insert tool: {}", e)))?;

            Ok(id)
        }
    }

    /// Merge new content with existing database record
    async fn merge_with_existing(
        &self,
        table: &str,
        id: &str,
        new_content: &Value,
    ) -> Result<Value> {
        // Fetch existing definition/config from database
        let column = match table {
            "workflow_templates" => "definition",
            "pattern_configs" => "config",
            "tools" => "config",
            _ => return Ok(new_content.clone()),
        };

        let query = format!("SELECT {} FROM {} WHERE id = ?", column, table);
        let row = sqlx::query(&query)
            .bind(id)
            .fetch_optional(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to fetch existing: {}", e)))?;

        if let Some(row) = row {
            let existing_json: String = row.get(column);
            let mut existing: Value = serde_json::from_str(&existing_json)
                .map_err(|e| OrcaError::Other(format!("Failed to parse existing JSON: {}", e)))?;

            // Deep merge: new content overlays existing
            deep_merge(&mut existing, new_content);
            Ok(existing)
        } else {
            Ok(new_content.clone())
        }
    }

    /// Get sync statistics
    pub async fn get_stats(&self) -> Result<crate::repositories::yaml_file_repository::YamlFileStats> {
        self.yaml_repo.get_stats().await
    }

    /// List files with errors
    pub async fn list_errors(&self) -> Result<Vec<YamlFile>> {
        self.yaml_repo.list_errors().await
    }

    /// List files pending sync
    pub async fn list_pending(&self) -> Result<Vec<YamlFile>> {
        self.yaml_repo.list_pending().await
    }

    /// Retry syncing files with errors
    pub async fn retry_errors(&self) -> Result<SyncReport> {
        let error_files = self.yaml_repo.list_errors().await?;
        let mut report = SyncReport::default();

        for yaml_file in error_files {
            let path = Path::new(&yaml_file.file_path);
            if path.exists() {
                let result = self.sync_file(path).await;
                report.add_result(result);
            } else {
                // File no longer exists, delete tracking entry
                let _ = self.yaml_repo.delete(&yaml_file.id).await;
            }
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_result() {
        let synced = SyncResult::synced("test.yaml".into(), "123".into());
        assert!(synced.synced);
        assert!(synced.error.is_none());

        let skipped = SyncResult::skipped("test.yaml".into());
        assert!(!skipped.synced);
        assert!(skipped.error.is_none());

        let error = SyncResult::error("test.yaml".into(), "oops".into());
        assert!(!error.synced);
        assert!(error.error.is_some());
    }

    #[test]
    fn test_sync_report() {
        let mut report = SyncReport::default();

        report.add_result(SyncResult::synced("a.yaml".into(), "1".into()));
        report.add_result(SyncResult::skipped("b.yaml".into()));
        report.add_result(SyncResult::error("c.yaml".into(), "err".into()));

        assert_eq!(report.total_scanned, 3);
        assert_eq!(report.synced, 1);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.errors, 1);
    }
}
