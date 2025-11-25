//! Task Classifier Service
//!
//! Classifies tasks into categories for dynamic pattern selection.
//! Uses keyword-based classification with optional LLM enhancement.

use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::Message;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, warn};

/// Task category for pattern routing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskCategory {
    /// Quick lookups, factual Q&A, simple questions
    SimpleQuery,
    /// Read/write/list files, file operations
    FileOperation,
    /// Write code, tests, refactoring, debugging
    CodeGeneration,
    /// Multi-step research, analysis, investigation
    Research,
    /// Analyze data, generate reports, statistics
    DataAnalysis,
    /// Run commands, check status, system operations
    SystemCommand,
    /// General tasks that don't fit other categories
    General,
    /// User-defined category
    Custom(String),
}

impl TaskCategory {
    /// Get string representation
    pub fn as_str(&self) -> &str {
        match self {
            Self::SimpleQuery => "simple_query",
            Self::FileOperation => "file_operation",
            Self::CodeGeneration => "code_generation",
            Self::Research => "research",
            Self::DataAnalysis => "data_analysis",
            Self::SystemCommand => "system_command",
            Self::General => "general",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Get human-readable name
    pub fn display_name(&self) -> &str {
        match self {
            Self::SimpleQuery => "Simple Query",
            Self::FileOperation => "File Operation",
            Self::CodeGeneration => "Code Generation",
            Self::Research => "Research",
            Self::DataAnalysis => "Data Analysis",
            Self::SystemCommand => "System Command",
            Self::General => "General",
            Self::Custom(_) => "Custom",
        }
    }

    /// Get recommended pattern config ID for this category
    pub fn default_pattern_config_id(&self) -> &str {
        match self {
            Self::SimpleQuery => "default_react_simple",
            Self::FileOperation => "default_react",
            Self::CodeGeneration => "default_reflection_code",
            Self::Research => "default_plan_execute",
            Self::DataAnalysis => "default_plan_execute",
            Self::SystemCommand => "default_react",
            Self::General => "default_react",
            Self::Custom(_) => "default_react",
        }
    }
}

impl std::fmt::Display for TaskCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Classification rule with pattern and category
#[derive(Debug, Clone)]
struct ClassificationRule {
    patterns: Vec<Regex>,
    category: TaskCategory,
    priority: u8, // Higher = more specific, checked first
}

/// Task classifier for dynamic pattern selection
pub struct TaskClassifier {
    rules: Vec<ClassificationRule>,
    /// Optional LLM client for enhanced classification
    llm_client: Option<Arc<dyn ChatModel>>,
}

impl std::fmt::Debug for TaskClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskClassifier")
            .field("rules_count", &self.rules.len())
            .field("has_llm", &self.llm_client.is_some())
            .finish()
    }
}

impl Default for TaskClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskClassifier {
    /// Create a new task classifier with default rules
    pub fn new() -> Self {
        let mut classifier = Self {
            rules: Vec::new(),
            llm_client: None,
        };
        classifier.add_default_rules();
        classifier
    }

    /// Create a task classifier with an LLM client for enhanced classification
    pub fn with_llm(llm_client: Arc<dyn ChatModel>) -> Self {
        let mut classifier = Self::new();
        classifier.llm_client = Some(llm_client);
        classifier
    }

    /// Set the LLM client for enhanced classification
    pub fn set_llm_client(&mut self, llm_client: Arc<dyn ChatModel>) {
        self.llm_client = Some(llm_client);
    }

    /// Check if LLM client is configured
    pub fn has_llm(&self) -> bool {
        self.llm_client.is_some()
    }

