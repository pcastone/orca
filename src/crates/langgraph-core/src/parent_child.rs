//! Parent-child graph communication support
//!
//! This module provides mechanisms for subgraphs to communicate with their parent graphs,
//! enabling complex hierarchical workflows with bidirectional communication.
//!
//! # Key Features
//!
//! - **Message Passing**: Subgraphs can send messages to parent graphs
//! - **Command Routing**: Commands can be directed to parent or child graphs
//! - **State Synchronization**: Share state between graph levels
//! - **Context Inheritance**: Child graphs inherit context from parents
//!
//! # Example
//!
//! ```rust,no_run
//! use langgraph_core::{StateGraph, Command, CommandGraph};
//! use langgraph_core::parent_child::{ParentContext, send_to_parent};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // In a subgraph node:
//! let mut child_graph = StateGraph::new();
//!
//! child_graph.add_node("process", |state| {
//!     Box::pin(async move {
//!         // Send message to parent
//!         send_to_parent("status_update", json!({
//!             "progress": 50,
//!             "message": "Processing halfway complete"
//!         }))?;
//!
//!         // Or use Command to target parent
//!         Command::new()
//!             .with_graph(CommandGraph::Parent)
//!             .with_update(json!({"child_result": 42}));
//!
//!         Ok(state)
//!     })
//! });
//! # Ok(())
//! # }
//! ```

