//! Configuration types for LLM requests.
//!
//! This module provides types for configuring LLM behavior, including:
//! - Request configuration (temperature, tokens, etc.)
//! - Reasoning modes for thinking models
//! - Stop sequences and other generation parameters

use crate::llm::tools::ToolDefinition;
use crate::Message;
use serde::{Deserialize, Serialize};

/// A request to a chat model containing messages and configuration.
///
/// This is the primary input type for `ChatModel::chat()` and `ChatModel::stream()`.
/// It bundles messages with optional configuration parameters.
///
/// # Example
///
/// ```rust,ignore
/// use langgraph_core::llm::{ChatRequest, ReasoningMode};
/// use langgraph_core::Message;
///
/// let request = ChatRequest::new(vec![
///     Message::system("You are a helpful assistant"),
///     Message::human("What is the capital of France?"),
/// ])
/// .with_temperature(0.7)
/// .with_max_tokens(1000)
/// .with_reasoning(ReasoningMode::Separated);
/// ```
#[derive(Debug, Clone)]
pub struct ChatRequest {
    /// The conversation messages to send to the model.
    pub messages: Vec<Message>,

    /// Optional configuration for generation behavior.
    pub config: ChatConfig,
}

impl ChatRequest {
    /// Create a new chat request with the given messages.
    ///
    /// Uses default configuration. Customize with builder methods.
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            config: ChatConfig::default(),
        }
    }

    /// Set the temperature for generation.
    ///
    /// Temperature controls randomness:
    /// - Lower values (0.0-0.3): More deterministic, focused
    /// - Medium values (0.4-0.7): Balanced creativity and coherence
    /// - Higher values (0.8-2.0): More creative, diverse, potentially less coherent
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.config.temperature = Some(temperature);
        self
    }

    /// Set the maximum number of tokens to generate.
    ///
    /// This limits the response length. The exact interpretation depends on
    /// the provider (some count only output tokens, others include input).
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.config.max_tokens = Some(max_tokens);
        self
    }

    /// Set the reasoning mode for thinking models.
    ///
    /// This controls how models with extended reasoning capabilities (like
    /// OpenAI o1 or DeepSeek R1) handle their "thinking" process.
    ///
    /// See `ReasoningMode` for details on each mode.
    pub fn with_reasoning(mut self, mode: ReasoningMode) -> Self {
        self.config.reasoning_mode = mode;
        self
    }

    /// Add stop sequences that halt generation.
    ///
    /// When the model generates any of these sequences, generation stops.
    /// Common uses:
    /// - Structured output formatting (stop at closing tags)
    /// - Multi-turn conversations (stop at turn markers)
    /// - Code generation (stop at certain keywords)
    pub fn with_stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.config.stop_sequences = sequences;
        self
    }

    /// Bind tools/functions that the model can call.
    ///
    /// For models that support function calling, this provides the definitions
    /// of available tools. The model can then request tool execution in its
    /// response via `tool_calls`.
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.config.tools = tools;
        self
    }

    /// Set top-p (nucleus) sampling parameter.
    ///
    /// An alternative to temperature. Only tokens with cumulative probability
    /// up to `top_p` are considered. Values:
    /// - 1.0: Consider all tokens (default)
    /// - 0.9: Consider top 90% probability mass
    /// - 0.1: Very deterministic, only highest probability tokens
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.config.top_p = Some(top_p);
        self
    }

    /// Set frequency penalty to reduce repetition.
    ///
    /// Positive values penalize tokens that have appeared frequently,
    /// reducing repetitive text.
    /// - 0.0: No penalty (default)
    /// - 1.0: Strong penalty
    pub fn with_frequency_penalty(mut self, penalty: f32) -> Self {
        self.config.frequency_penalty = Some(penalty);
        self
    }

    /// Set presence penalty to encourage topic diversity.
    ///
    /// Positive values penalize tokens that have appeared at all,
    /// encouraging exploration of new topics.
    /// - 0.0: No penalty (default)
    /// - 1.0: Strong penalty
    pub fn with_presence_penalty(mut self, penalty: f32) -> Self {
        self.config.presence_penalty = Some(penalty);
        self
    }
}

/// Configuration parameters for chat generation.
///
/// These parameters control the LLM's generation behavior. Not all parameters
/// are supported by all providers - implementations should document which
/// parameters they honor.
#[derive(Debug, Clone, Default)]
pub struct ChatConfig {
    /// Sampling temperature (0.0-2.0, provider-dependent).
    ///
    /// Controls randomness in generation. Lower = more deterministic.
    pub temperature: Option<f32>,

    /// Maximum tokens to generate.
    ///
    /// Response will be truncated if it exceeds this limit.
    pub max_tokens: Option<usize>,

