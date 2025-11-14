//! StateGraph builder API for constructing stateful graph workflows
//!
//! This module provides the primary API for constructing stateful, multi-actor applications
//! with LLMs. The [`StateGraph`] builder uses a fluent interface to define nodes, edges,
//! and conditional routing, then compiles into an executable workflow with automatic state
//! management, checkpointing, and parallel execution.
//!
//! # Overview
//!
//! `StateGraph` is the main way developers build graphs in rLangGraph. It handles:
//!
//! - **Node Management**: Add async processing nodes
//! - **Edge Routing**: Define static and conditional edges
//! - **State Channels**: Automatic shared state with custom reducers
//! - **Graph Validation**: Ensure structure is valid before execution
//! - **Compilation**: Transform builder into executable [`CompiledGraph`](crate::compiled::CompiledGraph)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  StateGraph (Builder)                                   │
//! │                                                          │
//! │  ┌──────────┐   ┌──────────┐   ┌──────────┐           │
//! │  │  Node A  │──→│  Node B  │──→│  Node C  │           │
//! │  └──────────┘   └──────────┘   └──────────┘           │
//! │        ↓              ↓              ↓                  │
//! │  ┌───────────────────────────────────────┐             │
//! │  │     Shared State Channel              │             │
//! │  │  (automatic merge/reduce)             │             │
//! │  └───────────────────────────────────────┘             │
//! └─────────────────────────────────────────────────────────┘
//!                       │ compile()
//!                       ↓
//! ┌─────────────────────────────────────────────────────────┐
//! │  CompiledGraph (Executable)                            │
//! │                                                          │
//! │  • Pregel-based execution                               │
//! │  • Parallel node processing                             │
//! │  • Checkpointing support                                │
//! │  • Streaming capabilities                               │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Types
//!
//! - [`StateGraph`] - The main builder for creating graphs
//! - [`CompiledGraph`](crate::compiled::CompiledGraph) - The executable graph after compilation
//! - [`NodeSpec`](crate::graph::NodeSpec) - Specification for node behavior
//! - [`ChannelSpec`](crate::graph::ChannelSpec) - Configuration for state channels
//!
//! # Quick Start
//!
//! ## Basic Linear Flow
//!
//! Build a simple sequential workflow:
//!
//! ```rust,no_run
//! use langgraph_core::StateGraph;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut graph = StateGraph::new();
//!
//! // Add processing nodes
//! graph.add_node("step1", |state| {
//!     Box::pin(async move {
//!         println!("Processing step 1");
//!         Ok(state)
//!     })
//! });
//!
//! graph.add_node("step2", |state| {
//!     Box::pin(async move {
//!         println!("Processing step 2");
//!         Ok(state)
//!     })
//! });
//!
//! // Define flow: START -> step1 -> step2 -> END
//! graph.add_edge("__start__", "step1");
//! graph.add_edge("step1", "step2");
//! graph.add_edge("step2", "__end__");
//!
//! // Compile and execute
//! let compiled = graph.compile()?;
//! let result = compiled.invoke(json!({"input": "data"})).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Conditional Routing
//!
//! Add dynamic routing based on state:
//!
//! ```rust,no_run
//! use langgraph_core::StateGraph;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut graph = StateGraph::new();
//!
//! graph.add_node("router", |state| {
//!     Box::pin(async move {
//!         // Analyze state to determine route
//!         Ok(state)
//!     })
//! });
//!
//! graph.add_node("path_a", |state| {
//!     Box::pin(async move {
//!         println!("Taking path A");
//!         Ok(state)
//!     })
//! });
//!
//! graph.add_node("path_b", |state| {
//!     Box::pin(async move {
//!         println!("Taking path B");
//!         Ok(state)
//!     })
//! });
//!
//! // Add conditional routing based on state value
//! graph.add_conditional_edge("router", |state| {
//!     let value = state.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
//!     if value > 100 {
//!         vec!["path_a"]
//!     } else {
//!         vec!["path_b"]
//!     }
//! });
//!
//! graph.add_edge("path_a", "__end__");
//! graph.add_edge("path_b", "__end__");
//!
//! let compiled = graph.compile()?;
//! # Ok(())
//! # }
//! ```
//!
//! # State Management
//!
//! StateGraph automatically creates a shared state channel using a merge reducer:
//!
//! ```rust,ignore
//! // Nodes receive and update state
//! graph.add_node("process", |state| {
//!     Box::pin(async move {
//!         let mut s = state.as_object().unwrap().clone();
//!         s.insert("processed".to_string(), json!(true));
//!         Ok(json!(s))  // Updates merged automatically
//!     })
//! });
//! ```
//!
//! ## Custom Channels
//!
//! Add custom channels with specific reducers:
//!
//! ```rust,ignore
//! use langgraph_core::graph::ChannelType;
//!
//! // Add a channel that appends values instead of overwriting
//! graph.add_channel("logs", ChannelType::Topic, None);
//! ```
//!
//! # Common Patterns
//!
//! ## Fan-Out/Fan-In (Parallel Processing)
//!
//! Process multiple paths in parallel, then merge:
//!
//! ```rust,ignore
//! // Fan-out: router -> [worker1, worker2, worker3]
//! graph.add_conditional_edge("router", |_state| {
//!     vec!["worker1", "worker2", "worker3"]  // All execute in parallel
//! });
//!
//! // Fan-in: All workers -> aggregator
//! graph.add_edge("worker1", "aggregator");
//! graph.add_edge("worker2", "aggregator");
//! graph.add_edge("worker3", "aggregator");
//! ```
//!
//! ## Loops (Iterative Refinement)
//!
//! Create cycles for iterative processing:
//!
//! ```rust,ignore
//! graph.add_node("process", |state| { /* ... */ });
//! graph.add_node("check_quality", |state| { /* ... */ });
//!
//! // Loop back if quality check fails
//! graph.add_conditional_edge("check_quality", |state| {
//!     if state["quality"].as_f64().unwrap_or(0.0) < 0.8 {
//!         vec!["process"]  // Try again
//!     } else {
//!         vec!["__end__"]  // Done
//!     }
//! });
//! ```
//!
//! ## Subgraphs (Composition)
//!
//! Embed compiled graphs as nodes:
//!
//! ```rust,ignore
//! let child_graph = StateGraph::new();
//! // ... configure child ...
//! let child_compiled = child_graph.compile()?;
//!
//! // Add as node in parent graph
//! graph.add_subgraph("child_workflow", child_compiled);
//! ```
//!
//! ## Map-Reduce Pattern
//!
//! Use [`Send`](crate::Send) for dynamic parallel execution:
//!
//! ```rust,ignore
//! use langgraph_core::Send;
//!
//! graph.add_conditional_edge("fanout", |state| {
//!     let items = state["items"].as_array().unwrap();
//!     items.iter().map(|item| {
//!         Send::new("process_item", json!({"item": item}))
//!     }).collect()
//! });
//! ```
//!
//! # Advanced Features
//!
//! ## Human-in-the-Loop (Interrupts)
//!
//! Configure execution to pause for approval:
//!
//! ```rust,ignore
//! use langgraph_core::InterruptConfig;
//!
//! let interrupt_config = InterruptConfig::new()
//!     .with_interrupt_before(vec!["approval_required".to_string()]);
//!
//! let compiled = graph.compile_with_interrupts(interrupt_config)?;
//! ```
//!
//! ## Persistent Storage
//!
//! Add a Store for persistent data:
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use langgraph_core::store::InMemoryStore;
//!
//! let store = Arc::new(InMemoryStore::new());
//! let compiled = graph.compile_with_store(store)?;
//! ```
//!
//! # Performance Tips
//!
//! 1. **Parallel Nodes**: Nodes without dependencies execute concurrently
//! 2. **State Size**: Keep state minimal - avoid large objects
//! 3. **Node Granularity**: Balance between too fine-grained (overhead) and too coarse (less parallelism)
//! 4. **Channel Reducers**: Simple reducers (overwrite) are faster than complex merges
//! 5. **Validation**: Always call `compile()` which validates the graph structure
//!
//! # Best Practices
//!
//! 1. **Single Responsibility**: Each node should do one thing well
//! 2. **Meaningful Names**: Use descriptive node names for debugging and visualization
//! 3. **Error Handling**: Handle errors within nodes when possible, propagate when necessary
//! 4. **State Shape**: Maintain consistent state structure across nodes
//! 5. **Testing**: Test nodes individually before integrating into graph
//! 6. **Validation**: Use visualization to verify graph structure
//!
//! # Comparison with Python LangGraph
//!
//! | Python LangGraph | rLangGraph | Notes |
//! |------------------|------------|-------|
//! | `StateGraph(state_schema)` | `StateGraph::new()` | Rust uses JSON state |
//! | `add_node(name, func)` | `add_node(name, closure)` | Closures must be `Box::pin(async move)` |
//! | `add_edge(from, to)` | `add_edge(from, to)` | Identical API |
//! | `add_conditional_edges(source, path_func)` | `add_conditional_edge(source, path_func)` | Returns `Vec<String>` |
//! | `compile(checkpointer=...)` | `compile()?.with_checkpointer(saver)` | Fluent API in Rust |
//! | `graph.stream(input)` | `compiled.stream(input).await?` | Async in Rust |
//!
//! # See Also
//!
//! - [`CompiledGraph`](crate::compiled::CompiledGraph) - Executing compiled graphs
//! - [`MessageGraph`](crate::message_graph::MessageGraph) - Alternative message-based API
//! - [`state`](crate::state) - State schema and reducers
//! - [`Send`](crate::Send) - Dynamic message passing
//! - [`InterruptConfig`](crate::interrupt::InterruptConfig) - Human-in-the-loop configuration

