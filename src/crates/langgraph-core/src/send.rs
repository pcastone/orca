//! Dynamic task creation and map-reduce patterns
//!
//! This module provides the [`Send`] type for creating dynamic parallel tasks within
//! graph execution. Send enables powerful patterns like map-reduce, fan-out/fan-in,
//! and dynamic parallelism where the number of parallel tasks is determined at runtime.
//!
//! # Overview
//!
//! Traditional graph edges route to fixed nodes determined at compile time. Send
//! objects allow conditional edges to **dynamically spawn multiple parallel tasks**
//! at runtime, each with its own custom state.
//!
//! ## Key Capabilities
//!
//! - **Map-Reduce**: Process collections in parallel, then aggregate results
//! - **Dynamic Fanout**: Create variable number of parallel tasks based on runtime data
//! - **Per-Task State**: Each parallel task gets its own custom input state
//! - **Tool Parallelism**: Execute multiple tool calls concurrently
//!
//! # When to Use Send
//!
//! Use Send when you need to:
//!
//! 1. **Process collections in parallel**: Split array into parallel tasks
//! 2. **Dynamic parallelism**: Number of parallel tasks determined at runtime
//! 3. **Custom state per task**: Each parallel task needs different input
//! 4. **LLM tool calling**: Execute multiple tool calls concurrently
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │           Conditional Edge (Router)                 │
//! │                                                     │
//! │  Input: ["item1", "item2", "item3"]                │
//! │                                                     │
//! │  Returns: [                                        │
//! │    Send("process", {"item": "item1"}),            │
//! │    Send("process", {"item": "item2"}),            │
//! │    Send("process", {"item": "item3"})             │
//! │  ]                                                  │
//! └──────┬─────────────┬─────────────┬──────────────────┘
//!        │             │             │
//!        ▼             ▼             ▼
//!   ┌────────┐   ┌────────┐   ┌────────┐
//!   │Process │   │Process │   │Process │  ← Parallel
//!   │ item1  │   │ item2  │   │ item3  │    Execution
//!   └────┬───┘   └────┬───┘   └────┬───┘
//!        │            │            │
//!        └────────────┴────────────┘
//!                     │
//!                     ▼
//!              ┌────────────┐
//!              │  Reduce    │
//!              │(Aggregate) │
//!              └────────────┘
//! ```
//!
//! # Examples
//!
//! ## Basic Map-Reduce
//!
//! ```rust
//! use langgraph_core::{StateGraph, Send};
//! use langgraph_core::send::ConditionalEdgeResult;
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! let mut graph = StateGraph::new();
//!
//! // Process individual items
//! graph.add_node("process_item", |state| {
//!     Box::pin(async move {
//!         let item = &state["item"];
//!         Ok(json!({"result": format!("processed_{}", item)}))
//!     })
//! });
//!
//! // Aggregate results
//! graph.add_node("aggregate", |state| {
//!     Box::pin(async move {
//!         Ok(json!({"done": true}))
//!     })
//! });
//!
//! // Map: Fan out to process items in parallel
//! graph.add_conditional_edge(
//!     "__start__",
//!     |state| {
//!         let items = state["items"].as_array().unwrap();
//!         let sends: Vec<Send> = items.iter()
//!             .map(|item| Send::new("process_item", json!({"item": item})))
//!             .collect();
//!         ConditionalEdgeResult::Sends(sends)
//!     },
//!     HashMap::new(),
//! );
//!
//! // Reduce: All tasks converge to aggregation
//! graph.add_edge("process_item", "aggregate");
//! ```
//!
//! ## Dynamic Tool Execution
//!
//! ```rust
//! use langgraph_core::{StateGraph, Send};
//! use langgraph_core::send::ConditionalEdgeResult;
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! let mut graph = StateGraph::new();
//!
//! // LLM node that decides which tools to call
//! graph.add_node("llm", |state| {
//!     Box::pin(async move {
//!         Ok(json!({
//!             "tool_calls": [
//!                 {"name": "search", "args": {"query": "weather"}},
//!                 {"name": "calculator", "args": {"expr": "2+2"}}
//!             ]
//!         }))
//!     })
//! });
//!
//! // Tool executor node
//! graph.add_node("execute_tool", |state| {
//!     Box::pin(async move {
//!         let tool_name = state["tool"]["name"].as_str().unwrap();
//!         // Execute tool...
//!         Ok(json!({"result": format!("{} executed", tool_name)}))
//!     })
//! });
//!
//! // Dynamic tool invocation
//! graph.add_conditional_edge(
//!     "llm",
//!     |state| {
//!         let tool_calls = state["tool_calls"].as_array().unwrap();
//!         let sends: Vec<Send> = tool_calls.iter()
//!             .map(|tool| Send::new("execute_tool", json!({"tool": tool})))
//!             .collect();
//!         ConditionalEdgeResult::Sends(sends)
//!     },
//!     HashMap::new(),
//! );
//! ```
//!
//! ## Conditional Parallel Processing
//!
//! ```rust
//! use langgraph_core::{StateGraph, Send};
//! use langgraph_core::send::ConditionalEdgeResult;
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! let mut graph = StateGraph::new();
//!
//! // Router that decides parallelism based on data size
//! graph.add_conditional_edge(
//!     "analyze",
//!     |state| {
//!         let items = state["items"].as_array().unwrap();
//!
//!         if items.len() > 10 {
//!             // Large dataset: parallel processing
//!             let sends: Vec<Send> = items.iter()
//!                 .map(|item| Send::new("process", json!({"item": item})))
//!                 .collect();
//!             ConditionalEdgeResult::Sends(sends)
//!         } else {
//!             // Small dataset: single-threaded
//!             ConditionalEdgeResult::Node("process_batch".to_string())
//!         }
//!     },
//!     HashMap::from([
//!         ("process_batch".to_string(), "process_batch".to_string()),
//!     ]),
//! );
//! ```
//!
//! # Performance Considerations
//!
//! - **Parallelism**: All Send tasks in a batch execute concurrently
//! - **State Isolation**: Each task gets its own state copy (no shared mutable state)
//! - **Checkpointing**: All parallel tasks complete before checkpoint
//! - **Error Handling**: If any task fails, the entire superstep fails
//!
//! # See Also
//!
//! - [`Send`] - Main type for dynamic task creation
//! - [`ConditionalEdgeResult`] - Return type for conditional edges
//! - [`StateGraph::add_conditional_edge`](crate::StateGraph::add_conditional_edge) - How to use Send

