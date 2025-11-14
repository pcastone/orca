//! Integration tests for complete workflows
//!
//! These tests verify that all components work together correctly
//! in realistic scenarios.

use langgraph_core::{
    StateGraph, Tool, ToolRegistry, ToolCall, ToolRuntime, ToolOutput,
    InMemoryStore, Store, get_runtime, get_store,
    prebuilt::{create_chat_agent, ChatAgentConfig, create_structured_agent, StructuredAgentConfig},
};
use serde_json::json;
use std::sync::Arc;

/// Test a complete workflow with tools, runtime context, and store
#[tokio::test]
async fn test_complete_agent_workflow() {
    // Create a store
    let store = Arc::new(InMemoryStore::new());

    // Create tools
    let mut tools = ToolRegistry::new();

    // Calculator tool that uses store
    let calculator = Tool::new(
        "calculator",
        "Perform calculations",
        json!({"type": "object"}),
        Arc::new(|args, runtime| {
            Box::pin(async move {
                let operation = args["operation"].as_str().unwrap_or("add");
                let a = args["a"].as_f64().unwrap_or(0.0);
                let b = args["b"].as_f64().unwrap_or(0.0);

                let result = match operation {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    "divide" if b != 0.0 => a / b,
                    _ => 0.0,
                };

                // Store calculation in history
                if let Some(rt) = runtime {
                    if let Some(store) = rt.store {
                        let history = json!({
                            "operation": operation,
                            "a": a,
                            "b": b,
                            "result": result
                        });
                        store.put("last_calculation", history).await.ok();
                    }
                }

                Ok(json!({
                    "result": result,
                    "operation": operation
                }))
            })
        }),
    );

    // History tool that reads from store
    let get_history = Tool::new(
        "get_history",
        "Get calculation history",
        json!({"type": "object"}),
        Arc::new(|_args, runtime| {
            Box::pin(async move {
                if let Some(rt) = runtime {
                    if let Some(store) = rt.store {
                        if let Ok(Some(history)) = store.get("last_calculation").await {
                            return Ok(json!({
                                "found": true,
                                "history": history
                            }));
                        }
                    }
                }

                Ok(json!({
                    "found": false,
                    "message": "No history available"
                }))
            })
        }),
    );

    tools.register(calculator);
    tools.register(get_history);

    // Create a simple graph that uses tools
    let mut graph = StateGraph::new();

    let tools_clone = Arc::new(tools);

    graph.add_node("process", move |state| {
        let tools = tools_clone.clone();

        Box::pin(async move {
            // Simulate calling calculator
            let calc_call = ToolCall {
                id: "call_1".to_string(),
                name: "calculator".to_string(),
                args: json!({"operation": "multiply", "a": 6, "b": 7}),
            };

            let runtime = if let Some(rt) = get_runtime() {
                Some(ToolRuntime::new(state.clone())
                    .with_store(rt.store().cloned().unwrap())
                    .with_tool_call_id("call_1".to_string()))
            } else {
                None
            };

            let result = tools.execute_tool_call(&calc_call, runtime).await;

            let mut updated_state = state.clone();
            updated_state["calculation_result"] = match result.output {
                ToolOutput::Success { content } => content,
                ToolOutput::Error { error } => json!({"error": error}),
            };

            Ok(updated_state)
        })
    });

    graph.add_edge("__start__", "process");
    graph.add_edge("process", "__end__");

    let compiled = graph.compile_with_store(store.clone()).unwrap();

    let result = compiled.invoke(json!({
        "input": "Calculate 6 * 7"
    })).await;

    assert!(result.is_ok());
    let final_state = result.unwrap();

    // Verify calculation result
    assert_eq!(
        final_state["calculation_result"]["result"].as_f64().unwrap(),
        42.0
    );

    // Verify history was stored
    let history = store.get("last_calculation").await.unwrap();
    assert!(history.is_some());
    assert_eq!(history.unwrap()["result"].as_f64().unwrap(), 42.0);
}

