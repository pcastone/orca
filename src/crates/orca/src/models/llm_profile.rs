use serde::{Deserialize, Serialize};

/// LLM configuration reference (planner or worker)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,    // 'openai', 'anthropic', 'ollama', etc.
    pub model: String,       // 'gpt-4', 'claude-3-sonnet', etc.
}

/// LLM Profile for multi-LLM workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProfile {
    pub id: String,
    pub name: String,

    // Planner LLM (for planning/decomposition)
    pub planner_provider: String,
    pub planner_model: String,

    // Worker LLM (for execution)
    pub worker_provider: String,
    pub worker_model: String,

    // Metadata
    pub description: Option<String>,
    pub active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl LlmProfile {
    /// Create a new LLM profile
    pub fn new(
        id: String,
        name: String,
        planner_provider: String,
        planner_model: String,
        worker_provider: String,
        worker_model: String,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            name,
            planner_provider,
            planner_model,
            worker_provider,
            worker_model,
            description: None,
            active: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get planner config
    pub fn planner_config(&self) -> LlmConfig {
        LlmConfig {
            provider: self.planner_provider.clone(),
            model: self.planner_model.clone(),
        }
    }

    /// Get worker config
    pub fn worker_config(&self) -> LlmConfig {
        LlmConfig {
            provider: self.worker_provider.clone(),
            model: self.worker_model.clone(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}
