//! llama.cpp server client implementation.
//!
//! Provides integration with llama.cpp's built-in HTTP server.
//! The llama.cpp server provides an OpenAI-compatible API.
//!
//! # Example
//!
//! ```rust,ignore
//! use llm::local::LlamaCppClient;
//! use llm::config::LocalLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! let config = LocalLlmConfig::new("http://localhost:8080", "llama-model");
//! let client = LlamaCppClient::new(config);
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

/// llama.cpp server client for local LLM inference.
#[derive(Clone)]
pub struct LlamaCppClient {
    config: LocalLlmConfig,
    client: Client,
    current_model: String,
}

impl LlamaCppClient {
    /// Create a new llama.cpp client with the given configuration.
    pub fn new(config: LocalLlmConfig) -> Self {
        let current_model = config.model.clone();
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client, current_model }
    }

    /// Check if llama.cpp server is running.
    pub async fn check_health(&self) -> Result<bool> {
        let url = format!("{}/health", self.config.base_url);
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Convert langgraph Message to llama.cpp message format.
    fn convert_message(&self, msg: &Message) -> LlamaCppMessage {
        LlamaCppMessage {
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

    /// Convert llama.cpp response to ChatResponse.
    fn convert_response(&self, cpp_resp: LlamaCppResponse) -> ChatResponse {
        let choice = &cpp_resp.choices[0];
        
        let message = Message {
            id: None,
            role: MessageRole::Assistant,
            content: MessageContent::Text(choice.message.content.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        };

        let usage = cpp_resp.usage.as_ref().map(|u| UsageMetadata {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            reasoning_tokens: None,
            total_tokens: u.total_tokens,
        });

        let mut metadata = HashMap::new();
        metadata.insert(
            "model".to_string(),
            serde_json::Value::String(cpp_resp.model),
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
impl ChatModel for LlamaCppClient {
    async fn chat(&self, request: ChatRequest) -> GraphResult<ChatResponse> {
        let url = format!("{}/v1/chat/completions", self.config.base_url);

        let messages: Vec<LlamaCppMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect();

        let req_body = LlamaCppRequest {
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
                "llama.cpp API error {}: {}",
                status, error_text
            ))
            .into());
        }

        let cpp_resp: LlamaCppResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(self.convert_response(cpp_resp))
    }

    async fn stream(&self, _request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        // TODO: Implement streaming support
        Err(LlmError::Other("Streaming not yet implemented for llama.cpp".to_string()).into())
    }

    async fn is_available(&self) -> GraphResult<bool> {
        Ok(self.check_health().await.unwrap_or(false))
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(self.clone())
    }
}

// llama.cpp API types (OpenAI-compatible format)
#[derive(Debug, Serialize)]
struct LlamaCppRequest {
    model: String,
    messages: Vec<LlamaCppMessage>,
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
struct LlamaCppMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct LlamaCppResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<LlamaCppChoice>,
    usage: Option<LlamaCppUsage>,
}

#[derive(Debug, Deserialize)]
struct LlamaCppChoice {
    index: usize,
    message: LlamaCppMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LlamaCppUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[async_trait]
impl ProviderUtils for LlamaCppClient {
    async fn ping(&self) -> Result<bool> {
        self.check_health().await
    }

    async fn fetch_models(&self) -> Result<Vec<ModelInfo>> {
        // llama.cpp typically runs a single model
        // Return the current model info
        Ok(vec![ModelInfo::new(&self.current_model)])
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
        let config = LocalLlmConfig::new("http://localhost:8080", "llama-model");
        let _client = LlamaCppClient::new(config);
    }

    #[test]
    fn test_message_conversion() {
        let config = LocalLlmConfig::new("http://localhost:8080", "llama-model");
        let client = LlamaCppClient::new(config);

        let msg = Message::human("Hello");
        let cpp_msg = client.convert_message(&msg);

        assert_eq!(cpp_msg.role, "user");
        assert_eq!(cpp_msg.content, "Hello");
    }

    #[test]
    fn test_current_model() {
        let config = LocalLlmConfig::new("http://localhost:8080", "llama-model");
        let client = LlamaCppClient::new(config);
        assert_eq!(client.current_model(), "llama-model");
    }
}

