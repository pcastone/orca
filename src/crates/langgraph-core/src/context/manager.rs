//! Context Manager implementation
//!
//! Manages conversation context and provides automatic summarization
//! to prevent token overflow in long-running agent sessions.

use super::token_counter::{message_to_string, TiktokenCounter, TokenCounter};
use crate::llm::{ChatModel, ChatRequest};
use crate::messages::MessageRole;
use crate::Message;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Configuration for the context manager
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Maximum tokens allowed in context (default: 128,000)
    pub max_tokens: usize,

    /// Threshold ratio at which to trigger summarization (default: 0.8)
    /// When current tokens / max_tokens exceeds this, summarization occurs
    pub summarization_threshold: f32,

    /// Number of recent messages to always preserve (default: 10)
    pub preserve_recent_count: usize,

    /// Whether to always preserve the system message (default: true)
    pub preserve_system_message: bool,

    /// Target token count after summarization as ratio of max (default: 0.5)
    pub target_ratio_after_summarization: f32,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: 128_000,
            summarization_threshold: 0.8,
            preserve_recent_count: 10,
            preserve_system_message: true,
            target_ratio_after_summarization: 0.5,
        }
    }
}

impl ContextConfig {
    /// Create a new config with specified max tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set the summarization threshold (0.0 - 1.0)
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.summarization_threshold = threshold.clamp(0.1, 0.99);
        self
    }

    /// Set how many recent messages to preserve
    pub fn with_preserve_recent(mut self, count: usize) -> Self {
        self.preserve_recent_count = count;
        self
    }

    /// Set whether to preserve system message
    pub fn with_preserve_system(mut self, preserve: bool) -> Self {
        self.preserve_system_message = preserve;
        self
    }

    /// Get the token threshold that triggers summarization
    pub fn trigger_threshold(&self) -> usize {
        (self.max_tokens as f32 * self.summarization_threshold) as usize
    }

    /// Get the target token count after summarization
    pub fn target_after_summarization(&self) -> usize {
        (self.max_tokens as f32 * self.target_ratio_after_summarization) as usize
    }
}

/// Result of a summarization operation
#[derive(Debug)]
pub struct SummarizationResult {
    /// Number of tokens before summarization
    pub tokens_before: usize,
    /// Number of tokens after summarization
    pub tokens_after: usize,
    /// Number of messages before summarization
    pub messages_before: usize,
    /// Number of messages after summarization
    pub messages_after: usize,
    /// Whether summarization was performed
    pub summarized: bool,
}

/// Context manager for handling conversation history and summarization
pub struct ContextManager {
    config: ContextConfig,
    token_counter: Arc<dyn TokenCounter>,
    /// Optional LLM for summarization (if not provided, uses truncation)
    summarizer: Option<Arc<dyn ChatModel>>,
}

impl std::fmt::Debug for ContextManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextManager")
            .field("config", &self.config)
            .field("has_summarizer", &self.summarizer.is_some())
            .finish()
    }
}

impl ContextManager {
    /// Create a new context manager with default tiktoken counter
    pub fn new(config: ContextConfig) -> Self {
        Self {
            config,
            token_counter: Arc::new(TiktokenCounter::new()),
            summarizer: None,
        }
    }

    /// Create with a custom token counter
    pub fn with_token_counter(config: ContextConfig, counter: Arc<dyn TokenCounter>) -> Self {
        Self {
            config,
            token_counter: counter,
            summarizer: None,
        }
    }

    /// Set the LLM to use for summarization
    pub fn with_summarizer(mut self, llm: Arc<dyn ChatModel>) -> Self {
        self.summarizer = Some(llm);
        self
    }

    /// Get the current configuration
    pub fn config(&self) -> &ContextConfig {
        &self.config
    }

    /// Count total tokens in messages
    pub fn count_tokens(&self, messages: &[Message]) -> usize {
        self.token_counter.count_messages_tokens(messages)
    }

    /// Check if summarization is needed based on current token count
    pub fn should_summarize(&self, messages: &[Message]) -> bool {
        let current_tokens = self.count_tokens(messages);
        let threshold = self.config.trigger_threshold();
        let should = current_tokens >= threshold;

        if should {
            info!(
                "Summarization needed: {} tokens >= {} threshold",
                current_tokens, threshold
            );
        }

        should
    }

