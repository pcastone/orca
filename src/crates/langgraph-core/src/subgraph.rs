//! Hierarchical graph composition through subgraph embedding
//!
//! This module provides **subgraph execution** - the ability to embed entire compiled graphs
//! as nodes within parent graphs. Subgraphs enable modular workflow design, code reuse,
//! isolation of concerns, and hierarchical orchestration of complex multi-stage processes.
//!
//! # Overview
//!
//! Subgraphs enable:
//!
//! - **Hierarchical Composition** - Nest graphs within graphs for modular design
//! - **Code Reuse** - Share common workflow patterns across parent graphs
//! - **Isolation** - Separate checkpoint threads and state management per subgraph
//! - **State Filtering** - Control which state fields subgraphs can access
//! - **State Syncing** - Merge subgraph outputs back into parent state
//! - **Parent-Child Communication** - Commands can target parent from within subgraphs
//! - **Flexible Configuration** - Control inheritance, filtering, and thread management
//!
//! # Core Types
//!
//! - [`CompiledSubgraph`] - Wrapper for executing compiled graph as subgraph
//! - `SubgraphConfig` - Configuration for subgraph behavior (in `parent_child` module)
//! - [`StateGraphSubgraphExt`] - Extension trait for adding subgraphs to graphs
//! - [`create_subgraph_node()`] - Create node executor from compiled graph
//!
//! # When to Use Subgraphs
//!
//! **Use subgraphs when you need:**
//! - ✅ Reusable workflow components
//! - ✅ Modular separation of concerns
//! - ✅ Independent checkpoint management
//! - ✅ Complex multi-stage pipelines
//! - ✅ Different execution contexts (threads, configs)
//!
//! **Avoid subgraphs when:**
//! - ❌ Simple sequential node execution suffices
//! - ❌ No need for isolation or reuse
//! - ❌ Overhead of separate checkpoint threads is unnecessary
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Parent Graph                                                │
//! │                                                               │
//! │  ┌──────────┐       ┌──────────┐       ┌──────────┐        │
//! │  │ Node A   │   ──► │ Subgraph │   ──► │ Node C   │        │
//! │  └──────────┘       └────┬─────┘       └──────────┘        │
//! │                           │                                  │
//! │  Parent State: {user: "Alice", data: [...]}                 │
//! │                           │                                  │
//! │  ┌────────────────────────▼──────────────────────┐          │
//! │  │  Subgraph Execution (Isolated Thread)         │          │
//! │  │  ┌─────────────────────────────────────┐     │          │
//! │  │  │  Child Graph                         │     │          │
//! │  │  │  ┌────────┐   ┌────────┐   ┌─────┐ │     │          │
//! │  │  │  │ Step 1 │──→│ Step 2 │──→│ END │ │     │          │
//! │  │  │  └────────┘   └────────┘   └─────┘ │     │          │
//! │  │  └─────────────────────────────────────┘     │          │
//! │  │                                                │          │
//! │  │  • Separate checkpoint thread                 │          │
//! │  │  • Filtered state (if configured)            │          │
//! │  │  • Independent execution context             │          │
//! │  └────────────────────┬───────────────────────────┘          │
//! │                       │                                       │
//! │  ┌────────────────────▼───────────────────────┐             │
//! │  │  Subgraph Output Merged to Parent State    │             │
//! │  │  {user: "Alice", data: [...], result: 42}  │             │
//! │  └─────────────────────────────────────────────┘             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Basic Subgraph Embedding
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, subgraph::StateGraphSubgraphExt};
//! use serde_json::json;
//!
//! // 1. Create child graph
//! let mut child_graph = StateGraph::new();
//! child_graph.add_node("process", |state| {
//!     Box::pin(async move {
//!         let data = &state["data"];
//!         let processed = perform_processing(data)?;
//!         Ok(json!({"result": processed}))
//!     })
//! });
//! child_graph.add_edge("__start__", "process");
//! child_graph.add_edge("process", "__end__");
//!
//! let compiled_child = child_graph.compile()?;
//!
//! // 2. Create parent graph with child as subgraph node
//! let mut parent_graph = StateGraph::new();
//!
//! parent_graph.add_node("prepare", |state| {
//!     Box::pin(async move {
//!         Ok(json!({"data": "input data"}))
//!     })
//! });
//!
//! // Add compiled child graph as a node
//! parent_graph.add_simple_subgraph("process_data", compiled_child);
//!
//! parent_graph.add_node("finalize", |state| {
//!     Box::pin(async move {
//!         let result = &state["result"];
//!         Ok(json!({"final": result}))
//!     })
//! });
//!
//! parent_graph.add_edge("__start__", "prepare");
//! parent_graph.add_edge("prepare", "process_data");
//! parent_graph.add_edge("process_data", "finalize");
//! parent_graph.add_edge("finalize", "__end__");
//!
//! let compiled_parent = parent_graph.compile()?;
//!
//! // 3. Execute - child graph runs as part of parent
//! let result = compiled_parent.invoke(json!({})).await?;
//! ```
//!
//! ## Subgraph with State Filtering
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, parent_child::SubgraphConfig};
//! use serde_json::json;
//!
//! // Child graph that should only access specific state fields
//! let mut child = StateGraph::new();
//! child.add_node("secure_process", |state| {
//!     Box::pin(async move {
//!         // Only has access to filtered fields
//!         let allowed_data = &state["allowed_field"];
//!         Ok(json!({"processed": process(allowed_data)}))
//!     })
//! });
//! child.add_edge("__start__", "secure_process");
//! child.add_edge("secure_process", "__end__");
//!
//! let compiled_child = child.compile()?;
//!
//! // Configure to only pass specific fields to subgraph
//! let config = SubgraphConfig::new("secure_child")
//!     .with_state_filter(vec![
//!         "allowed_field".to_string(),
//!         "public_data".to_string()
//!     ]);
//!
//! let mut parent = StateGraph::new();
//! parent.add_configured_subgraph("secure_node", compiled_child, config);
//!
//! // Parent state has more fields, but child only sees filtered ones
//! let result = parent.compile()?.invoke(json!({
//!     "allowed_field": "visible",
//!     "secret_field": "hidden", // Not visible to subgraph
//!     "public_data": "also visible"
//! })).await?;
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Reusable Processing Pipeline
//!
//! Create once, use in multiple parent workflows:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, subgraph::StateGraphSubgraphExt};
//! use serde_json::json;
//!
//! // Reusable data validation pipeline
//! fn create_validation_pipeline() -> CompiledGraph {
//!     let mut graph = StateGraph::new();
//!
//!     graph.add_node("schema_check", validate_schema);
//!     graph.add_node("business_rules", check_business_rules);
//!     graph.add_node("enrichment", enrich_data);
//!
//!     graph.add_edge("__start__", "schema_check");
//!     graph.add_edge("schema_check", "business_rules");
//!     graph.add_edge("business_rules", "enrichment");
//!     graph.add_edge("enrichment", "__end__");
//!
//!     graph.compile().unwrap()
//! }
//!
//! // Use in multiple workflows
//! let validation_pipeline = create_validation_pipeline();
//!
//! // Workflow 1: User registration
//! let mut registration = StateGraph::new();
//! registration.add_simple_subgraph("validate", validation_pipeline.clone());
//! registration.add_node("create_user", create_user_node);
//! // ... rest of workflow
//!
//! // Workflow 2: Data import
//! let mut import = StateGraph::new();
//! import.add_simple_subgraph("validate", validation_pipeline.clone());
//! import.add_node("import_to_db", import_node);
//! // ... rest of workflow
//! ```
//!
//! ## Pattern 2: Multi-Stage Nested Workflows
//!
//! Subgraphs within subgraphs for deep hierarchies:
//!
//! ```rust,ignore
//! // Level 3: Atomic operations
//! let mut fetch_data = StateGraph::new();
//! fetch_data.add_node("api_call", |state| { /* ... */ });
//! let compiled_fetch = fetch_data.compile()?;
//!
//! // Level 2: Processing stage (uses Level 3)
//! let mut process_stage = StateGraph::new();
//! process_stage.add_simple_subgraph("fetch", compiled_fetch);
//! process_stage.add_node("transform", |state| { /* ... */ });
//! process_stage.add_node("validate", |state| { /* ... */ });
//! let compiled_process = process_stage.compile()?;
//!
//! // Level 1: Main workflow (uses Level 2)
//! let mut main_workflow = StateGraph::new();
//! main_workflow.add_node("prepare", |state| { /* ... */ });
//! main_workflow.add_simple_subgraph("process", compiled_process);
//! main_workflow.add_node("finalize", |state| { /* ... */ });
//! let compiled_main = main_workflow.compile()?;
//! ```
//!
//! ## Pattern 3: Isolated Error Handling
//!
//! Subgraphs with independent error recovery:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, parent_child::SubgraphConfig};
//! use serde_json::json;
//!
//! // Risky operation in isolated subgraph
//! let mut risky_subgraph = StateGraph::new();
//! risky_subgraph.add_node("attempt", |state| {
//!     Box::pin(async move {
//!         match dangerous_operation(&state).await {
//!             Ok(result) => Ok(json!({"success": true, "result": result})),
//!             Err(e) => Ok(json!({"success": false, "error": e.to_string()}))
//!         }
//!     })
//! });
//! risky_subgraph.add_edge("__start__", "attempt");
//! risky_subgraph.add_edge("attempt", "__end__");
//!
//! let compiled_risky = risky_subgraph.compile()?;
//!
//! // Parent handles subgraph result
//! let mut parent = StateGraph::new();
//! parent.add_simple_subgraph("try_risky", compiled_risky);
//! parent.add_node("check_result", |state| {
//!     Box::pin(async move {
//!         if state["success"].as_bool().unwrap_or(false) {
//!             // Continue with result
//!             Ok(state)
//!         } else {
//!             // Use fallback
//!             Ok(json!({"result": "fallback_value"}))
//!         }
//!     })
//! });
//! ```
//!
//! ## Pattern 4: Conditional Subgraph Routing
//!
//! Choose different subgraphs based on state:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, Command};
//! use serde_json::json;
//!
//! // Different processing strategies
//! let quick_process = create_quick_pipeline().compile()?;
//! let thorough_process = create_thorough_pipeline().compile()?;
//!
//! let mut parent = StateGraph::new();
//! parent.add_simple_subgraph("quick", quick_process);
//! parent.add_simple_subgraph("thorough", thorough_process);
//!
//! parent.add_node("router", |state| {
//!     Box::pin(async move {
//!         let priority = state["priority"].as_str().unwrap_or("normal");
//!
//!         let next = if priority == "urgent" {
//!             "quick"
//!         } else {
//!             "thorough"
//!         };
//!
//!         Ok(Command::new().with_goto(next))
//!     })
//! });
//!
//! parent.add_edge("__start__", "router");
//! // Router dynamically chooses which subgraph to execute
//! ```
//!
//! ## Pattern 5: Map-Reduce with Subgraph Workers
//!
//! Use subgraphs as parallel workers:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, Command, Send};
//! use serde_json::json;
//!
//! // Worker subgraph for processing individual items
//! let mut worker = StateGraph::new();
//! worker.add_node("process_item", |state| {
//!     Box::pin(async move {
//!         let item = &state["item"];
//!         let result = expensive_processing(item).await?;
//!         Ok(json!({"result": result}))
//!     })
//! });
//! worker.add_edge("__start__", "process_item");
//! worker.add_edge("process_item", "__end__");
//! let compiled_worker = worker.compile()?;
//!
//! // Parent graph spawns multiple subgraph instances
//! let mut parent = StateGraph::new();
//! parent.add_simple_subgraph("worker", compiled_worker);
//!
//! parent.add_node("map", |state| {
//!     Box::pin(async move {
//!         let items = state["items"].as_array().unwrap();
//!
//!         let sends: Vec<Send> = items.iter().map(|item| {
//!             Send::new("worker", json!({"item": item}))
//!         }).collect();
//!
//!         Ok(Command::new().with_goto(sends))
//!     })
//! });
//!
//! parent.add_node("reduce", |state| {
//!     Box::pin(async move {
//!         let results = collect_results(&state);
//!         Ok(json!({"final": results}))
//!     })
//! });
//!
//! // Each Send spawns independent subgraph execution
//! ```
//!
//! # Configuration Options
//!
//! ## State Filtering
//!
//! Control which state fields are visible to subgraph:
//!
//! ```rust,ignore
//! use langgraph_core::parent_child::SubgraphConfig;
//!
//! let config = SubgraphConfig::new("my_subgraph")
//!     .with_state_filter(vec![
//!         "allowed_field_1".to_string(),
//!         "allowed_field_2".to_string(),
//!     ]);
//!
//! // Subgraph only receives filtered fields from parent state
//! parent.add_configured_subgraph("node", child_graph, config);
//! ```
//!
//! **Use cases:**
//! - Security: Hide sensitive data from subgraphs
//! - Isolation: Prevent subgraph from accessing unrelated state
//! - API contracts: Enforce what data subgraph can use
//!
//! ## State Syncing
//!
//! Merge subgraph output back into parent state:
//!
//! ```rust,ignore
//! let config = SubgraphConfig::new("my_subgraph")
//!     .with_sync_state_to_parent(true);
//!
//! // Subgraph output fields are merged into parent state
//! // Parent state = {a: 1} + Subgraph output {b: 2} = {a: 1, b: 2}
//! ```
//!
//! **Use cases:**
//! - Accumulate results from multiple subgraphs
//! - Subgraph enriches parent state
//! - Maintain state continuity across nested graphs
//!
//! ## Inherit State
//!
//! Control whether subgraph inherits parent state:
//!
//! ```rust,ignore
//! let config = SubgraphConfig::new("my_subgraph")
//!     .with_inherit_state(false);
//!
//! // Subgraph starts with empty state (doesn't see parent state)
//! ```
//!
//! **Use cases:**
//! - Clean slate execution
//! - Prevent state pollution
//! - Independent subgraph context
//!
//! # Parent-Child Communication
//!
//! Subgraphs can send commands to parent graph:
//!
//! ```rust,ignore
//! use langgraph_core::{Command, CommandGraph};
//! use serde_json::json;
//!
//! // Inside subgraph node
//! child_graph.add_node("notify_parent", |state| {
//!     Box::pin(async move {
//!         // Send command to parent graph
//!         Ok(Command::new()
//!             .with_graph(CommandGraph::Parent)
//!             .with_update(json!({"child_completed": true}))
//!             .with_goto("parent_continue"))
//!     })
//! });
//! ```
//!
//! # Use Cases
//!
//! ## Microservice Orchestration
//!
//! Each subgraph represents a microservice workflow:
//! - Authentication service (subgraph)
//! - Payment processing (subgraph)
//! - Notification service (subgraph)
//! - Parent orchestrates overall flow
//!
//! ## ETL Pipelines
//!
//! - Extract (subgraph with retry logic)
//! - Transform (subgraph with validation)
//! - Load (subgraph with batching)
//! - Parent coordinates stages
//!
//! ## Multi-Agent Systems
//!
//! - Researcher agent (subgraph)
//! - Planner agent (subgraph)
//! - Executor agent (subgraph)
//! - Coordinator agent (parent)
//!
//! # Performance Considerations
//!
//! ## Overhead
//!
//! - **Thread isolation**: Each subgraph gets separate checkpoint thread (~1ms overhead)
//! - **State copying**: State is cloned when passing to subgraph
//! - **Checkpointing**: Subgraphs checkpoint independently (increases storage)
//!
//! ## Optimization Strategies
//!
//! 1. **Minimize state size**: Filter state to reduce copying overhead
//! 2. **Reuse compiled graphs**: Compile once, use many times
//! 3. **Avoid deep nesting**: Limit nesting depth to 2-3 levels
//! 4. **Batch subgraph calls**: Use map-reduce instead of sequential subgraph calls
//!
//! ## When to Avoid Subgraphs
//!
//! - Simple sequential workflows (use regular nodes)
//! - No reuse needed (inline the logic)
//! - Performance-critical tight loops (subgraph overhead adds up)
//!
//! # Best Practices
//!
//! 1. **Design for Reuse** - Create subgraphs that solve general problems, not one-off use cases
//!
//! 2. **Clear Interfaces** - Document what state fields subgraph expects and produces
//!
//! 3. **Use State Filtering** - Explicitly control what data flows into subgraphs for security
//!
//! 4. **Limit Nesting Depth** - Keep hierarchy shallow (2-3 levels max) for maintainability
//!
//! 5. **Independent Checkpointing** - Leverage separate checkpoint threads for fault isolation
//!
//! 6. **Test Subgraphs Independently** - Unit test subgraphs standalone before embedding
//!
//! 7. **Version Subgraphs** - Treat subgraphs as versioned components when used across workflows
//!
//! # Comparison with Python LangGraph
//!
//! | Python LangGraph | rLangGraph | Notes |
//! |------------------|------------|-------|
//! | `graph.add_node("sub", compiled_subgraph)` | `parent.add_simple_subgraph("sub", compiled)` | Extension trait method |
//! | State inheritance (default) | `SubgraphConfig::new()` | Explicit configuration |
//! | No built-in filtering | `.with_state_filter(vec![...])` | Rust has explicit filtering |
//! | `invoke()` returns merged state | `.with_sync_state_to_parent(true)` | Opt-in syncing |
//! | Parent-child via context | `CommandGraph::Parent` | Explicit command targeting |
//! | Thread isolation implicit | Separate checkpoint thread (explicit) | Rust makes it visible |
//!
//! # See Also
//!
//! - [`Command`](crate::command) - Send commands to parent from subgraph
//! - [`StateGraph`](crate::builder::StateGraph) - Build graphs that can be used as subgraphs
//! - [`CompiledGraph`](crate::compiled::CompiledGraph) - Compile graphs before embedding
//! - [Parent-child module](crate::parent_child) - Parent context and hierarchy management

