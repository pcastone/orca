//! Integration tests for Context Manager
//!
//! These tests verify the complete context management functionality including
//! token counting, trimming, and priority-based retention.

use langgraph_core::messages::{Message, MessageRole};
use orchestrator::context::{
    create_counter, ClaudeCounter, ContextLimit, ContextManager, ContextStats, ContextStrategy,
    ContextTrimmer, TiktokenCounter,
};
use serde_json::json;

// ===== Token Counting Tests =====

#[test]
fn test_tiktoken_basic_counting() {
    let counter = TiktokenCounter::default();

    let msg = Message::new(MessageRole::Human, "Hello, world!");
    let tokens = counter.count_message_tokens(&msg);

    // Should be between 4-10 tokens (4 overhead + ~3-6 for content)
    assert!(tokens >= 4 && tokens <= 10, "Expected 4-10 tokens, got {}", tokens);
}

#[test]
fn test_tiktoken_counting_accuracy() {
    let counter = TiktokenCounter::default();

    // Test known text with predictable token count
    let short_text = "Hello";
    let msg_short = Message::new(MessageRole::Human, short_text);
    let tokens_short = counter.count_message_tokens(&msg_short);

    let long_text = "This is a much longer message that should definitely contain more tokens than the short message.";
    let msg_long = Message::new(MessageRole::Human, long_text);
    let tokens_long = counter.count_message_tokens(&msg_long);

    // Long message should have significantly more tokens
    assert!(tokens_long > tokens_short * 2,
        "Long message ({} tokens) should have >2x tokens of short message ({} tokens)",
        tokens_long, tokens_short);
}

#[test]
fn test_claude_estimation_accuracy() {
    let counter = ClaudeCounter::new("claude-3");

    let text_100_chars = "a".repeat(100);
    let msg = Message::new(MessageRole::Human, text_100_chars);
    let tokens = counter.count_message_tokens(&msg);

    // 100 characters should be roughly 25-30 tokens (4 chars/token + overhead)
    assert!(tokens >= 20 && tokens <= 40,
        "Expected 20-40 tokens for 100 chars, got {}", tokens);
}

#[test]
fn test_token_counting_with_tool_calls() {
    let counter = TiktokenCounter::default();

    let mut msg = Message::new(MessageRole::Assistant, "Let me search for that information.");
    msg.tool_calls = Some(vec![
        langgraph_core::ToolCall {
            id: "call_abc123".to_string(),
            name: "web_search".to_string(),
            args: json!({"query": "context window management", "max_results": 5}),
        }
    ]);

    let tokens = counter.count_message_tokens(&msg);

    // Should include content + tool call name + args
    assert!(tokens > 20, "Tool call message should have >20 tokens, got {}", tokens);
}

#[test]
fn test_multiple_messages_counting() {
    let counter = TiktokenCounter::default();

    let messages = vec![
        Message::new(MessageRole::System, "You are a helpful assistant."),
        Message::new(MessageRole::Human, "What is the weather today?"),
        Message::new(MessageRole::Assistant, "I need to check the weather for you."),
    ];

    let total_tokens = counter.count_tokens(&messages);

    // Each message has overhead + content, should be reasonable
    assert!(total_tokens > 30 && total_tokens < 100,
        "Expected 30-100 tokens for 3 messages, got {}", total_tokens);
}

// ===== Context Limit Tests =====

#[test]
fn test_context_limit_detection_gpt() {
    assert_eq!(ContextLimit::from_model_name("gpt-3.5-turbo"), ContextLimit::Small);
    assert_eq!(ContextLimit::from_model_name("gpt-4"), ContextLimit::Medium);
    assert_eq!(ContextLimit::from_model_name("gpt-4-32k"), ContextLimit::Large);
    assert_eq!(ContextLimit::from_model_name("gpt-4-turbo-128k"), ContextLimit::Extreme);
}

