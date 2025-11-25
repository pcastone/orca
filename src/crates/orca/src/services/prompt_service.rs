//! Prompt Service - Shared LLM prompt functionality
//!
//! Provides a unified interface for sending prompts to LLM providers.
//! Uses agent-based execution (ReAct pattern) for enhanced reasoning.
//! Used by both orca CLI/TUI and orchestrator-server.

use crate::config::OrcaConfig;
use crate::error::{OrcaError, Result};
use crate::executor::TaskExecutor;
use crate::tools::DirectToolBridge;
use crate::workflow::Task;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Service for sending prompts to LLM providers using agent-based execution
pub struct PromptService {
    executor: TaskExecutor,
}

impl PromptService {
    /// Create a new PromptService from configuration
    ///
    /// # Arguments
    /// * `config` - Orca configuration with LLM settings
    ///
    /// # Returns
    /// A PromptService instance configured with agent-based execution
    pub fn new(config: &OrcaConfig) -> Result<Self> {
        // Create DirectToolBridge (currently stub, will enable tools when implemented)
        let workspace_root = config.execution.workspace_root.clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let session_id = format!("prompt-{}", uuid::Uuid::new_v4());
        let bridge = Arc::new(DirectToolBridge::new(workspace_root, session_id)?);

        // Create TaskExecutor with bridge and config
        let executor = TaskExecutor::new(bridge, config.clone())?;

        Ok(Self {
            executor,
        })
    }

    /// Send a prompt to the LLM using agent-based execution (ReAct pattern)
    ///
    /// # Arguments
    /// * `prompt` - The user prompt to send
    ///
    /// # Returns
    /// The agent's response as a String
    pub async fn send_prompt(&self, prompt: &str) -> Result<String> {
        if prompt.is_empty() {
            return Err(OrcaError::Config("Prompt cannot be empty".to_string()));
        }

        info!("Executing prompt with agent: {}...", &prompt[..prompt.len().min(50)]);

        // Create a temporary task for the prompt
        let task = Task::new(prompt);

        debug!(
            task_id = %task.id,
            pattern = "react",
            "Created temporary task for prompt execution"
        );

        // Execute the task using agent system
        let result = self.executor.execute_task(&task).await?;

        // Check if execution was successful
        if !result.success {
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            warn!("Agent execution failed: {}", error_msg);
            return Err(OrcaError::Execution(format!("Agent execution failed: {}", error_msg)));
        }

        // Extract the result
        let response_text = result.result.unwrap_or_else(|| {
            // Fallback: try to extract from final state or messages
            if let Some(last_msg) = result.messages.last() {
                last_msg.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                "No response generated".to_string()
            }
        });

        if !response_text.is_empty() {
            info!("Agent response: {}...", &response_text[..response_text.len().min(50)]);
        }

        Ok(response_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LlmConfig;

    #[test]
    fn test_prompt_service_creates_correctly() {
        let config = OrcaConfig {
            llm: LlmConfig {
                provider: "ollama".to_string(),
                model: "llama2".to_string(),
                api_key: None,
                api_base: Some("http://localhost:11434".to_string()),
                temperature: 0.7,
                max_tokens: 1000,
            },
            ..Default::default()
        };

        let result = PromptService::new(&config);
        assert!(result.is_ok(), "PromptService should create with valid config");
    }

    #[tokio::test]
    async fn test_prompt_service_empty_prompt_error() {
        let config = OrcaConfig {
            llm: LlmConfig {
                provider: "ollama".to_string(),
                model: "llama2".to_string(),
                api_key: None,
                api_base: Some("http://localhost:11434".to_string()),
                temperature: 0.7,
                max_tokens: 1000,
            },
            ..Default::default()
        };

        let service = PromptService::new(&config).unwrap();
        let result = service.send_prompt("").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_prompt_service_requires_valid_provider() {
        let config = OrcaConfig {
            llm: LlmConfig {
                provider: "invalid_provider".to_string(),
                model: "model".to_string(),
                api_key: None,
                api_base: None,
                temperature: 0.7,
                max_tokens: 1000,
            },
            ..Default::default()
        };

        let result = PromptService::new(&config);
        assert!(result.is_err(), "Should fail with invalid provider");
    }
}
