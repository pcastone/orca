//! Backup service for database backup, restore, export, and import operations
//!
//! Provides functionality for:
//! - Full database backup (copy SQLite files)
//! - Restore from backup
//! - Export specific tables as SQL dump
//! - Import SQL dump with merge (INSERT OR REPLACE)

use crate::db::manager::DatabaseManager;
use crate::error::{OrcaError, Result};
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Table groups for export convenience
pub struct TableGroups;

impl TableGroups {
    /// LLM-related tables
    pub const LLM: &'static [&'static str] = &["llm_providers"];

    /// Budget-related tables
    pub const BUDGETS: &'static [&'static str] = &["budgets", "usage_logs", "llm_pricing"];

    /// Bug tracking tables
    pub const BUGS: &'static [&'static str] = &["bugs", "task_bugs"];

    /// Task and workflow tables
    pub const TASKS: &'static [&'static str] = &["tasks", "workflows", "workflow_tasks"];

    /// Pattern configuration tables
    pub const PATTERNS: &'static [&'static str] = &["pattern_configs"];

    /// AST cache tables
    pub const AST: &'static [&'static str] = &["ast_cache"];

    /// Get tables for a group name
    pub fn get(name: &str) -> Option<&'static [&'static str]> {
        match name.to_lowercase().as_str() {
            "llm" => Some(Self::LLM),
            "budgets" => Some(Self::BUDGETS),
            "bugs" => Some(Self::BUGS),
            "tasks" => Some(Self::TASKS),
            "patterns" => Some(Self::PATTERNS),
            "ast" => Some(Self::AST),
            _ => None,
        }
    }

    /// Get all table group names
    pub fn all_names() -> Vec<&'static str> {
        vec!["llm", "budgets", "bugs", "tasks", "patterns", "ast"]
    }
}

/// Information about a backup
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Path to the backup file
    pub path: PathBuf,
    /// When the backup was created
    pub timestamp: DateTime<Utc>,
    /// Size of the backup in bytes
    pub size_bytes: u64,
    /// Whether this backup includes user database
    pub includes_user_db: bool,
    /// Whether this backup includes project database
    pub includes_project_db: bool,
}

/// Result of an import operation
#[derive(Debug, Clone, Default)]
pub struct ImportResult {
    /// Number of records inserted
    pub records_inserted: usize,
    /// Number of records updated (replaced)
    pub records_updated: usize,
    /// Number of records skipped
    pub records_skipped: usize,
    /// Tables that were imported
    pub tables_imported: Vec<String>,
}

/// Backup service for managing database backups
#[derive(Clone, Debug)]
pub struct BackupService {
    /// Directory to store backups
    backup_dir: PathBuf,
}

