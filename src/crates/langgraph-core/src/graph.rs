//! Core graph data structures and types
//!
//! This module defines the fundamental building blocks for constructing executable graphs
//! in rLangGraph. The [`Graph`] struct represents the underlying graph structure used by
//! [`StateGraph`](crate::StateGraph) and other higher-level builders.
//!
//! # Graph Architecture
//!
//! A graph in rLangGraph consists of:
//!
//! - **Nodes**: Processing units that execute logic and transform state
//! - **Edges**: Connections that define control flow between nodes
//! - **Channels**: State storage containers with optional reducers for merging updates
//! - **Entry/Finish Points**: Special START and END nodes marking graph boundaries
//!
//! # Graph Structure
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                    Graph                            │
//! │                                                     │
//! │  START ──────┐                                     │
//! │              │                                     │
//! │              ▼                                     │
//! │         ┌─────────┐      Direct Edge              │
//! │         │ Node A  │──────────────────────┐        │
//! │         └─────────┘                       │        │
//! │                                           ▼        │
//! │         ┌─────────┐                 ┌─────────┐   │
//! │         │ Node B  │◄────────────────│ Node C  │   │
//! │         └─────────┘  Conditional    └─────────┘   │
//! │              │           Edge                      │
//! │              │                                     │
//! │              ▼                                     │
//! │            END                                     │
//! │                                                     │
//! │  Channels: { "state": LastValue, "logs": Topic }  │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # Examples
//!
//! ## Basic Graph Construction
//!
//! ```rust
//! use langgraph_core::graph::{Graph, NodeSpec, ChannelSpec, ChannelType, START, END};
//! use std::sync::Arc;
//!
//! let mut graph = Graph::new();
//!
//! // Add a processing node
//! let node_spec = NodeSpec {
//!     name: "process".to_string(),
//!     executor: Arc::new(|state| {
//!         Box::pin(async move { Ok(state) })
//!     }),
//!     reads: vec!["input".to_string()],
//!     writes: vec!["output".to_string()],
//!     subgraph: None,
//! };
//!
//! graph.add_node("process".to_string(), node_spec);
//!
//! // Connect START -> process -> END
//! graph.add_edge(START.to_string(), "process".to_string());
//! graph.add_edge("process".to_string(), END.to_string());
//!
//! // Validate the graph structure
//! assert!(graph.validate().is_ok());
//! ```
//!
//! ## Graph with Conditional Routing
//!
//! ```rust
//! use langgraph_core::graph::{Graph, START, END};
//! use langgraph_core::send::ConditionalEdgeResult;
//! use std::collections::HashMap;
//! use std::sync::Arc;
//!
//! let mut graph = Graph::new();
//!
//! // Add conditional edge with router function
//! let branches = HashMap::from([
//!     ("yes".to_string(), "node_a".to_string()),
//!     ("no".to_string(), "node_b".to_string()),
//! ]);
//!
//! graph.add_conditional_edge(
//!     START.to_string(),
//!     Arc::new(|state| {
//!         let condition = state["condition"].as_bool().unwrap_or(false);
//!         if condition {
//!             ConditionalEdgeResult::Single("yes".to_string())
//!         } else {
//!             ConditionalEdgeResult::Single("no".to_string())
//!         }
//!     }),
//!     branches,
//! );
//! ```
//!
//! # See Also
//!
//! - [`StateGraph`](crate::StateGraph) - High-level builder API
//! - [`CompiledGraph`](crate::CompiledGraph) - Executable graph
//! - [`ChannelType`] - Channel storage strategies

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Node identifier - unique name for each node in the graph
///
/// Node IDs are strings and must be unique within a graph. Special reserved
/// node IDs include [`START`] and [`END`] for entry and exit points.
///
/// # Examples
///
/// ```rust
/// use langgraph_core::graph::NodeId;
///
/// let node_id: NodeId = "my_processor".to_string();
/// ```
pub type NodeId = String;

/// Special node identifier for graph entry point
///
/// The `START` node is a virtual node that marks where graph execution begins.
/// It's automatically created when building graphs and doesn't execute any logic.
///
/// # Examples
///
/// ```rust
/// use langgraph_core::graph::{Graph, START};
///
/// let mut graph = Graph::new();
/// assert_eq!(graph.entry, START);
/// ```
pub const START: &str = "__start__";

/// Special node identifier for graph termination
///
/// The `END` node is a virtual node that marks successful graph completion.
/// Nodes can edge to END to signal they are terminal nodes.
///
/// # Examples
///
/// ```rust
/// use langgraph_core::graph::{Graph, END, START};
///
/// let mut graph = Graph::new();
/// graph.add_edge(START.to_string(), END.to_string());
/// ```
pub const END: &str = "__end__";

