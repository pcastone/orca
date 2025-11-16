//! Streaming execution methods
//!
//! This module contains methods for streaming graph execution events.

use super::{CompiledGraph, EventStream, StreamChunkStream};
use crate::error::Result;
use crate::stream::{StreamChunk, StreamMode};
use super::types::ExecutionEvent;
use langgraph_checkpoint::CheckpointConfig;
use serde_json::Value;

impl CompiledGraph {
    /// Stream execution events with default mode (Values)
    ///
    /// # Arguments
    ///
    /// * `input` - Initial state
    ///
    /// # Returns
    ///
    /// Stream of execution events
    pub async fn stream(&self, input: Value) -> Result<EventStream> {
        self.stream_with_modes(input, vec![StreamMode::Values], None).await
    }

    /// Stream execution events with specified modes
    ///
    /// # Arguments
    ///
    /// * `input` - Initial state
    /// * `modes` - Stream modes to enable (Values, Updates, Tasks, etc.)
    /// * `config` - Optional checkpoint configuration
    ///
    /// # Returns
    ///
    /// Stream of execution events
    pub async fn stream_with_modes(
        &self,
        input: Value,
        modes: Vec<StreamMode>,
        config: Option<CheckpointConfig>,
    ) -> Result<EventStream> {
        use tokio::sync::mpsc;

        // Create bounded channel for streaming events (100 item buffer for backpressure)
        let (tx, mut rx) = mpsc::channel(100);

        // Build Pregel loop with streaming enabled
        let mut pregel_loop = self.build_pregel_loop(input)?;

        // Configure streaming
        pregel_loop = pregel_loop.with_streaming_mux(modes, tx);

        // Set checkpointer if both saver and config are available
        if let (Some(saver), Some(cfg)) = (&self.checkpoint_saver, config) {
            pregel_loop = pregel_loop.with_checkpointer(saver.clone(), cfg);
        }

        // Set interrupt configuration
        if !self.interrupt_config.interrupt_before.is_empty() {
            let nodes: std::collections::HashSet<String> =
                self.interrupt_config.interrupt_before.iter().cloned().collect();
            pregel_loop = pregel_loop.with_interrupt_before(nodes);
        }
        if !self.interrupt_config.interrupt_after.is_empty() {
            let nodes: std::collections::HashSet<String> =
                self.interrupt_config.interrupt_after.iter().cloned().collect();
            pregel_loop = pregel_loop.with_interrupt_after(nodes);
        }

        // Spawn the execution in a background task
        tokio::spawn(async move {
            let _ = pregel_loop.run().await;
        });

        // Convert StreamChunk to ExecutionEvent
        let event_stream = async_stream::stream! {
            while let Some(chunk) = rx.recv().await {
                yield convert_stream_event(chunk.event);
            }
        };

        Ok(Box::pin(event_stream))
    }