use crate::{
    error::{GraphError, Result},
    runtime::get_runtime,
    Command, CommandGraph,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// Message sent from child to parent graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentMessage {
    /// Type of message
    pub message_type: String,

    /// Message payload
    pub payload: Value,

    /// Source subgraph name
    pub source: String,

    /// Optional target node in parent
    pub target_node: Option<String>,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ParentMessage {
    /// Create a new parent message
    pub fn new(
        message_type: impl Into<String>,
        payload: Value,
        source: impl Into<String>,
    ) -> Self {
        Self {
            message_type: message_type.into(),
            payload,
            source: source.into(),
            target_node: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Set the target node in parent graph
    pub fn with_target(mut self, node: impl Into<String>) -> Self {
        self.target_node = Some(node.into());
        self
    }
}

/// Context passed from parent to child graph
#[derive(Debug, Clone)]
pub struct ParentContext {
    /// Parent graph ID/name
    pub parent_id: String,

    /// Shared state from parent
    pub shared_state: Arc<RwLock<Value>>,

    /// Channel for sending messages to parent
    pub parent_channel: Option<mpsc::UnboundedSender<ParentMessage>>,

    /// Parent's checkpoint configuration
    pub parent_checkpoint: Option<crate::CheckpointConfig>,

    /// Metadata from parent
    pub metadata: HashMap<String, Value>,

    /// Depth in the graph hierarchy (0 = root)
    pub depth: usize,
}

impl ParentContext {
    /// Create a new parent context
    pub fn new(parent_id: impl Into<String>) -> Self {
        Self {
            parent_id: parent_id.into(),
            shared_state: Arc::new(RwLock::new(Value::Null)),
            parent_channel: None,
            parent_checkpoint: None,
            metadata: HashMap::new(),
            depth: 0,
        }
    }

    /// Set the shared state
    pub fn with_shared_state(mut self, state: Value) -> Self {
        self.shared_state = Arc::new(RwLock::new(state));
        self
    }

    /// Set the parent channel for communication
    pub fn with_parent_channel(mut self, tx: mpsc::UnboundedSender<ParentMessage>) -> Self {
        self.parent_channel = Some(tx);
        self
    }

    /// Set parent checkpoint configuration
    pub fn with_checkpoint(mut self, checkpoint: crate::CheckpointConfig) -> Self {
        self.parent_checkpoint = Some(checkpoint);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set hierarchy depth
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// Send a message to the parent graph
    pub fn send_to_parent(&self, message: ParentMessage) -> Result<()> {
        if let Some(ref channel) = self.parent_channel {
            channel
                .send(message)
                .map_err(|e| GraphError::Execution(format!("Failed to send to parent: {}", e)))?;
            Ok(())
        } else {
            Err(GraphError::Configuration(
                "No parent channel available".to_string(),
            ))
        }
    }

    /// Get shared state from parent
    pub fn get_shared_state(&self) -> Value {
        self.shared_state.read().unwrap().clone()
    }

    /// Update shared state (visible to parent)
    pub fn update_shared_state(&self, updates: Value) -> Result<()> {
        let mut state = self.shared_state.write().unwrap();

        // Merge updates into existing state
        if let (Some(state_obj), Some(update_obj)) = (state.as_object_mut(), updates.as_object()) {
            for (key, value) in update_obj {
                state_obj.insert(key.clone(), value.clone());
            }
        } else {
            *state = updates;
        }

        Ok(())
    }
}

/// Thread-local storage for parent context
thread_local! {
    static PARENT_CONTEXT: RwLock<Option<ParentContext>> = RwLock::new(None);
}

/// Set the parent context for the current execution
pub fn set_parent_context(context: ParentContext) {
    PARENT_CONTEXT.with(|ctx| {
        *ctx.write().unwrap() = Some(context);
    });
}

/// Get the current parent context
pub fn get_parent_context() -> Option<ParentContext> {
    PARENT_CONTEXT.with(|ctx| ctx.read().unwrap().clone())
}

/// Clear the parent context
pub fn clear_parent_context() {
    PARENT_CONTEXT.with(|ctx| {
        *ctx.write().unwrap() = None;
    });
}

/// Send a message to the parent graph
///
/// This is a convenience function that uses the current parent context.
///
/// # Arguments
///
/// * `message_type` - Type of message to send
/// * `payload` - Message data
///
/// # Returns
///
/// Returns Ok(()) if message was sent, or an error if no parent context exists
///
/// # Example
///
/// ```rust,no_run
/// use langgraph_core::parent_child::send_to_parent;
/// use serde_json::json;
///
/// // Inside a subgraph node:
/// send_to_parent("status", json!({"progress": 75})).ok();
/// ```
pub fn send_to_parent(message_type: impl Into<String>, payload: Value) -> Result<()> {
    let context = get_parent_context()
        .ok_or_else(|| GraphError::Configuration("No parent context available".to_string()))?;

    let runtime = get_runtime();
    let source = runtime
        .and_then(|r| r.current_node())
        .unwrap_or_else(|| "unknown".to_string());

    let message = ParentMessage::new(message_type, payload, source);
    context.send_to_parent(message)
}

/// Extension trait for Command to support parent-child communication
pub trait CommandParentExt {
    /// Send this command to the parent graph
    fn to_parent(self) -> Self;

    /// Send this command to a named subgraph
    fn to_subgraph(self, name: impl Into<String>) -> Self;
}

impl CommandParentExt for Command {
    fn to_parent(mut self) -> Self {
        self.graph = Some(CommandGraph::Parent);
        self
    }

    fn to_subgraph(mut self, name: impl Into<String>) -> Self {
        self.graph = Some(CommandGraph::Named(name.into()));
        self
    }
}

/// Subgraph configuration with parent communication
#[derive(Debug, Clone)]
pub struct SubgraphConfig {
    /// Name of the subgraph
    pub name: String,

    /// Whether to inherit parent state
    pub inherit_state: bool,

    /// Whether to sync state back to parent
    pub sync_state_to_parent: bool,

    /// Filter for state inheritance (only these keys)
    pub state_filter: Option<Vec<String>>,

    /// Whether to forward messages to parent
    pub forward_messages: bool,

    /// Maximum depth for nested subgraphs
    pub max_depth: Option<usize>,
}

impl SubgraphConfig {
    /// Create a new subgraph configuration
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            inherit_state: true,
            sync_state_to_parent: true,
            state_filter: None,
            forward_messages: true,
            max_depth: None,
        }
    }

    /// Set whether to inherit parent state
    pub fn with_inherit_state(mut self, inherit: bool) -> Self {
        self.inherit_state = inherit;
        self
    }

    /// Set whether to sync state back to parent
    pub fn with_sync_to_parent(mut self, sync: bool) -> Self {
        self.sync_state_to_parent = sync;
        self
    }

    /// Set state filter for inheritance
    pub fn with_state_filter(mut self, filter: Vec<String>) -> Self {
        self.state_filter = Some(filter);
        self
    }

    /// Set whether to forward messages
    pub fn with_forward_messages(mut self, forward: bool) -> Self {
        self.forward_messages = forward;
        self
    }

    /// Set maximum depth for nested subgraphs
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Filter state based on configuration
    pub fn filter_state(&self, state: &Value) -> Value {
        if let Some(ref filter) = self.state_filter {
            if let Some(obj) = state.as_object() {
                let mut filtered = serde_json::Map::new();
                for key in filter {
                    if let Some(value) = obj.get(key) {
                        filtered.insert(key.clone(), value.clone());
                    }
                }
                Value::Object(filtered)
            } else {
                state.clone()
            }
        } else {
            state.clone()
        }
    }
}

/// Manager for parent-child graph relationships
#[derive(Default)]
pub struct GraphHierarchy {
    /// Map of subgraph names to their configurations
    subgraphs: HashMap<String, SubgraphConfig>,

    /// Active parent contexts by thread
    contexts: Arc<RwLock<HashMap<std::thread::ThreadId, ParentContext>>>,

    /// Message receivers for each subgraph
    receivers: Arc<RwLock<HashMap<String, mpsc::UnboundedReceiver<ParentMessage>>>>,
}

impl GraphHierarchy {
    /// Create a new graph hierarchy manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a subgraph
    pub fn register_subgraph(&mut self, config: SubgraphConfig) {
        self.subgraphs.insert(config.name.clone(), config);
    }

    /// Create a parent context for a subgraph
    pub fn create_context(&self, subgraph_name: &str, parent_state: Value) -> Option<ParentContext> {
        let config = self.subgraphs.get(subgraph_name)?;

        let (tx, rx) = mpsc::unbounded_channel();

        // Store the receiver for later message retrieval
        self.receivers.write().unwrap().insert(subgraph_name.to_string(), rx);

        let filtered_state = if config.inherit_state {
            config.filter_state(&parent_state)
        } else {
            Value::Null
        };

        Some(
            ParentContext::new(subgraph_name)
                .with_shared_state(filtered_state)
                .with_parent_channel(tx)
        )
    }

    /// Poll for messages from a subgraph
    pub async fn poll_messages(&self, subgraph_name: &str) -> Vec<ParentMessage> {
        let mut messages = Vec::new();

        if let Some(mut rx) = self.receivers.write().unwrap().remove(subgraph_name) {
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
            // Put the receiver back
            self.receivers.write().unwrap().insert(subgraph_name.to_string(), rx);
        }

        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parent_message_creation() {
        let msg = ParentMessage::new(
            "status",
            serde_json::json!({"progress": 50}),
            "child_graph",
        )
        .with_target("parent_node");

        assert_eq!(msg.message_type, "status");
        assert_eq!(msg.source, "child_graph");
        assert_eq!(msg.target_node, Some("parent_node".to_string()));
    }

    #[test]
    fn test_parent_context() {
        let context = ParentContext::new("parent")
            .with_shared_state(serde_json::json!({"key": "value"}))
            .with_depth(1)
            .with_metadata("test", serde_json::json!(true));

        assert_eq!(context.parent_id, "parent");
        assert_eq!(context.depth, 1);
        assert_eq!(context.get_shared_state(), serde_json::json!({"key": "value"}));
        assert_eq!(context.metadata.get("test"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn test_subgraph_config() {
        let config = SubgraphConfig::new("child")
            .with_inherit_state(false)
            .with_sync_to_parent(true)
            .with_state_filter(vec!["allowed_key".to_string()])
            .with_max_depth(3);

        assert_eq!(config.name, "child");
        assert!(!config.inherit_state);
        assert!(config.sync_state_to_parent);
        assert_eq!(config.max_depth, Some(3));

        // Test state filtering
        let state = serde_json::json!({
            "allowed_key": "value1",
            "filtered_key": "value2"
        });

        let filtered = config.filter_state(&state);
        assert_eq!(filtered, serde_json::json!({"allowed_key": "value1"}));
    }

    #[tokio::test]
    async fn test_graph_hierarchy() {
        let mut hierarchy = GraphHierarchy::new();

        let config = SubgraphConfig::new("test_subgraph");
        hierarchy.register_subgraph(config);

        let context = hierarchy.create_context(
            "test_subgraph",
            serde_json::json!({"parent": "state"}),
        );

        assert!(context.is_some());

        let ctx = context.unwrap();
        assert_eq!(ctx.parent_id, "test_subgraph");
        assert_eq!(ctx.get_shared_state(), serde_json::json!({"parent": "state"}));
    }

    #[test]
    fn test_command_parent_ext() {
        let cmd = Command::new()
            .with_update(serde_json::json!({"test": true}))
            .to_parent();

        assert_eq!(cmd.graph, Some(CommandGraph::Parent));

        let cmd2 = Command::new().to_subgraph("child");
        assert_eq!(cmd2.graph, Some(CommandGraph::Named("child".to_string())));
    }

    #[test]
    fn test_shared_state_update() {
        let context = ParentContext::new("parent")
            .with_shared_state(serde_json::json!({"initial": "value"}));

        // Update shared state
        context.update_shared_state(serde_json::json!({"new": "data"})).unwrap();

        let state = context.get_shared_state();
        assert_eq!(state, serde_json::json!({"initial": "value", "new": "data"}));
    }
}