//! Context window trimming strategies
//!
//! Provides intelligent message truncation to fit within context limits.

use crate::context::token_counter::TokenCounter;
use langgraph_core::messages::Message;

/// Message priority for retention during trimming
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// Lowest priority - trim first
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority - trim last
    High = 2,
    /// System messages - never trim
    System = 3,
}

/// Trim strategy for context management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimStrategy {
    /// Keep most recent messages
    Recent,
    /// Keep based on priority
    Priority,
    /// Sliding window with overlap
    SlidingWindow { overlap: usize },
}

/// Context trimmer for managing message history
pub struct ContextTrimmer {
    /// Token counter
    counter: TokenCounter,
    /// Trim strategy
    strategy: TrimStrategy,
    /// Maximum tokens to keep
    max_tokens: usize,
}

impl ContextTrimmer {
    /// Create a new context trimmer
    pub fn new(model: impl Into<String>, max_tokens: usize) -> Self {
        Self {
            counter: TokenCounter::new(model),
            strategy: TrimStrategy::Priority,
            max_tokens,
        }
    }

    /// Set trim strategy
    pub fn with_strategy(mut self, strategy: TrimStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Trim messages to fit within token limit
    pub fn trim_messages(&self, messages: &[Message]) -> Vec<Message> {
        let total_tokens = self.counter.count_messages(messages);
        
        if total_tokens.tokens <= self.max_tokens {
            // No trimming needed
            return messages.to_vec();
        }

        match self.strategy {
            TrimStrategy::Recent => self.trim_recent(messages),
            TrimStrategy::Priority => self.trim_by_priority(messages),
            TrimStrategy::SlidingWindow { overlap } => self.trim_sliding_window(messages, overlap),
        }
    }

    /// Keep most recent messages that fit
    fn trim_recent(&self, messages: &[Message]) -> Vec<Message> {
        let mut result = Vec::new();
        let mut current_tokens = 0;

        // Iterate from most recent
        for message in messages.iter().rev() {
            let msg_tokens = self.counter.count_message(message).tokens;
            
            if current_tokens + msg_tokens <= self.max_tokens {
                result.insert(0, message.clone());
                current_tokens += msg_tokens;
            } else {
                break;
            }
        }

        result
    }

    /// Keep messages based on priority
    fn trim_by_priority(&self, messages: &[Message]) -> Vec<Message> {
        // Annotate messages with priorities
        let mut prioritized: Vec<(Message, MessagePriority, usize)> = messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| {
                let priority = self.get_message_priority(msg, idx, messages.len());
                (msg.clone(), priority, idx)
            })
            .collect();

        // Sort by priority (high to low), then by index (preserve order within priority)
        prioritized.sort_by(|a, b| {
            b.1.cmp(&a.1).then(a.2.cmp(&b.2))
        });

        // Take messages until we hit token limit
        let mut result = Vec::new();
        let mut current_tokens = 0;

        for (message, _, _) in prioritized {
            let msg_tokens = self.counter.count_message(&message).tokens;
            
            if current_tokens + msg_tokens <= self.max_tokens {
                result.push(message);
                current_tokens += msg_tokens;
            }
        }

        // Re-sort by original index to preserve conversation order
        // Use message text comparison for matching
        let mut final_result = Vec::new();
        for msg in messages {
            let msg_text = msg.text().unwrap_or("");
            if result.iter().any(|m| m.text().unwrap_or("") == msg_text) {
                final_result.push(msg.clone());
                // Remove from result to avoid duplicates
                if let Some(pos) = result.iter().position(|m| m.text().unwrap_or("") == msg_text) {
                    result.remove(pos);
                }
            }
        }

        final_result
    }

