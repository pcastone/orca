//! Ollama client implementation.
//!
//! Provides integration with Ollama, a popular local LLM runner.
//! Supports models like Llama 2, Mistral, Mixtral, and more.
//!
//! # Example
//!
//! ```rust,ignore
//! use llm::local::OllamaClient;
//! use llm::config::LocalLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
//! let client = OllamaClient::new(config);
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

/// Ollama client for local LLM inference.
#[derive(Clone)]
pub struct OllamaClient {
    config: LocalLlmConfig,
    client: Client,
    current_model: String,
}

impl OllamaClient {
    /// Create a new Ollama client with the given configuration.
    pub fn new(config: LocalLlmConfig) -> Self {
        let current_model = config.model.clone();
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client, current_model }
    }

    /// Check if Ollama server is running.
    pub async fn check_health(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.config.base_url);
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Convert langgraph Message to Ollama message format.
    fn convert_message(&self, msg: &Message) -> OllamaMessage {
        OllamaMessage {
            role: match &msg.role {
                MessageRole::System => "system".to_string(),
                MessageRole::Human => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::Tool => "user".to_string(), // Ollama doesn't have separate tool role
                MessageRole::Custom(role) => role.clone(),
            },
            content: msg.text().unwrap_or("").to_string(),
        }
    }

    /// Convert Ollama response to ChatResponse.
    fn convert_response(&self, ollama_resp: OllamaResponse) -> ChatResponse {
        let message = Message {
            id: None,
            role: MessageRole::Assistant,
            content: MessageContent::Text(ollama_resp.message.content),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        };

        let usage = if ollama_resp.prompt_eval_count.is_some() || ollama_resp.eval_count.is_some()
        {
            Some(UsageMetadata {
                input_tokens: ollama_resp.prompt_eval_count.unwrap_or(0),
                output_tokens: ollama_resp.eval_count.unwrap_or(0),
                reasoning_tokens: None,
                total_tokens: ollama_resp.prompt_eval_count.unwrap_or(0)
                    + ollama_resp.eval_count.unwrap_or(0),
            })
        } else {
            None
        };

        let mut metadata = HashMap::new();
        metadata.insert(
            "model".to_string(),
            serde_json::Value::String(ollama_resp.model),
        );
        if let Some(total_duration) = ollama_resp.total_duration {
            metadata.insert(
                "total_duration_ns".to_string(),
                serde_json::Value::Number(total_duration.into()),
            );
        }

        ChatResponse {
            message,
            usage,
            reasoning: None,
            metadata,
        }
    }
}

#[async_trait]
impl ChatModel for OllamaClient {
    async fn chat(&self, request: ChatRequest) -> GraphResult<ChatResponse> {
        let url = format!("{}/api/chat", self.config.base_url);

        let messages: Vec<OllamaMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect();

        let mut options = HashMap::new();
        if let Some(temp) = request.config.temperature {
            options.insert("temperature", serde_json::Value::from(temp));
        }
        if let Some(top_p) = request.config.top_p {
            options.insert("top_p", serde_json::Value::from(top_p));
        }

        let req_body = OllamaRequest {
            model: self.config.model.clone(),
            messages,
            stream: false,
            options: if options.is_empty() {
                None
            } else {
                Some(options)
            },
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
                "Ollama API error {}: {}",
                status, error_text
            ))
            .into());
        }

        let ollama_resp: OllamaResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(self.convert_response(ollama_resp))
    }

    async fn stream(&self, _request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        // TODO: Implement streaming support
        Err(LlmError::Other("Streaming not yet implemented for Ollama".to_string()).into())
    }

    async fn is_available(&self) -> GraphResult<bool> {
        Ok(self.check_health().await.unwrap_or(false))
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(self.clone())
    }
}

// Ollama API types
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<HashMap<&'static str, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    model: String,
    message: OllamaMessage,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    prompt_eval_count: Option<usize>,
    #[serde(default)]
    eval_count: Option<usize>,
}

#[async_trait]
impl ProviderUtils for OllamaClient {
    async fn ping(&self) -> Result<bool> {
        self.check_health().await
    }

    async fn fetch_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.config.base_url);
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(LlmError::ProviderError(
                "Failed to fetch models from Ollama".to_string()
            ));
        }

        #[derive(Deserialize)]
        struct OllamaModelsResponse {
            models: Vec<OllamaModelInfo>,
        }

        #[derive(Deserialize)]
        struct OllamaModelInfo {
            name: String,
            #[serde(default)]
            size: Option<u64>,
            #[serde(default)]
            modified_at: Option<String>,
        }

        let models_response: OllamaModelsResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        let models = models_response
            .models
            .into_iter()
            .map(|m| {
                let mut info = ModelInfo::new(&m.name).with_name(&m.name);
                
                if let Some(size) = m.size {
                    let size_gb = size as f64 / 1_000_000_000.0;
                    info.metadata.insert(
                        "size_gb".to_string(),
                        serde_json::Value::Number(serde_json::Number::from_f64(size_gb).unwrap()),
                    );
                }
                
                if let Some(modified) = m.modified_at {
                    info.metadata.insert(
                        "modified_at".to_string(),
                        serde_json::Value::String(modified),
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
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let _client = OllamaClient::new(config);
    }

    #[test]
    fn test_message_conversion() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let client = OllamaClient::new(config);

        let msg = Message::human("Hello");
        let ollama_msg = client.convert_message(&msg);

        assert_eq!(ollama_msg.role, "user");
        assert_eq!(ollama_msg.content, "Hello");
    }

    #[test]
    fn test_current_model() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let client = OllamaClient::new(config);
        assert_eq!(client.current_model(), "llama2");
    }
}

