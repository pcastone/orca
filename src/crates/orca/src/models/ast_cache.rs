//! AST cache model

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Abstract Syntax Tree cache entry
///
/// Stores parsed ASTs for code files to avoid repeated parsing
/// Stored in project database (<project>/.orca/project.db)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AstCache {
    /// Unique cache entry identifier (UUID string)
    pub id: String,

    /// File path (unique)
    pub file_path: String,

    /// Programming language (rust, python, javascript, typescript, etc.)
    pub language: String,

    /// SHA-256 hash of file content
    pub content_hash: String,

    /// Serialized AST data (JSON)
    pub ast_data: String,

    /// Extracted symbols (JSON array of functions, classes, etc.)
    pub symbols: Option<String>,

    /// Extracted imports (JSON array)
    pub imports: Option<String>,

    /// File size in bytes
    pub file_size: Option<i64>,

    /// Time taken to parse (milliseconds)
    pub parse_duration_ms: Option<i64>,

    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,

    /// Last access timestamp (Unix timestamp)
    pub accessed_at: i64,
}

impl AstCache {
    /// Create a new AST cache entry
    pub fn new(file_path: String, language: String, content_hash: String, ast_data: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            file_path,
            language,
            content_hash,
            ast_data,
            symbols: None,
            imports: None,
            file_size: None,
            parse_duration_ms: None,
            created_at: now,
            updated_at: now,
            accessed_at: now,
        }
    }

    /// Builder: Set symbols
    pub fn with_symbols(mut self, symbols: String) -> Self {
        self.symbols = Some(symbols);
        self
    }

    /// Builder: Set imports
    pub fn with_imports(mut self, imports: String) -> Self {
        self.imports = Some(imports);
        self
    }

    /// Builder: Set file size
    pub fn with_file_size(mut self, size: i64) -> Self {
        self.file_size = Some(size);
        self
    }

    /// Builder: Set parse duration
    pub fn with_parse_duration(mut self, duration_ms: i64) -> Self {
        self.parse_duration_ms = Some(duration_ms);
        self
    }

    /// Update access timestamp
    pub fn touch(&mut self) {
        self.accessed_at = Utc::now().timestamp();
    }

    /// Check if cache entry is stale based on content hash
    pub fn is_stale(&self, current_hash: &str) -> bool {
        self.content_hash != current_hash
    }
}
