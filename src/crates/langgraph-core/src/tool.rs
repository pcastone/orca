//! Tool execution framework for agent workflows and LLM integration
//!
//! This module provides the infrastructure for defining, registering, and executing tools
//! that AI agents can call. Tools enable LLMs to interact with external systems, databases,
//! APIs, and perform computations, forming the foundation for ReAct-style agents and
//! function calling patterns.
//!
//! # Overview
//!
//! The tool system provides:
//!
//! - **Tool Definition** - Create tools from async functions
//! - **Runtime Context** - Inject state, store, and streaming capabilities
//! - **Tool Registry** - Register and discover available tools
//! - **Parallel Execution** - Execute multiple tool calls concurrently
//! - **Error Handling** - Robust error tracking and recovery
//! - **Validation** - Input validation and type checking
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  LLM Agent Node                                          │
//! │  • Calls LLM with available tools                        │
//! │  • LLM returns ToolCall instructions                     │
//! └──────────────────┬──────────────────────────────────────┘
//!                    │
//!                    ↓
//! ┌─────────────────────────────────────────────────────────┐
//! │  Tool Execution Node                                     │
//! │                                                          │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
//! │  │ ToolCall 1   │  │ ToolCall 2   │  │ ToolCall 3   │ │
//! │  │ search(...)  │  │ calculate()  │  │ fetch(...)   │ │
//! │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘ │
//! │         │                 │                 │          │
//! │         ↓                 ↓                 ↓          │
//! │  ┌──────────────────────────────────────────────────┐ │
//! │  │  ToolRegistry                                     │ │
//! │  │  • Lookup tool by name                           │ │
//! │  │  • Execute with ToolRuntime context              │ │
//! │  │  • Handle errors                                 │ │
//! │  └──────┬───────────────────────────────────────────┘ │
//! │         │                                              │
//! │         ↓                                              │
//! │  [ToolResult, ToolResult, ToolResult]                 │
//! └──────────────────┬──────────────────────────────────────┘
//!                    │
//!                    ↓
//! ┌─────────────────────────────────────────────────────────┐
//! │  Tool Message Node                                       │
//! │  • Convert results to Message::tool(...)                │
//! │  • Append to conversation                               │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Types
//!
//! - [`Tool`] - Tool definition with executor function
//! - [`ToolCall`] - Request to execute a tool
//! - [`ToolRuntime`] - Runtime context for tools
//! - [`ToolRegistry`] - Collection of available tools
//! - [`ToolError`] - Tool execution errors
//! - [`ToolResult`] - Tool execution result type
//!
//! # Quick Start
//!
//! ## Defining Tools
//!
//! Create tools from async functions:
//!
//! ```rust,ignore
//! use langgraph_core::tool::{Tool, ToolResult};
//! use serde_json::{json, Value};
//!
//! // Simple tool
//! async fn search_web(query: String) -> ToolResult {
//!     let results = perform_search(&query).await?;
//!     Ok(json!({"results": results}))
//! }
//!
//! // Tool with runtime context
//! use langgraph_core::tool::ToolRuntime;
//!
//! async fn fetch_from_db(
//!     id: String,
//!     runtime: Option<ToolRuntime>
//! ) -> ToolResult {
//!     // Access store from runtime
//!     if let Some(rt) = runtime {
//!         if let Some(store) = rt.store {
//!             return store.get(&id).await
//!                 .map_err(|e| ToolError::ExecutionFailed {
//!                     tool: "fetch_from_db".to_string(),
//!                     error: e.to_string()
//!                 });
//!         }
//!     }
//!
//!     Err(ToolError::ExecutionFailed {
//!         tool: "fetch_from_db".to_string(),
//!         error: "No store available".to_string()
//!     })
//! }
//! ```
//!
//! ## Registering Tools
//!
//! Build a tool registry:
//!
//! ```rust,ignore
//! use langgraph_core::tool::{Tool, ToolRegistry};
//!
//! let mut registry = ToolRegistry::new();
//!
//! registry.register(Tool::new(
//!     "search",
//!     "Search the web for information",
//!     search_web_executor,
//!     None, // optional schema
//! ));
//!
//! registry.register(Tool::new(
//!     "calculator",
//!     "Perform mathematical calculations",
//!     calculator_executor,
//!     Some(calculator_schema()),
//! ));
//! ```
//!
//! # Common Patterns
//!
//! ## ReAct Agent with Tools
//!
//! Build an agent that reasons and acts with tools:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, messages::Message};
//!
//! let mut graph = StateGraph::new();
//!
//! // Agent decides what to do
//! graph.add_node("agent", |state| {
//!     Box::pin(async move {
//!         let messages = state["messages"].as_array().unwrap();
//!
//!         // Call LLM with tool definitions
//!         let response = call_llm_with_tools(messages, &tool_registry).await?;
//!
//!         Ok(json!({"messages": [response]}))
//!     })
//! });
//!
//! // Execute tools
//! graph.add_node("tools", |state| {
//!     Box::pin(async move {
//!         let messages = state["messages"].as_array().unwrap();
//!         let last_msg = messages.last().unwrap();
//!
//!         // Execute all tool calls in parallel
//!         let mut tool_messages = vec![];
//!         for tool_call in &last_msg.tool_calls {
//!             let result = execute_tool_call(tool_call, &tool_registry).await?;
//!             tool_messages.push(Message::tool(result, tool_call.id.clone()));
//!         }
//!
//!         Ok(json!({"messages": tool_messages}))
//!     })
//! });
//!
//! // Conditional routing
//! graph.add_conditional_edge("agent", |state| {
//!     let messages = state["messages"].as_array().unwrap();
//!     let last = messages.last().unwrap();
//!
//!     if !last.tool_calls.is_empty() {
//!         vec!["tools"]  // Execute tools
//!     } else {
//!         vec!["__end__"]  // Done
//!     }
//! });
//!
//! graph.add_edge("tools", "agent"); // Loop back for next action
//! ```
//!
//! ## Parallel Tool Execution
//!
//! Execute multiple tools concurrently:
//!
//! ```rust,ignore
//! use futures::future::join_all;
//!
//! async fn execute_tools_parallel(
//!     tool_calls: Vec<ToolCall>,
//!     registry: &ToolRegistry,
//!     runtime: ToolRuntime,
//! ) -> Vec<ToolResult> {
//!     let futures = tool_calls.into_iter().map(|call| {
//!         execute_single_tool(call, registry, runtime.clone())
//!     });
//!
//!     join_all(futures).await
//! }
//! ```
//!
//! ## Tool with State Access
//!
//! Access graph state within tools:
//!
//! ```rust,ignore
//! async fn personalized_search(
//!     query: String,
//!     runtime: Option<ToolRuntime>
//! ) -> ToolResult {
//!     let user_prefs = if let Some(rt) = runtime {
//!         rt.state.get("user_preferences").cloned()
//!     } else {
//!         None
//!     };
//!
//!     let results = search_with_preferences(&query, user_prefs).await?;
//!     Ok(json!({"results": results}))
//! }
//! ```
//!
//! ## Tool with Streaming
//!
//! Stream intermediate results:
//!
//! ```rust,ignore
//! async fn long_running_analysis(
//!     data: Value,
//!     runtime: Option<ToolRuntime>
//! ) -> ToolResult {
//!     if let Some(rt) = runtime {
//!         if let Some(writer) = rt.stream_writer {
//!             // Stream progress updates
//!             writer.send(json!({"progress": 0.25})).await.ok();
//!
//!             // ... process ...
//!
//!             writer.send(json!({"progress": 0.75})).await.ok();
//!         }
//!     }
//!
//!     Ok(json!({"complete": true}))
//! }
//! ```
//!
//! ## Error Handling
//!
//! Handle tool errors gracefully:
//!
//! ```rust,ignore
//! async fn execute_with_fallback(
//!     tool_call: &ToolCall,
//!     registry: &ToolRegistry,
//! ) -> Message {
//!     match execute_tool(tool_call, registry).await {
//!         Ok(result) => Message::tool(result, tool_call.id.clone()),
//!         Err(e) => Message::tool(
//!             json!({"error": e.to_string()}),
//!             tool_call.id.clone()
//!         )
//!     }
//! }
//! ```
//!
//! # Runtime Context
//!
//! Tools can access execution context via [`ToolRuntime`]:
//!
//! ```rust,ignore
//! let runtime = ToolRuntime::new(state.clone())
//!     .with_tool_call_id(tool_call.id.clone())
//!     .with_store(store.clone())
//!     .with_stream_writer(writer)
//!     .with_config("user_id".to_string(), json!("user-123"));
//!
//! let result = tool.execute(args, Some(runtime)).await?;
//! ```
//!
//! ## Available Context
//!
//! | Field | Type | Purpose |
//! |-------|------|---------|
//! | `state` | `Value` | Current graph state |
//! | `store` | `Arc<dyn Store>` | Persistent data storage |
//! | `stream_writer` | `StreamWriter` | Emit custom events |
//! | `tool_call_id` | `String` | Track this tool call |
//! | `config` | `HashMap` | Custom configuration |
//!
//! # Tool Validation
//!
//! Validate tool inputs against schemas:
//!
//! ```rust,ignore
//! use schemars::JsonSchema;
//!
//! #[derive(JsonSchema)]
//! struct SearchParams {
//!     query: String,
//!     limit: Option<u32>,
//! }
//!
//! let schema = schemars::schema_for!(SearchParams);
//!
//! let tool = Tool::new(
//!     "search",
//!     "Search with validation",
//!     search_executor,
//!     Some(serde_json::to_value(schema)?),
//! );
//!
//! // Validate before execution
//! tool.validate_args(&args)?;
//! ```
//!
//! # Performance Considerations
//!
//! - **Parallel Execution**: Use `join_all` for concurrent tool calls
//! - **Timeouts**: Implement timeouts for long-running tools
//! - **Caching**: Cache tool results where appropriate
//! - **Connection Pooling**: Reuse database/API connections
//! - **Async All The Way**: Keep tools fully async for best performance
//!
//! # Best Practices
//!
//! 1. **Idempotency**: Make tools idempotent where possible
//! 2. **Clear Errors**: Return descriptive error messages for LLM to understand
//! 3. **Timeouts**: Set reasonable timeouts to prevent hangs
//! 4. **Validation**: Validate inputs before expensive operations
//! 5. **Logging**: Log tool executions for debugging
//! 6. **Streaming**: Use streaming for long-running tools
//! 7. **Documentation**: Provide clear descriptions for LLM tool selection
//!
//! # Comparison with Python LangGraph
//!
//! | Python | Rust | Notes |
//! |--------|------|-------|
//! | `@tool` decorator | `Tool::new(...)` | Explicit registration |
//! | `def tool(arg: str)` | `async fn tool(arg: String)` | Async in Rust |
//! | Tool dict | `ToolCall` struct | Type-safe |
//! | ToolMessage | `Message::tool(...)` | Same concept |
//! | Parallel with `asyncio` | Parallel with `join_all` | Similar patterns |
//!
//! # See Also
//!
//! - [`messages`](crate::messages) - Message types including tool messages
//! - [`MessageGraph`](crate::MessageGraph) - Specialized graph for tool agents
//! - [`Store`](crate::store) - Persistent storage for tools
//! - [`prebuilt`](crate::prebuilt) - Pre-built ReAct agent pattern