#[test]
fn test_context_limit_detection_claude() {
    assert_eq!(ContextLimit::from_model_name("claude-2"), ContextLimit::VeryLarge);
    assert_eq!(ContextLimit::from_model_name("claude-3-opus"), ContextLimit::Extreme);
    assert_eq!(ContextLimit::from_model_name("claude-3-sonnet"), ContextLimit::Extreme);
}

#[test]
fn test_context_limit_unknown_defaults_small() {
    assert_eq!(ContextLimit::from_model_name("unknown-model-v1"), ContextLimit::Small);
}

#[test]
fn test_context_strategy_for_model() {
    let strategy = ContextStrategy::for_model("gpt-4");
    assert_eq!(strategy.max_tokens, 7_500);

    let strategy_claude = ContextStrategy::for_model("claude-3-opus");
    assert_eq!(strategy_claude.max_tokens, 190_000);
}

// ===== Trimming Tests =====

#[test]
fn test_trim_empty_context() {
    let counter = TiktokenCounter::default();
    let trimmer = ContextTrimmer::new(counter);
    let strategy = ContextStrategy::default();

    let messages: Vec<Message> = vec![];
    let trimmed = trimmer.trim(messages, &strategy);

    assert_eq!(trimmed.len(), 0);
}

#[test]
fn test_trim_single_message() {
    let counter = TiktokenCounter::default();
    let trimmer = ContextTrimmer::new(counter);
    let strategy = ContextStrategy::default();

    let messages = vec![Message::new(MessageRole::Human, "Hello")];
    let trimmed = trimmer.trim(messages.clone(), &strategy);

    assert_eq!(trimmed.len(), 1);
}

#[test]
fn test_trim_within_limit_no_changes() {
    let counter = TiktokenCounter::default();
    let trimmer = ContextTrimmer::new(counter);
    let strategy = ContextStrategy::default();

    let messages = vec![
        Message::new(MessageRole::System, "You are helpful"),
        Message::new(MessageRole::Human, "Hello"),
        Message::new(MessageRole::Assistant, "Hi!"),
    ];

    let original_len = messages.len();
    let trimmed = trimmer.trim(messages, &strategy);

    // Should preserve all messages when within limit
    assert_eq!(trimmed.len(), original_len);
}

#[test]
fn test_trim_exceeds_limit_reduces_count() {
    let counter = TiktokenCounter::default();
    let trimmer = ContextTrimmer::new(counter);
    let mut strategy = ContextStrategy::default();
    strategy.max_tokens = 150; // Small limit to force trimming

    let mut messages = vec![];
    for i in 0..20 {
        messages.push(Message::new(MessageRole::Human, format!("Message number {}", i)));
    }

    let trimmed = trimmer.trim(messages.clone(), &strategy);

    // Should have trimmed some messages
    assert!(trimmed.len() < messages.len(),
        "Expected trimming, got {} messages from {} original",
        trimmed.len(), messages.len());

    // Verify we're within limit
    let total_tokens = trimmer.counter().count_tokens(&trimmed);
    assert!(total_tokens <= strategy.max_tokens,
        "Trimmed messages ({} tokens) exceed limit ({} tokens)",
        total_tokens, strategy.max_tokens);
}

#[test]
fn test_trim_preserves_system_messages() {
    let counter = TiktokenCounter::default();
    let trimmer = ContextTrimmer::new(counter);
    let mut strategy = ContextStrategy::default();
    strategy.max_tokens = 100; // Very small limit
    strategy.preserve_system = true;

    let messages = vec![
        Message::new(MessageRole::System, "Important system instructions that must be preserved"),
        Message::new(MessageRole::Human, "Question 1"),
        Message::new(MessageRole::Assistant, "Answer 1"),
        Message::new(MessageRole::Human, "Question 2"),
        Message::new(MessageRole::Assistant, "Answer 2"),
        Message::new(MessageRole::Human, "Question 3"),
        Message::new(MessageRole::Assistant, "Answer 3"),
    ];

    let trimmed = trimmer.trim(messages, &strategy);

    // System message must be present
    assert!(trimmed.iter().any(|m| m.role == MessageRole::System),
        "System message must be preserved");
}

