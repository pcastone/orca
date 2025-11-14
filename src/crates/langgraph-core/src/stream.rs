//! Real-time streaming system for graph execution observability
//!
//! This module provides a flexible streaming system that emits events during graph execution,
//! enabling real-time observability, progress tracking, and interactive applications.
//!
//! # Overview
//!
//! Streaming allows you to observe graph execution as it happens, rather than waiting for
//! final results. This is essential for:
//!
//! - **User Experience**: Show progress indicators and partial results
//! - **Debugging**: Understand execution flow and intermediate states
//! - **LLM Applications**: Stream token-by-token responses from language models
//! - **Production Monitoring**: Track task execution and performance
//!
//! # Stream Modes
//!
//! rLangGraph supports 7 streaming modes, each providing different levels of detail:
//!
//! | Mode | What It Streams | Use Case | Overhead |
//! |------|----------------|----------|----------|
//! | **Values** | Complete state after each superstep | Standard workflows, full state visibility | Medium |
//! | **Updates** | Only node outputs (deltas) | Efficient state tracking | Low |
//! | **Checkpoints** | Checkpoint creation events | Recovery monitoring | Low |
//! | **Tasks** | Task start/end with results | Performance profiling | Medium |
//! | **Debug** | Checkpoints + Tasks combined | Development debugging | Medium |
//! | **Messages** | LLM message updates + chunks | Conversational AI | Low |
//! | **Tokens** | Token-level streaming | Real-time LLM responses | Low |
//! | **Custom** | Application-defined data | Custom observability | Varies |
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │             Graph Execution (Pregel Loop)                │
//! │                                                          │
//! │  Superstep 1:  Node A ──┐                              │
//! │                         │                               │
//! │  Superstep 2:  Node B ──┼─> StreamEventBuffer          │
//! │                         │       │                       │
//! │  Superstep 3:  Node C ──┘       │                       │
//! │                                  ↓                       │
//! │                         StreamMultiplexer               │
//! │                         (Filter by mode)                │
//! │                                  │                       │
//! └──────────────────────────────────┼───────────────────────┘
//!                                    │
//!                                    ↓
//!                         ┌──────────────────┐
//!                         │  Stream Channel  │
//!                         │  (mpsc::channel) │
//!                         └────────┬─────────┘
//!                                  │
//!                                  ↓
//!                           ┌─────────────┐
//!                           │ Application │
//!                           │  (Consumer) │
//!                           └─────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Streaming Complete State
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, StreamMode};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let graph = StateGraph::new();
//! // ... add nodes and edges ...
//! let compiled = graph.compile()?;
//!
//! // Stream with Values mode (complete state after each step)
//! let mut stream = compiled.stream_chunks_with_modes(
//!     serde_json::json!({"input": "data"}),
//!     vec![StreamMode::Values],
//!     None,
//! ).await?;
//!
//! while let Some(chunk) = stream.next().await {
//!     if let langgraph_core::stream::StreamEvent::Values { values } = chunk.event {
//!         println!("State: {:?}", values);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Streaming Node Updates Only
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, StreamMode};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let compiled = StateGraph::new().compile()?;
//! // Stream only node outputs (more efficient than Values)
//! let mut stream = compiled.stream_chunks_with_modes(
//!     serde_json::json!({"input": "data"}),
//!     vec![StreamMode::Updates],
//!     None,
//! ).await?;
//!
//! while let Some(chunk) = stream.next().await {
//!     if let langgraph_core::stream::StreamEvent::Updates { node, update } = chunk.event {
//!         println!("Node '{}' produced: {:?}", node, update);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Token-Level Streaming (LLMs)
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, StreamMode};
//! use langgraph_core::stream::StreamEvent;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let compiled = StateGraph::new().compile()?;
//! // Stream LLM tokens as they're generated
//! let mut stream = compiled.stream_chunks_with_modes(
//!     serde_json::json!({"prompt": "Hello"}),
//!     vec![StreamMode::Tokens],
//!     None,
//! ).await?;
//!
//! while let Some(chunk) = stream.next().await {
//!     if let StreamEvent::MessageChunk { chunk: token, node, .. } = chunk.event {
//!         print!("{}", token);  // Print token immediately
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Multiple Streaming Modes
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, StreamMode};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let compiled = StateGraph::new().compile()?;
//! // Enable multiple modes simultaneously
//! let mut stream = compiled.stream_chunks_with_modes(
//!     serde_json::json!({"input": "data"}),
//!     vec![StreamMode::Updates, StreamMode::Tasks, StreamMode::Messages],
//!     None,
//! ).await?;
//!
//! while let Some(chunk) = stream.next().await {
//!     match chunk.mode {
//!         StreamMode::Updates => println!("Update: {:?}", chunk.event),
//!         StreamMode::Tasks => println!("Task: {:?}", chunk.event),
//!         StreamMode::Messages => println!("Message: {:?}", chunk.event),
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Mode Details
//!
//! ## Values Mode
//!
//! Emits the complete graph state after each superstep. Best for applications that
//! need full state visibility but can handle larger payloads.
//!
//! **When to use**: General workflows, debugging, state inspection
//!
//! **Event**: `StreamEvent::Values { state }`
//!
//! ## Updates Mode
//!
//! Emits only the outputs from nodes that executed in each superstep. More efficient
//! than Values mode as it only sends deltas.
//!
//! **When to use**: Production apps, efficient state tracking, multi-step workflows
//!
//! **Event**: `StreamEvent::Updates { node, update }`
//!
//! ## Checkpoints Mode
//!
//! Emits events when checkpoints are created. Useful for monitoring persistence and
//! recovery capabilities.
//!
//! **When to use**: Monitoring checkpoint frequency, debugging persistence
//!
//! **Event**: `StreamEvent::Checkpoint { thread_id, namespace, checkpoint }`
//!
//! ## Tasks Mode
//!
//! Emits events when tasks start, complete, or fail. Includes task results and timing
//! information, perfect for performance profiling.
//!
//! **When to use**: Performance monitoring, error tracking, execution profiling
//!
//! **Events**: `TaskStart`, `TaskEnd`, `TaskError`
//!
//! ## Debug Mode
//!
//! Combines Checkpoints and Tasks modes for comprehensive debugging information.
//! Automatically enables both underlying modes.
//!
//! **When to use**: Development, debugging complex workflows
//!
//! **Events**: All from Checkpoints + Tasks modes
//!
//! ## Messages Mode
//!
//! Emits message updates and token chunks from LLM nodes. Supports both complete
//! messages and streaming chunks.
//!
//! **When to use**: Conversational AI, chatbots, LLM applications
//!
//! **Events**: `Message`, `MessageChunk`
//!
//! ## Tokens Mode
//!
//! Emits only token-level chunks (subset of Messages mode). Most efficient for
//! real-time LLM streaming.
//!
//! **When to use**: Real-time LLM responses, UI updates
//!
//! **Event**: `MessageChunk`
//!
//! ## Custom Mode
//!
//! Allows nodes to emit custom application-specific data using the StreamWriter.
//!
//! **When to use**: Custom metrics, application-specific events
//!
//! **Event**: `StreamEvent::Custom { data }`
//!
//! # Performance Considerations
//!
//! ## Overhead
//!
//! - **Values**: Medium overhead (serializes full state)
//! - **Updates**: Low overhead (only node outputs)
//! - **Tokens**: Very low overhead (individual strings)
//! - **Tasks**: Medium overhead (includes timing data)
//!
//! ## Backpressure
//!
//! The streaming system uses bounded channels. If consumers can't keep up:
//! - `emit()` (async) will wait until space is available
//! - `emit_sync()` will return an error if channel is full
//!
//! ## Ordering Guarantees
//!
//! - Events within a superstep are ordered by sequence number
//! - Events from different supersteps are naturally ordered
//! - Parallel node execution may produce events in any order within a superstep
//!
//! # See Also
//!
//! - [`StreamMode`] - Available streaming modes
//! - [`StreamEvent`] - Event types
//! - [`StreamConfig`] - Configuration builder
//! - [`CompiledGraph::stream`](crate::CompiledGraph::stream) - Main streaming API

