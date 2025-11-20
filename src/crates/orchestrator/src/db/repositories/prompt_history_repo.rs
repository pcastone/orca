//! Prompt history repository for database operations

use crate::db::connection::DatabasePool;
use crate::db::models::PromptHistory;
use chrono::Utc;

/// Prompt history repository for managing LLM interaction records
pub struct PromptHistoryRepository;

impl PromptHistoryRepository {
    /// Create a new prompt history entry
    pub async fn create(
        pool: &DatabasePool,
        id: String,
        provider: String,
        model: String,
        user_prompt: String,
        system_prompt: Option<String>,
        assistant_response: Option<String>,
        task_id: Option<String>,
        workflow_id: Option<String>,
        execution_id: Option<String>,
        session_id: Option<String>,
        input_tokens: Option<i32>,
        output_tokens: Option<i32>,
        cost_usd: Option<f64>,
        latency_ms: Option<i32>,
    ) -> Result<PromptHistory, sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        let total_tokens = match (input_tokens, output_tokens) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        };

        sqlx::query_as::<_, PromptHistory>(
            "INSERT INTO prompt_history (id, provider, model, user_prompt, system_prompt, assistant_response,
             task_id, workflow_id, execution_id, session_id, input_tokens, output_tokens, total_tokens,
             cost_usd, latency_ms, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'completed', ?)
             RETURNING *"
        )
        .bind(&id)
        .bind(&provider)
        .bind(&model)
        .bind(&user_prompt)
        .bind(&system_prompt)
        .bind(&assistant_response)
        .bind(&task_id)
        .bind(&workflow_id)
        .bind(&execution_id)
        .bind(&session_id)
        .bind(&input_tokens)
        .bind(&output_tokens)
        .bind(&total_tokens)
        .bind(&cost_usd)
        .bind(&latency_ms)
        .bind(&now)
        .fetch_one(pool)
        .await
    }

    /// Get a prompt history entry by ID
    pub async fn get_by_id(pool: &DatabasePool, id: &str) -> Result<Option<PromptHistory>, sqlx::Error> {
        sqlx::query_as::<_, PromptHistory>("SELECT * FROM prompt_history WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// List all prompt history entries
    pub async fn list(pool: &DatabasePool) -> Result<Vec<PromptHistory>, sqlx::Error> {
        sqlx::query_as::<_, PromptHistory>("SELECT * FROM prompt_history ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    }

    /// List prompt history by task
    pub async fn list_by_task(pool: &DatabasePool, task_id: &str) -> Result<Vec<PromptHistory>, sqlx::Error> {
        sqlx::query_as::<_, PromptHistory>(
            "SELECT * FROM prompt_history WHERE task_id = ? ORDER BY created_at DESC"
        )
        .bind(task_id)
        .fetch_all(pool)
        .await
    }

    /// List prompt history by session
    pub async fn list_by_session(pool: &DatabasePool, session_id: &str) -> Result<Vec<PromptHistory>, sqlx::Error> {
        sqlx::query_as::<_, PromptHistory>(
            "SELECT * FROM prompt_history WHERE session_id = ? ORDER BY created_at ASC"
        )
        .bind(session_id)
        .fetch_all(pool)
        .await
    }

    /// List prompt history by execution
    pub async fn list_by_execution(pool: &DatabasePool, execution_id: &str) -> Result<Vec<PromptHistory>, sqlx::Error> {
        sqlx::query_as::<_, PromptHistory>(
            "SELECT * FROM prompt_history WHERE execution_id = ? ORDER BY created_at ASC"
        )
        .bind(execution_id)
        .fetch_all(pool)
        .await
    }

    /// List prompt history by provider
    pub async fn list_by_provider(pool: &DatabasePool, provider: &str) -> Result<Vec<PromptHistory>, sqlx::Error> {
        sqlx::query_as::<_, PromptHistory>(
            "SELECT * FROM prompt_history WHERE provider = ? ORDER BY created_at DESC"
        )
        .bind(provider)
        .fetch_all(pool)
        .await
    }

    /// List prompt history by model
    pub async fn list_by_model(pool: &DatabasePool, model: &str) -> Result<Vec<PromptHistory>, sqlx::Error> {
        sqlx::query_as::<_, PromptHistory>(
            "SELECT * FROM prompt_history WHERE model = ? ORDER BY created_at DESC"
        )
        .bind(model)
        .fetch_all(pool)
        .await
    }

    /// Delete a prompt history entry
    pub async fn delete(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM prompt_history WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Get total token usage
    pub async fn get_total_tokens(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(total_tokens), 0) FROM prompt_history"
        )
        .fetch_one(pool)
        .await?;
        Ok(result.0)
    }

    /// Get total cost
    pub async fn get_total_cost(pool: &DatabasePool) -> Result<f64, sqlx::Error> {
        let result: (f64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM prompt_history"
        )
        .fetch_one(pool)
        .await?;
        Ok(result.0)
    }

    /// Count prompt history entries
    pub async fn count(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM prompt_history")
            .fetch_one(pool)
            .await?;
        Ok(result.0)
    }
}