    /// How to handle reasoning/thinking for models that support it.
    ///
    /// See `ReasoningMode` for details.
    pub reasoning_mode: ReasoningMode,

    /// Sequences that stop generation when encountered.
    ///
    /// Useful for structured output or controlling generation length.
    pub stop_sequences: Vec<String>,

    /// Tool/function definitions for function-calling models.
    ///
    /// If provided, the model may request tool execution via `tool_calls`
    /// in the response.
    pub tools: Vec<ToolDefinition>,

    /// Top-p (nucleus) sampling parameter (0.0-1.0).
    ///
    /// Alternative to temperature for controlling randomness.
    pub top_p: Option<f32>,

    /// Frequency penalty (-2.0 to 2.0, provider-dependent).
    ///
    /// Penalizes tokens based on their frequency in the text so far.
    pub frequency_penalty: Option<f32>,

    /// Presence penalty (-2.0 to 2.0, provider-dependent).
    ///
    /// Penalizes tokens based on whether they've appeared in the text so far.
    pub presence_penalty: Option<f32>,
}

/// Controls how thinking/reasoning content is handled for capable models.
///
/// Some models (OpenAI o1, DeepSeek R1, etc.) perform extended "thinking"
/// before generating a final answer. This enum controls how that reasoning
/// is captured and exposed.
///
/// # Model Support
///
/// ## Thinking Models
/// - OpenAI: o1, o1-mini, o1-preview
/// - DeepSeek: deepseek-r1:1.5b, deepseek-r1:8b, etc.
/// - Others with `<think>` or similar reasoning patterns
///
/// ## Standard Models
/// For models without thinking capabilities, this parameter is ignored.
///
/// # Example
///
/// ```rust,ignore
/// use langgraph_core::llm::{ChatRequest, ReasoningMode};
///
/// // See the model's thinking process
/// let request = ChatRequest::new(messages)
///     .with_reasoning(ReasoningMode::Separated);
///
/// let response = model.chat(request).await?;
/// if let Some(reasoning) = response.reasoning {
///     println!("Model thought: {}", reasoning.content);
/// }
/// println!("Final answer: {}", response.message.text());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningMode {
    /// Exclude reasoning/thinking content entirely.
    ///
    /// The model performs its internal reasoning but it is not included
    /// in the response at all. Useful for production where you only need
    /// the final answer.
    ///
    /// **Response**: `response.reasoning` will be `None`
    Disabled,

    /// Use the model's default behavior for reasoning.
    ///
    /// The model may include reasoning in the response message content,
    /// or handle it however it normally does. This is provider-specific.
    ///
    /// **Response**: Reasoning may be in `response.message.content`, or
    /// provider metadata.
    #[default]
    Default,

    /// Separate reasoning into a dedicated field.
    ///
    /// The reasoning/thinking process is extracted and provided separately
    /// from the final answer. This is the recommended mode for debugging
    /// or displaying the model's thought process.
    ///
    /// **Response**: `response.reasoning.content` contains thinking,
    /// `response.message.content` contains final answer only.
    Separated,

    /// Request extended reasoning (model-dependent).
    ///
    /// For models that support it (like OpenAI o1), this requests more
    /// thorough reasoning before answering. May increase latency and cost.
    ///
    /// **Response**: `response.reasoning` may have higher token count,
    /// longer duration.
    ///
    /// **Note**: Not all models support this. Falls back to `Separated`
    /// if not supported.
    Extended,
}

impl ReasoningMode {
    /// Check if reasoning content should be captured.
    ///
    /// Returns `true` for modes that need reasoning content extracted.
    pub fn should_capture(&self) -> bool {
        matches!(self, ReasoningMode::Separated | ReasoningMode::Extended)
    }

    /// Check if extended reasoning is requested.
    pub fn is_extended(&self) -> bool {
        matches!(self, ReasoningMode::Extended)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_builder() {
        let request = ChatRequest::new(vec![Message::human("test")])
            .with_temperature(0.7)
            .with_max_tokens(100)
            .with_reasoning(ReasoningMode::Separated);

        assert_eq!(request.config.temperature, Some(0.7));
        assert_eq!(request.config.max_tokens, Some(100));
        assert_eq!(request.config.reasoning_mode, ReasoningMode::Separated);
    }

    #[test]
    fn test_reasoning_mode_should_capture() {
        assert!(!ReasoningMode::Disabled.should_capture());
        assert!(!ReasoningMode::Default.should_capture());
        assert!(ReasoningMode::Separated.should_capture());
        assert!(ReasoningMode::Extended.should_capture());
    }

    #[test]
    fn test_default_config() {
        let config = ChatConfig::default();
        assert_eq!(config.reasoning_mode, ReasoningMode::Default);
        assert!(config.stop_sequences.is_empty());
        assert!(config.tools.is_empty());
    }
}
