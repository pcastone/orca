//! Internal pregel loop builder
//!
//! This module contains private helpers for building PregelLoop instances.

use super::CompiledGraph;
use crate::error::Result;
use crate::graph::{Edge, END, START, TASKS};
use crate::pregel::{
    Checkpoint as PregelCheckpoint, ChannelVersion, LastValueChannel, NodeExecutor,
    PregelLoop, PregelNodeSpec,
};
use langgraph_checkpoint::{BinaryOperatorChannel, Channel, TopicChannel};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

impl CompiledGraph {
    /// Build a PregelLoop from the graph structure
    ///
    /// This is a helper method that sets up the Pregel execution context
    /// including channels, node specs, and initial checkpoint.
    ///
    /// # Arguments
    ///
    /// * `input` - Initial state
    ///
    /// # Returns
    ///
    /// PregelLoop ready for execution
    pub(crate) fn build_pregel_loop(
        &self,
        input: Value,
    ) -> Result<PregelLoop> {
        // 1. Create initial checkpoint
        let mut checkpoint = PregelCheckpoint::new();

        // 2. Build reverse edge map: node → list of predecessors
        let mut incoming_edges: HashMap<String, Vec<String>> = HashMap::new();

        for (from_node, edges) in &self.graph.edges {
            for edge in edges {
                let to_node = match edge {
                    Edge::Direct(node_id) => node_id.clone(),
                    Edge::Conditional { .. } => {
                        // For conditional edges, DO NOT add to incoming_edges
                        // The conditional edge evaluation will handle routing dynamically
                        // at runtime based on the router function result
                        continue;
                    }
                };

                incoming_edges
                    .entry(to_node)
                    .or_default()
                    .push(from_node.clone());
            }
        }

        // 3. Create channels: one per node (represents that node's output)
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();

        // Create START channel with initial input
        channels.insert(
            START.to_string(),
            Box::new(LastValueChannel::with_value(input.clone())),
        );
        checkpoint
            .channel_versions
            .insert(START.to_string(), ChannelVersion::Int(1));

        // Create channel for each regular node
        for node_id in self.graph.nodes.keys() {
            channels.insert(node_id.clone(), Box::new(LastValueChannel::new()));
        }

        // Create END channel
        channels.insert(END.to_string(), Box::new(LastValueChannel::new()));

        // Create TASKS channel for dynamic task spawning (Send objects)
        // This is a Topic channel that accumulates Send objects from nodes
        channels.insert(TASKS.to_string(), Box::new(TopicChannel::new()));

        // Create custom channels from graph.channels
        for (channel_name, channel_spec) in &self.graph.channels {
            // Skip if already created (e.g., node output channels)
            if channels.contains_key(channel_name) {
                continue;
            }

            // Create the appropriate channel type based on spec
            let channel: Box<dyn Channel> = if let Some(reducer) = &channel_spec.reducer {
                // If there's a reducer, use BinaryOperatorChannel regardless of type
                let reducer_clone = Arc::clone(reducer);
                Box::new(BinaryOperatorChannel::new(move |left, right| {
                    reducer_clone(left, right)
                }))
            } else {
                // No reducer - create based on type
                match channel_spec.channel_type {
                    crate::graph::ChannelType::LastValue => Box::new(LastValueChannel::new()),
                    crate::graph::ChannelType::Topic => Box::new(TopicChannel::new()),
                    crate::graph::ChannelType::BinaryOp => {
                        // BinaryOp without reducer doesn't make sense, but default to LastValue
                        Box::new(LastValueChannel::new())
                    }
                }
            };

            channels.insert(channel_name.clone(), channel);
        }

        // Initialize custom channels with values from input
        for (channel_name, channel_spec) in &self.graph.channels {
            if let Some(channel) = channels.get_mut(channel_name) {
                // For StateGraph with "state" channel, use the entire input as initial state
                if channel_name == "state" {
                    let _ = channel.update(vec![input.clone()]);
                    checkpoint
                        .channel_versions
                        .insert(channel_name.clone(), ChannelVersion::Int(1));
                } else if let Some(input_obj) = input.as_object() {
                    // For other channels (like "messages"), look for matching field in input
                    if let Some(value) = input_obj.get(&channel_spec.name) {
                        // For channels with reducers that expect arrays (like add_messages),
                        // we need to bootstrap by setting empty array first, then merging the input
                        if channel_spec.reducer.is_some() && value.is_array() {
                            // First set empty array as initial value
                            let _ = channel.update(vec![serde_json::json!([])]);
                            // Then merge the input array using the reducer
                            let _ = channel.update(vec![value.clone()]);
                        } else {
                            // No reducer or not an array - just set the value directly
                            let _ = channel.update(vec![value.clone()]);
                        }
                        checkpoint
                            .channel_versions
                            .insert(channel_name.clone(), ChannelVersion::Int(1));
                    }
                }
            }
        }

        // 4. Convert graph nodes to Pregel node specs
        let mut pregel_nodes = HashMap::new();

        for (node_id, node_spec) in &self.graph.nodes {
            // Determine which channels trigger this node (its predecessors)
            let triggers = incoming_edges
                .get(node_id)
                .cloned()
                .unwrap_or_else(|| vec![START.to_string()]);

            // Wrap the existing executor in a Pregel-compatible adapter
            let executor_clone = node_spec.executor.clone();
            let edges_clone = self.graph.edges.get(node_id).cloned();

            let adapter = GraphExecutorAdapterWithEdges {
                executor: executor_clone,
                node_id: node_id.clone(),
                edges: edges_clone,
            };

            pregel_nodes.insert(
                node_id.clone(),
                PregelNodeSpec {
                    name: node_id.clone(),
                    triggers,
                    reads: node_spec.reads.clone(),
                    writes: node_spec.writes.clone(),
                    executor: Arc::new(adapter),
                },
            );
        }

        // 5. Create PregelLoop with edges for conditional routing
        let mut pregel_loop = PregelLoop::new_with_edges(
            checkpoint,
            channels,
            pregel_nodes,
            100,
            self.graph.edges.clone(),
        );

        // 6. Add store if available
        if let Some(store) = &self.store {
            pregel_loop = pregel_loop.with_store(store.clone());
        }

        Ok(pregel_loop)
    }

