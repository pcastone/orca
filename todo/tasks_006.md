# Task 006: Implement gRPC Streaming Utilities

## Objective
Create reusable utilities and patterns for gRPC streaming (server-side streaming for task execution events) including backpressure handling, error recovery, and stream lifecycle management.

## Priority
**HIGH** - Required for real-time execution updates

## Dependencies
- Task 001 (Protocol Buffer definitions)
- Task 004 (Proto-domain conversions)
- Task 005 (Error handling)

## Implementation Details

### Files to Create

1. **`src/crates/orchestrator/src/streaming/mod.rs`**:
```rust
pub mod broadcaster;
pub mod executor_stream;
pub mod backpressure;

pub use broadcaster::EventBroadcaster;
pub use executor_stream::ExecutorStreamHandler;
pub use backpressure::StreamThrottle;
```

2. **`src/crates/orchestrator/src/streaming/broadcaster.rs`**:
```rust
use tokio::sync::broadcast;
use domain::ExecutionEvent;
use std::sync::Arc;

/// Broadcaster for execution events to multiple clients
pub struct EventBroadcaster {
    sender: broadcast::Sender<Arc<ExecutionEvent>>,
}

impl EventBroadcaster {
    /// Create new broadcaster with capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Send event to all subscribers
    pub fn send(&self, event: ExecutionEvent) -> Result<usize, broadcast::error::SendError<Arc<ExecutionEvent>>> {
        self.sender.send(Arc::new(event))
    }

    /// Create a new receiver for this broadcaster
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<ExecutionEvent>> {
        self.sender.subscribe()
    }

    /// Get current subscriber count
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Clone for EventBroadcaster {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcaster_basic() {
        let broadcaster = EventBroadcaster::new(10);
        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();

        let event = ExecutionEvent::started("task-1".to_string());
        broadcaster.send(event.clone()).unwrap();

        let received1 = rx1.recv().await.unwrap();
        let received2 = rx2.recv().await.unwrap();

        assert_eq!(received1.task_id, event.task_id);
        assert_eq!(received2.task_id, event.task_id);
    }

    #[tokio::test]
    async fn test_late_subscriber_misses_old_events() {
        let broadcaster = EventBroadcaster::new(10);

        let event1 = ExecutionEvent::started("task-1".to_string());
        broadcaster.send(event1).unwrap();

        // Subscribe after event sent
        let mut rx = broadcaster.subscribe();

        let event2 = ExecutionEvent::completed("task-1".to_string(), None);
        broadcaster.send(event2.clone()).unwrap();

        // Should only receive event2
        let received = rx.recv().await.unwrap();
        assert_eq!(received.event_type, domain::ExecutionEventType::Completed);
    }

    #[tokio::test]
    async fn test_receiver_count() {
        let broadcaster = EventBroadcaster::new(10);
        assert_eq!(broadcaster.receiver_count(), 0);

        let _rx1 = broadcaster.subscribe();
        assert_eq!(broadcaster.receiver_count(), 1);

        let _rx2 = broadcaster.subscribe();
        assert_eq!(broadcaster.receiver_count(), 2);
    }
}
```

3. **`src/crates/orchestrator/src/streaming/executor_stream.rs`**:
```rust
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Status;
use domain::ExecutionEvent;
use crate::proto::tasks;
use crate::proto_conv::execution_event_to_proto;

pub struct ExecutorStreamHandler {
    sender: mpsc::Sender<Result<tasks::ExecutionEvent, Status>>,
}

impl ExecutorStreamHandler {
    /// Create new stream handler with buffer size
    pub fn new(buffer_size: usize) -> (Self, ReceiverStream<Result<tasks::ExecutionEvent, Status>>) {
        let (sender, receiver) = mpsc::channel(buffer_size);
        let stream = ReceiverStream::new(receiver);
        (Self { sender }, stream)
    }

    /// Send event to stream
    pub async fn send(&self, event: ExecutionEvent) -> Result<(), mpsc::error::SendError<Result<tasks::ExecutionEvent, Status>>> {
        let proto_event = execution_event_to_proto(&event);
        self.sender.send(Ok(proto_event)).await
    }

    /// Send error to stream and close it
    pub async fn send_error(&self, status: Status) -> Result<(), mpsc::error::SendError<Result<tasks::ExecutionEvent, Status>>> {
        self.sender.send(Err(status)).await
    }

    /// Check if stream is closed
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

impl Clone for ExecutorStreamHandler {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_stream_handler_basic() {
        let (handler, mut stream) = ExecutorStreamHandler::new(10);

        tokio::spawn(async move {
            let event = ExecutionEvent::started("task-1".to_string());
            handler.send(event).await.unwrap();

            let event = ExecutionEvent::completed("task-1".to_string(), None);
            handler.send(event).await.unwrap();
        });

        let item1 = stream.next().await.unwrap().unwrap();
        assert_eq!(item1.event_type, tasks::ExecutionEventType::Started as i32);

        let item2 = stream.next().await.unwrap().unwrap();
        assert_eq!(item2.event_type, tasks::ExecutionEventType::Completed as i32);
    }

    #[tokio::test]
    async fn test_stream_error() {
        let (handler, mut stream) = ExecutorStreamHandler::new(10);

        tokio::spawn(async move {
            let event = ExecutionEvent::started("task-1".to_string());
            handler.send(event).await.unwrap();

            handler.send_error(Status::internal("test error")).await.unwrap();
        });

        let item1 = stream.next().await.unwrap();
        assert!(item1.is_ok());

        let item2 = stream.next().await.unwrap();
        assert!(item2.is_err());
    }

    #[tokio::test]
    async fn test_stream_closes_when_handler_dropped() {
        let (handler, mut stream) = ExecutorStreamHandler::new(10);

        drop(handler);

        assert!(stream.next().await.is_none());
    }
}
```

