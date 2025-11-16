//! Task Execution Streaming Handler
//!
//! Manages streaming of task execution events to gRPC clients.
//! Handles event emission, client disconnection, and buffering.

use crate::proto::tasks::ExecutionEvent;
use tokio::sync::mpsc;

/// Types of execution events that can be streamed
#[derive(Debug, Clone)]
pub enum ExecutionEventType {
    /// Task execution started
    Started,
    /// Progress update during execution
    Progress,
    /// Output/token from LLM
    Output,
    /// Tool function call
    ToolCall,
    /// Tool execution result
    ToolResult,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
}

impl ExecutionEventType {
    /// Convert event type to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionEventType::Started => "started",
            ExecutionEventType::Progress => "progress",
            ExecutionEventType::Output => "output",
            ExecutionEventType::ToolCall => "tool_call",
            ExecutionEventType::ToolResult => "tool_result",
            ExecutionEventType::Completed => "completed",
            ExecutionEventType::Failed => "failed",
        }
    }
}

/// Builder for creating execution events
pub struct ExecutionEventBuilder {
    event_type: ExecutionEventType,
    message: String,
    status: String,
}

impl ExecutionEventBuilder {
    /// Create a new event builder
    pub fn new(event_type: ExecutionEventType) -> Self {
        Self {
            event_type,
            message: String::new(),
            status: "in_progress".to_string(),
        }
    }

    /// Set event message
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Set event status
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    /// Build the event
    pub fn build(self) -> ExecutionEvent {
        ExecutionEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: self.event_type.as_str().to_string(),
            message: self.message,
            status: self.status,
        }
    }
}

/// Task execution streaming handler
pub struct ExecutionStreamHandler {
    /// Channel for sending events
    sender: mpsc::Sender<Result<ExecutionEvent, tonic::Status>>,

    /// Buffer size for event channel
    buffer_size: usize,

    /// Whether streaming is still active
    active: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl ExecutionStreamHandler {
    /// Create a new streaming handler
    ///
    /// # Arguments
    /// * `buffer_size` - Size of the event channel buffer
    ///
    /// # Returns
    /// A tuple of (handler, receiver) for stream setup
    pub fn new(buffer_size: usize) -> (Self, mpsc::Receiver<Result<ExecutionEvent, tonic::Status>>) {
        let (tx, rx) = mpsc::channel(buffer_size);

        let handler = Self {
            sender: tx,
            buffer_size,
            active: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        };

        (handler, rx)
    }

    /// Send an execution event
    pub async fn send_event(&self, event: ExecutionEvent) -> Result<(), String> {
        if !self.active.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("Stream is closed".to_string());
        }

        self.sender
            .send(Ok(event))
            .await
            .map_err(|e| format!("Failed to send event: {}", e))
    }

    /// Send a started event
    pub async fn send_started(&self, task_id: &str) -> Result<(), String> {
        let event = ExecutionEventBuilder::new(ExecutionEventType::Started)
            .with_message(format!("Task {} started", task_id))
            .with_status("started")
            .build();

        self.send_event(event).await
    }

    /// Send a progress event
    pub async fn send_progress(&self, message: impl Into<String>) -> Result<(), String> {
        let event = ExecutionEventBuilder::new(ExecutionEventType::Progress)
            .with_message(message)
            .build();

        self.send_event(event).await
    }

    /// Send an output event
    pub async fn send_output(&self, output: impl Into<String>) -> Result<(), String> {
        let event = ExecutionEventBuilder::new(ExecutionEventType::Output)
            .with_message(output)
            .build();

        self.send_event(event).await
    }

    /// Send a tool call event
    pub async fn send_tool_call(&self, tool_name: &str, tool_input: &str) -> Result<(), String> {
        let event = ExecutionEventBuilder::new(ExecutionEventType::ToolCall)
            .with_message(format!("Tool: {} | Input: {}", tool_name, tool_input))
            .build();

        self.send_event(event).await
    }