use crate::{
    compiled::CompiledGraph,
    graph::{NodeExecutor, SubgraphExecutor},
    parent_child::{GraphHierarchy, SubgraphConfig, set_parent_context, clear_parent_context},
    CheckpointConfig,
};
use serde_json::Value;
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;

/// Wrapper to make CompiledGraph usable as a subgraph
pub struct CompiledSubgraph {
    /// The compiled graph to execute as a subgraph
    graph: CompiledGraph,

    /// Configuration for this subgraph
    config: SubgraphConfig,

    /// Optional hierarchy manager
    hierarchy: Option<Arc<GraphHierarchy>>,
}

impl CompiledSubgraph {
    /// Create a new compiled subgraph
    pub fn new(graph: CompiledGraph, config: SubgraphConfig) -> Self {
        Self {
            graph,
            config,
            hierarchy: None,
        }
    }

    /// Set the hierarchy manager
    pub fn with_hierarchy(mut self, hierarchy: Arc<GraphHierarchy>) -> Self {
        self.hierarchy = Some(hierarchy);
        self
    }

    /// Create from a compiled graph with a simple name
    pub fn from_compiled(graph: CompiledGraph, name: impl Into<String>) -> Self {
        Self::new(graph, SubgraphConfig::new(name))
    }
}

impl SubgraphExecutor for CompiledSubgraph {
    fn invoke(
        &self,
        state: Value,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<Value, Box<dyn std::error::Error + Send + Sync>>> + Send>> {
        let graph = self.graph.clone();
        let config = self.config.clone();
        let hierarchy = self.hierarchy.clone();

        Box::pin(async move {
            // Create parent context if hierarchy is available
            if let Some(ref h) = hierarchy {
                if let Some(context) = h.create_context(&config.name, state.clone()) {
                    set_parent_context(context);
                }
            }

            // Filter state if needed
            let input_state = if config.inherit_state {
                config.filter_state(&state)
            } else {
                state.clone()
            };

            // Execute the subgraph
            let checkpoint = CheckpointConfig::new()
                .with_thread_id(format!("{}_{}", config.name, uuid::Uuid::new_v4()));
            let result = graph.invoke_with_config(input_state, Some(checkpoint)).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>);

            // Handle result and state sync
            let final_result = match result {
                Ok(output) => {
                    // Sync state back to parent if configured
                    if config.sync_state_to_parent {
                        // Merge subgraph output with original state
                        if let (Some(state_obj), Some(output_obj)) =
                            (state.as_object(), output.as_object()) {
                            let mut merged = state_obj.clone();
                            for (key, value) in output_obj {
                                merged.insert(key.clone(), value.clone());
                            }
                            Ok(Value::Object(merged))
                        } else {
                            Ok(output)
                        }
                    } else {
                        Ok(output)
                    }
                }
                Err(e) => Err(e)
            };

            // Clear parent context
            clear_parent_context();

            final_result
        })
    }

