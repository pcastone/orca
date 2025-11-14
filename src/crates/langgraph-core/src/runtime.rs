//! Runtime context for graph execution
//!
//! This module provides runtime context that can be accessed from within nodes during execution.
//! Similar to Python LangGraph's `get_runtime()`, `get_store()`, and `get_stream_writer()`.
//!
//! # Example
//!
//! ```rust,no_run
//! use langgraph_core::runtime::Runtime;
//! use langgraph_core::StateGraph;
//!
//! let mut graph = StateGraph::new();
//!
//! graph.add_node("process", |state| {
//!     Box::pin(async move {
//!         // Access runtime context from within the node
//!         // let runtime = Runtime::current();
//!         // let store = runtime.store();
//!         Ok(state)
//!     })
//! });
//! ```

use crate::managed::ExecutionContext;
use crate::store::Store;
use crate::stream::StreamEvent;
use crate::inline_interrupt::{InlineInterruptState, InlineResumeValue};
use serde_json::Value;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// Stream writer for emitting custom events during execution
#[derive(Clone)]
pub struct StreamWriter {
    tx: mpsc::UnboundedSender<StreamEvent>,
}

impl StreamWriter {
    /// Create a new stream writer
    pub(crate) fn new(tx: mpsc::UnboundedSender<StreamEvent>) -> Self {
        Self { tx }
    }

    /// Write a custom event to the stream
    pub fn write(&self, data: Value) -> Result<(), String> {
        self.tx
            .send(StreamEvent::Custom { data })
            .map_err(|e| format!("Failed to send custom event: {}", e))
    }

    /// Write a message event to the stream
    pub fn write_message(&self, message: Value, metadata: Option<Value>) -> Result<(), String> {
        self.tx
            .send(StreamEvent::Message { message, metadata })
            .map_err(|e| format!("Failed to send message event: {}", e))
    }

    /// Check if the stream is closed
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

/// Runtime context bundle available during graph execution
///
/// This provides access to:
/// - Execution context (step counters, remaining steps)
/// - Store for persistent state
/// - Stream writer for custom events
/// - Previous values from earlier execution steps
#[derive(Clone)]
pub struct Runtime {
    /// Execution context with step tracking
    execution_context: ExecutionContext,

    /// Store for persistent data
    store: Option<Arc<dyn Store>>,

    /// Stream writer for custom events
    stream_writer: Option<StreamWriter>,

    /// Previous values from earlier execution (for access to intermediate results)
    previous_values: Arc<RwLock<Vec<Value>>>,

    /// Current node name (for context)
    current_node: Arc<RwLock<Option<String>>>,

    /// Pending inline interrupt (if any)
    inline_interrupt: Arc<RwLock<Option<InlineInterruptState>>>,

    /// Resume value for current interrupt
    resume_value: Arc<RwLock<Option<InlineResumeValue>>>,
}

impl Runtime {
    /// Create a new runtime context
    pub fn new(execution_context: ExecutionContext) -> Self {
        Self {
            execution_context,
            store: None,
            stream_writer: None,
            previous_values: Arc::new(RwLock::new(Vec::new())),
            current_node: Arc::new(RwLock::new(None)),
            inline_interrupt: Arc::new(RwLock::new(None)),
            resume_value: Arc::new(RwLock::new(None)),
        }
    }

    /// Create runtime with store
    pub fn with_store(mut self, store: Arc<dyn Store>) -> Self {
        self.store = Some(store);
        self
    }

    /// Create runtime with stream writer
    pub fn with_stream_writer(mut self, writer: StreamWriter) -> Self {
        self.stream_writer = Some(writer);
        self
    }

    /// Get the execution context
    pub fn execution_context(&self) -> &ExecutionContext {
        &self.execution_context
    }

    /// Get the store (if available)
    pub fn store(&self) -> Option<&Arc<dyn Store>> {
        self.store.as_ref()
    }