/// Special channel identifier for dynamic task accumulation
///
/// The `TASKS` channel is used internally to accumulate [`Send`](crate::send::Send)
/// objects for dynamic task spawning in map-reduce patterns.
///
/// # Examples
///
/// ```rust
/// use langgraph_core::graph::TASKS;
///
/// // The TASKS channel is managed automatically when using Send objects
/// println!("Tasks channel: {}", TASKS);
/// ```
pub const TASKS: &str = "__tasks__";

/// Edge type defining transitions between nodes
///
/// Edges control the flow of execution in a graph. There are two types:
///
/// 1. **Direct**: Unconditional transition to a single node
/// 2. **Conditional**: Dynamic routing based on state using a router function
///
/// # Edge Types
///
/// ## Direct Edge
///
/// A direct edge creates an unconditional transition from one node to another.
/// When the source node completes, execution always proceeds to the target node.
///
/// ```rust
/// use langgraph_core::graph::{Graph, Edge, START};
///
/// let mut graph = Graph::new();
/// graph.add_edge(START.to_string(), "next_node".to_string());
/// ```
///
/// ## Conditional Edge
///
/// A conditional edge uses a router function to determine the next node(s) at runtime
/// based on the current state. The router can return:
///
/// - A single node name (standard conditional routing)
/// - Multiple node names (parallel execution)
/// - Send objects (dynamic task spawning with custom state)
///
/// ```rust
/// use langgraph_core::graph::{Graph, START};
/// use langgraph_core::send::ConditionalEdgeResult;
/// use std::collections::HashMap;
/// use std::sync::Arc;
///
/// let mut graph = Graph::new();
///
/// let branches = HashMap::from([
///     ("success".to_string(), "handle_success".to_string()),
///     ("error".to_string(), "handle_error".to_string()),
/// ]);
///
/// graph.add_conditional_edge(
///     START.to_string(),
///     Arc::new(|state| {
///         if state["status"] == "ok" {
///             ConditionalEdgeResult::Single("success".to_string())
///         } else {
///             ConditionalEdgeResult::Single("error".to_string())
///         }
///     }),
///     branches,
/// );
/// ```
///
/// # See Also
///
/// - [`ConditionalEdgeResult`](crate::send::ConditionalEdgeResult) - Return types for router functions
/// - [`Send`](crate::send::Send) - Dynamic task specification
#[derive(Clone)]
pub enum Edge {
    /// Unconditional edge to a specific node
    ///
    /// When the source node completes, execution always proceeds to this target node.
    Direct(NodeId),

    /// Conditional edge with dynamic routing
    ///
    /// The router function is called with the current state and returns which node(s)
    /// to execute next. This enables:
    ///
    /// - **Conditional branching**: Route to different nodes based on state
    /// - **Parallel execution**: Return multiple nodes to run concurrently
    /// - **Map-reduce patterns**: Return Send objects with custom state per task
    Conditional {
        /// Router function that determines the next node(s) based on current state
        ///
        /// The function receives the graph state and must return a [`ConditionalEdgeResult`](crate::send::ConditionalEdgeResult):
        /// - `Single(String)` - Route to a single node
        /// - `Multiple(Vec<String>)` - Execute multiple nodes in parallel
        /// - `Send(Send)` or `Sends(Vec<Send>)` - Dynamic tasks with custom state
        router: Arc<dyn Fn(&serde_json::Value) -> crate::send::ConditionalEdgeResult + Send + Sync>,

        /// Map of branch keys to target nodes for validation and visualization
        ///
        /// This map defines all possible target nodes that the router might return.
        /// It's used for graph validation (ensuring all targets exist) and visualization
        /// (showing possible paths in graph diagrams).
        branches: HashMap<String, NodeId>,
    },
}

impl std::fmt::Debug for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Edge::Direct(node_id) => f.debug_tuple("Direct").field(node_id).finish(),
            Edge::Conditional { branches, .. } => f
                .debug_struct("Conditional")
                .field("router", &"<function>")
                .field("branches", branches)
                .finish(),
        }
    }
}