use crate::runtime::{Runtime, StreamWriter};
use crate::store::Store;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;

/// Tool execution result
pub type ToolResult = Result<Value, ToolError>;

/// Future type for async tool execution
pub type ToolFuture = Pin<Box<dyn Future<Output = ToolResult> + Send>>;

/// Tool executor function type
pub type ToolExecutor = Arc<dyn Fn(Value, Option<ToolRuntime>) -> ToolFuture + Send + Sync>;

/// Errors that can occur during tool execution
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ToolError {
    /// Tool not found in registry
    #[error("Tool '{0}' not found. Available tools: {1}")]
    ToolNotFound(String, String),

    /// Invalid tool arguments
    #[error("Invalid arguments for tool '{tool}': {error}")]
    InvalidArguments { tool: String, error: String },

    /// Tool execution failed
    #[error("Tool '{tool}' execution failed: {error}")]
    ExecutionFailed { tool: String, error: String },

    /// Tool invocation error
    #[error("Tool '{tool}' invocation error: {error}")]
    InvocationFailed { tool: String, error: String },

    /// Validation error
    #[error("Validation error for tool '{tool}': {error}")]
    ValidationError { tool: String, error: String },
}

/// Runtime context bundle for tool execution
///
/// This struct bundles together runtime context that can be injected into tools:
/// - Current graph state
/// - Store for persistent data
/// - Stream writer for custom events
/// - Tool call ID for tracking
/// - Configuration metadata
#[derive(Clone)]
pub struct ToolRuntime {
    /// Current graph state
    pub state: Value,

