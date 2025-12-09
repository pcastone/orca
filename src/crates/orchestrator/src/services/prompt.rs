//! Prompt Service - LLM prompt functionality for orchestrator
//!
//! Provides an interface for sending prompts to LLM providers via the API.

use crate::config::LlmConfig;
use llm::{ChatModel, ChatRequest, ChatResponse, Message, LocalLlmConfig, RemoteLlmConfig};
use thiserror::Error;
use tracing::{debug, warn};

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("Prompt cannot be empty")]
    EmptyPrompt,
    #[error("LLM not configured")]
    NotConfigured,
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),
    #[error("LLM call failed: {0}")]
    LlmError(String),
    #[error("Missing API key for provider: {0}")]
    MissingApiKey(String),
}

/// Service for sending prompts to LLM providers
pub struct PromptService {
    provider: std::sync::Arc<dyn ChatModel + Send + Sync>,
}

impl PromptService {
    /// Create a new PromptService from LLM configuration
    pub fn new(config: &LlmConfig) -> Result<Self, PromptError> {
        if !config.enabled {
            return Err(PromptError::NotConfigured);
        }

        let provider: std::sync::Arc<dyn ChatModel + Send + Sync> = match config.provider.to_lowercase().as_str() {
            "ollama" => {
                let api_base = config.api_base.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
                let local_config = LocalLlmConfig::new(&api_base, &config.model);
                std::sync::Arc::new(llm::local::OllamaClient::new(local_config))
            }
            "lmstudio" => {
                let api_base = config.api_base.clone().unwrap_or_else(|| "http://localhost:1234".to_string());
                let local_config = LocalLlmConfig::new(&api_base, &config.model);
                std::sync::Arc::new(llm::local::LmStudioClient::new(local_config))
            }
            "llamacpp" => {
                let api_base = config.api_base.clone().unwrap_or_else(|| "http://localhost:8080".to_string());
                let local_config = LocalLlmConfig::new(&api_base, &config.model);
                std::sync::Arc::new(llm::local::LlamaCppClient::new(local_config))
            }
            "openai" => {
                let api_key = config.get_api_key()
                    .ok_or_else(|| PromptError::MissingApiKey("openai".to_string()))?;
                let api_base = config.api_base.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                let remote_config = RemoteLlmConfig::new(api_key, api_base, config.model.clone());
                std::sync::Arc::new(llm::remote::OpenAiClient::new(remote_config))
            }
            "claude" | "anthropic" => {
                let api_key = config.get_api_key()
                    .ok_or_else(|| PromptError::MissingApiKey("claude".to_string()))?;
                let api_base = config.api_base.clone().unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
                let remote_config = RemoteLlmConfig::new(api_key, api_base, config.model.clone());
                std::sync::Arc::new(llm::remote::ClaudeClient::new(remote_config))
            }
            "deepseek" => {
                let api_key = config.get_api_key()
                    .ok_or_else(|| PromptError::MissingApiKey("deepseek".to_string()))?;
                let api_base = config.api_base.clone().unwrap_or_else(|| "https://api.deepseek.com".to_string());
                let remote_config = RemoteLlmConfig::new(api_key, api_base, config.model.clone());
                std::sync::Arc::new(llm::remote::DeepseekClient::new(remote_config))
            }
            "grok" => {
                let api_key = config.get_api_key()
                    .ok_or_else(|| PromptError::MissingApiKey("grok".to_string()))?;
                let api_base = config.api_base.clone().unwrap_or_else(|| "https://api.x.ai/v1".to_string());
                let remote_config = RemoteLlmConfig::new(api_key, api_base, config.model.clone());
                std::sync::Arc::new(llm::remote::GrokClient::new(remote_config))
            }
            "openrouter" => {
                let api_key = config.get_api_key()
                    .ok_or_else(|| PromptError::MissingApiKey("openrouter".to_string()))?;
                let api_base = config.api_base.clone().unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
                let remote_config = RemoteLlmConfig::new(api_key, api_base, config.model.clone());
                std::sync::Arc::new(llm::remote::OpenRouterClient::new(remote_config))
            }
            "gemini" | "google" => {
                let api_key = config.get_api_key()
                    .ok_or_else(|| PromptError::MissingApiKey("gemini".to_string()))?;
                let api_base = config.api_base.clone().unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());
                let remote_config = RemoteLlmConfig::new(api_key, api_base, config.model.clone());
                std::sync::Arc::new(llm::remote::GeminiClient::new(remote_config))
            }
            other => return Err(PromptError::UnsupportedProvider(other.to_string())),
        };

        Ok(Self { provider })
    }

    /// Send a prompt to the LLM and get a response
    pub async fn send_prompt(&self, prompt: &str) -> Result<String, PromptError> {
        if prompt.is_empty() {
            return Err(PromptError::EmptyPrompt);
        }

        debug!("Sending prompt to LLM: {}...", &prompt[..prompt.len().min(50)]);

        // Create a simple chat request with a human message
        let message = Message::human(prompt);
        let request = ChatRequest::new(vec![message]);

        // Call the LLM
        let response: ChatResponse = self.provider.chat(request).await.map_err(|e| {
            warn!("LLM call failed: {}", e);
            PromptError::LlmError(e.to_string())
        })?;

        // Extract the response text
        let response_text = response.message.text().unwrap_or("").to_string();

        if !response_text.is_empty() {
            debug!("Received response: {}...", &response_text[..response_text.len().min(50)]);
        }

        Ok(response_text)
    }

    /// Get a clone of the LLM client for use in task execution
    pub fn llm_client(&self) -> std::sync::Arc<dyn ChatModel + Send + Sync> {
        self.provider.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_service_not_configured() {
        let config = LlmConfig::default(); // enabled is false by default
        let result = PromptService::new(&config);
        assert!(matches!(result, Err(PromptError::NotConfigured)));
    }

    #[test]
    fn test_prompt_service_unsupported_provider() {
        let config = LlmConfig {
            enabled: true,
            provider: "invalid_provider".to_string(),
            ..Default::default()
        };
        let result = PromptService::new(&config);
        assert!(matches!(result, Err(PromptError::UnsupportedProvider(_))));
    }

    #[test]
    fn test_prompt_service_creates_with_ollama() {
        let config = LlmConfig {
            enabled: true,
            provider: "ollama".to_string(),
            model: "llama2".to_string(),
            api_base: Some("http://localhost:11434".to_string()),
            ..Default::default()
        };
        let result = PromptService::new(&config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_prompt_service_empty_prompt_error() {
        let config = LlmConfig {
            enabled: true,
            provider: "ollama".to_string(),
            model: "llama2".to_string(),
            ..Default::default()
        };
        let service = PromptService::new(&config).unwrap();
        let result = service.send_prompt("").await;
        assert!(matches!(result, Err(PromptError::EmptyPrompt)));
    }
}