use crate::graph::{ChannelSpec, ChannelType, Graph, NodeExecutor, NodeId, NodeSpec, ReducerFn, END};
use crate::compiled::CompiledGraph;
use crate::error::{GraphError, Result};
use crate::interrupt::InterruptConfig;
use std::collections::HashMap;
use std::sync::Arc;

/// Builder for constructing state graphs with shared state management
///
/// `StateGraph` is the primary way to build executable graphs in rLangGraph.
/// It provides a fluent API for defining nodes (processing steps) and edges
/// (transitions) between them. The graph maintains shared state that flows
/// through all nodes during execution.
///
/// ## State Management
///
/// StateGraph automatically creates a shared state channel that all nodes can
/// read from and write to. State updates are merged using a default reducer
/// that combines object fields.
///
/// ## Node Execution
///
/// Nodes are async functions that:
/// - Receive the current state as input
/// - Process the state
/// - Return the modified state or an error
///
/// ## Examples
///
/// ### Basic Usage
///
/// ```rust,no_run
/// use langgraph_core::StateGraph;
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut graph = StateGraph::new();
///
/// // Add a processing node
/// graph.add_node("process", |state| {
///     Box::pin(async move {
///         let mut s = state.as_object().unwrap().clone();
///         s.insert("processed".to_string(), json!(true));
///         Ok(json!(s))
///     })
/// });
///
/// // Define the flow
/// graph.add_edge("__start__", "process");
/// graph.add_edge("process", "__end__");
///
/// // Compile and execute
/// let compiled = graph.compile()?;
/// let result = compiled.invoke(json!({"input": "data"})).await?;
/// assert_eq!(result.get("processed"), Some(&json!(true)));
/// # Ok(())
/// # }
/// ```
///
/// ### With Conditional Routing
///
/// ```rust,no_run
/// use langgraph_core::StateGraph;
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut graph = StateGraph::new();
///
/// graph.add_node("classifier", |state| {
///     Box::pin(async move {
///         let value = state.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
///         let mut s = state.as_object().unwrap().clone();
///         s.insert("category".to_string(), json!(if value > 100 { "high" } else { "low" }));
///         Ok(json!(s))
///     })
/// });
///
/// graph.add_conditional_edges("classifier", |state| {
///     match state.get("category").and_then(|v| v.as_str()) {
///         Some("high") => vec!["process_high"],
///         Some("low") => vec!["process_low"],
///         _ => vec!["__end__"]
///     }
/// });
///
/// # Ok(())
/// # }
/// ```
pub struct StateGraph {
    graph: Graph,
}

