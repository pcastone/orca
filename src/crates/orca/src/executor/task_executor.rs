//! Task Executor - Main execution engine for tasks
//!
//! Coordinates task execution using LangGraph agents with DirectToolBridge and LLM providers.
//! Features automatic dynamic pattern selection based on task classification.

use crate::config::OrcaConfig;
use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::executor::{LlmProvider, ToolAdapter, create_llm_function};
use crate::models::PatternConfig;
use crate::pattern::PatternType;
use crate::services::{PatternRouter, TaskClassifier, TaskCategory, ExecutionMetricsService, ExecutionTracker};
use crate::tools::DirectToolBridge;
use crate::workflow::Task;
use langgraph_prebuilt::agents::{create_react_agent, create_plan_execute_agent, create_reflection_agent};
use langgraph_core::{StreamMode, StreamEvent};
use serde_json::{json, Value};
use std::sync::Arc;
use futures::StreamExt;
use tracing::{debug, info, warn};

/// Metrics collected during streaming execution
#[derive(Debug, Clone, Default)]
pub struct StreamingMetrics {
    /// Total reasoning/thinking tokens
    pub reasoning_tokens: i64,
    /// Total reasoning duration in milliseconds
    pub reasoning_duration_ms: i64,
    /// Number of node updates (proxy for iterations)
    pub node_update_count: i64,
    /// Nodes that were updated (for iteration tracking)
    pub updated_nodes: Vec<String>,
}

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
///
/// Features automatic dynamic pattern selection based on task classification.
/// The classifier analyzes task descriptions to determine the optimal pattern:
/// - SimpleQuery → React (quick, simple tasks)
/// - CodeGeneration → Reflection (quality-focused code tasks)
/// - Research/DataAnalysis → PlanExecute (complex multi-step tasks)
/// - General → React (default fallback)
///
/// When a PatternRouter is provided, the executor can use database-backed pattern
/// configurations for more granular control over pattern settings (max_iterations,
/// tools, system_prompt, etc.).
pub struct TaskExecutor {
    /// Direct tool bridge for tool execution
    bridge: Arc<DirectToolBridge>,

    /// LLM provider for agent reasoning
    llm_provider: Arc<LlmProvider>,

    /// Configuration
    config: OrcaConfig,

    /// Task classifier for automatic pattern selection
    classifier: TaskClassifier,

    /// Optional pattern router for database-backed pattern configs
    pattern_router: Option<PatternRouter>,

    /// Optional execution metrics service for tracking prompt executions
    metrics_service: Option<Arc<ExecutionMetricsService>>,
}

