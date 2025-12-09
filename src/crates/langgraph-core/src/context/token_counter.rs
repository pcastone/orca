//! Token Counter implementations
//!
//! Provides accurate token counting for context management.

use crate::messages::{ContentPart, Message, MessageContent};
use tiktoken_rs::{cl100k_base, CoreBPE};
use tracing::debug;

/// Helper function to convert MessageContent to a string for token counting
fn content_to_string(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(s) => s.clone(),
        MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text, .. } => Some(text.clone()),
                ContentPart::Image { .. } => Some("[image]".to_string()),
                ContentPart::Custom { data } => Some(format!("[custom:{}]", data))
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Helper function to convert a Message to a string for token counting
pub fn message_to_string(message: &Message) -> String {
    content_to_string(&message.content)
}

/// Trait for counting tokens in text and messages
pub trait TokenCounter: Send + Sync {
    /// Count tokens in a string
    fn count_tokens(&self, text: &str) -> usize;

    /// Count tokens in a message (includes role overhead)
    fn count_message_tokens(&self, message: &Message) -> usize {
        // Each message has overhead for role and formatting
        // Approximate: 4 tokens for message structure
        let overhead = 4;
        let content_tokens = self.count_tokens(&content_to_string(&message.content));
        overhead + content_tokens
    }

    /// Count total tokens in a list of messages
    fn count_messages_tokens(&self, messages: &[Message]) -> usize {
        messages.iter().map(|m| self.count_message_tokens(m)).sum()
    }
}

/// Token counter using tiktoken (cl100k_base encoding, used by GPT-4/Claude)
pub struct TiktokenCounter {
    bpe: CoreBPE,
}

impl TiktokenCounter {
    /// Create a new TiktokenCounter with cl100k_base encoding
    ///
    /// This encoding is used by:
    /// - GPT-4 and GPT-4 Turbo
    /// - GPT-3.5 Turbo
    /// - Claude models (approximate)
    pub fn new() -> Self {
        let bpe = cl100k_base().expect("Failed to load cl100k_base tokenizer");
        Self { bpe }
    }
}

impl Default for TiktokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter for TiktokenCounter {
    fn count_tokens(&self, text: &str) -> usize {
        let tokens = self.bpe.encode_with_special_tokens(text);
        tokens.len()
    }

    fn count_message_tokens(&self, message: &Message) -> usize {
        // GPT-4 message format overhead:
        // - 3 tokens for <|im_start|>role<|im_sep|>
        // - 1 token for <|im_end|>
        let overhead = 4;

        // Count content tokens
        let content = content_to_string(&message.content);
        let content_tokens = self.count_tokens(&content);

        // If message has tool calls, add those tokens too
        let tool_tokens = if let Some(ref tool_calls) = message.tool_calls {
            tool_calls
                .iter()
                .map(|tc| {
                    let name_tokens = self.count_tokens(&tc.name);
                    let args_tokens = self.count_tokens(&tc.args.to_string());
                    name_tokens + args_tokens + 4 // overhead for tool call structure
                })
                .sum()
        } else {
            0
        };

        let total = overhead + content_tokens + tool_tokens;
        debug!(
            "Message tokens: {} (content: {}, tool: {}, overhead: {})",
            total, content_tokens, tool_tokens, overhead
        );
        total
    }
}

/// Simple token counter that estimates based on character count
///
/// Uses the rough approximation of 4 characters per token.
/// Less accurate but doesn't require loading tokenizer.
pub struct SimpleTokenCounter {
    chars_per_token: f32,
}

impl SimpleTokenCounter {
    /// Create a new SimpleTokenCounter with default ratio (4 chars/token)
    pub fn new() -> Self {
        Self {
            chars_per_token: 4.0,
        }
    }

    /// Create with custom characters per token ratio
    pub fn with_ratio(chars_per_token: f32) -> Self {
        Self { chars_per_token }
    }
}

