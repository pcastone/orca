//! Error types and error handling for graph operations
//!
//! This module defines all error types that can occur during graph construction,
//! validation, and execution. All errors implement `std::error::Error` via the
//! `thiserror` crate.
//!
//! # Error Hierarchy
//!
//! ```text
//! GraphError
//! ├── Validation         - Graph structure errors
//! ├── NodeExecution      - Node execution failures
//! ├── Execution          - General execution errors
//! ├── Interrupted        - Human-in-the-loop interrupts
//! ├── StateError         - State management errors
//! ├── Checkpoint         - Persistence errors
//! ├── Serialization      - JSON/YAML errors
//! ├── Configuration      - Configuration errors
//! ├── Timeout            - Operation timeouts
//! └── Custom             - Application-defined errors
//! ```
//!
//! # Error Handling Patterns
//!
//! ## Basic Error Handling
//!
//! ```rust
//! use langgraph_core::{StateGraph, error::GraphError};
//!
//! fn build_graph() -> Result<(), GraphError> {
//!     let mut graph = StateGraph::new();
//!
//!     // ... add nodes and edges ...
//!
//!     let compiled = graph.compile()?;  // May return GraphError
//!     Ok(())
//! }
//! ```
//!
//! ## Matching Specific Errors
//!
//! ```rust
//! use langgraph_core::error::GraphError;
//!
//! fn handle_error(err: GraphError) {
//!     match err {
//!         GraphError::Validation(msg) => {
//!             eprintln!("Graph structure invalid: {}", msg);
//!             // Fix graph structure
//!         }
//!         GraphError::NodeExecution { node, error } => {
//!             eprintln!("Node '{}' failed: {}", node, error);
//!             // Handle node-specific failure
//!         }
//!         GraphError::Interrupted { node, reason } => {
//!             println!("Waiting for input at node '{}': {}", node, reason);
//!             // Resume with user input
//!         }
//!         GraphError::Checkpoint(e) => {
//!             eprintln!("Failed to save checkpoint: {}", e);
//!             // Handle persistence failure
//!         }
//!         _ => {
//!             eprintln!("Other error: {}", err);
//!         }
//!     }
//! }
//! ```
//!
//! ## Propagating Errors
//!
//! ```rust
//! use langgraph_core::error::{GraphError, Result};
//!
//! async fn execute_workflow() -> Result<serde_json::Value> {
//!     let mut graph = langgraph_core::StateGraph::new();
//!     // ... build graph ...
//!
//!     let compiled = graph.compile()?;  // Propagate validation errors
//!     let result = compiled.invoke(serde_json::json!({})).await?;  // Propagate execution errors
//!
//!     Ok(result)
//! }
//! ```
//!
//! # Error Recovery Strategies
//!
//! ## Validation Errors
//!
//! **Cause**: Graph structure invalid (missing nodes, invalid edges, etc.)
//!
//! **Recovery**: Fix graph construction logic, ensure all referenced nodes exist
//!
//! ```rust
//! use langgraph_core::{StateGraph, error::GraphError};
//!
//! fn safe_compile(mut graph: StateGraph) -> Result<(), GraphError> {
//!     match graph.compile() {
//!         Ok(compiled) => Ok(()),
//!         Err(GraphError::Validation(msg)) => {
//!             eprintln!("Validation error: {}", msg);
//!             // Log error, fix graph structure
//!             Err(GraphError::Validation(msg))
//!         }
//!         Err(e) => Err(e),
//!     }
//! }
//! ```
//!
//! ## Node Execution Errors
//!
//! **Cause**: Node logic threw an error
//!
//! **Recovery**: Implement error handling in node logic, add retry policies
//!
//! ```rust,no_run
//! use langgraph_core::StateGraph;
//! use serde_json::json;
//!
//! let mut graph = StateGraph::new();
//!
//! graph.add_node("resilient", |state| {
//!     Box::pin(async move {
//!         // Try operation with fallback
//!         match risky_operation(&state).await {
//!             Ok(result) => Ok(result),
//!             Err(e) => {
//!                 eprintln!("Operation failed, using fallback: {}", e);
//!                 Ok(json!({"fallback": true}))
//!             }
//!         }
//!     })
//! });
//!
//! async fn risky_operation(state: &serde_json::Value) -> Result<serde_json::Value, String> {
//!     Ok(json!({}))
//! }
//! ```
//!
//! ## Interrupt Handling (Human-in-the-Loop)
//!
//! **Cause**: Node triggered an interrupt for human approval
//!
//! **Recovery**: Collect user input, resume with updated state
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, error::GraphError};
//! use serde_json::json;
//!
//! async fn workflow_with_approval(mut graph: StateGraph) -> Result<(), GraphError> {
//!     let compiled = graph.compile()?;
//!
//!     loop {
//!         match compiled.invoke(json!({})).await {
//!             Ok(result) => {
//!                 println!("Workflow complete: {:?}", result);
//!                 break;
//!             }
//!             Err(GraphError::Interrupted { node, reason }) => {
//!                 println!("Approval needed at {}: {}", node, reason);
//!
//!                 // Get user input
//!                 let approved = get_user_approval().await;
//!
//!                 // Resume with approval
//!                 let resume_state = json!({"approved": approved});
//!                 // Resume execution (implementation depends on checkpoint system)
//!             }
//!             Err(e) => return Err(e),
//!         }
//!     }
//!
//!     Ok(())
//! }
//!
//! async fn get_user_approval() -> bool {
//!     // Collect user input
//!     true
//! }
//! ```
//!
//! ## Checkpoint Errors
//!
//! **Cause**: Failed to save/load checkpoint (database down, disk full, etc.)
//!
//! **Recovery**: Retry with backoff, switch to fallback checkpoint backend
//!
//! # Common Error Patterns
//!
//! ## Missing Node Reference
//!
//! ```text
//! GraphError::Validation("Edge target 'process' does not exist")
//! ```
//!
//! **Fix**: Ensure all edge targets are added as nodes:
//!
//! ```rust
//! use langgraph_core::StateGraph;
//!
//! let mut graph = StateGraph::new();
//!
//! // Add node BEFORE referencing it in edges
//! graph.add_node("process", |state| {
//!     Box::pin(async move { Ok(state) })
//! });
//!
//! graph.add_edge("__start__", "process");  // Now valid
//! ```
//!
//! ## State Type Mismatch
//!
//! ```text
//! GraphError::StateError { node: Some("process"), error: "Expected object, got array" }
//! ```
//!
//! **Fix**: Ensure consistent state shape across nodes
//!
//! ## Serialization Failures
//!
//! ```text
//! GraphError::Serialization(...)
//! ```
//!
//! **Fix**: Ensure all state values are JSON-serializable
//!
//! # See Also
//!
//! - [`Result`] - Convenience type alias
//! - [`GraphError`] - Main error enum
//! - [`langgraph_checkpoint::CheckpointError`] - Checkpoint-specific errors

