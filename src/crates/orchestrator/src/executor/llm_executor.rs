//! LLM-based Task Executor
//!
//! This module implements task execution using Large Language Models.
//! Tasks are converted to prompts, sent to an LLM, and the responses
//! are parsed back into task results.

use crate::executor::config::ExecutorConfig;
use crate::executor::parser::{ParsedResult, ResponseParser};
use crate::executor::retry::retry_with_backoff;
use crate::executor::streaming::{StreamBuilder, TaskUpdateSender};
use crate::{Result, Task, TaskExecutor, TaskStatus};
use async_trait::async_trait;
use futures::StreamExt;
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::messages::{Message, MessageRole};
use std::sync::Arc;
use tracing::{debug, info};

/// LLM-based task executor
///
/// Executes tasks by converting them into prompts for an LLM and
/// parsing the responses back into task results.
pub struct LlmTaskExecutor {
    /// The LLM client to use for task execution
    chat_model: Arc<dyn ChatModel>,

    /// Executor configuration
    config: ExecutorConfig,

    /// System prompt for task execution (overrides config if set)
    system_prompt: Option<String>,

    /// Response parser for LLM output
    parser: ResponseParser,
}

impl LlmTaskExecutor {
    /// Create a new LLM task executor with default configuration
    ///
    /// # Arguments
    /// * `chat_model` - The LLM client to use
    ///
    /// # Returns
    /// A new LlmTaskExecutor with default settings
    pub fn new(chat_model: Arc<dyn ChatModel>) -> Self {
        Self {
            chat_model,
            config: ExecutorConfig::default(),
            system_prompt: None,
            parser: ResponseParser::new(),
        }
    }

    /// Create a new LLM task executor with custom configuration
    ///
    /// # Arguments
    /// * `chat_model` - The LLM client to use
    /// * `config` - Executor configuration
    pub fn with_config(chat_model: Arc<dyn ChatModel>, config: ExecutorConfig) -> Self {
        Self {
            chat_model,
            config,
            system_prompt: None,
            parser: ResponseParser::new(),
        }
    }

    /// Set executor configuration
    pub fn set_config(mut self, config: ExecutorConfig) -> Self {
        self.config = config;
        self
    }

    /// Set custom response parser
    pub fn with_parser(mut self, parser: ResponseParser) -> Self {
        self.parser = parser;
        self
    }

