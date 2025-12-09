//! Conversation Service - Stateful conversation management with context summarization
//!
//! Provides conversation continuity with automatic context management to prevent
//! token overflow in long-running sessions. Uses the ContextManager from langgraph-core
//! for token counting and automatic summarization.
//!
//! LLM configuration is now database-only. Callers must provide an LlmProvider loaded
//! from the database llm_providers table.

use crate::config::OrcaConfig;
use crate::error::{OrcaError, Result};
use crate::executor::{create_llm_function, LlmProvider, ToolAdapter};
use crate::tools::DirectToolBridge;
use langgraph_core::context::{ContextConfig, ContextManager, SummarizationResult};
use langgraph_core::llm::ChatModel;
use langgraph_core::messages::Message;
use langgraph_prebuilt::agents::create_react_agent;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Service for managing stateful conversations with automatic context summarization
pub struct ConversationService {
    /// Configuration
    config: OrcaConfig,

    /// Direct tool bridge for tool execution
    bridge: Arc<DirectToolBridge>,

    /// LLM provider for agent reasoning
    llm_provider: Arc<LlmProvider>,

    /// Context manager for token counting and summarization
    context_manager: Arc<ContextManager>,

    /// Conversation history (protected by RwLock for async safety)
    messages: Arc<RwLock<Vec<Message>>>,

    /// System prompt (if any)
    system_prompt: Option<String>,
}

impl ConversationService {
    /// Create a new ConversationService with default context settings
    ///
    /// LLM configuration is now database-only. Callers must provide an LlmProvider
    /// loaded from the database llm_providers table.
    ///
    /// # Arguments
    /// * `config` - Orca configuration (execution settings, not LLM)
    /// * `llm_provider` - LLM provider loaded from database
    ///
    /// # Returns
    /// A ConversationService with automatic context management
    pub fn new(config: &OrcaConfig, llm_provider: Arc<LlmProvider>) -> Result<Self> {
        // Use context window size (typically 128k), not response max_tokens (typically 4096)
        let context_window = 128_000; // Default context window for modern LLMs

        let context_config = ContextConfig::default()
            .with_max_tokens(context_window)
            .with_threshold(0.8)
            .with_preserve_recent(10);

        Self::with_context_config(config, llm_provider, context_config)
    }

    /// Create a new ConversationService with custom context configuration
    ///
    /// # Arguments
    /// * `config` - Orca configuration (execution settings, not LLM)
    /// * `llm_provider` - LLM provider loaded from database
    /// * `context_config` - Custom context configuration
    ///
    /// # Returns
    /// A ConversationService with specified context settings
    pub fn with_context_config(config: &OrcaConfig, llm_provider: Arc<LlmProvider>, context_config: ContextConfig) -> Result<Self> {
        // Create DirectToolBridge
        let workspace_root = config
            .execution
            .workspace_root
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let session_id = format!("conversation-{}", uuid::Uuid::new_v4());
        let bridge = Arc::new(DirectToolBridge::new(workspace_root, session_id)?);

        // Create context manager
        let context_manager = Arc::new(ContextManager::new(context_config));

        Ok(Self {
            config: config.clone(),
            bridge,
            llm_provider,
            context_manager,
            messages: Arc::new(RwLock::new(Vec::new())),
            system_prompt: None,
        })
    }

    /// Set a system prompt for the conversation
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add an LLM for context summarization (enables LLM-based summarization instead of truncation)
    pub fn with_summarizer(self, _llm: Arc<dyn ChatModel>) -> Self {
        // Note: The ContextManager would need to be modified to accept an LLM for summarization
        // For now, we use truncation-based summarization
        self
    }

    /// Send a message and get a response, maintaining conversation history
    ///
    /// # Arguments
    /// * `user_message` - The user's message
    ///
    /// # Returns
    /// The assistant's response
    pub async fn send_message(&self, user_message: &str) -> Result<String> {
        if user_message.is_empty() {
            return Err(OrcaError::Config("Message cannot be empty".to_string()));
        }

        // Add user message to history
        {
            let mut messages = self.messages.write().await;

            // Add system prompt if first message and system prompt is set
            if messages.is_empty() {
                if let Some(ref prompt) = self.system_prompt {
                    messages.push(Message::system(prompt.clone()));
                }
            }

            messages.push(Message::human(user_message.to_string()));
        }

        // Check and summarize context if needed
        let summarization_result = self.maybe_summarize_context().await;
        if summarization_result.summarized {
            info!(
                tokens_before = summarization_result.tokens_before,
                tokens_after = summarization_result.tokens_after,
                "Context was summarized to prevent token overflow"
            );
        }

        // Execute with agent
        let response = self.execute_agent().await?;

        // Add assistant response to history
        {
            let mut messages = self.messages.write().await;
            messages.push(Message::assistant(response.clone()));
        }

        Ok(response)
    }

    /// Get current token count for the conversation
    pub async fn get_token_count(&self) -> usize {
        let messages = self.messages.read().await;
        self.context_manager.count_tokens(&messages)
    }

    /// Get context statistics
    pub async fn get_context_stats(&self) -> (usize, usize, f32) {
        let messages = self.messages.read().await;
        self.context_manager.get_stats(&messages)
    }

    /// Force context summarization
    pub async fn force_summarize(&self) -> SummarizationResult {
        let mut messages = self.messages.write().await;
        self.context_manager.maybe_summarize(&mut messages).await
    }