use crate::graph::NodeId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use tokio::sync::mpsc;

/// Namespace for tracking hierarchical subgraph execution
///
/// Namespaces track the execution context when graphs are nested (parent-child communication).
/// Each level of nesting adds an element to the namespace vector.
///
/// # Structure
///
/// - **Root graph**: Empty vector `vec![]`
/// - **First-level subgraph**: `vec!["subgraph_name"]`
/// - **Nested subgraph**: `vec!["parent", "child"]`
///
/// # Examples
///
/// ```rust
/// use langgraph_core::stream::Namespace;
///
/// // Root graph
/// let root: Namespace = vec![];
///
/// // First-level subgraph
/// let subgraph: Namespace = vec!["agent".to_string()];
///
/// // Nested subgraph
/// let nested: Namespace = vec!["parent".to_string(), "child".to_string()];
/// ```
pub type Namespace = Vec<String>;

/// Streaming mode determining what events are emitted during graph execution
///
/// Stream modes control the granularity and type of events emitted during execution.
/// Different modes provide different tradeoffs between observability and overhead.
///
/// # Mode Selection Guide
///
/// Choose based on your use case:
///
/// - **Development/Debugging**: Use `Debug` mode for comprehensive visibility
/// - **Production Workflows**: Use `Updates` for efficient state tracking
/// - **LLM Applications**: Use `Tokens` or `Messages` for real-time responses
/// - **State Inspection**: Use `Values` when you need complete state visibility
/// - **Performance Monitoring**: Use `Tasks` to track execution timing
/// - **Checkpoint Monitoring**: Use `Checkpoints` for persistence tracking
/// - **Custom Metrics**: Use `Custom` with StreamWriter
///
/// # Examples
///
/// ## Single Mode
///
/// ```rust
/// use langgraph_core::StreamMode;
///
/// // Stream complete state after each step
/// let mode = StreamMode::Values;
/// ```
///
/// ## Multiple Modes
///
/// ```rust
/// use langgraph_core::stream::{StreamConfig, StreamMode};
///
/// let config = StreamConfig::new(StreamMode::Updates)
///     .with_mode(StreamMode::Tasks);
/// ```
///
/// # See Also
///
/// - [`StreamConfig`] - For enabling multiple modes
/// - [`StreamEvent`] - Events emitted by each mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamMode {
    /// Emit complete graph state after each superstep
    ///
    /// **Emits**: `StreamEvent::Values { state }`
    ///
    /// **Overhead**: Medium (serializes full state)
    ///
    /// **Use when**: You need full state visibility or are debugging workflows
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use langgraph_core::StreamMode;
    ///
    /// # async fn example(compiled: langgraph_core::CompiledGraph) -> Result<(), Box<dyn std::error::Error>> {
    /// let stream = compiled.stream_chunks_with_modes(
    ///     serde_json::json!({}),
    ///     vec![StreamMode::Values],
    ///     None,
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    Values,

    /// Emit only node outputs (deltas) after each node execution
    ///
    /// **Emits**: `StreamEvent::Updates { node, update }`
    ///
    /// **Overhead**: Low (only node outputs)
    ///
    /// **Use when**: Production apps need efficient state tracking
    ///
    /// This is more efficient than Values mode as it only emits what changed,
    /// not the entire state.
    Updates,

    /// Emit events when checkpoints are created
    ///
    /// **Emits**: `StreamEvent::Checkpoint { thread_id, namespace, checkpoint }`
    ///
    /// **Overhead**: Low
    ///
    /// **Use when**: Monitoring persistence, debugging checkpoint frequency
    Checkpoints,

    /// Emit events when tasks start, complete, or fail
    ///
    /// **Emits**: `TaskStart`, `TaskEnd`, `TaskError`
    ///
    /// **Overhead**: Medium (includes timing and results)
    ///
    /// **Use when**: Performance profiling, error tracking, execution monitoring
    ///
    /// Includes task IDs, node names, inputs/outputs, and error details.
    Tasks,

    /// Combined mode: Checkpoints + Tasks (for debugging)
    ///
    /// **Emits**: All events from Checkpoints and Tasks modes
    ///
    /// **Overhead**: Medium
    ///
    /// **Use when**: Development, debugging complex workflows
    ///
    /// Automatically enables both Checkpoints and Tasks modes.
    Debug,

    /// Emit LLM message updates and token chunks
    ///
    /// **Emits**: `Message`, `MessageChunk`
    ///
    /// **Overhead**: Low
    ///
    /// **Use when**: Conversational AI, chatbots, message-based agents
    ///
    /// Includes both complete messages and streaming token chunks.
    Messages,

    /// Emit only token-level chunks (subset of Messages mode)
    ///
    /// **Emits**: `MessageChunk`
    ///
    /// **Overhead**: Very low
    ///
    /// **Use when**: Real-time LLM streaming, UI updates
    ///
    /// Most efficient mode for token-by-token streaming from language models.
    Tokens,

    /// Emit custom application-defined data
    ///
    /// **Emits**: `StreamEvent::Custom { data }`
    ///
    /// **Overhead**: Varies (depends on data size)
    ///
    /// **Use when**: Custom metrics, application-specific observability
    ///
    /// Nodes can emit custom data using `StreamWriter` from the runtime.
    Custom,
}

