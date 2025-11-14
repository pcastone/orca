//! Task Execution Streaming
//!
//! This module provides real-time streaming capabilities for task execution,
//! allowing clients to receive incremental updates as tasks progress.

use crate::{OrchestratorError, Result};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio::sync::mpsc;
use tracing::{debug, warn};
use uuid::Uuid;

/// Task update event emitted during streaming execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdate {
    /// Task ID this update belongs to
    pub task_id: Uuid,

    /// Type of update
    pub update_type: UpdateType,

    /// Sequence number for ordering updates
    pub sequence: u64,

    /// Timestamp of the update
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Partial content (for incremental updates)
    pub partial_content: Option<String>,

    /// Completion percentage (0-100)
    pub progress: Option<u8>,

    /// Error message if update indicates failure
    pub error: Option<String>,
}

impl TaskUpdate {
    /// Create a new task update
    pub fn new(task_id: Uuid, update_type: UpdateType, sequence: u64) -> Self {
        Self {
            task_id,
            update_type,
            sequence,
            timestamp: chrono::Utc::now(),
            partial_content: None,
            progress: None,
            error: None,
        }
    }

    /// Set partial content
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.partial_content = Some(content.into());
        self
    }

    /// Set progress percentage
    pub fn with_progress(mut self, progress: u8) -> Self {
        self.progress = Some(progress.clamp(0, 100));
        self
    }

    /// Set error message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

/// Type of task update
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateType {
    /// Task execution started
    Started,

    /// Incremental content/token received
    Token,

    /// Progress update (percentage change)
    Progress,

    /// Task completed successfully
    Completed,

    /// Task failed
    Failed,

    /// Task was cancelled/interrupted
    Cancelled,

    /// General status update
    Status,
}

/// Stream of task updates
pub type TaskUpdateStream = Pin<Box<dyn Stream<Item = TaskUpdate> + Send>>;

/// Builder for creating task update streams
pub struct StreamBuilder {
    /// Task ID for the stream
    task_id: Uuid,

    /// Buffer size for the update channel
    buffer_size: usize,

    /// Whether to include token-level updates
    include_tokens: bool,

    /// Whether to include progress updates
    include_progress: bool,
}

impl StreamBuilder {
    /// Create a new stream builder
    pub fn new(task_id: Uuid) -> Self {
        Self {
            task_id,
            buffer_size: 100,
            include_tokens: true,
            include_progress: true,
        }
    }

    /// Set buffer size for the update channel
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Set whether to include token-level updates
    pub fn include_tokens(mut self, include: bool) -> Self {
        self.include_tokens = include;
        self
    }

    /// Set whether to include progress updates
    pub fn include_progress(mut self, include: bool) -> Self {
        self.include_progress = include;
        self
    }

    /// Build the stream and return the sender/receiver pair
    pub fn build(self) -> (TaskUpdateSender, TaskUpdateStream) {
        let (tx, rx) = mpsc::channel(self.buffer_size);

        let sender = TaskUpdateSender {
            task_id: self.task_id,
            tx,
            sequence: 0,
            include_tokens: self.include_tokens,
            include_progress: self.include_progress,
        };

        let stream = Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));

        (sender, stream)
    }
}

/// Sender for task updates
pub struct TaskUpdateSender {
    /// Task ID
    task_id: Uuid,

    /// Channel sender
    tx: mpsc::Sender<TaskUpdate>,

    /// Current sequence number
    sequence: u64,

    /// Whether to send token updates
    include_tokens: bool,

    /// Whether to send progress updates
    include_progress: bool,
}

impl TaskUpdateSender {
    /// Send a task update
    async fn send(&mut self, update: TaskUpdate) -> Result<()> {
        self.tx
            .send(update)
            .await
            .map_err(|_| OrchestratorError::General("Update stream closed".to_string()))
    }

    /// Send a started update
    pub async fn started(&mut self) -> Result<()> {
        let update = TaskUpdate::new(self.task_id, UpdateType::Started, self.sequence);
        self.sequence += 1;
        self.send(update).await
    }

    /// Send a token update
    pub async fn token(&mut self, content: impl Into<String>) -> Result<()> {
        if !self.include_tokens {
            return Ok(());
        }

        let update = TaskUpdate::new(self.task_id, UpdateType::Token, self.sequence)
            .with_content(content);
        self.sequence += 1;
        self.send(update).await
    }

    /// Send a progress update
    pub async fn progress(&mut self, percentage: u8) -> Result<()> {
        if !self.include_progress {
            return Ok(());
        }

        let update = TaskUpdate::new(self.task_id, UpdateType::Progress, self.sequence)
            .with_progress(percentage);
        self.sequence += 1;
        self.send(update).await
    }

    /// Send a status update
    pub async fn status(&mut self, message: impl Into<String>) -> Result<()> {
        let update = TaskUpdate::new(self.task_id, UpdateType::Status, self.sequence)
            .with_content(message);
        self.sequence += 1;
        self.send(update).await
    }

    /// Send a completed update
    pub async fn completed(&mut self) -> Result<()> {
        let update = TaskUpdate::new(self.task_id, UpdateType::Completed, self.sequence)
            .with_progress(100);
        self.sequence += 1;
        self.send(update).await
    }

    /// Send a failed update
    pub async fn failed(&mut self, error: impl Into<String>) -> Result<()> {
        let update = TaskUpdate::new(self.task_id, UpdateType::Failed, self.sequence)
            .with_error(error);
        self.sequence += 1;
        self.send(update).await
    }

