//! Google Gemini client implementation.
//!
//! Provides integration with Google's Gemini models via the Gemini API.
//! Supports Gemini Pro, Gemini Pro Vision, and other Gemini models.
//!
//! # Example
//!
//! ```rust,ignore
//! use llm::remote::GeminiClient;
//! use llm::config::RemoteLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! let config = RemoteLlmConfig::from_env(
//!     "GOOGLE_API_KEY",
//!     "https://generativelanguage.googleapis.com/v1beta",
//!     "gemini-pro"
//! )?;
//! let client = GeminiClient::new(config);
//!
//! let request = ChatRequest::new(vec![Message::human("Hello!")]);
//! let response = client.chat(request).await?;
//! ```

use crate::config::RemoteLlmConfig;
use crate::error::LlmError;
use async_trait::async_trait;
use langgraph_core::error::Result as GraphResult;
use langgraph_core::llm::{
    ChatModel, ChatRequest, ChatResponse, ChatStreamResponse, UsageMetadata,
};
use langgraph_core::{Message, MessageContent, MessageRole};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Google Gemini API client.
#[derive(Clone)]
pub struct GeminiClient {
    config: RemoteLlmConfig,
    client: Client,
}

impl GeminiClient {
    /// Create a new Gemini client with the given configuration.
    pub fn new(config: RemoteLlmConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Convert langgraph messages to Gemini format.
    /// Gemini uses a different structure with roles and parts.
    fn convert_messages(&self, messages: &[Message]) -> Vec<GeminiMessage> {
        let mut gemini_messages = Vec::new();
        let mut system_instruction = None;

        for msg in messages {
            match &msg.role {
                MessageRole::System => {
                    // Gemini handles system messages as system instructions
                    system_instruction = Some(msg.text().unwrap_or("").to_string());
                }
                MessageRole::Human => {
                    gemini_messages.push(GeminiMessage {
                        role: "user".to_string(),
                        parts: vec![GeminiPart {
                            text: msg.text().unwrap_or("").to_string(),
                        }],
                    });
                }
                MessageRole::Assistant => {
                    gemini_messages.push(GeminiMessage {
                        role: "model".to_string(),
                        parts: vec![GeminiPart {
                            text: msg.text().unwrap_or("").to_string(),
                        }],
                    });
                }
                MessageRole::Tool => {
                    // Tool results are added as user messages with context
                    gemini_messages.push(GeminiMessage {
                        role: "user".to_string(),
                        parts: vec![GeminiPart {
                            text: format!("[Tool Result] {}", msg.text().unwrap_or("")),
                        }],
                    });
                }
                MessageRole::Custom(role) => {
                    gemini_messages.push(GeminiMessage {
                        role: role.clone(),
                        parts: vec![GeminiPart {
                            text: msg.text().unwrap_or("").to_string(),
                        }],
                    });
                }
            }
        }

        // If we have a system instruction, prepend it as a user message
        if let Some(instruction) = system_instruction {
            gemini_messages.insert(
                0,
                GeminiMessage {
                    role: "user".to_string(),
                    parts: vec![GeminiPart {
                        text: format!("[System] {}", instruction),
                    }],
                },
            );
        }

        gemini_messages
    }

    /// Convert Gemini response to ChatResponse.
    fn convert_response(&self, gemini_resp: GeminiResponse) -> ChatResponse {
        let candidate = &gemini_resp.candidates[0];
        
        let content_text = candidate
            .content
            .parts
            .iter()
            .map(|p| p.text.clone())
            .collect::<Vec<_>>()
            .join("");

        let message = Message {
            id: None,
            role: MessageRole::Assistant,
            content: MessageContent::Text(content_text),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        };

        let usage = gemini_resp.usage_metadata.as_ref().map(|u| {
            UsageMetadata::new(u.prompt_token_count, u.candidates_token_count)
        });

        let mut metadata = HashMap::new();
        metadata.insert(
            "model".to_string(),
            serde_json::Value::String(self.config.model.clone()),
        );
        if let Some(finish_reason) = &candidate.finish_reason {
            metadata.insert(
                "finish_reason".to_string(),
                serde_json::Value::String(finish_reason.clone()),
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
impl ChatModel for GeminiClient {
    async fn chat(&self, request: ChatRequest) -> GraphResult<ChatResponse> {
        // Gemini API URL format: base_url/models/{model}:generateContent
        let url = format!(
            "{}/models/{}:generateContent",
            self.config.base_url, self.config.model
        );

        let contents = self.convert_messages(&request.messages);

        let mut generation_config = GeminiGenerationConfig {
            temperature: request.config.temperature,
            max_output_tokens: request.config.max_tokens,
            top_p: request.config.top_p,
            stop_sequences: if request.config.stop_sequences.is_empty() {
                None
            } else {
                Some(request.config.stop_sequences.clone())
            },
        };

        let req_body = GeminiRequest {
            contents,
            generation_config: Some(generation_config),
        };

        // Gemini uses API key as query parameter
        let response = self
            .client
            .post(&url)
            .query(&[("key", &self.config.api_key)])
            .json(&req_body)
            .send()
            .await
            .map_err(|e| LlmError::HttpError(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            return Err(if status.as_u16() == 401 || status.as_u16() == 403 {
                LlmError::AuthenticationError(error_text)
            } else if status.as_u16() == 429 {
                LlmError::RateLimitExceeded(error_text)
            } else {
                LlmError::ProviderError(format!("Gemini API error {}: {}", status, error_text))
            }
            .into());
        }

        let gemini_resp: GeminiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(self.convert_response(gemini_resp))
    }

    async fn stream(&self, _request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        // TODO: Implement streaming support
        Err(LlmError::Other("Streaming not yet implemented for Gemini".to_string()).into())
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(self.clone())
    }
}

// Gemini API types
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiMessage {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
    role: String,
}

#[derive(Debug, Deserialize)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: usize,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: usize,
    #[serde(rename = "totalTokenCount")]
    total_token_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://generativelanguage.googleapis.com/v1beta",
            "gemini-pro",
        );
        let _client = GeminiClient::new(config);
    }

    #[test]
    fn test_message_conversion() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://generativelanguage.googleapis.com/v1beta",
            "gemini-pro",
        );
        let client = GeminiClient::new(config);

        let messages = vec![
            Message::system("You are helpful"),
            Message::human("Hello"),
        ];

        let gemini_msgs = client.convert_messages(&messages);

        // System message is converted to user message with [System] prefix
        assert_eq!(gemini_msgs.len(), 2);
        assert_eq!(gemini_msgs[0].role, "user");
        assert!(gemini_msgs[0].parts[0].text.starts_with("[System]"));
        assert_eq!(gemini_msgs[1].role, "user");
        assert_eq!(gemini_msgs[1].parts[0].text, "Hello");
    }
}

