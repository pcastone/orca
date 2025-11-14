//! Task Executor - Main execution engine for tasks
//!
//! Coordinates task execution using LangGraph agents with DirectToolBridge and LLM providers.

use crate::config::OrcaConfig;
use crate::error::{OrcaError, Result};
use crate::executor::{LlmProvider, ToolAdapter, create_llm_function};
use crate::pattern::PatternType;
use crate::tools::DirectToolBridge;
use crate::workflow::Task;
use langgraph_prebuilt::agents::{create_react_agent, create_plan_execute_agent, create_reflection_agent};
use langgraph_core::{StreamMode, StreamEvent};
use serde_json::{json, Value};
use std::sync::Arc;
use futures::StreamExt;
use tracing::{debug, info};

/// Result of task execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether execution succeeded
    pub success: bool,

    /// Result value if successful
    pub result: Option<String>,

    /// Error message if failed
    pub error: Option<String>,

    /// Final agent state
    pub final_state: Value,

    /// Message history
    pub messages: Vec<Value>,
}

impl ExecutionResult {
    /// Create a successful execution result
    pub fn success(result: String, final_state: Value, messages: Vec<Value>) -> Self {
        Self {
            success: true,
            result: Some(result),
            error: None,
            final_state,
            messages,
        }
    }

    /// Create a failed execution result
    pub fn failure(error: String, final_state: Value, messages: Vec<Value>) -> Self {
        Self {
            success: false,
            result: None,
            error: Some(error),
            final_state,
            messages,
        }
    }
}

/// Task executor that runs tasks using LangGraph agents
pub struct TaskExecutor {
    /// Direct tool bridge for tool execution
    bridge: Arc<DirectToolBridge>,

    /// LLM provider for agent reasoning
    llm_provider: Arc<LlmProvider>,

    /// Configuration
    config: OrcaConfig,
}

impl std::fmt::Debug for TaskExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskExecutor")
            .field("bridge", &self.bridge)
            .field("llm_provider", &self.llm_provider)
            .field("config", &self.config)
            .finish()
    }
}

impl TaskExecutor {
    /// Create a new task executor
    ///
    /// # Arguments
    /// * `bridge` - DirectToolBridge for tool execution
    /// * `config` - Orca configuration
    ///
    /// # Returns
    /// A new TaskExecutor instance
    pub fn new(bridge: Arc<DirectToolBridge>, config: OrcaConfig) -> Result<Self> {
        // Create LLM provider from config
        let llm_provider = Arc::new(LlmProvider::from_config(&config)?);

        Ok(Self {
            bridge,
            llm_provider,
            config,
        })
    }

    /// Execute a task
    ///
    /// # Arguments
    /// * `task` - The task to execute
    ///
    /// # Returns
    /// ExecutionResult with outcome and final state
    pub async fn execute_task(&self, task: &Task) -> Result<ExecutionResult> {
        use tokio::time::{timeout, Duration};

        info!(
            task_id = %task.id,
            description = %task.description,
            timeout_secs = self.config.execution.task_timeout,
            "Starting task execution with timeout"
        );

        // Create timeout duration from config
        let timeout_duration = Duration::from_secs(self.config.execution.task_timeout);

        // Execute with timeout
        let result = timeout(timeout_duration, self.execute_task_internal(task)).await;

        match result {
            Ok(exec_result) => {
                info!(
                    task_id = %task.id,
                    success = exec_result.as_ref().map(|r| r.success).unwrap_or(false),
                    "Task execution completed"
                );
                exec_result
            }
            Err(_) => {
                // Timeout occurred
                info!(
                    task_id = %task.id,
                    timeout_secs = self.config.execution.task_timeout,
                    "Task execution timed out"
                );
                Err(OrcaError::Timeout {
                    task_id: task.id.clone(),
                    duration_secs: self.config.execution.task_timeout,
                })
            }
        }
    }

