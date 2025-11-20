//! Prompt history model for database persistence

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents an LLM prompt/response in the orchestrator database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PromptHistory {
    /// Unique prompt history identifier (UUID string)
    pub id: String,

    /// Associated task ID (optional)
    pub task_id: Option<String>,

    /// Associated workflow ID (optional)
    pub workflow_id: Option<String>,

    /// Associated execution ID (optional)
    pub execution_id: Option<String>,

    /// Session ID for grouping related prompts
    pub session_id: Option<String>,

    /// Node ID in workflow
    pub node_id: Option<String>,

    /// LLM provider (e.g., "anthropic", "openai")
    pub provider: String,

    /// Model name (e.g., "claude-3-opus", "gpt-4")
    pub model: String,

    /// Prompt type: chat, completion, embedding, tool_use
    pub prompt_type: String,

    /// System prompt content
    pub system_prompt: Option<String>,

    /// User prompt content
    pub user_prompt: String,

    /// Assistant response content
    pub assistant_response: Option<String>,

    /// Full message history (JSON string)
    pub messages: Option<String>,

    /// Input token count
    pub input_tokens: Option<i32>,

    /// Output token count
    pub output_tokens: Option<i32>,

    /// Total token count
    pub total_tokens: Option<i32>,

    /// Cost in USD
    pub cost_usd: Option<f64>,

    /// Response latency in milliseconds
    pub latency_ms: Option<i32>,

    /// Temperature setting
    pub temperature: Option<f64>,

    /// Max tokens setting
    pub max_tokens: Option<i32>,

    /// Top-p setting
    pub top_p: Option<f64>,

    /// Stop sequences (JSON array string)
    pub stop_sequences: Option<String>,

    /// Available tools (JSON array string)
    pub tools_available: Option<String>,

    /// Tool calls made (JSON array string)
    pub tool_calls: Option<String>,

    /// Status: pending, streaming, completed, failed, cancelled
    pub status: String,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Additional metadata (JSON string)
    pub metadata: Option<String>,

    /// Creation timestamp (ISO8601 string)
    pub created_at: String,
}

impl PromptHistory {
    /// Create a new prompt history entry
    pub fn new(
        id: String,
        provider: String,
        model: String,
        user_prompt: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            task_id: None,
            workflow_id: None,
            execution_id: None,
            session_id: None,
            node_id: None,
            provider,
            model,
            prompt_type: "chat".to_string(),
            system_prompt: None,
            user_prompt,
            assistant_response: None,
            messages: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            cost_usd: None,
            latency_ms: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop_sequences: None,
            tools_available: None,
            tool_calls: None,
            status: "completed".to_string(),
            error_message: None,
            metadata: None,
            created_at: now,
        }
    }
}
