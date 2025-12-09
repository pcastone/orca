//! Execution Metrics Service - Track ReAct agent execution metrics
//!
//! Provides high-level tracking of prompt executions, iterations, and LLM calls.

use crate::db::connection::DatabasePool;
use crate::db::models::{
    PromptExecution, ExecutionIteration, AggregatedMetrics,
    IterationMetrics, LlmMetrics, ExecutionSummary, ExecutionDetails, PromptHistory,
};
use crate::db::repositories::ExecutionMetricsRepository;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum MetricsError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Execution not found: {0}")]
    NotFound(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

pub type MetricsResult<T> = Result<T, MetricsError>;

/// Service for tracking execution metrics
pub struct ExecutionMetricsService {
    pool: Arc<DatabasePool>,
    project_name: Option<String>,
}

impl ExecutionMetricsService {
    /// Create a new ExecutionMetricsService
    pub fn new(pool: Arc<DatabasePool>, project_name: Option<String>) -> Self {
        Self { pool, project_name }
    }

    /// Start tracking a new prompt execution
    pub async fn start_execution(
        &self,
        prompt: &str,
        agent_type: &str,
        session_id: Option<String>,
        task_id: Option<String>,
    ) -> MetricsResult<ExecutionTracker> {
        let id = Uuid::new_v4().to_string();
        let execution = ExecutionMetricsRepository::create_execution(
            &self.pool,
            id.clone(),
            prompt.to_string(),
            self.project_name.clone(),
            agent_type.to_string(),
            session_id,
            task_id,
        ).await?;

        Ok(ExecutionTracker {
            execution_id: execution.id,
            pool: Arc::clone(&self.pool),
            current_iteration_id: None,
            current_iteration_num: 0,
            aggregated: AggregatedMetrics::default(),
            start_time: Instant::now(),
            iteration_start_time: None,
        })
    }

    /// Get execution summary for TUI display
    pub async fn get_execution_summary(&self, execution_id: &str) -> MetricsResult<ExecutionSummary> {
        let execution = ExecutionMetricsRepository::get_execution(&self.pool, execution_id).await?
            .ok_or_else(|| MetricsError::NotFound(execution_id.to_string()))?;

        let total_tokens = execution.total_input_tokens + execution.total_output_tokens;
        let cost_display = format!("${:.4}", execution.total_cost_usd);
        let duration_display = format_duration(execution.total_duration_ms);

        Ok(ExecutionSummary {
            project_name: execution.project_name.clone(),
            agent_type: execution.agent_type.clone(),
            status: execution.status.clone(),
            total_tokens,
            cost_display,
            duration_display,
            execution,
        })
    }

    /// Get detailed breakdown for drill-down
    pub async fn get_execution_details(&self, execution_id: &str) -> MetricsResult<ExecutionDetails> {
        let execution = ExecutionMetricsRepository::get_execution(&self.pool, execution_id).await?
            .ok_or_else(|| MetricsError::NotFound(execution_id.to_string()))?;

        let iterations = ExecutionMetricsRepository::get_iterations(&self.pool, execution_id).await?;

        Ok(ExecutionDetails {
            execution,
            iterations,
        })
    }

    /// List recent executions
    pub async fn list_recent(&self, limit: i64) -> MetricsResult<Vec<PromptExecution>> {
        let executions = ExecutionMetricsRepository::list_executions(&self.pool, limit, 0).await?;
        Ok(executions)
    }

    /// List executions by project
    pub async fn list_by_project(&self, project: &str) -> MetricsResult<Vec<PromptExecution>> {
        let executions = ExecutionMetricsRepository::list_by_project(&self.pool, project).await?;
        Ok(executions)
    }

    /// Get LLM calls for an iteration
    pub async fn get_iteration_llm_calls(&self, iteration_id: &str) -> MetricsResult<Vec<PromptHistory>> {
        let calls = ExecutionMetricsRepository::get_iteration_llm_calls(&self.pool, iteration_id).await?;
        Ok(calls)
    }

    /// Get all LLM calls for an execution
    pub async fn get_execution_llm_calls(&self, execution_id: &str) -> MetricsResult<Vec<PromptHistory>> {
        let calls = ExecutionMetricsRepository::get_llm_calls(&self.pool, execution_id).await?;
        Ok(calls)
    }
}

/// Tracks an ongoing execution with iteration/call tracking
pub struct ExecutionTracker {
    execution_id: String,
    pool: Arc<DatabasePool>,
    current_iteration_id: Option<String>,
    current_iteration_num: i64,
    aggregated: AggregatedMetrics,
    start_time: Instant,
    iteration_start_time: Option<Instant>,
}

impl ExecutionTracker {
    /// Get the execution ID
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// Get current iteration number
    pub fn current_iteration(&self) -> i64 {
        self.current_iteration_num
    }

    /// Start a new iteration
    pub async fn start_iteration(&mut self) -> MetricsResult<String> {
        let id = Uuid::new_v4().to_string();
        self.current_iteration_num += 1;

        let iteration = ExecutionMetricsRepository::create_iteration(
            &self.pool,
            id.clone(),
            self.execution_id.clone(),
            self.current_iteration_num - 1, // 0-indexed
        ).await?;

        self.current_iteration_id = Some(iteration.id.clone());
        self.iteration_start_time = Some(Instant::now());
        self.aggregated.iteration_count += 1;

        // Update the execution's iteration count
        ExecutionMetricsRepository::increment_iteration_count(&self.pool, &self.execution_id).await?;

        Ok(iteration.id)
    }

    /// End the current iteration with metrics
    pub async fn end_iteration(&mut self, mut metrics: IterationMetrics) -> MetricsResult<()> {
        let iteration_id = self.current_iteration_id.as_ref()
            .ok_or_else(|| MetricsError::InvalidState("No iteration in progress".to_string()))?;

        // Calculate duration if not set
        if metrics.duration_ms == 0 {
            if let Some(start) = self.iteration_start_time.take() {
                metrics.duration_ms = start.elapsed().as_millis() as i64;
            }
        }

        // Update tool count
        if let Some(ref tools) = metrics.tool_calls {
            let tool_count = tools.len() as i64;
            self.aggregated.tool_call_count += tool_count;
            ExecutionMetricsRepository::increment_tool_count(&self.pool, &self.execution_id, tool_count).await?;
        }

        ExecutionMetricsRepository::complete_iteration(&self.pool, iteration_id, &metrics).await?;

        Ok(())
    }

    /// Record an LLM call for the current iteration
    pub async fn record_llm_call(
        &mut self,
        provider: &str,
        model: &str,
        metrics: &LlmMetrics,
    ) -> MetricsResult<()> {
        // Update aggregated metrics
        self.aggregated.total_input_tokens += metrics.input_tokens;
        self.aggregated.total_output_tokens += metrics.output_tokens;
        self.aggregated.total_reasoning_tokens += metrics.reasoning_tokens;
        self.aggregated.total_cost_usd += metrics.cost_usd;
        self.aggregated.total_duration_ms += metrics.latency_ms;
        self.aggregated.llm_call_count += 1;

        // Update the execution's metrics
        ExecutionMetricsRepository::update_metrics(
            &self.pool,
            &self.execution_id,
            metrics.input_tokens,
            metrics.output_tokens,
            metrics.reasoning_tokens,
            metrics.cost_usd,
            metrics.latency_ms,
        ).await?;

        Ok(())
    }

    /// Complete the execution successfully
    pub async fn complete(self, response: &str) -> MetricsResult<AggregatedMetrics> {
        let mut final_metrics = self.aggregated.clone();
        final_metrics.total_duration_ms = self.start_time.elapsed().as_millis() as i64;

        ExecutionMetricsRepository::complete_execution(
            &self.pool,
            &self.execution_id,
            response,
            &final_metrics,
        ).await?;

        Ok(final_metrics)
    }

    /// Fail the execution with an error
    pub async fn fail(self, error: &str) -> MetricsResult<()> {
        ExecutionMetricsRepository::fail_execution(&self.pool, &self.execution_id, error).await?;
        Ok(())
    }

    /// Get current aggregated metrics
    pub fn current_metrics(&self) -> &AggregatedMetrics {
        &self.aggregated
    }
}

/// Format duration in human-readable format
fn format_duration(ms: i64) -> String {
    if ms < 1000 {
        format!("{} ms", ms)
    } else if ms < 60_000 {
        format!("{:.1} s", ms as f64 / 1000.0)
    } else {
        let minutes = ms / 60_000;
        let seconds = (ms % 60_000) / 1000;
        format!("{}m {}s", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(format_duration(500), "500 ms");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(2500), "2.5 s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(125000), "2m 5s");
    }
}
