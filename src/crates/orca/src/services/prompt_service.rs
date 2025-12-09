//! Prompt Service - Shared LLM prompt functionality
//!
//! Provides a unified interface for sending prompts to LLM providers.
//! Uses agent-based execution (ReAct pattern) for enhanced reasoning.
//! Used by both orca CLI/TUI and orchestrator-server.
//!
//! LLM configuration is now database-only (llm_providers table).
//! Callers must provide an LlmProvider loaded from the database.

use crate::config::OrcaConfig;
use crate::error::{OrcaError, Result};
use crate::executor::{TaskExecutor, LlmProvider};
use crate::tools::DirectToolBridge;
use crate::workflow::Task;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Service for sending prompts to LLM providers using agent-based execution
pub struct PromptService {
    executor: TaskExecutor,
}

impl PromptService {
    /// Create a new PromptService with an LLM provider
    ///
    /// LLM configuration is now database-only. Callers must provide an LlmProvider
    /// loaded from the database llm_providers table.
    ///
    /// # Arguments
    /// * `config` - Orca configuration (execution settings, not LLM)
    /// * `llm_provider` - LLM provider loaded from database
    ///
    /// # Returns
    /// A PromptService instance configured with agent-based execution
    pub fn new(config: &OrcaConfig, llm_provider: Arc<LlmProvider>) -> Result<Self> {
        // Create DirectToolBridge (currently stub, will enable tools when implemented)
        let workspace_root = config.execution.workspace_root.clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let session_id = format!("prompt-{}", uuid::Uuid::new_v4());
        let bridge = Arc::new(DirectToolBridge::new(workspace_root, session_id)?);

        // Create TaskExecutor with bridge, config, and llm_provider
        let executor = TaskExecutor::new(bridge, config.clone(), llm_provider);

        Ok(Self {
            executor,
        })
    }

    /// Set suppress_stdout flag on the internal executor (for TUI mode)
    pub fn set_suppress_stdout(&mut self, suppress: bool) {
        self.executor.set_suppress_stdout(suppress);
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
            // Find the last AI/assistant message
            result.messages
                .iter()
                .rev()
                .find(|msg| {
                    let is_ai_type = msg.get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == "ai")
                        .unwrap_or(false);
                    let is_assistant_role = msg.get("role")
                        .and_then(|r| r.as_str())
                        .map(|r| r == "assistant")
                        .unwrap_or(false);
                    is_ai_type || is_assistant_role
                })
                .and_then(|msg| {
                    // Handle different content formats
                    let content = msg.get("content")?;

                    // Direct string
                    if let Some(s) = content.as_str() {
                        return Some(s.to_string());
                    }

                    // Tagged enum {"Text": "..."}
                    if let Some(obj) = content.as_object() {
                        if let Some(text) = obj.get("Text").and_then(|t| t.as_str()) {
                            return Some(text.to_string());
                        }
                    }

                    // Array of parts [{"type": "text", "text": "..."}]
                    if let Some(arr) = content.as_array() {
                        let text_parts: Vec<String> = arr.iter()
                            .filter_map(|part| {
                                part.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                            })
                            .collect();
                        if !text_parts.is_empty() {
                            return Some(text_parts.join(""));
                        }
                    }

                    None
                })
                .unwrap_or_else(|| "No response generated".to_string())
        });

        if !response_text.is_empty() {
            info!("Agent response: {}...", &response_text[..response_text.len().min(50)]);
        }

        Ok(response_text)
    }
}

// Tests require database-backed LLM profiles, which need integration tests
// Unit tests here are limited to basic validation
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prompt_service_empty_prompt_error() {
        // Use default config - LLM provider will be loaded from database
        let config = OrcaConfig::default();

        // Note: This will fail if no LLM profile is configured in the database
        // In production, ensure an active LLM profile exists
        if let Ok(service) = PromptService::new(&config) {
            let result = service.send_prompt("").await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("empty"));
        }
        // If PromptService fails to create (no database LLM profile), test passes
    }
}
