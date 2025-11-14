//! Simple graph example
//!
//! This example demonstrates how to create a basic graph with sequential execution.

use langgraph_core::StateGraph;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple Graph Example ===\n");

    // Create a new graph
    let mut graph = StateGraph::new();

    // Add a node that processes the input
    graph.add_node("step1", |mut state| {
        Box::pin(async move {
            println!("Executing step1...");
            if let Some(obj) = state.as_object_mut() {
                let value = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("value".to_string(), json!(value + 10));
                obj.insert("step1_executed".to_string(), json!(true));
            }
            println!("Step1 complete. State: {}", state);
            Ok(state)
        })
    });

    // Add another node
    graph.add_node("step2", |mut state| {
        Box::pin(async move {
            println!("Executing step2...");
            if let Some(obj) = state.as_object_mut() {
                let value = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("value".to_string(), json!(value * 2));
                obj.insert("step2_executed".to_string(), json!(true));
            }
            println!("Step2 complete. State: {}", state);
            Ok(state)
        })
    });

    // Define the execution flow
    graph.add_edge("__start__", "step1");
    graph.add_edge("step1", "step2");
    graph.add_edge("step2", "__end__");

    // Compile the graph
    let compiled = graph.compile()?;

    // Execute the graph
    let input = json!({
        "value": 5
    });

    println!("Initial state: {}\n", input);

    let result = compiled.invoke(input).await?;

    println!("\nFinal state: {}", result);
    println!("\nExpected: value = (5 + 10) * 2 = 30");
    println!("Actual: value = {}", result["value"]);

    Ok(())
}
