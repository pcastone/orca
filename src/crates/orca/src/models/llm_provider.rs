//! LLM Provider model

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// LLM Provider configuration
///
/// Stores configuration for LLM providers like OpenAI, Anthropic, Ollama, etc.
/// Stored in user database (~/.orca/user.db)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LlmProviderConfig {
    /// Unique provider identifier (UUID string)
    pub id: String,

    /// Human-readable name for the provider
    pub name: String,

    /// Provider type (openai, anthropic, ollama, etc.)
    pub provider_type: String,

    /// Model name/identifier
    pub model: String,

    /// API key (encrypted or env var reference)
    pub api_key: Option<String>,

    /// API base URL (optional, for custom endpoints)
    pub api_base: Option<String>,

    /// Temperature parameter (0.0 to 1.0)
    pub temperature: f64,

    /// Maximum tokens to generate
    pub max_tokens: i64,

    /// Additional settings as JSON string
    pub settings: String,

    /// Whether this is the default provider
    pub is_default: bool,

    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,
}

impl LlmProviderConfig {
    /// Create a new LLM provider configuration
    pub fn new(name: String, provider_type: String, model: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            provider_type,
            model,
            api_key: None,
            api_base: None,
            temperature: 0.7,
            max_tokens: 4096,
            settings: "{}".to_string(),
            is_default: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Builder: Set API key
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// Builder: Set API base URL
    pub fn with_api_base(mut self, api_base: String) -> Self {
        self.api_base = Some(api_base);
        self
    }

    /// Builder: Set temperature
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    /// Builder: Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: i64) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Builder: Set as default provider
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }
}
