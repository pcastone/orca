use orchestrator::execution::{ExecutionStreamHandler, ExecutionEventType, ExecutionEventBuilder};

#[tokio::test]
async fn test_execution_stream_handler_send_started() {
    let (handler, mut rx) = ExecutionStreamHandler::new(100);

    handler.send_started("task-1").await.unwrap();

    let event = rx.recv().await.unwrap().unwrap();
    assert_eq!(event.event_type, "started");
    assert!(event.message.contains("task-1"));
}

#[tokio::test]
async fn test_execution_stream_handler_send_progress() {
    let (handler, mut rx) = ExecutionStreamHandler::new(100);

    handler.send_progress("Processing step 1").await.unwrap();

    let event = rx.recv().await.unwrap().unwrap();
    assert_eq!(event.event_type, "progress");
    assert_eq!(event.message, "Processing step 1");
}

#[tokio::test]
async fn test_execution_stream_handler_send_output() {
    let (handler, mut rx) = ExecutionStreamHandler::new(100);

    handler.send_output("Task output data").await.unwrap();

    let event = rx.recv().await.unwrap().unwrap();
    assert_eq!(event.event_type, "output");
    assert_eq!(event.message, "Task output data");
}

#[tokio::test]
async fn test_execution_stream_handler_send_tool_call() {
    let (handler, mut rx) = ExecutionStreamHandler::new(100);

    handler.send_tool_call("my_tool", r#"{"param": "value"}"#).await.unwrap();

    let event = rx.recv().await.unwrap().unwrap();
    assert_eq!(event.event_type, "tool_call");
    assert!(event.message.contains("my_tool"));
}

#[tokio::test]
async fn test_execution_stream_handler_send_tool_result() {
    let (handler, mut rx) = ExecutionStreamHandler::new(100);

    handler.send_tool_result("my_tool", r#"{"result": "success"}"#).await.unwrap();

    let event = rx.recv().await.unwrap().unwrap();
    assert_eq!(event.event_type, "tool_result");
    assert!(event.message.contains("my_tool"));
}

#[tokio::test]
async fn test_execution_stream_handler_send_completed() {
    let (handler, mut rx) = ExecutionStreamHandler::new(100);

    assert!(handler.is_active());
    handler.send_completed("task-1", "Success").await.unwrap();
    assert!(!handler.is_active());

    let event = rx.recv().await.unwrap().unwrap();
    assert_eq!(event.event_type, "completed");
    assert_eq!(event.status, "completed");
}

#[tokio::test]
async fn test_execution_stream_handler_send_failed() {
    let (handler, mut rx) = ExecutionStreamHandler::new(100);

    assert!(handler.is_active());
    handler.send_failed("task-1", "Error occurred").await.unwrap();
    assert!(!handler.is_active());

    let event = rx.recv().await.unwrap().unwrap();
    assert_eq!(event.event_type, "failed");
    assert_eq!(event.status, "failed");
}

#[tokio::test]
async fn test_execution_event_builder_start() {
    let event = ExecutionEventBuilder::new(ExecutionEventType::Started)
        .with_message("Task started")
        .with_status("active")
        .build();

    assert_eq!(event.event_type, "started");
    assert_eq!(event.message, "Task started");
    assert_eq!(event.status, "active");
}

#[tokio::test]
async fn test_execution_event_builder_progress() {
    let event = ExecutionEventBuilder::new(ExecutionEventType::Progress)
        .with_message("50% complete")
        .build();

    assert_eq!(event.event_type, "progress");
    assert_eq!(event.message, "50% complete");
}

#[tokio::test]
async fn test_execution_event_builder_completion() {
    let event = ExecutionEventBuilder::new(ExecutionEventType::Completed)
        .with_message("Task finished")
        .with_status("completed")
        .build();

    assert_eq!(event.event_type, "completed");
    assert_eq!(event.status, "completed");
}

#[tokio::test]
async fn test_execution_stream_close_blocks_sends() {
    let (handler, _rx) = ExecutionStreamHandler::new(100);

    handler.close();
    assert!(!handler.is_active());

    // Attempting to send after close should fail
    let result = handler.send_progress("This should fail").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execution_stream_multiple_events() {
    let (handler, mut rx) = ExecutionStreamHandler::new(100);

    // Send multiple events
    handler.send_started("task-1").await.unwrap();
    handler.send_progress("Step 1").await.unwrap();
    handler.send_progress("Step 2").await.unwrap();
    handler.send_output("Output").await.unwrap();
    handler.send_completed("task-1", "Done").await.unwrap();

    // Receive and verify all events
    let mut event_types = vec![];
    while let Some(Ok(event)) = rx.recv().await {
        event_types.push(event.event_type.clone());
    }

    assert_eq!(event_types.len(), 5);
    assert_eq!(event_types[0], "started");
    assert_eq!(event_types[1], "progress");
    assert_eq!(event_types[2], "progress");
    assert_eq!(event_types[3], "output");
    assert_eq!(event_types[4], "completed");
}

#[tokio::test]
async fn test_execution_event_type_as_str() {
    assert_eq!(ExecutionEventType::Started.as_str(), "started");
    assert_eq!(ExecutionEventType::Progress.as_str(), "progress");
    assert_eq!(ExecutionEventType::Output.as_str(), "output");
    assert_eq!(ExecutionEventType::ToolCall.as_str(), "tool_call");
    assert_eq!(ExecutionEventType::ToolResult.as_str(), "tool_result");
    assert_eq!(ExecutionEventType::Completed.as_str(), "completed");
    assert_eq!(ExecutionEventType::Failed.as_str(), "failed");
}

#[tokio::test]
async fn test_execution_stream_timestamp() {
    let (handler, mut rx) = ExecutionStreamHandler::new(100);

    handler.send_started("task-1").await.unwrap();

    let event = rx.recv().await.unwrap().unwrap();
    // Verify timestamp is RFC3339 format
    assert!(!event.timestamp.is_empty());
    assert!(event.timestamp.contains("T"));
    assert!(event.timestamp.contains("Z") || event.timestamp.contains("+"));
}