use crate::graph::NodeId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A message to send to a specific node with custom state
///
/// `Send` enables dynamic task creation within conditional edges.
/// When a conditional edge returns `Vec<Send>`, each `Send` creates
/// a separate task that executes the target node with the provided state.
///
/// This is particularly useful for:
/// - Map-reduce patterns (process multiple items in parallel)
/// - Dynamic fanout (spawn variable number of tasks)
/// - Parallel processing with different configurations
///
/// # Example
///
/// ```rust
/// use langgraph_core::Send;
///
/// // Create a Send to invoke "process" node with custom state
/// let send = Send::new("process", serde_json::json!({
///     "item": "data",
///     "config": {"parallel": true}
/// }));
///
/// assert_eq!(send.node(), "process");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Send {
    /// Target node to invoke
    node: NodeId,

    /// State to pass to the target node
    arg: Value,
}

impl Send {
    /// Create a new Send command
    ///
    /// # Arguments
    ///
    /// * `node` - The name of the target node to invoke
    /// * `arg` - The state to pass to the target node
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::Send;
    ///
    /// let send = Send::new("my_node", serde_json::json!({"key": "value"}));
    /// ```
    pub fn new(node: impl Into<NodeId>, arg: Value) -> Self {
        Self {
            node: node.into(),
            arg,
        }
    }

    /// Get the target node name
    pub fn node(&self) -> &str {
        &self.node
    }

    /// Get the argument/state for the node
    pub fn arg(&self) -> &Value {
        &self.arg
    }

    /// Consume the Send and return its parts
    pub fn into_parts(self) -> (NodeId, Value) {
        (self.node, self.arg)
    }
}

