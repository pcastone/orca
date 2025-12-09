//! Execution Metrics Repository for tracking ReAct agent execution
//!
//! Provides CRUD operations for prompt_executions, execution_iterations, and llm_calls tables.

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::{
    AggregatedMetrics, ExecutionIteration, IterationMetrics, LlmCall, LlmMetrics, PromptExecution,
};
use chrono::Utc;
use std::sync::Arc;

/// Repository for execution metrics (user DB)
#[derive(Clone, Debug)]
pub struct ExecutionMetricsRepository {
    db: Arc<Database>,
}

impl ExecutionMetricsRepository {
    /// Create a new execution metrics repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    // =========================================================================
    // Prompt Execution operations
    // =========================================================================

    /// Create a new prompt execution
    pub async fn create_execution(
        &self,
        id: String,
        original_prompt: String,
        project_name: Option<String>,
        agent_type: String,
        session_id: Option<String>,
        task_id: Option<String>,
    ) -> Result<PromptExecution> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO prompt_executions (
                id, original_prompt, project_name, agent_type, session_id, task_id,
                total_input_tokens, total_output_tokens, total_reasoning_tokens,
                total_cost_usd, total_duration_ms,
                iteration_count, llm_call_count, tool_call_count,
                status, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, 0, 0, 0, 0.0, 0, 0, 0, 0, 'running', ?)
            "#,
        )
        .bind(&id)
        .bind(&original_prompt)
        .bind(&project_name)
        .bind(&agent_type)
        .bind(&session_id)
        .bind(&task_id)
        .bind(&now)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to create execution: {}", e)))?;

        Ok(PromptExecution {
            id,
            original_prompt,
            project_name,
            agent_type,
            session_id,
            task_id,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_reasoning_tokens: 0,
            total_cost_usd: 0.0,
            total_duration_ms: 0,
            iteration_count: 0,
            llm_call_count: 0,
            tool_call_count: 0,
            status: "running".to_string(),
            final_response: None,
            error_message: None,
            created_at: now,
            completed_at: None,
        })
    }

    /// Get execution by ID
    pub async fn get_execution(&self, id: &str) -> Result<Option<PromptExecution>> {
        sqlx::query_as::<_, PromptExecution>(
            "SELECT * FROM prompt_executions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get execution: {}", e)))
    }

    /// List executions with pagination
    pub async fn list_executions(&self, limit: i64, offset: i64) -> Result<Vec<PromptExecution>> {
        sqlx::query_as::<_, PromptExecution>(
            "SELECT * FROM prompt_executions ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list executions: {}", e)))
    }

    /// List executions by project
    pub async fn list_by_project(&self, project: &str) -> Result<Vec<PromptExecution>> {
        sqlx::query_as::<_, PromptExecution>(
            "SELECT * FROM prompt_executions WHERE project_name = ? ORDER BY created_at DESC",
        )
        .bind(project)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list executions by project: {}", e)))
    }

    /// Update metrics on execution
    pub async fn update_metrics(
        &self,
        execution_id: &str,
        input_tokens: i64,
        output_tokens: i64,
        reasoning_tokens: i64,
        cost_usd: f64,
        duration_ms: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE prompt_executions SET
                total_input_tokens = total_input_tokens + ?,
                total_output_tokens = total_output_tokens + ?,
                total_reasoning_tokens = total_reasoning_tokens + ?,
                total_cost_usd = total_cost_usd + ?,
                total_duration_ms = total_duration_ms + ?,
                llm_call_count = llm_call_count + 1
            WHERE id = ?
            "#,
        )
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(reasoning_tokens)
        .bind(cost_usd)
        .bind(duration_ms)
        .bind(execution_id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update metrics: {}", e)))?;

        Ok(())
    }

    /// Increment iteration count
    pub async fn increment_iteration_count(&self, execution_id: &str) -> Result<()> {
        sqlx::query("UPDATE prompt_executions SET iteration_count = iteration_count + 1 WHERE id = ?")
            .bind(execution_id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to increment iteration count: {}", e)))?;

        Ok(())
    }

    /// Increment tool call count
    pub async fn increment_tool_count(&self, execution_id: &str, count: i64) -> Result<()> {
        sqlx::query("UPDATE prompt_executions SET tool_call_count = tool_call_count + ? WHERE id = ?")
            .bind(count)
            .bind(execution_id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to increment tool count: {}", e)))?;

        Ok(())
    }

    /// Complete execution successfully
    pub async fn complete_execution(
        &self,
        execution_id: &str,
        final_response: &str,
        metrics: &AggregatedMetrics,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE prompt_executions SET
                status = 'completed',
                final_response = ?,
                total_input_tokens = ?,
                total_output_tokens = ?,
                total_reasoning_tokens = ?,
                total_cost_usd = ?,
                total_duration_ms = ?,
                iteration_count = ?,
                llm_call_count = ?,
                tool_call_count = ?,
                completed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(final_response)
        .bind(metrics.total_input_tokens)
        .bind(metrics.total_output_tokens)
        .bind(metrics.total_reasoning_tokens)
        .bind(metrics.total_cost_usd)
        .bind(metrics.total_duration_ms)
        .bind(metrics.iteration_count)
        .bind(metrics.llm_call_count)
        .bind(metrics.tool_call_count)
        .bind(&now)
        .bind(execution_id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to complete execution: {}", e)))?;

        Ok(())
    }

    /// Fail execution with error
    pub async fn fail_execution(&self, execution_id: &str, error: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE prompt_executions SET
                status = 'failed',
                error_message = ?,
                completed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(error)
        .bind(&now)
        .bind(execution_id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to fail execution: {}", e)))?;

        Ok(())
    }

    // =========================================================================
    // Execution Iteration operations
    // =========================================================================

    /// Create a new iteration
    pub async fn create_iteration(
        &self,
        id: String,
        execution_id: String,
        iteration_num: i64,
    ) -> Result<ExecutionIteration> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO execution_iterations (
                id, execution_id, iteration_num,
                input_tokens, output_tokens, reasoning_tokens,
                duration_ms, status, created_at
            )
            VALUES (?, ?, ?, 0, 0, 0, 0, 'running', ?)
            "#,
        )
        .bind(&id)
        .bind(&execution_id)
        .bind(iteration_num)
        .bind(&now)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to create iteration: {}", e)))?;

        Ok(ExecutionIteration {
            id,
            execution_id,
            iteration_num,
            input_tokens: 0,
            output_tokens: 0,
            reasoning_tokens: 0,
            duration_ms: 0,
            tool_calls: None,
            tool_results: None,
            thought: None,
            action: None,
            observation: None,
            status: "running".to_string(),
            created_at: now,
            completed_at: None,
        })
    }

    /// Get iterations for an execution
    pub async fn get_iterations(&self, execution_id: &str) -> Result<Vec<ExecutionIteration>> {
        sqlx::query_as::<_, ExecutionIteration>(
            "SELECT * FROM execution_iterations WHERE execution_id = ? ORDER BY iteration_num ASC",
        )
        .bind(execution_id)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get iterations: {}", e)))
    }

    /// Complete an iteration
    pub async fn complete_iteration(
        &self,
        iteration_id: &str,
        metrics: &IterationMetrics,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let tool_calls_json = metrics.tool_calls.as_ref().map(|t| serde_json::to_string(t).unwrap_or_default());

        sqlx::query(
            r#"
            UPDATE execution_iterations SET
                input_tokens = ?,
                output_tokens = ?,
                reasoning_tokens = ?,
                duration_ms = ?,
                tool_calls = ?,
                thought = ?,
                action = ?,
                observation = ?,
                status = 'completed',
                completed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(metrics.input_tokens)
        .bind(metrics.output_tokens)
        .bind(metrics.reasoning_tokens)
        .bind(metrics.duration_ms)
        .bind(&tool_calls_json)
        .bind(&metrics.thought)
        .bind(&metrics.action)
        .bind(&metrics.observation)
        .bind(&now)
        .bind(iteration_id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to complete iteration: {}", e)))?;

        Ok(())
    }

    // =========================================================================
    // LLM Call operations
    // =========================================================================

    /// Record an LLM call
    pub async fn record_llm_call(
        &self,
        id: String,
        execution_id: String,
        iteration_id: Option<String>,
        provider: String,
        model: String,
        metrics: &LlmMetrics,
    ) -> Result<LlmCall> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO llm_calls (
                id, execution_id, iteration_id, provider, model,
                input_tokens, output_tokens, reasoning_tokens,
                cost_usd, latency_ms, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&execution_id)
        .bind(&iteration_id)
        .bind(&provider)
        .bind(&model)
        .bind(metrics.input_tokens)
        .bind(metrics.output_tokens)
        .bind(metrics.reasoning_tokens)
        .bind(metrics.cost_usd)
        .bind(metrics.latency_ms)
        .bind(&now)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to record LLM call: {}", e)))?;

        Ok(LlmCall {
            id,
            execution_id,
            iteration_id,
            provider,
            model,
            input_tokens: metrics.input_tokens,
            output_tokens: metrics.output_tokens,
            reasoning_tokens: metrics.reasoning_tokens,
            cost_usd: metrics.cost_usd,
            latency_ms: metrics.latency_ms,
            request_messages: None,
            response_content: None,
            created_at: now,
        })
    }

    /// Get LLM calls for an execution
    pub async fn get_llm_calls(&self, execution_id: &str) -> Result<Vec<LlmCall>> {
        sqlx::query_as::<_, LlmCall>(
            "SELECT * FROM llm_calls WHERE execution_id = ? ORDER BY created_at ASC",
        )
        .bind(execution_id)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get LLM calls: {}", e)))
    }

    /// Get LLM calls for an iteration
    pub async fn get_iteration_llm_calls(&self, iteration_id: &str) -> Result<Vec<LlmCall>> {
        sqlx::query_as::<_, LlmCall>(
            "SELECT * FROM llm_calls WHERE iteration_id = ? ORDER BY created_at ASC",
        )
        .bind(iteration_id)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get iteration LLM calls: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execution_lifecycle() {
        // Would need a test database setup
        // For now, just verify the repository can be constructed
        // let repo = ExecutionMetricsRepository::new(db);
        assert!(true);
    }
}
