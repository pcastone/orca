//! Execution Metrics models for tracking ReAct agent execution
//!
//! Three-level hierarchy for metrics:
//! - PromptExecution: Top-level aggregation for a complete prompt
//! - ExecutionIteration: Per agent -> tools -> agent cycle
//! - LlmCall: Individual LLM API calls

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Top-level prompt execution tracking
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PromptExecution {
    pub id: String,
    pub original_prompt: String,
    pub project_name: Option<String>,
    pub agent_type: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,

    // Aggregated token metrics
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_reasoning_tokens: i64,

    // Cost and timing
    pub total_cost_usd: f64,
    pub total_duration_ms: i64,

    // Counts
    pub iteration_count: i64,
    pub llm_call_count: i64,
    pub tool_call_count: i64,

    // Status
    pub status: String,
    pub final_response: Option<String>,
    pub error_message: Option<String>,

    // Timestamps
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Execution iteration (one agent -> tools -> agent cycle)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExecutionIteration {
    pub id: String,
    pub execution_id: String,
    pub iteration_num: i64,

    // Token metrics
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,

    // Timing
    pub duration_ms: i64,

    // Tool execution info (JSON)
    pub tool_calls: Option<String>,
    pub tool_results: Option<String>,

    // Agent state
    pub thought: Option<String>,
    pub action: Option<String>,
    pub observation: Option<String>,

    // Status
    pub status: String,

    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Individual LLM API call
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LlmCall {
    pub id: String,
    pub execution_id: String,
    pub iteration_id: Option<String>,

    // Provider info
    pub provider: String,
    pub model: String,

    // Token metrics
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,

    // Cost and timing
    pub cost_usd: f64,
    pub latency_ms: i64,

    // Request/response (optional)
    pub request_messages: Option<String>,
    pub response_content: Option<String>,

    pub created_at: String,
}

/// Aggregated metrics for display
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_reasoning_tokens: i64,
    pub total_cost_usd: f64,
    pub total_duration_ms: i64,
    pub iteration_count: i64,
    pub llm_call_count: i64,
    pub tool_call_count: i64,
}

/// Metrics for a single iteration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IterationMetrics {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub duration_ms: i64,
    pub tool_calls: Option<Vec<String>>,
    pub thought: Option<String>,
    pub action: Option<String>,
    pub observation: Option<String>,
}

/// LLM call metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmMetrics {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub cost_usd: f64,
    pub latency_ms: i64,
}

/// Execution summary for TUI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub project_name: Option<String>,
    pub agent_type: String,
    pub status: String,
    pub total_tokens: i64,
    pub cost_display: String,
    pub duration_display: String,
    pub execution: PromptExecution,
}

/// Detailed execution breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDetails {
    pub execution: PromptExecution,
    pub iterations: Vec<ExecutionIteration>,
}
