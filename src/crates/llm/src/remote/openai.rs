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
use crate::streaming;
use async_trait::async_trait;
use langgraph_core::error::Result as GraphResult;
use langgraph_core::llm::{
    ChatModel, ChatRequest, ChatResponse, ChatStreamResponse, ReasoningContent, UsageMetadata,
};
use langgraph_core::{Message, MessageContent, MessageRole};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
            tool_calls: None, // We don't send tool_calls in requests
        }
    }

    /// Convert OpenAI response to ChatResponse.
    fn convert_response(&self, request: &ChatRequest, openai_resp: OpenAiResponse) -> Result<ChatResponse, LlmError> {
        use langgraph_core::ToolCall;

        let choice = openai_resp.choices.first().ok_or_else(|| {
            LlmError::InvalidResponse("OpenAI response contained no choices".to_string())
        })?;

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

        // Extract tool calls from response
        let tool_calls: Option<Vec<ToolCall>> = choice.message.tool_calls.as_ref().map(|calls| {
            calls.iter().filter_map(|tc| {
                // Parse the arguments JSON string
                match serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                    Ok(args) => Some(ToolCall {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        args,
                    }),
                    Err(_) => {
                        // If parsing fails, use the raw string as a JSON string value
                        Some(ToolCall {
                            id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            args: serde_json::json!(tc.function.arguments),
                        })
                    }
                }
            }).collect()
        }).filter(|v: &Vec<ToolCall>| !v.is_empty());

        let message = Message {
            id: None,
            role: MessageRole::Assistant,
            content: MessageContent::Text(message_content),
            name: None,
            tool_calls,
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

        Ok(ChatResponse {
            message,
            usage,
            reasoning,
            metadata,
        })
    }

    /// Convert ToolDefinition to OpenAI's tool format
    fn convert_tools(&self, tools: &[langgraph_core::llm::ToolDefinition]) -> Vec<OpenAiTool> {
        tools.iter().map(|t| OpenAiTool {
            tool_type: "function".to_string(),
            function: OpenAiFunctionDef {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        }).collect()
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

        // Convert tools if provided
        let tools = if request.config.tools.is_empty() {
            None
        } else {
            Some(self.convert_tools(&request.config.tools))
        };

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
            tools,
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

        self.convert_response(&request, openai_resp).map_err(|e| e.into())
    }

    async fn stream(&self, request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        // Convert messages
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                let openai_msg = self.convert_message(m);
                json!({
                    "role": openai_msg.role,
                    "content": openai_msg.content,
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

        // Add tools if provided
        if !request.config.tools.is_empty() {
            let openai_tools: Vec<serde_json::Value> = request.config.tools.iter().map(|t| {
                let mut tool = json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                    }
                });
                if let Some(params) = &t.parameters {
                    tool["function"]["parameters"] = params.clone();
                }
                tool
            }).collect();
            req_body["tools"] = json!(openai_tools);
        }

        // Build headers
        let mut headers = vec![("Authorization", format!("Bearer {}", self.config.api_key))];
        let org_header;
        if let Some(ref org) = self.config.organization {
            org_header = format!("{}", org);
            headers.push(("OpenAI-Organization", org_header.clone()));
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
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
    stream: bool,
}

/// OpenAI tool definition format
#[derive(Debug, Serialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    tool_type: String, // Always "function"
    function: OpenAiFunctionDef,
}

/// OpenAI function definition within a tool
#[derive(Debug, Serialize)]
struct OpenAiFunctionDef {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<serde_json::Value>,
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
    /// Tool calls requested by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// Tool call from OpenAI response
#[derive(Debug, Serialize, Deserialize)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String, // "function"
    function: OpenAiFunctionCall,
}