impl Default for StreamMode {
    fn default() -> Self {
        Self::Values
    }
}

/// Events emitted during graph execution
///
/// Stream events represent different types of observability data emitted during graph
/// execution. Each event type corresponds to one or more streaming modes.
///
/// # Event Types
///
/// - **Values**: Complete state snapshots
/// - **Updates**: Node output deltas
/// - **Checkpoint**: Checkpoint persistence events
/// - **TaskStart/TaskEnd/TaskError**: Task execution lifecycle
/// - **Message/MessageChunk**: LLM conversation events
/// - **Custom**: Application-defined events
///
/// # Examples
///
/// ## Handling Different Event Types
///
/// ```rust
/// use langgraph_core::stream::StreamEvent;
///
/// fn handle_event(event: StreamEvent) {
///     match event {
///         StreamEvent::Values { state } => {
///             println!("State: {:?}", state);
///         }
///         StreamEvent::Updates { node, update } => {
///             println!("Node '{}': {:?}", node, update);
///         }
///         StreamEvent::TaskStart { task_id, node, .. } => {
///             println!("Task {} starting on node {}", task_id, node);
///         }
///         StreamEvent::MessageChunk { chunk, .. } => {
///             print!("{}", chunk);  // Stream tokens
///         }
///         _ => {}
///     }
/// }
/// ```
///
/// # See Also
///
/// - [`StreamMode`] - Modes that emit these events
/// - [`StreamChunk`] - Container for events with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum StreamEvent {
    /// Complete graph state after a superstep
    ///
    /// Emitted by [`StreamMode::Values`]. Contains the entire graph state,
    /// including all channel values.
    ///
    /// # Fields
    ///
    /// * `state` - Complete state as JSON object
    Values {
        /// Current graph state (all channels)
        state: Value,
    },

    /// Node output (delta) after execution
    ///
    /// Emitted by [`StreamMode::Updates`]. Contains only what the node produced,
    /// not the full state. More efficient than Values events.
    ///
    /// # Fields
    ///
    /// * `node` - Name of the node that executed
    /// * `update` - Output produced by the node
    Updates {
        /// Node that produced this update
        node: NodeId,
        /// Update data (node output)
        update: Value,
    },

    /// Checkpoint creation event
    ///
    /// Emitted by [`StreamMode::Checkpoints`] and [`StreamMode::Debug`].
    /// Signals that graph state was persisted.
    ///
    /// # Fields
    ///
    /// * `thread_id` - Thread identifier for this execution
    /// * `namespace` - Checkpoint namespace (subgraph context)
    /// * `checkpoint` - Serialized checkpoint data
    Checkpoint {
        /// Thread ID for this execution
        thread_id: String,
        /// Checkpoint namespace
        namespace: String,
        /// Checkpoint data
        checkpoint: Value,
    },

    /// Task execution started
    ///
    /// Emitted by [`StreamMode::Tasks`] and [`StreamMode::Debug`].
    /// Marks the beginning of a node execution.
    ///
    /// # Fields
    ///
    /// * `task_id` - Unique task identifier
    /// * `node` - Node being executed
    /// * `input` - Input state for this task
    TaskStart {
        /// Unique task identifier
        task_id: String,
        /// Node being executed
        node: NodeId,
        /// Input state to the task
        input: Value,
    },

    /// Task execution completed successfully
    ///
    /// Emitted by [`StreamMode::Tasks`] and [`StreamMode::Debug`].
    /// Contains the task result.
    ///
    /// # Fields
    ///
    /// * `task_id` - Task identifier (matches TaskStart)
    /// * `node` - Node that was executed
    /// * `output` - Result produced by the task
    TaskEnd {
        /// Task identifier
        task_id: String,
        /// Node that was executed
        node: NodeId,
        /// Output from the task
        output: Value,
    },

    /// Task execution failed with error
    ///
    /// Emitted by [`StreamMode::Tasks`] and [`StreamMode::Debug`].
    /// Contains error information.
    ///
    /// # Fields
    ///
    /// * `task_id` - Task identifier (matches TaskStart)
    /// * `node` - Node that failed
    /// * `error` - Error message
    TaskError {
        /// Task identifier
        task_id: String,
        /// Node that failed
        node: NodeId,
        /// Error message
        error: String,
    },

    /// Complete message update (for conversational AI)
    ///
    /// Emitted by [`StreamMode::Messages`]. Represents a complete message
    /// in a conversation (after streaming is complete).
    ///
    /// # Fields
    ///
    /// * `message` - Complete message content
    /// * `metadata` - Optional message metadata (model, tokens, etc.)
    Message {
        /// Message content
        message: Value,
        /// Optional metadata (model, usage, etc.)
        metadata: Option<Value>,
    },

    /// Token-level streaming chunk from LLM
    ///
    /// Emitted by [`StreamMode::Messages`] and [`StreamMode::Tokens`].
    /// Represents individual tokens or partial content from streaming LLMs.
    ///
    /// # Fields
    ///
    /// * `chunk` - Token or content fragment
    /// * `message_id` - Optional message ID this chunk belongs to
    /// * `node` - Node producing this chunk
    /// * `metadata` - Optional chunk metadata (finish_reason, etc.)
    ///
    /// # Example
    ///
    /// ```rust
    /// use langgraph_core::stream::StreamEvent;
    ///
    /// let chunk = StreamEvent::message_chunk("llm_node", "Hello");
    /// ```
    MessageChunk {
        /// Content chunk (token or fragment)
        chunk: String,
        /// Message ID this chunk belongs to
        message_id: Option<String>,
        /// Node that produced this chunk
        node: NodeId,
        /// Optional metadata (model, finish_reason, etc.)
        metadata: Option<Value>,
    },

    /// Custom application-defined data
    ///
    /// Emitted by [`StreamMode::Custom`]. Allows nodes to emit arbitrary
    /// observability data using `StreamWriter`.
    ///
    /// # Fields
    ///
    /// * `data` - Custom application data
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::stream::StreamEvent;
    /// use serde_json::json;
    ///
    /// let custom = StreamEvent::Custom {
    ///     data: json!({"metric": "requests", "value": 42}),
    /// };
    /// ```
    Custom {
        /// Custom application data
        data: Value,
    },
}