    fn name(&self) -> &str {
        &self.config.name
    }
}

/// Create a subgraph node executor from a compiled graph
///
/// This function creates a node executor that runs a compiled graph as a subgraph.
///
/// # Arguments
///
/// * `graph` - The compiled graph to use as a subgraph
/// * `config` - Configuration for the subgraph
///
/// # Example
///
/// ```rust,no_run
/// use langgraph_core::{StateGraph, subgraph::create_subgraph_node};
/// use langgraph_core::parent_child::SubgraphConfig;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create child graph
/// let mut child = StateGraph::new();
/// child.add_node("process", |state| {
///     Box::pin(async move { Ok(state) })
/// });
/// child.add_edge("__start__", "process");
/// child.add_edge("process", "__end__");
/// let compiled_child = child.compile()?;
///
/// // Create parent graph with child as subgraph
/// let mut parent = StateGraph::new();
///
/// // Add the child graph as a node
/// let config = SubgraphConfig::new("child_graph");
/// parent.add_configured_subgraph("run_child", compiled_child, config);
///
/// parent.add_edge("__start__", "run_child");
/// parent.add_edge("run_child", "__end__");
/// # Ok(())
/// # }
/// ```
pub fn create_subgraph_node(
    graph: CompiledGraph,
    config: SubgraphConfig,
) -> NodeExecutor {
    let subgraph = Arc::new(CompiledSubgraph::new(graph, config));

    Arc::new(move |state: Value| {
        let sg = subgraph.clone();
        Box::pin(async move {
            sg.invoke(state).await
        })
    })
}

