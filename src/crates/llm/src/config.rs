//! Common configuration structures for LLM providers.

use crate::error::{LlmError, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for local LLM providers (Ollama, llama.cpp, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalLlmConfig {
    /// Base URL for the local LLM server.
    /// 
    /// Examples:
    /// - Ollama: "http://localhost:11434"
    /// - llama.cpp: "http://localhost:8080"
    /// - LM Studio: "http://localhost:1234/v1"
    pub base_url: String,

    /// Model name/identifier.
    pub model: String,

    /// Request timeout duration.
    #[serde(default = "default_timeout")]
    pub timeout: Duration,

    /// Maximum retries for failed requests.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl LocalLlmConfig {
    /// Create a new local LLM configuration.
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            timeout: default_timeout(),
            max_retries: default_max_retries(),
        }
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// Configuration for remote LLM providers (OpenAI, Anthropic, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteLlmConfig {
    /// API key for authentication.
    pub api_key: String,

    /// Base URL for the API.
    /// 
    /// Examples:
    /// - OpenAI: "https://api.openai.com/v1"
    /// - Anthropic: "https://api.anthropic.com"
    /// - Grok: "https://api.x.ai/v1"
    /// - Deepseek: "https://api.deepseek.com"
    /// - OpenRouter: "https://openrouter.ai/api/v1"
    pub base_url: String,

    /// Model name/identifier.
    pub model: String,

    /// Request timeout duration.
    #[serde(default = "default_timeout")]
    pub timeout: Duration,

    /// Maximum retries for failed requests.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Organization ID (optional, for providers that support it).
    pub organization: Option<String>,
}

impl RemoteLlmConfig {
    /// Create a new remote LLM configuration.
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            model: model.into(),
            timeout: default_timeout(),
            max_retries: default_max_retries(),
            organization: None,
        }
    }

    /// Create configuration from environment variable.
    pub fn from_env(
        env_var: &str,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self> {
        let api_key = std::env::var(env_var)
            .map_err(|_| LlmError::ApiKeyNotFound(format!("Environment variable: {}", env_var)))?;

        Ok(Self::new(api_key, base_url, model))
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the organization ID.
    pub fn with_organization(mut self, organization: impl Into<String>) -> Self {
        self.organization = Some(organization.into());
        self
    }
}

fn default_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_max_retries() -> u32 {
    3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_config_builder() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2")
            .with_timeout(Duration::from_secs(30))
            .with_max_retries(5);

        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.model, "llama2");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_remote_config_builder() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.openai.com/v1",
            "gpt-4",
        )
        .with_timeout(Duration::from_secs(120))
        .with_organization("org-123");

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.timeout, Duration::from_secs(120));
        assert_eq!(config.organization, Some("org-123".to_string()));
    }
}