impl StreamEvent {
    /// Create a new message chunk event
    pub fn message_chunk(
        node: impl Into<NodeId>,
        chunk: impl Into<String>,
    ) -> Self {
        Self::MessageChunk {
            chunk: chunk.into(),
            message_id: None,
            node: node.into(),
            metadata: None,
        }
    }

    /// Create a new message chunk event with metadata
    pub fn message_chunk_with_metadata(
        node: impl Into<NodeId>,
        chunk: impl Into<String>,
        message_id: Option<String>,
        metadata: Option<Value>,
    ) -> Self {
        Self::MessageChunk {
            chunk: chunk.into(),
            message_id,
            node: node.into(),
            metadata,
        }
    }

    /// Create a new message event
    pub fn message(message: Value, metadata: Option<Value>) -> Self {
        Self::Message { message, metadata }
    }

    /// Check if this event matches the given stream mode
    pub fn matches_mode(&self, mode: StreamMode) -> bool {
        match (mode, self) {
            (StreamMode::Values, StreamEvent::Values { .. }) => true,
            (StreamMode::Updates, StreamEvent::Updates { .. }) => true,
            (StreamMode::Checkpoints, StreamEvent::Checkpoint { .. }) => true,
            (StreamMode::Tasks, StreamEvent::TaskStart { .. })
            | (StreamMode::Tasks, StreamEvent::TaskEnd { .. })
            | (StreamMode::Tasks, StreamEvent::TaskError { .. }) => true,
            (StreamMode::Debug, StreamEvent::Checkpoint { .. })
            | (StreamMode::Debug, StreamEvent::TaskStart { .. })
            | (StreamMode::Debug, StreamEvent::TaskEnd { .. })
            | (StreamMode::Debug, StreamEvent::TaskError { .. }) => true,
            (StreamMode::Messages, StreamEvent::Message { .. })
            | (StreamMode::Messages, StreamEvent::MessageChunk { .. }) => true,
            (StreamMode::Tokens, StreamEvent::MessageChunk { .. }) => true,
            (StreamMode::Custom, StreamEvent::Custom { .. }) => true,
            _ => false,
        }
    }