#[test]
fn test_trim_preserves_recent_messages() {
    let counter = TiktokenCounter::default();
    let trimmer = ContextTrimmer::new(counter);
    let mut strategy = ContextStrategy::default();
    strategy.max_tokens = 200;
    strategy.keep_recent = 10;

    let messages = vec![
        Message::new(MessageRole::Human, "Old question 1"),
        Message::new(MessageRole::Assistant, "Old answer 1"),
        Message::new(MessageRole::Human, "Old question 2"),
        Message::new(MessageRole::Assistant, "Old answer 2"),
        Message::new(MessageRole::Human, "Recent question"),
        Message::new(MessageRole::Assistant, "Recent answer - this should be kept"),
    ];

    let trimmed = trimmer.trim(messages, &strategy);

    // Recent messages should be present
    let has_recent = trimmed.iter().any(|m| {
        match &m.content {
            langgraph_core::messages::MessageContent::Text(s) => s.contains("Recent answer"),
            _ => false,
        }
    });

    assert!(has_recent, "Recent messages should be preserved");
}

#[test]
fn test_trim_preserves_tool_pairs() {
    let counter = TiktokenCounter::default();
    let trimmer = ContextTrimmer::new(counter);
    let mut strategy = ContextStrategy::default();
    strategy.max_tokens = 300;
    strategy.preserve_tools = true;

    let mut assistant_with_tool = Message::new(MessageRole::Assistant, "Searching...");
    assistant_with_tool.tool_calls = Some(vec![
        langgraph_core::ToolCall {
            id: "call_1".to_string(),
            name: "search".to_string(),
            args: json!({"query": "test"}),
        }
    ]);

    let tool_response = Message::new(MessageRole::Tool, "Search results: Found 5 items");

    let messages = vec![
        Message::new(MessageRole::Human, "Old message"),
        assistant_with_tool.clone(),
        tool_response.clone(),
        Message::new(MessageRole::Human, "Recent message"),
    ];

    let trimmed = trimmer.trim(messages, &strategy);

    // If we have the tool call, we should have the response
    let has_tool_call = trimmed.iter().any(|m| m.tool_calls.is_some());
    let has_tool_response = trimmed.iter().any(|m| m.role == MessageRole::Tool);

    if has_tool_call {
        assert!(has_tool_response, "Tool response should be preserved with tool call");
    }
}

#[test]
fn test_trim_all_messages_high_priority() {
    let counter = TiktokenCounter::default();
    let trimmer = ContextTrimmer::new(counter);
    let mut strategy = ContextStrategy::default();
    strategy.max_tokens = 500;

    // Create only recent messages (all high priority)
    let mut messages = vec![];
    for i in 0..5 {
        messages.push(Message::new(MessageRole::Human, format!("Recent message {}", i)));
    }

    let original_count = messages.len();
    let trimmed = trimmer.trim(messages, &strategy);

    // All should fit within 500 tokens
    assert_eq!(trimmed.len(), original_count,
        "All high-priority messages should be preserved within limit");
}

// ===== Context Stats Tests =====

#[test]
fn test_context_stats_calculation() {
    let counter = TiktokenCounter::default();
    let messages = vec![
        Message::new(MessageRole::System, "System message"),
        Message::new(MessageRole::Human, "User message"),
        Message::new(MessageRole::Assistant, "Assistant response"),
    ];

    let stats = ContextStats::from_messages(&messages, 8_000, &counter);

    assert_eq!(stats.message_count, 3);
    assert!(stats.total_tokens > 0);
    assert!(stats.utilization > 0.0);
    assert!(!stats.over_limit);
}

#[test]
fn test_context_stats_over_limit() {
    let counter = TiktokenCounter::default();
    let messages = vec![
        Message::new(MessageRole::Human, "Test message"),
    ];

    let stats = ContextStats::from_messages(&messages, 5, &counter); // Unrealistically small limit

    assert!(stats.over_limit, "Should detect over-limit condition");
    assert!(stats.utilization > 100.0, "Utilization should exceed 100%");
}