use thiserror::Error;
use crate::inline_interrupt::InlineInterruptState;

/// Convenience result type using [`GraphError`]
///
/// This type alias simplifies function signatures by providing a standard
/// `Result<T, GraphError>` type.
///
/// # Examples
///
/// ```rust
/// use langgraph_core::error::{Result, GraphError};
///
/// fn validate_input(data: &str) -> Result<()> {
///     if data.is_empty() {
///         return Err(GraphError::Validation("Input cannot be empty".to_string()));
///     }
///     Ok(())
/// }
/// ```
pub type Result<T> = std::result::Result<T, GraphError>;

/// Comprehensive error type for all graph operations
///
/// `GraphError` represents all errors that can occur during graph construction,
/// validation, and execution. It uses `thiserror` for automatic `Error` trait
/// implementation and includes context where helpful.
///
/// # Error Categories
///
/// - **Construction**: `Validation`, `Configuration`
/// - **Execution**: `NodeExecution`, `Execution`, `Interrupted`
/// - **State**: `StateError`, `State`
/// - **Persistence**: `Checkpoint`
/// - **Serialization**: `Serialization`, `Yaml`
/// - **System**: `Io`, `Timeout`
/// - **Extension**: `Custom`, `InlineInterrupt`
///
/// # Examples
///
/// ## Creating Errors
///
/// ```rust
/// use langgraph_core::error::GraphError;
///
/// // Validation error
/// let err = GraphError::Validation("Missing entry node".to_string());
///
/// // Node execution error with context
/// let err = GraphError::node_execution("llm", "API key not found");
///
/// // Interrupt (human-in-the-loop)
/// let err = GraphError::interrupted("approval", "Manual review required");
/// ```
///
/// ## Matching Errors
///
/// ```rust
/// use langgraph_core::error::GraphError;
///
/// fn handle(err: GraphError) -> String {
///     match err {
///         GraphError::Interrupted { node, reason } => {
///             format!("Paused at {}: {}", node, reason)
///         }
///         GraphError::NodeExecution { node, error } => {
///             format!("Failed at {}: {}", node, error)
///         }
///         _ => format!("Error: {}", err),
///     }
/// }
/// ```
///
/// # See Also
///
/// - [`Result`] - Type alias for `Result<T, GraphError>`
#[derive(Error, Debug)]
pub enum GraphError {
    /// Graph structure validation failed
    ///
    /// Occurs during graph compilation when the graph structure is invalid.
    ///
    /// **Common causes**:
    /// - Referenced node doesn't exist
    /// - Missing entry point
    /// - Unreachable nodes
    /// - Cyclic dependencies (when not allowed)
    ///
    /// **Recovery**: Fix graph structure before compilation
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    ///
    /// let err = GraphError::Validation("Entry point 'start' does not exist".to_string());
    /// ```
    #[error("Graph validation failed: {0}")]
    Validation(String),