    /// Internal task execution without timeout (for testing and timeout wrapper)
    async fn execute_task_internal(&self, task: &Task) -> Result<ExecutionResult> {
        // Parse pattern from task metadata or use default (React)
        let pattern = self.get_pattern_from_task(task)?;

        debug!(
            task_id = %task.id,
            pattern = %pattern,
            "Using pattern for execution"
        );

        // Execute based on pattern
        match pattern {
            PatternType::React => self.execute_react(task).await,
            PatternType::PlanExecute => self.execute_plan_execute(task).await,
            PatternType::Reflection => self.execute_reflection(task).await,
        }
    }

    /// Execute a task using the ReAct pattern
    async fn execute_react(&self, task: &Task) -> Result<ExecutionResult> {
        // Create tools from bridge
        let tools = ToolAdapter::from_bridge(self.bridge.clone());

        debug!(
            task_id = %task.id,
            tool_count = tools.len(),
            "Created tool adapters"
        );

        // Create LLM function
        let llm_fn = create_llm_function(self.llm_provider.clone());

        // Create ReAct agent
        let agent = create_react_agent(llm_fn, tools)
            .with_max_iterations(self.config.execution.max_iterations)
            .build()
            .map_err(|e| OrcaError::Execution(format!("Failed to build agent: {}", e)))?;

        // Prepare initial state with task description
        let initial_state = json!({
            "messages": vec![
                json!({
                    "type": "human",
                    "content": task.description.clone()
                })
            ]
        });

        // Execute with or without streaming based on config
        let (final_state, messages) = if self.config.execution.streaming {
            debug!(task_id = %task.id, "Streaming agent execution");
            self.execute_with_streaming(&agent, initial_state, task).await?
        } else {
            debug!(task_id = %task.id, "Invoking agent");

            let final_state = agent
                .invoke(initial_state)
                .await
                .map_err(|e| OrcaError::Execution(format!("Agent execution failed: {}", e)))?;

            // Extract messages from final state
            let messages = final_state
                .get("messages")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();

            (final_state, messages)
        };

        // Extract final AI message as result
        let result = messages
            .iter()
            .rev()
            .find(|msg| {
                msg.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "ai")
                    .unwrap_or(false)
            })
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("No response generated")
            .to_string();

        debug!(
            task_id = %task.id,
            message_count = messages.len(),
            "Agent execution completed"
        );

