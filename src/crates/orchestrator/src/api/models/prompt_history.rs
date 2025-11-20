//! Prompt history API models and DTOs

use serde::{Deserialize, Serialize};

/// Request to create a new prompt history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePromptHistoryRequest {
    /// LLM provider (required)
    pub provider: String,

    /// Model name (required)
    pub model: String,

    /// User prompt (required)
    pub user_prompt: String,

    /// System prompt (optional)
    pub system_prompt: Option<String>,

    /// Assistant response (optional)
    pub assistant_response: Option<String>,

    /// Associated task ID (optional)
    pub task_id: Option<String>,

    /// Associated workflow ID (optional)
    pub workflow_id: Option<String>,

    /// Associated execution ID (optional)
    pub execution_id: Option<String>,

    /// Session ID (optional)
    pub session_id: Option<String>,

    /// Input token count (optional)
    pub input_tokens: Option<i32>,

    /// Output token count (optional)
    pub output_tokens: Option<i32>,

    /// Cost in USD (optional)
    pub cost_usd: Option<f64>,

    /// Latency in milliseconds (optional)
    pub latency_ms: Option<i32>,

    /// Temperature setting (optional)
    pub temperature: Option<f64>,

    /// Max tokens setting (optional)
    pub max_tokens: Option<i32>,

    /// Tools available (optional, JSON)
    pub tools_available: Option<String>,

    /// Tool calls made (optional, JSON)
    pub tool_calls: Option<String>,

    /// Additional metadata (optional, JSON)
    pub metadata: Option<String>,
}

impl CreatePromptHistoryRequest {
    /// Validate the create request
    pub fn validate(&self) -> crate::api::error::ApiResult<()> {
        crate::api::middleware::validation::validate_not_empty(&self.provider, "provider")?;
        crate::api::middleware::validation::validate_not_empty(&self.model, "model")?;
        crate::api::middleware::validation::validate_not_empty(&self.user_prompt, "user_prompt")?;
        Ok(())
    }
}

/// Prompt history response for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptHistoryResponse {
    pub id: String,
    pub task_id: Option<String>,
    pub workflow_id: Option<String>,
    pub execution_id: Option<String>,
    pub session_id: Option<String>,
    pub node_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub prompt_type: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub assistant_response: Option<String>,
    pub messages: Option<String>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub cost_usd: Option<f64>,
    pub latency_ms: Option<i32>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub top_p: Option<f64>,
    pub stop_sequences: Option<String>,
    pub tools_available: Option<String>,
    pub tool_calls: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

impl PromptHistoryResponse {
    /// Create a PromptHistoryResponse from database model
    pub fn from_db_prompt(prompt: crate::db::models::PromptHistory) -> Self {
        Self {
            id: prompt.id,
            task_id: prompt.task_id,
            workflow_id: prompt.workflow_id,
            execution_id: prompt.execution_id,
            session_id: prompt.session_id,
            node_id: prompt.node_id,
            provider: prompt.provider,
            model: prompt.model,
            prompt_type: prompt.prompt_type,
            system_prompt: prompt.system_prompt,
            user_prompt: prompt.user_prompt,
            assistant_response: prompt.assistant_response,
            messages: prompt.messages,
            input_tokens: prompt.input_tokens,
            output_tokens: prompt.output_tokens,
            total_tokens: prompt.total_tokens,
            cost_usd: prompt.cost_usd,
            latency_ms: prompt.latency_ms,
            temperature: prompt.temperature,
            max_tokens: prompt.max_tokens,
            top_p: prompt.top_p,
            stop_sequences: prompt.stop_sequences,
            tools_available: prompt.tools_available,
            tool_calls: prompt.tool_calls,
            status: prompt.status,
            error_message: prompt.error_message,
            metadata: prompt.metadata,
            created_at: prompt.created_at,
        }
    }
}

/// Query parameters for listing prompt history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptHistoryListQuery {
    /// Filter by task ID (optional)
    pub task_id: Option<String>,

    /// Filter by workflow ID (optional)
    pub workflow_id: Option<String>,

    /// Filter by execution ID (optional)
    pub execution_id: Option<String>,

    /// Filter by session ID (optional)
    pub session_id: Option<String>,

    /// Filter by provider (optional)
    pub provider: Option<String>,

    /// Filter by model (optional)
    pub model: Option<String>,

    /// Current page (0-indexed, default 0)
    pub page: Option<u32>,

    /// Items per page (default 20, max 100)
    pub per_page: Option<u32>,
}

/// Prompt history statistics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptHistoryStatsResponse {
    pub total_prompts: i64,
    pub total_tokens: i64,
    pub total_cost_usd: f64,
}