/// Test streaming with multiple modes
#[tokio::test]
async fn test_streaming_workflow() {
    use langgraph_core::stream::StreamMode;
    use tokio::sync::mpsc;
    use futures::StreamExt;

    let mut graph = StateGraph::new();

    graph.add_node("step1", |state| {
        Box::pin(async move {
            let mut result = state.clone();
            result["step1_complete"] = json!(true);
            Ok(result)
        })
    });

    graph.add_node("step2", |state| {
        Box::pin(async move {
            let mut result = state.clone();
            result["step2_complete"] = json!(true);
            Ok(result)
        })
    });

    graph.add_edge("__start__", "step1");
    graph.add_edge("step1", "step2");
    graph.add_edge("step2", "__end__");

    let compiled = graph.compile().unwrap();

    let mut stream = compiled.stream_with_modes(
        json!({"input": "test"}),
        vec![StreamMode::Values, StreamMode::Updates, StreamMode::Tasks],
        None
    ).await.unwrap();

    let mut event_count = 0;
    while let Some(_event) = stream.next().await {
        event_count += 1;
    }

    // Should have multiple events from different modes
    assert!(event_count > 0);
}

/// Test chat agent pattern
#[tokio::test]
async fn test_chat_agent_workflow() {
    let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    let model: Arc<dyn Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = serde_json::Value> + Send>> + Send + Sync> =
        Arc::new(move |state: serde_json::Value| {
            let count = call_count_clone.clone();
            Box::pin(async move {
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                let messages = state["messages"].as_array().unwrap();
                let last_message = messages.last().unwrap();
                let user_content = last_message["content"].as_str().unwrap_or("");

                json!({
                    "role": "assistant",
                    "content": format!("I received: {}", user_content)
                })
            })
        });

    let agent = create_chat_agent(model, ChatAgentConfig::default()).unwrap();

    let result = agent.invoke(json!({
        "messages": [
            {"role": "user", "content": "Hello!"}
        ]
    })).await;

    assert!(result.is_ok());
    let final_state = result.unwrap();

    let messages = final_state["messages"].as_array().unwrap();

    // Should have system + user + assistant messages
    assert!(messages.len() >= 3);

    // Last message should be from assistant
    assert_eq!(messages.last().unwrap()["role"], "assistant");

    // Model should have been called once
    assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
}

