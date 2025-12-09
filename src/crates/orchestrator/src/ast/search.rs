//! AST search service
//!
//! Provides semantic search capabilities across parsed ASTs.

use super::models::{CrossReference, Symbol};
use serde::{Deserialize, Serialize};

/// Search mode for AST queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    /// Search for symbols by exact name
    Symbol,
    /// Fuzzy text matching across symbols
    Fuzzy,
    /// Full semantic search including types, refs, and calls
    Semantic,
}

impl Default for SearchMode {
    fn default() -> Self {
        Self::Symbol
    }
}

/// Query for searching ASTs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstQuery {
    /// Search pattern (name, regex, or fuzzy query)
    pub pattern: String,
    /// Search mode
    pub mode: SearchMode,
    /// Optional language filter
    pub languages: Option<Vec<String>>,
    /// Include refined data in results
    pub include_refined: bool,
    /// Maximum results to return
    pub limit: Option<usize>,
    /// Minimum score threshold (0.0 - 1.0)
    pub min_score: Option<f32>,
}

impl AstQuery {
    /// Create a new symbol search query
    pub fn symbol(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            mode: SearchMode::Symbol,
            languages: None,
            include_refined: false,
            limit: Some(100),
            min_score: None,
        }
    }

    /// Create a new fuzzy search query
    pub fn fuzzy(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            mode: SearchMode::Fuzzy,
            languages: None,
            include_refined: false,
            limit: Some(100),
            min_score: Some(0.5),
        }
    }

    /// Create a new semantic search query
    pub fn semantic(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            mode: SearchMode::Semantic,
            languages: None,
            include_refined: true,
            limit: Some(50),
            min_score: Some(0.3),
        }
    }

    /// Filter by languages
    pub fn with_languages(mut self, languages: Vec<String>) -> Self {
        self.languages = Some(languages);
        self
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Include refined data
    pub fn with_refined(mut self, include: bool) -> Self {
        self.include_refined = include;
        self
    }
}

/// A match from AST search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstMatch {
    /// File path containing the match
    pub file_path: String,
    /// Matched symbol
    pub symbol: Symbol,
    /// Line range (start, end)
    pub line_range: (usize, usize),
    /// Code context (surrounding lines)
    pub context: String,
    /// Relevance score (0.0 - 1.0)
    pub score: f32,
    /// Cross-references (if refined data available)
    pub references: Vec<CrossReference>,
    /// Language of the file
    pub language: String,
}

impl AstMatch {
    /// Create a new match
    pub fn new(file_path: String, symbol: Symbol, language: String) -> Self {
        let line_range = (symbol.line, symbol.end_line);
        Self {
            file_path,
            symbol,
            line_range,
            context: String::new(),
            score: 1.0,
            references: Vec::new(),
            language,
        }
    }

    /// Set the context
    pub fn with_context(mut self, context: String) -> Self {
        self.context = context;
        self
    }

    /// Set the score
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }

    /// Add references
    pub fn with_references(mut self, refs: Vec<CrossReference>) -> Self {
        self.references = refs;
        self
    }
}

/// AST search service
pub struct AstSearchService {
    // Database connection would go here
    // db: Arc<Database>,
}

impl AstSearchService {
    /// Create a new search service
    pub fn new() -> Self {
        Self {}
    }

    /// Search for symbols matching the query
    ///
    /// This is a placeholder implementation. The actual implementation
    /// will query the ast_cache database table.
    pub async fn search(&self, query: &AstQuery) -> Vec<AstMatch> {
        // TODO: Implement actual database search
        // This will:
        // 1. Query ast_cache table based on mode
        // 2. Apply language filters
        // 3. Compute relevance scores
        // 4. Return sorted matches

        match query.mode {
            SearchMode::Symbol => self.search_symbols(query).await,
            SearchMode::Fuzzy => self.search_fuzzy(query).await,
            SearchMode::Semantic => self.search_semantic(query).await,
        }
    }

    async fn search_symbols(&self, _query: &AstQuery) -> Vec<AstMatch> {
        // TODO: Exact symbol name matching
        Vec::new()
    }

    async fn search_fuzzy(&self, _query: &AstQuery) -> Vec<AstMatch> {
        // TODO: Fuzzy matching using Levenshtein distance or similar
        Vec::new()
    }

    async fn search_semantic(&self, _query: &AstQuery) -> Vec<AstMatch> {
        // TODO: Full semantic search with cross-references
        Vec::new()
    }

    /// Compute fuzzy match score between two strings
    pub fn fuzzy_score(pattern: &str, target: &str) -> f32 {
        if pattern.is_empty() || target.is_empty() {
            return 0.0;
        }

        let pattern_lower = pattern.to_lowercase();
        let target_lower = target.to_lowercase();

        // Exact match
        if pattern_lower == target_lower {
            return 1.0;
        }

        // Contains match
        if target_lower.contains(&pattern_lower) {
            return 0.8;
        }

        // Prefix match
        if target_lower.starts_with(&pattern_lower) {
            return 0.9;
        }

        // Calculate Levenshtein-based score
        let distance = levenshtein_distance(&pattern_lower, &target_lower);
        let max_len = pattern.len().max(target.len()) as f32;
        let similarity = 1.0 - (distance as f32 / max_len);

        similarity.max(0.0)
    }
}

impl Default for AstSearchService {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate Levenshtein edit distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let m = s1_chars.len();
    let n = s2_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut matrix = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..=m {
        matrix[i][0] = i;
    }
    for j in 0..=n {
        matrix[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_score() {
        assert_eq!(AstSearchService::fuzzy_score("test", "test"), 1.0);
        assert!(AstSearchService::fuzzy_score("test", "testing") > 0.8);
        assert!(AstSearchService::fuzzy_score("test", "contest") > 0.5);
        assert!(AstSearchService::fuzzy_score("abc", "xyz") < 0.5);
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
    }

    #[test]
    fn test_query_builders() {
        let symbol_query = AstQuery::symbol("main");
        assert_eq!(symbol_query.mode, SearchMode::Symbol);

        let fuzzy_query = AstQuery::fuzzy("test");
        assert_eq!(fuzzy_query.mode, SearchMode::Fuzzy);

        let semantic_query = AstQuery::semantic("function");
        assert_eq!(semantic_query.mode, SearchMode::Semantic);
        assert!(semantic_query.include_refined);
    }
}