    /// Set custom system prompt (overrides config)
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set temperature (convenience method)
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.config.temperature = temperature.clamp(0.0, 1.0);
        self
    }

    /// Set max tokens (convenience method)
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.config.max_tokens = Some(max_tokens);
        self
    }

    /// Set max retries (convenience method)
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.config.retry.max_retries = retries;
        self
    }

    /// Enable or disable streaming
    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.config.streaming = enabled;
        self
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }

    /// Default system prompt for task execution
    fn default_system_prompt() -> String {
        r#"You are a task execution assistant. Your role is to execute tasks based on their descriptions.

For each task:
1. Analyze the task name and description
2. Determine what needs to be done
3. Execute the task to the best of your ability
4. Return a JSON result with the following structure:

```json
{
  "status": "completed" | "failed" | "needs_input",
  "result": "The result of the task execution",
  "error": "Error message if failed (optional)",
  "needs_input": "Description of what input is needed (optional)"
}
```

Be concise and focus on completing the task effectively."#
            .to_string()
    }

    /// Convert a task into an LLM prompt
    fn task_to_prompt(&self, task: &Task) -> String {
        let mut prompt = format!("Task: {}\n\n", task.name);

        if let Some(desc) = &task.description {
            prompt.push_str(&format!("Description: {}\n\n", desc));
        }

        if !task.metadata.is_empty() {
            prompt.push_str("Metadata:\n");
            for (key, value) in &task.metadata {
                prompt.push_str(&format!("  - {}: {}\n", key, value));
            }
            prompt.push('\n');
        }

        prompt.push_str("Please execute this task and provide the result in JSON format.");

        prompt
    }

    /// Parse LLM response into structured result
    ///
    /// Uses the ResponseParser to handle various formats and extract task status
    fn parse_response(&self, response: &str) -> Result<ParsedResult> {
        self.parser.parse(response)
    }

    /// Parse LLM response and return just the status (for backward compatibility)
    fn parse_response_status(&self, response: &str) -> Result<TaskStatus> {
        let parsed = self.parse_response(response)?;
        Ok(parsed.status)
    }

    /// Execute task with retry logic
    async fn execute_with_retry(&self, task: &Task) -> Result<String> {
        let prompt = self.task_to_prompt(task);
        let operation_name = format!("task_execution_{}", task.name);

        retry_with_backoff(&self.config.retry, &operation_name, || async {
            self.execute_llm_request(&prompt).await
        })
        .await
    }

    /// Execute a single LLM request
    async fn execute_llm_request(&self, prompt: &str) -> Result<String> {
        let system_prompt = self
            .system_prompt
            .as_ref()
            .or(self.config.system_prompt.as_ref())
            .map(|s| s.clone())
            .unwrap_or_else(Self::default_system_prompt);

        let messages = vec![
            Message::new(MessageRole::System, system_prompt),
            Message::new(MessageRole::Human, prompt),
        ];

        let mut request = ChatRequest::new(messages)
            .with_temperature(self.config.temperature);

        if let Some(max_tokens) = self.config.max_tokens {
            request = request.with_max_tokens(max_tokens);
        }

        let response = self
            .chat_model
            .chat(request)
            .await
            .map_err(|e| crate::OrchestratorError::General(format!("LLM request failed: {}", e)))?;

        // Extract text from response
        let text = response
            .message
            .text()
            .ok_or_else(|| crate::OrchestratorError::General("No text content in LLM response".to_string()))?;

        Ok(text.to_string())
    }

    /// Execute task with streaming support
    ///
    /// This method executes a task and streams incremental updates back to the caller.
    /// It's useful for providing real-time feedback during long-running tasks.
    ///
    /// # Arguments
    /// * `task` - The task to execute
    /// * `sender` - Update sender for streaming progress
    ///
    /// # Returns
    /// The LLM response text
    pub async fn execute_streaming(&self, task: &Task, sender: &mut TaskUpdateSender) -> Result<String> {
        let system_prompt = self
            .system_prompt
            .as_ref()
            .or(self.config.system_prompt.as_ref())
            .map(|s| s.clone())
            .unwrap_or_else(Self::default_system_prompt);

        let messages = vec![
            Message::new(MessageRole::System, system_prompt),
            Message::new(MessageRole::Human, self.task_to_prompt(task)),
        ];

        let mut request = ChatRequest::new(messages)
            .with_temperature(self.config.temperature);

        if let Some(max_tokens) = self.config.max_tokens {
            request = request.with_max_tokens(max_tokens);
        }

        // Send started update
        sender.started().await?;

        // Get streaming response
        let stream_response = self
            .chat_model
            .stream(request)
            .await
            .map_err(|e| crate::OrchestratorError::General(format!("LLM stream failed: {}", e)))?;

        let mut content = String::new();
        let mut stream = stream_response.stream;

        // Stream tokens
        while let Some(chunk) = stream.next().await {
            content.push_str(&chunk.content);

            // Send token update if sender is active
            if sender.is_active() {
                let _ = sender.token(&chunk.content).await;
            }

            // If this is the final chunk, we can estimate progress
            if chunk.is_final {
                let _ = sender.progress(90).await;
            }
        }

        // Send completion
        sender.completed().await?;

        Ok(content)
    }

    /// Create a streaming builder for this executor
    pub fn stream_builder(&self, task: &Task) -> StreamBuilder {
        StreamBuilder::new(task.id)
            .buffer_size(100)
            .include_tokens(true)
            .include_progress(true)
    }
}

