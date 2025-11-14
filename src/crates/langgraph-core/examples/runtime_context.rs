//! Example demonstrating runtime context access from within nodes
//!
//! This example shows how nodes can access runtime context during execution:
//! - Current step number
//! - Remaining steps
//! - Store for persistent data
//! - Stream writer for custom events
//!
//! Run with: cargo run --example runtime_context

use langgraph_core::{StateGraph, InMemoryStore, get_runtime, get_store};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Runtime Context Example");
    println!("=======================\n");

    // Create a state graph
    let mut graph = StateGraph::new();

    // Add a node that accesses runtime context
    graph.add_node("step1", |state| {
        Box::pin(async move {
            println!("Node: step1");

            // Access runtime context
            if let Some(runtime) = get_runtime() {
                println!("  Current step: {}", runtime.current_step());
                println!("  Remaining steps: {}", runtime.remaining_steps());
                println!("  Is last step: {}", runtime.is_last_step());
                println!("  Current node: {:?}", runtime.current_node());

                // Access store if available
                if let Some(store) = runtime.store() {
                    println!("  Store is available");

                    // Put a value in the store
                    store.put("my_key", json!({"visited": "step1"})).await.ok();
                }

                // Write custom event to stream
                if let Some(writer) = runtime.stream_writer() {
                    writer.write(json!({
                        "event": "custom_event",
                        "node": "step1",
                        "message": "Processing in step1"
                    })).ok();
                }
            } else {
                println!("  No runtime context available");
            }

            // Update state
            let mut result = state.clone();
            result["step1_complete"] = json!(true);
            result["count"] = json!(result["count"].as_i64().unwrap_or(0) + 1);

            Ok(result)
        })
    });

    // Add another node
    graph.add_node("step2", |state| {
        Box::pin(async move {
            println!("\nNode: step2");

            // Access runtime context
            if let Some(runtime) = get_runtime() {
                println!("  Current step: {}", runtime.current_step());
                println!("  Remaining steps: {}", runtime.remaining_steps());
                println!("  Current node: {:?}", runtime.current_node());

                // Access store using convenience function
                if let Some(store) = get_store() {
                    // Check if step1 stored something
                    if let Ok(Some(value)) = store.get("my_key").await {
                        println!("  Found data from step1: {}", value);
                    }

                    // Store our own data
                    store.put("step2_key", json!({"visited": "step2"})).await.ok();
                }

                // Write custom event
                if let Some(writer) = runtime.stream_writer() {
                    writer.write(json!({
                        "event": "custom_event",
                        "node": "step2",
                        "message": "Processing in step2"
                    })).ok();
                }
            }

            // Update state
            let mut result = state.clone();
            result["step2_complete"] = json!(true);
            result["count"] = json!(result["count"].as_i64().unwrap_or(0) + 1);

            Ok(result)
        })
    });

    // Add final node
    graph.add_node("step3", |state| {
        Box::pin(async move {
            println!("\nNode: step3 (final)");

            // Access runtime context
            if let Some(runtime) = get_runtime() {
                println!("  Current step: {}", runtime.current_step());
                println!("  Remaining steps: {}", runtime.remaining_steps());
                println!("  Is last step: {}", runtime.is_last_step());
                println!("  Current node: {:?}", runtime.current_node());

                // Check store for data from previous nodes
                if let Some(store) = get_store() {
                    if let Ok(Some(value)) = store.get("my_key").await {
                        println!("  Data from step1: {}", value);
                    }
                    if let Ok(Some(value)) = store.get("step2_key").await {
                        println!("  Data from step2: {}", value);
                    }
                }
            }

            // Update state
            let mut result = state.clone();
            result["step3_complete"] = json!(true);
            result["count"] = json!(result["count"].as_i64().unwrap_or(0) + 1);

            Ok(result)
        })
    });

    // Add edges
    graph.add_edge("__start__", "step1");
    graph.add_edge("step1", "step2");
    graph.add_edge("step2", "step3");
    graph.add_edge("step3", "__end__");

    // Compile the graph with a store
    let store = Arc::new(InMemoryStore::new());
    let compiled = graph.compile_with_store(store)?;

    // Create initial state
    let initial_state = json!({
        "count": 0,
        "message": "Starting execution"
    });

    println!("Executing graph...\n");

    // Execute the graph
    let final_state = compiled.invoke(initial_state).await?;

    println!("\n\nExecution Complete!");
    println!("===================");
    println!("Final state: {}", serde_json::to_string_pretty(&final_state)?);

    Ok(())
}
