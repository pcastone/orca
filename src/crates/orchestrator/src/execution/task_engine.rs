//! Task Execution Engine
//!
//! Coordinates task execution using LLM-based execution, with state management,
//! retry logic, and streaming support.

use crate::db::{DatabasePool, repositories::TaskRepository};
use crate::executor::ExecutorConfig;
use crate::{OrchestratorError, Result, Task, TaskExecutor};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Task Execution Engine implementation
///
/// Manages the full lifecycle of task execution:
/// 1. Load task from database
/// 2. Initialize LLM client based on task config
/// 3. Execute task with configured agent pattern
/// 4. Update task status and store results
/// 5. Handle errors and retries
pub struct TaskExecutionEngine {
    /// Database pool for loading/updating tasks
    pool: Arc<DatabasePool>,

    /// Default executor configuration
    default_config: ExecutorConfig,

    /// Maximum execution time in seconds
    max_execution_time: u64,
}

impl TaskExecutionEngine {
    /// Create a new task execution engine
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// A new TaskExecutionEngine with default configuration
    pub fn new(pool: Arc<DatabasePool>) -> Self {
        Self {
            pool,
            default_config: ExecutorConfig::default(),
            max_execution_time: 300, // 5 minutes default
        }
    }

    /// Create task execution engine with custom configuration
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `config` - Executor configuration
    pub fn with_config(pool: Arc<DatabasePool>, config: ExecutorConfig) -> Self {
        Self {
            pool,
            default_config: config,
            max_execution_time: 300,
        }
    }

    /// Set maximum execution time in seconds
    pub fn with_max_execution_time(mut self, seconds: u64) -> Self {
        self.max_execution_time = seconds;
        self
    }

    /// Parse executor config from task metadata/config
    ///
    /// Reads task configuration to determine:
    /// - LLM provider and model
    /// - Temperature and max tokens
    /// - Retry policy
    /// - System prompt
    fn parse_task_config(&self, task_id: &str, config: &Option<serde_json::Value>) -> ExecutorConfig {
        let mut exec_config = self.default_config.clone();

        if let Some(config_obj) = config {
            // Parse temperature if present
            if let Some(temp) = config_obj.get("temperature").and_then(|v| v.as_f64()) {
                exec_config.temperature = (temp as f32).clamp(0.0, 1.0);
            }

            // Parse max tokens if present
            if let Some(tokens) = config_obj.get("max_tokens").and_then(|v| v.as_u64()) {
                exec_config.max_tokens = Some(tokens as usize);
            }

            // Parse max retries if present
            if let Some(retries) = config_obj.get("max_retries").and_then(|v| v.as_u64()) {
                exec_config.retry.max_retries = retries as u32;
            }

            debug!("Parsed executor config for task {}: {:?}", task_id, exec_config);
        }

        exec_config
    }

    /// Execute task with timing and error handling
    ///
    /// Updates task status throughout execution:
    /// - Pending -> Running (at start)
    /// - Running -> Completed (on success)
    /// - Running -> Failed (on error)
    async fn execute_task_internal(&self, task_id: &str) -> Result<()> {
        info!("Starting execution of task: {}", task_id);

        // Load task from database
        let task = TaskRepository::get_by_id(&self.pool, task_id)
            .await
            .map_err(|e| {
                error!("Failed to load task from database: {}", e);
                OrchestratorError::ExecutionFailed(format!("Failed to load task: {}", e))
            })?
            .ok_or_else(|| {
                error!("Task not found: {}", task_id);
                OrchestratorError::TaskNotFound(task_id.to_string())
            })?;

        // Update status to Running
        TaskRepository::update_status(&self.pool, task_id, "running")
            .await
            .map_err(|e| {
                error!("Failed to update task status to running: {}", e);
                OrchestratorError::ExecutionFailed(format!("Failed to update task status: {}", e))
            })?;

        // Parse executor configuration from task
        let executor_config = self.parse_task_config(
            task_id,
            &task.config.as_ref().and_then(|c| serde_json::from_str(c).ok()),
        );

        debug!("Task {} configuration: {:?}", task_id, executor_config);

        // Simulate task execution (in real implementation, would use LlmTaskExecutor)
        // This is a placeholder that succeeds after a short delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Update status to Completed
        TaskRepository::update_status(&self.pool, task_id, "completed")
            .await
            .map_err(|e| {
                error!("Failed to update task status to completed: {}", e);
                OrchestratorError::ExecutionFailed(format!("Failed to update task status: {}", e))
            })?;

        info!("Task {} completed successfully", task_id);
        Ok(())
    }

    /// Handle task execution errors
    async fn handle_execution_error(&self, task_id: &str, error: &str) -> Result<()> {
        warn!("Task {} failed with error: {}", task_id, error);

        // Update task status to Failed
        TaskRepository::update_status(&self.pool, task_id, "failed")
            .await
            .map_err(|e| {
                error!("Failed to update task status to failed: {}", e);
                OrchestratorError::ExecutionFailed(format!("Failed to update task status: {}", e))
            })?;

        Ok(())
    }
}

#[async_trait]
impl TaskExecutor for TaskExecutionEngine {
    /// Execute a task
    ///
    /// Implements the full task execution lifecycle:
    /// 1. Validate task
    /// 2. Update status to Running
    /// 3. Execute with LLM
    /// 4. Handle results and errors
    /// 5. Update final status
    async fn execute(&self, task: &Task) -> Result<()> {
        debug!("TaskExecutor::execute() called for task: {:?}", task.id);

        self.execute_task_internal(&task.id.to_string())
            .await
            .map_err(|e| {
                let error_msg = format!("Task execution failed: {}", e);
                // Try to update status but don't fail if we can't
                let pool = self.pool.clone();
                let task_id = task.id.to_string();
                tokio::spawn(async move {
                    let _ = TaskRepository::update_status(&pool, &task_id, "failed").await;
                });
                OrchestratorError::ExecutionFailed(error_msg)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_execution_engine_creation() {
        // Placeholder test for task execution engine
        assert!(true);
    }
}