// ===== Factory Function Tests =====

#[test]
fn test_create_counter_for_gpt() {
    let counter = create_counter("gpt-4").unwrap();
    let msg = Message::new(MessageRole::Human, "Test");
    let tokens = counter.count_message_tokens(&msg);

    assert!(tokens > 0);
}

#[test]
fn test_create_counter_for_claude() {
    let counter = create_counter("claude-3-opus").unwrap();
    let msg = Message::new(MessageRole::Human, "Test");
    let tokens = counter.count_message_tokens(&msg);

    assert!(tokens > 0);
}

#[test]
fn test_create_counter_unknown_model() {
    // Unknown models should default to GPT-4
    let counter = create_counter("unknown-llm-v1");
    assert!(counter.is_ok());
}

// ===== Performance Tests =====

#[test]
fn test_token_counting_performance() {
    use std::time::Instant;

    let counter = TiktokenCounter::default();
    let msg = Message::new(MessageRole::Human, "This is a typical message of moderate length.");

    let start = Instant::now();
    for _ in 0..100 {
        let _ = counter.count_message_tokens(&msg);
    }
    let elapsed = start.elapsed();

    // Should complete 100 counts in <10ms (avg <100μs each)
    assert!(elapsed.as_millis() < 10,
        "100 token counts should take <10ms, took {}ms", elapsed.as_millis());
}

#[test]
fn test_trimming_performance() {
    use std::time::Instant;

    let counter = TiktokenCounter::default();
    let trimmer = ContextTrimmer::new(counter);
    let mut strategy = ContextStrategy::default();
    strategy.max_tokens = 1000;

    // Create 50 messages
    let mut messages = vec![];
    for i in 0..50 {
        messages.push(Message::new(MessageRole::Human, format!("Message {}", i)));
    }

    let start = Instant::now();
    let _trimmed = trimmer.trim(messages, &strategy);
    let elapsed = start.elapsed();

    // Trimming 50 messages should take <50ms
    assert!(elapsed.as_millis() < 50,
        "Trimming 50 messages should take <50ms, took {}ms", elapsed.as_millis());
}

// ===== Edge Cases =====

#[test]
fn test_fits_in_context_boundary() {
    let counter = TiktokenCounter::default();
    let messages = vec![Message::new(MessageRole::Human, "Test")];

    let tokens = counter.count_tokens(&messages);

    // Should fit in limit equal to token count
    assert!(counter.fits_in_context(&messages, tokens));

    // Should not fit in limit one less than token count
    assert!(!counter.fits_in_context(&messages, tokens - 1));
}

#[test]
fn test_coherence_preservation() {
    let counter = TiktokenCounter::default();
    let trimmer = ContextTrimmer::new(counter);
    let mut strategy = ContextStrategy::default();
    strategy.max_tokens = 250; // Force some trimming

    let messages = vec![
        Message::new(MessageRole::Human, "What is the weather?"),
        Message::new(MessageRole::Assistant, "Let me check the weather for you."),
        Message::new(MessageRole::Human, "What about tomorrow?"),
        Message::new(MessageRole::Assistant, "Tomorrow will be sunny."),
        Message::new(MessageRole::Human, "And the day after?"),
        Message::new(MessageRole::Assistant, "It will rain on the day after tomorrow."),
    ];

    let trimmed = trimmer.trim(messages, &strategy);

    // Check that we don't have orphaned assistant messages
    for (i, msg) in trimmed.iter().enumerate() {
        if msg.role == MessageRole::Assistant && i > 0 {
            // There should be a human message before it (or at the start)
            let has_preceding_human = trimmed[..i]
                .iter()
                .any(|m| m.role == MessageRole::Human);
            assert!(has_preceding_human || i == 0,
                "Assistant message should have preceding human message");
        }
    }
}
