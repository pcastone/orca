//! LLM Provider Integration - Wraps llm crate providers as LlmFunction
//!
//! Bridges the llm crate's ChatModel implementations to the LlmFunction type
//! expected by langgraph-prebuilt agents.

use crate::config::OrcaConfig;
use crate::error::{OrcaError, Result};
use langgraph_core::llm::ChatRequest;
use langgraph_prebuilt::Message; // Use the re-exported Message from langgraph_prebuilt
use langgraph_prebuilt::agents::react::LlmFunction;
use llm::config::{LocalLlmConfig, RemoteLlmConfig};
use llm::ChatModel; // Trait for chat method
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, warn};

/// LLM provider that implements ChatModel
///
/// Wraps either a local or remote LLM provider from the llm crate
pub enum LlmProvider {
    Ollama(llm::local::OllamaClient),
    OpenAI(llm::remote::OpenAiClient),
    Claude(llm::remote::ClaudeClient),
    Deepseek(llm::remote::DeepseekClient),
    Grok(llm::remote::GrokClient),
    OpenRouter(llm::remote::OpenRouterClient),
}

impl std::fmt::Debug for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ollama(_) => write!(f, "LlmProvider::Ollama"),
            Self::OpenAI(_) => write!(f, "LlmProvider::OpenAI"),
            Self::Claude(_) => write!(f, "LlmProvider::Claude"),
            Self::Deepseek(_) => write!(f, "LlmProvider::Deepseek"),
            Self::Grok(_) => write!(f, "LlmProvider::Grok"),
            Self::OpenRouter(_) => write!(f, "LlmProvider::OpenRouter"),
        }
    }
}

impl LlmProvider {
    /// Create an LLM provider from Orca configuration
    ///
    /// # Arguments
    /// * `config` - Orca configuration with LLM settings
    ///
    /// # Returns
    /// An LlmProvider instance based on the configured provider
    pub fn from_config(config: &OrcaConfig) -> Result<Self> {
        let provider = config.llm.provider.to_lowercase();

        match provider.as_str() {
            "ollama" => {
                let local_config = LocalLlmConfig::new(
                    config.llm.api_base.clone().unwrap_or_else(|| "http://localhost:11434".to_string()),
                    config.llm.model.clone(),
                );
                Ok(Self::Ollama(llm::local::OllamaClient::new(local_config)))
            }

            "openai" => {
                let api_key = config.llm.api_key.clone()
                    .ok_or_else(|| OrcaError::Config("OpenAI API key not configured".to_string()))?;

                let remote_config = RemoteLlmConfig::new(
                    api_key,
                    config.llm.api_base.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                    config.llm.model.clone(),
                );
                Ok(Self::OpenAI(llm::remote::OpenAiClient::new(remote_config)))
            }

            "anthropic" | "claude" => {
                let api_key = config.llm.api_key.clone()
                    .ok_or_else(|| OrcaError::Config("Anthropic API key not configured".to_string()))?;

                let remote_config = RemoteLlmConfig::new(
                    api_key,
                    config.llm.api_base.clone().unwrap_or_else(|| "https://api.anthropic.com".to_string()),
                    config.llm.model.clone(),
                );
                Ok(Self::Claude(llm::remote::ClaudeClient::new(remote_config)))
            }

            "deepseek" => {
                let api_key = config.llm.api_key.clone()
                    .ok_or_else(|| OrcaError::Config("Deepseek API key not configured".to_string()))?;

                let remote_config = RemoteLlmConfig::new(
                    api_key,
                    config.llm.api_base.clone().unwrap_or_else(|| "https://api.deepseek.com".to_string()),
                    config.llm.model.clone(),
                );
                Ok(Self::Deepseek(llm::remote::DeepseekClient::new(remote_config)))
            }

            "grok" | "xai" => {
                let api_key = config.llm.api_key.clone()
                    .ok_or_else(|| OrcaError::Config("Grok API key not configured".to_string()))?;

                let remote_config = RemoteLlmConfig::new(
                    api_key,
                    config.llm.api_base.clone().unwrap_or_else(|| "https://api.x.ai".to_string()),
                    config.llm.model.clone(),
                );
                Ok(Self::Grok(llm::remote::GrokClient::new(remote_config)))
            }

            "openrouter" => {
                let api_key = config.llm.api_key.clone()
                    .ok_or_else(|| OrcaError::Config("OpenRouter API key not configured".to_string()))?;

                let remote_config = RemoteLlmConfig::new(
                    api_key,
                    config.llm.api_base.clone().unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string()),
                    config.llm.model.clone(),
                );
                Ok(Self::OpenRouter(llm::remote::OpenRouterClient::new(remote_config)))
            }

            _ => Err(OrcaError::Config(format!(
                "Unsupported LLM provider: {}. Available: ollama, openai, claude, deepseek, grok, openrouter",
                provider
            ))),
        }
    }

    /// Call the LLM with a chat request
    async fn chat(&self, _request: ChatRequest) -> llm::Result<llm::ChatResponse> {
        match self {
            Self::Ollama(client) => {
                client.chat(_request).await
                    .map_err(|e| llm::LlmError::InvalidResponse(e.to_string()))
            }
            Self::OpenAI(client) => {
                client.chat(_request).await
                    .map_err(|e| llm::LlmError::InvalidResponse(e.to_string()))
            }
            Self::Claude(client) => {
                client.chat(_request).await
                    .map_err(|e| llm::LlmError::InvalidResponse(e.to_string()))
            }
            Self::Deepseek(client) => {
                client.chat(_request).await
                    .map_err(|e| llm::LlmError::InvalidResponse(e.to_string()))
            }
            Self::Grok(client) => {
                client.chat(_request).await
                    .map_err(|e| llm::LlmError::InvalidResponse(e.to_string()))
            }
            Self::OpenRouter(client) => {
                client.chat(_request).await
                    .map_err(|e| llm::LlmError::InvalidResponse(e.to_string()))
            }
        }
    }
}