    /// Node execution failed with context
    ///
    /// Occurs when a node's executor function returns an error during execution.
    ///
    /// **Common causes**:
    /// - Node logic threw an exception
    /// - Missing required state fields
    /// - External API failures
    /// - Invalid state transformations
    ///
    /// **Recovery**: Fix node logic, add error handling, implement retry
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    ///
    /// let err = GraphError::node_execution("llm_call", "API timeout");
    /// assert_eq!(format!("{}", err), "Node 'llm_call' execution failed: API timeout");
    /// ```
    #[error("Node '{node}' execution failed: {error}")]
    NodeExecution {
        /// Name of the node that failed
        node: String,
        /// Error message from node execution
        error: String,
    },

    /// Generic execution error without specific node context
    ///
    /// Used for execution errors that don't belong to a specific node.
    ///
    /// **Recovery**: Check execution logs, verify graph configuration
    #[error("Execution failed: {0}")]
    Execution(String),

    /// Graph execution interrupted (human-in-the-loop)
    ///
    /// Occurs when a node triggers an interrupt requesting human input.
    /// This is **not an error** but a normal workflow pause.
    ///
    /// **Common causes**:
    /// - Node returned `Interrupt` error
    /// - Manual approval required
    /// - Human review needed
    ///
    /// **Recovery**: Collect user input, resume execution with updated state
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    ///
    /// let err = GraphError::interrupted("approval_node", "Budget exceeds threshold");
    ///
    /// // Handle interrupt
    /// if let GraphError::Interrupted { node, reason } = err {
    ///     println!("Waiting for approval at {}: {}", node, reason);
    /// }
    /// ```
    #[error("Graph execution interrupted at node '{node}': {reason}")]
    Interrupted {
        /// Node where execution was interrupted
        node: String,
        /// Reason for the interrupt
        reason: String,
    },

    /// State management error with optional node context
    ///
    /// Occurs when state operations fail.
    ///
    /// **Common causes**:
    /// - Type mismatch in state updates
    /// - Invalid state structure
    /// - Missing required fields
    /// - Reducer function errors
    ///
    /// **Recovery**: Ensure consistent state shape, validate inputs
    #[error("State error{}: {error}", node.as_ref().map(|n| format!(" in node '{}'", n)).unwrap_or_default())]
    StateError {
        /// Optional node context where error occurred
        node: Option<String>,
        /// Error description
        error: String,
    },