4. **`src/crates/orchestrator/src/streaming/backpressure.rs`**:
```rust
use std::time::Duration;
use tokio::time::sleep;

/// Throttle for controlling stream output rate
pub struct StreamThrottle {
    min_interval: Duration,
    last_sent: std::sync::Arc<tokio::sync::Mutex<Option<tokio::time::Instant>>>,
}

impl StreamThrottle {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last_sent: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Wait if necessary to maintain minimum interval
    pub async fn wait(&self) {
        let mut last = self.last_sent.lock().await;

        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            if elapsed < self.min_interval {
                sleep(self.min_interval - elapsed).await;
            }
        }

        *last = Some(tokio::time::Instant::now());
    }

    /// Check if enough time has passed since last send
    pub async fn should_send(&self) -> bool {
        let last = self.last_sent.lock().await;

        match *last {
            None => true,
            Some(last_time) => last_time.elapsed() >= self.min_interval,
        }
    }
}

impl Clone for StreamThrottle {
    fn clone(&self) -> Self {
        Self {
            min_interval: self.min_interval,
            last_sent: self.last_sent.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_throttle_first_send_immediate() {
        let throttle = StreamThrottle::new(Duration::from_millis(100));
        assert!(throttle.should_send().await);
    }

    #[tokio::test]
    async fn test_throttle_waits() {
        let throttle = StreamThrottle::new(Duration::from_millis(50));

        let start = tokio::time::Instant::now();
        throttle.wait().await; // First wait - immediate
        throttle.wait().await; // Second wait - should wait ~50ms
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(50));
        assert!(elapsed < Duration::from_millis(100)); // Not too long
    }

    #[tokio::test]
    async fn test_throttle_should_send() {
        let throttle = StreamThrottle::new(Duration::from_millis(50));

        throttle.wait().await;
        assert!(!throttle.should_send().await); // Too soon

        sleep(Duration::from_millis(60)).await;
        assert!(throttle.should_send().await); // Enough time passed
    }
}
```

5. **`src/crates/aco/src/streaming/client_stream.rs`**:
```rust
use tokio_stream::StreamExt;
use tonic::Streaming;
use crate::proto::tasks::ExecutionEvent;
use crate::error::AcoError;

/// Client-side stream wrapper with error handling
pub struct ClientStream {
    stream: Streaming<ExecutionEvent>,
}

impl ClientStream {
    pub fn new(stream: Streaming<ExecutionEvent>) -> Self {
        Self { stream }
    }

    /// Get next event from stream
    pub async fn next(&mut self) -> Result<Option<ExecutionEvent>, AcoError> {
        match self.stream.next().await {
            Some(Ok(event)) => Ok(Some(event)),
            Some(Err(status)) => Err(AcoError::from_status(status)),
            None => Ok(None),
        }
    }

    /// Consume stream and collect all events
    pub async fn collect_all(mut self) -> Result<Vec<ExecutionEvent>, AcoError> {
        let mut events = Vec::new();

        while let Some(event) = self.next().await? {
            events.push(event);
        }

        Ok(events)
    }

    /// Process events with a callback
    pub async fn for_each<F>(mut self, mut f: F) -> Result<(), AcoError>
    where
        F: FnMut(ExecutionEvent) -> Result<(), AcoError>,
    {
        while let Some(event) = self.next().await? {
            f(event)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Status;

    // Helper to create mock stream
    fn mock_stream(events: Vec<Result<ExecutionEvent, Status>>) -> Streaming<ExecutionEvent> {
        let stream = tokio_stream::iter(events);
        Streaming::new(Box::pin(stream))
    }

    #[tokio::test]
    async fn test_client_stream_next() {
        let event = ExecutionEvent {
            task_id: "task-1".to_string(),
            event_type: 1,
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: None,
            error: None,
        };

        let mut stream = ClientStream::new(mock_stream(vec![Ok(event.clone())]));

        let received = stream.next().await.unwrap().unwrap();
        assert_eq!(received.task_id, event.task_id);

        assert!(stream.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_client_stream_error() {
        let stream = mock_stream(vec![Err(Status::internal("error"))]);
        let mut client_stream = ClientStream::new(stream);

        let result = client_stream.next().await;
        assert!(result.is_err());
    }
}
```

## Update Cargo.toml

**`src/crates/orchestrator/Cargo.toml`**:
```toml
[dependencies]
tokio-stream = { version = "0.1", features = ["sync"] }
```

**`src/crates/aco/Cargo.toml`**:
```toml
[dependencies]
tokio-stream = "0.1"
```

## Unit Tests

All tests embedded in implementation files.

## Acceptance Criteria

- [ ] EventBroadcaster for multi-client streaming
- [ ] ExecutorStreamHandler for server-side streaming
- [ ] StreamThrottle for backpressure control
- [ ] ClientStream wrapper with error handling
- [ ] Tests for late subscribers
- [ ] Tests for stream closure
- [ ] Tests for error propagation
- [ ] Tests for throttling behavior
- [ ] All tests pass
- [ ] Memory-safe stream handling (no leaks)

## Complexity
**Moderate** - Async streaming patterns with proper lifecycle management

## Estimated Effort
**5-6 hours**

## Notes
- Use tokio::sync::broadcast for multiple clients
- Use tokio::sync::mpsc for single client streams
- Implement backpressure to avoid overwhelming clients
- Handle client disconnection gracefully
- Arc<ExecutionEvent> to avoid cloning large events
- Stream closes automatically when sender dropped
- Throttle helps reduce network traffic for frequent events