/// Return type for conditional edge router functions
///
/// `ConditionalEdgeResult` defines what happens after a conditional edge evaluates.
/// It supports three execution patterns:
///
/// 1. **Single Node**: Route to one specific node
/// 2. **Multiple Nodes**: Execute multiple nodes in parallel (with same state)
/// 3. **Send Objects**: Execute tasks with custom per-task state (map-reduce)
///
/// # Variants
///
/// ## Node - Single Node Routing
///
/// Routes execution to a single target node. This is the most common case for
/// conditional branching (if-else logic).
///
/// ```rust
/// use langgraph_core::send::ConditionalEdgeResult;
///
/// let result: ConditionalEdgeResult = "next_step".into();
/// ```
///
/// ## Nodes - Parallel Execution (Same State)
///
/// Executes multiple nodes in parallel, all receiving the same state. Use when
/// you need parallel processing but don't need different states per task.
///
/// ```rust
/// use langgraph_core::send::ConditionalEdgeResult;
///
/// let result = ConditionalEdgeResult::Nodes(vec![
///     "validate".to_string(),
///     "analyze".to_string(),
///     "log".to_string(),
/// ]);
/// ```
///
/// ## Sends - Parallel Execution (Custom States)
///
/// Executes multiple tasks in parallel, each with its own custom state. This is
/// the most powerful variant, enabling map-reduce and dynamic parallelism.
///
/// ```rust
/// use langgraph_core::{Send, send::ConditionalEdgeResult};
/// use serde_json::json;
///
/// let sends = vec![
///     Send::new("process", json!({"item": "A", "priority": "high"})),
///     Send::new("process", json!({"item": "B", "priority": "low"})),
/// ];
/// let result = ConditionalEdgeResult::Sends(sends);
/// ```
///
/// # Comparison
///
/// | Variant | Parallelism | Per-Task State | Use Case |
/// |---------|-------------|----------------|----------|
/// | `Node` | No | N/A | Simple branching |
/// | `Nodes` | Yes | No (shared) | Parallel independent operations |
/// | `Sends` | Yes | Yes (custom) | Map-reduce, tool calling |
///
/// # Examples
///
/// ## Conditional Branching
///
/// ```rust
/// use langgraph_core::send::ConditionalEdgeResult;
/// use serde_json::Value;
///
/// fn router(state: &Value) -> ConditionalEdgeResult {
///     if state["status"] == "approved" {
///         "process".into()
///     } else {
///         "reject".into()
///     }
/// }
/// ```
///
/// ## Parallel Validation
///
/// ```rust
/// use langgraph_core::send::ConditionalEdgeResult;
/// use serde_json::Value;
///
/// fn router(state: &Value) -> ConditionalEdgeResult {
///     // Run multiple validators in parallel
///     ConditionalEdgeResult::Nodes(vec![
///         "validate_schema".to_string(),
///         "validate_business_rules".to_string(),
///         "validate_permissions".to_string(),
///     ])
/// }
/// ```
///
/// ## Map-Reduce Processing
///
/// ```rust
/// use langgraph_core::{Send, send::ConditionalEdgeResult};
/// use serde_json::{Value, json};
///
/// fn router(state: &Value) -> ConditionalEdgeResult {
///     let items = state["items"].as_array().unwrap();
///     let sends: Vec<Send> = items.iter()
///         .map(|item| Send::new("process", json!({"item": item})))
///         .collect();
///     ConditionalEdgeResult::Sends(sends)
/// }
/// ```
///
/// # See Also
///
/// - [`Send`] - For custom per-task state
/// - [`StateGraph::add_conditional_edge`](crate::StateGraph::add_conditional_edge) - How to use this type
#[derive(Debug, Clone)]
pub enum ConditionalEdgeResult {
    /// Route to a single node
    ///
    /// The target node receives the current graph state and executes normally.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::send::ConditionalEdgeResult;
    ///
    /// let result: ConditionalEdgeResult = "next_step".into();
    /// ```
    Node(NodeId),

    /// Execute multiple nodes in parallel with the same state
    ///
    /// All listed nodes execute concurrently, each receiving a copy of the
    /// current graph state. Execution waits for all nodes to complete before
    /// proceeding to the next superstep.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::send::ConditionalEdgeResult;
    ///
    /// let result = ConditionalEdgeResult::Nodes(vec![
    ///     "node_a".to_string(),
    ///     "node_b".to_string(),
    /// ]);
    /// ```
    Nodes(Vec<NodeId>),