    /// Tool call ID for tracking
    pub tool_call_id: Option<String>,

    /// Store for persistent data (if available)
    pub store: Option<Arc<dyn Store>>,

    /// Stream writer for custom events (if available)
    pub stream_writer: Option<StreamWriter>,

    /// Configuration metadata
    pub config: HashMap<String, Value>,

    /// LangGraph runtime context (execution context, step tracking, etc.)
    pub runtime: Option<Runtime>,
}

impl ToolRuntime {
    /// Create a new tool runtime context
    pub fn new(state: Value) -> Self {
        Self {
            state,
            tool_call_id: None,
            store: None,
            stream_writer: None,
            config: HashMap::new(),
            runtime: None,
        }
    }

    /// Set the tool call ID
    pub fn with_tool_call_id(mut self, id: String) -> Self {
        self.tool_call_id = Some(id);
        self
    }

    /// Set the store
    pub fn with_store(mut self, store: Arc<dyn Store>) -> Self {
        self.store = Some(store);
        self
    }

    /// Set the stream writer
    pub fn with_stream_writer(mut self, writer: StreamWriter) -> Self {
        self.stream_writer = Some(writer);
        self
    }

    /// Add configuration value
    pub fn with_config(mut self, key: String, value: Value) -> Self {
        self.config.insert(key, value);
        self
    }

