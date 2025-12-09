//! AST (Abstract Syntax Tree) module for semantic code indexing
//!
//! This module provides language-agnostic AST parsing and indexing capabilities
//! using tree-sitter. It supports a plugin-based architecture for adding
//! new language parsers.

pub mod models;
pub mod parser_trait;
pub mod parsers;
pub mod registry;
pub mod search;
pub mod service;

pub use models::{Import, ParsedAst, ProjectContext, RefinedAst, Symbol, SymbolKind};
pub use parser_trait::LanguageParser;
pub use registry::LanguageParserRegistry;
pub use search::{AstMatch, AstQuery, AstSearchService, SearchMode};
pub use service::{AstService, AstServiceConfig, AstServiceError, AstOperationStats};