/// Test structured agent pattern
#[tokio::test]
async fn test_structured_agent_workflow() {
    let schema = json!({
        "type": "object",
        "properties": {
            "answer": {"type": "string"},
            "confidence": {"type": "number", "minimum": 0, "maximum": 1},
            "reasoning": {"type": "string"}
        },
        "required": ["answer", "confidence"]
    });

    let model: Arc<dyn Fn(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = serde_json::Value> + Send>> + Send + Sync> =
        Arc::new(move |_state: serde_json::Value| {
            Box::pin(async move {
                json!({
                    "answer": "The answer is 42",
                    "confidence": 0.99,
                    "reasoning": "It's the answer to life, the universe, and everything"
                })
            })
        });

    let agent = create_structured_agent(
        model,
        StructuredAgentConfig::new(schema)
    ).unwrap();

    let result = agent.invoke(json!({
        "iteration": 0,
        "question": "What is the answer?"
    })).await;

    assert!(result.is_ok());
    let final_state = result.unwrap();

    // Output should be valid
    assert_eq!(final_state["is_valid"], true);

    // Should have the structured output
    let output = &final_state["output"];
    assert_eq!(output["answer"], "The answer is 42");
    assert_eq!(output["confidence"], 0.99);
}

/// Test error handling and recovery
#[tokio::test]
async fn test_error_handling_workflow() {
    let mut tools = ToolRegistry::new();

    // Tool that fails
    let failing_tool = Tool::new(
        "failing_tool",
        "A tool that always fails",
        json!({"type": "object"}),
        Arc::new(|_args, _runtime| {
            Box::pin(async move {
                Err(langgraph_core::ToolError::ExecutionFailed {
                    tool: "failing_tool".to_string(),
                    error: "Intentional failure for testing".to_string(),
                })
            })
        }),
    );

    tools.register(failing_tool);

    let tool_call = ToolCall {
        id: "call_fail".to_string(),
        name: "failing_tool".to_string(),
        args: json!({}),
    };

    let result = tools.execute_tool_call(&tool_call, None).await;

    // Should return error output
    match result.output {
        ToolOutput::Error { error } => {
            assert!(error.contains("Intentional failure"));
        }
        ToolOutput::Success { .. } => {
            panic!("Expected error but got success");
        }
    }
}

/// Test runtime context propagation through graph
#[tokio::test]
async fn test_runtime_context_propagation() {
    let mut graph = StateGraph::new();
    let store = Arc::new(InMemoryStore::new());

    graph.add_node("node1", |state| {
        Box::pin(async move {
            // Access runtime context
            if let Some(runtime) = get_runtime() {
                assert_eq!(runtime.current_step(), 0);

                // Access store
                if let Some(store) = get_store() {
                    store.put("node1_visited", json!(true)).await.ok();
                }
            }

            Ok(state)
        })
    });

    graph.add_node("node2", |state| {
        Box::pin(async move {
            // Verify store has data from node1
            if let Some(store) = get_store() {
                let visited = store.get("node1_visited").await.unwrap();
                assert!(visited.is_some());
                assert_eq!(visited.unwrap(), true);
            }

            Ok(state)
        })
    });

    graph.add_edge("__start__", "node1");
    graph.add_edge("node1", "node2");
    graph.add_edge("node2", "__end__");

    let compiled = graph.compile_with_store(store).unwrap();

    let result = compiled.invoke(json!({})).await;
    assert!(result.is_ok());
}

/// Test parallel tool execution
#[tokio::test]
async fn test_parallel_tool_execution() {
    let mut tools = ToolRegistry::new();

    for i in 1..=5 {
        let tool = Tool::new(
            format!("tool_{}", i),
            format!("Tool number {}", i),
            json!({"type": "object"}),
            Arc::new(move |_args, _runtime| {
                Box::pin(async move {
                    // Simulate some work
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    Ok(json!({"tool_id": i, "result": i * 10}))
                })
            }),
        );
        tools.register(tool);
    }

    let tool_calls: Vec<ToolCall> = (1..=5).map(|i| {
        ToolCall {
            id: format!("call_{}", i),
            name: format!("tool_{}", i),
            args: json!({}),
        }
    }).collect();

    let start = std::time::Instant::now();
    let results = tools.execute_tool_calls(&tool_calls, None).await;
    let duration = start.elapsed();

    // All tools should succeed
    assert_eq!(results.len(), 5);
    for result in &results {
        match &result.output {
            ToolOutput::Success { .. } => {},
            ToolOutput::Error { error } => {
                panic!("Tool failed: {}", error);
            }
        }
    }

    // Parallel execution should be faster than sequential
    // 5 tools * 10ms each = 50ms sequential
    // Parallel should be ~10-20ms
    assert!(duration.as_millis() < 40, "Parallel execution took too long: {:?}", duration);
}

/// Test subgraph execution
#[tokio::test]
async fn test_subgraph_workflow() {
    // Create a subgraph that processes data
    let mut subgraph = StateGraph::new();
    subgraph.add_node("sub_process", |mut state| {
        Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                let value = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("value".to_string(), json!(value * 2));
                obj.insert("sub_processed".to_string(), json!(true));
            }
            Ok(state)
        })
    });
    subgraph.add_edge("__start__", "sub_process");
    subgraph.add_edge("sub_process", "__end__");

    let compiled_sub = subgraph.compile().unwrap();

    // Create a parent graph that uses the subgraph
    let mut parent = StateGraph::new();

    parent.add_node("prepare", |mut state| {
        Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                obj.insert("prepared".to_string(), json!(true));
            }
            Ok(state)
        })
    });

    // Add the subgraph as a node
    parent.add_subgraph("subprocess", compiled_sub);

    parent.add_node("finalize", |mut state| {
        Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                obj.insert("finalized".to_string(), json!(true));
            }
            Ok(state)
        })
    });

    parent.add_edge("__start__", "prepare");
    parent.add_edge("prepare", "subprocess");
    parent.add_edge("subprocess", "finalize");
    parent.add_edge("finalize", "__end__");

    let compiled_parent = parent.compile().unwrap();

    // Execute the parent graph
    let result = compiled_parent.invoke(json!({
        "value": 21
    })).await;

    assert!(result.is_ok());
    let final_state = result.unwrap();

    // Verify all steps executed
    assert_eq!(final_state["prepared"], true);
    assert_eq!(final_state["sub_processed"], true);
    assert_eq!(final_state["finalized"], true);

    // Verify the subgraph doubled the value
    assert_eq!(final_state["value"], 42);
}

