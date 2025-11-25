//! Pattern Router Service
//!
//! Routes tasks to appropriate pattern configurations based on
//! explicit config ID or automatic classification.

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::PatternConfig;
use crate::repositories::PatternConfigRepository;
use crate::services::task_classifier::{TaskCategory, TaskClassifier};
use crate::workflow::Task;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Pattern router for dynamic pattern selection
pub struct PatternRouter {
    config_repo: PatternConfigRepository,
    classifier: TaskClassifier,
    category_map: HashMap<TaskCategory, String>,
}

impl PatternRouter {
    /// Create a new pattern router
    pub fn new(db: Arc<Database>) -> Self {
        let config_repo = PatternConfigRepository::new(db);
        let classifier = TaskClassifier::new();
        let category_map = Self::default_category_map();

        Self {
            config_repo,
            classifier,
            category_map,
        }
    }

    /// Create with custom category mappings
    pub fn with_category_map(db: Arc<Database>, category_map: HashMap<TaskCategory, String>) -> Self {
        let config_repo = PatternConfigRepository::new(db);
        let classifier = TaskClassifier::new();

        Self {
            config_repo,
            classifier,
            category_map,
        }
    }

    /// Get default category to config ID mappings
    fn default_category_map() -> HashMap<TaskCategory, String> {
        let mut map = HashMap::new();
        map.insert(TaskCategory::SimpleQuery, "default_react_simple".to_string());
        map.insert(TaskCategory::FileOperation, "default_react".to_string());
        map.insert(TaskCategory::CodeGeneration, "default_reflection_code".to_string());
        map.insert(TaskCategory::Research, "default_plan_execute".to_string());
        map.insert(TaskCategory::DataAnalysis, "default_plan_execute".to_string());
        map.insert(TaskCategory::SystemCommand, "default_react".to_string());
        map.insert(TaskCategory::General, "default_react".to_string());
        map
    }

    /// Route a task to its pattern configuration
    ///
    /// Priority:
    /// 1. If task.pattern_config_id is set, use that directly
    /// 2. Else classify the task and map to config
    /// 3. Fall back to default if config not found
    pub async fn route(&self, task: &Task) -> Result<PatternConfig> {
        // Priority 1: Explicit pattern config ID
        if let Some(ref config_id) = task.pattern_config_id {
            debug!(config_id = %config_id, "Using explicit pattern config");
            match self.config_repo.find_by_id(config_id).await {
                Ok(config) => {
                    info!(
                        task_id = %task.id,
                        pattern = %config.pattern_type,
                        config_name = %config.name,
                        "Routed task to explicit pattern config"
                    );
                    // Increment usage count
                    let _ = self.config_repo.increment_usage(config_id).await;
                    return Ok(config);
                }
                Err(e) => {
                    warn!(
                        config_id = %config_id,
                        error = %e,
                        "Explicit pattern config not found, falling back to classification"
                    );
                }
            }
        }

        // Priority 2: Classify and route
        let category = self.classifier.classify(&task.description);
        debug!(
            task_description = %task.description,
            category = %category.as_str(),
            "Classified task"
        );

        let config_id = self.map_category_to_config(&category);
        debug!(config_id = %config_id, "Mapped category to config");

        match self.config_repo.find_by_id(&config_id).await {
            Ok(config) => {
                info!(
                    task_id = %task.id,
                    category = %category.as_str(),
                    pattern = %config.pattern_type,
                    config_name = %config.name,
                    "Routed task via classification"
                );
                // Increment usage count
                let _ = self.config_repo.increment_usage(&config_id).await;
                Ok(config)
            }
            Err(_) => {
                // Priority 3: Fall back to default
                warn!(
                    config_id = %config_id,
                    "Mapped config not found, falling back to default"
                );
                self.get_default_config().await
            }
        }
    }

    /// Map a task category to its config ID
    pub fn map_category_to_config(&self, category: &TaskCategory) -> String {
        self.category_map
            .get(category)
            .cloned()
            .unwrap_or_else(|| category.default_pattern_config_id().to_string())
    }

    /// Get the default pattern configuration
    pub async fn get_default_config(&self) -> Result<PatternConfig> {
        match self.config_repo.find_default().await {
            Ok(config) => {
                info!(config_name = %config.name, "Using default pattern config");
                Ok(config)
            }
            Err(_) => {
                // Last resort: try to find any react config
                warn!("No default config found, trying to find any react config");
                match self.config_repo.find_by_id("default_react").await {
                    Ok(config) => Ok(config),
                    Err(_) => {
                        // Create a minimal fallback config in memory
                        warn!("No pattern configs found in database, using hardcoded fallback");
                        Ok(PatternConfig::new("Fallback", crate::models::PatternType::React)
                            .with_max_iterations(10)
                            .with_system_prompt("You are a helpful assistant."))
                    }
                }
            }
        }
    }

    /// Route by description only (without a full Task)
    pub async fn route_by_description(&self, description: &str) -> Result<PatternConfig> {
        let category = self.classifier.classify(description);
        let config_id = self.map_category_to_config(&category);

        match self.config_repo.find_by_id(&config_id).await {
            Ok(config) => {
                let _ = self.config_repo.increment_usage(&config_id).await;
                Ok(config)
            }
            Err(_) => self.get_default_config().await,
        }
    }

