//! Context Manager for LLM conversation history
//!
//! Manages context windows, token counts, and message trimming for LLM interactions.

use crate::context::token_counter::{TokenCount, TokenCounter};
use crate::context::trimmer::{ContextTrimmer, TrimStrategy};
use langgraph_core::messages::Message;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Context window limits for different models
#[derive(Debug, Clone, Copy)]
pub struct ContextLimits {
    /// Maximum tokens for the model
    pub max_tokens: usize,
    /// Reserved tokens for system prompt
    pub system_reserved: usize,
    /// Reserved tokens for response
    pub response_reserved: usize,
}

impl ContextLimits {
    /// Create context limits for a specific model
    pub fn for_model(model: &str) -> Self {
        let max_tokens = if model.contains("gpt-4-turbo") || model.contains("gpt-4-32k") {
            128000
        } else if model.contains("gpt-4") {
            8192
        } else if model.contains("gpt-3.5-turbo-16k") {
            16384
        } else if model.contains("gpt-3.5") {
            4096
        } else if model.contains("claude-3") {
            200000
        } else if model.contains("claude-2") {
            100000
        } else if model.contains("claude") {
            9000
        } else {
            // Default conservative limit
            4096
        };

        Self {
            max_tokens,
            system_reserved: 500,
            response_reserved: 1000,
        }
    }

    /// Get available tokens for conversation history
    pub fn available_for_history(&self) -> usize {
        self.max_tokens
            .saturating_sub(self.system_reserved)
            .saturating_sub(self.response_reserved)
    }
}

/// Context Manager for managing LLM conversation history
pub struct ContextManager {
    /// Model name
    model: String,
    /// Token counter
    counter: TokenCounter,
    /// Context trimmer
    trimmer: ContextTrimmer,
    /// Context limits
    limits: ContextLimits,
    /// Message history
    messages: Arc<RwLock<Vec<Message>>>,
    /// System prompt
    system_prompt: Arc<RwLock<Option<String>>>,
}

impl ContextManager {
    /// Create a new context manager for a specific model
    pub fn new(model: impl Into<String>) -> Self {
        let model = model.into();
        let limits = ContextLimits::for_model(&model);
        let available_tokens = limits.available_for_history();

        Self {
            counter: TokenCounter::new(&model),
            trimmer: ContextTrimmer::new(&model, available_tokens),
            limits,
            model,
            messages: Arc::new(RwLock::new(Vec::new())),
            system_prompt: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with custom context limits
    pub fn with_limits(mut self, limits: ContextLimits) -> Self {
        self.limits = limits;
        let available_tokens = limits.available_for_history();
        self.trimmer = ContextTrimmer::new(&self.model, available_tokens);
        self
    }

    /// Set trim strategy
    pub fn with_trim_strategy(mut self, strategy: TrimStrategy) -> Self {
        self.trimmer = self.trimmer.with_strategy(strategy);
        self
    }

    /// Set system prompt
    pub async fn set_system_prompt(&self, prompt: impl Into<String>) {
        *self.system_prompt.write().await = Some(prompt.into());
    }

    /// Get system prompt
    pub async fn get_system_prompt(&self) -> Option<String> {
        self.system_prompt.read().await.clone()
    }

    /// Add a message to the history
    pub async fn add_message(&self, message: Message) {
        let mut messages = self.messages.write().await;
        messages.push(message);
    }

    /// Add multiple messages to the history
    pub async fn add_messages(&self, new_messages: Vec<Message>) {
        let mut messages = self.messages.write().await;
        messages.extend(new_messages);
    }

    /// Get all messages (possibly trimmed)
    pub async fn get_messages(&self) -> Vec<Message> {
        let messages = self.messages.read().await;
        self.trimmer.trim_messages(&messages)
    }

    /// Get all messages without trimming
    pub async fn get_all_messages(&self) -> Vec<Message> {
        self.messages.read().await.clone()
    }

    /// Clear all messages
    pub async fn clear_messages(&self) {
        self.messages.write().await.clear();
    }

    /// Get current token count
    pub async fn get_token_count(&self) -> TokenCount {
        let messages = self.messages.read().await;
        let mut total = self.counter.count_messages(&messages);

        // Add system prompt tokens if present
        if let Some(prompt) = &*self.system_prompt.read().await {
            total.add(self.counter.count_system_prompt(prompt));
        }

        total
    }

    /// Check if adding a message would exceed limits
    pub async fn can_add_message(&self, message: &Message) -> bool {
        let current_count = self.get_token_count().await;
        let message_count = self.counter.count_message(message);
        
        current_count.tokens + message_count.tokens + self.limits.response_reserved 
            <= self.limits.max_tokens
    }

    /// Get remaining tokens available
    pub async fn remaining_tokens(&self) -> usize {
        let current_count = self.get_token_count().await;
        self.limits.max_tokens.saturating_sub(current_count.tokens)
            .saturating_sub(self.limits.response_reserved)
    }

    /// Estimate tokens for a tool response
    pub fn estimate_tool_response_tokens(&self, response: &Value) -> TokenCount {
        self.counter.count_tool_response(response)
    }

    /// Get context limits
    pub fn limits(&self) -> ContextLimits {
        self.limits
    }

    /// Get model name
    pub fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use langgraph_core::messages::MessageRole;

    #[tokio::test]
    async fn test_context_manager_creation() {
        let manager = ContextManager::new("gpt-4");
        assert_eq!(manager.model(), "gpt-4");
        assert_eq!(manager.limits().max_tokens, 8192);
    }

    #[tokio::test]
    async fn test_add_and_get_messages() {
        let manager = ContextManager::new("gpt-4");
        
        manager.add_message(Message::human("Hello")).await;
        manager.add_message(Message::ai("Hi")).await;
        
        let messages = manager.get_all_messages().await;
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_system_prompt() {
        let manager = ContextManager::new("gpt-4");
        
        manager.set_system_prompt("You are a helpful assistant").await;
        let prompt = manager.get_system_prompt().await;
        
        assert_eq!(prompt, Some("You are a helpful assistant".to_string()));
    }

    #[tokio::test]
    async fn test_token_counting() {
        let manager = ContextManager::new("gpt-4");
        
        manager.add_message(Message::human("Hello, world!")).await;
        let count = manager.get_token_count().await;
        
        assert!(count.tokens > 0);
    }

    #[tokio::test]
    async fn test_can_add_message() {
        let manager = ContextManager::new("gpt-4");
        let message = Message::human("Test message");
        
        let can_add = manager.can_add_message(&message).await;
        assert!(can_add);
    }

    #[tokio::test]
    async fn test_clear_messages() {
        let manager = ContextManager::new("gpt-4");
        
        manager.add_message(Message::human("Hello")).await;
        assert_eq!(manager.get_all_messages().await.len(), 1);
        
        manager.clear_messages().await;
        assert_eq!(manager.get_all_messages().await.len(), 0);
    }

    #[tokio::test]
    async fn test_remaining_tokens() {
        let manager = ContextManager::new("gpt-4");
        let remaining = manager.remaining_tokens().await;
        
        // Should have most of the context window available
        assert!(remaining > 6000);
    }

    #[tokio::test]
    async fn test_context_limits() {
        let limits = ContextLimits::for_model("gpt-4");
        assert_eq!(limits.max_tokens, 8192);
        
        let claude_limits = ContextLimits::for_model("claude-3");
        assert_eq!(claude_limits.max_tokens, 200000);
    }
}