    /// Get the stream writer (if available)
    pub fn stream_writer(&self) -> Option<&StreamWriter> {
        self.stream_writer.as_ref()
    }

    /// Get current step number
    pub fn current_step(&self) -> usize {
        self.execution_context.current_step()
    }

    /// Get remaining steps
    pub fn remaining_steps(&self) -> usize {
        self.execution_context.remaining_steps()
    }

    /// Check if this is the last step
    pub fn is_last_step(&self) -> bool {
        self.execution_context.is_last_step()
    }

    /// Get previous values
    pub fn previous_values(&self) -> Vec<Value> {
        self.previous_values.read().unwrap().clone()
    }

    /// Add a value to previous values (internal use)
    pub(crate) fn add_previous_value(&self, value: Value) {
        self.previous_values.write().unwrap().push(value);
    }

    /// Get current node name
    pub fn current_node(&self) -> Option<String> {
        self.current_node.read().unwrap().clone()
    }

    /// Set current node name (internal use)
    pub(crate) fn set_current_node(&self, node: Option<String>) {
        *self.current_node.write().unwrap() = node;
    }

    /// Increment step (internal use)
    pub(crate) fn increment_step(&self) {
        self.execution_context.increment_step();
    }

    /// Check if an inline interrupt is pending
    pub fn has_inline_interrupt(&self) -> bool {
        self.inline_interrupt.read().unwrap().is_some()
    }

    /// Get the pending inline interrupt
    pub fn get_inline_interrupt(&self) -> Option<InlineInterruptState> {
        self.inline_interrupt.read().unwrap().clone()
    }

    /// Set inline interrupt (internal use)
    pub(crate) fn set_inline_interrupt(&self, interrupt: Option<InlineInterruptState>) {
        *self.inline_interrupt.write().unwrap() = interrupt;
    }

    /// Get resume value for current interrupt
    pub fn get_resume_value(&self) -> Option<InlineResumeValue> {
        self.resume_value.read().unwrap().clone()
    }

    /// Set resume value for current interrupt
    pub fn set_resume_value(&self, resume: Option<InlineResumeValue>) {
        *self.resume_value.write().unwrap() = resume;
    }