/// Create an LlmFunction from an LlmProvider
///
/// This wraps the LlmProvider in the closure format expected by langgraph-prebuilt agents.
///
/// # Arguments
/// * `provider` - The LLM provider to wrap
///
/// # Returns
/// An LlmFunction that can be passed to create_react_agent and similar functions
pub fn create_llm_function(provider: Arc<LlmProvider>) -> LlmFunction {
    Arc::new(move |state: Value| {
        let provider = provider.clone();

        Box::pin(async move {
            // Extract messages from state (they are langgraph_prebuilt::Message)
            let prebuilt_messages: Vec<Message> = state
                .get("messages")
                .and_then(|m| m.as_array())
                .ok_or_else(|| {
                    langgraph_prebuilt::error::PrebuiltError::ToolExecution(
                        "No messages in state".to_string()
                    )
                })?
                .iter()
                .filter_map(|msg_val| serde_json::from_value(msg_val.clone()).ok())
                .collect();

            if prebuilt_messages.is_empty() {
                return Err(langgraph_prebuilt::error::PrebuiltError::ToolExecution(
                    "No valid messages found in state".to_string()
                ));
            }

            debug!("Calling LLM with {} messages", prebuilt_messages.len());

            // Convert langgraph_prebuilt::Message to langgraph_core::Message
            let core_messages: Vec<langgraph_core::Message> = prebuilt_messages
                .into_iter()
                .filter_map(|msg| {
                    // Serialize and deserialize to convert between types
                    serde_json::to_value(&msg)
                        .ok()
                        .and_then(|v| serde_json::from_value(v).ok())
                })
                .collect();

            if core_messages.is_empty() {
                return Err(langgraph_prebuilt::error::PrebuiltError::ToolExecution(
                    "Failed to convert messages for LLM".to_string()
                ));
            }

            // Create chat request
            let request = ChatRequest::new(core_messages);

            // Call LLM
            let response = provider
                .chat(request)
                .await
                .map_err(|e| {
                    warn!("LLM call failed: {}", e);
                    langgraph_prebuilt::error::PrebuiltError::ToolExecution(
                        format!("LLM call failed: {}", e)
                    )
                })?;

            debug!("LLM response received");

            // Convert response message from langgraph_core::Message to langgraph_prebuilt::Message
            // We do this by serializing to JSON and deserializing since they have compatible structures
            let core_message = response.message;
            let message_json = serde_json::to_value(&core_message)
                .map_err(|e| {
                    warn!("Failed to serialize message: {}", e);
                    langgraph_prebuilt::error::PrebuiltError::ToolExecution(
                        format!("Failed to serialize LLM response: {}", e)
                    )
                })?;

            let prebuilt_message: Message = serde_json::from_value(message_json)
                .map_err(|e| {
                    warn!("Failed to deserialize message: {}", e);
                    langgraph_prebuilt::error::PrebuiltError::ToolExecution(
                        format!("Failed to convert LLM response: {}", e)
                    )
                })?;

            Ok(prebuilt_message)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{OrcaConfig, LlmConfig};

    #[test]
    fn test_provider_from_config_missing_api_key() {
        let config = OrcaConfig {
            llm: LlmConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                api_key: None,
                api_base: None,
                temperature: 0.7,
                max_tokens: 1000,
            },
            ..Default::default()
        };

        let result = LlmProvider::from_config(&config);
        assert!(result.is_err());
        let error = result.unwrap_err();
        println!("Error: {}", error);
        assert!(error.to_string().contains("API key"));
    }

    #[test]
    fn test_provider_from_config_unsupported() {
        let config = OrcaConfig {
            llm: LlmConfig {
                provider: "unsupported".to_string(),
                model: "model".to_string(),
                api_key: Some("key".to_string()),
                api_base: None,
                temperature: 0.7,
                max_tokens: 1000,
            },
            ..Default::default()
        };

        let result = LlmProvider::from_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[test]
    fn test_provider_creation_openai() {
        let config = OrcaConfig {
            llm: LlmConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                api_key: Some("test-key".to_string()),
                api_base: None,
                temperature: 0.7,
                max_tokens: 1000,
            },
            ..Default::default()
        };

        let result = LlmProvider::from_config(&config);
        assert!(result.is_ok());
    }
}
