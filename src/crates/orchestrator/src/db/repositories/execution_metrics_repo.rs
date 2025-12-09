//! Execution metrics repository for database operations
//!
//! Manages prompt_executions, execution_iterations, and related queries.

use crate::db::connection::DatabasePool;
use crate::db::models::{PromptExecution, ExecutionIteration, AggregatedMetrics, IterationMetrics, PromptHistory};
use chrono::Utc;

/// Execution metrics repository for managing ReAct execution tracking
pub struct ExecutionMetricsRepository;

impl ExecutionMetricsRepository {
    // ============================================================
    // PromptExecution methods
    // ============================================================

    /// Create a new prompt execution
    pub async fn create_execution(
        pool: &DatabasePool,
        id: String,
        prompt: String,
        project_name: Option<String>,
        agent_type: String,
        session_id: Option<String>,
        task_id: Option<String>,
    ) -> Result<PromptExecution, sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        sqlx::query_as::<_, PromptExecution>(
            "INSERT INTO prompt_executions (id, original_prompt, project_name, agent_type, session_id, task_id, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             RETURNING *"
        )
        .bind(&id)
        .bind(&prompt)
        .bind(&project_name)
        .bind(&agent_type)
        .bind(&session_id)
        .bind(&task_id)
        .bind(&now)
        .fetch_one(pool)
        .await
    }

    /// Complete a prompt execution with final metrics
    pub async fn complete_execution(
        pool: &DatabasePool,
        id: &str,
        response: &str,
        metrics: &AggregatedMetrics,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE prompt_executions SET
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
             WHERE id = ?"
        )
        .bind(response)
        .bind(metrics.total_input_tokens)
        .bind(metrics.total_output_tokens)
        .bind(metrics.total_reasoning_tokens)
        .bind(metrics.total_cost_usd)
        .bind(metrics.total_duration_ms)
        .bind(metrics.iteration_count)
        .bind(metrics.llm_call_count)
        .bind(metrics.tool_call_count)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Fail a prompt execution
    pub async fn fail_execution(
        pool: &DatabasePool,
        id: &str,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE prompt_executions SET status = 'failed', error_message = ?, completed_at = ? WHERE id = ?"
        )
        .bind(error)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get a prompt execution by ID
    pub async fn get_execution(
        pool: &DatabasePool,
        id: &str,
    ) -> Result<Option<PromptExecution>, sqlx::Error> {
        sqlx::query_as::<_, PromptExecution>("SELECT * FROM prompt_executions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// List prompt executions with pagination
    pub async fn list_executions(
        pool: &DatabasePool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PromptExecution>, sqlx::Error> {
        sqlx::query_as::<_, PromptExecution>(
            "SELECT * FROM prompt_executions ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }

    /// List prompt executions by project
    pub async fn list_by_project(
        pool: &DatabasePool,
        project_name: &str,
    ) -> Result<Vec<PromptExecution>, sqlx::Error> {
        sqlx::query_as::<_, PromptExecution>(
            "SELECT * FROM prompt_executions WHERE project_name = ? ORDER BY created_at DESC"
        )
        .bind(project_name)
        .fetch_all(pool)
        .await
    }

    /// List prompt executions by session
    pub async fn list_by_session(
        pool: &DatabasePool,
        session_id: &str,
    ) -> Result<Vec<PromptExecution>, sqlx::Error> {
        sqlx::query_as::<_, PromptExecution>(
            "SELECT * FROM prompt_executions WHERE session_id = ? ORDER BY created_at ASC"
        )
        .bind(session_id)
        .fetch_all(pool)
        .await
    }

    /// Update aggregated metrics for an execution (incremental update)
    pub async fn update_metrics(
        pool: &DatabasePool,
        id: &str,
        input_tokens: i64,
        output_tokens: i64,
        reasoning_tokens: i64,
        cost_usd: f64,
        duration_ms: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE prompt_executions SET
             total_input_tokens = total_input_tokens + ?,
             total_output_tokens = total_output_tokens + ?,
             total_reasoning_tokens = total_reasoning_tokens + ?,
             total_cost_usd = total_cost_usd + ?,
             total_duration_ms = total_duration_ms + ?,
             llm_call_count = llm_call_count + 1
             WHERE id = ?"
        )
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(reasoning_tokens)
        .bind(cost_usd)
        .bind(duration_ms)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Increment iteration count
    pub async fn increment_iteration_count(
        pool: &DatabasePool,
        id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE prompt_executions SET iteration_count = iteration_count + 1 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Increment tool call count
    pub async fn increment_tool_count(
        pool: &DatabasePool,
        id: &str,
        count: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE prompt_executions SET tool_call_count = tool_call_count + ? WHERE id = ?")
            .bind(count)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // ============================================================
    // ExecutionIteration methods
    // ============================================================

    /// Create a new execution iteration
    pub async fn create_iteration(
        pool: &DatabasePool,
        id: String,
        execution_id: String,
        iteration_num: i64,
    ) -> Result<ExecutionIteration, sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        sqlx::query_as::<_, ExecutionIteration>(
            "INSERT INTO execution_iterations (id, execution_id, iteration_num, created_at)
             VALUES (?, ?, ?, ?)
             RETURNING *"
        )
        .bind(&id)
        .bind(&execution_id)
        .bind(iteration_num)
        .bind(&now)
        .fetch_one(pool)
        .await
    }

    /// Complete an execution iteration with metrics
    pub async fn complete_iteration(
        pool: &DatabasePool,
        id: &str,
        metrics: &IterationMetrics,
    ) -> Result<(), sqlx::Error> {
        let tool_calls_json = metrics.tool_calls.as_ref().map(|tc| serde_json::to_string(tc).unwrap_or_default());
        let tool_results_json = metrics.tool_results.as_ref().map(|tr| serde_json::to_string(tr).unwrap_or_default());

        sqlx::query(
            "UPDATE execution_iterations SET
             input_tokens = ?,
             output_tokens = ?,
             reasoning_tokens = ?,
             duration_ms = ?,
             agent_action = ?,
             tool_calls_json = ?,
             tool_results_json = ?
             WHERE id = ?"
        )
        .bind(metrics.input_tokens)
        .bind(metrics.output_tokens)
        .bind(metrics.reasoning_tokens)
        .bind(metrics.duration_ms)
        .bind(&metrics.agent_action)
        .bind(&tool_calls_json)
        .bind(&tool_results_json)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get all iterations for an execution
    pub async fn get_iterations(
        pool: &DatabasePool,
        execution_id: &str,
    ) -> Result<Vec<ExecutionIteration>, sqlx::Error> {
        sqlx::query_as::<_, ExecutionIteration>(
            "SELECT * FROM execution_iterations WHERE execution_id = ? ORDER BY iteration_num ASC"
        )
        .bind(execution_id)
        .fetch_all(pool)
        .await
    }

    /// Get a specific iteration
    pub async fn get_iteration(
        pool: &DatabasePool,
        id: &str,
    ) -> Result<Option<ExecutionIteration>, sqlx::Error> {
        sqlx::query_as::<_, ExecutionIteration>("SELECT * FROM execution_iterations WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    // ============================================================
    // Cross-table queries
    // ============================================================

    /// Get LLM calls (prompt_history) for an execution
    pub async fn get_llm_calls(
        pool: &DatabasePool,
        execution_id: &str,
    ) -> Result<Vec<PromptHistory>, sqlx::Error> {
        sqlx::query_as::<_, PromptHistory>(
            "SELECT * FROM prompt_history WHERE prompt_execution_id = ? ORDER BY created_at ASC"
        )
        .bind(execution_id)
        .fetch_all(pool)
        .await
    }

    /// Get LLM calls for a specific iteration
    pub async fn get_iteration_llm_calls(
        pool: &DatabasePool,
        iteration_id: &str,
    ) -> Result<Vec<PromptHistory>, sqlx::Error> {
        sqlx::query_as::<_, PromptHistory>(
            "SELECT * FROM prompt_history WHERE iteration_id = ? ORDER BY created_at ASC"
        )
        .bind(iteration_id)
        .fetch_all(pool)
        .await
    }

    /// Count executions
    pub async fn count_executions(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prompt_executions")
            .fetch_one(pool)
            .await?;
        Ok(result.0)
    }

    /// Get total tokens across all executions
    pub async fn get_total_tokens(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(total_input_tokens + total_output_tokens), 0) FROM prompt_executions"
        )
        .fetch_one(pool)
        .await?;
        Ok(result.0)
    }

    /// Get total cost across all executions
    pub async fn get_total_cost(pool: &DatabasePool) -> Result<f64, sqlx::Error> {
        let result: (f64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(total_cost_usd), 0.0) FROM prompt_executions"
        )
        .fetch_one(pool)
        .await?;
        Ok(result.0)
    }

    /// Delete an execution and all related data (cascade)
    pub async fn delete_execution(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM prompt_executions WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