    /// Execute multiple inputs in parallel
    ///
    /// # Arguments
    ///
    /// * `inputs` - Vector of initial states
    ///
    /// # Returns
    ///
    /// Vector of final states after execution
    pub async fn batch(&self, inputs: Vec<Value>) -> Result<Vec<Value>> {
        self.batch_with_config(inputs, None).await
    }

    /// Execute multiple inputs in parallel with configuration
    ///
    /// # Arguments
    ///
    /// * `inputs` - Vector of initial states
    /// * `config` - Optional checkpoint configuration
    ///
    /// # Returns
    ///
    /// Vector of final states after execution
    pub async fn batch_with_config(
        &self,
        inputs: Vec<Value>,
        config: Option<langgraph_checkpoint::CheckpointConfig>,
    ) -> Result<Vec<Value>> {
        // Execute all inputs in parallel
        let mut tasks = Vec::new();

        for input in inputs {
            let cfg = config.clone();
            let future = self.invoke_with_config(input, cfg);
            tasks.push(future);
        }

        // Wait for all tasks to complete
        let results = futures::future::join_all(tasks).await;

        // Collect results or return first error
        let mut outputs = Vec::new();
        for result in results {
            outputs.push(result?);
        }

        Ok(outputs)
    }
}

/// Adapter that wraps a graph NodeExecutor to make it compatible with Pregel's NodeExecutor trait
///
/// This adapter also stores the edges for this node, which are needed for conditional routing
struct GraphExecutorAdapterWithEdges {
    executor: crate::graph::NodeExecutor,
    node_id: String,
    edges: Option<Vec<Edge>>,
}

impl NodeExecutor for GraphExecutorAdapterWithEdges {
    fn execute(
        &self,
        input: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
        let executor = self.executor.clone();
        let _edges = self.edges.clone();

        Box::pin(async move {
            // Execute the node
            let result = executor(input)
                .await
                .map_err(|e| crate::error::GraphError::Execution(e.to_string()))?;

            // The result will be written to this node's channel by the loop
            Ok(result)
        })
    }
}