    /// Filter events by stream modes
    pub fn filter_by_modes(&self, modes: &[StreamMode]) -> bool {
        modes.iter().any(|mode| self.matches_mode(*mode))
    }
}

/// Stream chunk with namespace, mode, and ordering information
///
/// This is the unit of emission from the streaming system, containing
/// an event along with metadata about which mode produced it and where
/// in the execution graph it came from.
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Checkpoint namespace (empty for root graph)
    pub namespace: Namespace,

    /// Stream mode for this chunk
    pub mode: StreamMode,

    /// Event data
    pub event: StreamEvent,

    /// Emission sequence number (for ordering)
    pub(crate) sequence: u64,
}

impl StreamChunk {
    /// Create a new stream chunk
    pub fn new(namespace: Namespace, mode: StreamMode, event: StreamEvent, sequence: u64) -> Self {
        Self {
            namespace,
            mode,
            event,
            sequence,
        }
    }
}

/// Event buffer for ordered emission within a superstep
///
/// Events are collected during task execution and flushed in order
/// at the end of each superstep to ensure correct sequencing.
pub(crate) struct StreamEventBuffer {
    /// Buffered events waiting to be flushed
    events: Vec<StreamChunk>,

    /// Sequence counter for ordering
    sequence: u64,

    /// Current checkpoint namespace
    namespace: Namespace,
}