    /// Stream graph execution with fine-grained control over event types.
    ///
    /// This is the **recommended streaming API** for production use. It provides
    /// bounded channels for backpressure control and returns structured `StreamChunk`
    /// events with mode and namespace metadata.
    ///
    /// # Architecture
    ///
    /// ```text
    /// ┌──────────────┐    Bounded Channel (100)    ┌─────────────┐
    /// │ Pregel Loop  │ ──────────────────────────> │   Client    │
    /// │   Executor   │        StreamChunk           │   Consumer  │
    /// └──────────────┘                              └─────────────┘
    ///       ↓
    ///   Backpressure: Channel blocks when full
    /// ```
    ///
    /// # Stream Modes
    ///
    /// Control what events you receive:
    /// - `Values` - Complete state after each node execution
    /// - `Updates` - State changes/patches from each node
    /// - `Tasks` - Task execution details (node start/end)
    /// - `Messages` - Message-specific events (for MessageGraph)
    /// - `Debug` - Internal debugging information
    ///
    /// # Arguments
    ///
    /// * `input` - Initial state to start execution with
    /// * `modes` - Stream modes to enable (combine multiple for comprehensive monitoring)
    /// * `config` - Optional checkpoint configuration for resumption
    ///
    /// # Returns
    ///
    /// A stream of [`StreamChunk`] containing events with:
    /// - `mode` - The [`StreamMode`] that generated this event
    /// - `event` - The actual event data (state, update, or task info)
    /// - `namespace` - Hierarchical path for subgraph events
    ///
    /// # Example: Real-time Execution Monitoring
    ///
    /// ```rust,no_run
    /// use langgraph_core::{StateGraph, StreamMode};
    /// use serde_json::json;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut graph = StateGraph::new();
    /// graph.add_node("process", |state| {
    ///     Box::pin(async move {
    ///         // Simulate work
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///         Ok(json!({"processed": true}))
    ///     })
    /// });
    /// graph.add_edge("__start__", "process");
    /// graph.add_edge("process", "__end__");
    /// let compiled = graph.compile()?;
    ///
    /// // Stream with multiple modes for comprehensive monitoring
    /// let mut stream = compiled.stream_chunks_with_modes(
    ///     json!({"input": "data"}),
    ///     vec![
    ///         StreamMode::Values,   // Get full state
    ///         StreamMode::Updates,  // Get incremental changes
    ///         StreamMode::Tasks,    // Get execution progress
    ///     ],
    ///     None
    /// ).await?;
    ///
    /// // Process events as they arrive
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk.mode {
    ///         StreamMode::Tasks => {
    ///             println!("⚡ Task event: {:?}", chunk.event);
    ///         }
    ///         StreamMode::Updates => {
    ///             println!("📝 State update: {:?}", chunk.event);
    ///         }
    ///         StreamMode::Values => {
    ///             println!("📊 Full state: {:?}", chunk.event);
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example: Resumable Streaming with Checkpoints
    ///
    /// ```rust,no_run
    /// use langgraph_core::{CheckpointConfig, StreamMode};
    /// # use langgraph_core::StateGraph;
    /// # use serde_json::json;
    /// # use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let compiled = StateGraph::new().compile()?;
    /// // Configure checkpoint for resumption
    /// let config = CheckpointConfig::builder()
    ///     .with_thread_id("conversation-123")
    ///     .with_checkpoint_ns("user-456")
    ///     .build();
    ///
    /// // Stream from last checkpoint (if exists)
    /// let mut stream = compiled.stream_chunks_with_modes(
    ///     json!({"message": "Continue our discussion"}),
    ///     vec![StreamMode::Values],
    ///     Some(config)
    /// ).await?;
    ///
    /// // Process resumable stream
    /// while let Some(chunk) = stream.next().await {
    ///     // Each chunk is automatically checkpointed
    ///     println!("Resumable state: {:?}", chunk.event);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// - **Bounded Channel**: Uses a 100-item buffer to prevent memory exhaustion
    /// - **Backpressure**: Automatically slows execution if consumer can't keep up
    /// - **Selective Modes**: Only enable modes you need to reduce overhead
    /// - **Async Execution**: Runs in separate Tokio task for non-blocking operation
    ///
    /// # Migration from Legacy API
    ///
    /// ```rust,ignore
    /// // Old API (deprecated)
    /// let stream = compiled.stream_with_modes(
    ///     input,
    ///     vec!["values"],
    ///     None
    /// ).await?;
    ///
    /// // New API (recommended)
    /// let stream = compiled.stream_chunks_with_modes(
    ///     input,
    ///     vec![StreamMode::Values],
    ///     None
    /// ).await?;
    /// ```
    ///
    /// # See Also
    ///
    /// - [`StreamMode`] - Available streaming modes
    /// - [`StreamChunk`] - Structure of streamed events
    /// - [`stream_with_modes`](Self::stream_with_modes) - Legacy streaming API
    /// - [`invoke_with_config`](Self::invoke_with_config) - Non-streaming execution
    pub async fn stream_chunks_with_modes(
        &self,
        input: Value,
        modes: Vec<StreamMode>,
        config: Option<CheckpointConfig>,
    ) -> Result<StreamChunkStream> {
        use tokio::sync::mpsc;
        use tokio_stream::wrappers::ReceiverStream;

        // Create BOUNDED channel for backpressure (100 item buffer)
        let (tx, rx) = mpsc::channel::<StreamChunk>(100);

        // Build Pregel loop with streaming enabled
        let mut pregel_loop = self.build_pregel_loop(input)?;

        // Configure streaming with new API
        pregel_loop = pregel_loop.with_streaming_mux(modes, tx);

        // Set checkpointer if both saver and config are available
        if let (Some(saver), Some(cfg)) = (&self.checkpoint_saver, config) {
            pregel_loop = pregel_loop.with_checkpointer(saver.clone(), cfg);
        }

        // Set interrupt configuration
        if !self.interrupt_config.interrupt_before.is_empty() {
            let nodes: std::collections::HashSet<String> =
                self.interrupt_config.interrupt_before.iter().cloned().collect();
            pregel_loop = pregel_loop.with_interrupt_before(nodes);
        }
        if !self.interrupt_config.interrupt_after.is_empty() {
            let nodes: std::collections::HashSet<String> =
                self.interrupt_config.interrupt_after.iter().cloned().collect();
            pregel_loop = pregel_loop.with_interrupt_after(nodes);
        }

        // Spawn the execution in a background task
        tokio::spawn(async move {
            if let Err(e) = pregel_loop.run().await {
                tracing::error!(error = %e, "Streaming execution failed");
            }
        });

        // Return stream of chunks directly
        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}

/// Convert old-style StreamEvent to ExecutionEvent (for legacy API compatibility)
fn convert_stream_event(event: crate::stream::StreamEvent) -> ExecutionEvent {
    use crate::stream::StreamEvent;

    match event {
        StreamEvent::Values { state } => ExecutionEvent::StateUpdate { state },
        StreamEvent::Updates { node: _, update } => ExecutionEvent::StateUpdate { state: update },
        _ => ExecutionEvent::StateUpdate { state: serde_json::json!({}) },
    }
}