    /// Set the runtime context
    pub fn with_runtime(mut self, runtime: Runtime) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Get a configuration value
    pub fn get_config(&self, key: &str) -> Option<&Value> {
        self.config.get(key)
    }
}

impl std::fmt::Debug for ToolRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRuntime")
            .field("state", &self.state)
            .field("tool_call_id", &self.tool_call_id)
            .field("has_store", &self.store.is_some())
            .field("has_stream_writer", &self.stream_writer.is_some())
            .field("config", &self.config)
            .field("has_runtime", &self.runtime.is_some())
            .finish()
    }
}

/// Tool specification
pub struct Tool {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// Input schema (JSON Schema)
    pub input_schema: Value,

    /// Tool executor function
    pub executor: ToolExecutor,
}

impl Tool {
    /// Create a new tool
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
        executor: ToolExecutor,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            executor,
        }
    }

    /// Execute the tool with given arguments
    pub async fn execute(
        &self,
        args: Value,
        runtime: Option<ToolRuntime>,
    ) -> ToolResult {
        (self.executor)(args, runtime).await
    }

    /// Validate tool arguments against schema
    ///
    /// Validates the provided arguments against this tool's JSON Schema.
    /// Returns an error if validation fails.
    ///
    /// # Arguments
    ///
    /// * `args` - The tool arguments to validate (should be a JSON object)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if validation succeeds
    /// * `Err(ToolError::ValidationError)` if validation fails
    ///
    /// # Feature Flag
    ///
    /// Full JSON Schema validation requires the `json-validation` feature.
    /// Without this feature, only basic type checking is performed.
    pub fn validate_args(&self, args: &Value) -> Result<(), ToolError> {
        // Basic validation: args must be an object
        if !args.is_object() {
            return Err(ToolError::ValidationError {
                tool: self.name.clone(),
                error: "Arguments must be an object".to_string(),
            });
        }

        // Full JSON Schema validation (requires json-validation feature)
        #[cfg(feature = "json-validation")]
        {
            use jsonschema::JSONSchema;

            // Compile the schema
            let compiled_schema = JSONSchema::compile(&self.input_schema)
                .map_err(|e| ToolError::ValidationError {
                    tool: self.name.clone(),
                    error: format!("Invalid JSON Schema: {}", e),
                })?;

            // Validate and collect errors in a nested scope
            let error_messages = match compiled_schema.validate(args) {
                Ok(()) => None,
                Err(errors) => {
                    // Collect errors immediately while compiled_schema is still alive
                    Some(errors
                        .map(|e| format!("{}: {}", e.instance_path, e))
                        .collect::<Vec<String>>())
                }
            };

            // compiled_schema is dropped here, then we can safely return the error
            if let Some(messages) = error_messages {
                return Err(ToolError::ValidationError {
                    tool: self.name.clone(),
                    error: messages.join("; "),
                });
            }
        }

        #[cfg(not(feature = "json-validation"))]
        {
            // Without the feature, log a warning
            tracing::warn!(
                tool = %self.name,
                "JSON Schema validation skipped (enable 'json-validation' feature for full validation)"
            );
        }

        Ok(())
    }
}