impl StreamEventBuffer {
    /// Create a new event buffer
    pub fn new(namespace: Namespace) -> Self {
        Self {
            events: Vec::new(),
            sequence: 0,
            namespace,
        }
    }

    /// Push an event to the buffer
    pub fn push(&mut self, mode: StreamMode, event: StreamEvent) {
        self.sequence += 1;
        self.events.push(StreamChunk {
            namespace: self.namespace.clone(),
            mode,
            event,
            sequence: self.sequence,
        });
    }

    /// Flush all buffered events, returning them in order
    pub fn flush(&mut self) -> Vec<StreamChunk> {
        std::mem::take(&mut self.events)
    }

    /// Get the current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.sequence
    }

    /// Update the namespace
    pub fn set_namespace(&mut self, namespace: Namespace) {
        self.namespace = namespace;
    }
}

/// Stream multiplexer for filtering events by mode and sending to channel
///
/// Checks each event against enabled modes before emission, allowing
/// efficient multi-mode streaming from a single execution.
pub struct StreamMultiplexer {
    /// Enabled stream modes
    pub(crate) modes: HashSet<StreamMode>,

    /// Output channel for stream chunks
    tx: mpsc::Sender<StreamChunk>,
}

impl StreamMultiplexer {
    /// Create a new stream multiplexer
    ///
    /// Debug mode is automatically expanded to include Tasks and Checkpoints modes.
    pub fn new(modes: Vec<StreamMode>, tx: mpsc::Sender<StreamChunk>) -> Self {
        let mut expanded_modes: HashSet<StreamMode> = modes.into_iter().collect();

        // Expand Debug mode to include Tasks and Checkpoints
        if expanded_modes.contains(&StreamMode::Debug) {
            expanded_modes.insert(StreamMode::Tasks);
            expanded_modes.insert(StreamMode::Checkpoints);
        }

        Self {
            modes: expanded_modes,
            tx,
        }
    }