impl Default for SimpleTokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter for SimpleTokenCounter {
    fn count_tokens(&self, text: &str) -> usize {
        (text.len() as f32 / self.chars_per_token).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tiktoken_counter_creation() {
        let counter = TiktokenCounter::new();
        // Basic sanity check
        let tokens = counter.count_tokens("Hello, world!");
        assert!(tokens > 0);
        assert!(tokens < 10); // Should be about 4 tokens
    }

    #[test]
    fn test_tiktoken_count_simple_text() {
        let counter = TiktokenCounter::new();

        // "Hello" should be 1 token
        let hello_tokens = counter.count_tokens("Hello");
        assert_eq!(hello_tokens, 1);

        // Longer text should have more tokens
        let long_text = "The quick brown fox jumps over the lazy dog.";
        let long_tokens = counter.count_tokens(long_text);
        assert!(long_tokens > 5);
        assert!(long_tokens < 15);
    }

    #[test]
    fn test_tiktoken_count_message() {
        let counter = TiktokenCounter::new();

        let message = Message::human("Hello, how are you?");
        let tokens = counter.count_message_tokens(&message);

        // Should include overhead + content
        assert!(tokens > 4); // At least overhead
        assert!(tokens < 20); // Not too many
    }

    #[test]
    fn test_tiktoken_count_messages() {
        let counter = TiktokenCounter::new();

        let messages = vec![
            Message::system("You are a helpful assistant."),
            Message::human("Hello!"),
            Message::assistant("Hi there! How can I help you today?"),
        ];

        let total_tokens = counter.count_messages_tokens(&messages);
        assert!(total_tokens > 20);
        assert!(total_tokens < 100);
    }

    #[test]
    fn test_simple_counter() {
        let counter = SimpleTokenCounter::new();

        // 12 chars / 4 = 3 tokens
        let tokens = counter.count_tokens("Hello World!");
        assert_eq!(tokens, 3);
    }

    #[test]
    fn test_simple_counter_custom_ratio() {
        let counter = SimpleTokenCounter::with_ratio(3.0);

        // 12 chars / 3 = 4 tokens
        let tokens = counter.count_tokens("Hello World!");
        assert_eq!(tokens, 4);
    }

    #[test]
    fn test_empty_text() {
        let tiktoken = TiktokenCounter::new();
        let simple = SimpleTokenCounter::new();

        assert_eq!(tiktoken.count_tokens(""), 0);
        assert_eq!(simple.count_tokens(""), 0);
    }

    // ========== Additional Gap Tests ==========

    #[test]
    fn test_tiktoken_count_unicode_text() {
        let counter = TiktokenCounter::new();

        // Test emoji handling
        let emoji_text = "Hello 👋 World 🌍";
        let emoji_tokens = counter.count_tokens(emoji_text);
        assert!(emoji_tokens > 0);

        // Test CJK characters
        let cjk_text = "你好世界";
        let cjk_tokens = counter.count_tokens(cjk_text);
        assert!(cjk_tokens > 0);

        // Test mixed ASCII/Unicode
        let mixed_text = "Hello 你好 World 世界 👋";
        let mixed_tokens = counter.count_tokens(mixed_text);
        assert!(mixed_tokens > 0);

        // CJK typically uses more tokens than equivalent ASCII
        let ascii_hello = "Hello World";
        let cjk_hello = "你好世界";
        let ascii_count = counter.count_tokens(ascii_hello);
        let cjk_count = counter.count_tokens(cjk_hello);
        // Both should produce tokens, though counts may differ
        assert!(ascii_count > 0);
        assert!(cjk_count > 0);
    }

    #[test]
    fn test_tiktoken_count_message_with_all_content_types() {
        let counter = TiktokenCounter::new();

        // Test with text parts using helper methods
        let parts = vec![
            ContentPart::text("First part"),
            ContentPart::text("Second part"),
        ];

        let message = Message::human(MessageContent::Parts(parts));
        let tokens = counter.count_message_tokens(&message);

        // Should count both text parts
        assert!(tokens > 4); // More than just overhead
    }

    #[test]
    fn test_tiktoken_count_message_with_image_placeholder() {
        let counter = TiktokenCounter::new();

        // Test message with image content (should produce "[image]" placeholder)
        let parts = vec![
            ContentPart::text("Look at this:"),
            ContentPart::image_data("image/png", "fake_base64_data"),
        ];

        let message = Message::human(MessageContent::Parts(parts));
        let tokens = counter.count_message_tokens(&message);

        // Should count text + "[image]" placeholder
        assert!(tokens > 4);
    }

    #[test]
    fn test_tiktoken_count_message_with_custom_content() {
        let counter = TiktokenCounter::new();

        // Test message with custom content type
        let parts = vec![
            ContentPart::text("Custom content:"),
            ContentPart::Custom {
                data: serde_json::json!({"type": "audio", "data": "..."}),
            },
        ];

        let message = Message::human(MessageContent::Parts(parts));
        let tokens = counter.count_message_tokens(&message);

        // Should count text + custom placeholder
        assert!(tokens > 4);
    }

    #[test]
    fn test_content_to_string_helper() {
        // Test the helper function directly via message_to_string

        // Simple text content
        let simple_msg = Message::human("Simple text");
        let simple_str = message_to_string(&simple_msg);
        assert_eq!(simple_str, "Simple text");

        // Multi-part content
        let parts = vec![
            ContentPart::text("Part 1"),
            ContentPart::text("Part 2"),
        ];
        let multipart_msg = Message::human(MessageContent::Parts(parts));
        let multipart_str = message_to_string(&multipart_msg);
        assert!(multipart_str.contains("Part 1"));
        assert!(multipart_str.contains("Part 2"));
    }

    #[test]
    fn test_simple_counter_boundary_cases() {
        let counter = SimpleTokenCounter::new();

        // Single character
        let single_char = counter.count_tokens("a");
        assert_eq!(single_char, 1); // ceil(1/4) = 1

        // Exactly 4 characters (one token)
        let four_chars = counter.count_tokens("abcd");
        assert_eq!(four_chars, 1);

        // 5 characters should round up to 2 tokens
        let five_chars = counter.count_tokens("abcde");
        assert_eq!(five_chars, 2); // ceil(5/4) = 2
    }

    #[test]
    fn test_tiktoken_whitespace_handling() {
        let counter = TiktokenCounter::new();

        // Whitespace only
        let spaces = counter.count_tokens("   ");
        assert!(spaces >= 1);

        // Newlines
        let newlines = counter.count_tokens("\n\n\n");
        assert!(newlines >= 1);

        // Tabs and mixed whitespace
        let mixed = counter.count_tokens("\t\n \t\n");
        assert!(mixed >= 1);
    }

    #[test]
    fn test_tiktoken_special_characters() {
        let counter = TiktokenCounter::new();

        // Code-like content
        let code = "fn main() { println!(\"Hello\"); }";
        let code_tokens = counter.count_tokens(code);
        assert!(code_tokens > 5);
        assert!(code_tokens < 20);

        // JSON
        let json = r#"{"key": "value", "number": 123}"#;
        let json_tokens = counter.count_tokens(json);
        assert!(json_tokens > 5);
    }
}