#[async_trait]
impl TaskExecutor for LlmTaskExecutor {
    async fn execute(&self, task: &Task) -> Result<()> {
        info!("Executing task via LLM: {}", task.name);

        // Execute the task with retry logic
        let response = self.execute_with_retry(task).await?;

        // Parse the response to determine task status and result
        let parsed = self.parse_response(&response)?;

        debug!(
            "Task execution completed with status: {:?}, result: {:?}",
            parsed.status, parsed.result
        );

        // Note: In a full implementation, we would update the task's status
        // and store the result. For now, we just verify we can parse it.

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_response_with_code_block() {
        #[derive(Clone)]
        struct MockChatModel;
        #[async_trait]
        impl ChatModel for MockChatModel {
            async fn chat(&self, _request: ChatRequest) -> langgraph_core::error::Result<langgraph_core::llm::ChatResponse> {
                unimplemented!()
            }

            async fn stream(&self, _request: ChatRequest) -> langgraph_core::error::Result<langgraph_core::llm::ChatStreamResponse> {
                unimplemented!()
            }

            fn clone_box(&self) -> Box<dyn ChatModel> {
                Box::new(self.clone())
            }
        }

        let executor = LlmTaskExecutor::new(Arc::new(MockChatModel));

        let text = r#"Here's the result:
```json
{"status": "completed", "result": "Done"}
```
That's it!"#;

        let parsed = executor.parse_response(text).unwrap();
        assert_eq!(parsed.status, TaskStatus::Completed);
        assert_eq!(parsed.result, Some("Done".to_string()));
    }

    #[test]
    fn test_parse_response_raw_json() {
        #[derive(Clone)]
        struct MockChatModel;
        #[async_trait]
        impl ChatModel for MockChatModel {
            async fn chat(&self, _request: ChatRequest) -> langgraph_core::error::Result<langgraph_core::llm::ChatResponse> {
                unimplemented!()
            }

            async fn stream(&self, _request: ChatRequest) -> langgraph_core::error::Result<langgraph_core::llm::ChatStreamResponse> {
                unimplemented!()
            }

            fn clone_box(&self) -> Box<dyn ChatModel> {
                Box::new(self.clone())
            }
        }

        let executor = LlmTaskExecutor::new(Arc::new(MockChatModel));

        let text = r#"The result is {"status": "completed", "result": "Success"} as you can see."#;

        let parsed = executor.parse_response(text).unwrap();
        assert_eq!(parsed.status, TaskStatus::Completed);
        assert_eq!(parsed.result, Some("Success".to_string()));
    }

    #[test]
    fn test_task_to_prompt() {
        // Create a mock LLM (we'll use a dummy Arc, won't be called in this test)
        #[derive(Clone)]
        struct MockChatModel;
        #[async_trait]
        impl ChatModel for MockChatModel {
            async fn chat(&self, _request: ChatRequest) -> langgraph_core::error::Result<langgraph_core::llm::ChatResponse> {
                unimplemented!()
            }

            async fn stream(&self, _request: ChatRequest) -> langgraph_core::error::Result<langgraph_core::llm::ChatStreamResponse> {
                unimplemented!()
            }

            fn clone_box(&self) -> Box<dyn ChatModel> {
                Box::new(self.clone())
            }
        }

        let executor = LlmTaskExecutor::new(Arc::new(MockChatModel));

        let task = Task::new("Test Task")
            .with_description("This is a test")
            .with_metadata("priority", "high");

        let prompt = executor.task_to_prompt(&task);

        assert!(prompt.contains("Test Task"));
        assert!(prompt.contains("This is a test"));
        assert!(prompt.contains("high"));
    }

    #[test]
    fn test_default_system_prompt() {
        let prompt = LlmTaskExecutor::default_system_prompt();
        assert!(prompt.contains("task execution"));
        assert!(prompt.contains("JSON"));
    }
}