    /// Emit a chunk asynchronously
    ///
    /// Checks if the mode is enabled before sending to avoid unnecessary work.
    pub async fn emit(&self, chunk: StreamChunk) -> Result<(), String> {
        // Check if mode is enabled
        if self.modes.contains(&chunk.mode) {
            self.tx.send(chunk).await
                .map_err(|e| format!("Stream closed: {}", e))?;
        }
        Ok(())
    }

    /// Emit a chunk synchronously (try_send)
    ///
    /// Returns an error if the channel is full or closed.
    pub fn emit_sync(&self, chunk: StreamChunk) -> Result<(), String> {
        if self.modes.contains(&chunk.mode) {
            self.tx.try_send(chunk)
                .map_err(|e| format!("Stream full or closed: {}", e))?;
        }
        Ok(())
    }

    /// Check if a mode is enabled
    pub fn has_mode(&self, mode: StreamMode) -> bool {
        self.modes.contains(&mode)
    }
}

/// Configuration for streaming
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Stream modes to enable
    pub modes: Vec<StreamMode>,

    /// Whether to include all events or just primary ones
    pub include_all: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            modes: vec![StreamMode::Values],
            include_all: false,
        }
    }
}

impl StreamConfig {
    /// Create a new stream configuration with the given mode
    pub fn new(mode: StreamMode) -> Self {
        Self {
            modes: vec![mode],
            include_all: false,
        }
    }

    /// Add a stream mode
    pub fn with_mode(mut self, mode: StreamMode) -> Self {
        if !self.modes.contains(&mode) {
            self.modes.push(mode);
        }
        self
    }

    /// Enable multiple stream modes
    pub fn with_modes(mut self, modes: Vec<StreamMode>) -> Self {
        for mode in modes {
            if !self.modes.contains(&mode) {
                self.modes.push(mode);
            }
        }
        self
    }

    /// Include all events
    pub fn include_all(mut self) -> Self {
        self.include_all = true;
        self
    }

    /// Check if an event should be included based on this configuration
    pub fn should_include(&self, event: &StreamEvent) -> bool {
        self.include_all || event.filter_by_modes(&self.modes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_mode_serialization() {
        let mode = StreamMode::Values;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"values\"");

        let mode: StreamMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, StreamMode::Values);
    }

    #[test]
    fn test_event_matches_mode() {
        let event = StreamEvent::Values {
            state: serde_json::json!({"key": "value"}),
        };
        assert!(event.matches_mode(StreamMode::Values));
        assert!(!event.matches_mode(StreamMode::Updates));
    }

    #[test]
    fn test_debug_mode_matches_multiple_events() {
        let checkpoint_event = StreamEvent::Checkpoint {
            thread_id: "thread-1".to_string(),
            namespace: "default".to_string(),
            checkpoint: serde_json::json!({}),
        };
        assert!(checkpoint_event.matches_mode(StreamMode::Debug));
        assert!(checkpoint_event.matches_mode(StreamMode::Checkpoints));

        let task_event = StreamEvent::TaskStart {
            task_id: "task-1".to_string(),
            node: "node1".to_string(),
            input: serde_json::json!({}),
        };
        assert!(task_event.matches_mode(StreamMode::Debug));
        assert!(task_event.matches_mode(StreamMode::Tasks));
    }

    #[test]
    fn test_stream_config() {
        let config = StreamConfig::new(StreamMode::Values)
            .with_mode(StreamMode::Updates);

        assert_eq!(config.modes.len(), 2);
        assert!(config.modes.contains(&StreamMode::Values));
        assert!(config.modes.contains(&StreamMode::Updates));

        let values_event = StreamEvent::Values {
            state: serde_json::json!({}),
        };
        assert!(config.should_include(&values_event));

        let task_event = StreamEvent::TaskStart {
            task_id: "task-1".to_string(),
            node: "node1".to_string(),
            input: serde_json::json!({}),
        };
        assert!(!config.should_include(&task_event));
    }

    #[test]
    fn test_stream_config_include_all() {
        let config = StreamConfig::new(StreamMode::Values).include_all();

        let task_event = StreamEvent::TaskStart {
            task_id: "task-1".to_string(),
            node: "node1".to_string(),
            input: serde_json::json!({}),
        };
        // Should include even though Tasks mode is not enabled
        assert!(config.should_include(&task_event));
    }