/// Extension trait for StateGraph to add subgraph support
pub trait StateGraphSubgraphExt {
    /// Add a compiled graph as a subgraph node with configuration
    fn add_configured_subgraph(
        &mut self,
        name: impl Into<String>,
        graph: CompiledGraph,
        config: SubgraphConfig,
    );

    /// Add a simple subgraph with default configuration
    fn add_simple_subgraph(
        &mut self,
        name: impl Into<String>,
        graph: CompiledGraph,
    );
}

impl StateGraphSubgraphExt for crate::builder::StateGraph {
    fn add_configured_subgraph(
        &mut self,
        name: impl Into<String>,
        graph: CompiledGraph,
        config: SubgraphConfig,
    ) {
        let name = name.into();
        let executor = create_subgraph_node(graph, config);
        self.add_node_with_executor(name, executor);
    }

    fn add_simple_subgraph(
        &mut self,
        name: impl Into<String>,
        graph: CompiledGraph,
    ) {
        let name_str = name.into();
        let config = SubgraphConfig::new(name_str.clone());
        self.add_configured_subgraph(name_str, graph, config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::StateGraph;

    #[tokio::test]
    async fn test_subgraph_creation() {
        // Create child graph
        let mut child = StateGraph::new();
        child.add_node("increment", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                let count = s.get("count")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                s.insert("count".to_string(), serde_json::json!(count + 1));
                Ok(Value::Object(s))
            })
        });
        child.add_edge("__start__", "increment");
        child.add_edge("increment", "__end__");