impl BackupService {
    /// Create a new backup service
    ///
    /// # Arguments
    /// * `backup_dir` - Optional custom backup directory. Defaults to ~/.orca/backups/
    pub fn new(backup_dir: Option<PathBuf>) -> Self {
        let dir = backup_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .expect("Failed to get home directory")
                .join(".orca")
                .join("backups")
        });

        Self { backup_dir: dir }
    }

    /// Get the backup directory
    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// Ensure backup directory exists
    fn ensure_backup_dir(&self) -> Result<()> {
        if !self.backup_dir.exists() {
            std::fs::create_dir_all(&self.backup_dir).map_err(|e| {
                OrcaError::Other(format!("Failed to create backup directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Create a timestamped backup of databases
    ///
    /// # Arguments
    /// * `db_manager` - Database manager with connections
    /// * `include_project` - Whether to include project database
    ///
    /// # Returns
    /// Information about the created backup
    pub async fn backup(
        &self,
        db_manager: &DatabaseManager,
        include_project: bool,
    ) -> Result<BackupInfo> {
        self.ensure_backup_dir()?;

        let timestamp = Utc::now();
        let timestamp_str = timestamp.format("%Y%m%d_%H%M%S").to_string();

        // Get source paths
        let user_db_path = dirs::home_dir()
            .ok_or_else(|| OrcaError::Other("Cannot determine home directory".to_string()))?
            .join(".orca")
            .join("user.db");

        let project_db_path = db_manager.project_root().map(|root| root.join(".orca").join("project.db"));

        let mut includes_project = false;
        let backup_path: PathBuf;
        let mut total_size: u64 = 0;

        if include_project && project_db_path.is_some() && project_db_path.as_ref().unwrap().exists() {
            // Create combined backup as tar
            backup_path = self.backup_dir.join(format!("backup_{}.tar", timestamp_str));

            // Create tar archive
            let file = std::fs::File::create(&backup_path)
                .map_err(|e| OrcaError::Other(format!("Failed to create backup file: {}", e)))?;
            let mut builder = tar::Builder::new(file);

            // Add user database
            if user_db_path.exists() {
                builder
                    .append_path_with_name(&user_db_path, "user.db")
                    .map_err(|e| OrcaError::Other(format!("Failed to add user.db to backup: {}", e)))?;
            }

            // Add project database
            if let Some(ref proj_path) = project_db_path {
                if proj_path.exists() {
                    builder
                        .append_path_with_name(proj_path, "project.db")
                        .map_err(|e| OrcaError::Other(format!("Failed to add project.db to backup: {}", e)))?;
                    includes_project = true;
                }
            }

            builder
                .finish()
                .map_err(|e| OrcaError::Other(format!("Failed to finalize backup: {}", e)))?;

            total_size = std::fs::metadata(&backup_path)
                .map(|m| m.len())
                .unwrap_or(0);
        } else {
            // Just backup user database
            backup_path = self.backup_dir.join(format!("backup_{}_user.db", timestamp_str));

            std::fs::copy(&user_db_path, &backup_path)
                .map_err(|e| OrcaError::Other(format!("Failed to copy user database: {}", e)))?;

            total_size = std::fs::metadata(&backup_path)
                .map(|m| m.len())
                .unwrap_or(0);
        }

        info!(path = %backup_path.display(), "Backup created successfully");

        Ok(BackupInfo {
            path: backup_path,
            timestamp,
            size_bytes: total_size,
            includes_user_db: true,
            includes_project_db: includes_project,
        })
    }

    /// List available backups
    ///
    /// # Returns
    /// List of backup information, sorted by timestamp (newest first)
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        if !self.backup_dir.exists() {
            return Ok(vec![]);
        }

        let mut backups = vec![];

        for entry in std::fs::read_dir(&self.backup_dir)
            .map_err(|e| OrcaError::Other(format!("Failed to read backup directory: {}", e)))?
        {
            let entry = entry.map_err(|e| OrcaError::Other(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Parse backup filename: backup_YYYYMMDD_HHMMSS.tar or backup_YYYYMMDD_HHMMSS_user.db
            if !filename.starts_with("backup_") {
                continue;
            }

            let is_tar = filename.ends_with(".tar");
            let is_user_db = filename.ends_with("_user.db");

            if !is_tar && !is_user_db && !filename.ends_with(".db") {
                continue;
            }

            // Extract timestamp from filename
            let timestamp_str = if is_tar {
                filename.strip_prefix("backup_").and_then(|s| s.strip_suffix(".tar"))
            } else if is_user_db {
                filename.strip_prefix("backup_").and_then(|s| s.strip_suffix("_user.db"))
            } else {
                filename.strip_prefix("backup_").and_then(|s| s.strip_suffix(".db"))
            };

            let timestamp = if let Some(ts) = timestamp_str {
                chrono::NaiveDateTime::parse_from_str(ts, "%Y%m%d_%H%M%S")
                    .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
                    .unwrap_or_else(|_| Utc::now())
            } else {
                // Use file modification time as fallback
                std::fs::metadata(&path)
                    .and_then(|m| m.modified())
                    .map(|t| DateTime::from(t))
                    .unwrap_or_else(|_| Utc::now())
            };

            let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            backups.push(BackupInfo {
                path,
                timestamp,
                size_bytes,
                includes_user_db: true,
                includes_project_db: is_tar,
            });
        }

        // Sort by timestamp, newest first
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(backups)
    }

    /// Restore from a backup file
    ///
    /// # Arguments
    /// * `backup_path` - Path to the backup file
    /// * `db_manager` - Database manager for connection info
    ///
    /// # Warning
    /// This will overwrite existing databases!
    pub async fn restore(
        &self,
        backup_path: &Path,
        db_manager: &DatabaseManager,
    ) -> Result<()> {
        if !backup_path.exists() {
            return Err(OrcaError::NotFound(format!(
                "Backup file not found: {}",
                backup_path.display()
            )));
        }

        let user_db_path = dirs::home_dir()
            .ok_or_else(|| OrcaError::Other("Cannot determine home directory".to_string()))?
            .join(".orca")
            .join("user.db");

        let project_db_path = db_manager.project_root().map(|root| root.join(".orca").join("project.db"));

        let filename = backup_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if filename.ends_with(".tar") {
            // Extract tar archive
            let file = std::fs::File::open(backup_path)
                .map_err(|e| OrcaError::Other(format!("Failed to open backup: {}", e)))?;
            let mut archive = tar::Archive::new(file);

            for entry in archive
                .entries()
                .map_err(|e| OrcaError::Other(format!("Failed to read backup: {}", e)))?
            {
                let mut entry = entry.map_err(|e| OrcaError::Other(format!("Failed to read entry: {}", e)))?;
                let entry_path = entry.path()
                    .map_err(|e| OrcaError::Other(format!("Failed to get path: {}", e)))?
                    .to_path_buf();

                let entry_name = entry_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if entry_name == "user.db" {
                    // Create backup of current user.db before overwriting
                    if user_db_path.exists() {
                        let backup = user_db_path.with_extension("db.bak");
                        std::fs::copy(&user_db_path, &backup).ok();
                    }

                    // Extract to user database location
                    entry
                        .unpack(&user_db_path)
                        .map_err(|e| OrcaError::Other(format!("Failed to restore user.db: {}", e)))?;

                    info!("Restored user database");
                } else if entry_name == "project.db" {
                    if let Some(ref proj_path) = project_db_path {
                        // Create backup of current project.db before overwriting
                        if proj_path.exists() {
                            let backup = proj_path.with_extension("db.bak");
                            std::fs::copy(proj_path, &backup).ok();
                        }

                        entry
                            .unpack(proj_path)
                            .map_err(|e| OrcaError::Other(format!("Failed to restore project.db: {}", e)))?;

                        info!("Restored project database");
                    } else {
                        warn!("Backup contains project.db but no project context exists");
                    }
                }
            }
        } else {
            // Single database file - assume it's user.db
            if user_db_path.exists() {
                let backup = user_db_path.with_extension("db.bak");
                std::fs::copy(&user_db_path, &backup).ok();
            }

            std::fs::copy(backup_path, &user_db_path)
                .map_err(|e| OrcaError::Other(format!("Failed to restore database: {}", e)))?;

            info!("Restored user database");
        }

        Ok(())
    }

    /// Export specific tables as SQL dump
    ///
    /// # Arguments
    /// * `db_manager` - Database manager with connections
    /// * `tables` - Table names or group names to export ("all" for everything)
    /// * `output` - Output file path
    pub async fn export(
        &self,
        db_manager: &DatabaseManager,
        tables: &[String],
        output: &Path,
    ) -> Result<()> {
        let resolved_tables = self.resolve_table_names(tables)?;

        let mut sql = String::new();

        // Header
        sql.push_str("-- Orca Export\n");
        sql.push_str(&format!("-- Created: {}\n", Utc::now().to_rfc3339()));
        sql.push_str(&format!("-- Tables: {}\n", resolved_tables.iter().cloned().collect::<Vec<_>>().join(", ")));
        sql.push_str("\n");
        sql.push_str("BEGIN TRANSACTION;\n\n");

        // Export from user database
        let user_tables = self.get_existing_tables(db_manager.user_db().pool()).await?;
        for table in &resolved_tables {
            if user_tables.contains(&table.to_string()) {
                debug!(table = %table, "Exporting table from user database");
                let table_sql = self.export_table(db_manager.user_db().pool(), table).await?;
                sql.push_str(&format!("-- Table: {} (from user.db)\n", table));
                sql.push_str(&table_sql);
                sql.push_str("\n");
            }
        }

        // Export from project database if available
        if let Some(project_db) = db_manager.project_db() {
            let project_tables = self.get_existing_tables(project_db.pool()).await?;
            for table in &resolved_tables {
                if project_tables.contains(&table.to_string()) && !user_tables.contains(&table.to_string()) {
                    debug!(table = %table, "Exporting table from project database");
                    let table_sql = self.export_table(project_db.pool(), table).await?;
                    sql.push_str(&format!("-- Table: {} (from project.db)\n", table));
                    sql.push_str(&table_sql);
                    sql.push_str("\n");
                }
            }
        }

        sql.push_str("COMMIT;\n");

        // Write to file
        std::fs::write(output, &sql)
            .map_err(|e| OrcaError::Other(format!("Failed to write export file: {}", e)))?;

        info!(path = %output.display(), "Export completed");

        Ok(())
    }

    /// Import SQL dump with merge strategy
    ///
    /// # Arguments
    /// * `db_manager` - Database manager with connections
    /// * `input` - Input file path
    /// * `tables` - Optional list of tables to import ("all" for everything)
    pub async fn import(
        &self,
        db_manager: &DatabaseManager,
        input: &Path,
        tables: Option<&[String]>,
    ) -> Result<ImportResult> {
        if !input.exists() {
            return Err(OrcaError::NotFound(format!(
                "Import file not found: {}",
                input.display()
            )));
        }

        let content = std::fs::read_to_string(input)
            .map_err(|e| OrcaError::Other(format!("Failed to read import file: {}", e)))?;

        // Check if it's a SQL dump or a database file
        if input.extension().and_then(|e| e.to_str()) == Some("db") {
            return self.import_from_backup(db_manager, input, tables).await;
        }

        let resolved_tables: Option<HashSet<String>> = tables.map(|t| {
            self.resolve_table_names(t)
                .unwrap_or_default()
        });

        let mut result = ImportResult::default();

        // Parse and execute SQL statements
        let statements: Vec<&str> = content
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && !s.starts_with("--"))
            .collect();

        let user_pool = db_manager.user_db().pool();
        let project_pool = db_manager.project_db().map(|db| db.pool());

        let user_tables = self.get_existing_tables(user_pool).await?;
        let project_tables = if let Some(pool) = project_pool {
            self.get_existing_tables(pool).await?
        } else {
            HashSet::new()
        };

        for statement in statements {
            // Skip transaction control statements
            if statement.to_uppercase().starts_with("BEGIN") ||
               statement.to_uppercase().starts_with("COMMIT") {
                continue;
            }

            // Extract table name from INSERT statement
            let table_name = self.extract_table_name(statement);

            if let Some(ref filter) = resolved_tables {
                if let Some(ref name) = table_name {
                    if !filter.contains(name) {
                        result.records_skipped += 1;
                        continue;
                    }
                }
            }

            // Determine which database to use
            let pool = if let Some(ref name) = table_name {
                if user_tables.contains(name) {
                    user_pool
                } else if project_tables.contains(name) {
                    project_pool.unwrap_or(user_pool)
                } else {
                    user_pool // Default to user pool
                }
            } else {
                user_pool
            };

            // Execute statement
            match sqlx::query(statement).execute(pool).await {
                Ok(res) => {
                    let affected = res.rows_affected() as usize;
                    if affected > 0 {
                        result.records_inserted += affected;
                        if let Some(name) = table_name {
                            if !result.tables_imported.contains(&name) {
                                result.tables_imported.push(name);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, statement = %statement.chars().take(100).collect::<String>(), "Failed to execute statement");
                    result.records_skipped += 1;
                }
            }
        }

        info!(
            inserted = result.records_inserted,
            skipped = result.records_skipped,
            "Import completed"
        );

        Ok(result)
    }

    /// Import tables from a backup database file
    async fn import_from_backup(
        &self,
        db_manager: &DatabaseManager,
        backup_path: &Path,
        tables: Option<&[String]>,
    ) -> Result<ImportResult> {
        // Open the backup database
        let backup_url = format!("sqlite:{}", backup_path.display());
        let backup_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&backup_url)
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to open backup: {}", e)))?;

        let backup_tables = self.get_existing_tables(&backup_pool).await?;

        let resolved_tables: HashSet<String> = if let Some(t) = tables {
            self.resolve_table_names(t)?
        } else {
            backup_tables.clone()
        };

        let mut result = ImportResult::default();

        for table in resolved_tables.intersection(&backup_tables) {
            let table_sql = self.export_table(&backup_pool, table).await?;

            // Execute in appropriate database
            let user_tables = self.get_existing_tables(db_manager.user_db().pool()).await?;
            let pool = if user_tables.contains(table) {
                db_manager.user_db().pool()
            } else if let Some(project_db) = db_manager.project_db() {
                project_db.pool()
            } else {
                db_manager.user_db().pool()
            };

            for statement in table_sql.split(';').filter(|s| !s.trim().is_empty()) {
                match sqlx::query(statement.trim()).execute(pool).await {
                    Ok(res) => {
                        result.records_inserted += res.rows_affected() as usize;
                    }
                    Err(e) => {
                        warn!(error = %e, table = %table, "Failed to import row");
                        result.records_skipped += 1;
                    }
                }
            }

            if !result.tables_imported.contains(table) {
                result.tables_imported.push(table.clone());
            }
        }

        backup_pool.close().await;

        Ok(result)
    }

    /// Resolve table names/groups to actual table names
    fn resolve_table_names(&self, tables: &[String]) -> Result<HashSet<String>> {
        let mut resolved = HashSet::new();

        for table in tables {
            let lower = table.to_lowercase();

            if lower == "all" {
                // Add all known tables from all groups
                for group in TableGroups::all_names() {
                    if let Some(group_tables) = TableGroups::get(group) {
                        for t in group_tables {
                            resolved.insert(t.to_string());
                        }
                    }
                }
            } else if let Some(group_tables) = TableGroups::get(&lower) {
                // It's a group name
                for t in group_tables {
                    resolved.insert(t.to_string());
                }
            } else {
                // It's a table name
                resolved.insert(table.clone());
            }
        }

        Ok(resolved)
    }

    /// Get list of tables that exist in the database
    async fn get_existing_tables(&self, pool: &sqlx::SqlitePool) -> Result<HashSet<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx_%'"
        )
        .fetch_all(pool)
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get tables: {}", e)))?;

        Ok(rows.into_iter().map(|(name,)| name).collect())
    }

    /// Export a single table as SQL INSERT statements
    async fn export_table(&self, pool: &sqlx::SqlitePool, table: &str) -> Result<String> {
        // Get column info
        let columns: Vec<(String,)> = sqlx::query_as(&format!(
            "SELECT name FROM pragma_table_info('{}')",
            table
        ))
        .fetch_all(pool)
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get columns for {}: {}", table, e)))?;

        if columns.is_empty() {
            return Ok(String::new());
        }

        let column_names: Vec<String> = columns.into_iter().map(|(n,)| n).collect();
        let columns_str = column_names.join(", ");

        // Get all rows
        let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(&format!(
            "SELECT {} FROM {}",
            columns_str, table
        ))
        .fetch_all(pool)
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to query {}: {}", table, e)))?;

        let mut sql = String::new();

        for row in rows {
            let mut values = Vec::new();
            for col in &column_names {
                let value = self.get_row_value(&row, col);
                values.push(value);
            }

            sql.push_str(&format!(
                "INSERT OR REPLACE INTO {} ({}) VALUES ({});\n",
                table,
                columns_str,
                values.join(", ")
            ));
        }

        Ok(sql)
    }

    /// Get a value from a row and format it for SQL
    fn get_row_value(&self, row: &sqlx::sqlite::SqliteRow, column: &str) -> String {
        use sqlx::Row;

        // Try different types
        if let Ok(val) = row.try_get::<i64, _>(column) {
            return val.to_string();
        }
        if let Ok(val) = row.try_get::<f64, _>(column) {
            return val.to_string();
        }
        if let Ok(val) = row.try_get::<String, _>(column) {
            // Escape single quotes
            return format!("'{}'", val.replace('\'', "''"));
        }
        if let Ok(val) = row.try_get::<Vec<u8>, _>(column) {
            // Hex encode binary data
            return format!("X'{}'", hex::encode(val));
        }
        if let Ok(val) = row.try_get::<bool, _>(column) {
            return if val { "1".to_string() } else { "0".to_string() };
        }

        // NULL
        "NULL".to_string()
    }

    /// Extract table name from an INSERT statement
    fn extract_table_name(&self, statement: &str) -> Option<String> {
        let upper = statement.to_uppercase();

        if !upper.starts_with("INSERT") {
            return None;
        }

        // Pattern: INSERT [OR REPLACE] INTO table_name
        let parts: Vec<&str> = statement.split_whitespace().collect();

        for (i, part) in parts.iter().enumerate() {
            if part.to_uppercase() == "INTO" {
                if i + 1 < parts.len() {
                    // Remove any parentheses
                    let table = parts[i + 1].trim_start_matches('(').trim_end_matches(')');
                    return Some(table.to_string());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_table_groups() {
        assert!(TableGroups::get("llm").is_some());
        assert!(TableGroups::get("budgets").is_some());
        assert!(TableGroups::get("invalid").is_none());

        let llm_tables = TableGroups::get("llm").unwrap();
        assert!(llm_tables.contains(&"llm_providers"));
    }

    #[test]
    fn test_resolve_table_names() {
        let service = BackupService::new(None);

        // Test group resolution
        let tables = service.resolve_table_names(&["llm".to_string()]).unwrap();
        assert!(tables.contains("llm_providers"));

        // Test explicit table name
        let tables = service.resolve_table_names(&["my_table".to_string()]).unwrap();
        assert!(tables.contains("my_table"));

        // Test "all"
        let tables = service.resolve_table_names(&["all".to_string()]).unwrap();
        assert!(tables.len() > 5);
    }

    #[test]
    fn test_extract_table_name() {
        let service = BackupService::new(None);

        assert_eq!(
            service.extract_table_name("INSERT INTO users VALUES (1, 'test')"),
            Some("users".to_string())
        );
        assert_eq!(
            service.extract_table_name("INSERT OR REPLACE INTO budgets (id) VALUES (1)"),
            Some("budgets".to_string())
        );
        assert_eq!(service.extract_table_name("SELECT * FROM users"), None);
    }

    #[test]
    fn test_backup_dir_creation() {
        let temp = TempDir::new().unwrap();
        let backup_dir = temp.path().join("backups");

        let service = BackupService::new(Some(backup_dir.clone()));
        service.ensure_backup_dir().unwrap();

        assert!(backup_dir.exists());
    }

    #[test]
    fn test_list_backups_empty() {
        let temp = TempDir::new().unwrap();
        let service = BackupService::new(Some(temp.path().to_path_buf()));

        let backups = service.list_backups().unwrap();
        assert!(backups.is_empty());
    }
}
