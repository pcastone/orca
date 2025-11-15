//! Anthropic Claude client implementation.
//!
//! Provides integration with Anthropic's Claude models:
//! - Claude 3 Opus
//! - Claude 3 Sonnet
//! - Claude 3 Haiku
//! - Claude 3.5 Sonnet
//!
//! # Example
//!
//! ```rust,ignore
//! use llm::remote::ClaudeClient;
//! use llm::config::RemoteLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! let config = RemoteLlmConfig::from_env(
//!     "ANTHROPIC_API_KEY",
//!     "https://api.anthropic.com",
//!     "claude-3-opus-20240229"
//! )?;
//! let client = ClaudeClient::new(config);
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

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic Claude API client.
#[derive(Clone)]
pub struct ClaudeClient {
    config: RemoteLlmConfig,
    client: Client,
}

impl ClaudeClient {
    /// Create a new Claude client with the given configuration.
    pub fn new(config: RemoteLlmConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Convert langgraph messages to Claude format.
    /// Claude requires system messages to be separate from conversation messages.
    fn convert_messages(&self, messages: &[Message]) -> (Option<String>, Vec<ClaudeMessage>) {
        let mut system_prompt = None;
        let mut claude_messages = Vec::new();

        for msg in messages {
            match &msg.role {
                MessageRole::System => {
                    // Combine all system messages
                    let content = msg.text().unwrap_or("");
                    system_prompt = Some(match system_prompt {
                        Some(existing) => format!("{}\n\n{}", existing, content),
                        None => content.to_string(),
                    });
                }
                MessageRole::Human => {
                    claude_messages.push(ClaudeMessage {
                        role: "user".to_string(),
                        content: msg.text().unwrap_or("").to_string(),
                    });
                }
                MessageRole::Assistant => {
                    claude_messages.push(ClaudeMessage {
                        role: "assistant".to_string(),
                        content: msg.text().unwrap_or("").to_string(),
                    });
                }
                MessageRole::Tool => {
                    // Tool messages are converted to user messages with context
                    claude_messages.push(ClaudeMessage {
                        role: "user".to_string(),
                        content: format!("[Tool Result] {}", msg.text().unwrap_or("")),
                    });
                }
                MessageRole::Custom(role) => {
                    claude_messages.push(ClaudeMessage {
                        role: role.clone(),
                        content: msg.text().unwrap_or("").to_string(),
                    });
                }
            }
        }

        (system_prompt, claude_messages)
    }

    /// Convert Claude response to ChatResponse.
    fn convert_response(&self, claude_resp: ClaudeResponse) -> ChatResponse {
        let content_text = claude_resp
            .content
            .iter()
            .filter_map(|c| {
                if c.content_type == "text" {
                    c.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        let message = Message {
            id: Some(claude_resp.id),
            role: MessageRole::Assistant,
            content: MessageContent::Text(content_text),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        };

        let usage = Some(UsageMetadata::new(
            claude_resp.usage.input_tokens,
            claude_resp.usage.output_tokens,
        ));

        let mut metadata = HashMap::new();
        metadata.insert(
            "model".to_string(),
            serde_json::Value::String(claude_resp.model),
        );
        metadata.insert(
            "stop_reason".to_string(),
            serde_json::Value::String(claude_resp.stop_reason.unwrap_or_default()),
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
impl ChatModel for ClaudeClient {
    async fn chat(&self, request: ChatRequest) -> GraphResult<ChatResponse> {
        let url = format!("{}/v1/messages", self.config.base_url);

        let (system, messages) = self.convert_messages(&request.messages);

        let req_body = ClaudeRequest {
            model: self.config.model.clone(),
            messages,
            system,
            max_tokens: request.config.max_tokens.unwrap_or(4096),
            temperature: request.config.temperature,
            top_p: request.config.top_p,
            stop_sequences: if request.config.stop_sequences.is_empty() {
                None
            } else {
                Some(request.config.stop_sequences.clone())
            },
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&req_body)
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
                LlmError::ProviderError(format!("Claude API error {}: {}", status, error_text))
            }
            .into());
        }

        let claude_resp: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(self.convert_response(claude_resp))
    }

    async fn stream(&self, _request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        // TODO: Implement streaming support
        Err(LlmError::Other("Streaming not yet implemented for Claude".to_string()).into())
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(self.clone())
    }
}

// Claude API types
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<ClaudeContent>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: ClaudeUsage,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: usize,
    output_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ============================================================
    // Existing Tests
    // ============================================================

    #[test]
    fn test_client_creation() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let _client = ClaudeClient::new(config);
    }

    #[test]
    fn test_message_conversion() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        let messages = vec![
            Message::system("You are helpful"),
            Message::human("Hello"),
        ];

        let (system, claude_msgs) = client.convert_messages(&messages);

        assert_eq!(system, Some("You are helpful".to_string()));
        assert_eq!(claude_msgs.len(), 1);
        assert_eq!(claude_msgs[0].role, "user");
        assert_eq!(claude_msgs[0].content, "Hello");
    }

    // ============================================================
    // Message Conversion Tests
    // ============================================================

    #[test]
    fn test_message_conversion_all_roles() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let messages = vec![
            Message::system("You are helpful"),
            Message::human("Hello"),
            Message::assistant("Hi there!"),
        ];

        let (system, claude_msgs) = client.convert_messages(&messages);

        assert_eq!(system, Some("You are helpful".to_string()));
        assert_eq!(claude_msgs.len(), 2);
        assert_eq!(claude_msgs[0].role, "user");
        assert_eq!(claude_msgs[0].content, "Hello");
        assert_eq!(claude_msgs[1].role, "assistant");
        assert_eq!(claude_msgs[1].content, "Hi there!");
    }

