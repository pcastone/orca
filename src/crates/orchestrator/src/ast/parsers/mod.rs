//! Language-specific parsers
//!
//! This module contains tree-sitter based parsers for different programming languages.

pub mod python_parser;
pub mod rust_parser;

pub use python_parser::PythonParser;
pub use rust_parser::RustParser;