    /// Generic state error without context
    ///
    /// Used for state errors without specific node context.
    #[error("State error: {0}")]
    State(String),

    /// Checkpoint persistence error
    ///
    /// Occurs when saving or loading checkpoints fails.
    ///
    /// **Common causes**:
    /// - Database connection failed
    /// - Disk full
    /// - Serialization errors
    /// - Permission denied
    ///
    /// **Recovery**: Check persistence backend, retry with backoff, use fallback
    ///
    /// Wraps errors from `langgraph_checkpoint::CheckpointError`.
    #[error("Checkpoint error: {0}")]
    Checkpoint(#[from] langgraph_checkpoint::CheckpointError),

    /// JSON serialization/deserialization error
    ///
    /// Occurs when state cannot be serialized to/from JSON.
    ///
    /// **Common causes**:
    /// - Non-JSON-serializable values in state
    /// - Circular references
    /// - Invalid JSON structure
    ///
    /// **Recovery**: Ensure all state values are JSON-compatible
    ///
    /// Wraps errors from `serde_json::Error`.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// YAML parsing error
    ///
    /// Occurs when loading graph definitions from YAML files.
    ///
    /// **Common causes**:
    /// - Invalid YAML syntax
    /// - Missing required fields
    /// - Type mismatches
    ///
    /// **Recovery**: Validate YAML against schema
    ///
    /// Wraps errors from `serde_yaml::Error`.
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// I/O operation failed
    ///
    /// Occurs during file operations or network I/O.
    ///
    /// **Common causes**:
    /// - File not found
    /// - Permission denied
    /// - Network unreachable
    ///
    /// **Recovery**: Check file paths, permissions, network connectivity
    ///
    /// Wraps errors from `std::io::Error`.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Graph or node configuration error
    ///
    /// Occurs when configuration is invalid or missing.
    ///
    /// **Common causes**:
    /// - Missing required configuration
    /// - Invalid configuration values
    /// - Incompatible settings
    ///
    /// **Recovery**: Verify configuration, provide defaults
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Operation exceeded time limit
    ///
    /// Occurs when an operation takes longer than allowed.
    ///
    /// **Common causes**:
    /// - Node execution too slow
    /// - External API delays
    /// - Infinite loops
    ///
    /// **Recovery**: Optimize node logic, increase timeout, add cancellation
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    ///
    /// let err = GraphError::Timeout {
    ///     operation: "API call".to_string(),
    ///     duration_ms: 5000,
    /// };
    /// ```
    #[error("Operation timed out after {duration_ms}ms: {operation}")]
    Timeout {
        /// Description of the operation that timed out
        operation: String,
        /// Timeout duration in milliseconds
        duration_ms: u64,
    },

    /// Custom application-defined error
    ///
    /// Used for application-specific errors not covered by other variants.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    ///
    /// let err = GraphError::Custom("Business logic validation failed".to_string());
    /// ```
    #[error("{0}")]
    Custom(String),

    /// Inline interrupt requested within a node
    ///
    /// Similar to `Interrupted` but includes full interrupt state for inline interrupts.
    ///
    /// **Recovery**: Handle interrupt state, resume when ready
    #[error("Inline interrupt requested in node '{}'", .0.node)]
    InlineInterrupt(InlineInterruptState),
}

impl GraphError {
    /// Create a node execution error with context
    ///
    /// Helper constructor for creating node execution errors with node name and error message.
    ///
    /// # Arguments
    ///
    /// * `node` - Name of the node that failed
    /// * `error` - Error message or description
    ///
    /// # Examples
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    ///
    /// // From a node implementation
    /// fn my_node() -> Result<(), GraphError> {
    ///     // ... some operation fails ...
    ///     Err(GraphError::node_execution("my_node", "API call failed"))
    /// }
    /// ```
    ///
    /// # Usage in Node Implementations
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    ///
    /// async fn llm_node(state: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    ///     let response = call_llm(&state).await
    ///         .map_err(|e| GraphError::node_execution("llm_node", format!("LLM call failed: {}", e)))?;
    ///     Ok(response)
    /// }
    /// # async fn call_llm(state: &serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> { Ok(state.clone()) }
    /// ```
    pub fn node_execution(node: impl Into<String>, error: impl Into<String>) -> Self {
        Self::NodeExecution {
            node: node.into(),
            error: error.into(),
        }
    }