    #[test]
    fn test_message_conversion_multiple_system() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        let messages = vec![
            Message::system("First instruction"),
            Message::system("Second instruction"),
            Message::human("Question"),
        ];

        let (system, claude_msgs) = client.convert_messages(&messages);

        // Multiple system messages should be combined
        assert_eq!(system, Some("First instruction\n\nSecond instruction".to_string()));
        assert_eq!(claude_msgs.len(), 1);
        assert_eq!(claude_msgs[0].role, "user");
    }

    #[test]
    fn test_message_conversion_tool_result() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let mut tool_msg = Message::human("weather data");
        tool_msg.role = MessageRole::Tool;

        let messages = vec![tool_msg];

        let (system, claude_msgs) = client.convert_messages(&messages);

        assert_eq!(system, None);
        assert_eq!(claude_msgs.len(), 1);
        assert_eq!(claude_msgs[0].role, "user");
        assert_eq!(claude_msgs[0].content, "[Tool Result] weather data");
    }

    #[test]
    fn test_message_conversion_custom_role() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        let mut custom_msg = Message::human("custom content");
        custom_msg.role = MessageRole::Custom("moderator".to_string());

        let messages = vec![custom_msg];

        let (system, claude_msgs) = client.convert_messages(&messages);

        assert_eq!(system, None);
        assert_eq!(claude_msgs.len(), 1);
        assert_eq!(claude_msgs[0].role, "moderator");
        assert_eq!(claude_msgs[0].content, "custom content");
    }

    // ============================================================
    // Response Conversion Tests
    // ============================================================

    #[test]
    fn test_response_conversion_basic() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let claude_response = ClaudeResponse {
            id: "msg_123".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![ClaudeContent {
                content_type: "text".to_string(),
                text: Some("Hello there!".to_string()),
            }],
            model: "claude-3-sonnet-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 12,
                output_tokens: 25,
            },
        };

        let response = client.convert_response(claude_response);

        assert_eq!(response.message.text(), Some("Hello there!"));
        assert_eq!(response.message.role, MessageRole::Assistant);
        assert_eq!(response.message.id, Some("msg_123".to_string()));
        assert_eq!(response.usage.as_ref().unwrap().input_tokens, 12);
        assert_eq!(response.usage.as_ref().unwrap().output_tokens, 25);
        assert!(response.metadata.contains_key("model"));
        assert!(response.metadata.contains_key("stop_reason"));
    }

    #[test]
    fn test_response_conversion_multiple_content_blocks() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        let claude_response = ClaudeResponse {
            id: "msg_456".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![
                ClaudeContent {
                    content_type: "text".to_string(),
                    text: Some("First part. ".to_string()),
                },
                ClaudeContent {
                    content_type: "text".to_string(),
                    text: Some("Second part.".to_string()),
                },
            ],
            model: "claude-3-opus-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 10,
                output_tokens: 20,
            },
        };

        let response = client.convert_response(claude_response);

        // Multiple text blocks should be concatenated
        assert_eq!(response.message.text(), Some("First part. Second part."));
    }

    #[test]
    fn test_response_conversion_with_stop_reason() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let claude_response = ClaudeResponse {
            id: "msg_789".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![ClaudeContent {
                content_type: "text".to_string(),
                text: Some("Response".to_string()),
            }],
            model: "claude-3-sonnet-20240229".to_string(),
            stop_reason: Some("max_tokens".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 5,
                output_tokens: 100,
            },
        };

        let response = client.convert_response(claude_response);

        let stop_reason = response.metadata.get("stop_reason").unwrap();
        assert_eq!(stop_reason, &serde_json::Value::String("max_tokens".to_string()));
    }

    // ============================================================
    // Configuration Tests
    // ============================================================

    #[test]
    fn test_config_with_custom_timeout() {
        let mut config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        config.timeout = Duration::from_secs(120);

        let client = ClaudeClient::new(config.clone());
        assert_eq!(client.config.timeout, Duration::from_secs(120));
    }

    #[test]
    fn test_anthropic_version_constant() {
        // Verify the API version is set correctly
        assert_eq!(ANTHROPIC_VERSION, "2023-06-01");
    }

    // ============================================================
    // Future Implementation Tests (Marked #[ignore])
    // ============================================================

    /// Test: Streaming support
    ///
    /// Verifies that Claude streaming returns token-by-token responses.
    ///
    /// NOTE: Currently ignored - streaming not yet implemented for Claude.
    /// See line 211-214 in chat implementation.
    #[tokio::test]
    #[ignore]
    async fn test_streaming_basic() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Tell me a story")]);

        // TODO: Currently returns error "Streaming not yet implemented"
        // Once implemented, should work like:
        // let stream = client.stream(request).await.unwrap();
        // while let Some(result) = stream.receiver.recv().await {
        //     match result {
        //         Ok(event) => { /* process streaming event */ },
        //         Err(_) => break,
        //     }
        // }

        // For now, just verify it returns an error
        let result = client.stream(request).await;
        assert!(result.is_err());
    }

    /// Test: Tool use functionality
    ///
    /// Verifies that Claude tool use works correctly.
    ///
    /// NOTE: Currently ignored - tool use not yet implemented in this client.
    #[tokio::test]
    #[ignore]
    async fn test_tool_use() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        let request = ChatRequest::new(vec![Message::human("What's the weather?")]);

        // TODO: Add tool definitions to request
        // request.config.tools = vec![...];

        // Once implemented, should handle tool calls in response
        let _response = client.chat(request).await;
        // assert!(response.is_ok());
        // let response = response.unwrap();
        // assert!(response.message.tool_calls.is_some());
    }

    /// Test: Vision support
    ///
    /// Verifies that Claude can process image inputs.
    ///
    /// NOTE: Currently ignored - vision/multi-modal support not yet implemented.
    #[tokio::test]
    #[ignore]
    async fn test_vision_support() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        // TODO: Create message with image content
        // let mut msg = Message::human("What's in this image?");
        // msg.content = MessageContent::Image(...);

        let request = ChatRequest::new(vec![Message::human("image description")]);

        // Once implemented, should handle image content
        let _response = client.chat(request).await;
        // assert!(response.is_ok());
    }

    /// Test: Extended thinking / thinking tags extraction
    ///
    /// Verifies that Claude extended thinking models properly expose reasoning.
    ///
    /// NOTE: Currently ignored - thinking tags extraction not yet implemented.
    #[tokio::test]
    #[ignore]
    async fn test_thinking_tags_extraction() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Complex reasoning task")]);

        // TODO: Once thinking tags are supported
        // let response = client.chat(request).await.unwrap();
        // if response.message.text().unwrap().contains("<thinking>") {
        //     assert!(response.reasoning.is_some());
        // }
    }
}