    /// Send a cancelled update
    pub async fn cancelled(&mut self) -> Result<()> {
        let update = TaskUpdate::new(self.task_id, UpdateType::Cancelled, self.sequence);
        self.sequence += 1;
        self.send(update).await
    }

    /// Check if the receiver is still active
    pub fn is_active(&self) -> bool {
        !self.tx.is_closed()
    }

    /// Get current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.sequence
    }
}

/// Handle stream interruptions gracefully
pub async fn handle_stream_interruption<F, Fut>(
    sender: &mut TaskUpdateSender,
    operation: F,
) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    match operation().await {
        Ok(()) => Ok(()),
        Err(e) => {
            warn!("Stream operation interrupted: {}", e);

            // Try to send cancellation update if stream is still active
            if sender.is_active() {
                if let Err(send_err) = sender.cancelled().await {
                    debug!("Failed to send cancellation update: {}", send_err);
                }
            }

            Err(e)
        }
    }
}

/// Execute a function and send updates based on completion
pub async fn execute_with_updates<F, Fut, T>(
    sender: &mut TaskUpdateSender,
    operation: F,
) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    // Send started update
    sender.started().await?;

    // Execute the operation
    match operation().await {
        Ok(result) => {
            // Send completed update
            sender.completed().await?;
            Ok(result)
        }
        Err(e) => {
            // Send failed update
            let error_msg = e.to_string();
            if sender.is_active() {
                let _ = sender.failed(&error_msg).await;
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_stream_builder() {
        let task_id = Uuid::new_v4();
        let (mut sender, mut stream) = StreamBuilder::new(task_id)
            .buffer_size(10)
            .include_tokens(true)
            .build();

        // Send a started update
        sender.started().await.unwrap();

        // Receive the update
        let update = stream.next().await.unwrap();
        assert_eq!(update.task_id, task_id);
        assert_eq!(update.update_type, UpdateType::Started);
        assert_eq!(update.sequence, 0);
    }

    #[tokio::test]
    async fn test_task_update_builder() {
        let task_id = Uuid::new_v4();
        let update = TaskUpdate::new(task_id, UpdateType::Token, 0)
            .with_content("Hello")
            .with_progress(50);

        assert_eq!(update.task_id, task_id);
        assert_eq!(update.update_type, UpdateType::Token);
        assert_eq!(update.partial_content, Some("Hello".to_string()));
        assert_eq!(update.progress, Some(50));
    }

    #[tokio::test]
    async fn test_sender_sequence() {
        let task_id = Uuid::new_v4();
        let (mut sender, mut stream) = StreamBuilder::new(task_id).build();

        sender.started().await.unwrap();
        sender.token("test").await.unwrap();
        sender.progress(50).await.unwrap();
        sender.completed().await.unwrap();

        assert_eq!(stream.next().await.unwrap().sequence, 0);
        assert_eq!(stream.next().await.unwrap().sequence, 1);
        assert_eq!(stream.next().await.unwrap().sequence, 2);
        assert_eq!(stream.next().await.unwrap().sequence, 3);
    }

    #[tokio::test]
    async fn test_sender_token_filtering() {
        let task_id = Uuid::new_v4();
        let (mut sender, mut stream) = StreamBuilder::new(task_id)
            .include_tokens(false)
            .build();

        sender.started().await.unwrap();
        sender.token("should be filtered").await.unwrap();
        sender.progress(50).await.unwrap();

        // Should get started (seq 0) and progress (seq 2), token is filtered
        let update1 = stream.next().await.unwrap();
        assert_eq!(update1.update_type, UpdateType::Started);

        let update2 = stream.next().await.unwrap();
        assert_eq!(update2.update_type, UpdateType::Progress);
    }

    #[tokio::test]
    async fn test_sender_is_active() {
        let task_id = Uuid::new_v4();
        let (mut sender, stream) = StreamBuilder::new(task_id).build();

        assert!(sender.is_active());

        // Drop the stream to close the channel
        drop(stream);

        // Give it a moment to close
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert!(!sender.is_active());
    }

    #[tokio::test]
    async fn test_execute_with_updates_success() {
        let task_id = Uuid::new_v4();
        let (mut sender, mut stream) = StreamBuilder::new(task_id).build();

        let result = execute_with_updates(&mut sender, || async { Ok::<i32, OrchestratorError>(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        // Should have started and completed updates
        let update1 = stream.next().await.unwrap();
        assert_eq!(update1.update_type, UpdateType::Started);

        let update2 = stream.next().await.unwrap();
        assert_eq!(update2.update_type, UpdateType::Completed);
        assert_eq!(update2.progress, Some(100));
    }

    #[tokio::test]
    async fn test_execute_with_updates_failure() {
        let task_id = Uuid::new_v4();
        let (mut sender, mut stream) = StreamBuilder::new(task_id).build();

        let result = execute_with_updates(&mut sender, || async {
            Err::<i32, OrchestratorError>(OrchestratorError::General("test error".to_string()))
        })
        .await;

        assert!(result.is_err());

        // Should have started and failed updates
        let update1 = stream.next().await.unwrap();
        assert_eq!(update1.update_type, UpdateType::Started);

        let update2 = stream.next().await.unwrap();
        assert_eq!(update2.update_type, UpdateType::Failed);
        assert!(update2.error.is_some());
    }

    #[tokio::test]
    async fn test_progress_clamping() {
        let update = TaskUpdate::new(Uuid::new_v4(), UpdateType::Progress, 0).with_progress(150);

        assert_eq!(update.progress, Some(100)); // Should be clamped to 100
    }
}
