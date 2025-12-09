//! Test StateGraph basic routing to verify graph execution order

use langgraph_core::StateGraph;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_state_graph_basic_routing() {
    // Track execution order
    let execution_order = Arc::new(Mutex::new(Vec::new()));

    let mut graph = StateGraph::new();

    // Add node A
    let order_clone = execution_order.clone();
    graph.add_node("node_a", move |state: serde_json::Value| {
        let order = order_clone.clone();
        Box::pin(async move {
            println!("Executing node_a");
            order.lock().unwrap().push("node_a");
            Ok(state)
        })
    });

    // Add node B
    let order_clone = execution_order.clone();
    graph.add_node("node_b", move |state: serde_json::Value| {
        let order = order_clone.clone();
        Box::pin(async move {
            println!("Executing node_b");
            order.lock().unwrap().push("node_b");
            Ok(state)
        })
    });

    // Route: START -> A -> B
    graph.add_edge("__start__", "node_a");
    graph.add_edge("node_a", "node_b");

    let compiled = graph.compile().expect("Failed to compile graph");

    let initial_state = json!({"test": "value"});
    let _result = compiled.invoke(initial_state).await.expect("Failed to invoke graph");

    let order = execution_order.lock().unwrap();
    println!("Execution order: {:?}", *order);

    assert_eq!(
        *order,
        vec!["node_a", "node_b"],
        "Nodes should execute in order: node_a then node_b"
    );
}

#[tokio::test]
async fn test_start_edge_routes_to_first_node() {
    let executed = Arc::new(Mutex::new(false));
    let mut graph = StateGraph::new();

    let exec_clone = executed.clone();
    graph.add_node("first_node", move |state: serde_json::Value| {
        let exec = exec_clone.clone();
        Box::pin(async move {
            println!("first_node executed");
            *exec.lock().unwrap() = true;
            Ok(state)
        })
    });

    graph.add_edge("__start__", "first_node");

    let compiled = graph.compile().expect("Failed to compile graph");
    let _result = compiled.invoke(json!({})).await.expect("Failed to invoke graph");

    assert!(
        *executed.lock().unwrap(),
        "First node should have been executed via __start__ edge"
    );
}

#[tokio::test]
async fn test_graph_does_not_skip_nodes() {
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let mut graph = StateGraph::new();

    // Create a chain: START -> A -> B -> C
    for node_name in &["node_a", "node_b", "node_c"] {
        let order_clone = execution_order.clone();
        let name = node_name.to_string();
        graph.add_node(*node_name, move |state: serde_json::Value| {
            let order = order_clone.clone();
            let n = name.clone();
            Box::pin(async move {
                println!("Executing {}", n);
                order.lock().unwrap().push(n);
                Ok(state)
            })
        });
    }

    graph.add_edge("__start__", "node_a");
    graph.add_edge("node_a", "node_b");
    graph.add_edge("node_b", "node_c");

    let compiled = graph.compile().expect("Failed to compile graph");
    let _result = compiled.invoke(json!({})).await.expect("Failed to invoke graph");

    let order = execution_order.lock().unwrap();
    println!("Execution order: {:?}", *order);

    assert_eq!(*order, vec!["node_a", "node_b", "node_c"],
        "All nodes should execute in correct order without skipping");
}

#[tokio::test]
async fn test_multiple_predecessors() {
    // Test that a node with multiple predecessors waits for all to complete
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let mut graph = StateGraph::new();

    // Create nodes A, B, and C where C depends on both A and B
    let order_clone = execution_order.clone();
    graph.add_node("node_a", move |state: serde_json::Value| {
        let order = order_clone.clone();
        Box::pin(async move {
            println!("Executing node_a");
            order.lock().unwrap().push("node_a");
            Ok(state)
        })
    });

    let order_clone = execution_order.clone();
    graph.add_node("node_b", move |state: serde_json::Value| {
        let order = order_clone.clone();
        Box::pin(async move {
            println!("Executing node_b");
            order.lock().unwrap().push("node_b");
            Ok(state)
        })
    });

    let order_clone = execution_order.clone();
    graph.add_node("node_c", move |state: serde_json::Value| {
        let order = order_clone.clone();
        Box::pin(async move {
            println!("Executing node_c");
            order.lock().unwrap().push("node_c");
            Ok(state)
        })
    });

    // Graph: START -> A -> C, START -> B -> C
    // C should only execute after both A and B complete
    graph.add_edge("__start__", "node_a");
    graph.add_edge("__start__", "node_b");
    graph.add_edge("node_a", "node_c");
    graph.add_edge("node_b", "node_c");

    let compiled = graph.compile().expect("Failed to compile graph");
    let _result = compiled.invoke(json!({})).await.expect("Failed to invoke graph");

    let order = execution_order.lock().unwrap();
    println!("Execution order: {:?}", *order);

    // A and B can execute in parallel (order may vary), but C must come after both
    assert!(
        order.iter().any(|x| *x == "node_a") && order.iter().any(|x| *x == "node_b"),
        "Both A and B should execute"
    );
    
    let a_pos = order.iter().position(|x| *x == "node_a").unwrap();
    let b_pos = order.iter().position(|x| *x == "node_b").unwrap();
    let c_pos = order.iter().position(|x| *x == "node_c").unwrap();
    
    assert!(
        c_pos > a_pos && c_pos > b_pos,
        "Node C should execute after both A and B. Order: {:?}", *order
    );
}
