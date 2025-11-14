//! Task Execution Streaming Handler
//!
//! Manages streaming of task execution events to gRPC clients.
//! Handles event emission, client disconnection, and buffering.

use crate::proto::tasks::ExecutionEvent;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

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
}