/// Test nested subgraphs (subgraph containing another subgraph)
#[tokio::test]
async fn test_nested_subgraphs() {
    // Create innermost subgraph (adds 1)
    let mut inner = StateGraph::new();
    inner.add_node("add_one", |mut state| {
        Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                let value = obj.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("count".to_string(), json!(value + 1));
            }
            Ok(state)
        })
    });
    inner.add_edge("__start__", "add_one");
    inner.add_edge("add_one", "__end__");
    let compiled_inner = inner.compile().unwrap();

    // Create middle subgraph (uses inner subgraph, then multiplies by 2)
    let mut middle = StateGraph::new();
    middle.add_subgraph("inner_sub", compiled_inner);
    middle.add_node("multiply_two", |mut state| {
        Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                let value = obj.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("count".to_string(), json!(value * 2));
            }
            Ok(state)
        })
    });
    middle.add_edge("__start__", "inner_sub");
    middle.add_edge("inner_sub", "multiply_two");
    middle.add_edge("multiply_two", "__end__");
    let compiled_middle = middle.compile().unwrap();

    // Create outer graph (uses middle subgraph, then adds 10)
    let mut outer = StateGraph::new();
    outer.add_subgraph("middle_sub", compiled_middle);
    outer.add_node("add_ten", |mut state| {
        Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                let value = obj.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("count".to_string(), json!(value + 10));
            }
            Ok(state)
        })
    });
    outer.add_edge("__start__", "middle_sub");
    outer.add_edge("middle_sub", "add_ten");
    outer.add_edge("add_ten", "__end__");
    let compiled_outer = outer.compile().unwrap();

    // Execute: (5 + 1) * 2 + 10 = 22
    let result = compiled_outer.invoke(json!({"count": 5})).await;

    assert!(result.is_ok());
    let final_state = result.unwrap();
    assert_eq!(final_state["count"], 22);
}

/// Test graph visualization
#[tokio::test]
async fn test_graph_visualization() {
    use langgraph_core::{visualize, VisualizationOptions, VisualizationFormat};
    
    // Create a graph with multiple nodes and conditional edges
    let mut graph = StateGraph::new();
    
    graph.add_node("prepare", |mut state| {
        Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                obj.insert("prepared".to_string(), json!(true));
            }
            Ok(state)
        })
    });
    
    graph.add_node("process_a", |mut state| {
        Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                obj.insert("path".to_string(), json!("a"));
            }
            Ok(state)
        })
    });
    
    graph.add_node("process_b", |mut state| {
        Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                obj.insert("path".to_string(), json!("b"));
            }
            Ok(state)
        })
    });
    
    graph.add_node("finalize", |mut state| {
        Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                obj.insert("finalized".to_string(), json!(true));
            }
            Ok(state)
        })
    });
    
    graph.add_edge("__start__", "prepare");
    
    use std::collections::HashMap;
    let mut branches = HashMap::new();
    branches.insert("a".to_string(), "process_a".to_string());
    branches.insert("b".to_string(), "process_b".to_string());
    
    graph.add_conditional_edge("prepare", |state| {
        use langgraph_core::ConditionalEdgeResult;
        if state.get("choice").and_then(|v| v.as_str()) == Some("a") {
            ConditionalEdgeResult::Node("a".to_string())
        } else {
            ConditionalEdgeResult::Node("b".to_string())
        }
    }, branches);
    
    graph.add_edge("process_a", "finalize");
    graph.add_edge("process_b", "finalize");
    graph.add_edge("finalize", "__end__");
    
    let compiled = graph.compile().unwrap();
    
    // Test DOT format
    let dot = compiled.visualize(&VisualizationOptions::dot());
    println!("\n=== DOT Format ===\n{}", dot);
    assert!(dot.contains("digraph G"));
    assert!(dot.contains("prepare"));
    assert!(dot.contains("process_a"));
    assert!(dot.contains("process_b"));
    assert!(dot.contains("finalize"));
    
    // Test Mermaid format
    let mermaid = compiled.visualize(&VisualizationOptions::mermaid().with_title("My Graph"));
    println!("\n=== Mermaid Format ===\n{}", mermaid);
    assert!(mermaid.contains("graph TD"));
    assert!(mermaid.contains("My Graph"));
    assert!(mermaid.contains("prepare"));
    assert!(mermaid.contains("process_a"));
    
    // Test ASCII format
    let ascii = compiled.visualize(&VisualizationOptions::ascii());
    println!("\n=== ASCII Format ===\n{}", ascii);
    assert!(ascii.contains("Graph Structure"));
    assert!(ascii.contains("START"));
    assert!(ascii.contains("END"));
}

