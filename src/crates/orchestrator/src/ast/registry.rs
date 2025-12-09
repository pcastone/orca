//! Language parser registry
//!
//! Manages registered language parsers and provides lookup by language
//! or file extension.

use super::parser_trait::LanguageParser;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for language parsers
///
/// The registry maintains a collection of language parsers and provides
/// methods to look up the appropriate parser for a given language or file.
pub struct LanguageParserRegistry {
    /// Parsers indexed by language name
    parsers: HashMap<String, Arc<dyn LanguageParser>>,
    /// Extension to language mapping for quick lookup
    extension_map: HashMap<String, String>,
}

impl LanguageParserRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
            extension_map: HashMap::new(),
        }
    }

    /// Create a registry with default parsers (Rust and Python)
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Register Rust parser
        registry.register(Arc::new(super::parsers::RustParser::new()));

        // Register Python parser
        registry.register(Arc::new(super::parsers::PythonParser::new()));

        registry
    }

    /// Register a language parser
    ///
    /// # Arguments
    /// * `parser` - The parser to register
    ///
    /// # Returns
    /// The language name that was registered
    pub fn register(&mut self, parser: Arc<dyn LanguageParser>) -> String {
        let language = parser.language().to_string();

        // Map extensions to this language
        for ext in parser.extensions() {
            self.extension_map
                .insert(ext.to_string(), language.clone());
        }

        self.parsers.insert(language.clone(), parser);
        language
    }

    /// Get a parser by language name
    pub fn get_parser(&self, language: &str) -> Option<&Arc<dyn LanguageParser>> {
        self.parsers.get(language)
    }

    /// Get a parser for a file based on its extension
    pub fn get_parser_for_file(&self, file_path: &str) -> Option<&Arc<dyn LanguageParser>> {
        let path = std::path::Path::new(file_path);
        let ext = path.extension()?.to_str()?;
        let language = self.extension_map.get(ext)?;
        self.parsers.get(language)
    }

    /// Get the language for a file extension
    pub fn get_language_for_extension(&self, ext: &str) -> Option<&str> {
        self.extension_map.get(ext).map(|s| s.as_str())
    }

    /// Get list of supported languages
    pub fn supported_languages(&self) -> Vec<&str> {
        self.parsers.keys().map(|s| s.as_str()).collect()
    }

    /// Get list of supported extensions
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.extension_map.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a language is supported
    pub fn supports_language(&self, language: &str) -> bool {
        self.parsers.contains_key(language)
    }

    /// Check if a file extension is supported
    pub fn supports_extension(&self, ext: &str) -> bool {
        self.extension_map.contains_key(ext)
    }

    /// Number of registered parsers
    pub fn parser_count(&self) -> usize {
        self.parsers.len()
    }
}

impl Default for LanguageParserRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_with_defaults() {
        let registry = LanguageParserRegistry::with_defaults();

        assert!(registry.supports_language("rust"));
        assert!(registry.supports_language("python"));
        assert!(registry.supports_extension("rs"));
        assert!(registry.supports_extension("py"));
    }

    #[test]
    fn test_get_parser_for_file() {
        let registry = LanguageParserRegistry::with_defaults();

        let rust_parser = registry.get_parser_for_file("main.rs");
        assert!(rust_parser.is_some());
        assert_eq!(rust_parser.unwrap().language(), "rust");

        let python_parser = registry.get_parser_for_file("script.py");
        assert!(python_parser.is_some());
        assert_eq!(python_parser.unwrap().language(), "python");

        let unknown = registry.get_parser_for_file("file.xyz");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_supported_languages() {
        let registry = LanguageParserRegistry::with_defaults();
        let languages = registry.supported_languages();

        assert!(languages.contains(&"rust"));
        assert!(languages.contains(&"python"));
    }
}
