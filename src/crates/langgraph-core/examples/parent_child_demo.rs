//! Demonstration of parent-child graph communication
//!
//! This example shows how subgraphs can communicate with parent graphs
//! using messages, shared state, and commands.

use langgraph_core::{
    StateGraph,
    parent_child::{SubgraphConfig, send_to_parent, CommandParentExt},
    subgraph::StateGraphSubgraphExt,
    Command, CheckpointConfig,
};
use langgraph_checkpoint::InMemoryCheckpointSaver;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Parent-Child Graph Communication Demo ===\n");

    // === Create Child Graph ===
    // This child graph processes items and sends status updates to the parent
    let mut child_graph = StateGraph::new();

    child_graph.add_node("validate", |state| {
        Box::pin(async move {
            println!("  [Child] Validating input...");

            let value = state.get("value")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            // Send status to parent
            send_to_parent(
                "status_update",
                json!({
                    "step": "validation",
                    "status": "in_progress",
                    "value": value
                })
            ).ok();

            // Validate the value
            let is_valid = value > 0 && value < 1000;

            let mut result = state.as_object().unwrap().clone();
            result.insert("validated".to_string(), json!(is_valid));

            if !is_valid {
                // Send error to parent
                send_to_parent(
                    "validation_error",
                    json!({
                        "reason": "Value out of range",
                        "value": value
                    })
                ).ok();
            }

            Ok(json!(result))
        })
    });

    child_graph.add_node("process", |state| {
        Box::pin(async move {
            println!("  [Child] Processing value...");

            let value = state.get("value")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            // Send progress update to parent
            send_to_parent(
                "progress",
                json!({
                    "step": "processing",
                    "progress": 50,
                    "message": "Processing data"
                })
            ).ok();

            // Simulate processing
            let processed_value = value * 2;

            // Send completion to parent
            send_to_parent(
                "progress",
                json!({
                    "step": "processing",
                    "progress": 100,
                    "result": processed_value
                })
            ).ok();

            let mut result = state.as_object().unwrap().clone();
            result.insert("processed_value".to_string(), json!(processed_value));
            result.insert("processed".to_string(), json!(true));

            Ok(json!(result))
        })
    });

    child_graph.add_node("report", |state| {
        Box::pin(async move {
            println!("  [Child] Generating report...");

            // Use Command to send final result to parent
            let final_result = json!({
                "original": state.get("value"),
                "processed": state.get("processed_value"),
                "validated": state.get("validated")
            });

            // Send command to parent to update its state
            let cmd = Command::new()
                .to_parent()
                .with_update(json!({
                    "child_result": final_result
                }));

            println!("  [Child] Sending final result to parent via command");

            // In a real implementation, this would be handled by the framework
            // For now, we'll just send as a message
            send_to_parent(
                "final_result",
                final_result
            ).ok();

            Ok(state)
        })
    });

    // Add edges for child graph
    child_graph.add_edge("__start__", "validate");
    child_graph.add_conditional_edges(
        "validate",
        |state| {
            if state.get("validated") == Some(&json!(true)) {
                vec!["process"]
            } else {
                vec!["report"]
            }
        }
    );
    child_graph.add_edge("process", "report");
    child_graph.add_edge("report", "__end__");

    let compiled_child = child_graph.compile()?;

    // === Create Parent Graph ===
    let mut parent_graph = StateGraph::new();

    parent_graph.add_node("prepare", |state| {
        Box::pin(async move {
            println!("[Parent] Preparing data for child graph...");

            let mut result = state.as_object().unwrap().clone();
            result.insert("prepared".to_string(), json!(true));
            result.insert("parent_timestamp".to_string(), json!(chrono::Utc::now().to_rfc3339()));

            Ok(json!(result))
        })
    });

    // Configure the subgraph
    let subgraph_config = SubgraphConfig::new("data_processor")
        .with_inherit_state(true)           // Child inherits parent state
        .with_sync_to_parent(true)          // Sync child results back to parent
        .with_forward_messages(true)        // Forward child messages to parent
        .with_state_filter(vec![            // Only pass specific fields to child
            "value".to_string(),
            "prepared".to_string()
        ]);

    // Add child graph as a subgraph node
    parent_graph.add_configured_subgraph(
        "run_child",
        compiled_child,
        subgraph_config
    );

    parent_graph.add_node("finalize", |state| {
        Box::pin(async move {
            println!("[Parent] Finalizing results...");

            // Check if we received child results
            if let Some(child_result) = state.get("child_result") {
                println!("[Parent] Received child result: {}",
                    serde_json::to_string_pretty(child_result)?);
            }

            let mut result = state.as_object().unwrap().clone();
            result.insert("finalized".to_string(), json!(true));
            result.insert("completion_time".to_string(), json!(chrono::Utc::now().to_rfc3339()));

            Ok(json!(result))
        })
    });

    // Add edges for parent graph
    parent_graph.add_edge("__start__", "prepare");
    parent_graph.add_edge("prepare", "run_child");
    parent_graph.add_edge("run_child", "finalize");
    parent_graph.add_edge("finalize", "__end__");

    // Compile with checkpointer
    let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
    let compiled_parent = parent_graph.compile()?.with_checkpointer(checkpointer);

    // === Execute the Graph ===
    println!("Executing parent-child graph with valid value...\n");

    let input = json!({
        "value": 42,
        "user": "demo_user"
    });

    let config = CheckpointConfig::new().with_thread_id("demo_thread".to_string());
    let result = compiled_parent.invoke_with_config(input, Some(config.clone())).await?;

    println!("\n=== Final Result ===");
    println!("{}", serde_json::to_string_pretty(&result)?);

    // === Execute with Invalid Value ===
    println!("\n=== Testing with Invalid Value ===\n");

    let invalid_input = json!({
        "value": 5000,  // Out of range
        "user": "demo_user"
    });

    let config2 = CheckpointConfig::new().with_thread_id("demo_thread_2".to_string());
    let result2 = compiled_parent.invoke_with_config(invalid_input, Some(config2)).await?;

    println!("\n=== Final Result (Invalid) ===");
    println!("{}", serde_json::to_string_pretty(&result2)?);

    println!("\n=== Demo Complete ===");

    Ok(())
}