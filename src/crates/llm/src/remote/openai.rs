//! OpenAI client implementation.
//!
//! Provides integration with OpenAI's API, supporting models like:
//! - GPT-4, GPT-4 Turbo
//! - GPT-3.5 Turbo
//! - o1, o1-mini (thinking models)
//!
//! # Example
//!
//! ```rust,ignore
//! use llm::remote::OpenAiClient;
//! use llm::config::RemoteLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! let config = RemoteLlmConfig::from_env(
//!     "OPENAI_API_KEY",
//!     "https://api.openai.com/v1",
//!     "gpt-4"
//! )?;
//! let client = OpenAiClient::new(config);
//!
//! let request = ChatRequest::new(vec![Message::human("Hello!")]);
//! let response = client.chat(request).await?;
//! ```

use crate::config::RemoteLlmConfig;
use crate::error::LlmError;
use async_trait::async_trait;
use langgraph_core::error::Result as GraphResult;
use langgraph_core::llm::{
    ChatModel, ChatRequest, ChatResponse, ChatStreamResponse, ReasoningContent, UsageMetadata,
};
use langgraph_core::{Message, MessageContent, MessageRole};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenAI API client.
#[derive(Clone)]
pub struct OpenAiClient {
    config: RemoteLlmConfig,
    client: Client,
}

impl OpenAiClient {
    /// Create a new OpenAI client with the given configuration.
    pub fn new(config: RemoteLlmConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Convert langgraph Message to OpenAI message format.
    fn convert_message(&self, msg: &Message) -> OpenAiMessage {
        OpenAiMessage {
            role: match &msg.role {
                MessageRole::System => "system".to_string(),
                MessageRole::Human => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::Tool => "tool".to_string(),
                MessageRole::Custom(role) => role.clone(),
            },
            content: Some(msg.text().unwrap_or("").to_string()),
            name: msg.name.clone(),
            tool_call_id: msg.tool_call_id.clone(),
        }
    }

    /// Convert OpenAI response to ChatResponse.
    fn convert_response(&self, request: &ChatRequest, openai_resp: OpenAiResponse) -> ChatResponse {
        let choice = &openai_resp.choices[0];

        // Check if this is a thinking model (o1, o1-mini)
        let is_thinking_model = self.config.model.starts_with("o1");
        
        let (message_content, reasoning) = if is_thinking_model && request.config.reasoning_mode.should_capture() {
            // For o1 models, reasoning is typically in a separate field or prefixed
            // For now, we'll extract it based on content markers
            let content = choice.message.content.clone().unwrap_or_default();
            
            // Simple reasoning extraction (can be enhanced)
            if content.contains("<think>") && content.contains("</think>") {
                let parts: Vec<&str> = content.split("</think>").collect();
                if parts.len() >= 2 {
                    let thinking = parts[0].replace("<think>", "").trim().to_string();
                    let answer = parts[1].trim().to_string();
                    
                    let reasoning_content = ReasoningContent::new(thinking);
                    (answer, Some(reasoning_content))
                } else {
                    (content, None)
                }
            } else {
                (content, None)
            }
        } else {
            (choice.message.content.clone().unwrap_or_default(), None)
        };

        let message = Message {
            id: None,
            role: MessageRole::Assistant,
            content: MessageContent::Text(message_content),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        };

        let usage = openai_resp.usage.as_ref().map(|u| {
            if let Some(reasoning_tokens) = u.completion_tokens_details.as_ref().and_then(|d| d.reasoning_tokens) {
                UsageMetadata::with_reasoning(
                    u.prompt_tokens,
                    u.completion_tokens,
                    reasoning_tokens,
                )
            } else {
                UsageMetadata::new(u.prompt_tokens, u.completion_tokens)
            }
        });

        let mut metadata = HashMap::new();
        metadata.insert(
            "model".to_string(),
            serde_json::Value::String(openai_resp.model),
        );
        metadata.insert(
            "finish_reason".to_string(),
            serde_json::Value::String(choice.finish_reason.clone().unwrap_or_default()),
        );

        ChatResponse {
            message,
            usage,
            reasoning,
            metadata,
        }
    }
}

#[async_trait]
impl ChatModel for OpenAiClient {
    async fn chat(&self, request: ChatRequest) -> GraphResult<ChatResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect();

        let req_body = OpenAiRequest {
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
        
        // Add organization header if provided
        if let Some(org) = &self.config.organization {
            req = req.header("OpenAI-Organization", org);
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
                LlmError::ProviderError(format!("OpenAI API error {}: {}", status, error_text))
            }
            .into());
        }

