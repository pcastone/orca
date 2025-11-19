//! Core types for the TOON library

pub use crate::constants::Delimiter;
use serde_json::Value as JsonValue;

pub type Depth = usize;

/// Options for encoding values to TOON format
#[derive(Debug, Clone)]
pub struct EncodeOptions {
    /// Number of spaces per indentation level (default: 2)
    pub indent: usize,
    /// Delimiter to use for tabular array rows and inline primitive arrays
    pub delimiter: Delimiter,
    /// Enable key folding to collapse single-key wrapper chains
    pub key_folding: KeyFolding,
    /// Maximum number of segments to fold when key_folding is enabled
    pub flatten_depth: usize,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            indent: 2,
            delimiter: Delimiter::default(),
            key_folding: KeyFolding::Off,
            flatten_depth: usize::MAX,
        }
    }
}

/// Key folding mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFolding {
    /// No key folding
    Off,
    /// Safe key folding - only fold valid identifiers
    Safe,
}

/// Options for decoding TOON format strings
#[derive(Debug, Clone)]
pub struct DecodeOptions {
    /// Number of spaces per indentation level (default: 2)
    pub indent: usize,
    /// When true, enforce strict validation of array lengths and tabular row counts
    pub strict: bool,
    /// Enable path expansion to reconstruct dotted keys into nested objects
    pub expand_paths: PathExpansion,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            indent: 2,
            strict: true,
            expand_paths: PathExpansion::Off,
        }
    }
}

/// Path expansion mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathExpansion {
    /// No path expansion
    Off,
    /// Safe path expansion - only expand valid identifiers
    Safe,
}

/// Information about an array header
#[derive(Debug, Clone)]
pub struct ArrayHeaderInfo {
    pub key: Option<String>,
    pub length: usize,
    pub delimiter: Delimiter,
    pub fields: Option<Vec<String>>,
}

/// A parsed line from TOON input
#[derive(Debug, Clone)]
pub struct ParsedLine {
    pub raw: String,
    pub depth: Depth,
    pub indent: usize,
    pub content: String,
    pub line_number: usize,
}

/// Information about a blank line
#[derive(Debug, Clone)]
pub struct BlankLineInfo {
    pub line_number: usize,
    pub indent: usize,
    pub depth: Depth,
}

/// Result type alias for TOON operations
pub type ToonResult<T> = Result<T, ToonError>;

/// Errors that can occur during TOON encoding/decoding
#[derive(Debug, thiserror::Error)]
pub enum ToonError {
    #[error("Syntax error at line {line}: {message}")]
    SyntaxError { line: usize, message: String },

    #[error("Range error: {0}")]
    RangeError(String),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Reference error: {0}")]
    ReferenceError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl ToonError {
    pub fn syntax(line: usize, message: impl Into<String>) -> Self {
        ToonError::SyntaxError {
            line,
            message: message.into(),
        }
    }

    pub fn syntax_no_line(message: impl Into<String>) -> Self {
        ToonError::SyntaxError {
            line: 0,
            message: message.into(),
        }
    }
}

/// Type alias for JSON primitive values
pub type JsonPrimitive = serde_json::Value;

/// Helper functions for working with JSON values
pub fn is_json_primitive(value: &JsonValue) -> bool {
    matches!(
        value,
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_)
    )
}

pub fn is_json_array(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Array(_))
}

pub fn is_json_object(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Object(_))
}