    /// Get classification for a task description
    pub fn classify(&self, description: &str) -> TaskCategory {
        self.classifier.classify(description)
    }

    /// Get classification with confidence
    pub fn classify_with_confidence(&self, description: &str) -> (TaskCategory, f64) {
        self.classifier.classify_with_confidence(description)
    }

    /// List all available pattern configurations
    pub async fn list_configs(&self) -> Result<Vec<PatternConfig>> {
        self.config_repo.list().await
    }

    /// Get a specific pattern configuration by ID
    pub async fn get_config(&self, config_id: &str) -> Result<PatternConfig> {
        self.config_repo.find_by_id(config_id).await
    }

    /// Get a specific pattern configuration by name
    pub async fn get_config_by_name(&self, name: &str) -> Result<PatternConfig> {
        self.config_repo.find_by_name(name).await
    }

    /// Update category mapping
    pub fn set_category_mapping(&mut self, category: TaskCategory, config_id: String) {
        self.category_map.insert(category, config_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> Arc<Database> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        db.run_migrations().await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_route_with_explicit_config() {
        let db = setup_test_db().await;
        let router = PatternRouter::new(db);

        // Create task with explicit config
        let task = Task::new("Any task description")
            .with_pattern_config("default_react_simple");

        let config = router.route(&task).await.unwrap();
        assert_eq!(config.id, "default_react_simple");
        assert_eq!(config.pattern_type, "react");
    }

    #[tokio::test]
    async fn test_route_with_classification() {
        let db = setup_test_db().await;
        let router = PatternRouter::new(db);

        // Task without explicit config - should classify
        let task = Task::new("Write unit tests for the authentication module");
        let config = router.route(&task).await.unwrap();

        // Should classify as CodeGeneration -> default_reflection_code
        assert_eq!(config.id, "default_reflection_code");
        assert_eq!(config.pattern_type, "reflection");
    }

    #[tokio::test]
    async fn test_route_simple_query() {
        let db = setup_test_db().await;
        let router = PatternRouter::new(db);

        let task = Task::new("What is the capital of France?");
        let config = router.route(&task).await.unwrap();

        assert_eq!(config.id, "default_react_simple");
        assert_eq!(config.max_iterations, 3);
    }

    #[tokio::test]
    async fn test_route_research_task() {
        let db = setup_test_db().await;
        let router = PatternRouter::new(db);

        // Use a phrase that matches research patterns: "research how ... works"
        let task = Task::new("Research how error handling works in Rust");
        let config = router.route(&task).await.unwrap();

        assert_eq!(config.id, "default_plan_execute");
        assert_eq!(config.pattern_type, "plan_execute");
    }

    #[tokio::test]
    async fn test_route_fallback_to_default() {
        let db = setup_test_db().await;
        let router = PatternRouter::new(db);

        // Task with non-existent explicit config
        let task = Task::new("Some task")
            .with_pattern_config("non_existent_config");

        let config = router.route(&task).await.unwrap();

        // Should fall back through classification
        assert!(!config.id.is_empty());
    }

    #[tokio::test]
    async fn test_route_by_description() {
        let db = setup_test_db().await;
        let router = PatternRouter::new(db);

        let config = router.route_by_description("Write a function to parse JSON").await.unwrap();
        assert_eq!(config.pattern_type, "reflection");

        let config = router.route_by_description("What is 2+2?").await.unwrap();
        assert_eq!(config.id, "default_react_simple");
    }

    #[tokio::test]
    async fn test_classify() {
        let db = setup_test_db().await;
        let router = PatternRouter::new(db);

        assert_eq!(
            router.classify("Write unit tests"),
            TaskCategory::CodeGeneration
        );
        assert_eq!(
            router.classify("What is the weather?"),
            TaskCategory::SimpleQuery
        );
    }

    #[tokio::test]
    async fn test_list_configs() {
        let db = setup_test_db().await;
        let router = PatternRouter::new(db);

        let configs = router.list_configs().await.unwrap();
        assert!(configs.len() >= 4); // At least the 4 default configs
    }

    #[tokio::test]
    async fn test_get_config() {
        let db = setup_test_db().await;
        let router = PatternRouter::new(db);

        let config = router.get_config("default_react").await.unwrap();
        assert_eq!(config.name, "General ReAct");

        let config = router.get_config_by_name("Quick Tasks").await.unwrap();
        assert_eq!(config.id, "default_react_simple");
    }

    #[tokio::test]
    async fn test_custom_category_mapping() {
        let db = setup_test_db().await;
        let mut category_map = HashMap::new();
        category_map.insert(TaskCategory::SimpleQuery, "default_react".to_string());

        let router = PatternRouter::with_category_map(db, category_map);

        // SimpleQuery now maps to default_react instead of default_react_simple
        let task = Task::new("What is 2+2?");
        let config = router.route(&task).await.unwrap();
        assert_eq!(config.id, "default_react");
    }
}