    /// Clear interrupt state
    pub(crate) fn clear_interrupt(&self) {
        *self.inline_interrupt.write().unwrap() = None;
        *self.resume_value.write().unwrap() = None;
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("execution_context", &self.execution_context)
            .field("has_store", &self.store.is_some())
            .field("has_stream_writer", &self.stream_writer.is_some())
            .field("current_node", &self.current_node())
            .finish()
    }
}

// Thread-local storage for runtime context (similar to Python's contextvars)
thread_local! {
    static RUNTIME: RwLock<Option<Runtime>> = RwLock::new(None);
}

/// Set the runtime context for the current thread
///
/// This is used internally by the execution engine to make the runtime
/// available to nodes during execution.
pub(crate) fn set_runtime(runtime: Runtime) {
    RUNTIME.with(|r| {
        *r.write().unwrap() = Some(runtime);
    });
}

/// Get the current runtime context
///
/// This can be called from within node executors to access the runtime context.
///
/// # Example
///
/// ```rust,no_run
/// use langgraph_core::runtime::get_runtime;
///
/// // Inside a node executor:
/// if let Some(runtime) = get_runtime() {
///     println!("Current step: {}", runtime.current_step());
///
///     // Access store if available
///     if let Some(store) = runtime.store() {
///         // Use store...
///     }
///
///     // Write custom events
///     if let Some(writer) = runtime.stream_writer() {
///         writer.write(serde_json::json!({"custom": "data"})).ok();
///     }
/// }
/// ```
pub fn get_runtime() -> Option<Runtime> {
    RUNTIME.with(|r| r.read().unwrap().clone())
}

/// Clear the runtime context for the current thread
pub(crate) fn clear_runtime() {
    RUNTIME.with(|r| {
        *r.write().unwrap() = None;
    });
}

/// Get the store from the current runtime
///
/// Convenience function for accessing the store directly.
pub fn get_store() -> Option<Arc<dyn Store>> {
    get_runtime().and_then(|rt| rt.store().cloned())
}

/// Get the stream writer from the current runtime
///
/// Convenience function for accessing the stream writer directly.
pub fn get_stream_writer() -> Option<StreamWriter> {
    get_runtime().and_then(|rt| rt.stream_writer().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed::ExecutionContext;
    use crate::store::InMemoryStore;

    #[test]
    fn test_runtime_creation() {
        let context = ExecutionContext::new(10);
        let runtime = Runtime::new(context);

        assert_eq!(runtime.current_step(), 0);
        assert_eq!(runtime.remaining_steps(), 10);
        assert!(!runtime.is_last_step());
        assert!(runtime.store().is_none());
        assert!(runtime.stream_writer().is_none());
    }

    #[test]
    fn test_runtime_with_store() {
        let context = ExecutionContext::new(10);
        let store = Arc::new(InMemoryStore::new());
        let runtime = Runtime::new(context).with_store(store);

        assert!(runtime.store().is_some());
    }

    #[test]
    fn test_runtime_previous_values() {
        let context = ExecutionContext::new(10);
        let runtime = Runtime::new(context);

        assert_eq!(runtime.previous_values().len(), 0);

        runtime.add_previous_value(serde_json::json!({"step": 1}));
        runtime.add_previous_value(serde_json::json!({"step": 2}));

        assert_eq!(runtime.previous_values().len(), 2);
    }

    #[test]
    fn test_runtime_current_node() {
        let context = ExecutionContext::new(10);
        let runtime = Runtime::new(context);

        assert_eq!(runtime.current_node(), None);

        runtime.set_current_node(Some("test_node".to_string()));
        assert_eq!(runtime.current_node(), Some("test_node".to_string()));

        runtime.set_current_node(None);
        assert_eq!(runtime.current_node(), None);
    }

    #[test]
    fn test_thread_local_runtime() {
        // Initially, no runtime
        assert!(get_runtime().is_none());

        // Set runtime
        let context = ExecutionContext::new(5);
        let runtime = Runtime::new(context);
        set_runtime(runtime.clone());

        // Can retrieve it
        let retrieved = get_runtime().unwrap();
        assert_eq!(retrieved.current_step(), 0);

        // Clear it
        clear_runtime();
        assert!(get_runtime().is_none());
    }

    #[test]
    fn test_get_store_convenience() {
        let context = ExecutionContext::new(10);
        let store = Arc::new(InMemoryStore::new());
        let runtime = Runtime::new(context).with_store(store);

        set_runtime(runtime);

        let retrieved_store = get_store();
        assert!(retrieved_store.is_some());

        clear_runtime();
    }

    #[tokio::test]
    async fn test_stream_writer() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let writer = StreamWriter::new(tx);

        // Write custom event
        writer.write(serde_json::json!({"test": "data"})).unwrap();

        // Receive it
        let event = rx.recv().await.unwrap();
        match event {
            StreamEvent::Custom { data } => {
                assert_eq!(data, serde_json::json!({"test": "data"}));
            }
            _ => panic!("Expected Custom event"),
        }
    }

    #[tokio::test]
    async fn test_stream_writer_message() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let writer = StreamWriter::new(tx);

        // Write message event
        writer
            .write_message(
                serde_json::json!("Hello"),
                Some(serde_json::json!({"role": "assistant"})),
            )
            .unwrap();

        // Receive it
        let event = rx.recv().await.unwrap();
        match event {
            StreamEvent::Message { message, metadata } => {
                assert_eq!(message, serde_json::json!("Hello"));
                assert_eq!(metadata, Some(serde_json::json!({"role": "assistant"})));
            }
            _ => panic!("Expected Message event"),
        }
    }
}
