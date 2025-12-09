//! AST Service
//!
//! Provides core AST operations: build, update, refine, purge, and search.
//! This service manages the lifecycle of AST data in the database.

use super::models::{ParsedAst, ProjectContext, RefinedAst};
use super::parser_trait::ParseError;
use super::registry::LanguageParserRegistry;
use super::search::{AstMatch, AstQuery, AstSearchService};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use walkdir::WalkDir;

/// Errors from AST service operations
#[derive(Debug, Error)]
pub enum AstServiceError {
    #[error("Parse error: {0}")]
    ParseError(#[from] ParseError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("No parser found for file: {0}")]
    NoParserForFile(String),

    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),
}

/// Result type for AST service operations
pub type AstServiceResult<T> = Result<T, AstServiceError>;

/// Statistics from an AST operation
#[derive(Debug, Clone, Default)]
pub struct AstOperationStats {
    /// Number of files processed
    pub files_processed: usize,
    /// Number of files skipped (no parser available)
    pub files_skipped: usize,
    /// Number of files with errors
    pub files_with_errors: usize,
    /// Number of symbols extracted
    pub symbols_extracted: usize,
    /// Number of imports extracted
    pub imports_extracted: usize,
    /// Number of files refined
    pub files_refined: usize,
    /// Number of files purged
    pub files_purged: usize,
    /// Error messages (file path -> error)
    pub errors: HashMap<String, String>,
}

/// Configuration for AST service
#[derive(Debug, Clone)]
pub struct AstServiceConfig {
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Maximum file size to parse (in bytes)
    pub max_file_size: usize,
    /// Whether to follow symlinks
    pub follow_symlinks: bool,
    /// Maximum depth to traverse
    pub max_depth: Option<usize>,
}

impl Default for AstServiceConfig {
    fn default() -> Self {
        Self {
            exclude_patterns: vec![
                "target".to_string(),
                "node_modules".to_string(),
                ".git".to_string(),
                "__pycache__".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
                "build".to_string(),
                "dist".to_string(),
            ],
            max_file_size: 1_000_000, // 1MB
            follow_symlinks: false,
            max_depth: Some(50),
        }
    }
}

/// AST Service for managing code analysis
pub struct AstService {
    /// Parser registry
    registry: Arc<LanguageParserRegistry>,
    /// Search service
    search_service: AstSearchService,
    /// Service configuration
    config: AstServiceConfig,
    /// In-memory cache of parsed ASTs (keyed by file path)
    cache: HashMap<String, ParsedAst>,
    /// Refined ASTs cache
    refined_cache: HashMap<String, RefinedAst>,
}

impl AstService {
    /// Create a new AST service with default configuration
    pub fn new() -> Self {
        Self {
            registry: Arc::new(LanguageParserRegistry::with_defaults()),
            search_service: AstSearchService::new(),
            config: AstServiceConfig::default(),
            cache: HashMap::new(),
            refined_cache: HashMap::new(),
        }
    }

    /// Create a new AST service with custom configuration
    pub fn with_config(config: AstServiceConfig) -> Self {
        Self {
            registry: Arc::new(LanguageParserRegistry::with_defaults()),
            search_service: AstSearchService::new(),
            config,
            cache: HashMap::new(),
            refined_cache: HashMap::new(),
        }
    }

    /// Build initial AST for a project directory
    ///
    /// This scans the directory and parses all supported files.
    pub async fn build(&mut self, project_path: &Path) -> AstServiceResult<AstOperationStats> {
        if !project_path.exists() {
            return Err(AstServiceError::DirectoryNotFound(
                project_path.display().to_string(),
            ));
        }

        let mut stats = AstOperationStats::default();

        // Clear existing cache for fresh build
        self.cache.clear();
        self.refined_cache.clear();

        // Walk the directory
        let files = self.collect_files(project_path)?;

        for file_path in files {
            match self.parse_file(&file_path) {
                Ok(ast) => {
                    stats.symbols_extracted += ast.symbols.len();
                    stats.imports_extracted += ast.imports.len();
                    stats.files_processed += 1;

                    // Store in cache
                    self.cache.insert(file_path.clone(), ast);
                }
                Err(AstServiceError::NoParserForFile(_)) => {
                    stats.files_skipped += 1;
                }
                Err(e) => {
                    stats.files_with_errors += 1;
                    stats.errors.insert(file_path, e.to_string());
                }
            }
        }

        // TODO: Persist to database
        // self.persist_to_database().await?;

        Ok(stats)
    }