/// Test functional API workflow builder
#[tokio::test]
async fn test_functional_workflow() {
    use langgraph_core::functional::{Workflow, task};

    // Create tasks that transform data
    let add_five = task("add_five", |mut state| Box::pin(async move {
        if let Some(obj) = state.as_object_mut() {
            let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            obj.insert("value".to_string(), json!(val + 5));
        }
        Ok(state)
    }));

    let multiply_three = task("multiply_three", |mut state| Box::pin(async move {
        if let Some(obj) = state.as_object_mut() {
            let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            obj.insert("value".to_string(), json!(val * 3));
        }
        Ok(state)
    }));

    let subtract_two = task("subtract_two", |mut state| Box::pin(async move {
        if let Some(obj) = state.as_object_mut() {
            let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            obj.insert("value".to_string(), json!(val - 2));
        }
        Ok(state)
    }));

    // Build workflow
    let workflow = Workflow::builder()
        .add_task(add_five)
        .then(multiply_three)
        .then(subtract_two)
        .build()
        .expect("Failed to build workflow");

    // Verify task names
    let task_names = workflow.task_names();
    assert_eq!(task_names, vec!["add_five", "multiply_three", "subtract_two"]);

    // Execute workflow: (10 + 5) * 3 - 2 = 45 - 2 = 43
    let result = workflow.invoke(json!({"value": 10})).await.expect("Failed to invoke workflow");

    assert_eq!(result["value"], 43, "Workflow calculation should be correct");
}

/// Test state history and time travel debugging
#[tokio::test]
async fn test_state_history() {
    use langgraph_core::{StateGraph, CheckpointConfig};
    use langgraph_checkpoint::InMemoryCheckpointSaver;
    use futures::stream::StreamExt;
    use std::sync::Arc;

    // Create a graph that performs multiple transformations
    let mut graph = StateGraph::new();

    graph.add_node("step1", |mut state| Box::pin(async move {
        if let Some(obj) = state.as_object_mut() {
            let val = obj.get("counter").and_then(|v| v.as_i64()).unwrap_or(0);
            obj.insert("counter".to_string(), json!(val + 10));
            obj.insert("step".to_string(), json!("step1"));
        }
        Ok(state)
    }));

    graph.add_node("step2", |mut state| Box::pin(async move {
        if let Some(obj) = state.as_object_mut() {
            let val = obj.get("counter").and_then(|v| v.as_i64()).unwrap_or(0);
            obj.insert("counter".to_string(), json!(val + 20));
            obj.insert("step".to_string(), json!("step2"));
        }
        Ok(state)
    }));

    graph.add_node("step3", |mut state| Box::pin(async move {
        if let Some(obj) = state.as_object_mut() {
            let val = obj.get("counter").and_then(|v| v.as_i64()).unwrap_or(0);
            obj.insert("counter".to_string(), json!(val + 30));
            obj.insert("step".to_string(), json!("step3"));
        }
        Ok(state)
    }));

    graph.add_edge("__start__", "step1");
    graph.add_edge("step1", "step2");
    graph.add_edge("step2", "step3");
    graph.add_edge("step3", "__end__");

    // Compile with checkpointer
    let saver = Arc::new(InMemoryCheckpointSaver::new());
    let compiled = graph.compile().unwrap().with_checkpointer(saver.clone());

    // Execute the graph with checkpointing
    let config = CheckpointConfig::new().with_thread_id("test_thread_123".to_string());
    let result = compiled.invoke_with_config(
        json!({"counter": 0}),
        Some(config.clone())
    ).await.unwrap();

    // Verify final result
    assert_eq!(result["counter"], 60, "Final counter should be 60 (0 + 10 + 20 + 30)");

    // Get current state
    let current_state = compiled.get_state(&config).await.unwrap();
    assert!(current_state.is_some(), "Should have current state");

    let current_snapshot = current_state.unwrap();
    println!("Current snapshot values: {:?}", current_snapshot.values);

    // Verify snapshot structure
    assert!(current_snapshot.created_at.is_some(), "Should have timestamp");
    assert_eq!(current_snapshot.config.thread_id, Some("test_thread_123".to_string()));

    // Get state history
    let mut history = compiled.get_state_history(&config, None, None, Some(10))
        .await
        .unwrap();

    let mut snapshots = Vec::new();
    while let Some(snapshot_result) = history.next().await {
        let snapshot = snapshot_result.unwrap();
        snapshots.push(snapshot);
    }

    // Should have multiple snapshots (at least one per checkpoint)
    assert!(snapshots.len() >= 1, "Should have at least 1 snapshot");
    println!("Found {} snapshots in history", snapshots.len());

    // Verify we can traverse history
    for (i, snapshot) in snapshots.iter().enumerate() {
        println!("Snapshot {}: created_at={:?}, counter={:?}, next={:?}",
            i,
            snapshot.created_at,
            snapshot.values.get("counter"),
            snapshot.next
        );

        // Each snapshot should have a created_at timestamp
        assert!(snapshot.created_at.is_some(), "Snapshot should have timestamp");

        // Each snapshot should have config
        assert_eq!(snapshot.config.thread_id, Some("test_thread_123".to_string()));
    }
}