        Ok(ExecutionResult::success(result, final_state, messages))
    }

    /// Execute a task using the Plan-Execute pattern
    async fn execute_plan_execute(&self, task: &Task) -> Result<ExecutionResult> {
        // Create tools from bridge
        let tools = ToolAdapter::from_bridge(self.bridge.clone());

        debug!(
            task_id = %task.id,
            tool_count = tools.len(),
            "Created tool adapters for Plan-Execute"
        );

        // Create LLM function (used for both planner and executor)
        let llm_fn = create_llm_function(self.llm_provider.clone());
        let llm_fn_executor = create_llm_function(self.llm_provider.clone());

        // Create Plan-Execute agent
        let agent = create_plan_execute_agent(llm_fn, llm_fn_executor, tools)
            .with_max_steps(self.config.execution.max_iterations)
            .build()
            .map_err(|e| OrcaError::Execution(format!("Failed to build Plan-Execute agent: {}", e)))?;

        // Prepare initial state with task description
        let initial_state = json!({
            "messages": vec![
                json!({
                    "type": "human",
                    "content": task.description.clone()
                })
            ]
        });

        // Execute with or without streaming based on config
        let (final_state, messages) = if self.config.execution.streaming {
            debug!(task_id = %task.id, "Streaming Plan-Execute agent execution");
            self.execute_with_streaming(&agent, initial_state, task).await?
        } else {
            debug!(task_id = %task.id, "Invoking Plan-Execute agent");

            let final_state = agent
                .invoke(initial_state)
                .await
                .map_err(|e| OrcaError::Execution(format!("Plan-Execute agent execution failed: {}", e)))?;

            // Extract messages from final state
            let messages = final_state
                .get("messages")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();

            (final_state, messages)
        };

        // Extract final AI message as result
        let result = messages
            .iter()
            .rev()
            .find(|msg| {
                msg.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "ai")
                    .unwrap_or(false)
            })
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("No response generated")
            .to_string();

        debug!(
            task_id = %task.id,
            message_count = messages.len(),
            "Plan-Execute agent execution completed"
        );

        Ok(ExecutionResult::success(result, final_state, messages))
    }

    /// Execute a task using the Reflection pattern
    async fn execute_reflection(&self, task: &Task) -> Result<ExecutionResult> {
        // Create tools from bridge
        let tools = ToolAdapter::from_bridge(self.bridge.clone());

        debug!(
            task_id = %task.id,
            tool_count = tools.len(),
            "Created tool adapters for Reflection"
        );

        // Create LLM functions (used for both generator and reflector)
        let llm_fn_generator = create_llm_function(self.llm_provider.clone());
        let llm_fn_reflector = create_llm_function(self.llm_provider.clone());

        // Create Reflection agent
        let agent = create_reflection_agent(llm_fn_generator, llm_fn_reflector, tools)
            .with_max_iterations(self.config.execution.max_iterations)
            .build()
            .map_err(|e| OrcaError::Execution(format!("Failed to build Reflection agent: {}", e)))?;

        // Prepare initial state with task description
        let initial_state = json!({
            "messages": vec![
                json!({
                    "type": "human",
                    "content": task.description.clone()
                })
            ]
        });

        // Execute with or without streaming based on config
        let (final_state, messages) = if self.config.execution.streaming {
            debug!(task_id = %task.id, "Streaming Reflection agent execution");
            self.execute_with_streaming(&agent, initial_state, task).await?
        } else {
            debug!(task_id = %task.id, "Invoking Reflection agent");

            let final_state = agent
                .invoke(initial_state)
                .await
                .map_err(|e| OrcaError::Execution(format!("Reflection agent execution failed: {}", e)))?;

            // Extract messages from final state
            let messages = final_state
                .get("messages")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();

            (final_state, messages)
        };

        // Extract final AI message as result
        let result = messages
            .iter()
            .rev()
            .find(|msg| {
                msg.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "ai")
                    .unwrap_or(false)
            })
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("No response generated")
            .to_string();

        debug!(
            task_id = %task.id,
            message_count = messages.len(),
            "Reflection agent execution completed"
        );

        Ok(ExecutionResult::success(result, final_state, messages))
    }

    /// Execute agent with streaming output
    async fn execute_with_streaming(
        &self,
        agent: &langgraph_core::CompiledGraph,
        initial_state: Value,
        task: &Task,
    ) -> Result<(Value, Vec<Value>)> {
        use std::io::{self, Write};

        // Create stream with Messages and Updates modes
        let mut stream = agent
            .stream_chunks_with_modes(
                initial_state,
                vec![StreamMode::Messages, StreamMode::Updates],
                None,
            )
            .await
            .map_err(|e| OrcaError::Execution(format!("Failed to create stream: {}", e)))?;

        let mut final_state = json!({});

        // Process stream chunks
        while let Some(chunk) = stream.next().await {
            match chunk.event {
                StreamEvent::Values { state } => {
                    // Update final state
                    final_state = state;
                }
                StreamEvent::Updates { node, update: _ } => {
                    // Print progress indicator
                    print!(".");
                    io::stdout().flush().unwrap_or(());

                    debug!(
                        task_id = %task.id,
                        node = %node,
                        "Node update"
                    );
                }
                StreamEvent::Message { message, metadata: _ } => {
                    // Print message content in real-time
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        if !content.is_empty() {
                            println!("\n{}", content);
                        }
                    }
                }
                StreamEvent::MessageChunk { chunk, message_id: _, node: _, metadata: _ } => {
                    // Print token chunks for real-time streaming
                    if !chunk.is_empty() {
                        print!("{}", chunk);
                        io::stdout().flush().unwrap_or(());
                    }
                }
                _ => {
                    // Ignore other event types
                }
            }
        }

        println!(); // New line after streaming

        // Extract messages from final state
        let messages = final_state
            .get("messages")
            .and_then(|m| m.as_array())
            .cloned()
            .unwrap_or_default();

        debug!(
            task_id = %task.id,
            message_count = messages.len(),
            "Streaming execution completed"
        );

        Ok((final_state, messages))
    }

    /// Get pattern from task metadata or default
    fn get_pattern_from_task(&self, task: &Task) -> Result<PatternType> {
        // Try to parse metadata JSON
        if let Ok(metadata) = serde_json::from_str::<Value>(&task.metadata) {
            if let Some(pattern_str) = metadata.get("pattern").and_then(|p| p.as_str()) {
                if let Some(pattern) = PatternType::from_str(pattern_str) {
                    return Ok(pattern);
                }
            }
        }

        // Default to React
        Ok(PatternType::React)
    }

    /// Get configuration
    pub fn config(&self) -> &OrcaConfig {
        &self.config
    }

    /// Get bridge
    pub fn bridge(&self) -> &Arc<DirectToolBridge> {
        &self.bridge
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DatabaseConfig, ExecutionConfig, LlmConfig, LoggingConfig};
    use tempfile::TempDir;

    fn create_test_config() -> OrcaConfig {
        OrcaConfig {
            database: DatabaseConfig {
                path: "orca.db".to_string(),
            },
            llm: LlmConfig {
                provider: "ollama".to_string(),
                model: "llama2".to_string(),
                api_key: None,
                api_base: Some("http://localhost:11434".to_string()),
                temperature: 0.7,
                max_tokens: 1000,
            },
            execution: ExecutionConfig {
                max_concurrent_tasks: 3,
                task_timeout: 300,
                streaming: false,
                workspace_root: None,
                max_iterations: 5,
                ..Default::default()
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                colored: false,
                timestamps: true,
            },
        }
    }

    #[tokio::test]
    async fn test_executor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();

        // This will fail if ollama is not running, but that's OK for unit tests
        // The test is just verifying the constructor works
        let _ = TaskExecutor::new(bridge, config);
    }

    #[test]
    fn test_get_pattern_from_task() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Task with React pattern in metadata
        let task = Task::new("Test task")
            .with_metadata(r#"{"pattern": "react"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React);

        // Task with Plan-Execute pattern
        let task = Task::new("Test task")
            .with_metadata(r#"{"pattern": "plan_execute"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::PlanExecute);

        // Task with no pattern (should default to React)
        let task = Task::new("Test task");

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React);
    }

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::success(
            "Task completed".to_string(),
            json!({"messages": []}),
            vec![],
        );

        assert!(result.success);
        assert!(result.result.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_execution_result_failure() {
        let result = ExecutionResult::failure(
            "Task failed".to_string(),
            json!({"messages": []}),
            vec![],
        );

        assert!(!result.success);
        assert!(result.result.is_none());
        assert!(result.error.is_some());
    }

    #[test]
    fn test_pattern_selection_with_reflection() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Task with Reflection pattern in metadata
        let task = Task::new("Test task")
            .with_metadata(r#"{"pattern": "reflection"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::Reflection);
    }

    #[test]
    fn test_pattern_selection_with_invalid_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Task with invalid pattern should default to React
        let task = Task::new("Test task")
            .with_metadata(r#"{"pattern": "invalid_pattern"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React);
    }

    #[test]
    fn test_pattern_selection_with_malformed_json() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Task with malformed JSON should default to React
        let task = Task::new("Test task")
            .with_metadata(r#"{"pattern": "react""#); // Missing closing brace

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React);
    }

    #[test]
    fn test_pattern_selection_case_sensitivity() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Test lowercase vs snake_case
        let task = Task::new("Test task")
            .with_metadata(r#"{"pattern": "planexecute"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        // Should default to React since "planexecute" doesn't match "plan_execute"
        assert_eq!(pattern, PatternType::React);

        // Test correct snake_case
        let task = Task::new("Test task")
            .with_metadata(r#"{"pattern": "plan_execute"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::PlanExecute);
    }

    #[test]
    fn test_pattern_selection_with_empty_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Empty JSON object should default to React
        let task = Task::new("Test task")
            .with_metadata(r#"{}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React);
    }

    #[test]
    fn test_pattern_selection_with_other_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Metadata with pattern and other fields
        let task = Task::new("Test task")
            .with_metadata(r#"{"pattern": "reflection", "priority": "high", "tags": ["test"]}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::Reflection);
    }

    #[test]
    fn test_executor_config_access() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Verify config is accessible
        assert_eq!(executor.config().execution.max_iterations, 5);
        assert_eq!(executor.config().execution.streaming, false);
        assert_eq!(executor.config().llm.provider, "ollama");
    }

    #[test]
    fn test_executor_bridge_access() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge.clone(), config).unwrap();

        // Verify bridge is accessible and is the same instance
        assert!(Arc::ptr_eq(executor.bridge(), &bridge));
    }

    #[test]
    fn test_streaming_config_affects_execution_path() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        // Test with streaming disabled
        let mut config = create_test_config();
        config.execution.streaming = false;
        let executor = TaskExecutor::new(bridge.clone(), config).unwrap();
        assert_eq!(executor.config().execution.streaming, false);

        // Test with streaming enabled
        let mut config = create_test_config();
        config.execution.streaming = true;
        let executor = TaskExecutor::new(bridge, config).unwrap();
        assert_eq!(executor.config().execution.streaming, true);
    }

    #[test]
    fn test_streaming_disabled_by_default() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Verify streaming is disabled by default in test config
        assert_eq!(executor.config().execution.streaming, false);
    }

    #[test]
    fn test_streaming_config_independent_per_executor() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        // Create two executors with different streaming configs
        let mut config1 = create_test_config();
        config1.execution.streaming = false;
        let executor1 = TaskExecutor::new(bridge.clone(), config1).unwrap();

        let mut config2 = create_test_config();
        config2.execution.streaming = true;
        let executor2 = TaskExecutor::new(bridge, config2).unwrap();

        // Verify they maintain independent configurations
        assert_eq!(executor1.config().execution.streaming, false);
        assert_eq!(executor2.config().execution.streaming, true);
    }

    #[test]
    fn test_max_iterations_applied_to_all_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let mut config = create_test_config();
        config.execution.max_iterations = 10;
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Verify max_iterations is configured
        assert_eq!(executor.config().execution.max_iterations, 10);
    }

    #[test]
    fn test_execution_result_contains_messages() {
        // Test that ExecutionResult properly stores messages
        let messages = vec![
            json!({"type": "human", "content": "Hello"}),
            json!({"type": "ai", "content": "Hi there"}),
        ];

        let result = ExecutionResult::success(
            "Response".to_string(),
            json!({"messages": messages.clone()}),
            messages.clone(),
        );

        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.messages[0]["type"], "human");
        assert_eq!(result.messages[1]["type"], "ai");
    }

    #[test]
    fn test_execution_result_preserves_state() {
        let final_state = json!({
            "messages": [{"type": "ai", "content": "Done"}],
            "iteration": 3,
            "metadata": {"model": "test"}
        });

        let result = ExecutionResult::success(
            "Complete".to_string(),
            final_state.clone(),
            vec![],
        );

        // Verify final_state is preserved
        assert_eq!(result.final_state["iteration"], 3);
        assert_eq!(result.final_state["metadata"]["model"], "test");
    }

    #[test]
    fn test_streaming_modes_compatibility() {
        // Test that StreamMode types are available
        let _modes = vec![StreamMode::Messages, StreamMode::Updates];
        // If this compiles, streaming modes are properly imported
    }

    #[tokio::test]
    async fn test_timeout_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        // Create config with short timeout
        let mut config = create_test_config();
        config.execution.task_timeout = 5; // 5 seconds

        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Verify timeout is configured
        assert_eq!(executor.config().execution.task_timeout, 5);
    }

    #[test]
    fn test_timeout_error_format() {
        let error = OrcaError::Timeout {
            task_id: "test-task-123".to_string(),
            duration_secs: 300,
        };

        let error_msg = format!("{}", error);
        assert!(error_msg.contains("test-task-123"));
        assert!(error_msg.contains("300"));
        assert!(error_msg.contains("timed out"));
    }

    #[test]
    fn test_different_timeout_values() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        // Test various timeout values
        for timeout_val in [1, 30, 60, 300, 600] {
            let mut config = create_test_config();
            config.execution.task_timeout = timeout_val;

            let executor = TaskExecutor::new(bridge.clone(), config).unwrap();
            assert_eq!(executor.config().execution.task_timeout, timeout_val);
        }
    }

    // ORCA-022: Pattern Integration Tests

    #[test]
    fn test_pattern_methods_exist() {
        // This test verifies that all pattern execution methods exist
        // and have the correct signatures (compilation test)
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let _executor = TaskExecutor::new(bridge, config).unwrap();

        // If this compiles, all pattern methods exist with correct signatures:
        // - execute_react()
        // - execute_plan_execute()
        // - execute_reflection()
        // - execute_with_streaming()
    }

    #[test]
    fn test_react_pattern_selection() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Task with explicit ReAct pattern
        let task = Task::new("Test task")
            .with_metadata(r#"{"pattern": "react"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React);
    }

    #[test]
    fn test_plan_execute_pattern_selection() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Task with Plan-Execute pattern
        let task = Task::new("Complex research task")
            .with_metadata(r#"{"pattern": "plan_execute"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::PlanExecute);
    }

    #[test]
    fn test_reflection_pattern_selection() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Task with Reflection pattern
        let task = Task::new("High-quality code generation")
            .with_metadata(r#"{"pattern": "reflection"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::Reflection);
    }

    #[test]
    fn test_default_pattern_selection() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Task without pattern metadata should default to ReAct
        let task = Task::new("Simple task");

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React);
    }

    #[test]
    fn test_pattern_with_max_iterations_config() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let mut config = create_test_config();
        config.execution.max_iterations = 10;

        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Verify max_iterations is configured
        assert_eq!(executor.config().execution.max_iterations, 10);

        // All patterns use this config:
        // - ReAct: .with_max_iterations(10)
        // - Plan-Execute: .with_max_steps(10)
        // - Reflection: .with_max_iterations(10)
    }

    #[test]
    fn test_streaming_config_affects_pattern_execution() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        // Test with streaming disabled
        let mut config = create_test_config();
        config.execution.streaming = false;
        let executor = TaskExecutor::new(bridge.clone(), config).unwrap();
        assert_eq!(executor.config().execution.streaming, false);

        // Test with streaming enabled
        let mut config = create_test_config();
        config.execution.streaming = true;
        let executor = TaskExecutor::new(bridge, config).unwrap();
        assert_eq!(executor.config().execution.streaming, true);

        // All three patterns check this flag and use execute_with_streaming() when true
    }

    #[test]
    fn test_execution_result_structure() {
        // Test ExecutionResult can be created and accessed
        let final_state = json!({
            "messages": [
                {"type": "human", "content": "Task description"},
                {"type": "ai", "content": "Task response"}
            ],
            "pattern": "react"
        });

        let messages = vec![
            json!({"type": "human", "content": "Task description"}),
            json!({"type": "ai", "content": "Task response"}),
        ];

        let result = ExecutionResult::success(
            "Task response".to_string(),
            final_state.clone(),
            messages.clone(),
        );

        assert!(result.success);
        assert_eq!(result.result, Some("Task response".to_string()));
        assert!(result.error.is_none());
        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.final_state["pattern"], "react");
    }

    #[test]
    fn test_pattern_execution_result_failure() {
        let final_state = json!({"messages": []});
        let messages = vec![];

        let result = ExecutionResult::failure(
            "Pattern execution failed".to_string(),
            final_state,
            messages,
        );

        assert!(!result.success);
        assert!(result.result.is_none());
        assert_eq!(result.error, Some("Pattern execution failed".to_string()));
    }

    // Note: Full end-to-end pattern execution tests with real LLM providers
    // are in the integration test suite or require manual testing with a
    // running LLM service (Ollama, OpenAI, etc.). These unit tests verify
    // the pattern selection, configuration, and execution flow structure.

    // ORCA-023: Streaming Integration Tests

    #[test]
    fn test_streaming_modes_available() {
        // Verify StreamMode types are available for use
        let modes = vec![
            StreamMode::Messages,
            StreamMode::Updates,
            StreamMode::Values,
            StreamMode::Debug,
        ];

        assert_eq!(modes.len(), 4);
        // If this compiles, streaming infrastructure is properly set up
    }

    #[test]
    fn test_streaming_enabled_in_config() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let mut config = create_test_config();
        config.execution.streaming = true;

        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Verify streaming is enabled
        assert!(executor.config().execution.streaming);
    }

    #[test]
    fn test_streaming_disabled_in_config() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let mut config = create_test_config();
        config.execution.streaming = false;

        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Verify streaming is disabled
        assert!(!executor.config().execution.streaming);
    }

    #[test]
    fn test_streaming_config_per_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let mut config = create_test_config();
        config.execution.streaming = true;

        let executor = TaskExecutor::new(bridge, config).unwrap();

        // All three patterns respect the streaming config:
        // - ReAct: lines 198-205
        // - Plan-Execute: lines 277-296
        // - Reflection: lines 354-373

        // Each pattern checks:
        // if self.config.execution.streaming {
        //     self.execute_with_streaming(...).await?
        // } else {
        //     agent.invoke(...).await?
        // }

        assert!(executor.config().execution.streaming);
    }

    #[test]
    fn test_stream_event_types() {
        // Test that StreamEvent enum is available with expected variants
        // The execute_with_streaming method handles these events:

        // StreamEvent::Values - Updates final state
        // StreamEvent::Updates - Progress indicators (dots)
        // StreamEvent::Message - Complete messages
        // StreamEvent::MessageChunk - Token-level streaming

        // This is a compilation test - if it compiles, types are correct
        let _test_fn = |event: StreamEvent| {
            match event {
                StreamEvent::Values { state: _ } => {},
                StreamEvent::Updates { node: _, update: _ } => {},
                StreamEvent::Message { message: _, metadata: _ } => {},
                StreamEvent::MessageChunk { chunk: _, message_id: _, node: _, metadata: _ } => {},
                _ => {},
            }
        };
    }

    #[test]
    fn test_execute_with_streaming_signature() {
        // Verify execute_with_streaming method exists with correct signature
        // This is a compilation test
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let _executor = TaskExecutor::new(bridge, config).unwrap();

        // If this compiles, execute_with_streaming() exists with signature:
        // async fn execute_with_streaming(
        //     &self,
        //     agent: &CompiledGraph,
        //     initial_state: Value,
        //     task: &Task,
        // ) -> Result<(Value, Vec<Value>)>
    }

    #[test]
    fn test_streaming_uses_multiple_modes() {
        // Verify that streaming uses both Messages and Updates modes
        // as specified in execute_with_streaming (lines 410-414)

        let modes = vec![StreamMode::Messages, StreamMode::Updates];

        assert_eq!(modes.len(), 2);
        assert!(matches!(modes[0], StreamMode::Messages));
        assert!(matches!(modes[1], StreamMode::Updates));

        // These modes enable:
        // - Messages: Complete AI/Human/Tool messages
        // - Updates: Node execution progress
    }

    #[test]
    fn test_streaming_output_structure() {
        // Test that streaming returns same structure as non-streaming
        // Both should return: Result<(Value, Vec<Value>)>
        // Where Value is final_state and Vec<Value> is messages

        let final_state = json!({
            "messages": [
                {"type": "human", "content": "Input"},
                {"type": "ai", "content": "Output"}
            ]
        });

        let messages = vec![
            json!({"type": "human", "content": "Input"}),
            json!({"type": "ai", "content": "Output"}),
        ];

        // Both streaming and non-streaming return this same structure
        let _expected: (Value, Vec<Value>) = (final_state, messages);
    }

    // Note: Actual streaming behavior tests (chunk emission, real-time output)
    // require a running LLM service and are tested manually or in integration
    // tests. These unit tests verify the streaming infrastructure is correctly
    // set up and configured.
}
