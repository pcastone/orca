//! YAML file tracking model
//!
//! Tracks YAML configuration files for change detection and database sync.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// YAML file type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YamlFileType {
    Workflow,
    Template,
    Prompt,
    Pattern,
    Tool,
}

impl YamlFileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::Template => "template",
            Self::Prompt => "prompt",
            Self::Pattern => "pattern",
            Self::Tool => "tool",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "workflow" => Some(Self::Workflow),
            "template" => Some(Self::Template),
            "prompt" => Some(Self::Prompt),
            "pattern" => Some(Self::Pattern),
            "tool" => Some(Self::Tool),
            _ => None,
        }
    }

    /// Get the target database table for this file type
    pub fn target_table(&self) -> &'static str {
        match self {
            Self::Workflow => "workflow_templates",
            Self::Pattern => "pattern_configs",
            Self::Prompt => "prompts",
            Self::Template => "workflow_templates",
            Self::Tool => "tools",
        }
    }
}

/// Sync status for YAML files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Synced,
    Pending,
    Error,
}

impl SyncStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Synced => "synced",
            Self::Pending => "pending",
            Self::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "synced" => Some(Self::Synced),
            "pending" => Some(Self::Pending),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// YAML file tracking entry
///
/// Stored in user database (~/.orca/user.db)
/// Tracks file checksums to detect changes and sync status
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct YamlFile {
    /// Unique identifier (UUID string)
    pub id: String,

    /// Full file path
    pub file_path: String,

    /// File type (workflow, template, prompt, pattern, tool)
    pub file_type: String,

    /// SHA-256 hash of file content
    pub content_hash: String,

    /// Target database table (workflow_templates, pattern_configs, prompts)
    pub target_table: String,

    /// ID of record in target table (after sync)
    pub target_id: Option<String>,

    /// File size in bytes
    pub file_size: Option<i64>,

    /// Last sync timestamp (Unix timestamp)
    pub last_synced_at: i64,

    /// Sync status (synced, pending, error)
    pub sync_status: String,

    /// Error message if sync failed
    pub sync_error: Option<String>,

    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,
}

impl YamlFile {
    /// Create a new YAML file tracking entry
    pub fn new(
        file_path: String,
        file_type: YamlFileType,
        content_hash: String,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            file_path,
            file_type: file_type.as_str().to_string(),
            content_hash,
            target_table: file_type.target_table().to_string(),
            target_id: None,
            file_size: None,
            last_synced_at: now,
            sync_status: SyncStatus::Pending.as_str().to_string(),
            sync_error: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Builder: Set file size
    pub fn with_file_size(mut self, size: i64) -> Self {
        self.file_size = Some(size);
        self
    }

    /// Builder: Set target ID
    pub fn with_target_id(mut self, target_id: String) -> Self {
        self.target_id = Some(target_id);
        self
    }

    /// Check if file is stale based on content hash
    pub fn is_stale(&self, current_hash: &str) -> bool {
        self.content_hash != current_hash
    }

    /// Mark file as successfully synced
    pub fn mark_synced(&mut self, target_id: String) {
        let now = Utc::now().timestamp();
        self.target_id = Some(target_id);
        self.sync_status = SyncStatus::Synced.as_str().to_string();
        self.sync_error = None;
        self.last_synced_at = now;
        self.updated_at = now;
    }

    /// Mark file sync as error
    pub fn mark_error(&mut self, error: String) {
        let now = Utc::now().timestamp();
        self.sync_status = SyncStatus::Error.as_str().to_string();
        self.sync_error = Some(error);
        self.updated_at = now;
    }

    /// Mark file as pending sync
    pub fn mark_pending(&mut self) {
        let now = Utc::now().timestamp();
        self.sync_status = SyncStatus::Pending.as_str().to_string();
        self.updated_at = now;
    }

    /// Update content hash (when file changes)
    pub fn update_hash(&mut self, new_hash: String) {
        let now = Utc::now().timestamp();
        self.content_hash = new_hash;
        self.sync_status = SyncStatus::Pending.as_str().to_string();
        self.updated_at = now;
    }

    /// Get file type enum
    pub fn get_file_type(&self) -> Option<YamlFileType> {
        YamlFileType::from_str(&self.file_type)
    }

    /// Get sync status enum
    pub fn get_sync_status(&self) -> Option<SyncStatus> {
        SyncStatus::from_str(&self.sync_status)
    }

    /// Check if file is in error state
    pub fn is_error(&self) -> bool {
        self.sync_status == SyncStatus::Error.as_str()
    }

    /// Check if file is synced
    pub fn is_synced(&self) -> bool {
        self.sync_status == SyncStatus::Synced.as_str()
    }
}