    /// Add default classification rules
    fn add_default_rules(&mut self) {
        // Code Generation patterns (high priority)
        self.add_rule(
            TaskCategory::CodeGeneration,
            vec![
                r"(?i)\b(write|create|implement|build|develop)\s+(a\s+)?(code|function|method|class|module|test|tests|unit test)",
                r"(?i)\b(add|fix|refactor|debug|optimize)\s+(the\s+)?(code|function|method|bug|issue)",
                r"(?i)\bwrite\s+(me\s+)?(a\s+)?.*\b(function|method|class|script|program)",
                r"(?i)\b(unit\s+)?test(s)?\s+for\b",
                r"(?i)\brefactor\b",
                r"(?i)\bdebug\b",
                r"(?i)\bimplement\b.*\b(feature|functionality|interface|api)",
            ],
            90,
        );

        // Research patterns (high priority)
        self.add_rule(
            TaskCategory::Research,
            vec![
                r"(?i)\b(research|investigate|analyze|explore|study)\b.*\b(about|how|why|what)",
                r"(?i)\b(compare|contrast|evaluate)\b.*\b(options|approaches|methods|tools)",
                r"(?i)\bfind\s+(out|information|details)\s+about\b",
                r"(?i)\bwhat\s+are\s+(the\s+)?(best\s+)?practices\b",
                r"(?i)\bhow\s+(do|does|can|should)\b.*\b(work|implement|use)",
                r"(?i)\bexplain\s+(in\s+detail|thoroughly|comprehensively)",
                r"(?i)\bcreate\s+(a\s+)?(comprehensive|detailed)\s+(report|analysis|summary)",
            ],
            85,
        );

        // Data Analysis patterns
        self.add_rule(
            TaskCategory::DataAnalysis,
            vec![
                r"(?i)\b(analyze|analyse)\s+(the\s+)?(data|metrics|statistics|results)",
                r"(?i)\b(generate|create)\s+(a\s+)?(report|chart|graph|visualization)",
                r"(?i)\bcalculate\b.*\b(average|sum|total|statistics|metrics)",
                r"(?i)\b(aggregate|summarize|summarise)\s+(the\s+)?data",
                r"(?i)\bdata\s+(analysis|processing|transformation)",
            ],
            80,
        );

        // File Operation patterns
        self.add_rule(
            TaskCategory::FileOperation,
            vec![
                r"(?i)\b(read|open|load)\s+(the\s+)?file",
                r"(?i)\b(write|save|create)\s+(a\s+)?(new\s+)?file",
                r"(?i)\b(edit|modify|update)\s+(the\s+)?file",
                r"(?i)\b(delete|remove)\s+(the\s+)?file",
                r"(?i)\b(list|show|display)\s+(all\s+)?(the\s+)?files",
                r"(?i)\b(copy|move|rename)\s+(the\s+)?file",
                r"(?i)\bfind\s+(all\s+)?files\b",
                r"(?i)\b(in|from|to)\s+(the\s+)?(directory|folder|path)",
            ],
            75,
        );

        // System Command patterns
        self.add_rule(
            TaskCategory::SystemCommand,
            vec![
                r"(?i)\b(run|execute)\s+(the\s+)?(command|script|program)",
                r"(?i)\b(check|show|display)\s+(the\s+)?(status|version|info)",
                r"(?i)\b(install|uninstall|update)\s+(the\s+)?(package|dependency|tool)",
                r"(?i)\b(start|stop|restart)\s+(the\s+)?(server|service|process)",
                r"(?i)\bgit\s+(status|log|diff|commit|push|pull)",
                r"(?i)\b(build|compile|test)\s+(the\s+)?(project|code)",
            ],
            70,
        );

        // Simple Query patterns (lower priority - catch simple questions)
        self.add_rule(
            TaskCategory::SimpleQuery,
            vec![
                r"(?i)^what\s+is\b",
                r"(?i)^who\s+is\b",
                r"(?i)^where\s+is\b",
                r"(?i)^when\s+(is|was|did)\b",
                r"(?i)^how\s+many\b",
                r"(?i)^how\s+much\b",
                r"(?i)^(is|are|was|were|do|does|did|can|could|will|would)\s+",
                r"(?i)\b(list|show|tell\s+me)\s+(the\s+)?\w+$",
                r"(?i)^(yes|no)\??\s*$",
                r"(?i)^\d+\s*[\+\-\*\/]\s*\d+",  // Simple math
            ],
            50,
        );
    }