    /// Update AST for files that have changed
    ///
    /// This compares content hashes and only re-parses modified files.
    pub async fn update(&mut self, project_path: &Path) -> AstServiceResult<AstOperationStats> {
        if !project_path.exists() {
            return Err(AstServiceError::DirectoryNotFound(
                project_path.display().to_string(),
            ));
        }

        let mut stats = AstOperationStats::default();

        // Collect current files
        let files = self.collect_files(project_path)?;

        for file_path in files {
            // Check if we have a cached version
            let needs_update = if let Some(cached) = self.cache.get(&file_path) {
                // Check if content has changed
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => {
                        let current_hash = Self::calculate_hash(&content);
                        current_hash != cached.content_hash
                    }
                    Err(_) => true, // Re-parse on read error
                }
            } else {
                true // New file, needs parsing
            };

            if needs_update {
                match self.parse_file(&file_path) {
                    Ok(ast) => {
                        stats.symbols_extracted += ast.symbols.len();
                        stats.imports_extracted += ast.imports.len();
                        stats.files_processed += 1;

                        // Update cache
                        self.cache.insert(file_path.clone(), ast);

                        // Remove refined data for this file (needs re-refinement)
                        self.refined_cache.remove(&file_path);
                    }
                    Err(AstServiceError::NoParserForFile(_)) => {
                        stats.files_skipped += 1;
                    }
                    Err(e) => {
                        stats.files_with_errors += 1;
                        stats.errors.insert(file_path, e.to_string());
                    }
                }
            } else {
                stats.files_skipped += 1;
            }
        }

        // TODO: Persist updates to database
        // self.persist_updates_to_database(&stats).await?;