    /// Send a tool result event
    pub async fn send_tool_result(&self, tool_name: &str, result: &str) -> Result<(), String> {
        let event = ExecutionEventBuilder::new(ExecutionEventType::ToolResult)
            .with_message(format!("Tool: {} | Result: {}", tool_name, result))
            .build();

        self.send_event(event).await
    }

    /// Send a completion event
    pub async fn send_completed(&self, task_id: &str, result: impl Into<String>) -> Result<(), String> {
        let event = ExecutionEventBuilder::new(ExecutionEventType::Completed)
            .with_message(format!("Task {} completed: {}", task_id, result.into()))
            .with_status("completed")
            .build();

        let result = self.send_event(event).await;
        self.active.store(false, std::sync::atomic::Ordering::SeqCst);
        result
    }

    /// Send a failure event
    pub async fn send_failed(&self, task_id: &str, error: impl Into<String>) -> Result<(), String> {
        let event = ExecutionEventBuilder::new(ExecutionEventType::Failed)
            .with_message(format!("Task {} failed: {}", task_id, error.into()))
            .with_status("failed")
            .build();

        let result = self.send_event(event).await;
        self.active.store(false, std::sync::atomic::Ordering::SeqCst);
        result
    }

    /// Check if stream is still active
    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Close the stream
    pub fn close(&self) {
        self.active.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // ========================================================================
    // Phase 7.2: Orchestrator Streaming Tests
    // ========================================================================

    // ------------------------------------------------------------------------
    // Basic Event Building and Streaming
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_execution_event_builder() {
        let event = ExecutionEventBuilder::new(ExecutionEventType::Started)
            .with_message("Test message")
            .with_status("active")
            .build();

        assert_eq!(event.event_type, "started");
        assert_eq!(event.message, "Test message");
        assert_eq!(event.status, "active");
    }

    #[tokio::test]
    async fn test_execution_event_type_strings() {
        assert_eq!(ExecutionEventType::Started.as_str(), "started");
        assert_eq!(ExecutionEventType::Progress.as_str(), "progress");
        assert_eq!(ExecutionEventType::Output.as_str(), "output");
        assert_eq!(ExecutionEventType::ToolCall.as_str(), "tool_call");
        assert_eq!(ExecutionEventType::ToolResult.as_str(), "tool_result");
        assert_eq!(ExecutionEventType::Completed.as_str(), "completed");
        assert_eq!(ExecutionEventType::Failed.as_str(), "failed");
    }

    #[tokio::test]
    async fn test_execution_event_builder_default_status() {
        let event = ExecutionEventBuilder::new(ExecutionEventType::Progress)
            .with_message("Progress update")
            .build();

        assert_eq!(event.status, "in_progress");
    }

    #[tokio::test]
    async fn test_execution_event_timestamp_format() {
        let event = ExecutionEventBuilder::new(ExecutionEventType::Started)
            .with_message("Test")
            .build();

        // Timestamp should be in RFC3339 format
        assert!(!event.timestamp.is_empty());
        // Should be parseable as RFC3339
        assert!(chrono::DateTime::parse_from_rfc3339(&event.timestamp).is_ok());
    }

    #[tokio::test]
    async fn test_execution_stream_handler_creation() {
        let (handler, mut rx) = ExecutionStreamHandler::new(100);
        assert!(handler.is_active());

        // Send an event
        handler.send_started("test-task").await.unwrap();

        // Receive the event
        let event = rx.recv().await.unwrap().unwrap();
        assert_eq!(event.event_type, "started");
    }

    #[tokio::test]
    async fn test_execution_stream_close() {
        let (handler, _rx) = ExecutionStreamHandler::new(100);
        assert!(handler.is_active());

        handler.close();
        assert!(!handler.is_active());
    }

    #[tokio::test]
    async fn test_multiple_event_types() {
        let (handler, mut rx) = ExecutionStreamHandler::new(100);

        handler.send_started("task-1").await.unwrap();
        handler.send_progress("Processing...").await.unwrap();
        handler.send_output("Output data").await.unwrap();
        handler.send_tool_call("my_tool", "{\"input\": \"value\"}").await.unwrap();
        handler.send_tool_result("my_tool", "{\"result\": \"success\"}").await.unwrap();
        handler.send_completed("task-1", "Success").await.unwrap();

        // Verify all events were sent
        let mut count = 0;
        while let Some(_) = rx.recv().await {
            count += 1;
            if count >= 6 {
                break;
            }
        }
        assert_eq!(count, 6);
    }

    // ------------------------------------------------------------------------
    // Backpressure Handling Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_backpressure_small_buffer() {
        // Create handler with very small buffer
        let (handler, mut rx) = ExecutionStreamHandler::new(2);

        // Fill buffer completely
        handler.send_progress("Event 1").await.unwrap();
        handler.send_progress("Event 2").await.unwrap();

        // Next send should block or fail if buffer is full
        // Spawn a task to send while buffer is full
        let handler_clone = Arc::new(handler);
        let send_handle = {
            let h = handler_clone.clone();
            tokio::spawn(async move {
                h.send_progress("Event 3").await
            })
        };

        // Small delay to let send attempt start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Now consume one event to make room
        let event1 = rx.recv().await.unwrap().unwrap();
        assert!(event1.message.contains("Event 1"));

        // The blocked send should now complete
        let send_result = send_handle.await.unwrap();
        assert!(send_result.is_ok());

        // Consume remaining events
        let event2 = rx.recv().await.unwrap().unwrap();
        assert!(event2.message.contains("Event 2"));
        let event3 = rx.recv().await.unwrap().unwrap();
        assert!(event3.message.contains("Event 3"));
    }