    /// Sliding window with overlap
    fn trim_sliding_window(&self, messages: &[Message], overlap: usize) -> Vec<Message> {
        if messages.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut current_tokens = 0;

        // Always keep the most recent messages
        for message in messages.iter().rev() {
            let msg_tokens = self.counter.count_message(message).tokens;
            
            if current_tokens + msg_tokens <= self.max_tokens {
                result.insert(0, message.clone());
                current_tokens += msg_tokens;
            } else {
                break;
            }
        }

        // If we have room, add some overlap from earlier
        if result.len() < messages.len() && overlap > 0 {
            let start_idx = messages.len().saturating_sub(result.len() + overlap);
            let end_idx = messages.len() - result.len();
            
            for i in start_idx..end_idx {
                let msg_tokens = self.counter.count_message(&messages[i]).tokens;
                if current_tokens + msg_tokens <= self.max_tokens {
                    result.insert(0, messages[i].clone());
                    current_tokens += msg_tokens;
                } else {
                    break;
                }
            }
        }

        result
    }

    /// Get priority for a message
    fn get_message_priority(&self, message: &Message, index: usize, total: usize) -> MessagePriority {
        // System messages have highest priority
        if message.role == langgraph_core::messages::MessageRole::System {
            return MessagePriority::System;
        }

        // Recent messages (last 20%) are high priority
        if index >= total * 4 / 5 {
            return MessagePriority::High;
        }

        // Tool-related messages are normal priority
        if let Some(text) = message.text() {
            if text.contains("tool") || text.contains("function") {
                return MessagePriority::Normal;
            }
        }

        // Older messages are low priority
        MessagePriority::Low
    }

    /// Calculate tokens saved by trimming
    pub fn tokens_saved(&self, original: &[Message], trimmed: &[Message]) -> usize {
        let original_count = self.counter.count_messages(original);
        let trimmed_count = self.counter.count_messages(trimmed);
        original_count.tokens.saturating_sub(trimmed_count.tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use langgraph_core::messages::MessageRole;

    fn create_messages(count: usize) -> Vec<Message> {
        (0..count)
            .map(|i| Message::human(format!("Message {}", i)))
            .collect()
    }

    #[test]
    fn test_no_trimming_needed() {
        let trimmer = ContextTrimmer::new("gpt-4", 10000);
        let messages = create_messages(5);
        let trimmed = trimmer.trim_messages(&messages);
        
        assert_eq!(trimmed.len(), messages.len());
    }

    #[test]
    fn test_trim_recent() {
        let trimmer = ContextTrimmer::new("gpt-4", 50)
            .with_strategy(TrimStrategy::Recent);
        let messages = create_messages(10);
        let trimmed = trimmer.trim_messages(&messages);
        
        // Should keep most recent messages
        assert!(trimmed.len() < messages.len());
        assert!(trimmed.len() > 0);
    }

    #[test]
    fn test_trim_by_priority() {
        let trimmer = ContextTrimmer::new("gpt-4", 100)
            .with_strategy(TrimStrategy::Priority);
        
        let mut messages = vec![
            Message::system("System message"),
            Message::human("Old message"),
            Message::human("Recent message"),
        ];
        
        let trimmed = trimmer.trim_messages(&messages);
        
        // System message should always be kept
        assert!(trimmed.iter().any(|m| m.text().map_or(false, |text| text.contains("System"))));
    }

    #[test]
    fn test_sliding_window() {
        let trimmer = ContextTrimmer::new("gpt-4", 50)
            .with_strategy(TrimStrategy::SlidingWindow { overlap: 2 });
        let messages = create_messages(10);
        let trimmed = trimmer.trim_messages(&messages);
        
        assert!(trimmed.len() < messages.len());
        assert!(trimmed.len() > 0);
    }

    #[test]
    fn test_tokens_saved() {
        let trimmer = ContextTrimmer::new("gpt-4", 50);
        let messages = create_messages(10);
        let trimmed = trimmer.trim_messages(&messages);
        let saved = trimmer.tokens_saved(&messages, &trimmed);
        
        assert!(saved > 0);
    }
}








