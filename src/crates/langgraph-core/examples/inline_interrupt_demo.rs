//! Demonstration of inline interrupt functionality
//!
//! This example shows how to use the inline interrupt() function
//! to pause graph execution and request human input/approval.

use langgraph_core::{
    StateGraph,
    inline_interrupt::{
        interrupt_for_approval, interrupt_for_input, interrupt_for_edit,
        InterruptType, InlineResumeValue, ResumeAction
    },
    CheckpointConfig, GraphError
};
use langgraph_checkpoint::InMemoryCheckpointSaver;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Inline Interrupt Demo ===\n");

    // Create a graph with different types of interrupts
    let mut graph = StateGraph::new();

    // Node 1: Process data and check if approval needed
    graph.add_node("process_data", |state| {
        Box::pin(async move {
            let value = state.get("value")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            println!("Processing data with value: {}", value);

            // If value is high, request approval
            if value > 100 {
                println!("Value exceeds threshold - requesting approval...");

                // This will interrupt execution
                interrupt_for_approval(
                    "Value exceeds threshold. Continue processing?",
                    Some(json!({
                        "current_value": value,
                        "threshold": 100
                    })),
                )?;

                println!("Approval granted, continuing...");
            }

            // Update state
            let mut result = state.as_object().unwrap().clone();
            result.insert("processed".to_string(), json!(true));
            result.insert("processing_result".to_string(), json!(value * 2));

            Ok(json!(result))
        })
    });

    // Node 2: Request user input for configuration
    graph.add_node("configure", |state| {
        Box::pin(async move {
            println!("Configuring system...");

            // Request input from user
            interrupt_for_input(
                "Please provide configuration name",
                "config_name",
                Some(json!("default_config")),
                Some(json!({
                    "type": "string",
                    "minLength": 3,
                    "maxLength": 50
                })),
            )?;

            // After resume, the input will be available in state
            println!("Configuration name received");

            let mut result = state.as_object().unwrap().clone();
            result.insert("configured".to_string(), json!(true));

            Ok(json!(result))
        })
    });

    // Node 3: Allow user to edit results
    graph.add_node("review_results", |state| {
        Box::pin(async move {
            println!("Preparing results for review...");

            let current_results = json!({
                "value": state.get("value"),
                "processed": state.get("processed"),
                "processing_result": state.get("processing_result"),
                "configured": state.get("configured")
            });

            // Allow user to edit the results
            interrupt_for_edit(
                "Review and edit the results before finalizing",
                vec![
                    "processing_result".to_string(),
                    "final_notes".to_string()
                ],
                current_results.clone(),
            )?;

            println!("Results reviewed and edited");

            let mut result = state.as_object().unwrap().clone();
            result.insert("reviewed".to_string(), json!(true));

            Ok(json!(result))
        })
    });

    // Add edges
    graph.add_edge("__start__", "process_data");
    graph.add_edge("process_data", "configure");
    graph.add_edge("configure", "review_results");
    graph.add_edge("review_results", "__end__");

    // Compile with checkpointer for state persistence
    let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
    let compiled = graph.compile()?.with_checkpointer(checkpointer);

    // === First execution - will interrupt at approval ===
    println!("Starting execution with high value...\n");

    let config = CheckpointConfig::new("demo_thread");
    let initial_state = json!({
        "value": 150,  // Exceeds threshold
        "user": "demo_user"
    });

    // This will interrupt at the approval request
    let result = compiled.invoke(initial_state.clone(), config.clone()).await;

    match result {
        Err(GraphError::InlineInterrupt(interrupt)) => {
            println!("\n=== INTERRUPTED ===");
            println!("Interrupt ID: {}", interrupt.id);
            println!("Node: {}", interrupt.node);

            match &interrupt.interrupt_type {
                InterruptType::Approval { message, data } => {
                    println!("Type: Approval");
                    println!("Message: {}", message);
                    if let Some(d) = data {
                        println!("Data: {}", serde_json::to_string_pretty(d)?);
                    }
                }
                _ => {}
            }

            println!("\n=== SIMULATING USER APPROVAL ===");

            // Simulate user approving the request
            let resume_value = InlineResumeValue {
                action: ResumeAction::Continue,
                updates: Some(json!({
                    "approved_by": "manager",
                    "approval_time": "2024-01-15T10:30:00Z"
                })),
                inputs: None,
                metadata: Some(json!({
                    "approval_reason": "Value is acceptable for this operation"
                })),
            };

            // In a real application, you would:
            // 1. Store the interrupt state
            // 2. Present it to the user
            // 3. Get their response
            // 4. Resume with the response

            println!("Would resume with: {:#?}", resume_value);

            // Note: Full resume functionality would require extending the compiled graph
            // to handle inline interrupt resumption (future work)
        }
        Ok(final_state) => {
            println!("\nExecution completed without interrupts:");
            println!("{}", serde_json::to_string_pretty(&final_state)?);
        }
        Err(e) => {
            println!("\nExecution failed: {}", e);
        }
    }

    println!("\n=== Demo Complete ===");

    Ok(())
}