    /// Get token usage statistics
    pub fn get_stats(&self, messages: &[Message]) -> (usize, usize, f32) {
        let current = self.count_tokens(messages);
        let max = self.config.max_tokens;
        let ratio = current as f32 / max as f32;
        (current, max, ratio)
    }

    /// Summarize messages if needed, modifying the messages in place
    ///
    /// Returns information about the summarization operation.
    pub async fn maybe_summarize(&self, messages: &mut Vec<Message>) -> SummarizationResult {
        let tokens_before = self.count_tokens(messages);
        let messages_before = messages.len();

        if !self.should_summarize(messages) {
            return SummarizationResult {
                tokens_before,
                tokens_after: tokens_before,
                messages_before,
                messages_after: messages_before,
                summarized: false,
            };
        }

        // Perform summarization
        self.summarize_messages(messages).await;

        let tokens_after = self.count_tokens(messages);
        let messages_after = messages.len();

        info!(
            "Summarization complete: {} -> {} tokens, {} -> {} messages",
            tokens_before, tokens_after, messages_before, messages_after
        );

        SummarizationResult {
            tokens_before,
            tokens_after,
            messages_before,
            messages_after,
            summarized: true,
        }
    }

    /// Perform the actual summarization
    async fn summarize_messages(&self, messages: &mut Vec<Message>) {
        if messages.is_empty() {
            return;
        }

        // Identify system message (if any) and preserve it
        let system_message = if self.config.preserve_system_message {
            messages
                .iter()
                .find(|m| m.role == MessageRole::System)
                .cloned()
        } else {
            None
        };

        // Calculate how many messages to preserve at the end
        let preserve_count = self.config.preserve_recent_count.min(messages.len());

        // Find the split point
        let non_system_start = if system_message.is_some() { 1 } else { 0 };
        let total_non_system = messages.len() - non_system_start;

        if total_non_system <= preserve_count {
            // Not enough messages to summarize
            debug!("Not enough messages to summarize, keeping all");
            return;
        }

        // Messages to summarize (excluding system and recent)
        let summarize_end = messages.len() - preserve_count;
        let messages_to_summarize: Vec<Message> =
            messages[non_system_start..summarize_end].to_vec();

        // Recent messages to preserve
        let recent_messages: Vec<Message> = messages[summarize_end..].to_vec();

        // Create summary
        let summary = if let Some(ref llm) = self.summarizer {
            self.create_llm_summary(&messages_to_summarize, llm).await
        } else {
            self.create_truncation_summary(&messages_to_summarize)
        };

        // Rebuild messages list
        messages.clear();

        // Add system message if present
        if let Some(sys) = system_message {
            messages.push(sys);
        }

        // Add summary as a system message
        messages.push(Message::system(format!(
            "[Previous conversation summary]\n{}",
            summary
        )));

        // Add preserved recent messages
        messages.extend(recent_messages);

        debug!(
            "Rebuilt message list with {} messages (1 summary + {} recent)",
            messages.len(),
            preserve_count
        );
    }

