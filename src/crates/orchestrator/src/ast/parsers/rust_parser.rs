//! Rust language parser using tree-sitter
//!
//! Parses Rust source code and extracts symbols, imports, and semantic information.

use crate::ast::models::{
    CrossReference, FunctionCall, Import, Parameter, ParsedAst, ProjectContext, RefinedAst,
    Symbol, SymbolKind,
};
use crate::ast::parser_trait::{LanguageParser, ParseError, ParseResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

/// Rust language parser using tree-sitter-rust
pub struct RustParser {
    parser: std::sync::Mutex<tree_sitter::Parser>,
}

impl RustParser {
    /// Create a new Rust parser
    pub fn new() -> Self {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("Failed to load Rust grammar");

        Self {
            parser: std::sync::Mutex::new(parser),
        }
    }

    /// Extract symbols from tree-sitter tree
    fn extract_symbols_from_tree(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
    ) -> Vec<Symbol> {
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
            "function_item" | "function_signature_item" => {
                if let Some(symbol) = self.extract_function(node, source, parent) {
                    let name = symbol.name.clone();
                    symbols.push(symbol);
                    // Walk children with this function as parent
                    for child in node.children(&mut node.walk()) {
                        self.walk_node(&child, source, symbols, Some(&name));
                    }
                    return;
                }
            }
            "struct_item" => {
                if let Some(symbol) = self.extract_struct(node, source, parent) {
                    let name = symbol.name.clone();
                    symbols.push(symbol);
                    for child in node.children(&mut node.walk()) {
                        self.walk_node(&child, source, symbols, Some(&name));
                    }
                    return;
                }
            }
            "enum_item" => {
                if let Some(symbol) = self.extract_enum(node, source, parent) {
                    symbols.push(symbol);
                }
            }
            "trait_item" => {
                if let Some(symbol) = self.extract_trait(node, source, parent) {
                    let name = symbol.name.clone();
                    symbols.push(symbol);
                    for child in node.children(&mut node.walk()) {
                        self.walk_node(&child, source, symbols, Some(&name));
                    }
                    return;
                }
            }
            "impl_item" => {
                // Extract methods from impl blocks
                let impl_name = self.get_impl_name(node, source);
                for child in node.children(&mut node.walk()) {
                    self.walk_node(&child, source, symbols, impl_name.as_deref());
                }
                return;
            }
            "const_item" | "static_item" => {
                if let Some(symbol) = self.extract_const(node, source, parent) {
                    symbols.push(symbol);
                }
            }
            "type_alias" => {
                if let Some(symbol) = self.extract_type_alias(node, source, parent) {
                    symbols.push(symbol);
                }
            }
            "macro_definition" => {
                if let Some(symbol) = self.extract_macro(node, source, parent) {
                    symbols.push(symbol);
                }
            }
            "mod_item" => {
                if let Some(symbol) = self.extract_module(node, source, parent) {
                    let name = symbol.name.clone();
                    symbols.push(symbol);
                    for child in node.children(&mut node.walk()) {
                        self.walk_node(&child, source, symbols, Some(&name));
                    }
                    return;
                }
            }
            _ => {}
        }

        // Walk children for non-symbol nodes
        for child in node.children(&mut node.walk()) {
            self.walk_node(&child, source, symbols, parent);
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

        let visibility = self.get_visibility(node, source);
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
            visibility,
            return_type,
            parameters,
        })
    }

    fn extract_struct(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        parent: Option<&str>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.node_text(&name_node, source);

        Some(Symbol {
            name,
            kind: SymbolKind::Struct,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            parent: parent.map(|s| s.to_string()),
            documentation: self.get_documentation(node, source),
            visibility: self.get_visibility(node, source),
            return_type: None,
            parameters: Vec::new(),
        })
    }

    fn extract_enum(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        parent: Option<&str>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.node_text(&name_node, source);

        Some(Symbol {
            name,
            kind: SymbolKind::Enum,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            parent: parent.map(|s| s.to_string()),
            documentation: self.get_documentation(node, source),
            visibility: self.get_visibility(node, source),
            return_type: None,
            parameters: Vec::new(),
        })
    }

    fn extract_trait(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        parent: Option<&str>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.node_text(&name_node, source);

        Some(Symbol {
            name,
            kind: SymbolKind::Trait,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            parent: parent.map(|s| s.to_string()),
            documentation: self.get_documentation(node, source),
            visibility: self.get_visibility(node, source),
            return_type: None,
            parameters: Vec::new(),
        })
    }

    fn extract_const(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        parent: Option<&str>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.node_text(&name_node, source);

        Some(Symbol {
            name,
            kind: SymbolKind::Constant,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            parent: parent.map(|s| s.to_string()),
            documentation: self.get_documentation(node, source),
            visibility: self.get_visibility(node, source),
            return_type: None,
            parameters: Vec::new(),
        })
    }

    fn extract_type_alias(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        parent: Option<&str>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.node_text(&name_node, source);

        Some(Symbol {
            name,
            kind: SymbolKind::Type,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            parent: parent.map(|s| s.to_string()),
            documentation: self.get_documentation(node, source),
            visibility: self.get_visibility(node, source),
            return_type: None,
            parameters: Vec::new(),
        })
    }

    fn extract_macro(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        parent: Option<&str>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.node_text(&name_node, source);

        Some(Symbol {
            name,
            kind: SymbolKind::Macro,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            parent: parent.map(|s| s.to_string()),
            documentation: self.get_documentation(node, source),
            visibility: self.get_visibility(node, source),
            return_type: None,
            parameters: Vec::new(),
        })
    }

    fn extract_module(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        parent: Option<&str>,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.node_text(&name_node, source);

        Some(Symbol {
            name,
            kind: SymbolKind::Module,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            parent: parent.map(|s| s.to_string()),
            documentation: self.get_documentation(node, source),
            visibility: self.get_visibility(node, source),
            return_type: None,
            parameters: Vec::new(),
        })
    }

    fn get_impl_name(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        // Try to find the type being implemented
        for child in node.children(&mut node.walk()) {
            if child.kind() == "type_identifier" || child.kind() == "generic_type" {
                return Some(self.node_text(&child, source));
            }
        }
        None
    }

    fn get_visibility(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "visibility_modifier" {
                return Some(self.node_text(&child, source));
            }
        }
        None
    }

    fn get_return_type(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        if let Some(ret_type) = node.child_by_field_name("return_type") {
            Some(self.node_text(&ret_type, source))
        } else {
            None
        }
    }

    fn get_parameters(&self, node: &tree_sitter::Node, source: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            for child in params_node.children(&mut params_node.walk()) {
                if child.kind() == "parameter" {
                    if let Some(pattern) = child.child_by_field_name("pattern") {
                        let name = self.node_text(&pattern, source);
                        let type_ann = child
                            .child_by_field_name("type")
                            .map(|t| self.node_text(&t, source));
                        params.push(Parameter {
                            name,
                            type_annotation: type_ann,
                            default_value: None,
                        });
                    }
                }
            }
        }

        params
    }

    fn get_documentation(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        // Look for doc comments before this node
        if let Some(prev) = node.prev_sibling() {
            if prev.kind() == "line_comment" || prev.kind() == "block_comment" {
                let text = self.node_text(&prev, source);
                if text.starts_with("///") || text.starts_with("//!") {
                    return Some(text.trim_start_matches("///").trim().to_string());
                }
            }
        }
        None
    }

    fn node_text(&self, node: &tree_sitter::Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }

    /// Extract imports from tree
    fn extract_imports_from_tree(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
    ) -> Vec<Import> {
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
        if node.kind() == "use_declaration" {
            if let Some(import) = self.extract_use_declaration(node, source) {
                imports.push(import);
            }
        }

        for child in node.children(&mut node.walk()) {
            self.walk_for_imports(&child, source, imports);
        }
    }

    fn extract_use_declaration(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Import> {
        // Get the use path
        let path_text = self.node_text(node, source);
        let path = path_text
            .trim_start_matches("use ")
            .trim_end_matches(';')
            .to_string();

        Some(Import {
            path,
            names: Vec::new(),
            alias: None,
            line: node.start_position().row + 1,
            is_relative: false,
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
        let fn_context = if kind == "function_item" {
            node.child_by_field_name("name")
                .map(|n| self.node_text(&n, source))
        } else {
            current_fn.map(|s| s.to_string())
        };

        // Extract call expressions
        if kind == "call_expression" {
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

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for RustParser {
    fn language(&self) -> &str {
        "rust"
    }

    fn extensions(&self) -> &[&str] {
        &["rs"]
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
            language: "rust".to_string(),
            ast_json: String::new(), // Full AST not stored by default
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
            type_info: HashMap::new(), // TODO: Type inference
            cross_refs,
            refinement_level: 1,
        })
    }
}

impl RustParser {
    fn find_references(&self, symbol_name: &str, context: &ProjectContext) -> Vec<CrossReference> {
        let mut refs = Vec::new();

        for (file_path, parsed) in &context.parsed_files {
            // Search for uses of this symbol in other files
            for other_symbol in &parsed.symbols {
                // Check if this symbol references our target
                // This is a simplified check - a real implementation would use tree-sitter
                if other_symbol.parent.as_ref() == Some(&symbol_name.to_string()) {
                    refs.push(CrossReference {
                        file_path: file_path.clone(),
                        line: other_symbol.line,
                        column: other_symbol.column,
                        context: format!("{} in {}", other_symbol.name, file_path),
                        reference_kind: crate::ast::models::ReferenceKind::Call,
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
    fn test_parse_rust_function() {
        let parser = RustParser::new();
        let source = r#"
/// A test function
pub fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#;

        let result = parser.parse(source, "test.rs").unwrap();

        assert_eq!(result.language, "rust");
        assert!(!result.symbols.is_empty());

        let func = &result.symbols[0];
        assert_eq!(func.name, "hello");
        assert_eq!(func.kind, SymbolKind::Function);
        assert!(func.visibility.as_ref().map(|v| v.contains("pub")).unwrap_or(false));
    }

    #[test]
    fn test_parse_rust_struct() {
        let parser = RustParser::new();
        let source = r#"
pub struct User {
    name: String,
    age: u32,
}
"#;

        let result = parser.parse(source, "test.rs").unwrap();

        assert!(!result.symbols.is_empty());
        let struct_sym = &result.symbols[0];
        assert_eq!(struct_sym.name, "User");
        assert_eq!(struct_sym.kind, SymbolKind::Struct);
    }

    #[test]
    fn test_parse_rust_imports() {
        let parser = RustParser::new();
        let source = r#"
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

fn main() {}
"#;

        let result = parser.parse(source, "test.rs").unwrap();

        assert!(!result.imports.is_empty());
    }
}
