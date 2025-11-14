//! CompiledGraph struct and builder methods
//!
//! This module contains the CompiledGraph type and its constructor/builder methods.

use crate::error::Result;
use crate::graph::Graph;
use crate::interrupt::InterruptConfig;
use langgraph_checkpoint::CheckpointSaver;
use std::sync::Arc;

/// Compiled graph ready for execution
#[derive(Clone)]
pub struct CompiledGraph {
    pub(crate) graph: Graph,
    pub(crate) checkpoint_saver: Option<Arc<dyn CheckpointSaver>>,
    pub(crate) interrupt_config: InterruptConfig,
    pub(crate) store: Option<Arc<dyn crate::store::Store>>,
}

impl CompiledGraph {
    /// Create a new compiled graph
    pub(crate) fn new(graph: Graph) -> Result<Self> {
        Ok(Self {
            graph,
            checkpoint_saver: None,
            interrupt_config: InterruptConfig::default(),
            store: None,
        })
    }

    /// Create a new compiled graph with interrupt configuration
    pub(crate) fn new_with_interrupts(graph: Graph, interrupt_config: InterruptConfig) -> Result<Self> {
        Ok(Self {
            graph,
            checkpoint_saver: None,
            interrupt_config,
            store: None,
        })
    }

    /// Set the checkpoint saver
    pub fn with_checkpointer(mut self, saver: Arc<dyn CheckpointSaver>) -> Self {
        self.checkpoint_saver = Some(saver);
        self
    }

    /// Set the store for persistent state access
    pub fn with_store(mut self, store: Arc<dyn crate::store::Store>) -> Self {
        self.store = Some(store);
        self
    }

    /// Visualize the graph structure
    ///
    /// Returns a string representation of the graph in the specified format.
    ///
    /// # Arguments
    ///
    /// * `options` - Visualization options (format, details, etc.)
    ///
    /// # Returns
    ///
    /// String containing the graph visualization
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::{StateGraph, VisualizationOptions};
    ///
    /// let mut graph = StateGraph::new();
    /// graph.add_node("process", |state| {
    ///     Box::pin(async move { Ok(state) })
    /// });
    /// graph.add_edge("__start__", "process");
    /// graph.add_edge("process", "__end__");
    ///
    /// let compiled = graph.compile().unwrap();
    ///
    /// // Generate DOT format
    /// let dot = compiled.visualize(&VisualizationOptions::dot());
    /// println!("{}", dot);
    ///
    /// // Generate Mermaid format
    /// let mermaid = compiled.visualize(&VisualizationOptions::mermaid());
    /// println!("{}", mermaid);
    /// ```
    pub fn visualize(&self, options: &crate::visualization::VisualizationOptions) -> String {
        crate::visualization::visualize(&self.graph, options)
    }

    /// Get a reference to the underlying graph
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Get the interrupt configuration
    pub fn interrupt_config(&self) -> &InterruptConfig {
        &self.interrupt_config
    }

    /// Set the interrupt configuration
    pub fn with_interrupt_config(mut self, config: InterruptConfig) -> Self {
        self.interrupt_config = config;
        self
    }

    /// Get the checkpoint saver (internal use)
    pub(crate) fn get_checkpoint_saver(&self) -> Option<Arc<dyn CheckpointSaver>> {
        self.checkpoint_saver.clone()
    }
}

// Implement SubgraphExecutor so CompiledGraph can be used as a subgraph
impl crate::graph::SubgraphExecutor for CompiledGraph {
    fn invoke(
        &self,
        state: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>> + Send>> {
        // Clone self to move into the future
        let graph = self.clone();
        Box::pin(async move {
            graph.invoke(state)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        })
    }

    fn name(&self) -> &str {
        "subgraph"
    }
}