impl std::fmt::Debug for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_schema", &self.input_schema)
            .field("executor", &"<function>")
            .finish()
    }
}

/// Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool call ID (for tracking)
    pub id: String,

    /// Tool name to invoke
    pub name: String,

    /// Tool arguments (JSON object)
    pub args: Value,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// Tool call ID (matches the request)
    pub id: String,

    /// Tool name that was invoked
    pub name: String,

    /// Tool output (success or error)
    pub output: ToolOutput,
}

/// Tool execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ToolOutput {
    /// Successful execution
    Success { content: Value },

    /// Execution failed with error
    Error { error: String },
}

/// Tool registry for managing available tools
pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Tool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Execute a tool call
    pub async fn execute_tool_call(
        &self,
        tool_call: &ToolCall,
        runtime: Option<ToolRuntime>,
    ) -> ToolCallResult {
        let tool = match self.get(&tool_call.name) {
            Some(t) => t,
            None => {
                return ToolCallResult {
                    id: tool_call.id.clone(),
                    name: tool_call.name.clone(),
                    output: ToolOutput::Error {
                        error: format!(
                            "Tool '{}' not found. Available tools: {}",
                            tool_call.name,
                            self.tool_names().join(", ")
                        ),
                    },
                };
            }
        };

        // Validate arguments
        if let Err(e) = tool.validate_args(&tool_call.args) {
            return ToolCallResult {
                id: tool_call.id.clone(),
                name: tool_call.name.clone(),
                output: ToolOutput::Error {
                    error: e.to_string(),
                },
            };
        }

        // Execute the tool
        match tool.execute(tool_call.args.clone(), runtime).await {
            Ok(content) => ToolCallResult {
                id: tool_call.id.clone(),
                name: tool_call.name.clone(),
                output: ToolOutput::Success { content },
            },
            Err(e) => ToolCallResult {
                id: tool_call.id.clone(),
                name: tool_call.name.clone(),
                output: ToolOutput::Error {
                    error: e.to_string(),
                },
            },
        }
    }

    /// Execute multiple tool calls in parallel
    pub async fn execute_tool_calls(
        &self,
        tool_calls: &[ToolCall],
        runtime: Option<ToolRuntime>,
    ) -> Vec<ToolCallResult> {
        use futures::future::join_all;

        let futures = tool_calls.iter().map(|tc| {
            self.execute_tool_call(tc, runtime.clone())
        });

        join_all(futures).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_runtime_creation() {
        let state = serde_json::json!({"value": 42});
        let runtime = ToolRuntime::new(state.clone());

        assert_eq!(runtime.state, state);
        assert!(runtime.tool_call_id.is_none());
        assert!(runtime.store.is_none());
        assert!(runtime.stream_writer.is_none());
    }

    #[tokio::test]
    async fn test_tool_runtime_builder() {
        let state = serde_json::json!({"value": 42});
        let runtime = ToolRuntime::new(state.clone())
            .with_tool_call_id("call_123".to_string())
            .with_config("key".to_string(), serde_json::json!("value"));

        assert_eq!(runtime.tool_call_id, Some("call_123".to_string()));
        assert_eq!(runtime.get_config("key"), Some(&serde_json::json!("value")));
    }

    #[tokio::test]
    async fn test_tool_creation() {
        let tool = Tool::new(
            "test_tool",
            "A test tool",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "x": {"type": "number"}
                }
            }),
            Arc::new(|args, _runtime| {
                Box::pin(async move {
                    Ok(serde_json::json!({"result": args["x"].as_i64().unwrap() * 2}))
                })
            }),
        );

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let tool = Tool::new(
            "multiply",
            "Multiply by 2",
            serde_json::json!({}),
            Arc::new(|args, _runtime| {
                Box::pin(async move {
                    let x = args["x"].as_i64().unwrap();
                    Ok(serde_json::json!({"result": x * 2}))
                })
            }),
        );

        let result = tool.execute(serde_json::json!({"x": 21}), None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!({"result": 42}));
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();

        let tool = Tool::new(
            "test",
            "Test tool",
            serde_json::json!({}),
            Arc::new(|args, _runtime| {
                Box::pin(async move { Ok(args) })
            }),
        );

        registry.register(tool);

        assert!(registry.has_tool("test"));
        assert!(!registry.has_tool("nonexistent"));
        assert_eq!(registry.tool_names(), vec!["test"]);
    }

    #[tokio::test]
    async fn test_tool_call_execution() {
        let mut registry = ToolRegistry::new();

        let tool = Tool::new(
            "add",
            "Add two numbers",
            serde_json::json!({}),
            Arc::new(|args, _runtime| {
                Box::pin(async move {
                    let a = args["a"].as_i64().unwrap();
                    let b = args["b"].as_i64().unwrap();
                    Ok(serde_json::json!({"sum": a + b}))
                })
            }),
        );

        registry.register(tool);

        let tool_call = ToolCall {
            id: "call_1".to_string(),
            name: "add".to_string(),
            args: serde_json::json!({"a": 10, "b": 32}),
        };

        let result = registry.execute_tool_call(&tool_call, None).await;

        assert_eq!(result.id, "call_1");
        assert_eq!(result.name, "add");
        match result.output {
            ToolOutput::Success { content } => {
                assert_eq!(content, serde_json::json!({"sum": 42}));
            }
            ToolOutput::Error { error } => {
                panic!("Expected success, got error: {}", error);
            }
        }
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let registry = ToolRegistry::new();

        let tool_call = ToolCall {
            id: "call_1".to_string(),
            name: "nonexistent".to_string(),
            args: serde_json::json!({}),
        };

        let result = registry.execute_tool_call(&tool_call, None).await;

        match result.output {
            ToolOutput::Error { error } => {
                assert!(error.contains("not found"));
            }
            ToolOutput::Success { .. } => {
                panic!("Expected error for nonexistent tool");
            }
        }
    }

    // JSON Schema validation tests
    #[tokio::test]
    async fn test_validate_args_non_object() {
        let tool = Tool::new(
            "test",
            "Test tool",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                }
            }),
            Arc::new(|args, _runtime| {
                Box::pin(async move { Ok(args) })
            }),
        );

        // Test with non-object (array)
        let result = tool.validate_args(&serde_json::json!(["not", "an", "object"]));
        assert!(result.is_err());
        match result {
            Err(ToolError::ValidationError { tool: name, error }) => {
                assert_eq!(name, "test");
                assert!(error.contains("must be an object"));
            }
            _ => panic!("Expected ValidationError"),
        }

        // Test with non-object (string)
        let result = tool.validate_args(&serde_json::json!("not an object"));
        assert!(result.is_err());
    }

    #[cfg(feature = "json-validation")]
    #[tokio::test]
    async fn test_validate_args_valid_input() {
        let tool = Tool::new(
            "calculator",
            "Perform calculations",
            serde_json::json!({
                "type": "object",
                "required": ["operation", "x", "y"],
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["add", "subtract", "multiply", "divide"]
                    },
                    "x": {"type": "number"},
                    "y": {"type": "number"}
                }
            }),
            Arc::new(|args, _runtime| {
                Box::pin(async move { Ok(args) })
            }),
        );

        // Valid input - should pass
        let result = tool.validate_args(&serde_json::json!({
            "operation": "add",
            "x": 10,
            "y": 20
        }));
        assert!(result.is_ok());
    }

    #[cfg(feature = "json-validation")]
    #[tokio::test]
    async fn test_validate_args_missing_required() {
        let tool = Tool::new(
            "get_weather",
            "Get weather information",
            serde_json::json!({
                "type": "object",
                "required": ["location"],
                "properties": {
                    "location": {"type": "string"},
                    "units": {"type": "string", "enum": ["celsius", "fahrenheit"]}
                }
            }),
            Arc::new(|args, _runtime| {
                Box::pin(async move { Ok(args) })
            }),
        );

        // Missing required field
        let result = tool.validate_args(&serde_json::json!({
            "units": "celsius"
        }));
        assert!(result.is_err());
        match result {
            Err(ToolError::ValidationError { tool: name, error }) => {
                assert_eq!(name, "get_weather");
                assert!(error.contains("location") || error.contains("required"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[cfg(feature = "json-validation")]
    #[tokio::test]
    async fn test_validate_args_wrong_type() {
        let tool = Tool::new(
            "multiply",
            "Multiply numbers",
            serde_json::json!({
                "type": "object",
                "required": ["x", "y"],
                "properties": {
                    "x": {"type": "number"},
                    "y": {"type": "number"}
                }
            }),
            Arc::new(|args, _runtime| {
                Box::pin(async move { Ok(args) })
            }),
        );

        // Wrong type for x (string instead of number)
        let result = tool.validate_args(&serde_json::json!({
            "x": "not a number",
            "y": 10
        }));
        assert!(result.is_err());
        match result {
            Err(ToolError::ValidationError { tool: name, error }) => {
                assert_eq!(name, "multiply");
                // Error message should mention type mismatch
                assert!(error.to_lowercase().contains("type") ||
                       error.to_lowercase().contains("number") ||
                       error.to_lowercase().contains("string"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[cfg(feature = "json-validation")]
    #[tokio::test]
    async fn test_validate_args_enum_violation() {
        let tool = Tool::new(
            "search",
            "Search engine",
            serde_json::json!({
                "type": "object",
                "required": ["engine"],
                "properties": {
                    "engine": {"type": "string", "enum": ["google", "bing", "duckduckgo"]},
                    "query": {"type": "string"}
                }
            }),
            Arc::new(|args, _runtime| {
                Box::pin(async move { Ok(args) })
            }),
        );

        // Invalid enum value
        let result = tool.validate_args(&serde_json::json!({
            "engine": "yahoo",  // Not in enum
            "query": "test"
        }));
        assert!(result.is_err());
        match result {
            Err(ToolError::ValidationError { tool: name, error }) => {
                assert_eq!(name, "search");
                assert!(error.contains("enum") || error.contains("yahoo") || error.contains("engine"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[cfg(feature = "json-validation")]
    #[tokio::test]
    async fn test_validate_args_nested_objects() {
        let tool = Tool::new(
            "create_user",
            "Create a new user",
            serde_json::json!({
                "type": "object",
                "required": ["user"],
                "properties": {
                    "user": {
                        "type": "object",
                        "required": ["name", "email"],
                        "properties": {
                            "name": {"type": "string", "minLength": 1},
                            "email": {"type": "string", "format": "email"},
                            "age": {"type": "integer", "minimum": 0, "maximum": 150}
                        }
                    }
                }
            }),
            Arc::new(|args, _runtime| {
                Box::pin(async move { Ok(args) })
            }),
        );

        // Valid nested object
        let result = tool.validate_args(&serde_json::json!({
            "user": {
                "name": "John Doe",
                "email": "john@example.com",
                "age": 30
            }
        }));
        assert!(result.is_ok());

        // Invalid: missing nested required field
        let result = tool.validate_args(&serde_json::json!({
            "user": {
                "name": "John Doe"
                // missing email
            }
        }));
        assert!(result.is_err());

        // Invalid: nested field violates constraint
        let result = tool.validate_args(&serde_json::json!({
            "user": {
                "name": "John Doe",
                "email": "john@example.com",
                "age": 200  // Exceeds maximum
            }
        }));
        assert!(result.is_err());
    }

    #[cfg(feature = "json-validation")]
    #[tokio::test]
    async fn test_validate_args_with_defaults() {
        let tool = Tool::new(
            "config",
            "Configuration tool",
            serde_json::json!({
                "type": "object",
                "required": ["mode"],
                "properties": {
                    "mode": {"type": "string"},
                    "verbose": {"type": "boolean", "default": false},
                    "timeout": {"type": "integer", "default": 30}
                }
            }),
            Arc::new(|args, _runtime| {
                Box::pin(async move { Ok(args) })
            }),
        );

        // Valid with only required field (defaults are not enforced by validator)
        let result = tool.validate_args(&serde_json::json!({
            "mode": "production"
        }));
        assert!(result.is_ok());

        // Valid with all fields
        let result = tool.validate_args(&serde_json::json!({
            "mode": "production",
            "verbose": true,
            "timeout": 60
        }));
        assert!(result.is_ok());
    }
}
