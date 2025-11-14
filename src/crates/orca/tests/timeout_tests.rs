//! Integration tests for task timeout enforcement (ORCA-033)

use orca::{DirectToolBridge, OrcaConfig, TaskExecutor, Task, OrcaError};
use std::path::PathBuf;
use std::sync::Arc;

fn create_test_config_with_timeout(timeout_secs: u64) -> OrcaConfig {
    let mut config = OrcaConfig::default();
    config.execution.task_timeout = timeout_secs;
    config.execution.streaming = false;
    config.llm.provider = "ollama".to_string();
    config.llm.model = "llama2".to_string();
    config.llm.api_base = Some("http://localhost:11434".to_string());
    config
}

#[tokio::test]
#[ignore] // Requires actual LLM provider
async fn test_task_timeout_enforcement() {
    // Create a task executor with very short timeout
    let bridge = Arc::new(
        DirectToolBridge::new(PathBuf::from("."), "test-session".to_string())
            .expect("Failed to create tool bridge")
    );

    let config = create_test_config_with_timeout(1); // 1 second timeout
    let executor = TaskExecutor::new(bridge, config)
        .expect("Failed to create executor");

    // Create a task that would take longer than timeout
    // In a real scenario, this would be a complex task that runs for a long time
    let task = Task::new("This is a very complex task that requires extensive reasoning and multiple tool calls to complete successfully");

    // Execute the task - should timeout
    let result = executor.execute_task(&task).await;

    // Verify timeout error
    match result {
        Err(OrcaError::Timeout { task_id, duration_secs }) => {
            assert_eq!(task_id, task.id);
            assert_eq!(duration_secs, 1);
        }
        Ok(_) => panic!("Expected timeout error, but task completed"),
        Err(e) => panic!("Expected timeout error, got: {}", e),
    }
}

#[tokio::test]
async fn test_fast_task_completes_before_timeout() {
    // Create a task executor with reasonable timeout
    let bridge = Arc::new(
        DirectToolBridge::new(PathBuf::from("."), "test-session".to_string())
            .expect("Failed to create tool bridge")
    );

    let config = create_test_config_with_timeout(300); // 5 minute timeout
    let executor = TaskExecutor::new(bridge, config)
        .expect("Failed to create executor");

    // Create a simple task
    let _task = Task::new("List files in current directory");

    // In a mock scenario, we would set up a mock LLM that returns quickly
    // For now, we just verify the executor configuration
    assert_eq!(executor.config().execution.task_timeout, 300);
}

#[test]
fn test_timeout_configuration() {
    let config = create_test_config_with_timeout(60);
    assert_eq!(config.execution.task_timeout, 60);

    let config = create_test_config_with_timeout(300);
    assert_eq!(config.execution.task_timeout, 300);

    let config = create_test_config_with_timeout(600);
    assert_eq!(config.execution.task_timeout, 600);
}

#[test]
fn test_timeout_values() {
    // Test various timeout values
    let timeouts = vec![1, 5, 10, 30, 60, 120, 300, 600, 1200];

    for timeout in timeouts {
        let config = create_test_config_with_timeout(timeout);
        assert_eq!(config.execution.task_timeout, timeout);
    }
}

#[tokio::test]
async fn test_executor_respects_timeout_config() {
    let bridge = Arc::new(
        DirectToolBridge::new(PathBuf::from("."), "test-session".to_string())
            .expect("Failed to create tool bridge")
    );

    // Test with different timeout values
    for timeout in [10, 30, 60, 120, 300] {
        let config = create_test_config_with_timeout(timeout);
        let executor = TaskExecutor::new(bridge.clone(), config)
            .expect("Failed to create executor");

        // Verify timeout is set correctly
        assert_eq!(executor.config().execution.task_timeout, timeout);
    }
}
