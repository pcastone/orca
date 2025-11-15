//! Deepseek client implementation.
//!
//! Provides integration with Deepseek's API, including:
//! - Deepseek Chat
//! - Deepseek Coder
//! - Deepseek R1 (thinking model with extended reasoning)
//!
//! # Example
//!
//! ```rust,ignore
//! use llm::remote::DeepseekClient;
//! use llm::config::RemoteLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest, ReasoningMode};
//! use langgraph_core::Message;
//!
//! let config = RemoteLlmConfig::from_env(
//!     "DEEPSEEK_API_KEY",
//!     "https://api.deepseek.com",
//!     "deepseek-reasoner"
//! )?;
//! let client = DeepseekClient::new(config);
//!
//! // For R1 thinking model with reasoning
//! let request = ChatRequest::new(vec![Message::human("Solve this puzzle...")])
//!     .with_reasoning(ReasoningMode::Separated);
//! let response = client.chat(request).await?;
//! 
//! if let Some(reasoning) = response.reasoning {
//!     println!("Thinking: {}", reasoning.content);
//! }
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

/// Deepseek API client (OpenAI-compatible).
#[derive(Clone)]
pub struct DeepseekClient {
    config: RemoteLlmConfig,
    client: Client,
}

impl DeepseekClient {
    /// Create a new Deepseek client with the given configuration.
    pub fn new(config: RemoteLlmConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Check if this is a thinking model (R1 series).
    fn is_thinking_model(&self) -> bool {
        self.config.model.contains("reasoner") || self.config.model.contains("r1")
    }

    /// Convert langgraph Message to Deepseek message format.
    fn convert_message(&self, msg: &Message) -> DeepseekMessage {
        DeepseekMessage {
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

    /// Extract reasoning content from response if present.
    /// Deepseek R1 models may include thinking in <think> tags.
    fn extract_reasoning(&self, content: &str) -> (String, Option<ReasoningContent>) {
        if content.contains("<think>") && content.contains("</think>") {
            // Find the thinking section
            if let Some(think_start) = content.find("<think>") {
                if let Some(think_end) = content.find("</think>") {
                    let thinking = content[think_start + 7..think_end].trim().to_string();
                    let answer = content[think_end + 8..].trim().to_string();
                    
                    let reasoning = ReasoningContent::new(thinking);
                    return (answer, Some(reasoning));
                }
            }
        }
        (content.to_string(), None)
    }

    /// Convert Deepseek response to ChatResponse.
    fn convert_response(&self, request: &ChatRequest, deepseek_resp: DeepseekResponse) -> ChatResponse {
        let choice = &deepseek_resp.choices[0];
        let raw_content = choice.message.content.clone();

        // Extract reasoning if this is a thinking model and reasoning is requested
        let (message_content, reasoning) = if self.is_thinking_model() && request.config.reasoning_mode.should_capture() {
            self.extract_reasoning(&raw_content)
        } else {
            (raw_content, None)
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

        let usage = deepseek_resp.usage.as_ref().map(|u| {
            if let Some(reasoning_tokens) = u.reasoning_tokens {
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
            serde_json::Value::String(deepseek_resp.model),
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
impl ChatModel for DeepseekClient {
    async fn chat(&self, request: ChatRequest) -> GraphResult<ChatResponse> {
        let url = format!("{}/v1/chat/completions", self.config.base_url);

        let messages: Vec<DeepseekMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect();

        let req_body = DeepseekRequest {
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
            .header("Authorization", format!("Bearer {}", self.config.api_key))
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
                LlmError::ProviderError(format!("Deepseek API error {}: {}", status, error_text))
            }
            .into());
        }

        let deepseek_resp: DeepseekResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(self.convert_response(&request, deepseek_resp))
    }

    async fn stream(&self, _request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        // TODO: Implement streaming support
        Err(LlmError::Other("Streaming not yet implemented for Deepseek".to_string()).into())
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(self.clone())
    }
}

// Deepseek API types (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct DeepseekRequest {
    model: String,
    messages: Vec<DeepseekMessage>,
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
struct DeepseekMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct DeepseekResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<DeepseekChoice>,
    usage: Option<DeepseekUsage>,
}

#[derive(Debug, Deserialize)]
struct DeepseekChoice {
    index: usize,
    message: DeepseekMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeepseekUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
    #[serde(default)]
    reasoning_tokens: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use langgraph_core::llm::ReasoningMode;
    use std::time::Duration;

    // ============================================================
    // Existing Tests
    // ============================================================

    #[test]
    fn test_client_creation() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        let _client = DeepseekClient::new(config);
    }

    #[test]
    fn test_is_thinking_model() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        let client = DeepseekClient::new(config);
        assert!(client.is_thinking_model());

        let config2 = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-chat",
        );
        let client2 = DeepseekClient::new(config2);
        assert!(!client2.is_thinking_model());
    }

    #[test]
    fn test_extract_reasoning() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        let client = DeepseekClient::new(config);

        let content = "<think>Let me analyze this...</think>The answer is 42.";
        let (answer, reasoning) = client.extract_reasoning(content);

        assert_eq!(answer, "The answer is 42.");
        assert!(reasoning.is_some());
        assert_eq!(reasoning.unwrap().content, "Let me analyze this...");
    }

    // ============================================================
    // Message Conversion Tests
    // ============================================================

    #[test]
    fn test_message_conversion_all_roles() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-chat",
        );
        let client = DeepseekClient::new(config);

        let sys_msg = Message::system("You are helpful");
        let deepseek_sys = client.convert_message(&sys_msg);
        assert_eq!(deepseek_sys.role, "system");
        assert_eq!(deepseek_sys.content, "You are helpful");

        let user_msg = Message::human("Hello");
        let deepseek_user = client.convert_message(&user_msg);
        assert_eq!(deepseek_user.role, "user");
        assert_eq!(deepseek_user.content, "Hello");

        let asst_msg = Message::assistant("Hi there!");
        let deepseek_asst = client.convert_message(&asst_msg);
        assert_eq!(deepseek_asst.role, "assistant");
        assert_eq!(deepseek_asst.content, "Hi there!");
    }

