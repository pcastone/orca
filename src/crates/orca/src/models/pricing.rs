use serde::{Deserialize, Serialize};

/// LLM pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPricing {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub cost_per_input_token: f64,     // Cost per 1 token
    pub cost_per_output_token: f64,    // Cost per 1 token
    pub cost_per_reasoning_token: Option<f64>, // For thinking models
    pub updated_at: i64,
}

impl LlmPricing {
    /// Calculate cost for a request
    pub fn calculate_cost(
        &self,
        input_tokens: usize,
        output_tokens: usize,
        reasoning_tokens: Option<usize>,
    ) -> f64 {
        let input_cost = input_tokens as f64 * self.cost_per_input_token;
        let output_cost = output_tokens as f64 * self.cost_per_output_token;
        let reasoning_cost = reasoning_tokens
            .map(|t| t as f64 * self.cost_per_reasoning_token.unwrap_or(0.0))
            .unwrap_or(0.0);

        input_cost + output_cost + reasoning_cost
    }
}

/// Default LLM pricing data (as of November 2024)
pub fn default_pricing() -> Vec<(&'static str, &'static str, f64, f64, Option<f64>)> {
    vec![
        // OpenAI
        ("openai", "gpt-4-turbo", 0.00001, 0.00003, None),
        ("openai", "gpt-4", 0.00003, 0.00006, None),
        ("openai", "gpt-4o", 0.000005, 0.000015, None),
        ("openai", "gpt-3.5-turbo", 0.0000005, 0.0000015, None),

        // Anthropic Claude
        ("anthropic", "claude-3-opus", 0.000015, 0.000075, None),
        ("anthropic", "claude-3-sonnet", 0.000003, 0.000015, None),
        ("anthropic", "claude-3-haiku", 0.00000025, 0.00000125, None),
        ("anthropic", "claude-3-5-sonnet", 0.000003, 0.000015, None),
        ("anthropic", "claude-3-5-haiku", 0.00000080, 0.000004, None),

        // Google Gemini
        ("google", "gemini-pro", 0.0000005, 0.0000015, None),
        ("google", "gemini-pro-vision", 0.0000005, 0.0000015, None),
        ("google", "gemini-1-5-pro", 0.00000125, 0.000005, None),

        // Grok (xAI)
        ("grok", "grok-beta", 0.000005, 0.000015, None),

        // DeepSeek
        ("deepseek", "deepseek-chat", 0.00000014, 0.00000028, None),
        ("deepseek", "deepseek-coder", 0.00000014, 0.00000028, None),

        // OpenRouter (aggregator)
        ("openrouter", "openai/gpt-4-turbo", 0.00001, 0.00003, None),
        ("openrouter", "anthropic/claude-3-sonnet", 0.000003, 0.000015, None),

        // Local models (free)
        ("ollama", "llama2", 0.0, 0.0, None),
        ("ollama", "mistral", 0.0, 0.0, None),
        ("ollama", "neural-chat", 0.0, 0.0, None),
        ("llama_cpp", "default", 0.0, 0.0, None),
        ("lmstudio", "default", 0.0, 0.0, None),
    ]
}