        let openai_resp: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(self.convert_response(&request, openai_resp))
    }

    async fn stream(&self, _request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        // TODO: Implement streaming support
        Err(LlmError::Other("Streaming not yet implemented for OpenAI".to_string()).into())
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(self.clone())
    }
}

// OpenAI API types
#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
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
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    index: usize,
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
    #[serde(default)]
    completion_tokens_details: Option<OpenAiCompletionTokensDetails>,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompletionTokensDetails {
    reasoning_tokens: Option<usize>,
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
            "https://api.openai.com/v1",
            "gpt-4",
        );
        let _client = OpenAiClient::new(config);
    }

    #[test]
    fn test_message_conversion() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.openai.com/v1",
            "gpt-4",
        );
        let client = OpenAiClient::new(config);

        let msg = Message::human("Hello");
        let openai_msg = client.convert_message(&msg);

        assert_eq!(openai_msg.role, "user");
        assert_eq!(openai_msg.content, Some("Hello".to_string()));
    }

    // ============================================================
    // Phase 4.1: OpenAI Provider Tests
    // ============================================================

    #[test]
    fn test_message_conversion_all_roles() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        // Test System message
        let sys_msg = Message::system("You are helpful");
        let openai_sys = client.convert_message(&sys_msg);
        assert_eq!(openai_sys.role, "system");
        assert_eq!(openai_sys.content, Some("You are helpful".to_string()));

        // Test Human/User message
        let user_msg = Message::human("Hello");
        let openai_user = client.convert_message(&user_msg);
        assert_eq!(openai_user.role, "user");
        assert_eq!(openai_user.content, Some("Hello".to_string()));

        // Test Assistant message
        let asst_msg = Message::assistant("Hi there!");
        let openai_asst = client.convert_message(&asst_msg);
        assert_eq!(openai_asst.role, "assistant");
        assert_eq!(openai_asst.content, Some("Hi there!".to_string()));
    }

    #[test]
    fn test_message_conversion_with_name() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let mut msg = Message::human("Hello");
        msg.name = Some("user-123".to_string());

        let openai_msg = client.convert_message(&msg);
        assert_eq!(openai_msg.role, "user");
        assert_eq!(openai_msg.name, Some("user-123".to_string()));
    }

    #[test]
    fn test_config_with_organization() {
        let mut config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        config.organization = Some("org-123".to_string());

        let client = OpenAiClient::new(config.clone());
        assert_eq!(client.config.organization, Some("org-123".to_string()));
    }

    #[test]
    fn test_config_with_custom_timeout() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4")
            .with_timeout(Duration::from_secs(10));

        let client = OpenAiClient::new(config);
        assert_eq!(client.config.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_o1_model_detection() {
        let config_o1 = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "o1-preview");
        let client_o1 = OpenAiClient::new(config_o1);
        assert!(client_o1.config.model.starts_with("o1"));

        let config_o1_mini = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "o1-mini");
        let client_o1_mini = OpenAiClient::new(config_o1_mini);
        assert!(client_o1_mini.config.model.starts_with("o1"));

        let config_gpt4 = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client_gpt4 = OpenAiClient::new(config_gpt4);
        assert!(!client_gpt4.config.model.starts_with("o1"));
    }

    #[test]
    fn test_response_conversion_basic() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Hello")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Hi there!".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(OpenAiUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
                completion_tokens_details: None,
            }),
        };

        let response = client.convert_response(&request, openai_response);

        assert_eq!(response.message.text(), Some("Hi there!"));
        assert_eq!(response.message.role, MessageRole::Assistant);
        assert_eq!(response.usage.as_ref().unwrap().input_tokens, 10);
        assert_eq!(response.usage.as_ref().unwrap().output_tokens, 20);
        assert!(response.metadata.contains_key("model"));
        assert!(response.metadata.contains_key("finish_reason"));
    }

    #[test]
    fn test_response_conversion_with_reasoning_tokens() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "o1-preview");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Solve this problem")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-o1-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "o1-preview".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("The answer is 42".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(OpenAiUsage {
                prompt_tokens: 15,
                completion_tokens: 50,
                total_tokens: 65,
                completion_tokens_details: Some(OpenAiCompletionTokensDetails {
                    reasoning_tokens: Some(35),
                }),
            }),
        };

        let response = client.convert_response(&request, openai_response);

        assert_eq!(response.usage.as_ref().unwrap().input_tokens, 15);
        assert_eq!(response.usage.as_ref().unwrap().output_tokens, 50);
        assert_eq!(response.usage.as_ref().unwrap().reasoning_tokens, Some(35));
    }

    #[test]
    fn test_reasoning_extraction_with_think_tags() {
        use langgraph_core::llm::ReasoningMode;

        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "o1-preview");
        let client = OpenAiClient::new(config);

        let mut request = ChatRequest::new(vec![Message::human("Think about this")]);
        request.config.reasoning_mode = ReasoningMode::Separated;

        let openai_response = OpenAiResponse {
            id: "chatcmpl-o1-456".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "o1-preview".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("<think>Let me analyze this step by step</think>The solution is X".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response);

        // Should extract reasoning from <think> tags
        assert!(response.reasoning.is_some());
        assert_eq!(response.reasoning.as_ref().unwrap().content, "Let me analyze this step by step");
        assert_eq!(response.message.text(), Some("The solution is X"));
    }

    #[test]
    fn test_reasoning_extraction_no_tags() {
        use langgraph_core::llm::ReasoningMode;

        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "o1-preview");
        let client = OpenAiClient::new(config);

        let mut request = ChatRequest::new(vec![Message::human("Simple question")]);
        request.config.reasoning_mode = ReasoningMode::Separated;

        let openai_response = OpenAiResponse {
            id: "chatcmpl-o1-789".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "o1-preview".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Simple answer without thinking".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response);

        // Should not extract reasoning if no tags present
        assert!(response.reasoning.is_none());
        assert_eq!(response.message.text(), Some("Simple answer without thinking"));
    }

    #[test]
    fn test_reasoning_mode_disabled() {
        use langgraph_core::llm::ReasoningMode;

        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "o1-preview");
        let client = OpenAiClient::new(config);

        let mut request = ChatRequest::new(vec![Message::human("Question")]);
        request.config.reasoning_mode = ReasoningMode::Disabled;

        let openai_response = OpenAiResponse {
            id: "chatcmpl-o1-999".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "o1-preview".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("<think>Hidden reasoning</think>Answer".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response);

        // Should not extract reasoning when mode is None
        assert!(response.reasoning.is_none());
        assert_eq!(response.message.text(), Some("<think>Hidden reasoning</think>Answer"));
    }

    // ============================================================
    // Future Implementation Tests (Marked #[ignore])
    // ============================================================

    /// Test: Streaming support
    ///
    /// Verifies that OpenAI streaming returns token-by-token responses.
    ///
    /// NOTE: Currently ignored - streaming not yet implemented for OpenAI.
    /// See line 209-212 in chat implementation.
    #[test]
    fn test_message_conversion_tool_message() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let mut msg = Message::system("Tool result");
        msg.role = MessageRole::Tool;
        msg.tool_call_id = Some("tool-123".to_string());

        let openai_msg = client.convert_message(&msg);
        assert_eq!(openai_msg.role, "tool");
        assert_eq!(openai_msg.tool_call_id, Some("tool-123".to_string()));
    }

    #[test]
    fn test_message_conversion_custom_role() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let mut msg = Message::human("Custom message");
        msg.role = MessageRole::Custom("moderator".to_string());

        let openai_msg = client.convert_message(&msg);
        assert_eq!(openai_msg.role, "moderator");
        assert_eq!(openai_msg.content, Some("Custom message".to_string()));
    }

    #[test]
    fn test_message_conversion_empty_content() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let msg = Message {
            id: None,
            role: MessageRole::Assistant,
            content: MessageContent::Text("".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        };

        let openai_msg = client.convert_message(&msg);
        assert_eq!(openai_msg.role, "assistant");
        assert_eq!(openai_msg.content, Some("".to_string()));
    }

    #[test]
    fn test_multiple_messages_conversion() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let messages = vec![
            Message::system("System prompt"),
            Message::human("User question"),
            Message::assistant("Assistant response"),
        ];

        let converted: Vec<_> = messages
            .iter()
            .map(|m| client.convert_message(m))
            .collect();

        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].role, "system");
        assert_eq!(converted[1].role, "user");
        assert_eq!(converted[2].role, "assistant");
    }

    #[test]
    fn test_response_conversion_without_usage() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Hello")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Hi".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response);

        assert_eq!(response.message.text(), Some("Hi"));
        assert!(response.usage.is_none());
        assert!(response.metadata.contains_key("finish_reason"));
    }

    #[test]
    fn test_response_conversion_metadata_fields() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Hello")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4-turbo".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Response".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: Some("length".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response);

        assert_eq!(response.metadata["model"], "gpt-4-turbo");
        assert_eq!(response.metadata["finish_reason"], "length");
    }

    #[test]
    fn test_response_conversion_finish_reason_none() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Hello")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Hi".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: None,
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response);

        assert_eq!(response.metadata["finish_reason"], "");
    }

    #[test]
    fn test_config_builder_temperature() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4")
            .with_temperature(0.7);

        let client = OpenAiClient::new(config);
        assert_eq!(client.config.temperature, Some(0.7));
    }

    #[test]
    fn test_config_builder_max_tokens() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4")
            .with_max_tokens(2048);

        let client = OpenAiClient::new(config);
        assert_eq!(client.config.max_tokens, Some(2048));
    }

    #[test]
    fn test_config_builder_top_p() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4")
            .with_top_p(0.9);

        let client = OpenAiClient::new(config);
        assert_eq!(client.config.top_p, Some(0.9));
    }

    #[test]
    fn test_reasoning_extraction_multiple_think_blocks() {
        use langgraph_core::llm::ReasoningMode;

        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "o1-preview");
        let client = OpenAiClient::new(config);

        let mut request = ChatRequest::new(vec![Message::human("Complex problem")]);
        request.config.reasoning_mode = ReasoningMode::Separated;

        let openai_response = OpenAiResponse {
            id: "chatcmpl-o1-789".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "o1-preview".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("<think>First part of thinking</think>Middle section</think>And final answer".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response);

        // Should extract first reasoning block
        assert!(response.reasoning.is_some());
        assert_eq!(response.reasoning.as_ref().unwrap().content, "First part of thinking");
    }

    #[test]
    fn test_non_thinking_model_ignores_reasoning() {
        use langgraph_core::llm::ReasoningMode;

        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let mut request = ChatRequest::new(vec![Message::human("Question")]);
        request.config.reasoning_mode = ReasoningMode::Separated;

        let openai_response = OpenAiResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("<think>Not extracted</think>Answer".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response);

        // Non-o1 models should not extract reasoning even if tags present
        assert!(response.reasoning.is_none());
    }

    #[test]
    fn test_usage_metadata_without_reasoning_details() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Hello")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Hi there!".to_string()),
                    name: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(OpenAiUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
                completion_tokens_details: None,
            }),
        };

        let response = client.convert_response(&request, openai_response);

        let usage = response.usage.as_ref().unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
        assert!(usage.reasoning_tokens.is_none());
    }

    #[test]
    fn test_client_cloneable() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);
        let cloned = client.clone();

        assert_eq!(cloned.config.api_key, client.config.api_key);
        assert_eq!(cloned.config.model, client.config.model);
    }

    #[test]
    fn test_message_with_special_characters() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let special_text = "Test with special chars: éàü 中文 🚀 <>&\"'";
        let msg = Message::human(special_text);
        let openai_msg = client.convert_message(&msg);

        assert_eq!(openai_msg.content, Some(special_text.to_string()));
    }

    #[test]
    fn test_message_serialization_roundtrip() {
        let openai_msg = OpenAiMessage {
            role: "user".to_string(),
            content: Some("Test message".to_string()),
            name: Some("test-user".to_string()),
            tool_call_id: Some("tool-456".to_string()),
        };

        let json = serde_json::to_string(&openai_msg).unwrap();
        let deserialized: OpenAiMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.role, "user");
        assert_eq!(deserialized.content, Some("Test message".to_string()));
        assert_eq!(deserialized.name, Some("test-user".to_string()));
        assert_eq!(deserialized.tool_call_id, Some("tool-456".to_string()));
    }

    #[tokio::test]
    #[ignore]
    async fn test_streaming_basic() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Count to 5")]);

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

    /// Test: Tool calling support
    ///
    /// Verifies that OpenAI tool/function calling works correctly.
    ///
    /// NOTE: Currently ignored - tool calling not yet implemented in this client.
    #[tokio::test]
    #[ignore]
    async fn test_tool_calling() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("What's the weather?")]);

        // TODO: Add tool definitions to request
        // request.config.tools = vec![...];

        let response = client.chat(request).await.unwrap();

        // Should contain tool calls in response
        assert!(response.message.tool_calls.is_some());
    }

    /// Test: Vision/multi-modal support
    ///
    /// Verifies that GPT-4 Vision handles image inputs correctly.
    ///
    /// NOTE: Currently ignored - multi-modal support not yet implemented.
    #[tokio::test]
    #[ignore]
    async fn test_vision_support() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4-vision-preview");
        let client = OpenAiClient::new(config);

        // TODO: Create message with image content
        let request = ChatRequest::new(vec![Message::human("Describe this image")]);

        let response = client.chat(request).await.unwrap();

        assert!(response.message.text().is_some());
    }
}

