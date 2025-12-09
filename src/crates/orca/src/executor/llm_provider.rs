//! LLM Provider Integration - Wraps llm crate providers as LlmFunction
//!
//! Bridges the llm crate's ChatModel implementations to the LlmFunction type
//! expected by langgraph-prebuilt agents.
//!
//! LLM configuration is now database-only (llm_providers and llm_profiles tables).
//! Use `from_params` to create providers with explicit parameters.

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
    /// Create an LLM provider from explicit parameters
    ///
    /// LLM configuration is now database-only (llm_providers and llm_profiles tables).
    /// Callers should load configuration from the database and pass explicit parameters.
    ///
    /// # Arguments
    /// * `provider` - Provider type: "openai", "anthropic", "claude", "ollama", etc.
    /// * `model` - Model name (e.g., "gpt-4", "claude-sonnet-4-20250514")
    /// * `api_key` - API key (None for local providers like Ollama)
    /// * `api_base` - Custom API base URL (None for defaults)
    ///
    /// # Returns
    /// An LlmProvider instance
    pub fn from_params(
        provider: &str,
        model: &str,
        api_key: Option<&str>,
        api_base: Option<&str>,
    ) -> Result<Self> {
        let provider_lower = provider.to_lowercase();

        match provider_lower.as_str() {
            "ollama" => {
                let local_config = LocalLlmConfig::new(
                    api_base.unwrap_or("http://localhost:11434").to_string(),
                    model.to_string(),
                );
                Ok(Self::Ollama(llm::local::OllamaClient::new(local_config)))
            }

            "openai" => {
                let api_key = api_key
                    .ok_or_else(|| OrcaError::Config("OpenAI API key not configured".to_string()))?;

                let remote_config = RemoteLlmConfig::new(
                    api_key.to_string(),
                    api_base.unwrap_or("https://api.openai.com/v1").to_string(),
                    model.to_string(),
                );
                Ok(Self::OpenAI(llm::remote::OpenAiClient::new(remote_config)))
            }

            "anthropic" | "claude" => {
                let api_key = api_key
                    .ok_or_else(|| OrcaError::Config("Anthropic API key not configured".to_string()))?;

                let remote_config = RemoteLlmConfig::new(
                    api_key.to_string(),
                    api_base.unwrap_or("https://api.anthropic.com").to_string(),
                    model.to_string(),
                );
                Ok(Self::Claude(llm::remote::ClaudeClient::new(remote_config)))
            }

            "deepseek" => {
                let api_key = api_key
                    .ok_or_else(|| OrcaError::Config("Deepseek API key not configured".to_string()))?;

                let remote_config = RemoteLlmConfig::new(
                    api_key.to_string(),
                    api_base.unwrap_or("https://api.deepseek.com").to_string(),
                    model.to_string(),
                );
                Ok(Self::Deepseek(llm::remote::DeepseekClient::new(remote_config)))
            }

            "grok" | "xai" => {
                let api_key = api_key
                    .ok_or_else(|| OrcaError::Config("Grok API key not configured".to_string()))?;

                let remote_config = RemoteLlmConfig::new(
                    api_key.to_string(),
                    api_base.unwrap_or("https://api.x.ai").to_string(),
                    model.to_string(),
                );
                Ok(Self::Grok(llm::remote::GrokClient::new(remote_config)))
            }

            "openrouter" => {
                let api_key = api_key
                    .ok_or_else(|| OrcaError::Config("OpenRouter API key not configured".to_string()))?;

                let remote_config = RemoteLlmConfig::new(
                    api_key.to_string(),
                    api_base.unwrap_or("https://openrouter.ai/api/v1").to_string(),
                    model.to_string(),
                );
                Ok(Self::OpenRouter(llm::remote::OpenRouterClient::new(remote_config)))
            }

            _ => Err(OrcaError::Config(format!(
                "Unsupported LLM provider: {}. Available: ollama, openai, claude, deepseek, grok, openrouter",
                provider
            ))),
        }
    }

    /// Create an LLM provider from LlmProviderConfig (database model)
    ///
    /// This is the preferred way to create providers when loading from the database.
    pub fn from_provider_config(config: &crate::models::LlmProviderConfig) -> Result<Self> {
        Self::from_params(
            &config.provider_type,
            &config.model,
            config.api_key.as_deref(),
            config.api_base.as_deref(),
        )
    }

    /// Call the LLM with a chat request
    pub async fn chat(&self, _request: ChatRequest) -> llm::Result<llm::ChatResponse> {
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

    /// Send a simple prompt to the LLM and get a text response
    ///
    /// # Arguments
    /// * `prompt` - The user prompt to send
    ///
    /// # Returns
    /// The assistant's response as a String
    pub async fn send_prompt(&self, prompt: &str) -> Result<String> {
        if prompt.is_empty() {
            return Err(OrcaError::Config("Prompt cannot be empty".to_string()));
        }

        debug!("Sending prompt to LLM: {}...", &prompt[..prompt.len().min(50)]);

        // Create a simple chat request with a human message
        let message = langgraph_core::Message::human(prompt);
        let request = ChatRequest::new(vec![message]);

        // Call the LLM
        let response = self.chat(request).await.map_err(|e| {
            warn!("LLM call failed: {}", e);
            OrcaError::Execution(format!("LLM call failed: {}", e))
        })?;

        // Extract the response text
        let response_text = response.message.text().unwrap_or("").to_string();

        if !response_text.is_empty() {
            debug!("Received response: {}...", &response_text[..response_text.len().min(50)]);
        }

        Ok(response_text)
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
            debug!("LLM function async block started");

            // Extract messages from state (they are langgraph_prebuilt::Message)
            let prebuilt_messages: Vec<Message> = state
                .get("messages")
                .and_then(|m| m.as_array())
                .ok_or_else(|| {
                    warn!("No messages array in state");
                    langgraph_prebuilt::error::PrebuiltError::ToolExecution(
                        "No messages in state".to_string()
                    )
                })?
                .iter()
                .filter_map(|msg_val| {
                    match serde_json::from_value::<Message>(msg_val.clone()) {
                        Ok(msg) => Some(msg),
                        Err(e) => {
                            warn!("Failed to parse message: {}", e);
                            None
                        }
                    }
                })
                .collect();

            if prebuilt_messages.is_empty() {
                warn!("No valid messages found after parsing");
                return Err(langgraph_prebuilt::error::PrebuiltError::ToolExecution(
                    "No valid messages found in state".to_string()
                ));
            }

            debug!("Calling LLM with {} messages", prebuilt_messages.len());

            // Convert langgraph_prebuilt::Message to langgraph_core::Message
            // Manual conversion required because the structs are incompatible:
            // - prebuilt has message_type (serializes as "type"), core has role
            // - prebuilt has content: String, core has content: MessageContent enum
            let core_messages: Vec<langgraph_core::Message> = prebuilt_messages
                .into_iter()
                .map(|msg| {

                    // Convert MessageType to MessageRole
                    let role = match msg.message_type {
                        langgraph_prebuilt::messages::MessageType::System => langgraph_core::MessageRole::System,
                        langgraph_prebuilt::messages::MessageType::Human => langgraph_core::MessageRole::Human,
                        langgraph_prebuilt::messages::MessageType::AI => langgraph_core::MessageRole::Assistant,
                        langgraph_prebuilt::messages::MessageType::Tool => langgraph_core::MessageRole::Tool,
                        langgraph_prebuilt::messages::MessageType::Function => langgraph_core::MessageRole::Tool,
                    };

                    // Convert content String to MessageContent::Text
                    let content = langgraph_core::MessageContent::Text(msg.content.clone());

                    // Convert tool_calls if present
                    let tool_calls = msg.tool_calls.as_ref().map(|calls| {
                        calls.iter().map(|tc| {
                            langgraph_core::ToolCall {
                                id: tc.id.clone(),
                                name: tc.name.clone(),
                                args: tc.args.clone(),
                            }
                        }).collect()
                    });

                    // Create the core message
                    langgraph_core::Message {
                        id: None,
                        role,
                        content,
                        name: msg.name.clone(),
                        tool_calls,
                        tool_call_id: msg.tool_call_id.clone(),
                        metadata: None,
                    }
                })
                .collect();


            // Create chat request
            let request = ChatRequest::new(core_messages);

            // Call LLM
            debug!("About to call LLM provider");
            let response = provider
                .chat(request)
                .await
                .map_err(|e| {
                    warn!("LLM call failed: {}", e);
                    langgraph_prebuilt::error::PrebuiltError::ToolExecution(
                        format!("LLM call failed: {}", e)
                    )
                })?;

            debug!("LLM response received, converting message");

            // Convert response message from langgraph_core::Message to langgraph_prebuilt::Message
            // Manual conversion required (same as forward conversion)
            let core_message = response.message;

            // Convert MessageRole to MessageType
            let message_type = match core_message.role {
                langgraph_core::MessageRole::System => langgraph_prebuilt::messages::MessageType::System,
                langgraph_core::MessageRole::Human => langgraph_prebuilt::messages::MessageType::Human,
                langgraph_core::MessageRole::Assistant => langgraph_prebuilt::messages::MessageType::AI,
                langgraph_core::MessageRole::Tool => langgraph_prebuilt::messages::MessageType::Tool,
                langgraph_core::MessageRole::Custom(_) => langgraph_prebuilt::messages::MessageType::AI,  // Default to AI
            };

            // Convert MessageContent to String
            let content = match core_message.content {
                langgraph_core::MessageContent::Text(text) => text,
                langgraph_core::MessageContent::Parts(parts) => {
                    // For Parts, join all text parts
                    parts.iter()
                        .filter_map(|part| {
                            if let langgraph_core::ContentPart::Text { text, .. } = part {
                                Some(text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            };

            // Convert tool_calls if present
            let tool_calls = core_message.tool_calls.as_ref().map(|calls| {
                calls.iter().map(|tc| {
                    langgraph_prebuilt::ToolCall {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        args: tc.args.clone(),
                        call_type: "tool_call".to_string(),
                    }
                }).collect()
            });

            let prebuilt_message = Message {
                message_type,
                content,
                name: core_message.name,
                tool_call_id: core_message.tool_call_id,
                tool_calls,
                metadata: std::collections::HashMap::new(),
            };

            debug!("Successfully converted message to prebuilt Message, returning from LLM function");
            Ok(prebuilt_message)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_params_missing_api_key() {
        let result = LlmProvider::from_params("openai", "gpt-4", None, None);
        assert!(result.is_err());
        let error = result.unwrap_err();
        println!("Error: {}", error);
        assert!(error.to_string().contains("API key"));
    }

    #[test]
    fn test_provider_from_params_unsupported() {
        let result = LlmProvider::from_params("unsupported", "model", Some("key"), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[test]
    fn test_provider_creation_openai() {
        let result = LlmProvider::from_params("openai", "gpt-4", Some("test-key"), None);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_prompt_empty_returns_error() {
        let provider = LlmProvider::from_params("ollama", "llama2", None, None).unwrap();
        let result = provider.send_prompt("").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_provider_creation_claude() {
        let result = LlmProvider::from_params("claude", "claude-3-sonnet", Some("test-key"), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_creation_ollama() {
        let result = LlmProvider::from_params("ollama", "llama2", None, Some("http://localhost:11434"));
        assert!(result.is_ok());
    }
}
