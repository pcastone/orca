//! Graph composition methods (then, map, chain, etc.)
//!
//! This module contains methods for composing graphs together.

use super::CompiledGraph;
use crate::error::{GraphError, Result};
use serde_json::Value;
use std::sync::Arc;

impl CompiledGraph {
    /// Chain this graph with another graph sequentially
    ///
    /// Creates a new graph that executes this graph, then passes the output
    /// to the next graph.
    ///
    /// # Arguments
    ///
    /// * `next` - The graph to execute after this one
    ///
    /// # Returns
    ///
    /// A new `CompiledGraph` that sequences the two graphs
    pub fn then(self, next: CompiledGraph) -> CompiledGraph {
        use crate::builder::StateGraph;

        let mut combined = StateGraph::new();

        // Add first graph as a subgraph
        combined.add_subgraph("first", self);

        // Add second graph as a subgraph
        combined.add_subgraph("second", next);

        // Chain them sequentially
        combined.add_edge("__start__", "first");
        combined.add_edge("first", "second");
        combined.add_edge("second", "__end__");

        // Compile should not fail since we control the structure
        combined.compile().expect("Sequential composition should always succeed")
    }

    /// Transform the output of this graph
    ///
    /// Creates a new graph that executes this graph, then applies a transformation
    /// function to the output.
    ///
    /// # Arguments
    ///
    /// * `f` - Transformation function
    ///
    /// # Returns
    ///
    /// A new `CompiledGraph` with the transformation applied
    pub fn map<F>(self, f: F) -> CompiledGraph
    where
        F: Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        use crate::builder::StateGraph;

        let mut combined = StateGraph::new();

        // Add original graph as subgraph
        combined.add_subgraph("graph", self);

        // Add transformation node
        combined.add_node("transform", f);

        // Chain them
        combined.add_edge("__start__", "graph");
        combined.add_edge("graph", "transform");
        combined.add_edge("transform", "__end__");

        combined.compile().expect("Map composition should always succeed")
    }

    /// Chain multiple graphs sequentially
    ///
    /// This is a convenience method for chaining more than two graphs.
    ///
    /// # Arguments
    ///
    /// * `graphs` - Vector of graphs to chain
    ///
    /// # Returns
    ///
    /// A new `CompiledGraph` that sequences all graphs
    pub fn chain(graphs: Vec<CompiledGraph>) -> Result<CompiledGraph> {
        use crate::builder::StateGraph;

        if graphs.is_empty() {
            return Err(GraphError::Validation("Cannot chain empty graph list".to_string()));
        }

        if graphs.len() == 1 {
            return Ok(graphs.into_iter().next().unwrap());
        }

        let mut combined = StateGraph::new();
        let mut prev_node = "__start__".to_string();

        // Add each graph as a subgraph and chain them
        for (i, graph) in graphs.into_iter().enumerate() {
            let node_name = format!("step_{}", i);
            combined.add_subgraph(&node_name, graph);
            combined.add_edge(&prev_node, &node_name);
            prev_node = node_name;
        }

        // Connect last node to end
        combined.add_edge(&prev_node, "__end__");

        combined.compile()
    }

    /// Execute this graph conditionally based on a predicate
    ///
    /// Creates a new graph that only executes this graph if the predicate returns true.
    /// If false, the input state passes through unchanged.
    ///
    /// # Arguments
    ///
    /// * `predicate` - Function that determines whether to execute the graph
    ///
    /// # Returns
    ///
    /// A new `CompiledGraph` with conditional execution
    pub fn when<F>(self, predicate: F) -> CompiledGraph
    where
        F: Fn(&Value) -> bool + Send + Sync + 'static,
    {
        use crate::builder::StateGraph;
        use std::collections::HashMap;

        let mut combined = StateGraph::new();

        // Add router node that checks predicate
        let predicate = Arc::new(predicate);
        combined.add_node("router", |state| {
            Box::pin(async move {
                // Just pass state through - routing is done via conditional edge
                Ok(state)
            })
        });

        // Add the conditional graph as a subgraph
        combined.add_subgraph("conditional_graph", self);

        // Add passthrough node (does nothing)
        combined.add_node("passthrough", |state| {
            Box::pin(async move { Ok(state) })
        });

        // Set up conditional routing
        let mut branches = HashMap::new();
        branches.insert("execute".to_string(), "conditional_graph".to_string());
        branches.insert("skip".to_string(), "passthrough".to_string());

        combined.add_edge("__start__", "router");
        let predicate_clone = predicate.clone();
        combined.add_conditional_edge(
            "router",
            move |state| {
                use crate::send::ConditionalEdgeResult;
                if predicate_clone(state) {
                    ConditionalEdgeResult::Node("execute".to_string())
                } else {
                    ConditionalEdgeResult::Node("skip".to_string())
                }
            },
            branches,
        );
        combined.add_finish("conditional_graph");
        combined.add_finish("passthrough");

        combined.compile().expect("Conditional composition should always succeed")
    }
}
