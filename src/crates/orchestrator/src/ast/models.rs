//! AST data models
//!
//! Core data structures for representing parsed ASTs, symbols, imports,
//! and refined semantic information.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kind of symbol extracted from source code
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Constant,
    Variable,
    Module,
    Type,
    Macro,
    Other(String),
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "function"),
            SymbolKind::Method => write!(f, "method"),
            SymbolKind::Class => write!(f, "class"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::Interface => write!(f, "interface"),
            SymbolKind::Trait => write!(f, "trait"),
            SymbolKind::Constant => write!(f, "constant"),
            SymbolKind::Variable => write!(f, "variable"),
            SymbolKind::Module => write!(f, "module"),
            SymbolKind::Type => write!(f, "type"),
            SymbolKind::Macro => write!(f, "macro"),
            SymbolKind::Other(s) => write!(f, "{}", s),
        }
    }
}

/// A symbol extracted from source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name
    pub name: String,
    /// Kind of symbol
    pub kind: SymbolKind,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// End line
    pub end_line: usize,
    /// End column
    pub end_column: usize,
    /// Parent symbol (for nested symbols like methods in classes)
    pub parent: Option<String>,
    /// Documentation/docstring if present
    pub documentation: Option<String>,
    /// Visibility (pub, private, etc.)
    pub visibility: Option<String>,
    /// Return type (for functions/methods)
    pub return_type: Option<String>,
    /// Parameters (for functions/methods)
    pub parameters: Vec<Parameter>,
}

/// Function/method parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
}

/// An import statement from source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    /// Module or package path
    pub path: String,
    /// Imported names (if specific items are imported)
    pub names: Vec<String>,
    /// Alias if renamed
    pub alias: Option<String>,
    /// Line number
    pub line: usize,
    /// Whether it's a relative import
    pub is_relative: bool,
}

/// Parsed AST representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedAst {
    /// File path
    pub file_path: String,
    /// Language identifier
    pub language: String,
    /// Raw AST data as JSON
    pub ast_json: String,
    /// Extracted symbols
    pub symbols: Vec<Symbol>,
    /// Extracted imports
    pub imports: Vec<Import>,
    /// File content hash for change detection
    pub content_hash: String,
    /// Parse duration in milliseconds
    pub parse_duration_ms: u64,
}

/// Project context for refinement
#[derive(Debug, Clone, Default)]
pub struct ProjectContext {
    /// Project root path
    pub root_path: String,
    /// Other parsed files in the project (for cross-references)
    pub parsed_files: HashMap<String, ParsedAst>,
    /// Type definitions discovered across the project
    pub type_definitions: HashMap<String, TypeDefinition>,
}

/// Type definition for semantic analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub file_path: String,
    pub kind: SymbolKind,
    pub fields: Vec<TypeField>,
    pub methods: Vec<String>,
}

/// Field in a type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeField {
    pub name: String,
    pub type_annotation: Option<String>,
    pub visibility: Option<String>,
}

/// Cross-reference information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference {
    /// File where the reference occurs
    pub file_path: String,
    /// Line number
    pub line: usize,
    /// Column number
    pub column: usize,
    /// Context (surrounding code snippet)
    pub context: String,
    /// Type of reference (call, type usage, import, etc.)
    pub reference_kind: ReferenceKind,
}

/// Kind of cross-reference
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceKind {
    Call,
    TypeUsage,
    Import,
    Assignment,
    FieldAccess,
    Other(String),
}

/// Function call in call graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Name of the called function
    pub callee: String,
    /// File containing the callee (if known)
    pub callee_file: Option<String>,
    /// Line where call occurs
    pub line: usize,
    /// Arguments passed (if extractable)
    pub arguments: Vec<String>,
}

/// Refined AST with deeper semantic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinedAst {
    /// Base parsed AST
    pub base: ParsedAst,
    /// Call graph: function -> calls it makes
    pub call_graph: HashMap<String, Vec<FunctionCall>>,
    /// Type information for symbols
    pub type_info: HashMap<String, String>,
    /// Cross-references: symbol -> where it's used
    pub cross_refs: HashMap<String, Vec<CrossReference>>,
    /// Refinement level (depth of analysis)
    pub refinement_level: u32,
}

impl RefinedAst {
    /// Create a new refined AST from a parsed AST
    pub fn from_parsed(base: ParsedAst) -> Self {
        Self {
            base,
            call_graph: HashMap::new(),
            type_info: HashMap::new(),
            cross_refs: HashMap::new(),
            refinement_level: 0,
        }
    }
}