    #[test]
    fn test_stream_chunk_creation() {
        let event = StreamEvent::Values {
            state: serde_json::json!({"key": "value"}),
        };
        let chunk = StreamChunk::new(
            vec!["subgraph".to_string()],
            StreamMode::Values,
            event.clone(),
            42,
        );

        assert_eq!(chunk.namespace, vec!["subgraph".to_string()]);
        assert_eq!(chunk.mode, StreamMode::Values);
        assert_eq!(chunk.sequence, 42);
    }

    #[test]
    fn test_event_buffer_ordering() {
        let mut buffer = StreamEventBuffer::new(vec![]);

        let event1 = StreamEvent::Values { state: serde_json::json!({"step": 1}) };
        let event2 = StreamEvent::Values { state: serde_json::json!({"step": 2}) };
        let event3 = StreamEvent::Values { state: serde_json::json!({"step": 3}) };

        buffer.push(StreamMode::Values, event1);
        buffer.push(StreamMode::Values, event2);
        buffer.push(StreamMode::Values, event3);

        let events = buffer.flush();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[1].sequence, 2);
        assert_eq!(events[2].sequence, 3);

        // Buffer should be empty after flush
        assert_eq!(buffer.flush().len(), 0);
    }

    #[test]
    fn test_event_buffer_namespace() {
        let mut buffer = StreamEventBuffer::new(vec!["root".to_string()]);

        buffer.push(StreamMode::Values, StreamEvent::Values { state: serde_json::json!({}) });

        let events = buffer.flush();
        assert_eq!(events[0].namespace, vec!["root".to_string()]);

        // Change namespace
        buffer.set_namespace(vec!["root".to_string(), "subgraph".to_string()]);
        buffer.push(StreamMode::Values, StreamEvent::Values { state: serde_json::json!({}) });

        let events = buffer.flush();
        assert_eq!(events[0].namespace, vec!["root".to_string(), "subgraph".to_string()]);
    }

    #[tokio::test]
    async fn test_stream_multiplexer_filtering() {
        let (tx, mut rx) = mpsc::channel(10);
        let mux = StreamMultiplexer::new(vec![StreamMode::Values, StreamMode::Tasks], tx);

        // Should emit Values mode
        let chunk1 = StreamChunk::new(
            vec![],
            StreamMode::Values,
            StreamEvent::Values { state: serde_json::json!({}) },
            1,
        );
        mux.emit(chunk1).await.unwrap();

        // Should NOT emit Updates mode (not in enabled modes)
        let chunk2 = StreamChunk::new(
            vec![],
            StreamMode::Updates,
            StreamEvent::Updates {
                node: "test".to_string(),
                update: serde_json::json!({}),
            },
            2,
        );
        mux.emit(chunk2).await.unwrap();

        // Should emit Tasks mode
        let chunk3 = StreamChunk::new(
            vec![],
            StreamMode::Tasks,
            StreamEvent::TaskStart {
                task_id: "1".to_string(),
                node: "test".to_string(),
                input: serde_json::json!({}),
            },
            3,
        );
        mux.emit(chunk3).await.unwrap();

        // Close sender to signal we're done
        drop(mux);

        // Collect received chunks
        let mut received = vec![];
        while let Some(chunk) = rx.recv().await {
            received.push(chunk);
        }

        // Should have received 2 chunks (Values and Tasks, not Updates)
        assert_eq!(received.len(), 2);
        assert_eq!(received[0].mode, StreamMode::Values);
        assert_eq!(received[1].mode, StreamMode::Tasks);
    }

    #[test]
    fn test_multiplexer_has_mode() {
        let (tx, _rx) = mpsc::channel(10);
        let mux = StreamMultiplexer::new(vec![StreamMode::Values, StreamMode::Tasks], tx);

        assert!(mux.has_mode(StreamMode::Values));
        assert!(mux.has_mode(StreamMode::Tasks));
        assert!(!mux.has_mode(StreamMode::Updates));
        assert!(!mux.has_mode(StreamMode::Messages));
    }
}