    /// Clear conversation history
    pub async fn clear_history(&self) {
        let mut messages = self.messages.write().await;
        messages.clear();
        info!("Conversation history cleared");
    }

    /// Get the current message count
    pub async fn message_count(&self) -> usize {
        let messages = self.messages.read().await;
        messages.len()
    }

    /// Check and optionally summarize context
    async fn maybe_summarize_context(&self) -> SummarizationResult {
        let mut messages = self.messages.write().await;
        self.context_manager.maybe_summarize(&mut messages).await
    }

    /// Execute the agent with current conversation history
    async fn execute_agent(&self) -> Result<String> {
        // Create tools from bridge
        let tools = ToolAdapter::from_bridge(self.bridge.clone());

        // Create LLM function
        let llm_fn = create_llm_function(self.llm_provider.clone());

        // Build agent
        let agent = create_react_agent(llm_fn, tools)
            .with_max_iterations(self.config.execution.max_iterations)
            .build()
            .map_err(|e| OrcaError::Execution(format!("Failed to build agent: {}", e)))?;

        // Prepare state with conversation history
        let messages = self.messages.read().await;
        let messages_json: Vec<Value> = messages
            .iter()
            .map(|m| message_to_json(m))
            .collect();

        let initial_state = json!({
            "messages": messages_json
        });

        drop(messages); // Release lock before async operation

        debug!(
            message_count = messages_json.len(),
            "Invoking agent with conversation history"
        );

        // Execute agent
        let final_state = agent
            .invoke(initial_state)
            .await
            .map_err(|e| OrcaError::Execution(format!("Agent execution failed: {}", e)))?;

        // Extract final AI message
        let result = final_state
            .get("messages")
            .and_then(|m| m.as_array())
            .and_then(|messages| {
                messages.iter().rev().find(|msg| {
                    msg.get("type")
                        .or_else(|| msg.get("role"))
                        .and_then(|t| t.as_str())
                        .map(|t| t == "ai" || t == "assistant")
                        .unwrap_or(false)
                })
            })
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("No response generated")
            .to_string();

        Ok(result)
    }
}

/// Convert a Message to JSON Value for agent state
fn message_to_json(message: &Message) -> Value {
    use langgraph_core::messages::MessageRole;

    let msg_type = match message.role {
        MessageRole::System => "system",
        MessageRole::Human => "human",
        MessageRole::Assistant => "ai",
        MessageRole::Tool => "tool",
        MessageRole::Custom(_) => "custom",
    };

    let content = message.text().unwrap_or("").to_string();

    json!({
        "type": msg_type,
        "content": content
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use langgraph_core::messages::MessageRole;

    #[test]
    fn test_context_config_default() {
        let context_config = ContextConfig::default()
            .with_max_tokens(100_000)
            .with_threshold(0.8);

        assert_eq!(context_config.max_tokens, 100_000);
        assert!((context_config.summarization_threshold - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_message_to_json_human() {
        let msg = Message::human("Hello there");
        let json = message_to_json(&msg);

        assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("human"));
        assert_eq!(json.get("content").and_then(|v| v.as_str()), Some("Hello there"));
    }

    #[test]
    fn test_message_to_json_assistant() {
        let msg = Message::assistant("Hi! How can I help?");
        let json = message_to_json(&msg);

        assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("ai"));
        assert_eq!(
            json.get("content").and_then(|v| v.as_str()),
            Some("Hi! How can I help?")
        );
    }

    #[test]
    fn test_message_to_json_system() {
        let msg = Message::system("You are a helpful assistant.");
        let json = message_to_json(&msg);

        assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("system"));
        assert_eq!(
            json.get("content").and_then(|v| v.as_str()),
            Some("You are a helpful assistant.")
        );
    }

    #[test]
    fn test_message_to_json_tool() {
        let msg = Message::new(MessageRole::Tool, "Tool result here");
        let json = message_to_json(&msg);

        assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("tool"));
    }

    #[test]
    fn test_context_config_builder_chain() {
        let config = ContextConfig::default()
            .with_max_tokens(200_000)
            .with_threshold(0.9)
            .with_preserve_recent(20)
            .with_preserve_system(false);

        assert_eq!(config.max_tokens, 200_000);
        assert!((config.summarization_threshold - 0.9).abs() < 0.01);
        assert_eq!(config.preserve_recent_count, 20);
        assert!(!config.preserve_system_message);
    }

    #[test]
    fn test_context_config_threshold_clamping() {
        // Test that threshold is clamped to [0.1, 0.99]
        let config_low = ContextConfig::default().with_threshold(0.0);
        assert!((config_low.summarization_threshold - 0.1).abs() < 0.01);

        let config_high = ContextConfig::default().with_threshold(1.0);
        assert!((config_high.summarization_threshold - 0.99).abs() < 0.01);
    }

    #[test]
    fn test_context_config_trigger_threshold_calculation() {
        let config = ContextConfig::default()
            .with_max_tokens(100_000)
            .with_threshold(0.75);

        // 100,000 * 0.75 = 75,000
        assert_eq!(config.trigger_threshold(), 75_000);
    }

    #[test]
    fn test_context_config_target_after_summarization() {
        let config = ContextConfig::default()
            .with_max_tokens(100_000);

        // Default target_ratio_after_summarization is 0.5
        // 100,000 * 0.5 = 50,000
        assert_eq!(config.target_after_summarization(), 50_000);
    }
}
