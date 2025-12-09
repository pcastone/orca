//! Execution Metrics Service - Track ReAct agent execution metrics
//!
//! Provides high-level tracking of prompt executions, iterations, and LLM calls.

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::{
    AggregatedMetrics, ExecutionDetails, ExecutionIteration, ExecutionSummary,
    IterationMetrics, LlmCall, LlmMetrics, PromptExecution,
};
use crate::repositories::ExecutionMetricsRepository;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Service for tracking execution metrics
pub struct ExecutionMetricsService {
    repo: ExecutionMetricsRepository,
    project_name: Option<String>,
}

impl ExecutionMetricsService {
    /// Create a new ExecutionMetricsService
    pub fn new(db: Arc<Database>, project_name: Option<String>) -> Self {
        Self {
            repo: ExecutionMetricsRepository::new(db),
            project_name,
        }
    }

    /// Start tracking a new prompt execution
    pub async fn start_execution(
        &self,
        prompt: &str,
        agent_type: &str,
        session_id: Option<String>,
        task_id: Option<String>,
    ) -> Result<ExecutionTracker> {
        let id = Uuid::new_v4().to_string();
        let execution = self
            .repo
            .create_execution(
                id.clone(),
                prompt.to_string(),
                self.project_name.clone(),
                agent_type.to_string(),
                session_id,
                task_id,
            )
            .await?;

        Ok(ExecutionTracker {
            execution_id: execution.id,
            repo: self.repo.clone(),
            current_iteration_id: None,
            current_iteration_num: 0,
            aggregated: AggregatedMetrics::default(),
            start_time: Instant::now(),
            iteration_start_time: None,
        })
    }

    /// Get execution summary for TUI display
    pub async fn get_execution_summary(&self, execution_id: &str) -> Result<ExecutionSummary> {
        let execution = self
            .repo
            .get_execution(execution_id)
            .await?
            .ok_or_else(|| OrcaError::NotFound(format!("Execution not found: {}", execution_id)))?;

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
    pub async fn get_execution_details(&self, execution_id: &str) -> Result<ExecutionDetails> {
        let execution = self
            .repo
            .get_execution(execution_id)
            .await?
            .ok_or_else(|| OrcaError::NotFound(format!("Execution not found: {}", execution_id)))?;

        let iterations = self.repo.get_iterations(execution_id).await?;

        Ok(ExecutionDetails {
            execution,
            iterations,
        })
    }

    /// List recent executions
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<PromptExecution>> {
        self.repo.list_executions(limit, 0).await
    }

    /// List executions by project
    pub async fn list_by_project(&self, project: &str) -> Result<Vec<PromptExecution>> {
        self.repo.list_by_project(project).await
    }

    /// Get LLM calls for an iteration
    pub async fn get_iteration_llm_calls(&self, iteration_id: &str) -> Result<Vec<LlmCall>> {
        self.repo.get_iteration_llm_calls(iteration_id).await
    }

    /// Get all LLM calls for an execution
    pub async fn get_execution_llm_calls(&self, execution_id: &str) -> Result<Vec<LlmCall>> {
        self.repo.get_llm_calls(execution_id).await
    }
}

/// Tracks an ongoing execution with iteration/call tracking
pub struct ExecutionTracker {
    execution_id: String,
    repo: ExecutionMetricsRepository,
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
    pub async fn start_iteration(&mut self) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        self.current_iteration_num += 1;

        let iteration = self
            .repo
            .create_iteration(
                id.clone(),
                self.execution_id.clone(),
                self.current_iteration_num - 1, // 0-indexed
            )
            .await?;

        self.current_iteration_id = Some(iteration.id.clone());
        self.iteration_start_time = Some(Instant::now());
        self.aggregated.iteration_count += 1;

        // Update the execution's iteration count
        self.repo
            .increment_iteration_count(&self.execution_id)
            .await?;

        Ok(iteration.id)
    }

    /// End the current iteration with metrics
    pub async fn end_iteration(&mut self, mut metrics: IterationMetrics) -> Result<()> {
        let iteration_id = self.current_iteration_id.as_ref().ok_or_else(|| {
            OrcaError::Execution("No iteration in progress".to_string())
        })?;

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
            self.repo
                .increment_tool_count(&self.execution_id, tool_count)
                .await?;
        }

        self.repo.complete_iteration(iteration_id, &metrics).await?;

        Ok(())
    }

    /// Record an LLM call for the current iteration
    pub async fn record_llm_call(
        &mut self,
        provider: &str,
        model: &str,
        metrics: &LlmMetrics,
    ) -> Result<()> {
        // Update aggregated metrics
        self.aggregated.total_input_tokens += metrics.input_tokens;
        self.aggregated.total_output_tokens += metrics.output_tokens;
        self.aggregated.total_reasoning_tokens += metrics.reasoning_tokens;
        self.aggregated.total_cost_usd += metrics.cost_usd;
        self.aggregated.total_duration_ms += metrics.latency_ms;
        self.aggregated.llm_call_count += 1;

        // Record in database
        let id = Uuid::new_v4().to_string();
        self.repo
            .record_llm_call(
                id,
                self.execution_id.clone(),
                self.current_iteration_id.clone(),
                provider.to_string(),
                model.to_string(),
                metrics,
            )
            .await?;

        // Update the execution's metrics
        self.repo
            .update_metrics(
                &self.execution_id,
                metrics.input_tokens,
                metrics.output_tokens,
                metrics.reasoning_tokens,
                metrics.cost_usd,
                metrics.latency_ms,
            )
            .await?;

        Ok(())
    }

    /// Complete the execution successfully
    pub async fn complete(mut self, response: &str) -> Result<AggregatedMetrics> {
        self.aggregated.total_duration_ms = self.start_time.elapsed().as_millis() as i64;

        self.repo
            .complete_execution(&self.execution_id, response, &self.aggregated)
            .await?;

        Ok(self.aggregated)
    }

    /// Fail the execution with an error
    pub async fn fail(self, error: &str) -> Result<()> {
        self.repo.fail_execution(&self.execution_id, error).await?;
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
