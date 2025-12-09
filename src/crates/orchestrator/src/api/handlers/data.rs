//! Data management endpoint handlers (backup, restore, export, import)

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::api::error::{ApiError, ApiResult};

/// Backup request
#[derive(Debug, Deserialize)]
pub struct BackupRequest {
    /// Include project database in backup
    #[serde(default = "default_true")]
    pub include_project: bool,
    /// Override backup directory
    pub backup_dir: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Backup response
#[derive(Debug, Serialize)]
pub struct BackupResponse {
    /// Path to the backup file
    pub path: String,
    /// Timestamp of the backup
    pub timestamp: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Whether user database is included
    pub includes_user_db: bool,
    /// Whether project database is included
    pub includes_project_db: bool,
}

/// Restore request
#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    /// Path to the backup file to restore from
    pub backup_file: String,
}

/// Restore response
#[derive(Debug, Serialize)]
pub struct RestoreResponse {
    /// Success message
    pub message: String,
    /// Whether restore was successful
    pub success: bool,
}

/// Export request
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    /// Tables to export (or "all")
    pub tables: Vec<String>,
}

/// Import result
#[derive(Debug, Serialize)]
pub struct ImportResponse {
    /// Number of records inserted
    pub records_inserted: usize,
    /// Number of records updated
    pub records_updated: usize,
    /// Number of records skipped
    pub records_skipped: usize,
    /// Tables that were imported
    pub tables_imported: Vec<String>,
}

/// Backup info for listing
#[derive(Debug, Serialize)]
pub struct BackupInfo {
    /// Filename
    pub filename: String,
    /// Full path
    pub path: String,
    /// Timestamp
    pub timestamp: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Backup type
    pub backup_type: String,
}

/// Create a backup of databases
///
/// POST /api/v1/data/backup
pub async fn backup(
    State(app_state): State<crate::api::routes::AppState>,
    Json(req): Json<BackupRequest>,
) -> ApiResult<impl IntoResponse> {
    use orca::services::BackupService;
    use orca::db::manager::DatabaseManager;

    // Load orca config to get backup directory
    let config = orca::load_config()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load config: {}", e)))?;

    let backup_dir = req.backup_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| config.backup_dir());

    let backup_service = BackupService::new(Some(backup_dir));

    // Create database manager
    let db_manager = DatabaseManager::new(".")
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    let info = backup_service
        .backup(&db_manager, req.include_project)
        .await
        .map_err(|e| ApiError::InternalError(format!("Backup failed: {}", e)))?;

    tracing::info!("Created backup: {}", info.path.display());

    Ok((
        StatusCode::OK,
        Json(BackupResponse {
            path: info.path.display().to_string(),
            timestamp: info.timestamp.to_rfc3339(),
            size_bytes: info.size_bytes,
            includes_user_db: info.includes_user_db,
            includes_project_db: info.includes_project_db,
        }),
    ))
}

/// List available backups
///
/// GET /api/v1/data/backups
pub async fn list_backups(
    State(_app_state): State<crate::api::routes::AppState>,
) -> ApiResult<impl IntoResponse> {
    use orca::services::BackupService;

    // Load orca config to get backup directory
    let config = orca::load_config()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load config: {}", e)))?;

    let backup_service = BackupService::new(Some(config.backup_dir()));

    let backups = backup_service
        .list_backups()
        .map_err(|e| ApiError::InternalError(format!("Failed to list backups: {}", e)))?;

    let response: Vec<BackupInfo> = backups
        .into_iter()
        .map(|b| BackupInfo {
            filename: b.path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            path: b.path.display().to_string(),
            timestamp: b.timestamp.to_rfc3339(),
            size_bytes: b.size_bytes,
            backup_type: if b.includes_project_db {
                "full".to_string()
            } else {
                "user".to_string()
            },
        })
        .collect();

    Ok((StatusCode::OK, Json(response)))
}

/// Restore from a backup
///
/// POST /api/v1/data/restore
pub async fn restore(
    State(_app_state): State<crate::api::routes::AppState>,
    Json(req): Json<RestoreRequest>,
) -> ApiResult<impl IntoResponse> {
    use orca::services::BackupService;
    use orca::db::manager::DatabaseManager;

    // Load orca config to get backup directory
    let config = orca::load_config()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load config: {}", e)))?;

    let backup_service = BackupService::new(Some(config.backup_dir()));

    let backup_path = PathBuf::from(&req.backup_file);

    // Create database manager
    let db_manager = DatabaseManager::new(".")
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    backup_service
        .restore(&backup_path, &db_manager)
        .await
        .map_err(|e| ApiError::InternalError(format!("Restore failed: {}", e)))?;

    tracing::info!("Restored from backup: {}", backup_path.display());

    Ok((
        StatusCode::OK,
        Json(RestoreResponse {
            message: format!("Successfully restored from {}", backup_path.display()),
            success: true,
        }),
    ))
}

/// Export tables as SQL dump
///
/// POST /api/v1/data/export
pub async fn export(
    State(_app_state): State<crate::api::routes::AppState>,
    Json(req): Json<ExportRequest>,
) -> ApiResult<impl IntoResponse> {
    use orca::services::BackupService;
    use orca::db::manager::DatabaseManager;
    use chrono::Utc;

    // Load orca config
    let config = orca::load_config()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load config: {}", e)))?;

    let backup_service = BackupService::new(Some(config.backup_dir()));

    // Create database manager
    let db_manager = DatabaseManager::new(".")
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    // Generate output path
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let output_path = std::env::current_dir()
        .unwrap_or_default()
        .join(format!("export_{}.sql", timestamp));

    backup_service
        .export(&db_manager, &req.tables, &output_path)
        .await
        .map_err(|e| ApiError::InternalError(format!("Export failed: {}", e)))?;

    // Read the exported content
    let content = std::fs::read_to_string(&output_path)
        .map_err(|e| ApiError::InternalError(format!("Failed to read export: {}", e)))?;

    tracing::info!("Exported to: {}", output_path.display());

    // Return the SQL content directly
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        content,
    ))
}

/// Import SQL dump
///
/// POST /api/v1/data/import
pub async fn import(
    State(_app_state): State<crate::api::routes::AppState>,
    body: String,
) -> ApiResult<impl IntoResponse> {
    use orca::services::BackupService;
    use orca::db::manager::DatabaseManager;
    use chrono::Utc;

    // Load orca config
    let config = orca::load_config()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load config: {}", e)))?;

    let backup_service = BackupService::new(Some(config.backup_dir()));

    // Create database manager
    let db_manager = DatabaseManager::new(".")
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    // Write content to temporary file
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let temp_path = std::env::temp_dir().join(format!("import_{}.sql", timestamp));

    std::fs::write(&temp_path, &body)
        .map_err(|e| ApiError::InternalError(format!("Failed to write temp file: {}", e)))?;

    // Import
    let result = backup_service
        .import(&db_manager, &temp_path, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Import failed: {}", e)))?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    tracing::info!(
        "Imported {} records ({} skipped)",
        result.records_inserted + result.records_updated,
        result.records_skipped
    );

    Ok((
        StatusCode::OK,
        Json(ImportResponse {
            records_inserted: result.records_inserted,
            records_updated: result.records_updated,
            records_skipped: result.records_skipped,
            tables_imported: result.tables_imported,
        }),
    ))
}