/// Core graph structure containing nodes, edges, and channels
///
/// The `Graph` struct is the foundational data structure for all graph-based workflows
/// in rLangGraph. It stores nodes (processing units), edges (control flow), channels
/// (state storage), and metadata needed for execution.
///
/// Typically, you won't create `Graph` directly. Instead, use [`StateGraph`](crate::StateGraph)
/// which provides a higher-level builder API and handles channel setup automatically.
///
/// # Structure
///
/// A graph contains:
///
/// - **nodes**: Map of node IDs to [`NodeSpec`] definitions
/// - **edges**: Map of source nodes to lists of outgoing [`Edge`]s
/// - **entry**: The entry point node (usually [`START`])
/// - **channels**: Map of channel names to [`ChannelSpec`] definitions
///
/// # Lifecycle
///
/// 1. **Construction**: Create with [`Graph::new()`]
/// 2. **Building**: Add nodes and edges with [`add_node`](Self::add_node) and [`add_edge`](Self::add_edge)
/// 3. **Validation**: Verify structure with [`validate`](Self::validate)
/// 4. **Compilation**: Convert to executable [`CompiledGraph`](crate::CompiledGraph)
///
/// # Examples
///
/// ## Linear Flow
///
/// ```rust
/// use langgraph_core::graph::{Graph, NodeSpec, START, END};
/// use std::sync::Arc;
///
/// let mut graph = Graph::new();
///
/// let node1 = NodeSpec {
///     name: "step1".to_string(),
///     executor: Arc::new(|state| Box::pin(async move { Ok(state) })),
///     reads: vec![],
///     writes: vec![],
///     subgraph: None,
/// };
///
/// let node2 = NodeSpec {
///     name: "step2".to_string(),
///     executor: Arc::new(|state| Box::pin(async move { Ok(state) })),
///     reads: vec![],
///     writes: vec![],
///     subgraph: None,
/// };
///
/// graph.add_node("step1".to_string(), node1);
/// graph.add_node("step2".to_string(), node2);
///
/// // START -> step1 -> step2 -> END
/// graph.add_edge(START.to_string(), "step1".to_string());
/// graph.add_edge("step1".to_string(), "step2".to_string());
/// graph.add_edge("step2".to_string(), END.to_string());
///
/// assert!(graph.validate().is_ok());
/// ```
///
/// ## Branching Flow
///
/// ```rust
/// use langgraph_core::graph::{Graph, START, END};
/// use langgraph_core::send::ConditionalEdgeResult;
/// use std::collections::HashMap;
/// use std::sync::Arc;
///
/// let mut graph = Graph::new();
///
/// let branches = HashMap::from([
///     ("a".to_string(), "node_a".to_string()),
///     ("b".to_string(), "node_b".to_string()),
/// ]);
///
/// graph.add_conditional_edge(
///     START.to_string(),
///     Arc::new(|state| {
///         let choice = state["choice"].as_str().unwrap_or("a");
///         ConditionalEdgeResult::Single(choice.to_string())
///     }),
///     branches,
/// );
/// ```
///
/// # See Also
///
/// - [`StateGraph`](crate::StateGraph) - Recommended high-level builder
/// - [`NodeSpec`] - Node definition
/// - [`Edge`] - Edge types
/// - [`ChannelSpec`] - Channel configuration
#[derive(Debug)]
#[derive(Clone)]
pub struct Graph {
    /// All nodes in the graph mapped by their unique IDs
    ///
    /// Each node represents a processing unit with an executor function,
    /// channel read/write specifications, and optional subgraph.
    pub nodes: HashMap<NodeId, NodeSpec>,

    /// All edges in the graph (source node -> list of outgoing edges)
    ///
    /// Edges define the control flow between nodes. A node can have multiple
    /// outgoing edges (though typically just one direct edge or one conditional edge).
    pub edges: HashMap<NodeId, Vec<Edge>>,

    /// Entry point node ID where graph execution begins
    ///
    /// Defaults to [`START`]. Can be changed with [`set_entry`](Self::set_entry).
    pub entry: NodeId,

    /// Channel definitions for state management
    ///
    /// Channels store and manage graph state. Each channel has a type
    /// (LastValue, Topic, BinaryOp) and optional reducer function.
    pub channels: HashMap<String, ChannelSpec>,
}

