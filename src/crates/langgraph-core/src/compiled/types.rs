//! Type definitions for compiled graph execution
//!
//! This module contains the core types used throughout the compiled graph execution engine.

use crate::graph::NodeId;
use crate::interrupt::InterruptWhen;
use crate::stream::StreamChunk;
use crate::error::Result;
use langgraph_checkpoint::{CheckpointConfig, CheckpointMetadata};
use futures::stream::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Events emitted during graph execution (legacy streaming API).
///
/// These events provide observability into graph execution and were used
/// with the deprecated `stream_with_modes()` API. For new code, prefer the
/// [`StreamChunk`](crate::stream::StreamChunk) API with `stream_chunks_with_modes()`.
///
/// # Event Types and Lifecycle
///
/// Events are emitted in a specific order during execution:
///
/// ```text
/// NodeStart → NodeEnd/Error → StateUpdate → ... → Complete/Interrupted
/// ```
///
/// # Variants
///
/// ## NodeStart
///
/// Emitted when a node begins execution. Useful for:
/// - Progress tracking
/// - Performance monitoring (start time)
/// - Debug logging
///
/// ## NodeEnd
///
/// Emitted when a node completes successfully. Contains:
/// - Node name
/// - Output value produced by the node
///
/// Use for:
/// - Collecting intermediate results
/// - Monitoring successful completions
/// - Updating UI with partial results
///
/// ## StateUpdate
///
/// Emitted after state changes are applied. Contains the complete
/// current state. This is the most commonly used event for:
/// - Displaying current state to users
/// - State persistence
/// - React/UI updates
///
/// ## Error
///
/// Emitted when a node fails. Contains:
/// - Node that failed
/// - Error message
///
/// Use for:
/// - Error handling
/// - Retry logic
/// - User notifications
///
/// ## Interrupted
///
/// Emitted when execution pauses for human input. Contains:
/// - Node where interrupt occurred
/// - When (before/after node execution)
/// - Current state at interrupt point
///
/// Use for:
/// - Human-in-the-loop workflows
/// - Approval gates
/// - Manual intervention points
///
/// ## Complete
///
/// Emitted when the graph finishes all execution. Contains
/// the final state. Signals successful completion.
///
/// # Migration Guide
///
/// ## Old API (Deprecated)
///
/// ```rust,ignore
/// use langgraph_core::{StateGraph, ExecutionEvent};
/// use futures::StreamExt;
///
/// let compiled = graph.compile()?;
/// let mut stream = compiled.stream_with_modes(
///     input,
///     vec!["values"],
///     None,
/// ).await?;
///
/// while let Some(event) = stream.next().await {
///     match event {
///         ExecutionEvent::StateUpdate { state } => {
///             println!("State: {:?}", state);
///         }
///         ExecutionEvent::Complete { final_state } => {
///             println!("Done: {:?}", final_state);
///         }
///         _ => {}
///     }
/// }
/// ```
///
/// ## New API (Recommended)
///
/// ```rust,ignore
/// use langgraph_core::{StateGraph, StreamMode};
/// use futures::StreamExt;
///
/// let compiled = graph.compile()?;
/// let mut stream = compiled.stream_chunks_with_modes(
///     input,
///     vec![StreamMode::Values],
///     None,
/// ).await?;
///
/// while let Some(chunk) = stream.next().await {
///     match chunk.event {
///         StreamEvent::Values { values } => {
///             println!("State: {:?}", values);
///         }
///         StreamEvent::Complete => {
///             println!("Done");
///         }
///         _ => {}
///     }
/// }
/// ```
///
/// # Example: Tracking Execution Progress
///
/// ```rust,ignore
/// use langgraph_core::ExecutionEvent;
/// use std::time::Instant;
/// use std::collections::HashMap;
///
/// let mut node_times = HashMap::new();
///
/// for event in events {
///     match event {
///         ExecutionEvent::NodeStart { node } => {
///             node_times.insert(node.clone(), Instant::now());
///             println!("Starting {}", node);
///         }
///         ExecutionEvent::NodeEnd { node, output } => {
///             if let Some(start) = node_times.get(&node) {
///                 println!("{} took {:?}", node, start.elapsed());
///             }
///         }
///         ExecutionEvent::Error { node, error } => {
///             eprintln!("Failed at {}: {}", node, error);
///             // Implement retry logic
///         }
///         _ => {}
///     }
/// }
/// ```
///
/// # See Also
///
/// - [`StreamChunk`](crate::stream::StreamChunk) - New streaming API
/// - [`StreamMode`](crate::stream::StreamMode) - Streaming modes
/// - [`stream_chunks_with_modes`](super::CompiledGraph::stream_chunks_with_modes) - New streaming method
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// Node execution started.
    ///
    /// Emitted immediately before a node's executor function is called.
    NodeStart {
        /// Name of the node that started executing
        node: NodeId
    },

    /// Node execution completed successfully.
    ///
    /// Emitted after a node's executor returns Ok(value).
    NodeEnd {
        /// Name of the node that completed
        node: NodeId,
        /// Output value produced by the node
        output: Value
    },

    /// Graph state was updated.
    ///
    /// Emitted after channel writes are applied and state changes.
    /// Contains the complete current state of all channels.
    StateUpdate {
        /// Complete current state across all channels
        state: Value
    },

    /// Node execution failed with error.
    ///
    /// Emitted when a node's executor returns Err.
    Error {
        /// Name of the node that failed
        node: NodeId,
        /// Error message describing the failure
        error: String
    },

    /// Execution was interrupted for human input.
    ///
    /// Emitted when an interrupt condition is triggered.
    /// Execution can be resumed with updated state.
    Interrupted {
        /// Node where the interrupt occurred
        node: NodeId,
        /// Whether interrupt was before or after node execution
        when: InterruptWhen,
        /// Current state at the interrupt point
        state: Value,
    },

    /// Graph execution completed successfully.
    ///
    /// Emitted when no more tasks remain and graph reaches completion.
    Complete {
        /// Final state after all execution
        final_state: Value
    },
}

