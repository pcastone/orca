//! LLM-based Workflow Executor
//!
//! Executes workflow steps using an LLM (Execution LLM) with step context
//! and history for intelligent step execution.

use crate::executor::llm_executor::LlmTaskExecutor;
use crate::{OrchestratorError, Result, Task, TaskExecutor};
use langgraph_core::llm::ChatModel;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// LLM-based workflow executor
pub struct LlmWorkflowExecutor {
    /// LLM task executor
    task_executor: LlmTaskExecutor,
    /// Maximum step retries
    max_retries: u32,
}

impl LlmWorkflowExecutor {
    /// Create a new LLM workflow executor
    ///
    /// # Arguments
    /// * `chat_model` - The LLM client to use for execution
    pub fn new(chat_model: Arc<dyn ChatModel>) -> Self {
        Self {
            task_executor: LlmTaskExecutor::new(chat_model),
            max_retries: 3,
        }
    }

    /// Set maximum retries for failed steps
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Execute a workflow step using LLM
    ///
    /// # Arguments
    /// * `step` - The workflow step to execute (name, description, dependencies)
    /// * `context` - Step context (previous results, parameters)
    /// * `history` - Execution history for context
    ///
    /// # Returns
    /// * Result of step execution
    pub async fn execute_step(
        &self,
        step: &WorkflowStepInfo,
        context: &serde_json::Value,
        history: &[String],
    ) -> Result<serde_json::Value> {
        info!("Executing workflow step: {}", step.name);
        
        // Build step prompt with context and history
        let prompt = self.build_step_prompt(step, context, history);

        // Create task for step execution
        let task = Task::new(&step.name).with_description(&prompt);

        // Execute with retries
        let mut attempt = 0;
        loop {
            attempt += 1;
            debug!("Step execution attempt {}/{}", attempt, self.max_retries + 1);

            // Use TaskExecutor trait method
            match <LlmTaskExecutor as TaskExecutor>::execute(&self.task_executor, &task).await {
                Ok(_) => {
                    info!("Step '{}' completed successfully", step.name);
                    // Return mock result for now
                    return Ok(serde_json::json!({
                        "status": "completed",
                        "step": step.name,
                    }));
                }
                Err(e) if attempt <= self.max_retries => {
                    warn!("Step '{}' failed (attempt {}): {}", step.name, attempt, e);
                    continue;
                }
                Err(e) => {
                    return Err(OrchestratorError::ExecutionFailed(format!(
                        "Step '{}' failed after {} attempts: {}",
                        step.name,
                        attempt,
                        e
                    )));
                }
            }
        }
    }

    /// Build prompt for step execution
    fn build_step_prompt(
        &self,
        step: &WorkflowStepInfo,
        context: &serde_json::Value,
        history: &[String],
    ) -> String {
        let mut prompt = format!("Execute workflow step: {}\n\n", step.name);

        if let Some(desc) = &step.description {
            prompt.push_str(&format!("Description: {}\n\n", desc));
        }

        // Add context
        if !context.is_null() && context != &serde_json::json!({}) {
            prompt.push_str("Context:\n");
            prompt.push_str(&serde_json::to_string_pretty(context).unwrap_or_default());
            prompt.push_str("\n\n");
        }

        // Add execution history
        if !history.is_empty() {
            prompt.push_str("Previous steps:\n");
            for (i, entry) in history.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, entry));
            }
            prompt.push_str("\n");
        }

        prompt.push_str("Execute this step and provide the result.");
        prompt
    }

    /// Execute an entire workflow
    pub async fn execute_workflow(
        &self,
        steps: &[WorkflowStepInfo],
        initial_context: serde_json::Value,
    ) -> Result<WorkflowExecutionResult> {
        let mut history = Vec::new();
        let mut context = initial_context;

        for (i, step) in steps.iter().enumerate() {
            debug!("Executing step {}/{}: {}", i + 1, steps.len(), step.name);

            match self.execute_step(step, &context, &history).await {
                Ok(result) => {
                    history.push(format!("{}: Success", step.name));
                    context = result; // Use result as context for next step
                }
                Err(e) => {
                    history.push(format!("{}: Failed - {}", step.name, e));
                    return Err(e);
                }
            }
        }

        Ok(WorkflowExecutionResult {
            completed_steps: steps.len(),
            total_steps: steps.len(),
            status: "completed".to_string(),
            final_result: context,
        })
    }
}

/// Workflow step information
#[derive(Debug, Clone)]
pub struct WorkflowStepInfo {
    /// Step name
    pub name: String,
    /// Step description
    pub description: Option<String>,
    /// Dependencies (names of steps that must complete first)
    pub dependencies: Vec<String>,
}

/// Workflow execution result
#[derive(Debug, Clone)]
pub struct WorkflowExecutionResult {
    /// Number of completed steps
    pub completed_steps: usize,
    /// Total number of steps
    pub total_steps: usize,
    /// Execution status
    pub status: String,
    /// Final execution result
    pub final_result: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use langgraph_core::error::Result as GraphResult;
    use langgraph_core::llm::{self, ChatRequest, ChatResponse};
    use langgraph_core::messages::Message;

    // Mock ChatModel for testing
    #[derive(Clone)]
    struct MockChatModel;

    #[async_trait]
    impl ChatModel for MockChatModel {
        async fn chat(&self, _request: ChatRequest) -> GraphResult<ChatResponse> {
            Ok(ChatResponse {
                message: Message::ai(r#"{"status": "completed", "result": "success"}"#),
                reasoning: None,
                usage: None,
                metadata: Default::default(),
            })
        }

        async fn stream(&self, _request: ChatRequest) -> GraphResult<llm::ChatStreamResponse> {
            unimplemented!()
        }

        fn clone_box(&self) -> Box<dyn ChatModel> {
            Box::new(self.clone())
        }
    }

    #[tokio::test]
    async fn test_llm_workflow_executor_creation() {
        let executor = LlmWorkflowExecutor::new(Arc::new(MockChatModel));
        assert_eq!(executor.max_retries, 3);
    }

    #[tokio::test]
    async fn test_build_step_prompt() {
        let executor = LlmWorkflowExecutor::new(Arc::new(MockChatModel));
        
        let step = WorkflowStepInfo {
            name: "Test Step".to_string(),
            description: Some("Test description".to_string()),
            dependencies: vec![],
        };

        let context = serde_json::json!({"key": "value"});
        let history = vec!["Previous step: Success".to_string()];

        let prompt = executor.build_step_prompt(&step, &context, &history);

        assert!(prompt.contains("Test Step"));
        assert!(prompt.contains("Test description"));
        assert!(prompt.contains("Previous step"));
    }

    #[tokio::test]
    async fn test_execute_step() {
        let executor = LlmWorkflowExecutor::new(Arc::new(MockChatModel));
        
        let step = WorkflowStepInfo {
            name: "Test Step".to_string(),
            description: None,
            dependencies: vec![],
        };

        let context = serde_json::json!({});
        let history = vec![];

        let result = executor.execute_step(&step, &context, &history).await;
        assert!(result.is_ok());
    }
}

