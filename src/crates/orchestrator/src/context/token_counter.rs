//! Token counting for different LLM models
//!
//! Provides token counting functionality to estimate context window usage.

use langgraph_core::messages::Message;
use serde_json::Value;

/// Token count result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenCount {
    /// Number of tokens
    pub tokens: usize,
    /// Estimated characters
    pub chars: usize,
}

impl TokenCount {
    /// Create a new token count
    pub fn new(tokens: usize, chars: usize) -> Self {
        Self { tokens, chars }
    }

    /// Add another token count
    pub fn add(&mut self, other: TokenCount) {
        self.tokens += other.tokens;
        self.chars += other.chars;
    }
}

/// Token counting method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountingMethod {
    /// Character-based approximation (fast, less accurate)
    Approximation,
    /// Byte-pair encoding simulation (more accurate)
    BpeSimulation,
}

/// Token counter for different LLM models
pub struct TokenCounter {
    /// Model name for token counting
    model: String,
    /// Average tokens per character (model-specific)
    tokens_per_char: f32,
    /// Counting method
    method: CountingMethod,
}

impl TokenCounter {
    /// Create a new token counter for a specific model
    pub fn new(model: impl Into<String>) -> Self {
        let model = model.into();
        let tokens_per_char = Self::get_tokens_per_char(&model);

        Self {
            model,
            tokens_per_char,
            method: CountingMethod::Approximation,
        }
    }

    /// Create with specific counting method
    pub fn with_method(mut self, method: CountingMethod) -> Self {
        self.method = method;
        self
    }

    /// Get approximate tokens per character for a model
    fn get_tokens_per_char(model: &str) -> f32 {
        // Approximate values based on common models
        // GPT-4, GPT-3.5: ~0.25 tokens/char (4 chars/token)
        // Claude: ~0.27 tokens/char
        // Llama: ~0.23 tokens/char
        if model.contains("gpt") || model.contains("openai") {
            0.25
        } else if model.contains("claude") || model.contains("anthropic") {
            0.27
        } else if model.contains("llama") {
            0.23
        } else {
            // Default approximation
            0.25
        }
    }

    /// Count tokens in a text string
    pub fn count_text(&self, text: &str) -> TokenCount {
        let chars = text.len();
        let tokens = match self.method {
            CountingMethod::Approximation => {
                (chars as f32 * self.tokens_per_char).ceil() as usize
            }
            CountingMethod::BpeSimulation => {
                // Simulate BPE: count words, punctuation, and adjust
                let words = text.split_whitespace().count();
                let punct = text.chars().filter(|c| c.is_ascii_punctuation()).count();
                let base_tokens = words + (punct / 2);

                // Apply model-specific multiplier
                (base_tokens as f32 * 1.3).ceil() as usize
            }
        };
        TokenCount::new(tokens, chars)
    }

    /// Count tokens in a message
    pub fn count_message(&self, message: &Message) -> TokenCount {
        let mut total = TokenCount::new(0, 0);
        
        // Count role (typically 1 token)
        total.add(TokenCount::new(1, 0));
        
        // Count content (get text representation)
        let text = message.text().unwrap_or("");
        total.add(self.count_text(text));
        
        // Add overhead for message formatting (~3 tokens)
        total.add(TokenCount::new(3, 0));
        
        total
    }

    /// Count tokens in multiple messages
    pub fn count_messages(&self, messages: &[Message]) -> TokenCount {
        let mut total = TokenCount::new(0, 0);
        for message in messages {
            total.add(self.count_message(message));
        }
        total
    }

    /// Count tokens in a tool response
    pub fn count_tool_response(&self, response: &Value) -> TokenCount {
        // Serialize to JSON and count
        let json_str = serde_json::to_string(response).unwrap_or_default();
        let mut count = self.count_text(&json_str);
        
        // Add overhead for tool response formatting (~5 tokens)
        count.add(TokenCount::new(5, 0));
        
        count
    }

    /// Estimate tokens for a system prompt
    pub fn count_system_prompt(&self, prompt: &str) -> TokenCount {
        let mut count = self.count_text(prompt);
        // Add overhead for system message formatting (~4 tokens)
        count.add(TokenCount::new(4, 0));
        count
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new("gpt-4")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use langgraph_core::messages::MessageRole;

    #[test]
    fn test_count_text() {
        let counter = TokenCounter::new("gpt-4");
        let count = counter.count_text("Hello, world!");
        
        // "Hello, world!" is 13 chars, ~3-4 tokens
        assert!(count.tokens >= 3 && count.tokens <= 4);
        assert_eq!(count.chars, 13);
    }

    #[test]
    fn test_count_message() {
        let counter = TokenCounter::new("gpt-4");
        let message = Message::human("Hello, world!");
        let count = counter.count_message(&message);
        
        // Should include role (1) + content (~3) + formatting (3) = ~7 tokens
        assert!(count.tokens >= 6 && count.tokens <= 10);
    }

    #[test]
    fn test_count_messages() {
        let counter = TokenCounter::new("gpt-4");
        let messages = vec![
            Message::human("Hello!"),
            Message::ai("Hi there!"),
        ];
        let count = counter.count_messages(&messages);
        
        // Should be sum of both messages
        assert!(count.tokens > 0);
    }

    #[test]
    fn test_different_models() {
        let gpt_counter = TokenCounter::new("gpt-4");
        let claude_counter = TokenCounter::new("claude-3");
        
        let text = "Test message";
        let gpt_count = gpt_counter.count_text(text);
        let claude_count = claude_counter.count_text(text);
        
        // Claude typically has slightly more tokens per char
        assert!(claude_count.tokens >= gpt_count.tokens ||
                gpt_count.tokens.abs_diff(claude_count.tokens) <= 1);
    }
}
