//! Example demonstrating tool execution with runtime context injection
//!
//! This example shows how to:
//! - Define tools that can be called by AI models
//! - Execute tools with runtime context injection
//! - Access store and stream writer from within tools
//! - Handle tool errors gracefully
//!
//! Run with: cargo run --example tool_execution

use langgraph_core::{
    Tool, ToolCall, ToolRegistry, ToolRuntime, ToolOutput, InMemoryStore,
};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Tool Execution Example");
    println!("======================\n");

    // Create a tool registry
    let mut registry = ToolRegistry::new();

    // Define a calculator tool that adds two numbers
    let calculator_add = Tool::new(
        "calculator_add",
        "Add two numbers together",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "First number"},
                "b": {"type": "number", "description": "Second number"}
            },
            "required": ["a", "b"]
        }),
        Arc::new(|args, runtime| {
            Box::pin(async move {
                let a = args["a"].as_i64().unwrap_or(0);
                let b = args["b"].as_i64().unwrap_or(0);
                let sum = a + b;

                // Access runtime context if available
                if let Some(rt) = runtime {
                    println!("  [calculator_add] Executing with runtime context");

                    // Store calculation history in the store
                    if let Some(store) = rt.store {
                        store.put("last_calculation", json!({
                            "operation": "add",
                            "a": a,
                            "b": b,
                            "result": sum
                        })).await.ok();
                        println!("  [calculator_add] Stored calculation history");
                    }

                    // Write custom event to stream
                    if let Some(writer) = rt.stream_writer {
                        writer.write(json!({
                            "event": "calculation",
                            "operation": "add",
                            "result": sum
                        })).ok();
                        println!("  [calculator_add] Emitted calculation event");
                    }

                    println!("  [calculator_add] Tool call ID: {:?}", rt.tool_call_id);
                }

                Ok(json!({
                    "result": sum,
                    "message": format!("{} + {} = {}", a, b, sum)
                }))
            })
        }),
    );

    // Define a tool that reads from the store
    let get_last_calculation = Tool::new(
        "get_last_calculation",
        "Get the last calculation from history",
        json!({
            "type": "object",
            "properties": {}
        }),
        Arc::new(|_args, runtime| {
            Box::pin(async move {
                if let Some(rt) = runtime {
                    if let Some(store) = rt.store {
                        match store.get("last_calculation").await {
                            Ok(Some(value)) => {
                                println!("  [get_last_calculation] Found history: {}", value);
                                return Ok(json!({
                                    "found": true,
                                    "calculation": value
                                }));
                            }
                            Ok(None) => {
                                println!("  [get_last_calculation] No history found");
                                return Ok(json!({
                                    "found": false,
                                    "message": "No calculation history"
                                }));
                            }
                            Err(e) => {
                                return Err(langgraph_core::ToolError::ExecutionFailed {
                                    tool: "get_last_calculation".to_string(),
                                    error: e.to_string(),
                                });
                            }
                        }
                    }
                }

                Ok(json!({
                    "found": false,
                    "message": "Store not available"
                }))
            })
        }),
    );

    // Define a tool that multiplies two numbers
    let calculator_multiply = Tool::new(
        "calculator_multiply",
        "Multiply two numbers together",
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"}
            }
        }),
        Arc::new(|args, _runtime| {
            Box::pin(async move {
                let a = args["a"].as_i64().unwrap_or(0);
                let b = args["b"].as_i64().unwrap_or(0);
                let product = a * b;

                Ok(json!({
                    "result": product,
                    "message": format!("{} × {} = {}", a, b, product)
                }))
            })
        }),
    );

    // Register all tools
    registry.register(calculator_add);
    registry.register(get_last_calculation);
    registry.register(calculator_multiply);

    println!("Registered tools: {:?}\n", registry.tool_names());

    // Create a store for persistent data
    let store = Arc::new(InMemoryStore::new());

    // Example 1: Execute a single tool call without runtime context
    println!("Example 1: Simple tool execution (no runtime context)");
    println!("--------------------------------------------------------");
    let tool_call1 = ToolCall {
        id: "call_1".to_string(),
        name: "calculator_multiply".to_string(),
        args: json!({"a": 6, "b": 7}),
    };

    let result1 = registry.execute_tool_call(&tool_call1, None).await;
    println!("Result: {:?}\n", result1);

    // Example 2: Execute tool with runtime context
    println!("Example 2: Tool execution with runtime context");
    println!("-----------------------------------------------");

    let state = json!({
        "messages": [],
        "counter": 0
    });

    let runtime = ToolRuntime::new(state.clone())
        .with_tool_call_id("call_2".to_string())
        .with_store(store.clone());

    let tool_call2 = ToolCall {
        id: "call_2".to_string(),
        name: "calculator_add".to_string(),
        args: json!({"a": 15, "b": 27}),
    };

    let result2 = registry.execute_tool_call(&tool_call2, Some(runtime)).await;
    println!("Result: {:?}\n", result2);

    // Example 3: Read from store
    println!("Example 3: Reading calculation history from store");
    println!("--------------------------------------------------");

    let runtime3 = ToolRuntime::new(state.clone())
        .with_tool_call_id("call_3".to_string())
        .with_store(store.clone());

    let tool_call3 = ToolCall {
        id: "call_3".to_string(),
        name: "get_last_calculation".to_string(),
        args: json!({}),
    };

    let result3 = registry.execute_tool_call(&tool_call3, Some(runtime3)).await;
    println!("Result: {:?}\n", result3);

    // Example 4: Execute multiple tools in parallel
    println!("Example 4: Parallel tool execution");
    println!("-----------------------------------");

    let tool_calls = vec![
        ToolCall {
            id: "call_4a".to_string(),
            name: "calculator_add".to_string(),
            args: json!({"a": 100, "b": 200}),
        },
        ToolCall {
            id: "call_4b".to_string(),
            name: "calculator_multiply".to_string(),
            args: json!({"a": 10, "b": 20}),
        },
        ToolCall {
            id: "call_4c".to_string(),
            name: "calculator_add".to_string(),
            args: json!({"a": 5, "b": 10}),
        },
    ];

    let runtime4 = ToolRuntime::new(state.clone())
        .with_store(store.clone());

    let results = registry.execute_tool_calls(&tool_calls, Some(runtime4)).await;

    println!("Executed {} tools in parallel:", results.len());
    for result in &results {
        println!("  - {}: {:?}", result.name, result.output);
    }
    println!();

    // Example 5: Handle tool not found error
    println!("Example 5: Tool not found error");
    println!("--------------------------------");

    let tool_call5 = ToolCall {
        id: "call_5".to_string(),
        name: "nonexistent_tool".to_string(),
        args: json!({}),
    };

    let result5 = registry.execute_tool_call(&tool_call5, None).await;
    match result5.output {
        ToolOutput::Error { error } => {
            println!("Expected error: {}\n", error);
        }
        ToolOutput::Success { .. } => {
            println!("Unexpected success!\n");
        }
    }

    println!("All examples completed!");
    Ok(())
}