    /// Execute multiple tasks in parallel with custom states
    ///
    /// Each Send object specifies a target node and custom state for that task.
    /// All tasks execute concurrently. Use this for map-reduce patterns where
    /// each parallel task needs different input data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::{Send, send::ConditionalEdgeResult};
    /// use serde_json::json;
    ///
    /// let sends = vec![
    ///     Send::new("process", json!({"id": 1})),
    ///     Send::new("process", json!({"id": 2})),
    /// ];
    /// let result = ConditionalEdgeResult::Sends(sends);
    /// ```
    Sends(Vec<Send>),
}

impl From<&str> for ConditionalEdgeResult {
    fn from(node: &str) -> Self {
        ConditionalEdgeResult::Node(node.to_string())
    }
}

impl From<String> for ConditionalEdgeResult {
    fn from(node: String) -> Self {
        ConditionalEdgeResult::Node(node)
    }
}

impl From<Vec<String>> for ConditionalEdgeResult {
    fn from(nodes: Vec<String>) -> Self {
        ConditionalEdgeResult::Nodes(nodes)
    }
}

impl From<Vec<&str>> for ConditionalEdgeResult {
    fn from(nodes: Vec<&str>) -> Self {
        ConditionalEdgeResult::Nodes(nodes.iter().map(|s| s.to_string()).collect())
    }
}

impl From<Vec<Send>> for ConditionalEdgeResult {
    fn from(sends: Vec<Send>) -> Self {
        ConditionalEdgeResult::Sends(sends)
    }
}

impl From<Send> for ConditionalEdgeResult {
    fn from(send: Send) -> Self {
        ConditionalEdgeResult::Sends(vec![send])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_creation() {
        let send = Send::new("my_node", serde_json::json!({"key": "value"}));

        assert_eq!(send.node(), "my_node");
        assert_eq!(send.arg(), &serde_json::json!({"key": "value"}));
    }

    #[test]
    fn test_send_into_parts() {
        let send = Send::new("test", serde_json::json!({"data": 123}));
        let (node, arg) = send.into_parts();

        assert_eq!(node, "test");
        assert_eq!(arg, serde_json::json!({"data": 123}));
    }

    #[test]
    fn test_send_serialization() {
        let send = Send::new("process", serde_json::json!({"item": "test"}));
        let json = serde_json::to_string(&send).unwrap();

        let deserialized: Send = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.node(), "process");
        assert_eq!(deserialized.arg(), &serde_json::json!({"item": "test"}));
    }

    #[test]
    fn test_conditional_edge_result_from_node() {
        let result: ConditionalEdgeResult = "my_node".into();
        match result {
            ConditionalEdgeResult::Node(node) => assert_eq!(node, "my_node"),
            _ => panic!("Expected Node variant"),
        }
    }

    #[test]
    fn test_conditional_edge_result_from_send() {
        let send = Send::new("process", serde_json::json!({}));
        let result: ConditionalEdgeResult = send.into();

        match result {
            ConditionalEdgeResult::Sends(sends) => {
                assert_eq!(sends.len(), 1);
                assert_eq!(sends[0].node(), "process");
            }
            _ => panic!("Expected Sends variant"),
        }
    }

    #[test]
    fn test_conditional_edge_result_from_vec_sends() {
        let sends = vec![
            Send::new("node1", serde_json::json!({"id": 1})),
            Send::new("node2", serde_json::json!({"id": 2})),
        ];
        let result: ConditionalEdgeResult = sends.into();

        match result {
            ConditionalEdgeResult::Sends(sends) => {
                assert_eq!(sends.len(), 2);
                assert_eq!(sends[0].node(), "node1");
                assert_eq!(sends[1].node(), "node2");
            }
            _ => panic!("Expected Sends variant"),
        }
    }

    #[test]
    fn test_map_reduce_pattern() {
        // Simulate map-reduce: create multiple sends from a list
        let items = vec!["apple", "banana", "cherry"];
        let sends: Vec<Send> = items
            .iter()
            .map(|item| Send::new("process", serde_json::json!({"fruit": item})))
            .collect();

        assert_eq!(sends.len(), 3);
        assert_eq!(sends[0].node(), "process");
        assert_eq!(sends[0].arg()["fruit"], "apple");
        assert_eq!(sends[1].arg()["fruit"], "banana");
        assert_eq!(sends[2].arg()["fruit"], "cherry");
    }
}
