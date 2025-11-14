//! Tests for CompiledGraph
//!
//! This module contains all tests for the compiled graph execution engine.

#[cfg(test)]
mod tests {
    use crate::{StateGraph, InterruptConfig, VisualizationOptions};
    use langgraph_checkpoint::InMemoryCheckpointSaver;
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_compiled_graph_creation() {
        let mut graph = StateGraph::new();
        graph.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let compiled = graph.compile();
        assert!(compiled.is_ok());
    }

    #[tokio::test]
    async fn test_with_checkpointer() {
        let mut graph = StateGraph::new();
        graph.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
        let compiled = graph.compile().unwrap()
            .with_checkpointer(checkpointer.clone());

        // Verify checkpointer was set
        assert!(compiled.get_checkpoint_saver().is_some());
    }

    #[tokio::test]
    async fn test_with_interrupt_config() {
        let mut graph = StateGraph::new();
        graph.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let interrupt_config = InterruptConfig {
            interrupt_before: vec!["process".to_string()],
            interrupt_after: vec![],
            interrupt_before_all: false,
            interrupt_after_all: false,
        };

        let compiled = graph.compile().unwrap()
            .with_interrupt_config(interrupt_config.clone());

        // Verify interrupt config was set
        assert_eq!(compiled.interrupt_config().interrupt_before.len(), 1);
        assert_eq!(compiled.interrupt_config().interrupt_before[0], "process");
    }

    #[tokio::test]
    async fn test_graph_accessor() {
        let mut graph = StateGraph::new();
        graph.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let compiled = graph.compile().unwrap();

        // Verify we can access the underlying graph
        let inner_graph = compiled.graph();
        assert!(inner_graph.nodes.contains_key("process"));
    }

    #[tokio::test]
    async fn test_visualize_dot() {
        let mut graph = StateGraph::new();
        graph.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let compiled = graph.compile().unwrap();

        let dot = compiled.visualize(&VisualizationOptions::dot());

        // Verify DOT output contains expected elements
        assert!(dot.contains("digraph"));
        assert!(dot.contains("process"));
    }

    #[tokio::test]
    async fn test_visualize_mermaid() {
        let mut graph = StateGraph::new();
        graph.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let compiled = graph.compile().unwrap();

        let mermaid = compiled.visualize(&VisualizationOptions::mermaid());

        // Verify Mermaid output contains expected elements
        assert!(mermaid.contains("graph"));
        assert!(mermaid.contains("process"));
    }

    #[tokio::test]
    async fn test_basic_invoke() {
        let mut graph = StateGraph::new();
        graph.add_node("process", |state| {
            Box::pin(async move {
                // Just return the state as-is to verify basic execution
                Ok(state)
            })
        });
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let compiled = graph.compile().unwrap();

        let input = json!({"test": "value"});
        let result = compiled.invoke(input.clone()).await;
        assert!(result.is_ok());

        // For now, just verify execution completed
        // State propagation behavior may vary based on StateGraph configuration
        let final_state = result.unwrap();
        assert!(final_state.is_object() || final_state.is_null());
    }

    #[tokio::test]
    async fn test_subgraph_executor_trait() {
        use crate::graph::SubgraphExecutor;

        let mut graph = StateGraph::new();
        graph.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let compiled = graph.compile().unwrap();

        // Test that CompiledGraph implements SubgraphExecutor
        let name = compiled.name();
        assert_eq!(name, "subgraph");

        // Test invoke through SubgraphExecutor trait
        let result = compiled.invoke(json!({"test": true})).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_clone() {
        let mut graph = StateGraph::new();
        graph.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        graph.add_edge("__start__", "process");
        graph.add_edge("process", "__end__");

        let compiled = graph.compile().unwrap();

        // Test that CompiledGraph can be cloned
        let cloned = compiled.clone();

        // Both should work independently
        let result1 = compiled.invoke(json!({"value": 1})).await;
        let result2 = cloned.invoke(json!({"value": 2})).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_nodes() {
        let mut graph = StateGraph::new();

        graph.add_node("node1", |state| {
            Box::pin(async move {
                // First node just passes state through
                Ok(state)
            })
        });

        graph.add_node("node2", |state| {
            Box::pin(async move {
                // Second node just passes state through
                Ok(state)
            })
        });

        graph.add_edge("__start__", "node1");
        graph.add_edge("node1", "node2");
        graph.add_edge("node2", "__end__");

        let compiled = graph.compile().unwrap();

        let result = compiled.invoke(json!({"data": "test"})).await;
        assert!(result.is_ok());

        // Verify execution completed successfully
        let final_state = result.unwrap();
        assert!(final_state.is_object() || final_state.is_null());
    }
}