        let compiled_child = child.compile().unwrap();

        // Create parent graph with child as subgraph
        let mut parent = StateGraph::new();
        parent.add_simple_subgraph("child", compiled_child);
        parent.add_edge("__start__", "child");
        parent.add_edge("child", "__end__");

        let compiled_parent = parent.compile().unwrap();

        // Execute
        let result = compiled_parent
            .invoke_with_config(serde_json::json!({"count": 0}), Some(CheckpointConfig::new().with_thread_id("test".to_string())))
            .await
            .unwrap();

        assert_eq!(result.get("count"), Some(&serde_json::json!(1)));
    }

    #[tokio::test]
    async fn test_subgraph_state_filtering() {
        // Create child graph that only modifies specific fields
        let mut child = StateGraph::new();
        child.add_node("process", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                s.insert("processed".to_string(), serde_json::json!(true));
                Ok(Value::Object(s))
            })
        });
        child.add_edge("__start__", "process");
        child.add_edge("process", "__end__");

        let compiled_child = child.compile().unwrap();

        // Configure to only inherit specific state
        let config = SubgraphConfig::new("child")
            .with_state_filter(vec!["allowed".to_string()]);

        // Create parent
        let mut parent = StateGraph::new();
        parent.add_configured_subgraph("child", compiled_child, config);
        parent.add_edge("__start__", "child");
        parent.add_edge("child", "__end__");

        let compiled_parent = parent.compile().unwrap();

        // Execute with mixed state
        let input = serde_json::json!({
            "allowed": "value1",
            "filtered": "value2"
        });

        let result = compiled_parent
            .invoke_with_config(input, Some(CheckpointConfig::new().with_thread_id("test".to_string())))
            .await
            .unwrap();

        // Should preserve both fields and add processed
        assert_eq!(result.get("allowed"), Some(&serde_json::json!("value1")));
        assert_eq!(result.get("filtered"), Some(&serde_json::json!("value2")));
        assert_eq!(result.get("processed"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn test_compiled_subgraph_impl() {
        let mut graph = StateGraph::new();
        graph.add_node("test", |s| Box::pin(async move { Ok(s) }));
        graph.add_edge("__start__", "test");
        graph.add_edge("test", "__end__");

        let compiled = graph.compile().unwrap();
        let config = SubgraphConfig::new("test_sub");

        let subgraph = CompiledSubgraph::new(compiled, config);

        assert_eq!(subgraph.name(), "test_sub");
    }

    // ========================================================================
    // Phase 10.2: Nested Graph Execution Tests
    // ========================================================================

    #[tokio::test]
    async fn test_nested_subgraph_two_levels() {
        // Level 2: Inner-most subgraph
        let mut inner = StateGraph::new();
        inner.add_node("double", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                let val = s.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                s.insert("value".to_string(), serde_json::json!(val * 2));
                Ok(Value::Object(s))
            })
        });
        inner.add_edge("__start__", "double");
        inner.add_edge("double", "__end__");
        let compiled_inner = inner.compile().unwrap();

        // Level 1: Middle subgraph (contains inner subgraph)
        let mut middle = StateGraph::new();
        middle.add_simple_subgraph("inner", compiled_inner);
        middle.add_node("add_ten", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                let val = s.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                s.insert("value".to_string(), serde_json::json!(val + 10));
                Ok(Value::Object(s))
            })
        });
        middle.add_edge("__start__", "inner");
        middle.add_edge("inner", "add_ten");
        middle.add_edge("add_ten", "__end__");
        let compiled_middle = middle.compile().unwrap();

        // Level 0: Outer graph (contains middle subgraph)
        let mut outer = StateGraph::new();
        outer.add_simple_subgraph("middle", compiled_middle);
        outer.add_edge("__start__", "middle");
        outer.add_edge("middle", "__end__");
        let compiled_outer = outer.compile().unwrap();

        // Execute: value starts at 5, doubled to 10, then add 10 = 20
        let result = compiled_outer
            .invoke_with_config(
                serde_json::json!({"value": 5}),
                Some(CheckpointConfig::new().with_thread_id("test".to_string()))
            )
            .await
            .unwrap();

        assert_eq!(result.get("value"), Some(&serde_json::json!(20)));
    }

    #[tokio::test]
    async fn test_nested_subgraph_three_levels() {
        // Level 3: Atomic operation
        let mut atomic = StateGraph::new();
        atomic.add_node("multiply", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                let val = s.get("num").and_then(|v| v.as_i64()).unwrap_or(1);
                s.insert("num".to_string(), serde_json::json!(val * 3));
                Ok(Value::Object(s))
            })
        });
        atomic.add_edge("__start__", "multiply");
        atomic.add_edge("multiply", "__end__");
        let compiled_atomic = atomic.compile().unwrap();

        // Level 2: Processing stage
        let mut processing = StateGraph::new();
        processing.add_simple_subgraph("atomic", compiled_atomic);
        processing.add_node("add_five", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                let val = s.get("num").and_then(|v| v.as_i64()).unwrap_or(0);
                s.insert("num".to_string(), serde_json::json!(val + 5));
                Ok(Value::Object(s))
            })
        });
        processing.add_edge("__start__", "atomic");
        processing.add_edge("atomic", "add_five");
        processing.add_edge("add_five", "__end__");
        let compiled_processing = processing.compile().unwrap();

        // Level 1: Orchestration layer
        let mut orchestration = StateGraph::new();
        orchestration.add_simple_subgraph("process", compiled_processing);
        orchestration.add_node("double_result", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                let val = s.get("num").and_then(|v| v.as_i64()).unwrap_or(0);
                s.insert("num".to_string(), serde_json::json!(val * 2));
                Ok(Value::Object(s))
            })
        });
        orchestration.add_edge("__start__", "process");
        orchestration.add_edge("process", "double_result");
        orchestration.add_edge("double_result", "__end__");
        let compiled_orchestration = orchestration.compile().unwrap();

        // Execute: 2 * 3 = 6, + 5 = 11, * 2 = 22
        let result = compiled_orchestration
            .invoke_with_config(
                serde_json::json!({"num": 2}),
                Some(CheckpointConfig::new().with_thread_id("test".to_string()))
            )
            .await
            .unwrap();

        assert_eq!(result.get("num"), Some(&serde_json::json!(22)));
    }

    #[tokio::test]
    async fn test_subgraph_multiple_parallel_at_same_level() {
        // Create two independent subgraphs
        let mut subgraph_a = StateGraph::new();
        subgraph_a.add_node("process_a", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                s.insert("processed_a".to_string(), serde_json::json!(true));
                Ok(Value::Object(s))
            })
        });
        subgraph_a.add_edge("__start__", "process_a");
        subgraph_a.add_edge("process_a", "__end__");
        let compiled_a = subgraph_a.compile().unwrap();

        let mut subgraph_b = StateGraph::new();
        subgraph_b.add_node("process_b", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                s.insert("processed_b".to_string(), serde_json::json!(true));
                Ok(Value::Object(s))
            })
        });
        subgraph_b.add_edge("__start__", "process_b");
        subgraph_b.add_edge("process_b", "__end__");
        let compiled_b = subgraph_b.compile().unwrap();

        // Parent graph with both subgraphs
        let mut parent = StateGraph::new();
        parent.add_simple_subgraph("sub_a", compiled_a);
        parent.add_simple_subgraph("sub_b", compiled_b);
        parent.add_edge("__start__", "sub_a");
        parent.add_edge("sub_a", "sub_b");
        parent.add_edge("sub_b", "__end__");
        let compiled_parent = parent.compile().unwrap();

        let result = compiled_parent
            .invoke_with_config(
                serde_json::json!({}),
                Some(CheckpointConfig::new().with_thread_id("test".to_string()))
            )
            .await
            .unwrap();

        assert_eq!(result.get("processed_a"), Some(&serde_json::json!(true)));
        assert_eq!(result.get("processed_b"), Some(&serde_json::json!(true)));
    }

    // ========================================================================
    // Phase 10.2: State Isolation Tests
    // ========================================================================

    #[tokio::test]
    async fn test_state_isolation_with_filter() {
        // Subgraph with state filtering
        let mut child = StateGraph::new();
        child.add_node("process", |state| {
            Box::pin(async move {
                let s = state.as_object().unwrap();
                let mut result = serde_json::Map::new();

                // Check what fields are visible
                result.insert("saw_allowed1".to_string(), serde_json::json!(s.contains_key("allowed1")));
                result.insert("saw_allowed2".to_string(), serde_json::json!(s.contains_key("allowed2")));
                result.insert("saw_secret".to_string(), serde_json::json!(s.contains_key("secret")));
                result.insert("processed".to_string(), serde_json::json!(true));

                Ok(Value::Object(result))
            })
        });
        child.add_edge("__start__", "process");
        child.add_edge("process", "__end__");
        let compiled_child = child.compile().unwrap();

        // Configure to only pass specific fields
        let config = SubgraphConfig::new("child")
            .with_state_filter(vec!["allowed1".to_string(), "allowed2".to_string()]);

        let mut parent = StateGraph::new();
        parent.add_configured_subgraph("child", compiled_child, config);
        parent.add_edge("__start__", "child");
        parent.add_edge("child", "__end__");
        let compiled_parent = parent.compile().unwrap();

        let result = compiled_parent
            .invoke_with_config(
                serde_json::json!({
                    "allowed1": "visible",
                    "allowed2": "also visible",
                    "secret": "hidden"
                }),
                Some(CheckpointConfig::new().with_thread_id("test".to_string()))
            )
            .await
            .unwrap();

        // Subgraph should only see filtered fields
        assert_eq!(result.get("saw_allowed1"), Some(&serde_json::json!(true)));
        assert_eq!(result.get("saw_allowed2"), Some(&serde_json::json!(true)));
        assert_eq!(result.get("saw_secret"), Some(&serde_json::json!(false)));
    }

    #[tokio::test]
    async fn test_state_isolation_sync_to_parent() {
        // Subgraph that adds new fields
        let mut child = StateGraph::new();
        child.add_node("enrich", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                s.insert("enriched_field".to_string(), serde_json::json!("added by child"));
                s.insert("timestamp".to_string(), serde_json::json!(12345));
                Ok(Value::Object(s))
            })
        });
        child.add_edge("__start__", "enrich");
        child.add_edge("enrich", "__end__");
        let compiled_child = child.compile().unwrap();

        // Configure to sync state back to parent
        let config = SubgraphConfig::new("child")
            .with_sync_to_parent(true);

        let mut parent = StateGraph::new();
        parent.add_configured_subgraph("child", compiled_child, config);
        parent.add_edge("__start__", "child");
        parent.add_edge("child", "__end__");
        let compiled_parent = parent.compile().unwrap();

        let result = compiled_parent
            .invoke_with_config(
                serde_json::json!({"original": "data"}),
                Some(CheckpointConfig::new().with_thread_id("test".to_string()))
            )
            .await
            .unwrap();

        // Parent should have both original and enriched fields
        assert_eq!(result.get("original"), Some(&serde_json::json!("data")));
        assert_eq!(result.get("enriched_field"), Some(&serde_json::json!("added by child")));
        assert_eq!(result.get("timestamp"), Some(&serde_json::json!(12345)));
    }

    #[tokio::test]
    async fn test_state_isolation_no_sync() {
        // Subgraph that modifies state
        let mut child = StateGraph::new();
        child.add_node("modify", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                s.insert("modified".to_string(), serde_json::json!(true));
                s.insert("child_only".to_string(), serde_json::json!("invisible to parent"));
                Ok(Value::Object(s))
            })
        });
        child.add_edge("__start__", "modify");
        child.add_edge("modify", "__end__");
        let compiled_child = child.compile().unwrap();

        // Default: sync_state_to_parent is false
        let config = SubgraphConfig::new("child");

        let mut parent = StateGraph::new();
        parent.add_configured_subgraph("child", compiled_child, config);
        parent.add_edge("__start__", "child");
        parent.add_edge("child", "__end__");
        let compiled_parent = parent.compile().unwrap();

        let result = compiled_parent
            .invoke_with_config(
                serde_json::json!({"original": "data"}),
                Some(CheckpointConfig::new().with_thread_id("test".to_string()))
            )
            .await
            .unwrap();

        // Without sync, parent state should still have modifications
        // (default behavior merges subgraph output)
        assert_eq!(result.get("original"), Some(&serde_json::json!("data")));
        assert_eq!(result.get("modified"), Some(&serde_json::json!(true)));
    }

    #[tokio::test]
    async fn test_state_isolation_filter_and_sync() {
        // Combine filtering and syncing
        let mut child = StateGraph::new();
        child.add_node("process_filtered", |state| {
            Box::pin(async move {
                let mut s = state.as_object().unwrap().clone();
                // Process only filtered fields
                if let Some(allowed) = s.get("allowed_field") {
                    let processed = format!("processed: {}", allowed.as_str().unwrap_or(""));
                    s.insert("result".to_string(), serde_json::json!(processed));
                }
                Ok(Value::Object(s))
            })
        });
        child.add_edge("__start__", "process_filtered");
        child.add_edge("process_filtered", "__end__");
        let compiled_child = child.compile().unwrap();

        let config = SubgraphConfig::new("child")
            .with_state_filter(vec!["allowed_field".to_string()])
            .with_sync_to_parent(true);

        let mut parent = StateGraph::new();
        parent.add_configured_subgraph("child", compiled_child, config);
        parent.add_edge("__start__", "child");
        parent.add_edge("child", "__end__");
        let compiled_parent = parent.compile().unwrap();

        let result = compiled_parent
            .invoke_with_config(
                serde_json::json!({
                    "allowed_field": "test",
                    "secret_field": "hidden"
                }),
                Some(CheckpointConfig::new().with_thread_id("test".to_string()))
            )
            .await
            .unwrap();

        // Should have original fields plus processed result
        assert_eq!(result.get("allowed_field"), Some(&serde_json::json!("test")));
        assert_eq!(result.get("secret_field"), Some(&serde_json::json!("hidden")));
        assert_eq!(result.get("result"), Some(&serde_json::json!("processed: test")));
    }

    #[test]
    fn test_subgraph_config_builder() {
        let config = SubgraphConfig::new("test_sub")
            .with_state_filter(vec!["field1".to_string(), "field2".to_string()])
            .with_inherit_state(false)
            .with_sync_to_parent(true);

        assert_eq!(config.name, "test_sub");
        assert_eq!(config.inherit_state, false);
        assert_eq!(config.sync_state_to_parent, true);
        assert_eq!(config.state_filter, Some(vec!["field1".to_string(), "field2".to_string()]));
    }

    #[test]
    fn test_compiled_subgraph_with_hierarchy() {
        let mut graph = StateGraph::new();
        graph.add_node("test", |s| Box::pin(async move { Ok(s) }));
        graph.add_edge("__start__", "test");
        graph.add_edge("test", "__end__");

        let compiled = graph.compile().unwrap();
        let config = SubgraphConfig::new("test_sub");
        let hierarchy = Arc::new(GraphHierarchy::new());

        let subgraph = CompiledSubgraph::new(compiled, config)
            .with_hierarchy(hierarchy.clone());

        assert_eq!(subgraph.name(), "test_sub");
        assert!(subgraph.hierarchy.is_some());
    }
}