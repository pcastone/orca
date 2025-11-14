//! Conditional routing example
//!
//! This example demonstrates how to use conditional edges to route execution
//! based on state values.

use langgraph_core::StateGraph;
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Conditional Routing Example ===\n");

    // Create a new graph
    let mut graph = StateGraph::new();

    // Router node that doesn't modify state
    graph.add_node("router", |state| {
        Box::pin(async move {
            println!("Router: Examining state...");
            Ok(state)
        })
    });

    // Path A: Multiply by 2
    graph.add_node("multiply", |mut state| {
        Box::pin(async move {
            println!("Taking multiply path...");
            if let Some(obj) = state.as_object_mut() {
                let value = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("value".to_string(), json!(value * 2));
                obj.insert("operation".to_string(), json!("multiply"));
            }
            Ok(state)
        })
    });

    // Path B: Add 100
    graph.add_node("add", |mut state| {
        Box::pin(async move {
            println!("Taking add path...");
            if let Some(obj) = state.as_object_mut() {
                let value = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("value".to_string(), json!(value + 100));
                obj.insert("operation".to_string(), json!("add"));
            }
            Ok(state)
        })
    });

    // Define edges
    graph.add_edge("__start__", "router");

    // Conditional edge based on 'action' field
    let mut branches = HashMap::new();
    branches.insert("multiply".to_string(), "multiply".to_string());
    branches.insert("add".to_string(), "add".to_string());

    graph.add_conditional_edge(
        "router",
        |state| {
            use langgraph_core::send::ConditionalEdgeResult;
            let action = state
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("add");
            println!("Routing decision: {} path", action);
            ConditionalEdgeResult::Node(action.to_string())
        },
        branches,
    );

    graph.add_finish("multiply");
    graph.add_finish("add");

    // Compile the graph
    let compiled = graph.compile()?;

    // Test multiply path
    println!("\n--- Test 1: Multiply Path ---");
    let input1 = json!({
        "value": 10,
        "action": "multiply"
    });
    println!("Input: {}", input1);
    let result1 = compiled.invoke(input1).await?;
    println!("Output: {}\n", result1);

    // Test add path
    println!("--- Test 2: Add Path ---");
    let input2 = json!({
        "value": 10,
        "action": "add"
    });
    println!("Input: {}", input2);
    let result2 = compiled.invoke(input2).await?;
    println!("Output: {}\n", result2);

    Ok(())
}
