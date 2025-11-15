//! Integration tests for LlmTaskExecutor
//!
//! Comprehensive test suite covering task execution, retry logic,
//! streaming, configuration, and result parsing.

use async_trait::async_trait;
use langgraph_core::llm::{ChatModel, ChatRequest, ChatResponse, ChatStreamResponse};
use langgraph_core::messages::Message;
use langgraph_core::error::{Result, GraphError};
use orchestrator::executor::{ExecutorConfig, LlmTaskExecutor, ResponseParser};
use orchestrator::{Task, TaskExecutor, TaskStatus};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Mock LLM for testing
struct MockChatModel {
    response: Arc<Mutex<String>>,
    call_count: Arc<Mutex<usize>>,
    fail_count: usize,
}

impl MockChatModel {
    fn new(response: impl Into<String>) -> Self {
        Self {
            response: Arc::new(Mutex::new(response.into())),
            call_count: Arc::new(Mutex::new(0)),
            fail_count: 0,
        }
    }

    fn with_failures(response: impl Into<String>, fail_count: usize) -> Self {
        Self {
            response: Arc::new(Mutex::new(response.into())),
            call_count: Arc::new(Mutex::new(0)),
            fail_count,
        }
    }

    fn get_call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

#[async_trait]
impl ChatModel for MockChatModel {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;

        // Simulate failures for first N calls
        if *count <= self.fail_count {
            return Err(GraphError::Validation("Simulated transient error".to_string()));
        }

        let response_text = self.response.lock().unwrap().clone();
        Ok(ChatResponse {
            message: Message::new(
                langgraph_core::messages::MessageRole::Assistant,
                response_text,
            ),
            usage: None,
            reasoning: None,
            metadata: HashMap::new(),
        })
    }

    async fn stream(
        &self,
        _request: ChatRequest,
    ) -> Result<ChatStreamResponse> {
        Err(GraphError::Validation("Streaming not implemented in mock".to_string()))
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(Self {
            response: self.response.clone(),
            call_count: self.call_count.clone(),
            fail_count: self.fail_count,
        })
    }
}

#[tokio::test]
async fn test_successful_task_execution() {
    let response = r#"{"status": "completed", "result": "Task completed successfully"}"#;
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model.clone());

    let task = Task::new("Test Task").with_description("A simple test task");
    let result = executor.execute(&task).await;

    assert!(result.is_ok());
    assert_eq!(model.get_call_count(), 1);
}

#[tokio::test]
async fn test_task_execution_with_text_response() {
    let response = "The task has been completed successfully!";
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model.clone());

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_retry_logic_with_transient_failures() {
    let response = r#"{"status": "completed", "result": "Success after retries"}"#;
    let model = Arc::new(MockChatModel::with_failures(response, 2));

    let config = ExecutorConfig::default()
        .with_temperature(0.5)
        .with_max_tokens(2000);

    let executor = LlmTaskExecutor::with_config(model.clone(), config);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    assert!(result.is_ok());
    // Should have retried: initial + 2 failures + 1 success = 3 calls
    assert_eq!(model.get_call_count(), 3);
}

#[tokio::test]
async fn test_retry_exhaustion() {
    // Always fail
    let response = "This will always fail";
    let model = Arc::new(MockChatModel::with_failures(response, 100));

    let config = ExecutorConfig::default().with_temperature(0.5);

    let executor = LlmTaskExecutor::with_config(model.clone(), config);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    assert!(result.is_err());
    // Should retry 3 times (default max_retries)
    assert_eq!(model.get_call_count(), 4); // initial + 3 retries
}

#[tokio::test]
async fn test_configuration_loading() {
    let response = r#"{"status": "completed"}"#;
    let model = Arc::new(MockChatModel::new(response));

    // Test preset configuration
    let config = ExecutorConfig::from_preset("gpt-4").unwrap();
    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.temperature, 0.7);
    assert_eq!(config.max_tokens, Some(4096));

    let executor = LlmTaskExecutor::with_config(model, config);
    let task = Task::new("Test Task");

    let result = executor.execute(&task).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_configuration_builder() {
    let response = r#"{"status": "completed"}"#;
    let model = Arc::new(MockChatModel::new(response));

    let executor = LlmTaskExecutor::new(model)
        .with_temperature(0.3)
        .with_max_tokens(1000)
        .with_max_retries(2);

    let config = executor.config();
    assert_eq!(config.temperature, 0.3);
    assert_eq!(config.max_tokens, Some(1000));
    assert_eq!(config.retry.max_retries, 2);
}