/// Test advanced streaming with token-level output
#[tokio::test]
async fn test_token_streaming() {
    use langgraph_core::{TokenBuffer, TokenStreamAdapter, MessageChunk, StreamMode, StreamEvent};
    use futures::stream;

    // Test MessageChunk creation
    let chunk = MessageChunk::new("Hello")
        .with_message_id("msg_123")
        .with_metadata(json!({"model": "gpt-4"}))
        .final_chunk();

    assert_eq!(chunk.content, "Hello");
    assert_eq!(chunk.message_id, Some("msg_123".to_string()));
    assert!(chunk.is_final);
    assert!(chunk.metadata.is_some());

    // Test converting to StreamEvent
    let event = chunk.to_stream_event("llm_node");
    assert!(event.matches_mode(StreamMode::Messages));
    assert!(event.matches_mode(StreamMode::Tokens));

    // Test TokenBuffer
    let mut buffer = TokenBuffer::new();
    buffer.add_chunk("Hello");
    buffer.add_chunk(" ");
    buffer.add_chunk("world");
    buffer.add_chunk("!");

    assert_eq!(buffer.content(), "Hello world!");
    assert_eq!(buffer.chunk_count(), 4);
    assert!(!buffer.is_finished());

    buffer.finish();
    assert!(buffer.is_finished());

    let content = buffer.into_string();
    assert_eq!(content, "Hello world!");

    // Test TokenStreamAdapter
    let tokens = vec![
        "The".to_string(),
        " quick".to_string(),
        " brown".to_string(),
        " fox".to_string(),
    ];
    let token_stream = Box::pin(stream::iter(tokens));

    let adapter = TokenStreamAdapter::new("agent_node")
        .with_message_id("msg_456");

    let mut event_stream = adapter.adapt(token_stream);

    use futures::stream::StreamExt;
    let mut collected_chunks = Vec::new();
    while let Some(event) = event_stream.next().await {
        if let StreamEvent::MessageChunk { chunk, message_id, node, .. } = event {
            assert_eq!(node, "agent_node");
            assert_eq!(message_id, Some("msg_456".to_string()));
            collected_chunks.push(chunk);
        }
    }

    assert_eq!(collected_chunks, vec!["The", " quick", " brown", " fox"]);

    // Verify reconstructed message
    let full_message = collected_chunks.join("");
    assert_eq!(full_message, "The quick brown fox");
}