    #[test]
    fn test_message_conversion_tool_role() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-chat",
        );
        let client = DeepseekClient::new(config);

        let mut tool_msg = Message::human("tool result");
        tool_msg.role = MessageRole::Tool;

        let deepseek_msg = client.convert_message(&tool_msg);

        // Tool messages are converted to user role
        assert_eq!(deepseek_msg.role, "user");
        assert_eq!(deepseek_msg.content, "tool result");
    }

    // ============================================================
    // R1 Model Detection Tests
    // ============================================================

    #[test]
    fn test_r1_model_detection_variants() {
        // Test "reasoner" variant
        let config1 = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        let client1 = DeepseekClient::new(config1);
        assert!(client1.is_thinking_model());

        // Test "r1" variant
        let config2 = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-r1",
        );
        let client2 = DeepseekClient::new(config2);
        assert!(client2.is_thinking_model());

        // Test regular chat model
        let config3 = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-chat",
        );
        let client3 = DeepseekClient::new(config3);
        assert!(!client3.is_thinking_model());

        // Test coder model
        let config4 = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-coder",
        );
        let client4 = DeepseekClient::new(config4);
        assert!(!client4.is_thinking_model());
    }

    // ============================================================
    // Reasoning Extraction Tests
    // ============================================================

    #[test]
    fn test_reasoning_extraction_with_think_tags() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        let client = DeepseekClient::new(config);

        let content = "<think>Step 1: Analyze the problem\nStep 2: Consider alternatives\nStep 3: Choose best solution</think>The optimal solution is to use approach B.";
        let (answer, reasoning) = client.extract_reasoning(content);

        assert_eq!(answer, "The optimal solution is to use approach B.");
        assert!(reasoning.is_some());
        let reasoning_content = reasoning.unwrap();
        assert_eq!(
            reasoning_content.content,
            "Step 1: Analyze the problem\nStep 2: Consider alternatives\nStep 3: Choose best solution"
        );
    }

    #[test]
    fn test_reasoning_extraction_no_tags() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        let client = DeepseekClient::new(config);

        let content = "This is a regular response without thinking tags.";
        let (answer, reasoning) = client.extract_reasoning(content);

        assert_eq!(answer, "This is a regular response without thinking tags.");
        assert!(reasoning.is_none());
    }

    #[test]
    fn test_reasoning_extraction_empty_think() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        let client = DeepseekClient::new(config);

        let content = "<think></think>Answer with no reasoning.";
        let (answer, reasoning) = client.extract_reasoning(content);

        assert_eq!(answer, "Answer with no reasoning.");
        assert!(reasoning.is_some());
        assert_eq!(reasoning.unwrap().content, "");
    }

    // ============================================================
    // Response Conversion Tests
    // ============================================================

    #[test]
    fn test_response_conversion_basic() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-chat",
        );
        let client = DeepseekClient::new(config);

        let request = ChatRequest::new(vec![Message::human("Hello")]);

        let deepseek_response = DeepseekResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "deepseek-chat".to_string(),
            choices: vec![DeepseekChoice {
                index: 0,
                message: DeepseekMessage {
                    role: "assistant".to_string(),
                    content: "Hi there!".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(DeepseekUsage {
                prompt_tokens: 5,
                completion_tokens: 10,
                total_tokens: 15,
                reasoning_tokens: None,
            }),
        };

        let response = client.convert_response(&request, deepseek_response);

        assert_eq!(response.message.text(), Some("Hi there!"));
        assert_eq!(response.message.role, MessageRole::Assistant);
        assert_eq!(response.usage.as_ref().unwrap().input_tokens, 5);
        assert_eq!(response.usage.as_ref().unwrap().output_tokens, 10);
        assert!(response.metadata.contains_key("model"));
        assert!(response.metadata.contains_key("finish_reason"));
    }

    #[test]
    fn test_response_conversion_with_reasoning_tokens() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        let client = DeepseekClient::new(config);

        let mut request = ChatRequest::new(vec![Message::human("Solve this")]);
        request.config.reasoning_mode = ReasoningMode::Separated;

        let deepseek_response = DeepseekResponse {
            id: "chatcmpl-r1-456".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "deepseek-reasoner".to_string(),
            choices: vec![DeepseekChoice {
                index: 0,
                message: DeepseekMessage {
                    role: "assistant".to_string(),
                    content: "<think>Analyzing the problem...</think>Solution found.".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(DeepseekUsage {
                prompt_tokens: 10,
                completion_tokens: 50,
                total_tokens: 100,
                reasoning_tokens: Some(40),
            }),
        };

        let response = client.convert_response(&request, deepseek_response);

        assert_eq!(response.message.text(), Some("Solution found."));
        assert!(response.reasoning.is_some());
        assert_eq!(
            response.reasoning.as_ref().unwrap().content,
            "Analyzing the problem..."
        );
        assert_eq!(response.usage.as_ref().unwrap().input_tokens, 10);
        assert_eq!(response.usage.as_ref().unwrap().output_tokens, 50);
        assert_eq!(response.usage.as_ref().unwrap().reasoning_tokens, Some(40));
    }

    #[test]
    fn test_response_conversion_reasoning_mode_disabled() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        let client = DeepseekClient::new(config);

        let mut request = ChatRequest::new(vec![Message::human("Question")]);
        request.config.reasoning_mode = ReasoningMode::Disabled;

        let deepseek_response = DeepseekResponse {
            id: "chatcmpl-789".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "deepseek-reasoner".to_string(),
            choices: vec![DeepseekChoice {
                index: 0,
                message: DeepseekMessage {
                    role: "assistant".to_string(),
                    content: "<think>Hidden thinking</think>Answer".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, deepseek_response);

        // When reasoning mode is Disabled, should not extract reasoning
        assert!(response.reasoning.is_none());
        assert_eq!(
            response.message.text(),
            Some("<think>Hidden thinking</think>Answer")
        );
    }

    #[test]
    fn test_response_conversion_non_thinking_model() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-chat",
        );
        let client = DeepseekClient::new(config);

        let mut request = ChatRequest::new(vec![Message::human("Question")]);
        request.config.reasoning_mode = ReasoningMode::Separated;

        let deepseek_response = DeepseekResponse {
            id: "chatcmpl-999".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "deepseek-chat".to_string(),
            choices: vec![DeepseekChoice {
                index: 0,
                message: DeepseekMessage {
                    role: "assistant".to_string(),
                    content: "<think>Some text</think>Answer".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let response = client.convert_response(&request, deepseek_response);

        // Non-thinking models should not extract reasoning even if tags present
        assert!(response.reasoning.is_none());
        assert_eq!(
            response.message.text(),
            Some("<think>Some text</think>Answer")
        );
    }

    // ============================================================
    // Configuration Tests
    // ============================================================

    #[test]
    fn test_config_with_custom_timeout() {
        let mut config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        config.timeout = Duration::from_secs(90);

        let client = DeepseekClient::new(config.clone());
        assert_eq!(client.config.timeout, Duration::from_secs(90));
    }

    // ============================================================
    // Future Implementation Tests (Marked #[ignore])
    // ============================================================

    /// Test: Streaming support
    ///
    /// Verifies that Deepseek streaming returns token-by-token responses.
    ///
    /// NOTE: Currently ignored - streaming not yet implemented for Deepseek.
    /// See line 211-214 in chat implementation.
    #[tokio::test]
    #[ignore]
    async fn test_streaming_basic() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-chat",
        );
        let client = DeepseekClient::new(config);

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

    /// Test: Streaming with R1 reasoning
    ///
    /// Verifies that Deepseek R1 streaming includes reasoning tokens.
    ///
    /// NOTE: Currently ignored - streaming not yet implemented for Deepseek.
    #[tokio::test]
    #[ignore]
    async fn test_streaming_r1_reasoning() {
        let config = RemoteLlmConfig::new(
            "test-key",
            "https://api.deepseek.com",
            "deepseek-reasoner",
        );
        let client = DeepseekClient::new(config);

        let mut request = ChatRequest::new(vec![Message::human("Complex problem")]);
        request.config.reasoning_mode = ReasoningMode::Separated;

        // TODO: Once streaming is implemented for R1
        // Should stream thinking tokens separately from answer tokens
        // let stream = client.stream(request).await.unwrap();
        // Verify reasoning is streamed before answer
    }
}

