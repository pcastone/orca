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
        use langgraph_core::llm::ReasoningContent;
        use langgraph_core::ToolCall;

        // Extract thinking blocks
        let thinking_blocks: Vec<String> = claude_resp
            .content
            .iter()
            .filter_map(|c| {
                if c.content_type == "thinking" {
                    c.thinking.clone()
                } else {
                    None
                }
            })
            .collect();

        // Extract text blocks for the final answer
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

        // Extract tool_use blocks for tool calls
        let tool_calls: Vec<ToolCall> = claude_resp
            .content
            .iter()
            .filter_map(|c| {
                if c.content_type == "tool_use" {
                    // tool_use requires id, name, and input
                    match (c.id.as_ref(), c.name.as_ref(), c.input.as_ref()) {
                        (Some(id), Some(name), Some(input)) => {
                            Some(ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                                args: input.clone(),
                            })
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect();

        let message = Message {
            id: Some(claude_resp.id),
            role: MessageRole::Assistant,
            content: MessageContent::Text(content_text),
            name: None,
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            tool_call_id: None,
            metadata: None,
        };

        let usage = Some(UsageMetadata::new(
            claude_resp.usage.input_tokens,
            claude_resp.usage.output_tokens,
        ));

        // Create reasoning content if thinking blocks were present
        let reasoning = if !thinking_blocks.is_empty() {
            let combined_thinking = thinking_blocks.join("\n\n");
            let thinking_tokens = combined_thinking.split_whitespace().count();
            Some(ReasoningContent::new(combined_thinking).with_tokens(thinking_tokens))
        } else {
            None
        };

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
            reasoning,
            metadata,
        }
    }

    /// Convert ToolDefinition to Claude's tool format
    fn convert_tools(&self, tools: &[langgraph_core::llm::ToolDefinition]) -> Vec<ClaudeTool> {
        tools.iter().map(|t| ClaudeTool {
            name: t.name.clone(),
            description: t.description.clone(),
            input_schema: t.parameters.clone(),
        }).collect()
    }
}

#[async_trait]
impl ChatModel for ClaudeClient {
    async fn chat(&self, request: ChatRequest) -> GraphResult<ChatResponse> {
        let url = format!("{}/v1/messages", self.config.base_url);

        let (system, messages) = self.convert_messages(&request.messages);

        // Convert tools if provided
        let tools = if request.config.tools.is_empty() {
            None
        } else {
            Some(self.convert_tools(&request.config.tools))
        };

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
            tools,
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

    async fn stream(&self, request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        let url = format!("{}/v1/messages", self.config.base_url);

        let (system, messages) = self.convert_messages(&request.messages);

        // Convert messages to JSON for streaming
        let messages_json: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        // Build request body with stream: true
        let mut req_body = json!({
            "model": self.config.model,
            "messages": messages_json,
            "max_tokens": request.config.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        // Add system prompt if present
        if let Some(sys) = system {
            req_body["system"] = json!(sys);
        }

        // Add optional parameters
        if let Some(temp) = request.config.temperature {
            req_body["temperature"] = json!(temp);
        }
        if let Some(top_p) = request.config.top_p {
            req_body["top_p"] = json!(top_p);
        }
        if !request.config.stop_sequences.is_empty() {
            req_body["stop_sequences"] = json!(request.config.stop_sequences);
        }

        // Add tools if provided
        if !request.config.tools.is_empty() {
            let claude_tools: Vec<serde_json::Value> = request.config.tools.iter().map(|t| {
                let mut tool = json!({
                    "name": t.name,
                    "description": t.description,
                });
                if let Some(params) = &t.parameters {
                    tool["input_schema"] = params.clone();
                }
                tool
            }).collect();
            req_body["tools"] = json!(claude_tools);
        }

        // Use Claude streaming helper
        let (content_stream, reasoning_stream) = streaming::stream_claude(
            &self.client,
            &url,
            req_body,
            &self.config.api_key,
            ANTHROPIC_VERSION,
        )
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
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ClaudeTool>>,
    stream: bool,
}

/// Claude's tool definition format
#[derive(Debug, Serialize)]
struct ClaudeTool {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_schema: Option<serde_json::Value>,
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
    thinking: Option<String>,
    /// Tool use fields (for tool_use content blocks)
    id: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
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
                thinking: None,
                id: None,
                name: None,
                input: None,
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
                    thinking: None,
                    id: None,
                    name: None,
                    input: None,
                },
                ClaudeContent {
                    content_type: "text".to_string(),
                    text: Some("Second part.".to_string()),
                    thinking: None,
                    id: None,
                    name: None,
                    input: None,
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
                thinking: None,
                id: None,
                name: None,
                input: None,
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

    /// Test: Response conversion with thinking blocks
    ///
    /// Verifies that Claude extended thinking response properly extracts reasoning.
    #[test]
    fn test_response_conversion_with_thinking() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        // Simulate Claude response with thinking + text blocks
        let claude_resp = ClaudeResponse {
            id: "msg_123".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![
                ClaudeContent {
                    content_type: "thinking".to_string(),
                    text: None,
                    thinking: Some("Let me analyze this step by step. First, I need to...".to_string()),
                    id: None,
                    name: None,
                    input: None,
                },
                ClaudeContent {
                    content_type: "text".to_string(),
                    text: Some("The answer is 42.".to_string()),
                    thinking: None,
                    id: None,
                    name: None,
                    input: None,
                },
            ],
            model: "claude-3-opus-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 10,
                output_tokens: 50,
            },
        };

        let response = client.convert_response(claude_resp);

        // Verify thinking was extracted
        assert!(response.reasoning.is_some());
        let reasoning = response.reasoning.unwrap();
        assert_eq!(reasoning.content, "Let me analyze this step by step. First, I need to...");
        assert!(reasoning.tokens > 0);

        // Verify text was extracted separately
        assert_eq!(response.message.text().unwrap(), "The answer is 42.");
    }

    /// Test: Response conversion without thinking blocks
    ///
    /// Verifies normal responses without thinking work as before.
    #[test]
    fn test_response_conversion_without_thinking() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        // Simulate standard Claude response (no thinking)
        let claude_resp = ClaudeResponse {
            id: "msg_456".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![
                ClaudeContent {
                    content_type: "text".to_string(),
                    text: Some("Hello! How can I help you?".to_string()),
                    thinking: None,
                    id: None,
                    name: None,
                    input: None,
                },
            ],
            model: "claude-3-sonnet-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 5,
                output_tokens: 10,
            },
        };

        let response = client.convert_response(claude_resp);

        // Verify no thinking was extracted
        assert!(response.reasoning.is_none());

        // Verify text was extracted normally
        assert_eq!(response.message.text().unwrap(), "Hello! How can I help you?");
    }

    /// Test: Response conversion with multiple thinking blocks
    ///
    /// Verifies multiple thinking blocks are properly combined.
    #[test]
    fn test_response_conversion_multiple_thinking() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        // Simulate Claude response with multiple thinking blocks
        let claude_resp = ClaudeResponse {
            id: "msg_789".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![
                ClaudeContent {
                    content_type: "thinking".to_string(),
                    text: None,
                    thinking: Some("First consideration: ...".to_string()),
                    id: None,
                    name: None,
                    input: None,
                },
                ClaudeContent {
                    content_type: "thinking".to_string(),
                    text: None,
                    thinking: Some("Second consideration: ...".to_string()),
                    id: None,
                    name: None,
                    input: None,
                },
                ClaudeContent {
                    content_type: "text".to_string(),
                    text: Some("Based on my analysis...".to_string()),
                    thinking: None,
                    id: None,
                    name: None,
                    input: None,
                },
            ],
            model: "claude-3-opus-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 15,
                output_tokens: 80,
            },
        };

        let response = client.convert_response(claude_resp);

        // Verify thinking blocks were combined
        assert!(response.reasoning.is_some());
        let reasoning = response.reasoning.unwrap();
        assert!(reasoning.content.contains("First consideration"));
        assert!(reasoning.content.contains("Second consideration"));

        // Verify text was extracted
        assert_eq!(response.message.text().unwrap(), "Based on my analysis...");
    }

    // ============================================================
    // Additional Gap Tests
    // ============================================================

    /// Test: Empty message list conversion
    ///
    /// Verifies empty message list produces no system prompt and no messages.
    #[test]
    fn test_convert_messages_empty_list() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let messages: Vec<Message> = vec![];
        let (system, claude_msgs) = client.convert_messages(&messages);

        assert_eq!(system, None);
        assert!(claude_msgs.is_empty());
    }

    /// Test: Only system messages
    ///
    /// Verifies only system messages produces system prompt but no conversation.
    #[test]
    fn test_convert_messages_only_system() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        let messages = vec![Message::system("You are a helpful assistant.")];

        let (system, claude_msgs) = client.convert_messages(&messages);

        assert_eq!(system, Some("You are a helpful assistant.".to_string()));
        assert!(claude_msgs.is_empty());
    }

    /// Test: System messages not at start
    ///
    /// Verifies system messages are combined even if interleaved.
    #[test]
    fn test_convert_messages_interleaved_system() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let messages = vec![
            Message::human("Hello"),
            Message::system("Be concise"),
            Message::assistant("Hi!"),
        ];

        let (system, claude_msgs) = client.convert_messages(&messages);

        // System should be extracted even though it's not first
        assert_eq!(system, Some("Be concise".to_string()));
        assert_eq!(claude_msgs.len(), 2);
        assert_eq!(claude_msgs[0].role, "user");
        assert_eq!(claude_msgs[1].role, "assistant");
    }

    /// Test: Response with empty content array
    ///
    /// Verifies empty content array doesn't cause panic.
    #[test]
    fn test_convert_response_empty_content() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let claude_response = ClaudeResponse {
            id: "msg_empty".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![], // Empty content
            model: "claude-3-sonnet-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 10,
                output_tokens: 0,
            },
        };

        let response = client.convert_response(claude_response);

        // Should handle empty content gracefully
        assert_eq!(response.message.text(), Some(""));
        assert_eq!(response.message.role, MessageRole::Assistant);
    }

    /// Test: Response with missing stop_reason
    ///
    /// Verifies None stop_reason is handled gracefully.
    #[test]
    fn test_convert_response_missing_stop_reason() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        let claude_response = ClaudeResponse {
            id: "msg_no_stop".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![ClaudeContent {
                content_type: "text".to_string(),
                text: Some("Response without stop reason".to_string()),
                thinking: None,
                id: None,
                name: None,
                input: None,
            }],
            model: "claude-3-opus-20240229".to_string(),
            stop_reason: None, // Missing stop_reason
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 5,
                output_tokens: 10,
            },
        };

        let response = client.convert_response(claude_response);

        // Should handle None stop_reason
        assert_eq!(
            response.message.text(),
            Some("Response without stop reason")
        );
        // Metadata should still be populated
        assert!(response.metadata.contains_key("model"));
    }

    /// Test: Message with empty content
    ///
    /// Verifies messages with empty text are handled.
    #[test]
    fn test_convert_messages_empty_content() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let messages = vec![
            Message::human(""),
            Message::assistant(""),
        ];

        let (system, claude_msgs) = client.convert_messages(&messages);

        assert_eq!(system, None);
        assert_eq!(claude_msgs.len(), 2);
        // Empty strings should be preserved
        assert_eq!(claude_msgs[0].content, "");
        assert_eq!(claude_msgs[1].content, "");
    }

    /// Test: Response conversion preserves message ID
    ///
    /// Verifies the Claude message ID is stored in the response.
    #[test]
    fn test_response_conversion_preserves_id() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let claude_response = ClaudeResponse {
            id: "msg_unique_id_12345".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![ClaudeContent {
                content_type: "text".to_string(),
                text: Some("Test".to_string()),
                thinking: None,
                id: None,
                name: None,
                input: None,
            }],
            model: "claude-3-sonnet-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
        };

        let response = client.convert_response(claude_response);

        assert_eq!(response.message.id, Some("msg_unique_id_12345".to_string()));
    }

    /// Test: Usage metadata is correctly populated
    ///
    /// Verifies token counts are accurately transferred.
    #[test]
    fn test_response_conversion_usage_accuracy() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-opus-20240229",
        );
        let client = ClaudeClient::new(config);

        let claude_response = ClaudeResponse {
            id: "msg_usage".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![ClaudeContent {
                content_type: "text".to_string(),
                text: Some("Test".to_string()),
                thinking: None,
                id: None,
                name: None,
                input: None,
            }],
            model: "claude-3-opus-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 1234,
                output_tokens: 5678,
            },
        };

        let response = client.convert_response(claude_response);

        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 1234);
        assert_eq!(usage.output_tokens, 5678);
        // Total tokens should be sum
        assert_eq!(usage.total_tokens, 1234 + 5678);
    }

    // ============================================================
    // Tool Calling Tests
    // ============================================================

    #[test]
    fn test_response_conversion_with_tool_use() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let claude_response = ClaudeResponse {
            id: "msg_tool".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![
                ClaudeContent {
                    content_type: "text".to_string(),
                    text: Some("Let me check the weather.".to_string()),
                    thinking: None,
                    id: None,
                    name: None,
                    input: None,
                },
                ClaudeContent {
                    content_type: "tool_use".to_string(),
                    text: None,
                    thinking: None,
                    id: Some("toolu_123".to_string()),
                    name: Some("get_weather".to_string()),
                    input: Some(serde_json::json!({"location": "San Francisco"})),
                },
            ],
            model: "claude-3-sonnet-20240229".to_string(),
            stop_reason: Some("tool_use".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 50,
                output_tokens: 30,
            },
        };

        let response = client.convert_response(claude_response);

        // Should extract tool calls
        assert!(response.message.tool_calls.is_some());
        let tool_calls = response.message.tool_calls.clone().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "toolu_123");
        assert_eq!(tool_calls[0].name, "get_weather");
        assert_eq!(tool_calls[0].args["location"], "San Francisco");

        // Text should still be extracted
        assert_eq!(response.message.text(), Some("Let me check the weather."));
    }

    #[test]
    fn test_convert_tools() {
        use langgraph_core::llm::ToolDefinition;

        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let tools = vec![
            ToolDefinition::new("get_weather", "Get weather for a location")
                .with_parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    },
                    "required": ["location"]
                })),
        ];

        let claude_tools = client.convert_tools(&tools);

        assert_eq!(claude_tools.len(), 1);
        assert_eq!(claude_tools[0].name, "get_weather");
        assert_eq!(claude_tools[0].description, "Get weather for a location");
        assert!(claude_tools[0].input_schema.is_some());
    }

    #[test]
    fn test_response_no_tool_calls() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.anthropic.com",
            "claude-3-sonnet-20240229",
        );
        let client = ClaudeClient::new(config);

        let claude_response = ClaudeResponse {
            id: "msg_no_tools".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![ClaudeContent {
                content_type: "text".to_string(),
                text: Some("Just a regular response".to_string()),
                thinking: None,
                id: None,
                name: None,
                input: None,
            }],
            model: "claude-3-sonnet-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: ClaudeUsage {
                input_tokens: 10,
                output_tokens: 5,
            },
        };

        let response = client.convert_response(claude_response);

        // No tool calls should be present
        assert!(response.message.tool_calls.is_none());
        assert_eq!(response.message.text(), Some("Just a regular response"));
    }
}