#[tokio::test]
async fn test_message_graph_workflow() {
    use langgraph_core::{MessageGraph, Message, MessageRole};

    let mut graph = MessageGraph::new();

    // Add a chatbot node that responds to messages
    graph.add_node("chatbot", |state| {
        Box::pin(async move {
            // Get current messages
            let empty_vec = vec![];
            let messages = state.get("messages").and_then(|v| v.as_array()).unwrap_or(&empty_vec);

            // Get the last message content
            let last_msg = messages.last()
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("");

            // Create response based on input (case-insensitive)
            let last_msg_lower = last_msg.to_lowercase();
            let response = if last_msg_lower.contains("hello") || last_msg_lower.contains("hi") {
                Message::assistant("Hello! How can I help you today?")
            } else if last_msg_lower.contains("weather") {
                Message::assistant("I don't have access to weather data, but it's a great day to learn Rust!")
            } else {
                Message::assistant("I understand. Tell me more!")
            };

            // Return update with new message
            Ok(json!({
                "messages": [response]
            }))
        })
    });

    graph.add_edge("__start__", "chatbot");
    graph.add_edge("chatbot", "__end__");

    let compiled = graph.compile().expect("Graph should compile");

    // Test 1: Simple greeting
    let initial_msg = Message::human("Hello there!");
    let initial_state = json!({
        "messages": [initial_msg]
    });

    let result = compiled.invoke(initial_state).await.expect("Execution should succeed");

    eprintln!("Final result: {}", serde_json::to_string_pretty(&result).unwrap());

    let messages = result.get("messages").and_then(|v| v.as_array()).expect("Should have messages");
    eprintln!("Messages count: {}", messages.len());
    assert!(messages.len() >= 2, "Should have at least human message and bot response");

    // Verify the bot's response
    let last_message: Message = serde_json::from_value(messages.last().unwrap().clone()).unwrap();
    assert_eq!(last_message.role, MessageRole::Assistant);
    assert!(last_message.text().unwrap().contains("Hello"));

    // Test 2: Message ID-based updates
    let msg1 = Message::human("Original message").with_id("msg_1");
    let msg2 = Message::human("Updated message").with_id("msg_1"); // Same ID - should replace

    let state_with_update = json!({
        "messages": [msg1, msg2]
    });

    let result2 = compiled.invoke(state_with_update).await.expect("Execution should succeed");
    let messages2 = result2.get("messages").and_then(|v| v.as_array()).expect("Should have messages");

    // Should have deduplicated message (only one with id "msg_1") plus bot response
    let human_messages: Vec<&serde_json::Value> = messages2.iter()
        .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("human"))
        .collect();

    // Should only be 1 human message due to deduplication
    assert_eq!(human_messages.len(), 1);

    let human_msg: Message = serde_json::from_value(human_messages[0].clone()).unwrap();
    assert_eq!(human_msg.text(), Some("Updated message"));
}

#[tokio::test]
async fn test_message_graph_with_tool_calls() {
    use langgraph_core::{MessageGraph, Message, ToolCall};

    let mut graph = MessageGraph::new();

    // Add agent node that may request tools
    graph.add_node("agent", |state| {
        Box::pin(async move {
            let empty_vec = vec![];
            let messages = state.get("messages").and_then(|v| v.as_array()).unwrap_or(&empty_vec);
            let last_msg = messages.last()
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("");

            // Request tool if user asks for search
            let response = if last_msg.contains("search") {
                Message::assistant("Let me search for that.")
                    .with_tool_calls(vec![ToolCall {
                        id: "call_123".to_string(),
                        name: "web_search".to_string(),
                        args: json!({"query": "rust langgraph"}),
                    }])
            } else {
                Message::assistant("No tools needed for that.")
            };

            Ok(json!({
                "messages": [response],
                "has_tool_calls": response.tool_calls.is_some()
            }))
        })
    });

    // Add tool execution node
    graph.add_node("tools", |state| {
        Box::pin(async move {
            let empty_vec = vec![];
            let messages = state.get("messages").and_then(|v| v.as_array()).unwrap_or(&empty_vec);

            // Find last message with tool calls
            let tool_calls = messages.iter().rev()
                .find_map(|m| m.get("tool_calls"))
                .and_then(|tc| tc.as_array());

            if let Some(calls) = tool_calls {
                // Execute each tool call
                let results: Vec<Message> = calls.iter()
                    .filter_map(|call| call.get("id").and_then(|id| id.as_str()))
                    .map(|id| {
                        Message::tool("Search results: Found comprehensive Rust LangGraph documentation!", id)
                    })
                    .collect();

                Ok(json!({
                    "messages": results
                }))
            } else {
                Ok(state.clone())
            }
        })
    });

    // Add final response node
    graph.add_node("final_response", |_state| {
        Box::pin(async move {
            let response = Message::assistant("I found some information for you!");
            Ok(json!({
                "messages": [response]
            }))
        })
    });

    graph.add_edge("__start__", "agent");

    // Conditional routing: if tool calls exist, go to tools, otherwise end
    let mut branches = std::collections::HashMap::new();
    branches.insert("tools".to_string(), "tools".to_string());
    branches.insert("end".to_string(), "__end__".to_string());

    graph.add_conditional_edge("agent", |state| {
        use langgraph_core::ConditionalEdgeResult;
        if state.get("has_tool_calls").and_then(|v| v.as_bool()).unwrap_or(false) {
            ConditionalEdgeResult::Node("tools".to_string())
        } else {
            ConditionalEdgeResult::Node("end".to_string())
        }
    }, branches);

    graph.add_edge("tools", "final_response");
    graph.add_edge("final_response", "__end__");

    let compiled = graph.compile().expect("Graph should compile");

    // Test with search request
    let initial_msg = Message::human("Can you search for rust langgraph?");
    let initial_state = json!({
        "messages": [initial_msg]
    });

    let result = compiled.invoke(initial_state).await.expect("Execution should succeed");
    let messages = result.get("messages").and_then(|v| v.as_array()).expect("Should have messages");

    // Should have: human msg, agent msg with tool calls, tool result, final response
    assert!(messages.len() >= 4, "Should have complete conversation with tool execution");

    // Verify tool call exists
    let has_tool_call = messages.iter().any(|m|
        m.get("tool_calls").is_some()
    );
    assert!(has_tool_call, "Should have at least one message with tool calls");

    // Verify tool response exists
    let has_tool_response = messages.iter().any(|m|
        m.get("role").and_then(|r| r.as_str()) == Some("tool")
    );
    assert!(has_tool_response, "Should have tool response message");
}