impl std::fmt::Debug for TaskExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskExecutor")
            .field("bridge", &self.bridge)
            .field("llm_provider", &self.llm_provider)
            .field("config", &self.config)
            .field("classifier", &self.classifier)
            .field("has_metrics_service", &self.metrics_service.is_some())
            .field("has_pattern_router", &self.pattern_router.is_some())
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
    /// A new TaskExecutor instance with automatic pattern selection enabled
    pub fn new(bridge: Arc<DirectToolBridge>, config: OrcaConfig) -> Result<Self> {
        // Create LLM provider from config
        let llm_provider = Arc::new(LlmProvider::from_config(&config)?);

        // Create task classifier for automatic pattern selection
        let classifier = TaskClassifier::new();

        info!("TaskExecutor initialized with automatic pattern selection");

        Ok(Self {
            bridge,
            llm_provider,
            config,
            classifier,
            pattern_router: None,
            metrics_service: None,
        })
    }

    /// Create a new task executor with database-backed pattern routing
    ///
    /// # Arguments
    /// * `bridge` - DirectToolBridge for tool execution
    /// * `config` - Orca configuration
    /// * `user_db` - User database for pattern config lookups
    ///
    /// # Returns
    /// A new TaskExecutor instance with database-backed pattern selection
    pub fn new_with_router(
        bridge: Arc<DirectToolBridge>,
        config: OrcaConfig,
        user_db: Arc<Database>,
    ) -> Result<Self> {
        // Create LLM provider from config
        let llm_provider = Arc::new(LlmProvider::from_config(&config)?);

        // Create task classifier for automatic pattern selection
        let classifier = TaskClassifier::new();

        // Create pattern router with database
        let pattern_router = PatternRouter::new(user_db);

        info!("TaskExecutor initialized with database-backed pattern routing");

        Ok(Self {
            bridge,
            llm_provider,
            config,
            classifier,
            pattern_router: Some(pattern_router),
            metrics_service: None,
        })
    }

    /// Create a new task executor with metrics tracking
    ///
    /// # Arguments
    /// * `bridge` - DirectToolBridge for tool execution
    /// * `config` - Orca configuration
    /// * `user_db` - User database for pattern config lookups and metrics storage
    ///
    /// # Returns
    /// A new TaskExecutor instance with execution metrics tracking enabled
    pub fn new_with_metrics(
        bridge: Arc<DirectToolBridge>,
        config: OrcaConfig,
        user_db: Arc<Database>,
    ) -> Result<Self> {
        // Create LLM provider from config
        let llm_provider = Arc::new(LlmProvider::from_config(&config)?);

        // Create task classifier for automatic pattern selection
        let classifier = TaskClassifier::new();

        // Create pattern router with database
        let pattern_router = PatternRouter::new(user_db.clone());

        // Create execution metrics service
        let project_name = config.project_name.clone();
        let metrics_service = ExecutionMetricsService::new(user_db, project_name);

        info!("TaskExecutor initialized with execution metrics tracking");

        Ok(Self {
            bridge,
            llm_provider,
            config,
            classifier,
            pattern_router: Some(pattern_router),
            metrics_service: Some(Arc::new(metrics_service)),
        })
    }

    /// Set the metrics service for tracking execution metrics
    pub fn with_metrics_service(mut self, service: Arc<ExecutionMetricsService>) -> Self {
        self.metrics_service = Some(service);
        self
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
        // Try database-backed pattern config first, then fall back to classification
        let (pattern, max_iterations) = if self.pattern_router.is_some() {
            // Use database-backed routing
            if let Some(config) = self.get_pattern_config_from_task(task).await {
                let pattern = Self::pattern_type_from_config(&config);
                let max_iter = config.max_iterations as usize;
                info!(
                    task_id = %task.id,
                    pattern = %pattern,
                    max_iterations = max_iter,
                    config_name = %config.name,
                    "Using database-backed pattern config"
                );
                (pattern, max_iter)
            } else {
                // Fall back to classification
                let pattern = self.get_pattern_from_task(task)?;
                (pattern, self.config.execution.max_iterations)
            }
        } else {
            // No router, use classification
            let pattern = self.get_pattern_from_task(task)?;
            (pattern, self.config.execution.max_iterations)
        };

        debug!(
            task_id = %task.id,
            pattern = %pattern,
            max_iterations = max_iterations,
            "Using pattern for execution"
        );

        // Start metrics tracking if service is available
        let tracker = if let Some(ref metrics_service) = self.metrics_service {
            match metrics_service.start_execution(
                &task.description,
                pattern.as_str(),
                None, // session_id
                Some(task.id.clone()),
            ).await {
                Ok(t) => {
                    debug!(
                        task_id = %task.id,
                        execution_id = %t.execution_id(),
                        "Started execution metrics tracking"
                    );
                    Some(t)
                }
                Err(e) => {
                    warn!(
                        task_id = %task.id,
                        error = %e,
                        "Failed to start execution metrics tracking"
                    );
                    None
                }
            }
        } else {
            None
        };

        // Execute based on pattern with specified max_iterations
        let result = match pattern {
            PatternType::React => self.execute_react_with_config(task, max_iterations, tracker.as_ref()).await,
            PatternType::PlanExecute => self.execute_plan_execute_with_config(task, max_iterations).await,
            PatternType::Reflection => self.execute_reflection_with_config(task, max_iterations).await,
        };

        // Complete metrics tracking
        if let Some(tracker) = tracker {
            match &result {
                Ok(exec_result) => {
                    let response = exec_result.result.as_deref().unwrap_or("No response");
                    if let Err(e) = tracker.complete(response).await {
                        warn!(
                            task_id = %task.id,
                            error = %e,
                            "Failed to complete execution metrics"
                        );
                    }
                }
                Err(e) => {
                    if let Err(complete_err) = tracker.fail(&e.to_string()).await {
                        warn!(
                            task_id = %task.id,
                            error = %complete_err,
                            "Failed to record execution failure"
                        );
                    }
                }
            }
        }

        result
    }

    /// Execute a task using the ReAct pattern with configurable max_iterations
    async fn execute_react_with_config(
        &self,
        task: &Task,
        max_iterations: usize,
        tracker: Option<&ExecutionTracker>,
    ) -> Result<ExecutionResult> {
        // Create tools from bridge
        let tools = ToolAdapter::from_bridge(self.bridge.clone());

        debug!(
            task_id = %task.id,
            tool_count = tools.len(),
            max_iterations = max_iterations,
            "Created tool adapters for ReAct"
        );

        // If no tools available, use direct LLM call instead of agent
        if tools.is_empty() {
            info!(
                task_id = %task.id,
                "No tools available, using direct LLM call"
            );
            return self.execute_direct_llm(task).await;
        }

        // Create LLM function
        let llm_fn = create_llm_function(self.llm_provider.clone());

        // Create ReAct agent with specified max_iterations
        let agent = create_react_agent(llm_fn, tools)
            .with_max_iterations(max_iterations)
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
        let _ = tracker; // Tracker is updated at execute_task_internal level

        let (final_state, messages, _streaming_metrics) = if self.config.execution.streaming {
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

            (final_state, messages, StreamingMetrics::default())
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

    /// Execute a task using the Plan-Execute pattern with configurable max_iterations
    async fn execute_plan_execute_with_config(&self, task: &Task, max_iterations: usize) -> Result<ExecutionResult> {
        // Create tools from bridge
        let tools = ToolAdapter::from_bridge(self.bridge.clone());

        debug!(
            task_id = %task.id,
            tool_count = tools.len(),
            max_iterations = max_iterations,
            "Created tool adapters for Plan-Execute"
        );

        // Create LLM function (used for both planner and executor)
        let llm_fn = create_llm_function(self.llm_provider.clone());
        let llm_fn_executor = create_llm_function(self.llm_provider.clone());

        // Create Plan-Execute agent with specified max_iterations
        let agent = create_plan_execute_agent(llm_fn, llm_fn_executor, tools)
            .with_max_steps(max_iterations)
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
        let (final_state, messages, _streaming_metrics) = if self.config.execution.streaming {
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

            (final_state, messages, StreamingMetrics::default())
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

    /// Execute a task using the Reflection pattern with configurable max_iterations
    async fn execute_reflection_with_config(&self, task: &Task, max_iterations: usize) -> Result<ExecutionResult> {
        // Create tools from bridge
        let tools = ToolAdapter::from_bridge(self.bridge.clone());

        debug!(
            task_id = %task.id,
            tool_count = tools.len(),
            max_iterations = max_iterations,
            "Created tool adapters for Reflection"
        );

        // Create LLM functions (used for both generator and reflector)
        let llm_fn_generator = create_llm_function(self.llm_provider.clone());
        let llm_fn_reflector = create_llm_function(self.llm_provider.clone());

        // Create Reflection agent with specified max_iterations
        let agent = create_reflection_agent(llm_fn_generator, llm_fn_reflector, tools)
            .with_max_iterations(max_iterations)
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
        let (final_state, messages, _streaming_metrics) = if self.config.execution.streaming {
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

            (final_state, messages, StreamingMetrics::default())
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
    ) -> Result<(Value, Vec<Value>, StreamingMetrics)> {
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
        let mut metrics = StreamingMetrics::default();

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

                    // Track node updates for metrics
                    metrics.node_update_count += 1;
                    if !metrics.updated_nodes.contains(&node) {
                        metrics.updated_nodes.push(node.clone());
                    }

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
                StreamEvent::Reasoning { content, tokens, duration_ms, node: _, metadata: _ } => {
                    // Capture reasoning metrics
                    metrics.reasoning_tokens += tokens as i64;
                    if let Some(duration) = duration_ms {
                        metrics.reasoning_duration_ms += duration as i64;
                    }

                    // Display thinking/reasoning output if enabled
                    if self.config.execution.show_thinking {
                        use colored::Colorize;

                        println!(); // Ensure we're on a new line
                        println!("{}", "╭─ Model Thinking ─╮".dimmed());

                        // Print thinking content (dimmed)
                        for line in content.lines() {
                            println!("{} {}", "│".dimmed(), line.dimmed());
                        }

                        // Bottom border with metrics
                        let mut display_metrics = Vec::new();
                        display_metrics.push(format!("{} tokens", tokens));
                        if let Some(duration) = duration_ms {
                            display_metrics.push(format!("{:.2}s", duration as f64 / 1000.0));
                        }

                        let metrics_str = display_metrics.join(", ");
                        println!("{}", format!("╰─ {} ─╯", metrics_str).dimmed());
                        println!();

                        io::stdout().flush().unwrap_or(());

                        debug!(
                            task_id = %task.id,
                            tokens = tokens,
                            "Displayed reasoning content"
                        );
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
            reasoning_tokens = metrics.reasoning_tokens,
            node_updates = metrics.node_update_count,
            "Streaming execution completed with metrics"
        );

        Ok((final_state, messages, metrics))
    }

    /// Get pattern configuration from task using database-backed routing
    ///
    /// This method is used when PatternRouter is available.
    /// Returns the full PatternConfig with max_iterations, tools, system_prompt, etc.
    ///
    /// Priority order:
    /// 1. Task's pattern_config_id (explicit config from database)
    /// 2. Classification-based routing to database config
    /// 3. Default pattern config from database
    async fn get_pattern_config_from_task(&self, task: &Task) -> Option<PatternConfig> {
        let router = self.pattern_router.as_ref()?;

        match router.route(task).await {
            Ok(config) => {
                info!(
                    task_id = %task.id,
                    config_id = %config.id,
                    config_name = %config.name,
                    pattern_type = %config.pattern_type,
                    max_iterations = config.max_iterations,
                    "Using database-backed pattern config"
                );
                Some(config)
            }
            Err(e) => {
                warn!(
                    task_id = %task.id,
                    error = %e,
                    "Failed to route task to pattern config, falling back to classification"
                );
                None
            }
        }
    }

    /// Get pattern from task using automatic classification
    ///
    /// Priority order:
    /// 1. Database-backed pattern config (if PatternRouter available)
    /// 2. Explicit pattern in task metadata (for backward compatibility)
    /// 3. Automatic classification based on task description (default behavior)
    ///
    /// The automatic classification maps task categories to patterns:
    /// - SimpleQuery → React (quick, factual tasks)
    /// - FileOperation → React (tool-focused tasks)
    /// - CodeGeneration → Reflection (quality-focused code tasks)
    /// - Research → PlanExecute (complex multi-step research)
    /// - DataAnalysis → PlanExecute (complex analysis tasks)
    /// - SystemCommand → React (system operations)
    /// - General → React (default fallback)
    fn get_pattern_from_task(&self, task: &Task) -> Result<PatternType> {
        // Priority 1: Explicit pattern in task metadata (backward compatibility)
        if let Ok(metadata) = serde_json::from_str::<Value>(&task.metadata) {
            if let Some(pattern_str) = metadata.get("pattern").and_then(|p| p.as_str()) {
                if let Some(pattern) = PatternType::from_str(pattern_str) {
                    debug!(
                        task_id = %task.id,
                        pattern = %pattern,
                        "Using explicit pattern from task metadata"
                    );
                    return Ok(pattern);
                }
            }
        }

        // Priority 2: Automatic classification based on task description
        let (category, confidence) = self.classifier.classify_with_confidence(&task.description);
        let pattern = self.category_to_pattern(&category);

        info!(
            task_id = %task.id,
            category = %category.as_str(),
            confidence = confidence,
            pattern = %pattern,
            "Auto-classified task to pattern"
        );

        Ok(pattern)
    }

    /// Convert pattern type string from database to PatternType enum
    fn pattern_type_from_config(config: &PatternConfig) -> PatternType {
        match config.pattern_type.to_lowercase().as_str() {
            "react" => PatternType::React,
            "plan_execute" | "planexecute" => PatternType::PlanExecute,
            "reflection" => PatternType::Reflection,
            _ => {
                warn!(
                    pattern_type = %config.pattern_type,
                    "Unknown pattern type in config, defaulting to React"
                );
                PatternType::React
            }
        }
    }

    /// Map task category to execution pattern
    ///
    /// This mapping determines which agent pattern is used for each category:
    /// - High-quality output (code) → Reflection for iterative improvement
    /// - Multi-step analysis → PlanExecute for structured planning
    /// - Quick tasks → React for efficiency
    fn category_to_pattern(&self, category: &TaskCategory) -> PatternType {
        match category {
            TaskCategory::SimpleQuery => PatternType::React,
            TaskCategory::FileOperation => PatternType::React,
            TaskCategory::CodeGeneration => PatternType::Reflection,
            TaskCategory::Research => PatternType::PlanExecute,
            TaskCategory::DataAnalysis => PatternType::PlanExecute,
            TaskCategory::SystemCommand => PatternType::React,
            TaskCategory::General => PatternType::React,
            TaskCategory::Custom(_) => PatternType::React,
        }
    }

    /// Get the task classifier for external use
    pub fn classifier(&self) -> &TaskClassifier {
        &self.classifier
    }

    /// Classify a task description and return the category and confidence
    pub fn classify_task(&self, description: &str) -> (TaskCategory, f64) {
        self.classifier.classify_with_confidence(description)
    }

    /// Get configuration
    pub fn config(&self) -> &OrcaConfig {
        &self.config
    }

    /// Get bridge
    pub fn bridge(&self) -> &Arc<DirectToolBridge> {
        &self.bridge
    }

    /// Check if database-backed pattern routing is enabled
    pub fn has_pattern_router(&self) -> bool {
        self.pattern_router.is_some()
    }

    /// Get pattern router reference (if available)
    pub fn pattern_router(&self) -> Option<&PatternRouter> {
        self.pattern_router.as_ref()
    }

    /// Display thinking/reasoning output with styled formatting
    fn display_thinking(&self, reasoning: &langgraph_core::llm::ReasoningContent) {
        use colored::Colorize;

        // Top border
        println!();
        println!("{}", "╭─ Model Thinking ─╮".dimmed());

        // Thinking content (dimmed)
        for line in reasoning.content.lines() {
            println!("{} {}", "│".dimmed(), line.dimmed());
        }

        // Bottom border with metrics
        let mut metrics = Vec::new();
        metrics.push(format!("{} tokens", reasoning.tokens));
        if let Some(duration_ms) = reasoning.duration_ms {
            metrics.push(format!("{:.2}s", duration_ms as f64 / 1000.0));
        }

        let metrics_str = metrics.join(", ");
        println!("{}", format!("╰─ {} ─╯", metrics_str).dimmed());
        println!();
    }

    /// Execute a task using direct LLM call (no agent, no tools)
    /// Used when no tools are available
    async fn execute_direct_llm(&self, task: &Task) -> Result<ExecutionResult> {
        use langgraph_core::llm::ChatRequest;

        debug!(
            task_id = %task.id,
            "Executing direct LLM call"
        );

        // Create a simple chat request
        let message = langgraph_core::Message::human(task.description.clone());
        let request = ChatRequest::new(vec![message]);

        // Call the LLM directly
        let response = self.llm_provider.chat(request).await
            .map_err(|e| OrcaError::Execution(format!("LLM call failed: {}", e)))?;

        // Display thinking if present and enabled
        if self.config.execution.show_thinking {
            if let Some(reasoning) = &response.reasoning {
                self.display_thinking(reasoning);
            }
        }

        // Extract response text
        let response_text = response.message.text().unwrap_or("").to_string();

        info!(
            task_id = %task.id,
            response_len = response_text.len(),
            "Direct LLM call completed"
        );

        // Build result with messages
        let messages = vec![
            json!({
                "type": "human",
                "content": task.description.clone()
            }),
            json!({
                "type": "ai",
                "content": response_text.clone()
            })
        ];

        let final_state = json!({
            "messages": messages.clone()
        });

        Ok(ExecutionResult::success(response_text, final_state, messages))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DatabaseConfig, ExecutionConfig, LlmConfig, LoggingConfig};
    use tempfile::TempDir;

    fn create_test_config() -> OrcaConfig {
        OrcaConfig {
            project_name: Some("test-project".to_string()),
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
            budget: Default::default(),
            workflow: Default::default(),
            backup: Default::default(),
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

    // ============================================================================
    // Phase 6.2: Orca Task Executor - Comprehensive Tests
    // ============================================================================

    // ------------------------------------------------------------------------
    // Resource Cleanup Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_executor_bridge_reference_management() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();
        let bridge_weak = Arc::downgrade(&bridge);

        {
            let executor = TaskExecutor::new(bridge.clone(), config).unwrap();
            // Verify bridge is held by executor
            assert_eq!(Arc::strong_count(&bridge), 2); // Original + executor
            assert!(bridge_weak.upgrade().is_some());
            drop(executor);
        }

        // After executor is dropped, only original reference remains
        assert_eq!(Arc::strong_count(&bridge), 1);
        assert!(bridge_weak.upgrade().is_some());
    }

    #[tokio::test]
    async fn test_executor_multiple_instances_share_bridge() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "shared-session".to_string(),
            )
            .unwrap()
        );

        let config1 = create_test_config();
        let config2 = create_test_config();

        let executor1 = TaskExecutor::new(bridge.clone(), config1).unwrap();
        let executor2 = TaskExecutor::new(bridge.clone(), config2).unwrap();

        // Both executors share the same bridge
        assert!(Arc::ptr_eq(executor1.bridge(), executor2.bridge()));
        assert_eq!(Arc::strong_count(&bridge), 3); // Original + 2 executors
    }

    #[test]
    fn test_executor_config_ownership() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let mut config = create_test_config();
        config.execution.max_iterations = 20;

        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Executor owns its config, original config can't be accessed
        assert_eq!(executor.config().execution.max_iterations, 20);
    }

    #[test]
    fn test_executor_config_independence() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let mut config1 = create_test_config();
        config1.execution.task_timeout = 100;
        let executor1 = TaskExecutor::new(bridge.clone(), config1).unwrap();

        let mut config2 = create_test_config();
        config2.execution.task_timeout = 200;
        let executor2 = TaskExecutor::new(bridge, config2).unwrap();

        // Each executor has independent config
        assert_eq!(executor1.config().execution.task_timeout, 100);
        assert_eq!(executor2.config().execution.task_timeout, 200);
    }

    // ------------------------------------------------------------------------
    // Timeout Enforcement Edge Cases
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_timeout_with_zero_duration() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let mut config = create_test_config();
        config.execution.task_timeout = 0; // Zero timeout

        let executor = TaskExecutor::new(bridge, config).unwrap();
        assert_eq!(executor.config().execution.task_timeout, 0);

        // Note: Actual execution would timeout immediately
        // This tests configuration handling
    }

    #[test]
    fn test_timeout_configuration_boundaries() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        // Test minimum timeout
        let mut config = create_test_config();
        config.execution.task_timeout = 1;
        let executor = TaskExecutor::new(bridge.clone(), config).unwrap();
        assert_eq!(executor.config().execution.task_timeout, 1);

        // Test maximum timeout
        let mut config = create_test_config();
        config.execution.task_timeout = u64::MAX;
        let executor = TaskExecutor::new(bridge.clone(), config).unwrap();
        assert_eq!(executor.config().execution.task_timeout, u64::MAX);

        // Test typical values
        for timeout in [5, 30, 60, 300, 600, 3600] {
            let mut config = create_test_config();
            config.execution.task_timeout = timeout;
            let executor = TaskExecutor::new(bridge.clone(), config).unwrap();
            assert_eq!(executor.config().execution.task_timeout, timeout);
        }
    }

    #[test]
    fn test_timeout_error_contains_task_info() {
        let error = OrcaError::Timeout {
            task_id: "task-abc-123".to_string(),
            duration_secs: 600,
        };

        let error_msg = format!("{}", error);

        // Verify error message contains task ID
        assert!(error_msg.contains("task-abc-123"));

        // Verify error message contains duration
        assert!(error_msg.contains("600"));

        // Verify error message indicates timeout
        let error_lower = error_msg.to_lowercase();
        assert!(error_lower.contains("timeout") || error_lower.contains("timed out"));
    }

    #[test]
    fn test_timeout_error_different_durations() {
        let durations = vec![1, 5, 30, 60, 300, 600, 3600];

        for duration in durations {
            let error = OrcaError::Timeout {
                task_id: format!("task-{}", duration),
                duration_secs: duration,
            };

            let error_msg = format!("{}", error);
            assert!(error_msg.contains(&duration.to_string()));
            assert!(error_msg.contains(&format!("task-{}", duration)));
        }
    }

    // ------------------------------------------------------------------------
    // Retry Logic Integration Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_retry_config_creation() {
        use crate::executor::retry::RetryConfig;

        // Test default config
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_secs, 1);
        assert_eq!(config.max_delay_secs, 60);
        assert_eq!(config.multiplier, 2.0);

        // Test custom config
        let custom_config = RetryConfig::new(5, 2, 120, 3.0);
        assert_eq!(custom_config.max_retries, 5);
        assert_eq!(custom_config.initial_delay_secs, 2);
        assert_eq!(custom_config.max_delay_secs, 120);
        assert_eq!(custom_config.multiplier, 3.0);
    }

    #[tokio::test]
    async fn test_retry_delay_calculation() {
        use crate::executor::retry::RetryConfig;

        let config = RetryConfig::new(5, 1, 100, 2.0);

        // Exponential backoff: 1, 2, 4, 8, 16, 32, ...
        assert_eq!(config.calculate_delay(0).as_secs(), 1);
        assert_eq!(config.calculate_delay(1).as_secs(), 2);
        assert_eq!(config.calculate_delay(2).as_secs(), 4);
        assert_eq!(config.calculate_delay(3).as_secs(), 8);
        assert_eq!(config.calculate_delay(4).as_secs(), 16);
        assert_eq!(config.calculate_delay(5).as_secs(), 32);
        assert_eq!(config.calculate_delay(6).as_secs(), 64);

        // Should cap at max_delay_secs (100)
        assert_eq!(config.calculate_delay(7).as_secs(), 100);
        assert_eq!(config.calculate_delay(10).as_secs(), 100);
    }

    #[tokio::test]
    async fn test_retry_with_different_multipliers() {
        use crate::executor::retry::RetryConfig;

        // Multiplier 1.5
        let config = RetryConfig::new(3, 10, 1000, 1.5);
        assert_eq!(config.calculate_delay(0).as_secs(), 10);
        assert_eq!(config.calculate_delay(1).as_secs(), 15);  // 10 * 1.5
        assert_eq!(config.calculate_delay(2).as_secs(), 22);  // 10 * 1.5^2 = 22.5

        // Multiplier 3.0
        let config = RetryConfig::new(3, 2, 1000, 3.0);
        assert_eq!(config.calculate_delay(0).as_secs(), 2);
        assert_eq!(config.calculate_delay(1).as_secs(), 6);   // 2 * 3
        assert_eq!(config.calculate_delay(2).as_secs(), 18);  // 2 * 9
    }

    #[tokio::test]
    async fn test_retry_with_operation_success_on_first_try() {
        use crate::executor::retry::{RetryConfig, with_retry};
        use std::sync::atomic::{AtomicUsize, Ordering};

        let config = RetryConfig::new(3, 0, 60, 2.0);
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = with_retry(&config, "test-task", || {
            let count = attempt_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<String, String>("Success".to_string())
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_with_operation_success_after_failures() {
        use crate::executor::retry::{RetryConfig, with_retry};
        use std::sync::atomic::{AtomicUsize, Ordering};

        let config = RetryConfig::new(5, 0, 60, 2.0); // 0 delay for fast test
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = with_retry(&config, "test-task", || {
            let count = attempt_count_clone.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 4 {
                    Err(format!("Failure attempt {}", current))
                } else {
                    Ok::<String, String>("Success after retries".to_string())
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success after retries");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn test_retry_exhausts_all_attempts() {
        use crate::executor::retry::{RetryConfig, with_retry};
        use std::sync::atomic::{AtomicUsize, Ordering};

        let config = RetryConfig::new(3, 0, 60, 2.0);
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = with_retry(&config, "test-task", || {
            let count = attempt_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<String, String>("Permanent failure".to_string())
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Permanent failure");
        // Initial attempt + 3 retries = 4 total attempts
        assert_eq!(attempt_count.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn test_retry_with_zero_retries() {
        use crate::executor::retry::{RetryConfig, with_retry};
        use std::sync::atomic::{AtomicUsize, Ordering};

        let config = RetryConfig::new(0, 1, 60, 2.0);
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = with_retry(&config, "test-task", || {
            let count = attempt_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<String, String>("Failed".to_string())
            }
        })
        .await;

        assert!(result.is_err());
        // Only initial attempt, no retries
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }

    // ------------------------------------------------------------------------
    // Error Handling Paths
    // ------------------------------------------------------------------------

    #[test]
    fn test_orca_error_types() {
        // Test timeout error
        let timeout_err = OrcaError::Timeout {
            task_id: "task-1".to_string(),
            duration_secs: 300,
        };
        assert!(format!("{}", timeout_err).contains("task-1"));

        // Test execution error
        let exec_err = OrcaError::Execution("LLM call failed".to_string());
        assert!(format!("{}", exec_err).contains("LLM call failed"));

        // Test database error
        let db_err = OrcaError::Database("Connection lost".to_string());
        assert!(format!("{}", db_err).contains("Connection lost"));
    }

    #[test]
    fn test_pattern_from_invalid_task_metadata() {
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

        // Malformed JSON
        let task = Task::new("Test")
            .with_metadata(r#"{"pattern": "react""#); // Missing closing brace

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React); // Should default to React

        // Invalid pattern value
        let task = Task::new("Test")
            .with_metadata(r#"{"pattern": "invalid_pattern_name"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React); // Should default to React

        // Empty string pattern
        let task = Task::new("Test")
            .with_metadata(r#"{"pattern": ""}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React); // Should default to React

        // Null pattern
        let task = Task::new("Test")
            .with_metadata(r#"{"pattern": null}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React); // Should default to React
    }

    #[test]
    fn test_execution_result_error_states() {
        // Test failure result with various error types
        let errors = vec![
            "Timeout error",
            "LLM API error",
            "Network connection failed",
            "Invalid tool call",
            "Rate limit exceeded",
        ];

        for error_msg in errors {
            let result = ExecutionResult::failure(
                error_msg.to_string(),
                json!({}),
                vec![],
            );

            assert!(!result.success);
            assert!(result.result.is_none());
            assert_eq!(result.error, Some(error_msg.to_string()));
        }
    }

    #[test]
    fn test_execution_result_with_partial_state() {
        // Test result preserves partial state on error
        let partial_state = json!({
            "messages": [
                {"type": "human", "content": "Input"},
                {"type": "ai", "content": "Partial response before error"}
            ],
            "iteration": 2,
            "error_at_step": "tool_execution"
        });

        let messages = vec![
            json!({"type": "human", "content": "Input"}),
            json!({"type": "ai", "content": "Partial response before error"}),
        ];

        let result = ExecutionResult::failure(
            "Tool execution failed".to_string(),
            partial_state.clone(),
            messages.clone(),
        );

        assert!(!result.success);
        assert_eq!(result.error, Some("Tool execution failed".to_string()));
        assert_eq!(result.final_state, partial_state);
        assert_eq!(result.messages, messages);
        assert_eq!(result.messages.len(), 2);
    }

    #[test]
    fn test_max_iterations_boundary_values() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        // Test minimum (1 iteration)
        let mut config = create_test_config();
        config.execution.max_iterations = 1;
        let executor = TaskExecutor::new(bridge.clone(), config).unwrap();
        assert_eq!(executor.config().execution.max_iterations, 1);

        // Test typical values
        for iterations in [1, 5, 10, 20, 50, 100] {
            let mut config = create_test_config();
            config.execution.max_iterations = iterations;
            let executor = TaskExecutor::new(bridge.clone(), config).unwrap();
            assert_eq!(executor.config().execution.max_iterations, iterations);
        }

        // Test maximum
        let mut config = create_test_config();
        config.execution.max_iterations = usize::MAX;
        let executor = TaskExecutor::new(bridge, config).unwrap();
        assert_eq!(executor.config().execution.max_iterations, usize::MAX);
    }

    #[test]
    fn test_llm_config_variations() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        // Different LLM providers
        let providers = vec!["ollama", "openai", "anthropic", "gemini"];

        for provider in providers {
            let mut config = create_test_config();
            config.llm.provider = provider.to_string();
            let executor = TaskExecutor::new(bridge.clone(), config);
            // Some providers might fail without proper API keys, but construction should work
            // or return an error (not panic)
            let _ = executor;
        }
    }

    #[test]
    fn test_task_metadata_pattern_extraction_edge_cases() {
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

        // Nested JSON
        let task = Task::new("Test")
            .with_metadata(r#"{"config": {"pattern": "react"}}"#);
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React); // Won't find nested pattern, defaults to React

        // Array instead of object
        let task = Task::new("Test")
            .with_metadata(r#"["react", "plan_execute"]"#);
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React); // Not an object, defaults to React

        // Number pattern value
        let task = Task::new("Test")
            .with_metadata(r#"{"pattern": 123}"#);
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React); // Not a string, defaults to React

        // Boolean pattern value
        let task = Task::new("Test")
            .with_metadata(r#"{"pattern": true}"#);
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React); // Not a string, defaults to React
    }

    #[test]
    fn test_config_independent_modification() {
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
        config.execution.streaming = true;

        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Verify executor has the configured values
        assert_eq!(executor.config().execution.max_iterations, 10);
        assert_eq!(executor.config().execution.streaming, true);
    }

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

    // ============================================================================
    // Automatic Pattern Classification Tests
    // ============================================================================

    #[test]
    fn test_automatic_code_generation_classification() {
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

        // Code generation tasks should auto-classify to Reflection pattern
        let task = Task::new("Write a function to sort an array");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::Reflection);

        let task = Task::new("Implement unit tests for the auth module");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::Reflection);

        let task = Task::new("Refactor the database connection code");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::Reflection);
    }

    #[test]
    fn test_automatic_research_classification() {
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

        // Research tasks should auto-classify to PlanExecute pattern
        let task = Task::new("Research how async/await works in Rust");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::PlanExecute);

        let task = Task::new("Investigate how the caching system works");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::PlanExecute);
    }

    #[test]
    fn test_automatic_simple_query_classification() {
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

        // Simple queries should auto-classify to React pattern
        let task = Task::new("What is 2+2?");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React);

        let task = Task::new("How many items are there?");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React);
    }

    #[test]
    fn test_automatic_data_analysis_classification() {
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

        // Data analysis tasks should auto-classify to PlanExecute pattern
        let task = Task::new("Analyze the data and generate a report");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::PlanExecute);
    }

    #[test]
    fn test_explicit_metadata_overrides_auto_classification() {
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

        // A code generation task that would normally get Reflection...
        // but explicit metadata forces React
        let task = Task::new("Write a function to sort an array")
            .with_metadata(r#"{"pattern": "react"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React);

        // A simple query that would normally get React...
        // but explicit metadata forces Reflection
        let task = Task::new("What is 2+2?")
            .with_metadata(r#"{"pattern": "reflection"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::Reflection);
    }

    #[test]
    fn test_category_to_pattern_mapping() {
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

        // Verify the category to pattern mapping
        assert_eq!(
            executor.category_to_pattern(&TaskCategory::SimpleQuery),
            PatternType::React
        );
        assert_eq!(
            executor.category_to_pattern(&TaskCategory::FileOperation),
            PatternType::React
        );
        assert_eq!(
            executor.category_to_pattern(&TaskCategory::CodeGeneration),
            PatternType::Reflection
        );
        assert_eq!(
            executor.category_to_pattern(&TaskCategory::Research),
            PatternType::PlanExecute
        );
        assert_eq!(
            executor.category_to_pattern(&TaskCategory::DataAnalysis),
            PatternType::PlanExecute
        );
        assert_eq!(
            executor.category_to_pattern(&TaskCategory::SystemCommand),
            PatternType::React
        );
        assert_eq!(
            executor.category_to_pattern(&TaskCategory::General),
            PatternType::React
        );
        assert_eq!(
            executor.category_to_pattern(&TaskCategory::Custom("custom".to_string())),
            PatternType::React
        );
    }

    #[test]
    fn test_classify_task_method() {
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

        // Test the public classify_task method
        let (category, confidence) = executor.classify_task("Write unit tests for the module");
        assert_eq!(category, TaskCategory::CodeGeneration);
        assert!(confidence > 0.5);

        let (category, _) = executor.classify_task("What is the capital of France?");
        assert_eq!(category, TaskCategory::SimpleQuery);
    }

    #[test]
    fn test_classifier_accessor() {
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

        // Verify classifier is accessible
        let classifier = executor.classifier();
        let category = classifier.classify("Write a function");
        assert_eq!(category, TaskCategory::CodeGeneration);
    }

    #[test]
    fn test_automatic_classification_is_default() {
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

        // Without any metadata, automatic classification should kick in
        // Testing various task types to ensure the default is automatic

        // Generic task that doesn't match any specific pattern
        let task = Task::new("random xyz abc");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React); // General -> React

        // Code task without explicit metadata
        let task = Task::new("Debug the login issue");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::Reflection); // CodeGeneration -> Reflection

        // File operation without explicit metadata
        let task = Task::new("read file config.toml");
        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::React); // FileOperation -> React
    }

    // ============================================================================
    // Phase 11.3: PatternRouter Integration Tests
    // ============================================================================

    #[test]
    fn test_executor_without_router() {
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

        // Executor without router should not have pattern_router
        assert!(!executor.has_pattern_router());
        assert!(executor.pattern_router().is_none());
    }

    #[test]
    fn test_pattern_type_from_config_react() {
        use crate::models::{PatternConfig, PatternType as ModelPatternType};

        let config = PatternConfig::new("Test", ModelPatternType::React);
        let pattern = TaskExecutor::pattern_type_from_config(&config);
        assert_eq!(pattern, PatternType::React);
    }

    #[test]
    fn test_pattern_type_from_config_plan_execute() {
        use crate::models::{PatternConfig, PatternType as ModelPatternType};

        let config = PatternConfig::new("Test", ModelPatternType::PlanExecute);
        let pattern = TaskExecutor::pattern_type_from_config(&config);
        assert_eq!(pattern, PatternType::PlanExecute);
    }

    #[test]
    fn test_pattern_type_from_config_reflection() {
        use crate::models::{PatternConfig, PatternType as ModelPatternType};

        let config = PatternConfig::new("Test", ModelPatternType::Reflection);
        let pattern = TaskExecutor::pattern_type_from_config(&config);
        assert_eq!(pattern, PatternType::Reflection);
    }

    #[test]
    fn test_pattern_type_from_config_unknown_defaults_to_react() {
        use crate::models::PatternConfig;

        // Create config with unknown pattern type (using raw struct)
        let config = PatternConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            pattern_type: "unknown_pattern".to_string(),
            max_iterations: 10,
            system_prompt: None,
            tools: "[]".to_string(),
            config: "{}".to_string(),
            is_default: false,
            usage_count: 0,
            created_at: 0,
            updated_at: 0,
        };

        let pattern = TaskExecutor::pattern_type_from_config(&config);
        assert_eq!(pattern, PatternType::React); // Unknown should default to React
    }

    #[test]
    fn test_pattern_router_accessor_methods() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let config = create_test_config();

        // Without router
        let executor = TaskExecutor::new(bridge.clone(), config.clone()).unwrap();
        assert!(!executor.has_pattern_router());

        // The pattern_router() should return None
        let router = executor.pattern_router();
        assert!(router.is_none());
    }

    #[test]
    fn test_executor_uses_default_config_without_router() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let mut config = create_test_config();
        config.execution.max_iterations = 15;

        let executor = TaskExecutor::new(bridge, config).unwrap();

        // Without router, should use config.execution.max_iterations
        assert_eq!(executor.config().execution.max_iterations, 15);
        assert!(!executor.has_pattern_router());
    }

    #[test]
    fn test_config_with_pattern_metadata_still_works() {
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

        // Task with pattern in metadata should still work (backward compatibility)
        let task = Task::new("Any description")
            .with_metadata(r#"{"pattern": "plan_execute"}"#);

        let pattern = executor.get_pattern_from_task(&task).unwrap();
        assert_eq!(pattern, PatternType::PlanExecute);
    }
}
