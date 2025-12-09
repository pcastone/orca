//! Language parser trait
//!
//! Defines the interface for language-specific AST parsers.
//! Implement this trait to add support for new programming languages.

use super::models::{Import, ParsedAst, ProjectContext, RefinedAst, Symbol};
use thiserror::Error;

/// Errors that can occur during parsing
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Failed to parse file: {0}")]
    ParseFailed(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Tree-sitter error: {0}")]
    TreeSitterError(String),
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Trait for language-specific AST parsing
///
/// Implement this trait to add support for a new programming language.
/// The parser is responsible for:
/// - Parsing source code into an AST
/// - Extracting symbols (functions, classes, etc.)
/// - Extracting imports/dependencies
/// - Optionally refining with deeper semantic analysis
pub trait LanguageParser: Send + Sync {
    /// Language identifier (e.g., "rust", "python")
    fn language(&self) -> &str;

    /// File extensions this parser handles (e.g., ["rs"] for Rust)
    fn extensions(&self) -> &[&str];

    /// Parse source code into a basic AST
    ///
    /// # Arguments
    /// * `content` - The source code to parse
    /// * `file_path` - Path to the file (for error messages and metadata)
    ///
    /// # Returns
    /// A ParsedAst containing the AST data and extracted information
    fn parse(&self, content: &str, file_path: &str) -> ParseResult<ParsedAst>;

    /// Extract symbols from a parsed AST
    ///
    /// Called automatically by `parse()`, but can be overridden for
    /// custom symbol extraction logic.
    fn extract_symbols(&self, ast: &ParsedAst) -> Vec<Symbol> {
        ast.symbols.clone()
    }

    /// Extract imports from a parsed AST
    ///
    /// Called automatically by `parse()`, but can be overridden for
    /// custom import extraction logic.
    fn extract_imports(&self, ast: &ParsedAst) -> Vec<Import> {
        ast.imports.clone()
    }

    /// Refine AST with deeper semantic information
    ///
    /// This method performs additional analysis to extract:
    /// - Call graphs (what functions call what)
    /// - Type information
    /// - Cross-references (where symbols are used)
    ///
    /// # Arguments
    /// * `ast` - The base parsed AST to refine
    /// * `context` - Project context with other parsed files
    ///
    /// # Returns
    /// A RefinedAst with additional semantic information
    fn refine(&self, ast: &ParsedAst, context: &ProjectContext) -> ParseResult<RefinedAst>;

    /// Check if this parser can handle a file based on its extension
    fn can_parse(&self, file_path: &str) -> bool {
        let path = std::path::Path::new(file_path);
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            self.extensions().contains(&ext)
        } else {
            false
        }
    }
}