impl Graph {
    /// Create a new empty graph with default settings
    ///
    /// The graph is initialized with:
    /// - No nodes
    /// - No edges
    /// - Entry point set to [`START`]
    /// - No channels
    ///
    /// # Examples
    ///
    /// ```rust
    /// use langgraph_core::graph::{Graph, START};
    ///
    /// let graph = Graph::new();
    /// assert_eq!(graph.nodes.len(), 0);
    /// assert_eq!(graph.entry, START);
    /// ```
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            entry: START.to_string(),
            channels: HashMap::new(),
        }
    }

    /// Add a node to the graph
    ///
    /// Nodes are the processing units of the graph. Each node executes logic
    /// and can read from and write to channels.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this node (must not conflict with existing nodes)
    /// * `spec` - Node specification including executor function and channel access
    ///
    /// # Examples
    ///
    /// ```rust
    /// use langgraph_core::graph::{Graph, NodeSpec};
    /// use std::sync::Arc;
    ///
    /// let mut graph = Graph::new();
    ///
    /// let node_spec = NodeSpec {
    ///     name: "processor".to_string(),
    ///     executor: Arc::new(|state| {
    ///         Box::pin(async move {
    ///             // Process state
    ///             Ok(state)
    ///         })
    ///     }),
    ///     reads: vec!["input".to_string()],
    ///     writes: vec!["output".to_string()],
    ///     subgraph: None,
    /// };
    ///
    /// graph.add_node("processor".to_string(), node_spec);
    /// assert_eq!(graph.nodes.len(), 1);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`NodeSpec`] - Node specification structure
    pub fn add_node(&mut self, id: NodeId, spec: NodeSpec) {
        self.nodes.insert(id, spec);
    }

    /// Add a direct (unconditional) edge between two nodes
    ///
    /// Creates an unconditional transition from the source node to the target node.
    /// When the source node completes execution, the target node will be scheduled next.
    ///
    /// # Arguments
    ///
    /// * `from` - Source node ID (or [`START`])
    /// * `to` - Target node ID (or [`END`])
    ///
    /// # Examples
    ///
    /// ```rust
    /// use langgraph_core::graph::{Graph, START, END};
    ///
    /// let mut graph = Graph::new();
    ///
    /// // Create a simple flow: START -> process -> END
    /// graph.add_edge(START.to_string(), "process".to_string());
    /// graph.add_edge("process".to_string(), END.to_string());
    ///
    /// assert_eq!(graph.edges.len(), 2);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`add_conditional_edge`](Self::add_conditional_edge) - For dynamic routing
    /// - [`Edge::Direct`] - The edge variant created by this method
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) {
        self.edges
            .entry(from)
            .or_insert_with(Vec::new)
            .push(Edge::Direct(to));
    }

    /// Add a conditional edge with dynamic routing
    ///
    /// Creates a conditional edge that uses a router function to determine which node(s)
    /// to execute next based on the current state. The router can return:
    ///
    /// - A single node name (standard conditional branching)
    /// - Multiple node names (parallel execution)
    /// - Send objects (map-reduce with per-task state)
    ///
    /// # Arguments
    ///
    /// * `from` - Source node ID
    /// * `router` - Function that receives state and returns [`ConditionalEdgeResult`](crate::send::ConditionalEdgeResult)
    /// * `branches` - Map of branch keys to target node IDs (for validation/visualization)
    ///
    /// # Examples
    ///
    /// ## Simple Branching
    ///
    /// ```rust
    /// use langgraph_core::graph::{Graph, START};
    /// use langgraph_core::send::ConditionalEdgeResult;
    /// use std::collections::HashMap;
    /// use std::sync::Arc;
    ///
    /// let mut graph = Graph::new();
    ///
    /// let branches = HashMap::from([
    ///     ("positive".to_string(), "handle_positive".to_string()),
    ///     ("negative".to_string(), "handle_negative".to_string()),
    /// ]);
    ///
    /// graph.add_conditional_edge(
    ///     START.to_string(),
    ///     Arc::new(|state| {
    ///         let value = state["value"].as_i64().unwrap_or(0);
    ///         if value >= 0 {
    ///             ConditionalEdgeResult::Single("positive".to_string())
    ///         } else {
    ///             ConditionalEdgeResult::Single("negative".to_string())
    ///         }
    ///     }),
    ///     branches,
    /// );
    /// ```
    ///
    /// # See Also
    ///
    /// - [`ConditionalEdgeResult`](crate::send::ConditionalEdgeResult) - Router return types
    /// - [`Send`](crate::send::Send) - For map-reduce patterns
    /// - [`Edge::Conditional`] - The edge variant created by this method
    pub fn add_conditional_edge(
        &mut self,
        from: NodeId,
        router: Arc<dyn Fn(&serde_json::Value) -> crate::send::ConditionalEdgeResult + Send + Sync>,
        branches: HashMap<String, NodeId>,
    ) {
        self.edges
            .entry(from)
            .or_insert_with(Vec::new)
            .push(Edge::Conditional { router, branches });
    }

    /// Set the entry point for graph execution
    ///
    /// Changes where the graph execution begins. By default, graphs start at [`START`].
    ///
    /// # Arguments
    ///
    /// * `node` - Node ID to use as entry point (must exist in the graph)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use langgraph_core::graph::{Graph, NodeSpec, START};
    /// use std::sync::Arc;
    ///
    /// let mut graph = Graph::new();
    ///
    /// let node_spec = NodeSpec {
    ///     name: "custom_start".to_string(),
    ///     executor: Arc::new(|state| Box::pin(async move { Ok(state) })),
    ///     reads: vec![],
    ///     writes: vec![],
    ///     subgraph: None,
    /// };
    ///
    /// graph.add_node("custom_start".to_string(), node_spec);
    /// graph.set_entry("custom_start".to_string());
    ///
    /// assert_eq!(graph.entry, "custom_start");
    /// ```
    pub fn set_entry(&mut self, node: NodeId) {
        self.entry = node;
    }

    /// Validate the graph structure for correctness
    ///
    /// Performs structural validation to ensure the graph is well-formed:
    ///
    /// - Entry point exists (or is [`START`])
    /// - All edge source nodes exist
    /// - All edge target nodes exist (or are [`END`])
    /// - All conditional branch targets exist
    ///
    /// This validation is automatically performed during compilation but can be
    /// called manually to catch errors early.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the graph is valid
    /// - `Err(String)` with a descriptive error message if validation fails
    ///
    /// # Examples
    ///
    /// ## Valid Graph
    ///
    /// ```rust
    /// use langgraph_core::graph::{Graph, NodeSpec, START, END};
    /// use std::sync::Arc;
    ///
    /// let mut graph = Graph::new();
    ///
    /// let node = NodeSpec {
    ///     name: "processor".to_string(),
    ///     executor: Arc::new(|state| Box::pin(async move { Ok(state) })),
    ///     reads: vec![],
    ///     writes: vec![],
    ///     subgraph: None,
    /// };
    ///
    /// graph.add_node("processor".to_string(), node);
    /// graph.add_edge(START.to_string(), "processor".to_string());
    /// graph.add_edge("processor".to_string(), END.to_string());
    ///
    /// assert!(graph.validate().is_ok());
    /// ```
    ///
    /// ## Invalid Graph
    ///
    /// ```rust
    /// use langgraph_core::graph::{Graph, START};
    ///
    /// let mut graph = Graph::new();
    ///
    /// // Add edge to non-existent node
    /// graph.add_edge(START.to_string(), "missing_node".to_string());
    ///
    /// assert!(graph.validate().is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Entry point doesn't exist (and isn't START)
    /// - Any edge source node doesn't exist (and isn't START)
    /// - Any edge target node doesn't exist (and isn't END)
    /// - Any conditional branch target doesn't exist (and isn't END)
    pub fn validate(&self) -> Result<(), String> {
        // Check entry point exists
        if !self.nodes.contains_key(&self.entry) && self.entry != START {
            return Err(format!("Entry point {} does not exist", self.entry));
        }

        // Check all edge targets exist
        for (from, edges) in &self.edges {
            if !self.nodes.contains_key(from) && from != START {
                return Err(format!("Edge source {} does not exist", from));
            }

            for edge in edges {
                match edge {
                    Edge::Direct(to) => {
                        if !self.nodes.contains_key(to) && to != END {
                            return Err(format!("Edge target {} does not exist", to));
                        }
                    }
                    Edge::Conditional { branches, .. } => {
                        for to in branches.values() {
                            if !self.nodes.contains_key(to) && to != END {
                                return Err(format!("Branch target {} does not exist", to));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for embedding compiled graphs as subgraphs within parent graphs
///
/// `SubgraphExecutor` allows a compiled graph to be embedded as a node in another graph,
/// enabling hierarchical graph composition. This is used for:
///
/// - **Modularity**: Encapsulate complex logic in reusable subgraphs
/// - **Parent-Child Communication**: Pass messages between graph layers
/// - **Multi-Agent Systems**: Coordinate multiple independent agent graphs
///
/// # Implementation
///
/// This trait is automatically implemented for [`CompiledGraph`](crate::CompiledGraph),
/// allowing any compiled graph to be used as a subgraph.
///
/// # Examples
///
/// ```rust,no_run
/// use langgraph_core::graph::SubgraphExecutor;
/// use serde_json::json;
///
/// async fn use_subgraph(subgraph: &dyn SubgraphExecutor) {
///     let input = json!({"request": "process this"});
///     let output = subgraph.invoke(input).await.unwrap();
///     println!("Subgraph output: {}", output);
/// }
/// ```
///
/// # See Also
///
/// - [`CompiledGraph`](crate::CompiledGraph) - Primary implementation
/// - [Parent-Child Communication](crate::parent_child) - Inter-graph messaging
pub trait SubgraphExecutor: Send + Sync {
    /// Execute the subgraph with the given input state
    ///
    /// Invokes the subgraph's execution, passing the provided state as input.
    /// The subgraph runs to completion and returns its final state.
    ///
    /// # Arguments
    ///
    /// * `state` - Input state for the subgraph (typically a JSON object)
    ///
    /// # Returns
    ///
    /// The final state after subgraph execution completes
    ///
    /// # Errors
    ///
    /// Returns an error if subgraph execution fails
    fn invoke(
        &self,
        state: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>> + Send>>;

    /// Get the name of this subgraph
    ///
    /// Returns a human-readable name for debugging and logging
    fn name(&self) -> &str;
}

/// Node specification defining a processing unit in the graph
///
/// A `NodeSpec` completely describes a node including its executor function,
/// which channels it accesses, and whether it represents a subgraph.
///
/// # Structure
///
/// - **name**: Human-readable node identifier
/// - **executor**: Async function that processes state
/// - **reads**: Channels this node reads from (for dependency tracking)
/// - **writes**: Channels this node writes to (for change tracking)
/// - **subgraph**: Optional nested graph execution
///
/// # Examples
///
/// ## Simple Processing Node
///
/// ```rust
/// use langgraph_core::graph::NodeSpec;
/// use std::sync::Arc;
/// use serde_json::json;
///
/// let node_spec = NodeSpec {
///     name: "data_processor".to_string(),
///     executor: Arc::new(|state| {
///         Box::pin(async move {
///             let mut s = state.as_object().unwrap().clone();
///             s.insert("processed".to_string(), json!(true));
///             Ok(json!(s))
///         })
///     }),
///     reads: vec!["input_data".to_string()],
///     writes: vec!["output_data".to_string()],
///     subgraph: None,
/// };
/// ```
///
/// ## Subgraph Node
///
/// ```rust,no_run
/// use langgraph_core::graph::NodeSpec;
/// use std::sync::Arc;
///
/// # async fn example(subgraph: Arc<dyn langgraph_core::graph::SubgraphExecutor>) {
/// let node_spec = NodeSpec {
///     name: "agent_subgraph".to_string(),
///     executor: Arc::new(move |state| {
///         let sg = subgraph.clone();
///         Box::pin(async move {
///             sg.invoke(state).await
///         })
///     }),
///     reads: vec!["agent_input".to_string()],
///     writes: vec!["agent_output".to_string()],
///     subgraph: Some(subgraph.clone()),
/// };
/// # }
/// ```
///
/// # See Also
///
/// - [`NodeExecutor`] - The executor function type
/// - [`SubgraphExecutor`] - For hierarchical graphs
#[derive(Clone)]
pub struct NodeSpec {
    /// Human-readable name for this node
    ///
    /// Used for logging, debugging, and visualization. Should be unique
    /// within the graph but this is not strictly enforced.
    pub name: String,

    /// Async executor function that processes state
    ///
    /// The executor receives the current state as a JSON value and returns
    /// the updated state (or an error). See [`NodeExecutor`] for details.
    pub executor: NodeExecutor,

    /// List of channel names this node reads from
    ///
    /// Used for dependency tracking in the Pregel execution model.
    /// The executor will be invoked when any of these channels have new values.
    pub reads: Vec<String>,

    /// List of channel names this node writes to
    ///
    /// Used for change tracking to determine which downstream nodes need
    /// to be scheduled in the next superstep.
    pub writes: Vec<String>,

    /// Optional subgraph that this node executes
    ///
    /// When present, indicates this node represents a nested graph execution.
    /// The executor typically wraps calls to `subgraph.invoke()`.
    pub subgraph: Option<Arc<dyn SubgraphExecutor>>,
}

impl std::fmt::Debug for NodeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeSpec")
            .field("name", &self.name)
            .field("executor", &"<function>")
            .field("reads", &self.reads)
            .field("writes", &self.writes)
            .field("subgraph", &self.subgraph.as_ref().map(|sg| sg.name()))
            .finish()
    }
}

/// Node executor function type
///
/// A `NodeExecutor` is an async function that processes graph state. It receives
/// the current state as a JSON value and returns the updated state (or an error).
///
/// # Function Signature
///
/// ```text
/// Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<Value, Error>>>>
/// ```
///
/// Where:
/// - **Input**: Current state as [`serde_json::Value`]
/// - **Output**: Updated state or error
/// - **Async**: Returns a pinned future for async execution
/// - **Thread-safe**: Must be `Send + Sync` for concurrent execution
///
/// # Execution Context
///
/// Node executors run within the Pregel execution engine and have access to:
/// - Current graph state (via input parameter)
/// - Stream writer (via [`crate::runtime::get_stream_writer`])
/// - Store (via [`crate::runtime::get_store`])
///
/// # Examples
///
/// ## Simple State Transformation
///
/// ```rust
/// use langgraph_core::graph::NodeExecutor;
/// use std::sync::Arc;
/// use serde_json::json;
///
/// let executor: NodeExecutor = Arc::new(|state| {
///     Box::pin(async move {
///         let mut s = state.as_object().unwrap().clone();
///         s.insert("processed".to_string(), json!(true));
///         Ok(json!(s))
///     })
/// });
/// ```
///
/// ## With Error Handling
///
/// ```rust
/// use langgraph_core::graph::NodeExecutor;
/// use std::sync::Arc;
/// use serde_json::json;
///
/// let executor: NodeExecutor = Arc::new(|state| {
///     Box::pin(async move {
///         let value = state["input"]
///             .as_i64()
///             .ok_or("Missing input field")?;
///
///         if value < 0 {
///             return Err("Value must be non-negative".into());
///         }
///
///         Ok(json!({"result": value * 2}))
///     })
/// });
/// ```
///
/// # See Also
///
/// - [`NodeSpec`] - Node specification using executor
/// - [`CompiledGraph`](crate::CompiledGraph) - Execution environment
pub type NodeExecutor = Arc<dyn Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send + Sync>;

/// Channel specification defining state storage and merge behavior
///
/// Channels are the state management primitives in rLangGraph. Each channel stores
/// a piece of graph state and defines how multiple writes are combined.
///
/// # Channel Types
///
/// - **LastValue**: Keeps only the most recent value (default)
/// - **Topic**: Appends all values to a list
/// - **BinaryOp**: Merges values using a custom reducer function
///
/// # Reducer Functions
///
/// For `BinaryOp` channels, the reducer function combines multiple writes:
///
/// ```text
/// reducer(current_value, new_value) -> merged_value
/// ```
///
/// Common reducers:
/// - **add_messages**: Merge message lists by ID (see [`crate::messages::add_messages`])
/// - **append**: Concatenate arrays
/// - **sum**: Add numeric values
/// - **custom**: Any domain-specific merge logic
///
/// # Examples
///
/// ## LastValue Channel
///
/// ```rust
/// use langgraph_core::graph::{ChannelSpec, ChannelType};
///
/// let channel = ChannelSpec {
///     name: "current_state".to_string(),
///     channel_type: ChannelType::LastValue,
///     reducer: None,
/// };
/// ```
///
/// ## Topic Channel for Logs
///
/// ```rust
/// use langgraph_core::graph::{ChannelSpec, ChannelType};
///
/// let channel = ChannelSpec {
///     name: "logs".to_string(),
///     channel_type: ChannelType::Topic,
///     reducer: None,  // Topic automatically appends
/// };
/// ```
///
/// ## BinaryOp with Custom Reducer
///
/// ```rust
/// use langgraph_core::graph::{ChannelSpec, ChannelType};
/// use std::sync::Arc;
/// use serde_json::json;
///
/// let channel = ChannelSpec {
///     name: "sum".to_string(),
///     channel_type: ChannelType::BinaryOp,
///     reducer: Some(Arc::new(|a, b| {
///         json!(a.as_i64().unwrap_or(0) + b.as_i64().unwrap_or(0))
///     })),
/// };
/// ```
///
/// # See Also
///
/// - [`ChannelType`] - Available channel types
/// - [`ReducerFn`] - Reducer function type
/// - [`add_messages`](crate::messages::add_messages) - Message list reducer
#[derive(Clone, Serialize, Deserialize)]
pub struct ChannelSpec {
    /// Unique name for this channel
    ///
    /// Used to reference the channel when reading/writing from nodes
    pub name: String,

    /// Type of channel determining storage behavior
    ///
    /// See [`ChannelType`] for available types
    pub channel_type: ChannelType,

    /// Optional reducer function for merging multiple writes
    ///
    /// Required for `BinaryOp` channels, ignored for others.
    /// The reducer combines the current value with new writes.
    #[serde(skip)]
    pub reducer: Option<ReducerFn>,
}

impl std::fmt::Debug for ChannelSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelSpec")
            .field("name", &self.name)
            .field("channel_type", &self.channel_type)
            .field("reducer", &self.reducer.as_ref().map(|_| "<function>"))
            .finish()
    }
}

/// Channel storage strategy determining how values are stored and merged
///
/// The channel type controls how the channel stores values and handles multiple
/// concurrent writes from different nodes.
///
/// # Available Types
///
/// ## LastValue
///
/// Stores only the most recent value, discarding previous values. This is the
/// default and most common channel type.
///
/// **Use when**: You only care about the current state, not history.
///
/// ```rust
/// use langgraph_core::graph::ChannelType;
///
/// // Example: Current user session
/// let channel_type = ChannelType::LastValue;
/// ```
///
/// ## Topic
///
/// Appends all values to a list, preserving complete history. Values are never
/// removed (except by explicit state manipulation).
///
/// **Use when**: You need a complete log or history of all updates.
///
/// ```rust
/// use langgraph_core::graph::ChannelType;
///
/// // Example: Conversation message history
/// let channel_type = ChannelType::Topic;
/// ```
///
/// ## BinaryOp
///
/// Uses a custom reducer function to merge values. The reducer combines the
/// current value with each new write.
///
/// **Use when**: You need custom merge logic (sum, merge objects, etc.).
///
/// ```rust
/// use langgraph_core::graph::ChannelType;
///
/// // Example: Accumulating numeric totals
/// let channel_type = ChannelType::BinaryOp;
/// // Requires a reducer function in ChannelSpec
/// ```
///
/// # Behavior with Multiple Writes
///
/// When multiple nodes write to the same channel in one superstep:
///
/// - **LastValue**: Last write wins (non-deterministic with parallel writes)
/// - **Topic**: All writes appended to the list
/// - **BinaryOp**: All writes merged via reducer function
///
/// # Examples
///
/// ## Choosing the Right Type
///
/// ```rust
/// use langgraph_core::graph::ChannelType;
///
/// // User profile (only current state matters)
/// let profile_type = ChannelType::LastValue;
///
/// // Chat messages (need full history)
/// let messages_type = ChannelType::Topic;
///
/// // Running total (need to merge values)
/// let counter_type = ChannelType::BinaryOp;
/// ```
///
/// # See Also
///
/// - [`ChannelSpec`] - Channel specification with type
/// - [`ReducerFn`] - Custom merge function for BinaryOp
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// Store only the most recent value
    ///
    /// When multiple nodes write to this channel, the last write wins.
    /// Previous values are discarded.
    ///
    /// This is the default channel type used by [`StateGraph`](crate::StateGraph).
    LastValue,

    /// Append all values to a list
    ///
    /// All writes are appended to a growing list. Values are never removed
    /// automatically (use explicit state manipulation if needed).
    ///
    /// Useful for logs, message histories, or any append-only data.
    Topic,

    /// Merge values using a custom reducer function
    ///
    /// Each write is merged with the current value using the reducer function
    /// specified in [`ChannelSpec::reducer`].
    ///
    /// Common use cases:
    /// - Summing numeric values
    /// - Merging message lists by ID
    /// - Combining objects
    /// - Custom domain logic
    BinaryOp,
}

/// Reducer function type for merging channel values
///
/// A reducer combines the current channel value with a new write, producing
/// the merged result. Reducers are used with [`ChannelType::BinaryOp`] channels.
///
/// # Function Signature
///
/// ```text
/// Fn(current: Value, new: Value) -> Value
/// ```
///
/// Where:
/// - **current**: The current value in the channel
/// - **new**: The new value being written
/// - **Returns**: The merged result
///
/// # Properties
///
/// Reducers should typically be:
/// - **Associative**: `r(r(a,b),c) == r(a,r(b,c))`
/// - **Deterministic**: Same inputs always produce same output
/// - **Pure**: No side effects
///
/// # Common Reducers
///
/// ## Sum Numbers
///
/// ```rust
/// use langgraph_core::graph::ReducerFn;
/// use std::sync::Arc;
/// use serde_json::json;
///
/// let sum_reducer: ReducerFn = Arc::new(|a, b| {
///     json!(a.as_i64().unwrap_or(0) + b.as_i64().unwrap_or(0))
/// });
/// ```
///
/// ## Merge Objects
///
/// ```rust
/// use langgraph_core::graph::ReducerFn;
/// use std::sync::Arc;
/// use serde_json::{json, Value};
///
/// let merge_reducer: ReducerFn = Arc::new(|a, b| {
///     let mut result = a.as_object().unwrap().clone();
///     for (k, v) in b.as_object().unwrap() {
///         result.insert(k.clone(), v.clone());
///     }
///     Value::Object(result)
/// });
/// ```
///
/// ## Add Messages (Built-in)
///
/// ```rust
/// use langgraph_core::messages::add_messages;
/// use langgraph_core::graph::ReducerFn;
/// use std::sync::Arc;
///
/// // Use the built-in message reducer
/// let message_reducer: ReducerFn = Arc::new(|a, b| {
///     let left = serde_json::from_value(a).unwrap_or_default();
///     let right = serde_json::from_value(b).unwrap_or_default();
///     serde_json::to_value(add_messages(left, right)).unwrap()
/// });
/// ```
///
/// # See Also
///
/// - [`ChannelType::BinaryOp`] - Channel type that uses reducers
/// - [`ChannelSpec`] - Channel specification with reducer
/// - [`add_messages`](crate::messages::add_messages) - Message list reducer
pub type ReducerFn = Arc<dyn Fn(serde_json::Value, serde_json::Value) -> serde_json::Value + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let graph = Graph::new();
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
        assert_eq!(graph.entry, START);
    }

    #[test]
    fn test_add_nodes_and_edges() {
        let mut graph = Graph::new();

        let node_spec = NodeSpec {
            name: "node1".to_string(),
            executor: Arc::new(|state| {
                Box::pin(async move { Ok(state) })
            }),
            reads: vec!["input".to_string()],
            writes: vec!["output".to_string()],
            subgraph: None,
        };

        graph.add_node("node1".to_string(), node_spec);
        graph.add_edge(START.to_string(), "node1".to_string());
        graph.add_edge("node1".to_string(), END.to_string());

        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn test_graph_validation() {
        let mut graph = Graph::new();

        let node_spec = NodeSpec {
            name: "node1".to_string(),
            executor: Arc::new(|state| {
                Box::pin(async move { Ok(state) })
            }),
            reads: vec![],
            writes: vec![],
            subgraph: None,
        };

        graph.add_node("node1".to_string(), node_spec);
        graph.set_entry("node1".to_string());

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_graph_validation_fails_missing_node() {
        let mut graph = Graph::new();
        graph.set_entry("nonexistent".to_string());

        assert!(graph.validate().is_err());
    }

    #[test]
    fn test_special_constants() {
        // Verify special node identifiers
        assert_eq!(START, "__start__");
        assert_eq!(END, "__end__");

        // Verify special channel identifiers
        assert_eq!(TASKS, "__tasks__");
    }
}