        Ok(stats)
    }

    /// Refine AST for a specific directory
    ///
    /// This performs deeper semantic analysis including:
    /// - Call graph extraction
    /// - Type information
    /// - Cross-references
    pub async fn refine(&mut self, directory: &Path) -> AstServiceResult<AstOperationStats> {
        if !directory.exists() {
            return Err(AstServiceError::DirectoryNotFound(
                directory.display().to_string(),
            ));
        }

        let mut stats = AstOperationStats::default();

        // Build project context from cached ASTs
        let context = self.build_project_context();

        // Find all cached ASTs in the target directory
        let dir_str = directory.to_string_lossy().to_string();
        let files_to_refine: Vec<String> = self
            .cache
            .keys()
            .filter(|path| path.starts_with(&dir_str))
            .cloned()
            .collect();

        for file_path in files_to_refine {
            if let Some(ast) = self.cache.get(&file_path) {
                // Get the appropriate parser
                if let Some(parser) = self.registry.get_parser_for_file(&file_path) {
                    match parser.refine(ast, &context) {
                        Ok(refined) => {
                            stats.files_refined += 1;
                            stats.files_processed += 1;
                            self.refined_cache.insert(file_path, refined);
                        }
                        Err(e) => {
                            stats.files_with_errors += 1;
                            stats
                                .errors
                                .insert(file_path.clone(), format!("Refinement failed: {}", e));
                        }
                    }
                }
            }
        }

        // TODO: Persist refined data to database
        // self.persist_refined_to_database(&stats).await?;

        Ok(stats)
    }

    /// Purge refined AST data for a directory
    ///
    /// This removes the deeper semantic analysis data but keeps the base AST.
    pub async fn purge(&mut self, directory: &Path) -> AstServiceResult<AstOperationStats> {
        let mut stats = AstOperationStats::default();

        let dir_str = directory.to_string_lossy().to_string();

        // Find and remove refined data for files in the directory
        let files_to_purge: Vec<String> = self
            .refined_cache
            .keys()
            .filter(|path| path.starts_with(&dir_str))
            .cloned()
            .collect();

        for file_path in files_to_purge {
            self.refined_cache.remove(&file_path);
            stats.files_purged += 1;
        }

        // TODO: Update database to remove refined data
        // self.purge_from_database(&dir_str).await?;

        Ok(stats)
    }

    /// Search across the AST cache
    pub async fn search(&self, query: &AstQuery) -> Vec<AstMatch> {
        // For now, search in-memory cache
        // TODO: Query database when implemented

        let mut matches = Vec::new();

        for (file_path, ast) in &self.cache {
            // Apply language filter if specified
            if let Some(ref languages) = query.languages {
                if !languages.contains(&ast.language) {
                    continue;
                }
            }

            // Search symbols
            for symbol in &ast.symbols {
                let score = AstSearchService::fuzzy_score(&query.pattern, &symbol.name);

                // Check minimum score threshold
                if let Some(min_score) = query.min_score {
                    if score < min_score {
                        continue;
                    }
                }

                if score > 0.0 {
                    let ast_match = AstMatch::new(
                        file_path.clone(),
                        symbol.clone(),
                        ast.language.clone(),
                    )
                    .with_score(score);

                    // Add cross-references if available and requested
                    let ast_match = if query.include_refined {
                        if let Some(refined) = self.refined_cache.get(file_path) {
                            let refs = refined
                                .cross_refs
                                .get(&symbol.name)
                                .cloned()
                                .unwrap_or_default();
                            ast_match.with_references(refs)
                        } else {
                            ast_match
                        }
                    } else {
                        ast_match
                    };

                    matches.push(ast_match);
                }
            }
        }

        // Sort by score (highest first)
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        if let Some(limit) = query.limit {
            matches.truncate(limit);
        }

        matches
    }

    /// Get cached AST for a file
    pub fn get_ast(&self, file_path: &str) -> Option<&ParsedAst> {
        self.cache.get(file_path)
    }

    /// Get refined AST for a file
    pub fn get_refined(&self, file_path: &str) -> Option<&RefinedAst> {
        self.refined_cache.get(file_path)
    }

    /// Get statistics about the current cache
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.refined_cache.len())
    }

    /// Get list of cached file paths
    pub fn cached_files(&self) -> Vec<&str> {
        self.cache.keys().map(|s| s.as_str()).collect()
    }

    /// Get the parser registry
    pub fn registry(&self) -> &LanguageParserRegistry {
        &self.registry
    }

    // --- Private methods ---

    /// Collect all files to process in a directory
    fn collect_files(&self, root: &Path) -> AstServiceResult<Vec<String>> {
        let mut files = Vec::new();

        let walker = WalkDir::new(root)
            .follow_links(self.config.follow_symlinks)
            .max_depth(self.config.max_depth.unwrap_or(usize::MAX));

        for entry in walker.into_iter().filter_entry(|e| self.should_include(e)) {
            let entry = entry.map_err(|e| AstServiceError::IoError(e.into()))?;

            if entry.file_type().is_file() {
                let path = entry.path();

                // Check if we have a parser for this file
                if self.registry.get_parser_for_file(&path.to_string_lossy()).is_some() {
                    // Check file size
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.len() <= self.config.max_file_size as u64 {
                            files.push(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(files)
    }

    /// Check if a directory entry should be included
    fn should_include(&self, entry: &walkdir::DirEntry) -> bool {
        let name = entry.file_name().to_string_lossy();

        // Check exclusion patterns
        for pattern in &self.config.exclude_patterns {
            if name == pattern.as_str() {
                return false;
            }
        }

        true
    }

    /// Parse a single file
    fn parse_file(&self, file_path: &str) -> AstServiceResult<ParsedAst> {
        let parser = self
            .registry
            .get_parser_for_file(file_path)
            .ok_or_else(|| AstServiceError::NoParserForFile(file_path.to_string()))?;

        let content = std::fs::read_to_string(file_path)?;
        let ast = parser.parse(&content, file_path)?;

        Ok(ast)
    }

    /// Build project context from cached ASTs
    fn build_project_context(&self) -> ProjectContext {
        ProjectContext {
            parsed_files: self.cache.clone(),
            root_path: String::new(), // Would be set from project config
            type_definitions: std::collections::HashMap::new(),
        }
    }

    /// Calculate content hash
    fn calculate_hash(content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl Default for AstService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_build_empty_directory() {
        let dir = tempdir().unwrap();
        let mut service = AstService::new();

        let stats = service.build(dir.path()).await.unwrap();
        assert_eq!(stats.files_processed, 0);
    }

    #[tokio::test]
    async fn test_build_with_rust_files() {
        let dir = tempdir().unwrap();

        // Create a test Rust file
        let file_path = dir.path().join("test.rs");
        fs::write(
            &file_path,
            r#"
fn main() {
    println!("Hello, world!");
}

struct Point {
    x: f64,
    y: f64,
}
"#,
        )
        .unwrap();

        let mut service = AstService::new();
        let stats = service.build(dir.path()).await.unwrap();

        assert_eq!(stats.files_processed, 1);
        assert!(stats.symbols_extracted > 0);
    }

    #[tokio::test]
    async fn test_build_with_python_files() {
        let dir = tempdir().unwrap();

        // Create a test Python file
        let file_path = dir.path().join("test.py");
        fs::write(
            &file_path,
            r#"
def hello(name: str) -> str:
    """Say hello."""
    return f"Hello, {name}!"

class Greeter:
    def greet(self):
        pass
"#,
        )
        .unwrap();

        let mut service = AstService::new();
        let stats = service.build(dir.path()).await.unwrap();

        assert_eq!(stats.files_processed, 1);
        assert!(stats.symbols_extracted > 0);
    }

    #[tokio::test]
    async fn test_update_unchanged_files() {
        let dir = tempdir().unwrap();

        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let mut service = AstService::new();

        // Initial build
        let build_stats = service.build(dir.path()).await.unwrap();
        assert_eq!(build_stats.files_processed, 1);

        // Update without changes
        let update_stats = service.update(dir.path()).await.unwrap();
        assert_eq!(update_stats.files_processed, 0);
        assert_eq!(update_stats.files_skipped, 1);
    }

    #[tokio::test]
    async fn test_update_modified_files() {
        let dir = tempdir().unwrap();

        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let mut service = AstService::new();

        // Initial build
        service.build(dir.path()).await.unwrap();

        // Modify the file
        fs::write(&file_path, "fn main() { println!(\"modified\"); }").unwrap();

        // Update should detect the change
        let update_stats = service.update(dir.path()).await.unwrap();
        assert_eq!(update_stats.files_processed, 1);
    }

    #[tokio::test]
    async fn test_search() {
        let dir = tempdir().unwrap();

        let file_path = dir.path().join("test.rs");
        fs::write(
            &file_path,
            r#"
fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}

fn calculate_product(a: i32, b: i32) -> i32 {
    a * b
}
"#,
        )
        .unwrap();

        let mut service = AstService::new();
        service.build(dir.path()).await.unwrap();

        // Search for "calculate"
        let query = AstQuery::fuzzy("calculate");
        let matches = service.search(&query).await;

        assert!(!matches.is_empty());
        assert!(matches.iter().any(|m| m.symbol.name.contains("calculate")));
    }

    #[tokio::test]
    async fn test_purge() {
        let dir = tempdir().unwrap();

        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let mut service = AstService::new();
        service.build(dir.path()).await.unwrap();

        // Refine
        service.refine(dir.path()).await.unwrap();

        // Check we have refined data
        let (_, refined_count) = service.cache_stats();
        assert!(refined_count > 0);

        // Purge
        service.purge(dir.path()).await.unwrap();

        // Refined data should be gone
        let (base_count, refined_count) = service.cache_stats();
        assert!(base_count > 0);
        assert_eq!(refined_count, 0);
    }

    #[test]
    fn test_exclude_patterns() {
        let service = AstService::new();
        assert!(service.config.exclude_patterns.contains(&"target".to_string()));
        assert!(service.config.exclude_patterns.contains(&"node_modules".to_string()));
    }
}
