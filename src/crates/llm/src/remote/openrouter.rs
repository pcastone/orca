//! OpenRouter client implementation.
//!
//! Provides integration with OpenRouter, a unified API that routes requests
//! to multiple LLM providers (OpenAI, Anthropic, Google, Meta, etc.).
//!
//! OpenRouter uses an OpenAI-compatible API format with additional routing features.
//!
//! # Example
//!
//! ```rust,ignore
//! use llm::remote::OpenRouterClient;
//! use llm::config::RemoteLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! let config = RemoteLlmConfig::from_env(
//!     "OPENROUTER_API_KEY",
//!     "https://openrouter.ai/api/v1",
//!     "anthropic/claude-3-opus"
//! )?;
//! let client = OpenRouterClient::new(config);
//!
//! let request = ChatRequest::new(vec![Message::human("Hello!")]);
//! let response = client.chat(request).await?;
//! ```

use crate::config::RemoteLlmConfig;
use crate::error::LlmError;
use crate::streaming;
use async_trait::async_trait;
use langgraph_core::error::Result as GraphResult;
use langgraph_core::llm::{
    ChatModel, ChatRequest, ChatResponse, ChatStreamResponse, UsageMetadata,
};
use langgraph_core::{Message, MessageContent, MessageRole};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// OpenRouter API client.
#[derive(Clone)]
pub struct OpenRouterClient {
    config: RemoteLlmConfig,
    client: Client,
    app_name: Option<String>,
}

