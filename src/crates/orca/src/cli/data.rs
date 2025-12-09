//! Data management command handlers (backup, restore, export, import)

use crate::config::OrcaConfig;
use crate::error::Result;
use crate::services::BackupService;
use crate::DatabaseManager;
use chrono::Utc;
use colored::Colorize;
use std::path::PathBuf;
use std::sync::Arc;
use tabled::{Table, Tabled};

/// Backup display row for table output
#[derive(Tabled)]
struct BackupRow {
    #[tabled(rename = "Timestamp")]
    timestamp: String,
    #[tabled(rename = "File")]
    file: String,
    #[tabled(rename = "Size")]
    size: String,
    #[tabled(rename = "Type")]
    backup_type: String,
}

/// Handle backup command
pub async fn handle_backup(
    db_manager: Arc<DatabaseManager>,
    config: &OrcaConfig,
    dir: Option<PathBuf>,
    include_project: bool,
) -> Result<()> {
    let backup_dir = dir.or_else(|| Some(config.backup_dir()));
    let service = BackupService::new(backup_dir);

    println!("{}", "Creating backup...".cyan());

    let info = service.backup(&db_manager, include_project).await?;

    println!();
    println!("{}", "Backup created successfully!".green().bold());
    println!("  Path: {}", info.path.display());
    println!("  Size: {}", format_size(info.size_bytes));
    println!("  User DB: {}", if info.includes_user_db { "Yes" } else { "No" });
    println!("  Project DB: {}", if info.includes_project_db { "Yes" } else { "No" });
    println!("  Timestamp: {}", info.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));

    Ok(())
}

/// Handle restore command
pub async fn handle_restore(
    db_manager: Arc<DatabaseManager>,
    config: &OrcaConfig,
    file: Option<PathBuf>,
    list: bool,
) -> Result<()> {
    let service = BackupService::new(Some(config.backup_dir()));

    if list {
        // List available backups
        let backups = service.list_backups()?;

        if backups.is_empty() {
            println!("{}", "No backups found.".yellow());
            println!("Backup directory: {}", config.backup_dir().display());
            return Ok(());
        }

        println!("{}", "Available backups:".cyan().bold());
        println!();

        let rows: Vec<BackupRow> = backups
            .iter()
            .map(|b| BackupRow {
                timestamp: b.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                file: b.path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                size: format_size(b.size_bytes),
                backup_type: if b.includes_project_db {
                    "Full (user+project)".to_string()
                } else {
                    "User only".to_string()
                },
            })
            .collect();

        let table = Table::new(rows).to_string();
        println!("{}", table);
        println!();
        println!("To restore, use: {}", "orca data restore --file <backup_file>".yellow());

        return Ok(());
    }

    // Restore from file
    if let Some(backup_path) = file {
        println!("{}", format!("Restoring from {}...", backup_path.display()).cyan());
        println!("{}", "WARNING: This will overwrite existing databases!".yellow().bold());

        service.restore(&backup_path, &db_manager).await?;

        println!();
        println!("{}", "Restore completed successfully!".green().bold());
        println!("Note: You may need to restart orca for changes to take effect.");
    } else {
        // No file specified, show available backups
        let backups = service.list_backups()?;

        if backups.is_empty() {
            println!("{}", "No backups found.".yellow());
            println!("Backup directory: {}", config.backup_dir().display());
            return Ok(());
        }

        println!("{}", "No backup file specified. Available backups:".yellow());
        println!();

        for backup in backups.iter().take(5) {
            println!("  {} - {}",
                backup.timestamp.format("%Y-%m-%d %H:%M:%S"),
                backup.path.display()
            );
        }

        if backups.len() > 5 {
            println!("  ... and {} more", backups.len() - 5);
        }

        println!();
        println!("Use: {}", "orca data restore --file <backup_file>".yellow());
        println!("Or:  {}", "orca data restore --list".yellow());
    }

    Ok(())
}

/// Handle export command
pub async fn handle_export(
    db_manager: Arc<DatabaseManager>,
    config: &OrcaConfig,
    tables: String,
    output: Option<PathBuf>,
) -> Result<()> {
    let service = BackupService::new(Some(config.backup_dir()));

    // Parse tables
    let table_list: Vec<String> = tables
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        PathBuf::from(format!("export_{}.sql", timestamp))
    });

    println!("{}", format!("Exporting tables: {}...", tables).cyan());

    service.export(&db_manager, &table_list, &output_path).await?;

    println!();
    println!("{}", "Export completed successfully!".green().bold());
    println!("  Output: {}", output_path.display());
    println!("  Tables: {}", tables);

    Ok(())
}

/// Handle import command
pub async fn handle_import(
    db_manager: Arc<DatabaseManager>,
    config: &OrcaConfig,
    file: PathBuf,
    tables: String,
) -> Result<()> {
    let service = BackupService::new(Some(config.backup_dir()));

    // Parse tables
    let table_list: Vec<String> = tables
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let filter = if tables.to_lowercase() == "all" {
        None
    } else {
        Some(table_list.as_slice())
    };

    println!("{}", format!("Importing from {}...", file.display()).cyan());

    let result = service.import(&db_manager, &file, filter).await?;

    println!();
    println!("{}", "Import completed successfully!".green().bold());
    println!("  Records inserted: {}", result.records_inserted);
    println!("  Records updated: {}", result.records_updated);
    println!("  Records skipped: {}", result.records_skipped);
    if !result.tables_imported.is_empty() {
        println!("  Tables: {}", result.tables_imported.join(", "));
    }

    Ok(())
}

/// Format file size in human-readable form
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