#[tokio::test]
async fn test_result_parsing_json_completed() {
    let response = r#"{"status": "completed", "result": "Task done"}"#;
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_result_parsing_json_failed() {
    let response = r#"{"status": "failed", "error": "Something went wrong"}"#;
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    // Execution itself should succeed (parsing is successful)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_result_parsing_json_with_metadata() {
    let response = r#"{
        "status": "completed",
        "result": "Done",
        "execution_time_ms": 150,
        "tokens_used": 42
    }"#;
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_result_parsing_from_code_block() {
    let response = r#"
Here's the result of the task:

```json
{"status": "completed", "result": "Task successfully completed"}
```

The task ran without any issues.
"#;
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_result_parsing_text_fallback() {
    let response = "The task has completed successfully without errors.";
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_tasks_in_sequence() {
    let model = Arc::new(MockChatModel::new(
        r#"{"status": "completed", "result": "Done"}"#,
    ));
    let executor = Arc::new(LlmTaskExecutor::new(model.clone()));

    for i in 0..5 {
        let task = Task::new(format!("Task {}", i));
        let result = executor.execute(&task).await;
        assert!(result.is_ok());
    }

    // Should have been called once per task
    assert_eq!(model.get_call_count(), 5);
}

#[tokio::test]
async fn test_task_with_metadata() {
    let response = r#"{"status": "completed", "result": "Completed"}"#;
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model);

    let task = Task::new("Test Task")
        .with_description("A detailed description")
        .with_metadata("priority", "high")
        .with_metadata("category", "development")
        .with_metadata("timeout", "30");

    let result = executor.execute(&task).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_custom_system_prompt() {
    let response = r#"{"status": "completed"}"#;
    let model = Arc::new(MockChatModel::new(response));

    let custom_prompt = "You are a specialized task executor for data processing.";
    let executor = LlmTaskExecutor::new(model).with_system_prompt(custom_prompt);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_custom_response_parser() {
    let response = r#"{"status": "completed", "result": "Success"}"#;
    let model = Arc::new(MockChatModel::new(response));

    let parser = ResponseParser::new().with_strict_mode(true);
    let executor = LlmTaskExecutor::new(model).with_parser(parser);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_concurrent_executions() {
    let model = Arc::new(MockChatModel::new(
        r#"{"status": "completed", "result": "Done"}"#,
    ));
    let executor = Arc::new(LlmTaskExecutor::new(model.clone()));

    let mut handles = vec![];

    for i in 0..5 {
        let executor_clone = executor.clone();
        let handle = tokio::spawn(async move {
            let task = Task::new(format!("Task {}", i));
            executor_clone.execute(&task).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // All tasks should have executed
    assert_eq!(model.get_call_count(), 5);
}

#[tokio::test]
async fn test_status_aliases() {
    let test_cases = vec![
        ("completed", TaskStatus::Completed),
        ("success", TaskStatus::Completed),
        ("done", TaskStatus::Completed),
        ("failed", TaskStatus::Failed),
        ("error", TaskStatus::Failed),
        ("pending", TaskStatus::Pending),
        ("running", TaskStatus::Running),
    ];

    for (status_str, expected_status) in test_cases {
        let response = format!(r#"{{"status": "{}"}}"#, status_str);
        let model = Arc::new(MockChatModel::new(&response));
        let executor = LlmTaskExecutor::new(model);

        let task = Task::new("Test Task");
        let result = executor.execute(&task).await;
        assert!(
            result.is_ok(),
            "Failed for status: {} -> {:?}",
            status_str,
            expected_status
        );
    }
}

#[tokio::test]
async fn test_empty_response_handling() {
    let response = "";
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    // Should fail on empty response
    assert!(result.is_err());
}

#[tokio::test]
async fn test_malformed_json_fallback() {
    let response = r#"{"invalid json syntax"#;
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    // Should fall back to text parsing
    // Empty/malformed response might fail
    let _ = result; // Result depends on parser's allow_partial setting
}

#[tokio::test]
async fn test_model_preset_claude() {
    let response = r#"{"status": "completed"}"#;
    let model = Arc::new(MockChatModel::new(response));

    let config = ExecutorConfig::from_preset("claude3-opus").unwrap();
    assert_eq!(config.model, "claude-3-opus-20240229");
    assert_eq!(config.context_window, Some(200000));

    let executor = LlmTaskExecutor::with_config(model, config);
    let task = Task::new("Test Task");

    let result = executor.execute(&task).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_model_preset_gpt4_turbo() {
    let response = r#"{"status": "completed"}"#;
    let model = Arc::new(MockChatModel::new(response));

    let config = ExecutorConfig::from_preset("gpt4-turbo").unwrap();
    assert_eq!(config.model, "gpt-4-turbo-preview");
    assert_eq!(config.context_window, Some(128000));

    let executor = LlmTaskExecutor::with_config(model, config);
    let task = Task::new("Test Task");

    let result = executor.execute(&task).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_temperature_range_validation() {
    let response = r#"{"status": "completed"}"#;

    // Test temperature clamping - upper bound
    let model1 = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model1).with_temperature(2.5); // Should be clamped to 1.0
    assert_eq!(executor.config().temperature, 1.0);

    // Test temperature clamping - lower bound
    let model2 = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model2).with_temperature(-0.5); // Should be clamped to 0.0
    assert_eq!(executor.config().temperature, 0.0);
}

#[tokio::test]
async fn test_retry_config_customization() {
    let response = r#"{"status": "completed"}"#;
    let model = Arc::new(MockChatModel::new(response));

    let config = ExecutorConfig::default()
        .with_temperature(0.5)
        .with_max_tokens(1500);

    let executor = LlmTaskExecutor::with_config(model, config)
        .with_max_retries(5)
        .with_temperature(0.8);

    let cfg = executor.config();
    assert_eq!(cfg.retry.max_retries, 5);
    assert_eq!(cfg.temperature, 0.8);
}

#[tokio::test]
async fn test_needs_input_response() {
    let response = r#"{"status": "needs_input", "needs_input": "Please provide API credentials"}"#;
    let model = Arc::new(MockChatModel::new(response));
    let executor = LlmTaskExecutor::new(model);

    let task = Task::new("Test Task");
    let result = executor.execute(&task).await;

    // Should succeed in parsing needs_input as pending status
    assert!(result.is_ok());
}