impl OpenRouterClient {
    /// Create a new OpenRouter client with the given configuration.
    pub fn new(config: RemoteLlmConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            app_name: None,
        }
    }

    /// Set the application name for OpenRouter tracking.
    pub fn with_app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name = Some(app_name.into());
        self
    }

    /// Convert langgraph Message to OpenRouter message format.
    fn convert_message(&self, msg: &Message) -> OpenRouterMessage {
        OpenRouterMessage {
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

    /// Convert OpenRouter response to ChatResponse.
    fn convert_response(&self, router_resp: OpenRouterResponse) -> ChatResponse {
        let choice = &router_resp.choices[0];

        let message = Message {
            id: Some(router_resp.id.clone()),
            role: MessageRole::Assistant,
            content: MessageContent::Text(choice.message.content.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        };

        let usage = router_resp.usage.as_ref().map(|u| {
            UsageMetadata::new(u.prompt_tokens, u.completion_tokens)
        });

        let mut metadata = HashMap::new();
        metadata.insert(
            "model".to_string(),
            serde_json::Value::String(router_resp.model),
        );
        metadata.insert(
            "finish_reason".to_string(),
            serde_json::Value::String(choice.finish_reason.clone().unwrap_or_default()),
        );
        
        // OpenRouter-specific metadata
        if let Some(provider) = router_resp.provider.as_ref() {
            metadata.insert(
                "provider".to_string(),
                serde_json::Value::String(provider.clone()),
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
impl ChatModel for OpenRouterClient {
    async fn chat(&self, request: ChatRequest) -> GraphResult<ChatResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let messages: Vec<OpenRouterMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect();

        let req_body = OpenRouterRequest {
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

        let mut req = self.client.post(&url).json(&req_body);

        // Add authorization header
        req = req.header("Authorization", format!("Bearer {}", self.config.api_key));

        // Add OpenRouter-specific headers
        if let Some(app_name) = &self.app_name {
            req = req.header("HTTP-Referer", app_name);
            req = req.header("X-Title", app_name);
        }

        let response = req
            .send()
            .await
            .map_err(|e| LlmError::HttpError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            return Err(if status.as_u16() == 401 {
                LlmError::AuthenticationError(error_text)
            } else if status.as_u16() == 429 {
                LlmError::RateLimitExceeded(error_text)
            } else {
                LlmError::ProviderError(format!("OpenRouter API error {}: {}", status, error_text))
            }
            .into());
        }

        let router_resp: OpenRouterResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(self.convert_response(router_resp))
    }

    async fn stream(&self, request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        // Convert messages
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                let msg = self.convert_message(m);
                json!({
                    "role": msg.role,
                    "content": msg.content,
                })
            })
            .collect();

        // Build request body with stream: true
        let mut req_body = json!({
            "model": self.config.model,
            "messages": messages,
            "stream": true,
        });

        // Add optional parameters
        if let Some(temp) = request.config.temperature {
            req_body["temperature"] = json!(temp);
        }
        if let Some(max) = request.config.max_tokens {
            req_body["max_tokens"] = json!(max);
        }
        if let Some(top_p) = request.config.top_p {
            req_body["top_p"] = json!(top_p);
        }

        // Build headers
        let mut headers = vec![("Authorization", format!("Bearer {}", self.config.api_key))];
        let app_header;
        if let Some(ref app_name) = self.app_name {
            app_header = app_name.clone();
            headers.push(("HTTP-Referer", app_header.clone()));
            headers.push(("X-Title", app_header.clone()));
        }

        // Convert headers to borrowed string slices
        let header_refs: Vec<(&str, &str)> = headers
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

        // Use common streaming helper
        let (content_stream, reasoning_stream) =
            streaming::stream_openai_compatible(&self.client, &url, req_body, header_refs)
                .await?;

        Ok(ChatStreamResponse {
            stream: content_stream,
            reasoning_stream,
            usage: None,
            metadata: HashMap::new(),
        })
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(self.clone())
    }
}

// OpenRouter API types (OpenAI-compatible with extensions)
#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
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
struct OpenRouterMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    id: String,
    model: String,
    choices: Vec<OpenRouterChoice>,
    usage: Option<OpenRouterUsage>,
    #[serde(default)]
    provider: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    index: usize,
    message: OpenRouterMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://openrouter.ai/api/v1",
            "anthropic/claude-3-opus",
        );
        let _client = OpenRouterClient::new(config);
    }

    #[test]
    fn test_client_with_app_name() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://openrouter.ai/api/v1",
            "anthropic/claude-3-opus",
        );
        let client = OpenRouterClient::new(config).with_app_name("my-app");
        assert!(client.app_name.is_some());
        assert_eq!(client.app_name.unwrap(), "my-app");
    }

    #[test]
    fn test_message_conversion() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://openrouter.ai/api/v1",
            "anthropic/claude-3-opus",
        );
        let client = OpenRouterClient::new(config);

        let msg = Message::human("Hello");
        let router_msg = client.convert_message(&msg);

        assert_eq!(router_msg.role, "user");
        assert_eq!(router_msg.content, "Hello");
    }

    #[test]
    fn test_message_conversion_all_roles() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://openrouter.ai/api/v1",
            "anthropic/claude-3-opus",
        );
        let client = OpenRouterClient::new(config);

        // System message
        let sys_msg = Message::system("System prompt");
        assert_eq!(client.convert_message(&sys_msg).role, "system");

        // Human message
        let human_msg = Message::human("User message");
        assert_eq!(client.convert_message(&human_msg).role, "user");

        // Assistant message
        let asst_msg = Message::assistant("AI response");
        assert_eq!(client.convert_message(&asst_msg).role, "assistant");

        // Tool message (converts to user)
        let mut tool_msg = Message::human("tool result");
        tool_msg.role = MessageRole::Tool;
        assert_eq!(client.convert_message(&tool_msg).role, "user");

        // Custom role
        let mut custom_msg = Message::human("custom");
        custom_msg.role = MessageRole::Custom("moderator".to_string());
        assert_eq!(client.convert_message(&custom_msg).role, "moderator");
    }

    #[test]
    fn test_response_conversion_basic() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://openrouter.ai/api/v1",
            "anthropic/claude-3-opus",
        );
        let client = OpenRouterClient::new(config);

        let router_response = OpenRouterResponse {
            id: "gen-123".to_string(),
            model: "anthropic/claude-3-opus".to_string(),
            choices: vec![OpenRouterChoice {
                index: 0,
                message: OpenRouterMessage {
                    role: "assistant".to_string(),
                    content: "Hello there!".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(OpenRouterUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
            provider: None,
        };

        let response = client.convert_response(router_response);

        assert_eq!(response.message.text(), Some("Hello there!"));
        assert_eq!(response.message.role, MessageRole::Assistant);
        assert_eq!(response.usage.as_ref().unwrap().input_tokens, 10);
        assert_eq!(response.usage.as_ref().unwrap().output_tokens, 20);
        assert!(response.metadata.contains_key("model"));
        assert!(response.metadata.contains_key("finish_reason"));
    }

    #[test]
    fn test_response_conversion_with_provider() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://openrouter.ai/api/v1",
            "anthropic/claude-3-opus",
        );
        let client = OpenRouterClient::new(config);

        let router_response = OpenRouterResponse {
            id: "gen-456".to_string(),
            model: "anthropic/claude-3-opus".to_string(),
            choices: vec![OpenRouterChoice {
                index: 0,
                message: OpenRouterMessage {
                    role: "assistant".to_string(),
                    content: "Response from specific provider".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(OpenRouterUsage {
                prompt_tokens: 15,
                completion_tokens: 25,
                total_tokens: 40,
            }),
            provider: Some("anthropic".to_string()),
        };

        let response = client.convert_response(router_response);

        assert_eq!(response.message.text(), Some("Response from specific provider"));
        assert!(response.metadata.contains_key("provider"));
        assert_eq!(
            response.metadata.get("provider").and_then(|v| v.as_str()),
            Some("anthropic")
        );
    }

    #[test]
    fn test_response_conversion_no_usage() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://openrouter.ai/api/v1",
            "anthropic/claude-3-opus",
        );
        let client = OpenRouterClient::new(config);

        let router_response = OpenRouterResponse {
            id: "gen-789".to_string(),
            model: "anthropic/claude-3-opus".to_string(),
            choices: vec![OpenRouterChoice {
                index: 0,
                message: OpenRouterMessage {
                    role: "assistant".to_string(),
                    content: "Response without usage".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
            provider: None,
        };

        let response = client.convert_response(router_response);

        assert!(response.usage.is_none());
        assert_eq!(response.message.text(), Some("Response without usage"));
    }

    #[test]
    fn test_config_with_custom_timeout() {
        use std::time::Duration;

        let mut config = RemoteLlmConfig::new(
            "test-key",
            "https://openrouter.ai/api/v1",
            "anthropic/claude-3-opus",
        );
        config.timeout = Duration::from_secs(120);

        let _client = OpenRouterClient::new(config);
        // Client should be created successfully with custom timeout
    }
}