    /// Add a classification rule
    fn add_rule(&mut self, category: TaskCategory, patterns: Vec<&str>, priority: u8) {
        let compiled_patterns: Vec<Regex> = patterns
            .into_iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        if !compiled_patterns.is_empty() {
            self.rules.push(ClassificationRule {
                patterns: compiled_patterns,
                category,
                priority,
            });
        }

        // Sort rules by priority (highest first)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Classify a task based on its description
    pub fn classify(&self, task_description: &str) -> TaskCategory {
        let description = task_description.trim();

        // Check each rule in priority order
        for rule in &self.rules {
            for pattern in &rule.patterns {
                if pattern.is_match(description) {
                    return rule.category.clone();
                }
            }
        }

        // Default to General if no patterns match
        TaskCategory::General
    }

    /// Classify with confidence score (0.0 - 1.0)
    pub fn classify_with_confidence(&self, task_description: &str) -> (TaskCategory, f64) {
        let description = task_description.trim();
        let mut match_count = 0;
        let mut matched_category: Option<TaskCategory> = None;

        // Check each rule and count matches
        for rule in &self.rules {
            let rule_matches: usize = rule
                .patterns
                .iter()
                .filter(|p| p.is_match(description))
                .count();

            if rule_matches > 0 && matched_category.is_none() {
                matched_category = Some(rule.category.clone());
            }
            match_count += rule_matches;
        }

        let category = matched_category.unwrap_or(TaskCategory::General);

        // Calculate confidence based on number of matching patterns
        let confidence = match match_count {
            0 => 0.3,       // Default fallback
            1 => 0.6,       // Single match
            2 => 0.75,      // Two matches
            3 => 0.85,      // Three matches
            _ => 0.95,      // Multiple matches = high confidence
        };

        (category, confidence)
    }

    /// Get the recommended pattern config ID for a task
    pub fn get_pattern_config_id(&self, task_description: &str) -> String {
        let category = self.classify(task_description);
        category.default_pattern_config_id().to_string()
    }

    /// Classify a task using LLM for enhanced accuracy
    ///
    /// Uses the LLM to analyze the task description and determine the best category.
    /// Falls back to keyword-based classification if LLM is not configured or fails.
    ///
    /// # Arguments
    /// * `task_description` - The task to classify
    ///
    /// # Returns
    /// A tuple of (TaskCategory, confidence) where confidence is 0.0-1.0
    pub async fn classify_with_llm(
        &self,
        task_description: &str,
    ) -> (TaskCategory, f64) {
        // If no LLM client, fall back to keyword-based classification
        let Some(llm) = &self.llm_client else {
            debug!("No LLM client configured, using keyword-based classification");
            return self.classify_with_confidence(task_description);
        };

        let prompt = format!(
            r#"Classify the following task into exactly ONE of these categories:
- simple_query: Quick lookups, factual Q&A, simple questions (e.g., "What is X?", "How many?")
- file_operation: Read/write/list files, file operations (e.g., "Read config.toml", "List files")
- code_generation: Write code, tests, refactoring, debugging (e.g., "Write a function", "Fix bug")
- research: Multi-step research, analysis, investigation (e.g., "Research best practices", "Compare approaches")
- data_analysis: Analyze data, generate reports, statistics (e.g., "Analyze metrics", "Generate report")
- system_command: Run commands, check status, system operations (e.g., "Run tests", "Git status")
- general: Tasks that don't fit other categories

Task: "{}"

Respond with ONLY the category name (one of: simple_query, file_operation, code_generation, research, data_analysis, system_command, general). No explanation."#,
            task_description
        );

        let request = ChatRequest::new(vec![Message::human(prompt)])
            .with_temperature(0.0)  // Deterministic for classification
            .with_max_tokens(50);   // We only need a short response

        match llm.chat(request).await {
            Ok(response) => {
                let response_text = response.message.text().unwrap_or_default();
                let category_str = response_text.trim().to_lowercase();

                debug!("LLM classification response: '{}'", category_str);

                let category = match category_str.as_str() {
                    "simple_query" => TaskCategory::SimpleQuery,
                    "file_operation" => TaskCategory::FileOperation,
                    "code_generation" => TaskCategory::CodeGeneration,
                    "research" => TaskCategory::Research,
                    "data_analysis" => TaskCategory::DataAnalysis,
                    "system_command" => TaskCategory::SystemCommand,
                    "general" => TaskCategory::General,
                    _ => {
                        // If LLM returned unexpected value, fall back to keyword classification
                        warn!(
                            "LLM returned unexpected category '{}', falling back to keywords",
                            category_str
                        );
                        return self.classify_with_confidence(task_description);
                    }
                };

                // LLM classification has high confidence
                (category, 0.9)
            }
            Err(e) => {
                warn!("LLM classification failed: {}, falling back to keywords", e);
                self.classify_with_confidence(task_description)
            }
        }
    }

    /// Classify using LLM if available, otherwise use keywords
    ///
    /// This is a convenience method that uses LLM classification when available
    /// and falls back to keyword-based classification otherwise.
    pub async fn classify_smart(&self, task_description: &str) -> TaskCategory {
        if self.llm_client.is_some() {
            let (category, _confidence) = self.classify_with_llm(task_description).await;
            category
        } else {
            self.classify(task_description)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_simple_query() {
        let classifier = TaskClassifier::new();

        // Questions starting with "what is" should match
        assert_eq!(
            classifier.classify("What is 2+2?"),
            TaskCategory::SimpleQuery
        );
        assert_eq!(
            classifier.classify("How many items are there?"),
            TaskCategory::SimpleQuery
        );
    }

    #[test]
    fn test_classify_code_generation() {
        let classifier = TaskClassifier::new();

        assert_eq!(
            classifier.classify("Write a function to sort an array"),
            TaskCategory::CodeGeneration
        );
        assert_eq!(
            classifier.classify("Implement unit tests for the auth module"),
            TaskCategory::CodeGeneration
        );
        assert_eq!(
            classifier.classify("Refactor the database connection code"),
            TaskCategory::CodeGeneration
        );
        assert_eq!(
            classifier.classify("Debug the login issue"),
            TaskCategory::CodeGeneration
        );
    }

    #[test]
    fn test_classify_research() {
        let classifier = TaskClassifier::new();

        assert_eq!(
            classifier.classify("Research how async/await works in Rust"),
            TaskCategory::Research
        );
        assert_eq!(
            classifier.classify("Investigate how the caching system works"),
            TaskCategory::Research
        );
        assert_eq!(
            classifier.classify("Compare different approaches for state management"),
            TaskCategory::Research
        );
    }

    #[test]
    fn test_classify_file_operation() {
        let classifier = TaskClassifier::new();

        // "read ... file" pattern
        assert_eq!(
            classifier.classify("read file config.toml"),
            TaskCategory::FileOperation
        );
        // "write ... file" pattern
        assert_eq!(
            classifier.classify("write file output.txt"),
            TaskCategory::FileOperation
        );
        // "list ... files" pattern
        assert_eq!(
            classifier.classify("list all files"),
            TaskCategory::FileOperation
        );
    }

    #[test]
    fn test_classify_system_command() {
        let classifier = TaskClassifier::new();

        // "run ... command" pattern
        assert_eq!(
            classifier.classify("run the command now"),
            TaskCategory::SystemCommand
        );
        // "git status" pattern
        assert_eq!(
            classifier.classify("git status"),
            TaskCategory::SystemCommand
        );
        // "build ... project" pattern
        assert_eq!(
            classifier.classify("build the project"),
            TaskCategory::SystemCommand
        );
    }

    #[test]
    fn test_classify_data_analysis() {
        let classifier = TaskClassifier::new();

        // "analyze ... data" pattern
        assert_eq!(
            classifier.classify("analyze the data"),
            TaskCategory::DataAnalysis
        );
        // "generate ... report" pattern
        assert_eq!(
            classifier.classify("generate a report"),
            TaskCategory::DataAnalysis
        );
    }

    #[test]
    fn test_classify_general_fallback() {
        let classifier = TaskClassifier::new();

        // Vague or unclear tasks should fall back to General
        assert_eq!(
            classifier.classify("xyz abc 123 random"),
            TaskCategory::General
        );
        assert_eq!(
            classifier.classify("blah blah"),
            TaskCategory::General
        );
    }

    #[test]
    fn test_classify_with_confidence() {
        let classifier = TaskClassifier::new();

        let (category, confidence) = classifier.classify_with_confidence("Write unit tests for the auth module");
        assert_eq!(category, TaskCategory::CodeGeneration);
        assert!(confidence > 0.5);

        let (category, confidence) = classifier.classify_with_confidence("xyz abc 123 random stuff");
        assert_eq!(category, TaskCategory::General);
        assert!(confidence < 0.5);
    }

    #[test]
    fn test_get_pattern_config_id() {
        let classifier = TaskClassifier::new();

        // Code generation should map to reflection
        assert_eq!(
            classifier.get_pattern_config_id("Write unit tests for auth"),
            "default_reflection_code"
        );

        // Research should map to plan_execute
        assert_eq!(
            classifier.get_pattern_config_id("Research how to implement caching"),
            "default_plan_execute"
        );
    }

    #[test]
    fn test_task_category_display() {
        assert_eq!(TaskCategory::SimpleQuery.display_name(), "Simple Query");
        assert_eq!(TaskCategory::CodeGeneration.display_name(), "Code Generation");
        assert_eq!(TaskCategory::Research.display_name(), "Research");
    }

    #[test]
    fn test_task_category_as_str() {
        assert_eq!(TaskCategory::SimpleQuery.as_str(), "simple_query");
        assert_eq!(TaskCategory::CodeGeneration.as_str(), "code_generation");
        assert_eq!(TaskCategory::Custom("my_category".to_string()).as_str(), "my_category");
    }

    #[test]
    fn test_classifier_without_llm() {
        let classifier = TaskClassifier::new();
        assert!(!classifier.has_llm());
    }

    #[tokio::test]
    async fn test_classify_with_llm_no_client_fallback() {
        // Without LLM client, should fall back to keyword classification
        let classifier = TaskClassifier::new();

        let (category, confidence) = classifier.classify_with_llm("Write a function").await;
        assert_eq!(category, TaskCategory::CodeGeneration);
        assert!(confidence > 0.5);
    }

    #[tokio::test]
    async fn test_classify_smart_without_llm() {
        let classifier = TaskClassifier::new();

        // Without LLM, should use keyword-based classification
        // "Research how ..." matches the research pattern
        let category = classifier.classify_smart("Research how to implement caching").await;
        assert_eq!(category, TaskCategory::Research);
    }

    #[test]
    fn test_classifier_debug_format() {
        let classifier = TaskClassifier::new();
        let debug_str = format!("{:?}", classifier);
        assert!(debug_str.contains("TaskClassifier"));
        assert!(debug_str.contains("rules_count"));
        assert!(debug_str.contains("has_llm"));
    }
}