    /// Create a state error with optional node context
    ///
    /// Helper constructor for creating state-related errors, optionally associated with a node.
    ///
    /// # Arguments
    ///
    /// * `node` - Optional name of the node where state error occurred
    /// * `error` - Error message describing the state issue
    ///
    /// # Examples
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    ///
    /// // State error with node context
    /// let error = GraphError::state_error(
    ///     Some("process_node"),
    ///     "Required field 'user_id' missing"
    /// );
    ///
    /// // State error without node context (during initialization)
    /// let error = GraphError::state_error(
    ///     None::<String>,
    ///     "Initial state must contain 'messages' field"
    /// );
    /// ```
    ///
    /// # Usage in State Validation
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    /// use serde_json::Value;
    ///
    /// fn validate_state(node_name: &str, state: &Value) -> Result<(), GraphError> {
    ///     if !state.is_object() {
    ///         return Err(GraphError::state_error(
    ///             Some(node_name),
    ///             "State must be an object"
    ///         ));
    ///     }
    ///     if state.get("messages").is_none() {
    ///         return Err(GraphError::state_error(
    ///             Some(node_name),
    ///             "State missing required 'messages' field"
    ///         ));
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn state_error(node: Option<impl Into<String>>, error: impl Into<String>) -> Self {
        Self::StateError {
            node: node.map(|n| n.into()),
            error: error.into(),
        }
    }

    /// Create an interrupted error
    ///
    /// Helper constructor for creating interrupt errors when a node requests human-in-the-loop interaction.
    ///
    /// # Arguments
    ///
    /// * `node` - Name of the node requesting the interrupt
    /// * `reason` - Reason for the interrupt (e.g., "approval_needed", "clarification_required")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    ///
    /// // Node requests approval
    /// fn approval_node(state: serde_json::Value) -> Result<serde_json::Value, GraphError> {
    ///     if state["needs_approval"].as_bool().unwrap_or(false) {
    ///         return Err(GraphError::interrupted("approval_node", "user_approval_required"));
    ///     }
    ///     Ok(state)
    /// }
    /// ```
    ///
    /// # Usage in Human-in-the-Loop Workflows
    ///
    /// ```rust
    /// use langgraph_core::error::GraphError;
    /// use serde_json::{json, Value};
    ///
    /// async fn decision_node(state: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    ///     let confidence = state["confidence"].as_f64().unwrap_or(0.0);
    ///
    ///     // Low confidence - request human review
    ///     if confidence < 0.7 {
    ///         return Err(Box::new(GraphError::interrupted(
    ///             "decision_node",
    ///             format!("Low confidence ({:.2}), human review required", confidence)
    ///         )));
    ///     }
    ///
    ///     // High confidence - proceed automatically
    ///     Ok(json!({
    ///         "decision": "approved",
    ///         "automated": true
    ///     }))
    /// }
    /// ```
    ///
    /// # Handling Interrupts
    ///
    /// When execution is interrupted, the graph is checkpointed and control returns to the caller.
    /// To resume after collecting user input:
    ///
    /// ```rust,ignore
    /// // Initial execution
    /// match graph.invoke(initial_state, config).await {
    ///     Ok(result) => println!("Completed: {:?}", result),
    ///     Err(e) if matches!(e, GraphError::Interrupted { .. }) => {
    ///         // Collect user input
    ///         let user_input = get_user_approval().await;
    ///
    ///         // Resume from checkpoint with updated state
    ///         let updated_state = json!({ "approval": user_input });
    ///         let result = graph.invoke(updated_state, config).await?;
    ///     }
    ///     Err(e) => return Err(e),
    /// }
    /// ```
    pub fn interrupted(node: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Interrupted {
            node: node.into(),
            reason: reason.into(),
        }
    }
}
