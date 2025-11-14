//! LM Studio client implementation.
//!
//! Provides integration with LM Studio, a user-friendly local LLM interface.
//! LM Studio provides an OpenAI-compatible API endpoint.
//!
//! # Example
//!
//! ```rust,ignore
//! use llm::local::LmStudioClient;
//! use llm::config::LocalLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! let config = LocalLlmConfig::new("http://localhost:1234/v1", "local-model");
//! let client = LmStudioClient::new(config);
//!
//! let request = ChatRequest::new(vec![Message::human("Hello!")]);
//! let response = client.chat(request).await?;
//! ```

use crate::config::LocalLlmConfig;
use crate::error::{LlmError, Result};
use crate::provider_utils::{ModelInfo, ProviderUtils};
use async_trait::async_trait;
use langgraph_core::error::Result as GraphResult;
use langgraph_core::llm::{
    ChatModel, ChatRequest, ChatResponse, ChatStreamResponse, UsageMetadata,
};
use langgraph_core::{Message, MessageContent, MessageRole};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LM Studio client for local LLM inference.
///
/// LM Studio provides an OpenAI-compatible API, so this implementation
/// follows the OpenAI API format.
#[derive(Clone)]
pub struct LmStudioClient {
    config: LocalLlmConfig,
    client: Client,
    current_model: String,
}

impl LmStudioClient {
    /// Create a new LM Studio client with the given configuration.
    pub fn new(config: LocalLlmConfig) -> Self {
        let current_model = config.model.clone();
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client, current_model }
    }

    /// Check if LM Studio server is running.
    pub async fn check_health(&self) -> Result<bool> {
        let url = format!("{}/models", self.config.base_url);
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Convert langgraph Message to LM Studio message format.
    fn convert_message(&self, msg: &Message) -> LmStudioMessage {
        LmStudioMessage {
            role: match &msg.role {
                MessageRole::System => "system".to_string(),
                MessageRole::Human => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::Tool => "user".to_string(),
                MessageRole::Custom(role) => role.clone(),
            },
            content: msg.text().unwrap_or("").to_string(),
        }
    }

    /// Convert LM Studio response to ChatResponse.
    fn convert_response(&self, lms_resp: LmStudioResponse) -> ChatResponse {
        let choice = &lms_resp.choices[0];

        let message = Message {
            id: None,
            role: MessageRole::Assistant,
            content: MessageContent::Text(choice.message.content.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        };

        let usage = lms_resp.usage.as_ref().map(|u| UsageMetadata {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            reasoning_tokens: None,
            total_tokens: u.total_tokens,
        });

        let mut metadata = HashMap::new();
        metadata.insert(
            "model".to_string(),
            serde_json::Value::String(lms_resp.model),
        );
        metadata.insert(
            "finish_reason".to_string(),
            serde_json::Value::String(choice.finish_reason.clone().unwrap_or_default()),
        );

        ChatResponse {
            message,
            usage,
            reasoning: None,
            metadata,
        }
    }
}

#[async_trait]
impl ChatModel for LmStudioClient {
    async fn chat(&self, request: ChatRequest) -> GraphResult<ChatResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let messages: Vec<LmStudioMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect();

        let req_body = LmStudioRequest {
            model: self.config.model.clone(),
            messages,
            temperature: request.config.temperature,
            max_tokens: request.config.max_tokens,
            top_p: request.config.top_p,
            frequency_penalty: request.config.frequency_penalty,
            presence_penalty: request.config.presence_penalty,
            stop: if request.config.stop_sequences.is_empty() {
                None
            } else {
                Some(request.config.stop_sequences.clone())
            },
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .json(&req_body)
            .send()
            .await
            .map_err(|e| LlmError::HttpError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ProviderError(format!(
                "LM Studio API error {}: {}",
                status, error_text
            ))
            .into());
        }

        let lms_resp: LmStudioResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(self.convert_response(lms_resp))
    }

    async fn stream(&self, _request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        // TODO: Implement streaming support
        Err(LlmError::Other("Streaming not yet implemented for LM Studio".to_string()).into())
    }

    async fn is_available(&self) -> GraphResult<bool> {
        Ok(self.check_health().await.unwrap_or(false))
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(self.clone())
    }
}

// LM Studio API types (OpenAI-compatible format)
#[derive(Debug, Serialize)]
struct LmStudioRequest {
    model: String,
    messages: Vec<LmStudioMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct LmStudioMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct LmStudioResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<LmStudioChoice>,
    usage: Option<LmStudioUsage>,
}

#[derive(Debug, Deserialize)]
struct LmStudioChoice {
    index: usize,
    message: LmStudioMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LmStudioUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[async_trait]
impl ProviderUtils for LmStudioClient {
    async fn ping(&self) -> Result<bool> {
        self.check_health().await
    }

    async fn fetch_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/models", self.config.base_url);
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            // If endpoint doesn't exist, return current model
            return Ok(vec![ModelInfo::new(&self.current_model)]);
        }

        #[derive(Deserialize)]
        struct ModelsResponse {
            data: Vec<ModelData>,
        }

        #[derive(Deserialize)]
        struct ModelData {
            id: String,
            #[serde(default)]
            owned_by: Option<String>,
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        let models = models_response
            .data
            .into_iter()
            .map(|m| {
                let mut info = ModelInfo::new(&m.id);
                if let Some(owned_by) = m.owned_by {
                    info.metadata.insert(
                        "owned_by".to_string(),
                        serde_json::Value::String(owned_by),
                    );
                }
                info
            })
            .collect();

        Ok(models)
    }

    async fn use_model(&mut self, model: impl Into<String> + Send) -> Result<String> {
        let model = model.into();
        self.current_model = model.clone();
        self.config.model = model.clone();
        Ok(model)
    }

    fn current_model(&self) -> &str {
        &self.current_model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = LocalLlmConfig::new("http://localhost:1234/v1", "local-model");
        let _client = LmStudioClient::new(config);
    }

    #[test]
    fn test_message_conversion() {
        let config = LocalLlmConfig::new("http://localhost:1234/v1", "local-model");
        let client = LmStudioClient::new(config);

        let msg = Message::human("Hello");
        let lms_msg = client.convert_message(&msg);

        assert_eq!(lms_msg.role, "user");
        assert_eq!(lms_msg.content, "Hello");
    }

    #[test]
    fn test_current_model() {
        let config = LocalLlmConfig::new("http://localhost:1234/v1", "local-model");
        let client = LmStudioClient::new(config);
        assert_eq!(client.current_model(), "local-model");
    }
}