/// Function call details in a tool call
#[derive(Debug, Serialize, Deserialize)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String, // JSON string
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
                    tool_calls: None,
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

        let response = client.convert_response(&request, openai_response).unwrap();

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
                    tool_calls: None,
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

        let response = client.convert_response(&request, openai_response).unwrap();

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
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response).unwrap();

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
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response).unwrap();

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
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response).unwrap();

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

    // ============================================================
    // Additional Gap Tests
    // ============================================================

    /// Test: Tool message conversion
    ///
    /// Verifies tool messages have correct role.
    #[test]
    fn test_convert_message_tool_role() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let mut tool_msg = Message::human("Tool result data");
        tool_msg.role = MessageRole::Tool;
        tool_msg.tool_call_id = Some("call_123".to_string());

        let openai_msg = client.convert_message(&tool_msg);

        assert_eq!(openai_msg.role, "tool");
        assert_eq!(openai_msg.tool_call_id, Some("call_123".to_string()));
    }

    /// Test: Message with empty content
    ///
    /// Verifies empty content is handled.
    #[test]
    fn test_convert_message_empty_content() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let empty_msg = Message::human("");
        let openai_msg = client.convert_message(&empty_msg);

        assert_eq!(openai_msg.content, Some("".to_string()));
    }

    /// Test: Response without usage data
    ///
    /// Verifies None usage is handled gracefully.
    #[test]
    fn test_convert_response_no_usage() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Test")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-no-usage".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Response".to_string()),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None, // No usage data
        };

        let response = client.convert_response(&request, openai_response).unwrap();

        // Should not panic on None usage
        assert!(response.usage.is_none());
        assert_eq!(response.message.text(), Some("Response"));
    }

    /// Test: Response with empty choices
    ///
    /// Verifies that empty choices returns an error instead of panicking.
    #[test]
    fn test_convert_response_empty_choices_returns_error() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Test")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-empty".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![], // Empty choices
            usage: Some(OpenAiUsage {
                prompt_tokens: 10,
                completion_tokens: 0,
                total_tokens: 10,
                completion_tokens_details: None,
            }),
        };

        // Should return an error, not panic
        let result = client.convert_response(&request, openai_response);
        assert!(result.is_err());

        // Verify error message
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("no choices"), "Expected error about no choices, got: {}", err_msg);
    }

    /// Test: All message roles conversion
    ///
    /// Verifies all MessageRole variants are handled.
    #[test]
    fn test_convert_message_all_roles() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        // System message
        let sys_msg = Message::system("System prompt");
        assert_eq!(client.convert_message(&sys_msg).role, "system");

        // Human message
        let human_msg = Message::human("User message");
        assert_eq!(client.convert_message(&human_msg).role, "user");

        // Assistant message
        let asst_msg = Message::assistant("AI response");
        assert_eq!(client.convert_message(&asst_msg).role, "assistant");

        // Tool message
        let mut tool_msg = Message::human("tool result");
        tool_msg.role = MessageRole::Tool;
        assert_eq!(client.convert_message(&tool_msg).role, "tool");

        // Custom role
        let mut custom_msg = Message::human("custom");
        custom_msg.role = MessageRole::Custom("moderator".to_string());
        assert_eq!(client.convert_message(&custom_msg).role, "moderator");
    }

    /// Test: Response preserves model info
    ///
    /// Verifies model is stored in metadata.
    #[test]
    fn test_convert_response_preserves_model() {
        let config = RemoteLlmConfig::new("test-key", "https://api.openai.com/v1", "gpt-4");
        let client = OpenAiClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Test")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-model".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4-0125-preview".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Test".to_string()),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response).unwrap();

        assert!(response.metadata.contains_key("model"));
        assert_eq!(
            response.metadata.get("model").and_then(|v| v.as_str()),
            Some("gpt-4-0125-preview")
        );
    }

    // ============================================================
    // Tool Calling Tests
    // ============================================================

    #[test]
    fn test_response_with_tool_calls() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.openai.com/v1",
            "gpt-4",
        );
        let client = OpenAiClient::new(config);
        let request = ChatRequest::new(vec![Message::human("What's the weather in SF?")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-tool123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: None, // Tool calls often have no content
                    name: None,
                    tool_call_id: None,
                    tool_calls: Some(vec![OpenAiToolCall {
                        id: "call_abc123".to_string(),
                        call_type: "function".to_string(),
                        function: OpenAiFunctionCall {
                            name: "get_weather".to_string(),
                            arguments: r#"{"location": "San Francisco"}"#.to_string(),
                        },
                    }]),
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: Some(OpenAiUsage {
                prompt_tokens: 50,
                completion_tokens: 20,
                total_tokens: 70,
                completion_tokens_details: None,
            }),
        };

        let response = client.convert_response(&request, openai_response).unwrap();

        // Should extract tool calls
        assert!(response.message.tool_calls.is_some());
        let tool_calls = response.message.tool_calls.clone().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_abc123");
        assert_eq!(tool_calls[0].name, "get_weather");
        assert_eq!(tool_calls[0].args["location"], "San Francisco");
    }

    #[test]
    fn test_convert_tools() {
        use langgraph_core::llm::ToolDefinition;

        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.openai.com/v1",
            "gpt-4",
        );
        let client = OpenAiClient::new(config);

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

        let openai_tools = client.convert_tools(&tools);

        assert_eq!(openai_tools.len(), 1);
        assert_eq!(openai_tools[0].tool_type, "function");
        assert_eq!(openai_tools[0].function.name, "get_weather");
        assert_eq!(openai_tools[0].function.description, "Get weather for a location");
        assert!(openai_tools[0].function.parameters.is_some());
    }

    #[test]
    fn test_response_no_tool_calls() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.openai.com/v1",
            "gpt-4",
        );
        let client = OpenAiClient::new(config);
        let request = ChatRequest::new(vec![Message::human("Hello")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-no-tools".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Hello! How can I help you?".to_string()),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(OpenAiUsage {
                prompt_tokens: 10,
                completion_tokens: 8,
                total_tokens: 18,
                completion_tokens_details: None,
            }),
        };

        let response = client.convert_response(&request, openai_response).unwrap();

        // No tool calls should be present
        assert!(response.message.tool_calls.is_none());
        assert_eq!(response.message.text(), Some("Hello! How can I help you?"));
    }

    #[test]
    fn test_response_with_multiple_tool_calls() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.openai.com/v1",
            "gpt-4",
        );
        let client = OpenAiClient::new(config);
        let request = ChatRequest::new(vec![Message::human("Compare weather in SF and LA")]);

        let openai_response = OpenAiResponse {
            id: "chatcmpl-multi".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![OpenAiChoice {
                index: 0,
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: None,
                    name: None,
                    tool_call_id: None,
                    tool_calls: Some(vec![
                        OpenAiToolCall {
                            id: "call_1".to_string(),
                            call_type: "function".to_string(),
                            function: OpenAiFunctionCall {
                                name: "get_weather".to_string(),
                                arguments: r#"{"location": "San Francisco"}"#.to_string(),
                            },
                        },
                        OpenAiToolCall {
                            id: "call_2".to_string(),
                            call_type: "function".to_string(),
                            function: OpenAiFunctionCall {
                                name: "get_weather".to_string(),
                                arguments: r#"{"location": "Los Angeles"}"#.to_string(),
                            },
                        },
                    ]),
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, openai_response).unwrap();

        // Should extract both tool calls
        assert!(response.message.tool_calls.is_some());
        let tool_calls = response.message.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 2);
        assert_eq!(tool_calls[0].args["location"], "San Francisco");
        assert_eq!(tool_calls[1].args["location"], "Los Angeles");
    }
}