    #[tokio::test]
    async fn test_backpressure_high_throughput() {
        let (handler, mut rx) = ExecutionStreamHandler::new(50);

        // Spawn consumer task
        let consumer_handle = tokio::spawn(async move {
            let mut count = 0;
            while let Some(result) = rx.recv().await {
                assert!(result.is_ok());
                count += 1;
                if count >= 100 {
                    break;
                }
            }
            count
        });

        // Send 100 events rapidly
        for i in 0..100 {
            handler.send_progress(format!("Event {}", i)).await.unwrap();
        }

        // Wait for consumer to receive all
        let received_count = consumer_handle.await.unwrap();
        assert_eq!(received_count, 100);
    }

    #[tokio::test]
    async fn test_backpressure_slow_consumer() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        // Spawn slow consumer
        let consumer_handle = tokio::spawn(async move {
            let mut count = 0;
            while let Some(result) = rx.recv().await {
                assert!(result.is_ok());
                count += 1;
                // Simulate slow processing
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                if count >= 20 {
                    break;
                }
            }
            count
        });

        // Send events faster than consumer can process
        for i in 0..20 {
            handler.send_progress(format!("Event {}", i)).await.unwrap();
        }

        let received_count = consumer_handle.await.unwrap();
        assert_eq!(received_count, 20);
    }

    // ------------------------------------------------------------------------
    // Stream Error Recovery Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_send_after_close() {
        let (handler, _rx) = ExecutionStreamHandler::new(10);

        // Close the stream
        handler.close();
        assert!(!handler.is_active());

        // Attempt to send should fail
        let result = handler.send_progress("Should fail").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Stream is closed"));
    }

    #[tokio::test]
    async fn test_send_after_completion() {
        let (handler, _rx) = ExecutionStreamHandler::new(10);

        // Send completion event (should close stream)
        handler.send_completed("task-1", "Done").await.unwrap();

        // Stream should now be inactive
        assert!(!handler.is_active());

        // Subsequent sends should fail
        let result = handler.send_progress("After completion").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_after_failure() {
        let (handler, _rx) = ExecutionStreamHandler::new(10);

        // Send failure event (should close stream)
        handler.send_failed("task-1", "Error occurred").await.unwrap();

        // Stream should now be inactive
        assert!(!handler.is_active());

        // Subsequent sends should fail
        let result = handler.send_output("After failure").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_receiver_dropped() {
        let (handler, rx) = ExecutionStreamHandler::new(10);

        // Drop the receiver
        drop(rx);

        // Sends should fail when receiver is dropped
        let result = handler.send_progress("No receiver").await;
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Client Disconnection Handling Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_client_disconnection_cleanup() {
        let (handler, rx) = ExecutionStreamHandler::new(10);

        // Send some events
        handler.send_started("task-1").await.unwrap();
        handler.send_progress("Processing...").await.unwrap();

        // Simulate client disconnection by dropping receiver
        drop(rx);

        // Handler should detect disconnection on next send
        let result = handler.send_output("After disconnect").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_graceful_stream_shutdown() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        // Send events
        handler.send_started("task-1").await.unwrap();
        handler.send_completed("task-1", "Success").await.unwrap();

        // Stream should be closed
        assert!(!handler.is_active());

        // Receiver should get all events before closure
        let event1 = rx.recv().await.unwrap().unwrap();
        assert_eq!(event1.event_type, "started");
        let event2 = rx.recv().await.unwrap().unwrap();
        assert_eq!(event2.event_type, "completed");

        // No more events should arrive
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn test_partial_event_delivery_on_disconnect() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        // Send multiple events
        for i in 0..5 {
            handler.send_progress(format!("Event {}", i)).await.unwrap();
        }

        // Consume only some events
        let _event1 = rx.recv().await.unwrap();
        let _event2 = rx.recv().await.unwrap();

        // Drop receiver (simulate disconnect)
        drop(rx);

        // Handler should detect on next send
        let result = handler.send_progress("Lost event").await;
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Concurrent Stream Consumer Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_concurrent_event_sends() {
        let (handler, mut rx) = ExecutionStreamHandler::new(100);
        let handler = Arc::new(handler);

        // Spawn multiple concurrent senders
        let mut send_handles = vec![];
        for i in 0..10 {
            let h = handler.clone();
            let handle = tokio::spawn(async move {
                for j in 0..10 {
                    h.send_progress(format!("Thread {} Event {}", i, j)).await.unwrap();
                }
            });
            send_handles.push(handle);
        }

        // Spawn receiver
        let recv_handle = tokio::spawn(async move {
            let mut count = 0;
            while let Some(result) = rx.recv().await {
                assert!(result.is_ok());
                count += 1;
                if count >= 100 {
                    break;
                }
            }
            count
        });

        // Wait for all senders
        for handle in send_handles {
            handle.await.unwrap();
        }

        // Wait for receiver
        let received = recv_handle.await.unwrap();
        assert_eq!(received, 100);
    }

    #[tokio::test]
    async fn test_stream_ordering_maintained() {
        let (handler, mut rx) = ExecutionStreamHandler::new(100);

        // Send ordered events
        for i in 0..50 {
            handler.send_progress(format!("Event {}", i)).await.unwrap();
        }

        // Verify order is maintained
        for i in 0..50 {
            let event = rx.recv().await.unwrap().unwrap();
            assert_eq!(event.message, format!("Event {}", i));
        }
    }

    #[tokio::test]
    async fn test_multiple_concurrent_receivers_not_supported() {
        let (handler, rx) = ExecutionStreamHandler::new(10);

        // Can only have one receiver (by design of mpsc)
        // This test documents that behavior

        // Send event
        handler.send_started("task-1").await.unwrap();

        // Can't create second receiver from same channel
        // (This is a design constraint of mpsc::channel)

        drop(rx);
        // After dropping, sends should fail
        let result = handler.send_progress("After drop").await;
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Event Content Validation Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_started_event_format() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        handler.send_started("task-123").await.unwrap();

        let event = rx.recv().await.unwrap().unwrap();
        assert_eq!(event.event_type, "started");
        assert!(event.message.contains("task-123"));
        assert!(event.message.contains("started"));
        assert_eq!(event.status, "started");
    }

    #[tokio::test]
    async fn test_tool_call_event_format() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        handler.send_tool_call("read_file", "/path/to/file").await.unwrap();

        let event = rx.recv().await.unwrap().unwrap();
        assert_eq!(event.event_type, "tool_call");
        assert!(event.message.contains("read_file"));
        assert!(event.message.contains("/path/to/file"));
    }

    #[tokio::test]
    async fn test_tool_result_event_format() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        handler.send_tool_result("read_file", "file contents").await.unwrap();

        let event = rx.recv().await.unwrap().unwrap();
        assert_eq!(event.event_type, "tool_result");
        assert!(event.message.contains("read_file"));
        assert!(event.message.contains("file contents"));
    }

    #[tokio::test]
    async fn test_completed_event_closes_stream() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        assert!(handler.is_active());

        handler.send_completed("task-1", "All done").await.unwrap();

        assert!(!handler.is_active());

        let event = rx.recv().await.unwrap().unwrap();
        assert_eq!(event.event_type, "completed");
        assert_eq!(event.status, "completed");
        assert!(event.message.contains("task-1"));
        assert!(event.message.contains("All done"));
    }

    #[tokio::test]
    async fn test_failed_event_closes_stream() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        assert!(handler.is_active());

        handler.send_failed("task-1", "Something went wrong").await.unwrap();

        assert!(!handler.is_active());

        let event = rx.recv().await.unwrap().unwrap();
        assert_eq!(event.event_type, "failed");
        assert_eq!(event.status, "failed");
        assert!(event.message.contains("task-1"));
        assert!(event.message.contains("Something went wrong"));
    }

    // ------------------------------------------------------------------------
    // Buffer Size Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_buffer_size_one() {
        let (handler, mut rx) = ExecutionStreamHandler::new(1);

        // Can send one event
        handler.send_progress("Event 1").await.unwrap();

        // Second send will block until first is consumed
        let send_future = handler.send_progress("Event 2");

        // Spawn send in background
        let send_handle = tokio::spawn(send_future);

        // Small delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Consume first event
        let _e1 = rx.recv().await.unwrap();

        // Second send should complete
        send_handle.await.unwrap().unwrap();

        // Receive second event
        let e2 = rx.recv().await.unwrap().unwrap();
        assert!(e2.message.contains("Event 2"));
    }

    #[tokio::test]
    async fn test_large_buffer_no_blocking() {
        let (handler, mut rx) = ExecutionStreamHandler::new(1000);

        // Send many events without consuming
        for i in 0..500 {
            handler.send_progress(format!("Event {}", i)).await.unwrap();
        }

        // All should have been buffered successfully
        // Now consume them all
        for i in 0..500 {
            let event = rx.recv().await.unwrap().unwrap();
            assert_eq!(event.message, format!("Event {}", i));
        }
    }

    // ------------------------------------------------------------------------
    // Edge Cases
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_empty_message() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        handler.send_progress("").await.unwrap();

        let event = rx.recv().await.unwrap().unwrap();
        assert_eq!(event.message, "");
    }

    #[tokio::test]
    async fn test_very_long_message() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        let long_message = "A".repeat(10000);
        handler.send_output(&long_message).await.unwrap();

        let event = rx.recv().await.unwrap().unwrap();
        assert_eq!(event.message.len(), 10000);
    }

    #[tokio::test]
    async fn test_special_characters_in_message() {
        let (handler, mut rx) = ExecutionStreamHandler::new(10);

        let special = "Test with 特殊文字 and émojis 🚀💯";
        handler.send_progress(special).await.unwrap();

        let event = rx.recv().await.unwrap().unwrap();
        assert_eq!(event.message, special);
    }
}