/// Stream of execution events
pub type EventStream = Pin<Box<dyn Stream<Item = ExecutionEvent> + Send>>;

/// Stream of streaming chunks (new API)
pub type StreamChunkStream = Pin<Box<dyn Stream<Item = StreamChunk> + Send>>;

/// Snapshot of the graph state at a specific point in time.
///
/// Represents a complete view of the graph's execution state, including
/// what has been executed, what will execute next, and all state values.
/// Essential for time-travel debugging, state inspection, and resumption.
///
/// # Structure
///
/// Each snapshot captures:
/// - **values**: Complete state across all channels
/// - **next**: Nodes scheduled for next execution
/// - **config**: Checkpoint configuration (thread_id, checkpoint_id)
/// - **metadata**: Execution metadata (step, source, writes)
/// - **created_at**: ISO 8601 timestamp
/// - **parent_config**: Link to previous snapshot for history traversal
///
/// # Use Cases
///
/// ## Time-Travel Debugging
///
/// ```rust,ignore
/// use langgraph_core::CompiledGraph;
///
/// // Get snapshots for a thread
/// let history = compiled.get_state_history(config).await?;
///
/// // Iterate through execution history
/// while let Some(Ok(snapshot)) = history.next().await {
///     println!("Step: {:?}", snapshot.metadata.as_ref().map(|m| m.step));
///     println!("State: {:?}", snapshot.values);
///     println!("Next: {:?}", snapshot.next);
///
///     // Navigate to parent snapshot
///     if let Some(parent) = snapshot.parent_config {
///         // Load parent state...
///     }
/// }
/// ```
///
/// ## State Inspection
///
/// ```rust,ignore
/// // Get current state
/// let snapshot = compiled.get_state(config).await?;
///
/// // Check what will execute next
/// if snapshot.next.is_empty() {
///     println!("Graph complete");
/// } else {
///     println!("Next nodes: {:?}", snapshot.next);
/// }
///
/// // Inspect specific values
/// if let Some(messages) = snapshot.values.get("messages") {
///     println!("Messages: {:?}", messages);
/// }
/// ```
///
/// ## Resumption from Checkpoint
///
/// ```rust,ignore
/// // Get a historical snapshot
/// let snapshot = compiled.get_state(
///     config.with_checkpoint_id("checkpoint-123")
/// ).await?;
///
/// // Resume from that point
/// let result = compiled.invoke_with_config(
///     snapshot.values,
///     &snapshot.config,
/// ).await?;
/// ```
///
/// # Metadata Fields
///
/// The `metadata` field contains:
/// - **step**: Superstep number (0-based)
/// - **source**: What triggered this checkpoint ("input", "loop", "update")
/// - **writes**: Channel writes that created this checkpoint
/// - **thread_id**: Thread identifier for multi-tenant execution
/// - **checkpoint_ns**: Namespace for subgraph checkpoints
/// - **parent_config**: Previous checkpoint in the chain
///
/// # Example
///
/// ```rust,ignore
/// use langgraph_core::StateSnapshot;
///
/// let snapshot = StateSnapshot {
///     values: json!({"count": 5, "messages": ["Hello"]}),
///     next: vec!["process".to_string()],
///     config: CheckpointConfig::new("thread-1")
///         .with_checkpoint_id("checkpoint-456"),
///     metadata: Some(CheckpointMetadata {
///         step: Some(3),
///         source: Some("loop".to_string()),
///         writes: Some(json!({"process": {"count": 5}})),
///         ..Default::default()
///     }),
///     created_at: Some("2024-01-15T10:30:00Z".to_string()),
///     parent_config: Some(CheckpointConfig::new("thread-1")
///         .with_checkpoint_id("checkpoint-455")),
/// };
///
/// // Check if execution is complete
/// if snapshot.next.is_empty() {
///     println!("Execution complete at step {:?}",
///              snapshot.metadata.as_ref().and_then(|m| m.step));
/// }
/// ```
///
/// # See Also
///
/// - [`get_state`](super::CompiledGraph::get_state) - Retrieve current snapshot
/// - [`get_state_history`](super::CompiledGraph::get_state_history) - Get snapshot history
/// - [`update_state`](super::CompiledGraph::update_state) - Modify state
/// - [`CheckpointConfig`] - Configuration for checkpoints
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    /// Complete current state values across all channels.
    ///
    /// This is a JSON object containing all channel values at the
    /// time of the snapshot. The structure depends on your graph's
    /// state schema.
    pub values: Value,

    /// Names of nodes scheduled to execute next.
    ///
    /// Empty vec indicates graph completion. Nodes listed here
    /// will execute in the next superstep when execution resumes.
    pub next: Vec<String>,

    /// Checkpoint configuration identifying this snapshot.
    ///
    /// Contains thread_id and checkpoint_id. Used to resume
    /// execution from this exact point.
    pub config: CheckpointConfig,

    /// Execution metadata for this snapshot.
    ///
    /// Contains step number, source, writes, and other execution
    /// context. May be None for manually created snapshots.
    pub metadata: Option<CheckpointMetadata>,

    /// Timestamp when this snapshot was created (ISO 8601 format).
    ///
    /// Example: "2024-01-15T10:30:00Z"
    pub created_at: Option<String>,

    /// Configuration of the parent snapshot for history traversal.
    ///
    /// Links to the previous checkpoint in the execution chain.
    /// None for the initial checkpoint.
    pub parent_config: Option<CheckpointConfig>,
}

/// Stream of state snapshots for history traversal
pub type StateSnapshotStream = Pin<Box<dyn Stream<Item = Result<StateSnapshot>> + Send>>;