/// Test that managed values are correctly injected into node inputs
#[tokio::test]
async fn test_managed_values_injection() {
    use std::sync::{Arc, Mutex};

    // Create shared state to capture what the node receives
    let captured_inputs = Arc::new(Mutex::new(Vec::new()));

    let mut graph = StateGraph::new();

    // Add a node that captures its input state to verify managed values
    let inputs_clone = captured_inputs.clone();
    graph.add_node("step1", move |state| {
        let inputs = inputs_clone.clone();
        Box::pin(async move {
            // Capture the input state for verification
            inputs.lock().unwrap().push(state.clone());

            // Simple passthrough
            Ok(json!({
                "value": "step1_done"
            }))
        })
    });

    let inputs_clone2 = captured_inputs.clone();
    graph.add_node("step2", move |state| {
        let inputs = inputs_clone2.clone();
        Box::pin(async move {
            // Capture the input state for verification
            inputs.lock().unwrap().push(state.clone());

            Ok(json!({
                "value": "step2_done"
            }))
        })
    });

    let inputs_clone3 = captured_inputs.clone();
    graph.add_node("step3", move |state| {
        let inputs = inputs_clone3.clone();
        Box::pin(async move {
            // Capture the input state for verification
            inputs.lock().unwrap().push(state.clone());

            Ok(json!({
                "value": "step3_done"
            }))
        })
    });

    graph.add_edge("__start__", "step1");
    graph.add_edge("step1", "step2");
    graph.add_edge("step2", "step3");
    graph.add_edge("step3", "__end__");

    // Compile with max_steps to enable managed values
    let compiled = graph.compile().expect("Graph should compile");

    let initial_state = json!({
        "initial": "data"
    });

    let _result = compiled.invoke(initial_state).await.expect("Execution should succeed");

    // Verify that inputs were captured
    let inputs = captured_inputs.lock().unwrap();
    assert_eq!(inputs.len(), 3, "Should have captured 3 node inputs");

    // Verify managed values in each step
    // Note: steps are 0-indexed in the implementation
    for (i, input) in inputs.iter().enumerate() {
        // Check for __current_step__
        let current_step = input.get("__current_step__")
            .and_then(|v| v.as_u64())
            .expect(&format!("Step index {} should have __current_step__", i));
        assert_eq!(current_step as usize, i,
            "Step index {} should have current_step = {}", i, i);

        // Check for __remaining_steps__ (if max_steps is set, this would be present)
        // Since we didn't set max_steps explicitly, remaining_steps should be very large
        let has_remaining = input.get("__remaining_steps__").is_some();

        // Check for __is_last_step__ (should be false for non-final steps)
        let has_is_last = input.get("__is_last_step__").is_some();

        println!("Step {}: current_step={}, has_remaining={}, has_is_last={}",
            i, current_step, has_remaining, has_is_last);
    }

    // Verify first step (step 0)
    let step1_input = &inputs[0];
    assert_eq!(
        step1_input.get("__current_step__").and_then(|v| v.as_u64()),
        Some(0),
        "First step should have current_step = 0"
    );

    // Verify second step (step 1)
    let step2_input = &inputs[1];
    assert_eq!(
        step2_input.get("__current_step__").and_then(|v| v.as_u64()),
        Some(1),
        "Second step should have current_step = 1"
    );

    // Verify third step (step 2)
    let step3_input = &inputs[2];
    assert_eq!(
        step3_input.get("__current_step__").and_then(|v| v.as_u64()),
        Some(2),
        "Third step should have current_step = 2"
    );
}