impl StateGraph {
    /// Creates a new state graph builder with default shared state channel
    ///
    /// This initializes an empty graph with a shared state channel that uses
    /// a merge reducer for combining state updates from different nodes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use langgraph_core::StateGraph;
    ///
    /// let mut graph = StateGraph::new();
    /// // Graph is ready for nodes and edges to be added
    /// ```
    pub fn new() -> Self {
        let mut graph = Self {
            graph: Graph::new(),
        };

        // Add a default shared state channel for proper state sharing
        let reducer: ReducerFn = Arc::new(|left: serde_json::Value, right: serde_json::Value| {
            // Merge objects, with right overwriting left
            if let (Some(left_obj), Some(right_obj)) = (left.as_object(), right.as_object()) {
                let mut merged = left_obj.clone();
                for (key, value) in right_obj {
                    merged.insert(key.clone(), value.clone());
                }
                serde_json::Value::Object(merged)
            } else {
                // If not both objects, use right value
                right
            }
        });

        graph.add_channel("state", ChannelType::LastValue, Some(reducer));
        graph
    }

    /// Create a state graph with shared state channel
    ///
    /// This creates a graph where all nodes share a single "state" channel with a merge reducer.
    /// This enables proper state sharing across all nodes in the graph.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::StateGraph;
    /// use serde_json::json;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut graph = StateGraph::with_state();
    ///
    /// graph.add_node("step1", |mut state| {
    ///     Box::pin(async move {
    ///         // Modify state
    ///         if let Some(obj) = state.as_object_mut() {
    ///             obj.insert("step1_done".to_string(), json!(true));
    ///         }
    ///         Ok(state)
    ///     })
    /// });
    ///
    /// graph.add_node("step2", |mut state| {
    ///     Box::pin(async move {
    ///         // Can see step1's changes
    ///         if let Some(obj) = state.as_object_mut() {
    ///             obj.insert("step2_done".to_string(), json!(true));
    ///         }
    ///         Ok(state)
    ///     })
    /// });
    ///
    /// graph.add_edge("__start__", "step1");
    /// graph.add_edge("step1", "step2");
    /// graph.add_edge("step2", "__end__");
    ///
    /// let compiled = graph.compile()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_state() -> Self {
        // Just call new() which now creates a shared state channel by default
        Self::new()
    }

    /// Create a state graph with messages support (recommended pattern)
    ///
    /// This is the recommended way to build message-based agents in the modern API.
    /// It creates a graph with a "messages" channel that uses the `add_messages` reducer
    /// for intelligent message history management with ID-based deduplication.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::{StateGraph, Message};
    /// use serde_json::json;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut graph = StateGraph::with_messages();
    ///
    /// graph.add_node("chatbot", |state| {
    ///     Box::pin(async move {
    ///         // Access messages from state
    ///         let messages = state.get("messages").unwrap().as_array().unwrap();
    ///
    ///         // Add response
    ///         Ok(json!({
    ///             "messages": [Message::assistant("Hello!")]
    ///         }))
    ///     })
    /// });
    ///
    /// graph.add_edge("__start__", "chatbot");
    /// graph.add_edge("chatbot", "__end__");
    ///
    /// let compiled = graph.compile()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_messages() -> Self {
        let mut graph = Self::new();

        // Remove the default "state" channel since we're using "messages" instead
        // This ensures compile() will use "messages" as the primary state channel
        graph.graph.channels.remove("state");

        // Add messages channel with add_messages reducer
        let reducer: ReducerFn = Arc::new(|left: serde_json::Value, right: serde_json::Value| {
            use crate::messages::add_messages;

            // Convert values to message arrays
            let left_msgs = if let Some(arr) = left.as_array() {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            } else {
                vec![]
            };

            let right_msgs = if let Some(arr) = right.as_array() {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            } else {
                vec![]
            };

            // Merge using add_messages
            let merged = add_messages(left_msgs, right_msgs);

            // Convert back to JSON
            serde_json::Value::Array(
                merged
                    .into_iter()
                    .map(|m| serde_json::to_value(m).unwrap())
                    .collect(),
            )
        });

        graph.add_channel("messages", ChannelType::LastValue, Some(reducer));
        graph
    }

    /// Add a node to the graph
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the node
    /// * `executor` - Async function that processes state
    ///
    /// # Returns
    ///
    /// Mutable reference to self for method chaining
    /// Adds a processing node to the graph
    ///
    /// Nodes are the primary processing units in a graph. Each node receives
    /// the current state, processes it, and returns the modified state.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the node (e.g., "process", "transform")
    /// * `executor` - Async function that processes state
    ///
    /// # Returns
    ///
    /// Returns `&mut Self` for method chaining
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use langgraph_core::StateGraph;
    /// use serde_json::json;
    ///
    /// let mut graph = StateGraph::new();
    ///
    /// // Add a simple processing node
    /// graph.add_node("uppercase", |state| {
    ///     Box::pin(async move {
    ///         let text = state.get("text").and_then(|v| v.as_str()).unwrap_or("");
    ///         let mut s = state.as_object().unwrap().clone();
    ///         s.insert("text".to_string(), json!(text.to_uppercase()));
    ///         Ok(json!(s))
    ///     })
    /// });
    ///
    /// // Add a node that can fail
    /// graph.add_node("validate", |state| {
    ///     Box::pin(async move {
    ///         let value = state.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
    ///         if value < 0 {
    ///             return Err(GraphError::Execution("Value cannot be negative".into()));
    ///         }
    ///         Ok(state)
    ///     })
    /// });
    /// ```
    pub fn add_node<F>(&mut self, id: impl Into<NodeId>, executor: F) -> &mut Self
    where
        F: Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let id = id.into();
        let executor: NodeExecutor = Arc::new(move |state| {
            let fut = executor(state);
            Box::pin(async move {
                fut.await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
        });

        // For StateGraph nodes, we'll set up proper reads/writes during compilation
        // For now, create with empty reads/writes
        let spec = NodeSpec {
            name: id.clone(),
            executor,
            reads: vec![],
            writes: vec![],
            subgraph: None,
        };

        self.graph.add_node(id.clone(), spec);

        // Only add individual node channels if not using shared state
        if !self.graph.channels.contains_key("state") && !self.graph.channels.contains_key("messages") {
            // Add a channel for this node to write to
            if !id.starts_with("__") {
                self.add_channel(id, ChannelType::LastValue, None);
            }
        }

        self
    }

    /// Add a node with full specification
    pub fn add_node_spec(&mut self, id: impl Into<NodeId>, spec: NodeSpec) -> &mut Self {
        self.graph.add_node(id.into(), spec);
        self
    }

    /// Add a node with a pre-built executor
    ///
    /// This is useful when you need more control over the node executor,
    /// such as when creating subgraphs or using custom executors.
    ///
    /// # Arguments
    ///
    /// * `id` - Node identifier
    /// * `executor` - Pre-built node executor
    ///
    /// # Returns
    ///
    /// Mutable reference to self for method chaining
    pub fn add_node_with_executor(&mut self, id: impl Into<NodeId>, executor: NodeExecutor) -> &mut Self {
        let id = id.into();
        let spec = NodeSpec {
            name: id.clone(),
            executor,
            reads: vec![],
            writes: vec![],
            subgraph: None,
        };
        self.add_node_spec(id, spec)
    }

    /// Add a subgraph as a node
    ///
    /// A subgraph is a compiled graph that executes as a single node in the parent graph.
    /// State flows from parent → subgraph → parent.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the subgraph node
    /// * `subgraph` - Compiled graph to use as a node
    ///
    /// # Returns
    ///
    /// Mutable reference to self for method chaining
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::StateGraph;
    ///
    /// // Create a subgraph
    /// let mut sub = StateGraph::new();
    /// sub.add_node("process", |state| {
    ///     Box::pin(async move { Ok(state) })
    /// });
    /// sub.add_edge("__start__", "process");
    /// sub.add_edge("process", "__end__");
    /// let compiled_sub = sub.compile().unwrap();
    ///
    /// // Add as a node in parent graph
    /// let mut parent = StateGraph::new();
    /// parent.add_subgraph("subprocess", compiled_sub);
    /// parent.add_edge("__start__", "subprocess");
    /// parent.add_edge("subprocess", "__end__");
    /// ```
    pub fn add_subgraph(&mut self, id: impl Into<NodeId>, subgraph: CompiledGraph) -> &mut Self {
        let id = id.into();
        let subgraph_arc = Arc::new(subgraph);
        let subgraph_clone = subgraph_arc.clone();

        // Wrap the subgraph's invoke as a node executor
        let executor: NodeExecutor = Arc::new(move |state| {
            let subgraph = subgraph_clone.clone();
            Box::pin(async move {
                // Execute the subgraph with the parent's state as input
                subgraph.invoke(state)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
        });

        let spec = NodeSpec {
            name: id.clone(),
            executor,
            reads: vec![],
            writes: vec![],
            subgraph: Some(subgraph_arc),
        };

        self.graph.add_node(id, spec);
        self
    }

    /// Add a direct edge from one node to another
    ///
    /// # Arguments
    ///
    /// * `from` - Source node ID
    /// * `to` - Target node ID
    /// Adds a directed edge between two nodes
    ///
    /// Edges define the flow of execution through the graph. When a node
    /// completes, execution continues to all nodes connected by outgoing edges.
    ///
    /// # Arguments
    ///
    /// * `from` - Source node ID (can be "__start__" for graph entry)
    /// * `to` - Target node ID (can be "__end__" for graph exit)
    ///
    /// # Returns
    ///
    /// Returns `&mut Self` for method chaining
    ///
    /// # Special Nodes
    ///
    /// - `__start__` - Virtual entry point of the graph
    /// - `__end__` - Virtual exit point of the graph
    ///
    /// # Examples
    ///
    /// ```rust
    /// use langgraph_core::StateGraph;
    ///
    /// let mut graph = StateGraph::new();
    ///
    /// graph.add_node("step1", |state| {
    ///     Box::pin(async move { Ok(state) })
    /// });
    ///
    /// graph.add_node("step2", |state| {
    ///     Box::pin(async move { Ok(state) })
    /// });
    ///
    /// // Create a linear flow
    /// graph.add_edge("__start__", "step1");
    /// graph.add_edge("step1", "step2");
    /// graph.add_edge("step2", "__end__");
    ///
    /// // Create parallel branches
    /// graph.add_edge("__start__", "branch1");
    /// graph.add_edge("__start__", "branch2");
    /// graph.add_edge("branch1", "__end__");
    /// graph.add_edge("branch2", "__end__");
    /// ```
    pub fn add_edge(&mut self, from: impl Into<NodeId>, to: impl Into<NodeId>) -> &mut Self {
        self.graph.add_edge(from.into(), to.into());
        self
    }

    /// Add a conditional edge that routes based on state
    ///
    /// # Arguments
    ///
    /// * `from` - Source node ID
    /// * `router` - Function that examines state and returns the next node(s) or Send objects
    /// * `branches` - Map of branch names to node IDs (for validation)
    pub fn add_conditional_edge<F>(
        &mut self,
        from: impl Into<NodeId>,
        router: F,
        branches: HashMap<String, NodeId>,
    ) -> &mut Self
    where
        F: Fn(&serde_json::Value) -> crate::send::ConditionalEdgeResult + Send + Sync + 'static,
    {
        self.graph.add_conditional_edge(from.into(), Arc::new(router), branches);
        self
    }

    /// Set the entry point of the graph
    ///
    /// This method both sets the entry point and adds an edge from START to the specified node,
    /// ensuring the node is triggered on the first execution.
    ///
    /// # Arguments
    ///
    /// * `node` - The node ID to use as entry point
    pub fn set_entry(&mut self, node: impl Into<NodeId>) -> &mut Self {
        let node_id = node.into();
        self.graph.set_entry(node_id.clone());
        // Also add edge from START to ensure the node is triggered
        self.add_edge("__start__", node_id);
        self
    }

    /// Add a finish point (edge to END)
    ///
    /// # Arguments
    ///
    /// * `node` - The node ID that should connect to END
    pub fn add_finish(&mut self, node: impl Into<NodeId>) -> &mut Self {
        self.graph.add_edge(node.into(), END.to_string());
        self
    }

    /// Add a channel to the state
    ///
    /// # Arguments
    ///
    /// * `name` - Channel name
    /// * `channel_type` - Type of channel (LastValue, Topic, BinaryOp)
    /// * `reducer` - Optional reducer function for BinaryOp channels
    pub fn add_channel(
        &mut self,
        name: impl Into<String>,
        channel_type: ChannelType,
        reducer: Option<ReducerFn>,
    ) -> &mut Self {
        let name = name.into();
        self.graph.channels.insert(
            name.clone(),
            ChannelSpec {
                name,
                channel_type,
                reducer,
            },
        );
        self
    }

    /// Compile the graph into an executable form
    ///
    /// # Returns
    ///
    /// A compiled graph ready for execution
    ///
    /// # Errors
    ///
    /// Returns an error if the graph structure is invalid
    /// Compiles the graph into an executable form
    ///
    /// This method validates the graph structure and creates a [`CompiledGraph`]
    /// that can be executed. The compilation process:
    ///
    /// 1. Validates graph structure (no missing nodes, valid edges)
    /// 2. Sets up state channels and reducers
    /// 3. Configures node specifications for state sharing
    /// 4. Prepares the execution engine
    ///
    /// # Returns
    ///
    /// Returns `Ok(CompiledGraph)` if compilation succeeds, or an error if
    /// the graph structure is invalid.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The graph has no nodes
    /// - Referenced nodes don't exist
    /// - The graph has circular dependencies without exit conditions
    /// - Channel configurations are invalid
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use langgraph_core::StateGraph;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut graph = StateGraph::new();
    ///
    /// graph.add_node("process", |state| {
    ///     Box::pin(async move { Ok(state) })
    /// });
    ///
    /// graph.add_edge("__start__", "process");
    /// graph.add_edge("process", "__end__");
    ///
    /// // Compile the graph
    /// let compiled = graph.compile()?;
    ///
    /// // Now ready for execution
    /// let result = compiled.invoke(json!({"input": "data"})).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # With Checkpointing
    ///
    /// ```rust,no_run
    /// use langgraph_core::StateGraph;
    /// use langgraph_checkpoint::InMemoryCheckpointSaver;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut graph = StateGraph::new();
    /// // ... add nodes and edges ...
    ///
    /// let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
    /// let compiled = graph.compile()?.with_checkpointer(checkpointer);
    /// # Ok(())
    /// # }
    /// ```
    pub fn compile(mut self) -> Result<CompiledGraph> {
        // Fix up node specs to ensure proper state sharing in StateGraph
        // If there's a shared state channel, all nodes should read from and write to it
        let has_state_channel = self.graph.channels.contains_key("state");
        let has_messages_channel = self.graph.channels.contains_key("messages");

        if has_state_channel || has_messages_channel {
            // Using shared state or messages - nodes should read from and write to it
            let channel_name = if has_state_channel { "state" } else { "messages" };

            // Remove any individual node channels that were created
            let node_names: Vec<String> = self.graph.nodes.keys().cloned().collect();
            for node_name in &node_names {
                self.graph.channels.remove(node_name);
            }

            // Configure nodes to use the shared channel
            for (_, spec) in self.graph.nodes.iter_mut() {
                spec.reads = vec![channel_name.to_string()];
                spec.writes = vec![channel_name.to_string()];
            }
        } else {
            // Standard StateGraph - each node reads from all other node channels
            // and writes to its own channel
            let node_names: Vec<String> = self.graph.nodes.keys()
                .filter(|n| !n.starts_with("__"))
                .cloned()
                .collect();

            for (node_id, spec) in self.graph.nodes.iter_mut() {
                // Node reads from all other node channels (for state merging)
                spec.reads = node_names.iter()
                    .filter(|n| n != &node_id)
                    .cloned()
                    .collect();
                // Node writes to its own channel
                spec.writes = vec![node_id.clone()];
            }
        }

        // Validate the graph structure
        self.graph.validate().map_err(GraphError::Validation)?;

        // Create compiled graph
        CompiledGraph::new(self.graph)
    }

    /// Compile the graph with a store for persistent state
    ///
    /// # Arguments
    ///
    /// * `store` - Store implementation for persistent state access
    ///
    /// # Returns
    ///
    /// A compiled graph ready for execution with store available
    ///
    /// # Errors
    ///
    /// Returns an error if the graph structure is invalid
    pub fn compile_with_store(self, store: Arc<dyn crate::store::Store>) -> Result<CompiledGraph> {
        // Validate the graph structure
        self.graph.validate().map_err(GraphError::Validation)?;

        // Create compiled graph with store
        CompiledGraph::new(self.graph).map(|g| g.with_store(store))
    }

    /// Compile the graph with interrupt configuration
    ///
    /// # Arguments
    ///
    /// * `interrupt_config` - Configuration for interrupting execution
    ///
    /// # Returns
    ///
    /// A compiled graph ready for execution with interrupts enabled
    ///
    /// # Errors
    ///
    /// Returns an error if the graph structure is invalid
    pub fn compile_with_interrupts(self, interrupt_config: InterruptConfig) -> Result<CompiledGraph> {
        // Validate the graph structure
        self.graph.validate().map_err(GraphError::Validation)?;

        // Create compiled graph with interrupts
        CompiledGraph::new_with_interrupts(self.graph, interrupt_config)
    }

    /// Get a reference to the underlying graph
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Get a mutable reference to the underlying graph
    pub fn graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }
}

impl Default for StateGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_graph() {
        let mut graph = StateGraph::new();

        graph.add_node("process", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("processed".to_string(), serde_json::json!(true));
                }
                Ok(state)
            })
        });

        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let compiled = graph.compile();
        assert!(compiled.is_ok());
    }

    #[tokio::test]
    async fn test_conditional_routing() {
        let mut graph = StateGraph::new();

        graph.add_node("start", |state| {
            Box::pin(async move { Ok(state) })
        });

        graph.add_node("path_a", |state| {
            Box::pin(async move { Ok(state) })
        });

        graph.add_node("path_b", |state| {
            Box::pin(async move { Ok(state) })
        });

        let mut branches = HashMap::new();
        branches.insert("a".to_string(), "path_a".to_string());
        branches.insert("b".to_string(), "path_b".to_string());

        graph.add_edge("__start__", "start");
        graph.add_conditional_edge(
            "start",
            |state| {
                use crate::send::ConditionalEdgeResult;
                if let Some(choice) = state.get("choice").and_then(|v| v.as_str()) {
                    if choice == "a" {
                        return ConditionalEdgeResult::Node("path_a".to_string());
                    }
                }
                ConditionalEdgeResult::Node("path_b".to_string())
            },
            branches,
        );

        graph.add_finish("path_a");
        graph.add_finish("path_b");

        let compiled = graph.compile();
        assert!(compiled.is_ok());
    }

    #[test]
    fn test_graph_validation_error() {
        let mut graph = StateGraph::new();

        // Add edge to non-existent node
        graph.add_edge("__start__", "nonexistent");

        let result = graph.compile();
        assert!(result.is_err());
    }

    // ===== SUBGRAPH TESTS =====

    #[tokio::test]
    async fn test_simple_subgraph() {
        // Create a subgraph that increments a counter
        let mut subgraph = StateGraph::new();
        subgraph.add_node("increment", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    let count = obj.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
                    obj.insert("count".to_string(), serde_json::json!(count + 1));
                }
                Ok(state)
            })
        });
        subgraph.add_edge("__start__", "increment");
        subgraph.add_edge("increment", "__end__");

        let compiled_subgraph = subgraph.compile().unwrap();

        // Create parent graph with subgraph as a node
        let mut parent = StateGraph::new();
        parent.add_subgraph("subprocess", compiled_subgraph);
        parent.add_edge("__start__", "subprocess");
        parent.add_edge("subprocess", "__end__");

        let compiled_parent = parent.compile().unwrap();

        // Execute parent graph
        let input = serde_json::json!({"count": 0});
        let output = compiled_parent.invoke(input).await.unwrap();

        // Debug: print actual output
        eprintln!("Test output: {}", serde_json::to_string_pretty(&output).unwrap());

        // Verify subgraph executed and modified state
        assert_eq!(output.get("count"), Some(&serde_json::json!(1)));
    }

    #[tokio::test]
    async fn test_nested_subgraphs() {
        // Create innermost subgraph (adds 10)
        let mut inner_sub = StateGraph::new();
        inner_sub.add_node("add_ten", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    obj.insert("value".to_string(), serde_json::json!(val + 10));
                }
                Ok(state)
            })
        });
        inner_sub.add_edge("__start__", "add_ten");
        inner_sub.add_edge("add_ten", "__end__");
        let compiled_inner = inner_sub.compile().unwrap();

        // Create middle subgraph (multiplies by 2, then calls inner subgraph)
        let mut middle_sub = StateGraph::new();
        middle_sub.add_node("multiply", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    obj.insert("value".to_string(), serde_json::json!(val * 2));
                }
                Ok(state)
            })
        });
        middle_sub.add_subgraph("inner_process", compiled_inner);
        middle_sub.add_edge("__start__", "multiply");
        middle_sub.add_edge("multiply", "inner_process");
        middle_sub.add_edge("inner_process", "__end__");
        let compiled_middle = middle_sub.compile().unwrap();

        // Create parent graph (adds 1, then calls middle subgraph)
        let mut parent = StateGraph::new();
        parent.add_node("add_one", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    obj.insert("value".to_string(), serde_json::json!(val + 1));
                }
                Ok(state)
            })
        });
        parent.add_subgraph("middle_process", compiled_middle);
        parent.add_edge("__start__", "add_one");
        parent.add_edge("add_one", "middle_process");
        parent.add_edge("middle_process", "__end__");

        let compiled_parent = parent.compile().unwrap();

        // Execute: 5 + 1 = 6, then 6 * 2 = 12, then 12 + 10 = 22
        let input = serde_json::json!({"value": 5});
        let output = compiled_parent.invoke(input).await.unwrap();

        assert_eq!(output.get("value"), Some(&serde_json::json!(22)));
    }

    #[tokio::test]
    async fn test_subgraph_with_multiple_nodes() {
        // Create a subgraph with multiple processing steps
        let mut subgraph = StateGraph::new();

        subgraph.add_node("step1", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("step1".to_string(), serde_json::json!(true));
                }
                Ok(state)
            })
        });

        subgraph.add_node("step2", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("step2".to_string(), serde_json::json!(true));
                }
                Ok(state)
            })
        });

        subgraph.add_edge("__start__", "step1");
        subgraph.add_edge("step1", "step2");
        subgraph.add_edge("step2", "__end__");

        let compiled_subgraph = subgraph.compile().unwrap();

        // Create parent with subgraph and additional nodes
        let mut parent = StateGraph::new();

        parent.add_node("before", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("before".to_string(), serde_json::json!(true));
                }
                Ok(state)
            })
        });

        parent.add_subgraph("subprocess", compiled_subgraph);

        parent.add_node("after", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    obj.insert("after".to_string(), serde_json::json!(true));
                }
                Ok(state)
            })
        });

        parent.add_edge("__start__", "before");
        parent.add_edge("before", "subprocess");
        parent.add_edge("subprocess", "after");
        parent.add_edge("after", "__end__");

        let compiled_parent = parent.compile().unwrap();

        let input = serde_json::json!({});
        let output = compiled_parent.invoke(input).await.unwrap();

        // Verify all nodes executed
        assert_eq!(output.get("before"), Some(&serde_json::json!(true)));
        assert_eq!(output.get("step1"), Some(&serde_json::json!(true)));
        assert_eq!(output.get("step2"), Some(&serde_json::json!(true)));
        assert_eq!(output.get("after"), Some(&serde_json::json!(true)));
    }

    #[tokio::test]
    async fn test_subgraph_state_isolation() {
        // Create a subgraph that adds a temporary field
        let mut subgraph = StateGraph::new();
        subgraph.add_node("process", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    let val = obj.get("input").and_then(|v| v.as_i64()).unwrap_or(0);
                    // Add temporary field
                    obj.insert("temp".to_string(), serde_json::json!("temporary"));
                    // Add output field
                    obj.insert("output".to_string(), serde_json::json!(val * 2));
                }
                Ok(state)
            })
        });
        subgraph.add_edge("__start__", "process");
        subgraph.add_edge("process", "__end__");

        let compiled_subgraph = subgraph.compile().unwrap();

        // Parent graph
        let mut parent = StateGraph::new();
        parent.add_subgraph("subprocess", compiled_subgraph);
        parent.add_edge("__start__", "subprocess");
        parent.add_edge("subprocess", "__end__");

        let compiled_parent = parent.compile().unwrap();

        let input = serde_json::json!({"input": 5});
        let output = compiled_parent.invoke(input).await.unwrap();

        // Both input and output should be in result
        assert_eq!(output.get("input"), Some(&serde_json::json!(5)));
        assert_eq!(output.get("output"), Some(&serde_json::json!(10)));
        // Temporary field should also be present (no isolation by default)
        assert_eq!(output.get("temp"), Some(&serde_json::json!("temporary")));
    }

    #[tokio::test]
    async fn test_parallel_subgraphs() {
        // This test demonstrates that the same subgraph definition can be used multiple times
        let mut subgraph = StateGraph::new();
        subgraph.add_node("double", |mut state: serde_json::Value| {
            Box::pin(async move {
                if let Some(obj) = state.as_object_mut() {
                    let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    obj.insert("value".to_string(), serde_json::json!(val * 2));
                }
                Ok(state)
            })
        });
        subgraph.add_edge("__start__", "double");
        subgraph.add_edge("double", "__end__");

        let compiled_subgraph = subgraph.compile().unwrap();

        // Create parent with two invocations of the same subgraph
        let mut parent = StateGraph::new();
        parent.add_subgraph("first_double", compiled_subgraph.clone());
        parent.add_subgraph("second_double", compiled_subgraph);

        parent.add_edge("__start__", "first_double");
        parent.add_edge("first_double", "second_double");
        parent.add_edge("second_double", "__end__");

        let compiled_parent = parent.compile().unwrap();

        // Execute: 3 * 2 = 6, then 6 * 2 = 12
        let input = serde_json::json!({"value": 3});
        let output = compiled_parent.invoke(input).await.unwrap();

        assert_eq!(output.get("value"), Some(&serde_json::json!(12)));
    }
}
