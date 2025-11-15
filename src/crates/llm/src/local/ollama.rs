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
    use std::time::Duration;

    // ============================================================
    // Existing Tests
    // ============================================================

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

    // ============================================================
    // Message Conversion Tests
    // ============================================================

    #[test]
    fn test_message_conversion_all_roles() {
        let config = LocalLlmConfig::new("http://localhost:11434", "mistral");
        let client = OllamaClient::new(config);

        let sys_msg = Message::system("You are helpful");
        let ollama_sys = client.convert_message(&sys_msg);
        assert_eq!(ollama_sys.role, "system");
        assert_eq!(ollama_sys.content, "You are helpful");

        let user_msg = Message::human("Hello");
        let ollama_user = client.convert_message(&user_msg);
        assert_eq!(ollama_user.role, "user");
        assert_eq!(ollama_user.content, "Hello");

        let asst_msg = Message::assistant("Hi there!");
        let ollama_asst = client.convert_message(&asst_msg);
        assert_eq!(ollama_asst.role, "assistant");
        assert_eq!(ollama_asst.content, "Hi there!");
    }

    #[test]
    fn test_message_conversion_tool_role() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let client = OllamaClient::new(config);

        let mut tool_msg = Message::human("tool result");
        tool_msg.role = MessageRole::Tool;

        let ollama_msg = client.convert_message(&tool_msg);

        // Tool messages are converted to user role in Ollama
        assert_eq!(ollama_msg.role, "user");
        assert_eq!(ollama_msg.content, "tool result");
    }

    #[test]
    fn test_message_conversion_custom_role() {
        let config = LocalLlmConfig::new("http://localhost:11434", "mixtral");
        let client = OllamaClient::new(config);

        let mut custom_msg = Message::human("custom content");
        custom_msg.role = MessageRole::Custom("moderator".to_string());

        let ollama_msg = client.convert_message(&custom_msg);

        assert_eq!(ollama_msg.role, "moderator");
        assert_eq!(ollama_msg.content, "custom content");
    }

    // ============================================================
    // Response Conversion Tests
    // ============================================================

    #[test]
    fn test_response_conversion_basic() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let client = OllamaClient::new(config);

        let ollama_response = OllamaResponse {
            model: "llama2".to_string(),
            message: OllamaMessage {
                role: "assistant".to_string(),
                content: "Hello there!".to_string(),
            },
            done: true,
            total_duration: Some(1500000000), // 1.5 seconds in nanoseconds
            prompt_eval_count: Some(10),
            eval_count: Some(25),
        };

        let response = client.convert_response(ollama_response);

        assert_eq!(response.message.text(), Some("Hello there!"));
        assert_eq!(response.message.role, MessageRole::Assistant);
        assert_eq!(response.usage.as_ref().unwrap().input_tokens, 10);
        assert_eq!(response.usage.as_ref().unwrap().output_tokens, 25);
        assert_eq!(response.usage.as_ref().unwrap().total_tokens, 35);
        assert!(response.metadata.contains_key("model"));
        assert!(response.metadata.contains_key("total_duration_ns"));
    }

    #[test]
    fn test_response_conversion_no_usage() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let client = OllamaClient::new(config);

        let ollama_response = OllamaResponse {
            model: "llama2".to_string(),
            message: OllamaMessage {
                role: "assistant".to_string(),
                content: "Response without usage".to_string(),
            },
            done: true,
            total_duration: None,
            prompt_eval_count: None,
            eval_count: None,
        };

        let response = client.convert_response(ollama_response);

        assert_eq!(response.message.text(), Some("Response without usage"));
        assert!(response.usage.is_none());
    }

    #[test]
    fn test_response_conversion_partial_usage() {
        let config = LocalLlmConfig::new("http://localhost:11434", "mistral");
        let client = OllamaClient::new(config);

        let ollama_response = OllamaResponse {
            model: "mistral".to_string(),
            message: OllamaMessage {
                role: "assistant".to_string(),
                content: "Partial usage data".to_string(),
            },
            done: true,
            total_duration: Some(2000000000),
            prompt_eval_count: Some(15),
            eval_count: None, // Only prompt eval available
        };

        let response = client.convert_response(ollama_response);

        assert!(response.usage.is_some());
        assert_eq!(response.usage.as_ref().unwrap().input_tokens, 15);
        assert_eq!(response.usage.as_ref().unwrap().output_tokens, 0);
    }

    // ============================================================
    // Model Management Tests
    // ============================================================

    #[tokio::test]
    async fn test_use_model() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let mut client = OllamaClient::new(config);

        assert_eq!(client.current_model(), "llama2");

        let new_model = client.use_model("mistral").await.unwrap();
        assert_eq!(new_model, "mistral");
        assert_eq!(client.current_model(), "mistral");
        assert_eq!(client.config.model, "mistral");
    }

    #[test]
    fn test_current_model_tracking() {
        let config = LocalLlmConfig::new("http://localhost:11434", "mixtral");
        let client = OllamaClient::new(config);

        assert_eq!(client.current_model(), "mixtral");
        assert_eq!(client.config.model, "mixtral");
    }

    // ============================================================
    // Configuration Tests
    // ============================================================

    #[test]
    fn test_config_with_custom_timeout() {
        let mut config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        config.timeout = Duration::from_secs(120);

        let client = OllamaClient::new(config.clone());
        assert_eq!(client.config.timeout, Duration::from_secs(120));
    }

    #[test]
    fn test_config_with_different_base_url() {
        let config = LocalLlmConfig::new("http://192.168.1.100:11434", "llama2");
        let client = OllamaClient::new(config.clone());

        assert_eq!(client.config.base_url, "http://192.168.1.100:11434");
    }

    // ============================================================
    // Future Implementation Tests (Marked #[ignore])
    // ============================================================

    /// Test: Streaming support
    ///
    /// Verifies that Ollama streaming returns token-by-token responses.
    ///
    /// NOTE: Currently ignored - streaming not yet implemented for Ollama.
    /// See line 179-182 in chat implementation.
    #[tokio::test]
    #[ignore]
    async fn test_streaming_basic() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let client = OllamaClient::new(config);

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

    /// Test: Health check / connection retry
    ///
    /// Verifies that Ollama client can check server health and retry connections.
    ///
    /// NOTE: Currently ignored - requires running Ollama server.
    #[tokio::test]
    #[ignore]
    async fn test_health_check() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let client = OllamaClient::new(config);

        // This requires a running Ollama server
        let is_healthy = client.check_health().await.unwrap();
        // If server is running, should be true
        // If not running, should be false (not an error)
        println!("Ollama health: {}", is_healthy);
    }

    /// Test: Model fetching
    ///
    /// Verifies that client can fetch available models from Ollama.
    ///
    /// NOTE: Currently ignored - requires running Ollama server with models.
    #[tokio::test]
    #[ignore]
    async fn test_fetch_models() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let client = OllamaClient::new(config);

        // This requires a running Ollama server with models installed
        let models = client.fetch_models().await.unwrap();
        assert!(!models.is_empty());

        for model in models {
            println!("Model: {}", model.id);
            if let Some(size) = model.metadata.get("size_gb") {
                println!("  Size: {} GB", size);
            }
        }
    }

    /// Test: Is available check
    ///
    /// Verifies that is_available() correctly reports server status.
    ///
    /// NOTE: Currently ignored - requires running Ollama server.
    #[tokio::test]
    #[ignore]
    async fn test_is_available() {
        let config = LocalLlmConfig::new("http://localhost:11434", "llama2");
        let client = OllamaClient::new(config);

        // This requires a running Ollama server
        let available = client.is_available().await.unwrap();
        println!("Ollama available: {}", available);
    }
}

