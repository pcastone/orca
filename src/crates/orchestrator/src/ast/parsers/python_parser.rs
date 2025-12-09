//! Python language parser using tree-sitter
//!
//! Parses Python source code and extracts symbols, imports, and semantic information.

use crate::ast::models::{
    CrossReference, FunctionCall, Import, Parameter, ParsedAst, ProjectContext, ReferenceKind,
    RefinedAst, Symbol, SymbolKind,
};
use crate::ast::parser_trait::{LanguageParser, ParseError, ParseResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

/// Python language parser using tree-sitter-python
pub struct PythonParser {
    parser: std::sync::Mutex<tree_sitter::Parser>,
}

impl PythonParser {
    /// Create a new Python parser
    pub fn new() -> Self {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .expect("Failed to load Python grammar");

        Self {
            parser: std::sync::Mutex::new(parser),
        }
    }

    /// Extract symbols from tree-sitter tree
    fn extract_symbols_from_tree(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let root = tree.root_node();
        self.walk_node(&root, source, &mut symbols, None);
        symbols
    }

    /// Recursively walk tree nodes to extract symbols
    fn walk_node(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>,
        parent: Option<&str>,
    ) {
        let kind = node.kind();

        match kind {
            "function_definition" => {
                if let Some(symbol) = self.extract_function(node, source, parent) {
                    symbols.push(symbol);
                }
            }
            "class_definition" => {
                if let Some(symbol) = self.extract_class(node, source, parent) {
                    let name = symbol.name.clone();
                    symbols.push(symbol);
                    // Walk class body for methods
                    if let Some(body) = node.child_by_field_name("body") {
                        for child in body.children(&mut body.walk()) {
                            self.walk_node(&child, source, symbols, Some(&name));
                        }
                    }
                    return;
                }
            }
            "decorated_definition" => {
                // Handle decorated functions/classes
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "function_definition" || child.kind() == "class_definition" {
                        self.walk_node(&child, source, symbols, parent);
                    }
                }
                return;
            }
            _ => {}
        }

        // Walk children for other nodes
        if kind != "class_definition" {
            for child in node.children(&mut node.walk()) {
                self.walk_node(&child, source, symbols, parent);
            }
        }
    }

    fn extract_function(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        parent: Option<&str>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.node_text(&name_node, source);

        let return_type = self.get_return_type(node, source);
        let parameters = self.get_parameters(node, source);
        let documentation = self.get_documentation(node, source);

        Some(Symbol {
            name,
            kind: if parent.is_some() {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            },
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            parent: parent.map(|s| s.to_string()),
            documentation,
            visibility: Some("public".to_string()), // Python has no visibility modifiers
            return_type,
            parameters,
        })
    }

    fn extract_class(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        parent: Option<&str>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.node_text(&name_node, source);

        Some(Symbol {
            name,
            kind: SymbolKind::Class,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            parent: parent.map(|s| s.to_string()),
            documentation: self.get_documentation(node, source),
            visibility: Some("public".to_string()),
            return_type: None,
            parameters: Vec::new(),
        })
    }

    fn get_return_type(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        node.child_by_field_name("return_type")
            .map(|n| self.node_text(&n, source))
    }

    fn get_parameters(&self, node: &tree_sitter::Node, source: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            for child in params_node.children(&mut params_node.walk()) {
                match child.kind() {
                    "identifier" => {
                        params.push(Parameter {
                            name: self.node_text(&child, source),
                            type_annotation: None,
                            default_value: None,
                        });
                    }
                    "typed_parameter" => {
                        let name = child
                            .child_by_field_name("name")
                            .map(|n| self.node_text(&n, source))
                            .unwrap_or_default();
                        let type_ann = child
                            .child_by_field_name("type")
                            .map(|n| self.node_text(&n, source));
                        params.push(Parameter {
                            name,
                            type_annotation: type_ann,
                            default_value: None,
                        });
                    }
                    "default_parameter" => {
                        let name = child
                            .child_by_field_name("name")
                            .map(|n| self.node_text(&n, source))
                            .unwrap_or_default();
                        let default = child
                            .child_by_field_name("value")
                            .map(|n| self.node_text(&n, source));
                        params.push(Parameter {
                            name,
                            type_annotation: None,
                            default_value: default,
                        });
                    }
                    "typed_default_parameter" => {
                        let name = child
                            .child_by_field_name("name")
                            .map(|n| self.node_text(&n, source))
                            .unwrap_or_default();
                        let type_ann = child
                            .child_by_field_name("type")
                            .map(|n| self.node_text(&n, source));
                        let default = child
                            .child_by_field_name("value")
                            .map(|n| self.node_text(&n, source));
                        params.push(Parameter {
                            name,
                            type_annotation: type_ann,
                            default_value: default,
                        });
                    }
                    _ => {}
                }
            }
        }

        params
    }

    fn get_documentation(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        // Look for docstring in function/class body
        if let Some(body) = node.child_by_field_name("body") {
            if let Some(first_stmt) = body.named_child(0) {
                if first_stmt.kind() == "expression_statement" {
                    if let Some(string_node) = first_stmt.named_child(0) {
                        if string_node.kind() == "string" {
                            let text = self.node_text(&string_node, source);
                            // Remove quotes
                            let trimmed = text
                                .trim_start_matches("\"\"\"")
                                .trim_start_matches("'''")
                                .trim_start_matches('"')
                                .trim_start_matches('\'')
                                .trim_end_matches("\"\"\"")
                                .trim_end_matches("'''")
                                .trim_end_matches('"')
                                .trim_end_matches('\'')
                                .trim();
                            return Some(trimmed.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn node_text(&self, node: &tree_sitter::Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }

    /// Extract imports from tree
    fn extract_imports_from_tree(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Import> {
        let mut imports = Vec::new();
        let root = tree.root_node();
        self.walk_for_imports(&root, source, &mut imports);
        imports
    }

    fn walk_for_imports(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        imports: &mut Vec<Import>,
    ) {
        match node.kind() {
            "import_statement" => {
                if let Some(import) = self.extract_import_statement(node, source) {
                    imports.push(import);
                }
            }
            "import_from_statement" => {
                if let Some(import) = self.extract_import_from_statement(node, source) {
                    imports.push(import);
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.walk_for_imports(&child, source, imports);
        }
    }

    fn extract_import_statement(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Import> {
        // import foo, bar
        let mut names = Vec::new();
        for child in node.children(&mut node.walk()) {
            if child.kind() == "dotted_name" {
                names.push(self.node_text(&child, source));
            } else if child.kind() == "aliased_import" {
                if let Some(name) = child.child_by_field_name("name") {
                    names.push(self.node_text(&name, source));
                }
            }
        }

        if names.is_empty() {
            return None;
        }

        Some(Import {
            path: names.join(", "),
            names: names.clone(),
            alias: None,
            line: node.start_position().row + 1,
            is_relative: false,
        })
    }

    fn extract_import_from_statement(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Import> {
        // from foo import bar, baz
        let module = node
            .child_by_field_name("module_name")
            .map(|n| self.node_text(&n, source))
            .unwrap_or_default();

        let mut names = Vec::new();
        for child in node.children(&mut node.walk()) {
            if child.kind() == "dotted_name" {
                let text = self.node_text(&child, source);
                if text != module {
                    names.push(text);
                }
            } else if child.kind() == "aliased_import" {
                if let Some(name) = child.child_by_field_name("name") {
                    names.push(self.node_text(&name, source));
                }
            } else if child.kind() == "wildcard_import" {
                names.push("*".to_string());
            }
        }

        Some(Import {
            path: module.clone(),
            names,
            alias: None,
            line: node.start_position().row + 1,
            is_relative: module.starts_with('.'),
        })
    }

    /// Extract function calls for call graph
    fn extract_calls_from_tree(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
    ) -> HashMap<String, Vec<FunctionCall>> {
        let mut calls: HashMap<String, Vec<FunctionCall>> = HashMap::new();
        let root = tree.root_node();
        self.walk_for_calls(&root, source, &mut calls, None);
        calls
    }

    fn walk_for_calls(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        calls: &mut HashMap<String, Vec<FunctionCall>>,
        current_fn: Option<&str>,
    ) {
        let kind = node.kind();

        // Track current function context
        let fn_context = if kind == "function_definition" {
            node.child_by_field_name("name")
                .map(|n| self.node_text(&n, source))
        } else {
            current_fn.map(|s| s.to_string())
        };

        // Extract call expressions
        if kind == "call" {
            if let Some(fn_name) = fn_context.as_deref() {
                if let Some(callee) = node.child_by_field_name("function") {
                    let callee_name = self.node_text(&callee, source);
                    let call = FunctionCall {
                        callee: callee_name,
                        callee_file: None,
                        line: node.start_position().row + 1,
                        arguments: Vec::new(),
                    };
                    calls.entry(fn_name.to_string()).or_default().push(call);
                }
            }
        }

        for child in node.children(&mut node.walk()) {
            self.walk_for_calls(&child, source, calls, fn_context.as_deref());
        }
    }
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for PythonParser {
    fn language(&self) -> &str {
        "python"
    }

    fn extensions(&self) -> &[&str] {
        &["py", "pyi"]
    }

    fn parse(&self, content: &str, file_path: &str) -> ParseResult<ParsedAst> {
        let start = Instant::now();

        let mut parser = self.parser.lock().map_err(|e| {
            ParseError::ParseFailed(format!("Failed to lock parser: {}", e))
        })?;

        let tree = parser.parse(content, None).ok_or_else(|| {
            ParseError::ParseFailed("Tree-sitter failed to parse content".to_string())
        })?;

        let symbols = self.extract_symbols_from_tree(&tree, content);
        let imports = self.extract_imports_from_tree(&tree, content);

        // Compute content hash
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = format!("{:x}", hasher.finalize());

        let parse_duration = start.elapsed().as_millis() as u64;

        Ok(ParsedAst {
            file_path: file_path.to_string(),
            language: "python".to_string(),
            ast_json: String::new(),
            symbols,
            imports,
            content_hash,
            parse_duration_ms: parse_duration,
        })
    }

    fn refine(&self, ast: &ParsedAst, context: &ProjectContext) -> ParseResult<RefinedAst> {
        // Re-parse to get the tree for call graph extraction
        let content = std::fs::read_to_string(&ast.file_path).map_err(ParseError::IoError)?;

        let mut parser = self.parser.lock().map_err(|e| {
            ParseError::ParseFailed(format!("Failed to lock parser: {}", e))
        })?;

        let tree = parser.parse(&content, None).ok_or_else(|| {
            ParseError::ParseFailed("Tree-sitter failed to parse content".to_string())
        })?;

        let call_graph = self.extract_calls_from_tree(&tree, &content);

        // Build cross-references from project context
        let mut cross_refs: HashMap<String, Vec<CrossReference>> = HashMap::new();

        // For each symbol, search other files for references
        for symbol in &ast.symbols {
            let refs = self.find_references(&symbol.name, context);
            if !refs.is_empty() {
                cross_refs.insert(symbol.name.clone(), refs);
            }
        }

        Ok(RefinedAst {
            base: ast.clone(),
            call_graph,
            type_info: HashMap::new(),
            cross_refs,
            refinement_level: 1,
        })
    }
}

impl PythonParser {
    fn find_references(&self, symbol_name: &str, context: &ProjectContext) -> Vec<CrossReference> {
        let mut refs = Vec::new();

        for (file_path, parsed) in &context.parsed_files {
            for other_symbol in &parsed.symbols {
                if other_symbol.parent.as_ref() == Some(&symbol_name.to_string()) {
                    refs.push(CrossReference {
                        file_path: file_path.clone(),
                        line: other_symbol.line,
                        column: other_symbol.column,
                        context: format!("{} in {}", other_symbol.name, file_path),
                        reference_kind: ReferenceKind::Call,
                    });
                }
            }
        }

        refs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_python_function() {
        let parser = PythonParser::new();
        let source = r#"
def hello(name: str) -> str:
    """Say hello to someone."""
    return f"Hello, {name}!"
"#;

        let result = parser.parse(source, "test.py").unwrap();

        assert_eq!(result.language, "python");
        assert!(!result.symbols.is_empty());

        let func = &result.symbols[0];
        assert_eq!(func.name, "hello");
        assert_eq!(func.kind, SymbolKind::Function);
        assert!(func.documentation.is_some());
    }

    #[test]
    fn test_parse_python_class() {
        let parser = PythonParser::new();
        let source = r#"
class Animal:
    """A base class for animals."""

    def __init__(self, name: str):
        self.name = name

    def speak(self) -> str:
        pass
"#;

        let result = parser.parse(source, "test.py").unwrap();

        assert!(!result.symbols.is_empty());

        let class_sym = result.symbols.iter().find(|s| s.name == "Animal").unwrap();
        assert_eq!(class_sym.kind, SymbolKind::Class);

        // Methods should have parent
        let init_method = result.symbols.iter().find(|s| s.name == "__init__").unwrap();
        assert_eq!(init_method.kind, SymbolKind::Method);
        assert_eq!(init_method.parent.as_deref(), Some("Animal"));
    }

    #[test]
    fn test_parse_python_imports() {
        let parser = PythonParser::new();
        let source = r#"
import os
import sys
from typing import List, Optional
from . import local_module

def main():
    pass
"#;

        let result = parser.parse(source, "test.py").unwrap();

        assert!(!result.imports.is_empty());
    }
}
