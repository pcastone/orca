//! Claude Code CLI client implementation.
//!
//! Provides integration with Claude Code CLI, allowing use of Claude Pro/Max
//! subscriptions via the `claude` command-line tool.
//!
//! # Requirements
//!
//! - Claude Code CLI must be installed (`claude` command available)
//! - User must be logged in (`claude login`)
//!
//! # Example
//!
//! ```rust,ignore
//! use llm::local::ClaudeCodeClient;
//! use llm::config::LocalLlmConfig;
//! use langgraph_core::llm::{ChatModel, ChatRequest};
//! use langgraph_core::Message;
//!
//! let config = LocalLlmConfig::new("claude", "claude-sonnet-4-5-20250514");
//! let client = ClaudeCodeClient::new(config);
//!
//! let request = ChatRequest::new(vec![Message::human("Hello!")]);
//! let response = client.chat(request).await?;
//! ```

use crate::config::LocalLlmConfig;
use crate::error::{LlmError, Result};
use async_trait::async_trait;
use langgraph_core::error::Result as GraphResult;
use langgraph_core::llm::{
    ChatModel, ChatRequest, ChatResponse, ChatStreamResponse,
};
use langgraph_core::llm_stream::{MessageChunk, MessageChunkStream};
use langgraph_core::{Message, MessageContent, MessageRole};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use futures::stream;

/// Claude Code CLI client for using Claude Pro/Max subscription.
#[derive(Clone)]
pub struct ClaudeCodeClient {
    config: LocalLlmConfig,
}

impl ClaudeCodeClient {
    /// Create a new Claude Code client with the given configuration.
    pub fn new(config: LocalLlmConfig) -> Self {
        Self { config }
    }

    /// Check if Claude Code CLI is available.
    pub async fn check_health(&self) -> Result<bool> {
        match Command::new("claude")
            .arg("--version")
            .output()
            .await
        {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    /// Build the prompt from messages.
    fn build_prompt(&self, messages: &[Message]) -> String {
        let mut prompt_parts = Vec::new();

        for msg in messages {
            let text = msg.text().unwrap_or("");
            match &msg.role {
                MessageRole::System => {
                    prompt_parts.push(format!("[System]: {}", text));
                }
                MessageRole::Human => {
                    prompt_parts.push(text.to_string());
                }
                MessageRole::Assistant => {
                    prompt_parts.push(format!("[Previous Assistant Response]: {}", text));
                }
                MessageRole::Tool => {
                    prompt_parts.push(format!("[Tool Result]: {}", text));
                }
                MessageRole::Custom(role) => {
                    prompt_parts.push(format!("[{}]: {}", role, text));
                }
            }
        }

        prompt_parts.join("\n\n")
    }

    /// Run claude CLI and get response.
    async fn run_claude(&self, prompt: &str) -> Result<String> {
        let mut cmd = Command::new("claude");

        // Use -p for print mode (non-interactive)
        cmd.arg("-p")
            .arg(prompt);

        // Add model if specified and not default
        if !self.config.model.is_empty() && self.config.model != "default" {
            cmd.arg("--model").arg(&self.config.model);
        }

        // Capture output
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().await.map_err(|e| {
            LlmError::ServiceUnavailable(format!("Failed to run claude CLI: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LlmError::ProviderError(format!(
                "Claude CLI error (exit {}): {}",
                output.status.code().unwrap_or(-1),
                stderr
            )));
        }

        let response = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(response.trim().to_string())
    }
}

#[async_trait]
impl ChatModel for ClaudeCodeClient {
    async fn chat(&self, request: ChatRequest) -> GraphResult<ChatResponse> {
        let prompt = self.build_prompt(&request.messages);

        let response_text = self.run_claude(&prompt).await?;

        let message = Message {
            id: None,
            role: MessageRole::Assistant,
            content: MessageContent::Text(response_text),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        };

        let mut metadata = HashMap::new();
        metadata.insert(
            "model".to_string(),
            serde_json::Value::String(self.config.model.clone()),
        );
        metadata.insert(
            "provider".to_string(),
            serde_json::Value::String("claude-code".to_string()),
        );

        Ok(ChatResponse {
            message,
            usage: None,  // Claude Code CLI doesn't report token usage
            reasoning: None,
            metadata,
        })
    }

    async fn stream(&self, request: ChatRequest) -> GraphResult<ChatStreamResponse> {
        // For simplicity, run non-streaming and return as single chunk
        let prompt = self.build_prompt(&request.messages);

        let response_text = self.run_claude(&prompt).await?;

        // Create a single chunk with the complete response
        let chunk = MessageChunk::new(response_text).final_chunk();

        // Create a stream that yields the single chunk
        let stream: MessageChunkStream = Box::pin(stream::once(async move { chunk }));

        let mut metadata = HashMap::new();
        metadata.insert(
            "model".to_string(),
            serde_json::Value::String(self.config.model.clone()),
        );

        Ok(ChatStreamResponse {
            stream,
            reasoning_stream: None,
            usage: None,
            metadata,
        })
    }

    async fn is_available(&self) -> GraphResult<bool> {
        Ok(self.check_health().await.unwrap_or(false))
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_health() {
        let config = LocalLlmConfig::new("claude", "default");
        let client = ClaudeCodeClient::new(config);

        // This will return false if claude CLI is not installed
        let _health = client.check_health().await;
    }

    #[test]
    fn test_build_prompt() {
        let config = LocalLlmConfig::new("claude", "default");
        let client = ClaudeCodeClient::new(config);

        let messages = vec![
            Message::system("You are helpful"),
            Message::human("Hello"),
        ];

        let prompt = client.build_prompt(&messages);
        assert!(prompt.contains("[System]: You are helpful"));
        assert!(prompt.contains("Hello"));
    }
}
