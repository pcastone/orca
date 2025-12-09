//! Test ReAct agent routing and execution

use langgraph_prebuilt::{create_react_agent, Message};
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_react_agent_calls_llm_function() {
    // Track if LLM function is called
    let llm_called = Arc::new(Mutex::new(false));
    let called_clone = llm_called.clone();

    // Create simple LLM function that returns immediately with no tool calls
    let llm_fn = Arc::new(move |_state: serde_json::Value| {
        let called = called_clone.clone();
        Box::pin(async move {
            println!("LLM function called");
            *called.lock().unwrap() = true;
            // Return AI message with NO tool calls
            Ok(Message::ai("Test response"))
        }) as Pin<Box<dyn Future<Output = Result<Message, langgraph_prebuilt::error::PrebuiltError>> + Send>>
    });

    // Create agent with NO tools (should use direct response)
    let agent = create_react_agent(llm_fn, vec![])
        .with_max_iterations(1)
        .build()
        .expect("Failed to build agent");

    // Execute
    let input = json!({
        "messages": vec![json!({
            "type": "human",
            "content": "test"
        })]
    });

    println!("Invoking agent...");
    let result = agent.invoke(input).await.expect("Agent execution failed");

    println!("Agent execution completed");
    println!("Result: {:?}", result);
    println!("Result type: {}", if result.is_object() { "object" } else if result.is_array() { "array" } else { "other" });
    if result.is_object() {
        println!("Result keys: {:?}", result.as_object().unwrap().keys().collect::<Vec<_>>());
    }

    // Verify LLM was called
    assert!(
        *llm_called.lock().unwrap(),
        "LLM function should have been called"
    );

    // Verify messages are in the result
    if !result.is_object() || !result.as_object().unwrap().contains_key("messages") {
        panic!("Result doesn't have messages key. Result: {:?}", result);
    }

    let messages = result["messages"]
        .as_array()
        .expect("Should have messages array");

    println!("Messages count: {}", messages.len());
    println!("Messages: {:?}", messages);

    assert!(
        messages.len() >= 2,
        "Should have at least human + AI message, got {}",
        messages.len()
    );

    // Verify AI message exists
    let has_ai_message = messages.iter().any(|msg| {
        msg.get("type")
            .and_then(|t| t.as_str())
            .map(|t| t == "ai")
            .unwrap_or(false)
    });

    assert!(has_ai_message, "Should have AI message in results");
}

#[tokio::test]
async fn test_react_agent_with_tools_list() {
    use langgraph_prebuilt::tools::Tool;
    use async_trait::async_trait;

    // Simple test tool
    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        async fn execute(&self, _input: serde_json::Value) -> Result<serde_json::Value, langgraph_prebuilt::error::PrebuiltError> {
            Ok(json!({"result": "tool executed"}))
        }
    }

    // Track execution
    let llm_called = Arc::new(Mutex::new(false));
    let called_clone = llm_called.clone();

    // LLM function that returns without tool calls
    let llm_fn = Arc::new(move |_state: serde_json::Value| {
        let called = called_clone.clone();
        Box::pin(async move {
            *called.lock().unwrap() = true;
            Ok(Message::ai("Response without tools"))
        }) as Pin<Box<dyn Future<Output = _> + Send>>
    });

    // Create agent WITH tools
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(TestTool)];

    let agent = create_react_agent(llm_fn, tools)
        .with_max_iterations(1)
        .build()
        .expect("Failed to build agent with tools");

    let input = json!({
        "messages": vec![json!({
            "type": "human",
            "content": "test"
        })]
    });

    println!("Invoking agent with tools...");
    let result = agent.invoke(input).await.expect("Agent execution failed");

    println!("LLM called: {}", *llm_called.lock().unwrap());
    println!("Result: {:?}", result);

    assert!(
        *llm_called.lock().unwrap(),
        "LLM function should have been called even with tools available"
    );
}

#[tokio::test]
async fn test_react_agent_execution_order() {
    // Track execution order
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let order_clone = execution_order.clone();

    let llm_fn = Arc::new(move |state: serde_json::Value| {
        let order = order_clone.clone();
        Box::pin(async move {
            println!("LLM function executing");
            order.lock().unwrap().push("llm");

            // Check current messages
            let msg_count = state["messages"]
                .as_array()
                .map(|a| a.len())
                .unwrap_or(0);
            println!("LLM sees {} messages", msg_count);

            Ok(Message::ai("response"))
        }) as Pin<Box<dyn Future<Output = _> + Send>>
    });

    let agent = create_react_agent(llm_fn, vec![])
        .with_max_iterations(1)
        .build()
        .expect("Failed to build agent");

    let input = json!({
        "messages": vec![json!({
            "type": "human",
            "content": "test"
        })]
    });

    let _result = agent.invoke(input).await.expect("Failed to execute");

    let order = execution_order.lock().unwrap();
    println!("Execution order: {:?}", *order);

    assert!(
        !order.is_empty(),
        "LLM should have been called"
    );
    assert_eq!(order[0], "llm", "LLM should be called first");
}