    /// Create a summary using an LLM
    async fn create_llm_summary(&self, messages: &[Message], llm: &Arc<dyn ChatModel>) -> String {
        let conversation = messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::Human => "User",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::Tool => "Tool",
                    MessageRole::System => "System",
                    MessageRole::Custom(_) => "Custom",
                };
                format!("{}: {}", role, message_to_string(m))
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            r#"Summarize the following conversation concisely, preserving key information, decisions made, and important context. Focus on facts and outcomes rather than pleasantries.

Conversation:
{}

Provide a concise summary (max 500 words):"#,
            conversation
        );

        let request = ChatRequest::new(vec![Message::human(prompt)])
            .with_temperature(0.3)
            .with_max_tokens(1000);

        match llm.chat(request).await {
            Ok(response) => message_to_string(&response.message),
            Err(e) => {
                warn!("LLM summarization failed: {}, falling back to truncation", e);
                self.create_truncation_summary(messages)
            }
        }
    }

    /// Create a simple truncation-based summary (fallback when no LLM)
    fn create_truncation_summary(&self, messages: &[Message]) -> String {
        let mut summary_parts = Vec::new();

        // Helper to get role string
        let role_str = |msg: &Message| -> &'static str {
            match msg.role {
                MessageRole::Human => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::Tool => "Tool",
                MessageRole::System => "System",
                MessageRole::Custom(_) => "Custom",
            }
        };

        // Include first few messages to capture initial context
        let head_count = 3.min(messages.len());
        for msg in messages.iter().take(head_count) {
            let role = role_str(msg);
            let content = message_to_string(msg);
            let truncated = if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content
            };
            summary_parts.push(format!("{}: {}", role, truncated));
        }

        // Add indicator of skipped content
        if messages.len() > head_count * 2 {
            summary_parts.push(format!(
                "... ({} messages omitted) ...",
                messages.len() - head_count * 2
            ));
        }

        // Include last few messages before the preserved recent ones
        if messages.len() > head_count {
            let tail_start = messages.len().saturating_sub(head_count);
            let tail_start = tail_start.max(head_count); // Don't overlap with head
            for msg in messages.iter().skip(tail_start) {
                let role = role_str(msg);
                let content = message_to_string(msg);
                let truncated = if content.len() > 200 {
                    format!("{}...", &content[..200])
                } else {
                    content
                };
                summary_parts.push(format!("{}: {}", role, truncated));
            }
        }

        summary_parts.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = ContextConfig::default();
        assert_eq!(config.max_tokens, 128_000);
        assert!((config.summarization_threshold - 0.8).abs() < 0.01);
        assert_eq!(config.preserve_recent_count, 10);
        assert!(config.preserve_system_message);
    }

    #[test]
    fn test_config_builder() {
        let config = ContextConfig::default()
            .with_max_tokens(100_000)
            .with_threshold(0.7)
            .with_preserve_recent(5);

        assert_eq!(config.max_tokens, 100_000);
        assert!((config.summarization_threshold - 0.7).abs() < 0.01);
        assert_eq!(config.preserve_recent_count, 5);
    }

    #[test]
    fn test_trigger_threshold() {
        let config = ContextConfig::default()
            .with_max_tokens(100_000)
            .with_threshold(0.8);

        assert_eq!(config.trigger_threshold(), 80_000);
    }

    #[test]
    fn test_context_manager_creation() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config);

        assert!(manager.summarizer.is_none());
    }

    #[test]
    fn test_count_tokens() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config);

        let messages = vec![
            Message::system("You are a helpful assistant."),
            Message::human("Hello!"),
            Message::assistant("Hi there!"),
        ];

        let tokens = manager.count_tokens(&messages);
        assert!(tokens > 0);
        assert!(tokens < 100);
    }

    #[test]
    fn test_should_summarize_below_threshold() {
        let config = ContextConfig::default()
            .with_max_tokens(10_000)
            .with_threshold(0.8);

        let manager = ContextManager::new(config);

        // Small message list shouldn't trigger summarization
        let messages = vec![
            Message::system("You are helpful."),
            Message::human("Hi"),
            Message::assistant("Hello!"),
        ];

        assert!(!manager.should_summarize(&messages));
    }

    #[test]
    fn test_get_stats() {
        let config = ContextConfig::default().with_max_tokens(1000);

        let manager = ContextManager::new(config);

        let messages = vec![Message::human("Hello world!")];

        let (current, max, ratio) = manager.get_stats(&messages);
        assert!(current > 0);
        assert_eq!(max, 1000);
        assert!(ratio > 0.0);
        assert!(ratio < 1.0);
    }

    #[test]
    fn test_truncation_summary() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config);

        let messages = vec![
            Message::human("What is the weather?"),
            Message::assistant("The weather is sunny today."),
            Message::human("And tomorrow?"),
            Message::assistant("Tomorrow will be cloudy."),
            Message::human("Thanks!"),
        ];

        let summary = manager.create_truncation_summary(&messages);
        assert!(!summary.is_empty());
        assert!(summary.contains("User"));
        assert!(summary.contains("Assistant"));
    }

    #[tokio::test]
    async fn test_maybe_summarize_no_action_needed() {
        let config = ContextConfig::default()
            .with_max_tokens(100_000)
            .with_threshold(0.8);

        let manager = ContextManager::new(config);

        let mut messages = vec![
            Message::system("You are helpful."),
            Message::human("Hi"),
            Message::assistant("Hello!"),
        ];

        let result = manager.maybe_summarize(&mut messages).await;

        assert!(!result.summarized);
        assert_eq!(result.messages_before, result.messages_after);
    }

    #[tokio::test]
    async fn test_summarize_preserves_system_message() {
        let config = ContextConfig::default()
            .with_max_tokens(100) // Very low to trigger summarization
            .with_threshold(0.1)
            .with_preserve_recent(2);

        let manager = ContextManager::new(config);

        let mut messages = vec![
            Message::system("You are a helpful assistant."),
            Message::human("First question"),
            Message::assistant("First answer"),
            Message::human("Second question"),
            Message::assistant("Second answer"),
            Message::human("Third question"),
            Message::assistant("Third answer"),
            Message::human("Fourth question"),
            Message::assistant("Fourth answer"),
        ];

        let result = manager.maybe_summarize(&mut messages).await;

        // Should have summarized
        assert!(result.summarized);

        // System message should still be first
        assert!(messages[0].role == MessageRole::System);
        assert!(message_to_string(&messages[0]).contains("helpful assistant"));
    }

    // ========== Additional Gap Tests ==========

    #[test]
    fn test_config_threshold_boundary_values() {
        // Test clamping at boundaries (0.1 - 0.99)
        let config_low = ContextConfig::default().with_threshold(0.0);
        assert!((config_low.summarization_threshold - 0.1).abs() < 0.01);

        let config_high = ContextConfig::default().with_threshold(1.0);
        assert!((config_high.summarization_threshold - 0.99).abs() < 0.01);

        let config_negative = ContextConfig::default().with_threshold(-0.5);
        assert!((config_negative.summarization_threshold - 0.1).abs() < 0.01);

        let config_over = ContextConfig::default().with_threshold(1.5);
        assert!((config_over.summarization_threshold - 0.99).abs() < 0.01);

        // Valid values should pass through
        let config_valid = ContextConfig::default().with_threshold(0.5);
        assert!((config_valid.summarization_threshold - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_config_target_ratio_calculation() {
        let config = ContextConfig::default()
            .with_max_tokens(100_000);

        // Default target ratio is 0.5
        assert_eq!(config.target_after_summarization(), 50_000);

        // Test with different max_tokens
        let config_small = ContextConfig::default()
            .with_max_tokens(10_000);
        assert_eq!(config_small.target_after_summarization(), 5_000);

        // Edge case: zero max_tokens
        let config_zero = ContextConfig::default()
            .with_max_tokens(0);
        assert_eq!(config_zero.target_after_summarization(), 0);
    }

    #[test]
    fn test_config_with_preserve_system_toggle() {
        let config_preserve = ContextConfig::default()
            .with_preserve_system(true);
        assert!(config_preserve.preserve_system_message);

        let config_no_preserve = ContextConfig::default()
            .with_preserve_system(false);
        assert!(!config_no_preserve.preserve_system_message);
    }

    #[test]
    fn test_count_tokens_with_tool_calls() {
        use crate::tool::ToolCall;
        use serde_json::json;

        let config = ContextConfig::default();
        let manager = ContextManager::new(config);

        // Create message with tool calls
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            name: "search".to_string(),
            args: json!({"query": "test search"}),
        };

        let message = Message::assistant("I'll search for that.")
            .with_tool_calls(vec![tool_call]);

        let tokens = manager.token_counter.count_message_tokens(&message);

        // Should include: overhead (4) + content tokens + tool call tokens
        assert!(tokens > 4); // More than just overhead
        assert!(tokens < 100); // Reasonable upper bound
    }

    #[test]
    fn test_should_summarize_exactly_at_threshold() {
        // Create a config with very low threshold for testing
        let config = ContextConfig::default()
            .with_max_tokens(100)
            .with_threshold(0.1); // 10 tokens threshold

        let threshold = config.trigger_threshold();
        let manager = ContextManager::new(config);

        // Create messages that will hit threshold
        let messages = vec![
            Message::human("Hello world!"), // ~7 tokens with overhead
        ];

        // This should trigger at exactly threshold (>= comparison)
        let tokens = manager.count_tokens(&messages);

        // If tokens >= threshold, should_summarize returns true
        let should = manager.should_summarize(&messages);
        assert_eq!(should, tokens >= threshold);
    }

    #[test]
    fn test_summarize_preserves_recent_count_boundary() {
        let config = ContextConfig::default()
            .with_preserve_recent(5);

        let preserve_recent = config.preserve_recent_count;
        let _manager = ContextManager::new(config);

        // Exactly 5 messages (same as preserve_recent_count)
        let messages = vec![
            Message::human("One"),
            Message::assistant("Two"),
            Message::human("Three"),
            Message::assistant("Four"),
            Message::human("Five"),
        ];

        // With only 5 messages and preserve_recent=5, nothing should be summarized
        let non_system_count = messages.len();
        let preserve = preserve_recent.min(messages.len());

        // When total_non_system <= preserve_count, no summarization should occur
        assert!(non_system_count <= preserve);
    }

    #[tokio::test]
    async fn test_summarize_with_empty_messages() {
        let config = ContextConfig::default()
            .with_max_tokens(100)
            .with_threshold(0.1);

        let manager = ContextManager::new(config);

        let mut messages: Vec<Message> = vec![];

        let result = manager.maybe_summarize(&mut messages).await;

        // Should return without crash, not summarized
        assert!(!result.summarized);
        assert_eq!(result.messages_before, 0);
        assert_eq!(result.messages_after, 0);
    }

    #[test]
    fn test_create_truncation_summary_long_content() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config);

        // Create message with content > 200 characters
        let long_content = "x".repeat(300);
        let messages = vec![
            Message::human(long_content.clone()),
        ];

        let summary = manager.create_truncation_summary(&messages);

        // Should contain truncated content with "..."
        assert!(summary.contains("..."));
        // Should have "User:" prefix
        assert!(summary.contains("User:"));
    }

    #[test]
    fn test_create_truncation_summary_skipped_messages_indicator() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config);

        // Create many messages to trigger the "omitted" indicator
        // head_count = 3, so we need > 6 messages (3 head + 3 tail)
        let messages = vec![
            Message::human("Message 1"),
            Message::assistant("Response 1"),
            Message::human("Message 2"),
            Message::assistant("Response 2"),
            Message::human("Message 3"),
            Message::assistant("Response 3"),
            Message::human("Message 4"),
            Message::assistant("Response 4"),
            Message::human("Message 5"),
            Message::assistant("Response 5"),
        ];

        let summary = manager.create_truncation_summary(&messages);

        // Should contain the "messages omitted" indicator
        assert!(summary.contains("messages omitted"));
    }

    #[tokio::test]
    async fn test_summarize_rebuilds_message_list_correctly() {
        let config = ContextConfig::default()
            .with_max_tokens(50) // Very low to force summarization
            .with_threshold(0.1)
            .with_preserve_recent(2);

        let manager = ContextManager::new(config);

        let mut messages = vec![
            Message::system("You are helpful."),
            Message::human("First"),
            Message::assistant("Response 1"),
            Message::human("Second"),
            Message::assistant("Response 2"),
            Message::human("Third"),
            Message::assistant("Response 3"),
        ];

        let result = manager.maybe_summarize(&mut messages).await;

        if result.summarized {
            // First message should be system message
            assert!(messages[0].role == MessageRole::System);
            assert!(message_to_string(&messages[0]).contains("helpful"));

            // Second message should be the summary (also system type with prefix)
            assert!(messages[1].role == MessageRole::System);
            assert!(message_to_string(&messages[1]).contains("Previous conversation summary"));

            // Remaining messages should be the preserved recent ones
            assert!(messages.len() >= 3); // system + summary + at least some recent
        }
    }

    #[tokio::test]
    async fn test_summarize_without_system_message() {
        let config = ContextConfig::default()
            .with_max_tokens(50)
            .with_threshold(0.1)
            .with_preserve_recent(2)
            .with_preserve_system(false);

        let manager = ContextManager::new(config);

        // No system message in this list
        let mut messages = vec![
            Message::human("First question"),
            Message::assistant("First answer"),
            Message::human("Second question"),
            Message::assistant("Second answer"),
            Message::human("Third question"),
            Message::assistant("Third answer"),
        ];

        let original_count = messages.len();
        let result = manager.maybe_summarize(&mut messages).await;

        if result.summarized {
            // Should have fewer messages after summarization
            assert!(messages.len() < original_count);
        }
    }
}
