//! Execution metrics models for tracking ReAct agent iterations
//!
//! Three-level hierarchy:
//! - PromptExecution: Top-level (one per user prompt)
//! - ExecutionIteration: Per ReAct iteration (agent -> tools -> agent cycle)
//! - PromptHistory: Individual LLM calls (existing table)

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Top-level execution tracking (one per user prompt)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PromptExecution {
    /// Unique execution identifier (UUID string)
    pub id: String,

    /// Original user prompt
    pub original_prompt: String,

    /// Project name from config
    pub project_name: Option<String>,

    /// Agent type: react, plan_execute, reflection, direct
    pub agent_type: String,

    /// Session ID for grouping related executions
    pub session_id: Option<String>,

    /// Associated task ID
    pub task_id: Option<String>,

    /// Aggregated input tokens across all iterations
    pub total_input_tokens: i64,

    /// Aggregated output tokens across all iterations
    pub total_output_tokens: i64,

    /// Aggregated reasoning tokens (for extended thinking)
    pub total_reasoning_tokens: i64,

    /// Total cost in USD
    pub total_cost_usd: f64,

    /// Total duration in milliseconds
    pub total_duration_ms: i64,

    /// Number of iterations completed
    pub iteration_count: i64,

    /// Number of LLM calls made
    pub llm_call_count: i64,

    /// Number of tool calls made
    pub tool_call_count: i64,

    /// Status: running, completed, failed, cancelled
    pub status: String,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Final response from the agent
    pub final_response: Option<String>,

    /// Creation timestamp (ISO8601 string)
    pub created_at: String,

    /// Completion timestamp (ISO8601 string)
    pub completed_at: Option<String>,
}

impl PromptExecution {
    /// Create a new prompt execution
    pub fn new(id: String, prompt: String, project_name: Option<String>, agent_type: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            original_prompt: prompt,
            project_name,
            agent_type,
            session_id: None,
            task_id: None,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_reasoning_tokens: 0,
            total_cost_usd: 0.0,
            total_duration_ms: 0,
            iteration_count: 0,
            llm_call_count: 0,
            tool_call_count: 0,
            status: "running".to_string(),
            error_message: None,
            final_response: None,
            created_at: now,
            completed_at: None,
        }
    }
}

/// Per-iteration metrics (one ReAct cycle: agent -> tools -> agent)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExecutionIteration {
    /// Unique iteration identifier (UUID string)
    pub id: String,

    /// Parent execution ID
    pub execution_id: String,

    /// Iteration number (0-indexed)
    pub iteration_num: i64,

    /// Input tokens for this iteration
    pub input_tokens: i64,

    /// Output tokens for this iteration
    pub output_tokens: i64,

    /// Reasoning tokens for this iteration
    pub reasoning_tokens: i64,

    /// Duration in milliseconds for this iteration
    pub duration_ms: i64,

    /// Agent action: tool_call, final_answer, error
    pub agent_action: Option<String>,

    /// Tool calls made (JSON array string)
    pub tool_calls_json: Option<String>,

    /// Tool results (JSON array string)
    pub tool_results_json: Option<String>,

    /// Creation timestamp (ISO8601 string)
    pub created_at: String,
}

impl ExecutionIteration {
    /// Create a new execution iteration
    pub fn new(id: String, execution_id: String, iteration_num: i64) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            execution_id,
            iteration_num,
            input_tokens: 0,
            output_tokens: 0,
            reasoning_tokens: 0,
            duration_ms: 0,
            agent_action: None,
            tool_calls_json: None,
            tool_results_json: None,
            created_at: now,
        }
    }
}

/// Aggregated metrics for a prompt execution
#[derive(Debug, Clone, Default)]
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
#[derive(Debug, Clone, Default)]
pub struct IterationMetrics {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub duration_ms: i64,
    pub agent_action: Option<String>,
    pub tool_calls: Option<Vec<serde_json::Value>>,
    pub tool_results: Option<Vec<serde_json::Value>>,
}

/// Metrics for a single LLM call
#[derive(Debug, Clone, Default)]
pub struct LlmMetrics {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub latency_ms: i64,
    pub cost_usd: f64,
}

/// Summary of an execution for TUI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub execution: PromptExecution,
    pub project_name: Option<String>,
    pub agent_type: String,
    pub status: String,
    pub total_tokens: i64,
    pub cost_display: String,
    pub duration_display: String,
}

/// Detailed breakdown of an execution for drill-down
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDetails {
    pub execution: PromptExecution,
    pub iterations: Vec<ExecutionIteration>,
}
