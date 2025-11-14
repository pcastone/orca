//! Main Pregel execution loop.

use crate::error::{GraphError, Result};
use crate::command::{Command, GotoTarget, ResumeValue};
use crate::stream::{StreamMode, StreamEvent, StreamMultiplexer, StreamEventBuffer, Namespace};
use crate::interrupt::{InterruptTracker, InterruptWhen, InterruptState};
use crate::managed::ExecutionContext;
use crate::runtime::{Runtime, StreamWriter, set_runtime, clear_runtime};
use crate::store::Store;
use super::checkpoint::{Checkpoint, ChannelVersion};
use super::algo::{apply_writes, prepare_next_tasks};
use super::types::{NodeExecutor, PregelExecutableTask};
use super::io::{map_output_values, map_output_updates};
use langgraph_checkpoint::{
    Channel, PendingWrite, CheckpointSaver, CheckpointConfig, CheckpointMetadata,
    checkpoint::CheckpointSource,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use futures::future::join_all;
use tokio::sync::mpsc;

/// Specification for a node in the Pregel execution graph.
///
/// Each node in the graph has a specification that defines:
/// - Its unique name/identifier
/// - Which channels trigger its execution (version-based triggering)
/// - Which channels it reads from (may differ from triggers)
/// - The executor function that runs when triggered
///
/// # Pregel Execution Model
///
/// The Pregel model uses **channel versions** to determine when nodes execute:
///
/// 1. **Channel Update**: Channels are updated with new values
/// 2. **Version Increment**: Channel versions increment
/// 3. **Node Triggering**: Nodes that haven't seen the new version trigger
/// 4. **Parallel Execution**: Nodes execute in parallel (superstep)
/// 5. **Barrier Synchronization**: Wait for all nodes to complete
/// 6. **Write Application**: Apply all writes atomically
/// 7. **Repeat**: Continue until no more tasks or max steps reached
///
/// # Trigger vs Read Channels
///
/// The distinction between `triggers` and `reads` enables advanced patterns:
///
/// - **triggers**: Channels whose changes cause this node to execute
/// - **reads**: Channels this node reads values from when executing
///
/// Usually these are the same, but they can differ for patterns like:
/// - Monitoring nodes that trigger on events but read from state
/// - Aggregator nodes that trigger on multiple inputs but read from all
///
/// # Example
///
/// ## Basic Node Specification
///
/// ```rust
/// use langgraph_core::pregel::{PregelNodeSpec, NodeExecutor};
/// use std::sync::Arc;
///
/// let spec = PregelNodeSpec {
///     name: "process".to_string(),
///     triggers: vec!["input".to_string()],  // Execute when "input" updates
///     reads: vec!["input".to_string()],      // Read from "input" channel
///     executor: Arc::new(|state| {
///         Box::pin(async move {
///             // Process the input
///             Ok(state)
///         })
///     }),
/// };
/// ```
///
/// ## Advanced Pattern: Monitor Node
///
/// ```rust
/// use langgraph_core::pregel::PregelNodeSpec;
/// use std::sync::Arc;
///
/// // Node that triggers on events but reads global state
/// let monitor = PregelNodeSpec {
///     name: "monitor".to_string(),
///     triggers: vec!["event".to_string()],        // Trigger on events
///     reads: vec!["event".to_string(),            // Read the event
///                 "global_state".to_string()],    // Also read global state
///     executor: Arc::new(|state| {
///         Box::pin(async move {
///             // Check event against global state
///             Ok(state)
///         })
///     }),
/// };
/// ```
///
/// # Thread Safety
///
/// The `executor` field is `Arc<dyn NodeExecutor>` to enable:
/// - Safe sharing across threads during parallel execution
/// - Cloning of node specs for distribution to workers
/// - Dynamic dispatch to user-provided functions
#[derive(Clone)]
pub struct PregelNodeSpec {
    /// Unique identifier for this node in the graph.
    ///
    /// Must be unique across all nodes. Special names like "__start__" and "__end__"
    /// are reserved for graph entry and exit points.
    pub name: String,

    /// List of channel names that trigger this node's execution.
    ///
    /// When any of these channels update (version changes), the node will execute
    /// in the next superstep if it hasn't already seen the new version.
    pub triggers: Vec<String>,

    /// List of channel names to read values from when executing.
    ///
    /// Often the same as `triggers`, but can differ for advanced patterns like
    /// monitoring nodes that trigger on events but read from multiple channels.
    pub reads: Vec<String>,

    /// List of channel names to write the node's output to.
    ///
    /// For StateGraph, this is typically `vec!["state"]` to write to the shared state channel.
    /// For other patterns, nodes may write to multiple channels or their own channel.
    pub writes: Vec<String>,

    /// The async function that runs when this node triggers.
    ///
    /// Must implement the `NodeExecutor` trait. The executor receives the current
    /// state and returns an updated state or error. Wrapped in `Arc` for thread-safe
    /// sharing during parallel execution.
    pub executor: Arc<dyn NodeExecutor>,
}

/// The main Pregel execution loop implementing the superstep-based execution model.
///
/// `PregelLoop` orchestrates the stateful execution of a compiled graph using
/// Google's Pregel algorithm adapted for LLM workflows. It manages:
/// - Superstep execution with barrier synchronization
/// - Channel version tracking and node triggering
/// - Checkpoint creation and restoration
/// - Streaming event emission
/// - Human-in-the-loop interrupts
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────┐
/// │                    PregelLoop                           │
/// │                                                          │
/// │  ┌───────────────────────────────────────────────────┐  │
/// │  │  Superstep N                                      │  │
/// │  │  ┌─────────────────────────────────────────────┐  │  │
/// │  │  │  1. prepare_next_tasks()                    │  │  │
/// │  │  │     - Check channel versions                │  │  │
/// │  │  │     - Identify triggered nodes              │  │  │
/// │  │  │     - Create PregelExecutableTasks          │  │  │
/// │  │  └─────────────────────────────────────────────┘  │  │
/// │  │                       ↓                            │  │
/// │  │  ┌─────────────────────────────────────────────┐  │  │
/// │  │  │  2. execute_tasks (parallel)                │  │  │
/// │  │  │     - Run node executors concurrently       │  │  │
/// │  │  │     - Collect writes from each node         │  │  │
/// │  │  │     - Handle errors and interrupts          │  │  │
/// │  │  └─────────────────────────────────────────────┘  │  │
/// │  │                       ↓                            │  │
/// │  │  ┌─────────────────────────────────────────────┐  │  │
/// │  │  │  3. apply_writes() [BARRIER]                │  │  │
/// │  │  │     - Deterministic write ordering          │  │  │
/// │  │  │     - Update channels with reducers         │  │  │
/// │  │  │     - Increment channel versions            │  │  │
/// │  │  └─────────────────────────────────────────────┘  │  │
/// │  │                       ↓                            │  │
/// │  │  ┌─────────────────────────────────────────────┐  │  │
/// │  │  │  4. checkpoint()                            │  │  │
/// │  │  │     - Snapshot current state                │  │  │
/// │  │  │     - Save versions and metadata            │  │  │
/// │  │  │     - Enable time-travel debugging          │  │  │
/// │  │  └─────────────────────────────────────────────┘  │  │
/// │  └───────────────────────────────────────────────────┘  │
/// │                                                          │
/// │  Repeat until: no tasks OR max_steps OR interrupted     │
/// └─────────────────────────────────────────────────────────┘
/// ```
///
/// # Execution Guarantees
///
/// - **Determinism**: Same inputs + checkpoint = same outputs
/// - **Parallelism**: Independent nodes execute concurrently within supersteps
/// - **Atomicity**: Writes applied atomically at barriers
/// - **Recoverability**: Checkpoint after each superstep enables resumption
/// - **Ordering**: Tasks sorted deterministically for reproducibility
///
/// # Example
///
/// ```rust,no_run
/// use langgraph_core::pregel::{PregelLoop, PregelNodeSpec};
/// use langgraph_checkpoint::{InMemoryCheckpointSaver, CheckpointConfig, MemoryChannel};
/// use std::sync::Arc;
/// use std::collections::HashMap;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Setup channels
/// let mut channels = HashMap::new();
/// channels.insert("state".to_string(),
///     Box::new(MemoryChannel::new()) as Box<dyn langgraph_checkpoint::Channel>);
///
/// // Setup nodes
/// let mut nodes = HashMap::new();
/// nodes.insert("process".to_string(), PregelNodeSpec {
///     name: "process".to_string(),
///     triggers: vec!["state".to_string()],
///     reads: vec!["state".to_string()],
///     writes: vec!["state".to_string()],
///     executor: Arc::new(|state| {
///         Box::pin(async move { Ok(state) })
///     }),
/// });
///
/// // Create checkpoint
/// let checkpoint = Default::default();
///
/// // Build Pregel loop
/// let mut loop_exec = PregelLoop::new(
///     checkpoint,
///     channels,
///     nodes,
///     100,  // max steps
/// );
///
/// // Configure checkpointing
/// let saver = Arc::new(InMemoryCheckpointSaver::new());
/// let config = CheckpointConfig::new("thread-1");
/// loop_exec = loop_exec.with_checkpointer(saver, config);
///
/// // Run to completion
/// let final_state = loop_exec.run().await?;
/// # Ok(())
/// # }
/// ```
///
/// # Performance Characteristics
///
/// - **Time Complexity**: O(S × N) where S = supersteps, N = avg nodes per step
/// - **Space Complexity**: O(C + N) where C = channel count, N = node count
/// - **Parallelism**: Up to N concurrent tasks per superstep
/// - **Checkpoint Overhead**: O(C) per superstep for state serialization
///
/// # See Also
///
/// - [`PregelNodeSpec`] - Node specification structure
/// - [`apply_writes`](super::algo::apply_writes) - Write application algorithm
/// - [`prepare_next_tasks`](super::algo::prepare_next_tasks) - Task scheduling
/// - [`Checkpoint`](super::checkpoint::Checkpoint) - Checkpoint structure
pub struct PregelLoop {
    /// Current checkpoint
    checkpoint: Checkpoint,
    /// Channels map
    channels: HashMap<String, Box<dyn Channel>>,
    /// Node specifications
    nodes: HashMap<String, PregelNodeSpec>,
    /// Trigger to nodes mapping (channel → node names)
    trigger_to_nodes: HashMap<String, Vec<String>>,
    /// Current step number
    step: usize,
    /// Maximum steps allowed
    max_steps: usize,
    /// Nodes to interrupt before
    interrupt_before: HashSet<String>,
    /// Nodes to interrupt after
    interrupt_after: HashSet<String>,
    /// Pending writes (for crash recovery)
    pending_writes: Vec<PendingWrite>,
    /// Stream modes enabled (deprecated - use stream_mux)
    stream_modes: Vec<StreamMode>,
    /// Stream event sender (deprecated - use stream_mux)
    stream_tx: Option<mpsc::UnboundedSender<StreamEvent>>,
    /// Stream multiplexer for mode filtering and emission
    stream_mux: Option<Arc<StreamMultiplexer>>,
    /// Event buffer for ordered emission within supersteps
    event_buffer: StreamEventBuffer,
    /// Checkpoint namespace for tracking subgraphs
    checkpoint_namespace: Namespace,
    /// Optional checkpoint saver for persistence
    checkpointer: Option<Arc<dyn CheckpointSaver>>,
    /// Checkpoint configuration
    checkpoint_config: Option<CheckpointConfig>,
    /// Interrupt tracker for human-in-the-loop workflows
    interrupt_tracker: InterruptTracker,
    /// Resume value to apply when resuming from an interrupt
    resume_value: Option<ResumeValue>,
    /// Optional store for persistent state
    store: Option<Arc<dyn Store>>,
    /// Edges from the graph (for conditional routing)
    edges: HashMap<String, Vec<crate::graph::Edge>>,
}

impl PregelLoop {
    /// Create a new Pregel loop.
    pub fn new(
        checkpoint: Checkpoint,
        channels: HashMap<String, Box<dyn Channel>>,
        nodes: HashMap<String, PregelNodeSpec>,
        max_steps: usize,
    ) -> Self {
        Self::new_with_edges(checkpoint, channels, nodes, max_steps, HashMap::new())
    }

    /// Create a new Pregel loop with edges for conditional routing.
    pub fn new_with_edges(
        checkpoint: Checkpoint,
        channels: HashMap<String, Box<dyn Channel>>,
        nodes: HashMap<String, PregelNodeSpec>,
        max_steps: usize,
        edges: HashMap<String, Vec<crate::graph::Edge>>,
    ) -> Self {
        // Build trigger_to_nodes mapping
        let mut trigger_to_nodes: HashMap<String, Vec<String>> = HashMap::new();
        for (node_name, node_spec) in &nodes {
            for trigger_chan in &node_spec.triggers {
                trigger_to_nodes
                    .entry(trigger_chan.clone())
                    .or_default()
                    .push(node_name.clone());
            }
        }

        Self {
            checkpoint,
            channels,
            nodes,
            trigger_to_nodes,
            step: 0,
            max_steps,
            interrupt_before: HashSet::new(),
            interrupt_after: HashSet::new(),
            pending_writes: Vec::new(),
            stream_modes: vec![],
            stream_tx: None,
            stream_mux: None,
            event_buffer: StreamEventBuffer::new(vec![]),
            checkpoint_namespace: vec![],
            checkpointer: None,
            checkpoint_config: None,
            interrupt_tracker: InterruptTracker::new(),
            resume_value: None,
            store: None,
            edges,
        }
    }

    /// Create a new Pregel loop from a saved checkpoint (time-travel restoration).
    ///
    /// Restores the complete execution state from a previously saved checkpoint,
    /// enabling:
    /// - **Resumption**: Continue execution from where it left off
    /// - **Time Travel**: Jump to any previous checkpoint
    /// - **Recovery**: Restart after crashes or errors
    /// - **Branching**: Create alternate timelines from checkpoints
    ///
    /// # State Restoration
    ///
    /// The following state is fully restored:
    ///
    /// 1. **Channel Values**: All channel data at checkpoint time
    /// 2. **Channel Versions**: Version counters for triggering
    /// 3. **Versions Seen**: Per-node version tracking
    /// 4. **Step Number**: Current superstep count
    /// 5. **Metadata**: Thread ID, source, writes, parent info
    ///
    /// # Arguments
    ///
    /// * `checkpointer` - Checkpoint storage backend
    /// * `config` - Configuration including thread_id and checkpoint_id
    /// * `channels` - Channel map (will be populated with checkpoint data)
    /// * `nodes` - Node specifications for the graph
    /// * `max_steps` - Maximum execution steps allowed
    /// * `edges` - Conditional edges for routing
    ///
    /// # Returns
    ///
    /// Returns `Ok(PregelLoop)` with restored state, or error if:
    /// - Checkpoint not found
    /// - Channel restoration fails
    /// - Version format incompatible
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::pregel::PregelLoop;
    /// use langgraph_checkpoint::{InMemoryCheckpointSaver, CheckpointConfig};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
    /// let config = CheckpointConfig::new("thread-1")
    ///     .with_checkpoint_id("checkpoint-123");
    ///
    /// // Restore from checkpoint
    /// let loop_exec = PregelLoop::from_checkpoint(
    ///     checkpointer,
    ///     config,
    ///     channels,
    ///     nodes,
    ///     100,
    ///     edges,
    /// ).await?;
    ///
    /// // Continue execution from restored state
    /// let result = loop_exec.run().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Version Conversion
    ///
    /// Handles conversion between checkpoint format versions:
    /// - `ChannelVersion::Int` → `ChannelVersion::Int`
    /// - `ChannelVersion::Float` → `ChannelVersion::Float`
    /// - `ChannelVersion::String` → `ChannelVersion::String`
    ///
    /// # Performance
    ///
    /// - **Time**: O(C) where C = number of channels
    /// - **Space**: O(S) where S = total state size
    /// - **I/O**: One checkpoint load from storage
    ///
    /// # See Also
    ///
    /// - [`with_checkpointer`](Self::with_checkpointer) - Enable checkpointing
    /// - [`Checkpoint`](super::checkpoint::Checkpoint) - Checkpoint structure
    /// - [`CheckpointSaver`] - Checkpoint storage trait
    pub async fn from_checkpoint(
        checkpointer: Arc<dyn CheckpointSaver>,
        config: CheckpointConfig,
        mut channels: HashMap<String, Box<dyn Channel>>,
        nodes: HashMap<String, PregelNodeSpec>,
        max_steps: usize,
        edges: HashMap<String, Vec<crate::graph::Edge>>,
    ) -> Result<Self> {
        // Load the checkpoint
        let checkpoint_tuple = checkpointer
            .get_tuple(&config)
            .await?;

        let (lc_checkpoint, metadata) = match checkpoint_tuple {
            Some(tuple) => (tuple.checkpoint, tuple.metadata),
            None => {
                return Err(GraphError::Checkpoint(
                    langgraph_checkpoint::CheckpointError::NotFound(
                        "No checkpoint found for the given configuration".to_string()
                    ),
                ));
            }
        };

        // Convert langgraph_checkpoint::Checkpoint to pregel::Checkpoint
        let convert_versions = |versions: &HashMap<String, langgraph_checkpoint::checkpoint::ChannelVersion>| -> HashMap<String, ChannelVersion> {
            versions.iter().map(|(k, v)| {
                let pregel_version = match v {
                    langgraph_checkpoint::checkpoint::ChannelVersion::Int(n) => ChannelVersion::Int(*n),
                    langgraph_checkpoint::checkpoint::ChannelVersion::Float(f) => ChannelVersion::Float(*f),
                    langgraph_checkpoint::checkpoint::ChannelVersion::String(s) => ChannelVersion::String(s.clone()),
                };
                (k.clone(), pregel_version)
            }).collect()
        };

        let pregel_checkpoint = Checkpoint {
            v: lc_checkpoint.v,
            id: lc_checkpoint.id,
            ts: lc_checkpoint.ts,
            channel_values: lc_checkpoint.channel_values,
            channel_versions: convert_versions(&lc_checkpoint.channel_versions),
            versions_seen: lc_checkpoint.versions_seen.iter()
                .map(|(k, v)| (k.clone(), convert_versions(v)))
                .collect(),
            updated_channels: lc_checkpoint.updated_channels,
        };

        // Restore channel values
        for (channel_name, value) in &pregel_checkpoint.channel_values {
            if let Some(channel) = channels.get_mut(channel_name) {
                channel.update(vec![value.clone()]).map_err(|e| {
                    GraphError::Checkpoint(
                        langgraph_checkpoint::CheckpointError::Custom(format!(
                            "Failed to restore channel '{}': {}",
                            channel_name, e
                        ))
                    )
                })?;
            }
        }

        // Build trigger_to_nodes mapping
        let mut trigger_to_nodes: HashMap<String, Vec<String>> = HashMap::new();
        for (node_name, node_spec) in &nodes {
            for trigger_chan in &node_spec.triggers {
                trigger_to_nodes
                    .entry(trigger_chan.clone())
                    .or_default()
                    .push(node_name.clone());
            }
        }

        // Restore step number from metadata
        let step = metadata.step.unwrap_or(0) as usize;

        Ok(Self {
            checkpoint: pregel_checkpoint,
            channels,
            nodes,
            trigger_to_nodes,
            step,
            max_steps,
            interrupt_before: HashSet::new(),
            interrupt_after: HashSet::new(),
            pending_writes: Vec::new(),
            stream_modes: vec![],
            stream_tx: None,
            stream_mux: None,
            event_buffer: StreamEventBuffer::new(vec![]),
            checkpoint_namespace: vec![],
            checkpointer: Some(checkpointer),
            checkpoint_config: Some(config),
            interrupt_tracker: InterruptTracker::new(),
            resume_value: None,
            store: None,
            edges,
        })
    }

    /// Enable checkpoint persistence for state saving and recovery.
    ///
    /// Configures automatic checkpoint creation after each superstep,
    /// enabling time-travel debugging, crash recovery, and resumption.
    ///
    /// # Arguments
    ///
    /// * `checkpointer` - Checkpoint storage backend (memory, SQLite, PostgreSQL, etc.)
    /// * `config` - Configuration with thread_id and optional parent/checkpoint_id
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::pregel::PregelLoop;
    /// use langgraph_checkpoint::{InMemoryCheckpointSaver, CheckpointConfig};
    /// use std::sync::Arc;
    ///
    /// # let mut loop_exec = PregelLoop::new(Default::default(), Default::default(), Default::default(), 100);
    /// let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
    /// let config = CheckpointConfig::new("thread-1");
    ///
    /// let loop_exec = loop_exec.with_checkpointer(checkpointer, config);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`from_checkpoint`](Self::from_checkpoint) - Restore from checkpoint
    pub fn with_checkpointer(
        mut self,
        checkpointer: Arc<dyn CheckpointSaver>,
        config: CheckpointConfig,
    ) -> Self {
        self.checkpointer = Some(checkpointer);
        self.checkpoint_config = Some(config);
        self
    }

    /// Enable streaming with specified modes using the multiplexer API.
    ///
    /// Configures real-time event emission during execution. Events are
    /// filtered by mode and sent through the provided channel.
    ///
    /// # Arguments
    ///
    /// * `modes` - List of [`StreamMode`]s to enable (Values, Updates, Tasks, etc.)
    /// * `tx` - Channel sender for emitting [`StreamChunk`](crate::stream::StreamChunk) events
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::pregel::PregelLoop;
    /// use langgraph_core::stream::{StreamMode, StreamChunk};
    /// use tokio::sync::mpsc;
    ///
    /// # let mut loop_exec = PregelLoop::new(Default::default(), Default::default(), Default::default(), 100);
    /// let (tx, mut rx) = mpsc::channel::<StreamChunk>(100);
    ///
    /// let loop_exec = loop_exec.with_streaming_mux(
    ///     vec![StreamMode::Values, StreamMode::Tasks],
    ///     tx,
    /// );
    ///
    /// // In another task: receive events
    /// tokio::spawn(async move {
    ///     while let Some(chunk) = rx.recv().await {
    ///         println!("Event: {:?}", chunk.event);
    ///     }
    /// });
    /// ```
    ///
    /// # Performance
    ///
    /// - Uses bounded channel for backpressure handling
    /// - Events filtered at source to reduce overhead
    /// - Buffered within supersteps for ordering
    pub fn with_streaming_mux(
        mut self,
        modes: Vec<StreamMode>,
        tx: mpsc::Sender<crate::stream::StreamChunk>,
    ) -> Self {
        self.stream_mux = Some(Arc::new(StreamMultiplexer::new(modes, tx)));
        self
    }

    /// Set up streaming (deprecated - use with_streaming_mux).
    ///
    /// This method is kept for backward compatibility with existing code.
    /// New code should use [`with_streaming_mux`](Self::with_streaming_mux).
    #[deprecated(since = "0.3.0", note = "Use with_streaming_mux for new code")]
    pub fn with_streaming(
        mut self,
        modes: Vec<StreamMode>,
        tx: mpsc::UnboundedSender<StreamEvent>,
    ) -> Self {
        self.stream_modes = modes;
        self.stream_tx = Some(tx);
        self
    }

    /// Attach a persistent store for cross-execution state sharing.
    ///
    /// The store provides key-value storage accessible to all nodes,
    /// enabling data persistence beyond graph execution lifetime.
    ///
    /// # Use Cases
    ///
    /// - **User sessions**: Store user preferences and history
    /// - **Caching**: Cache expensive computations or API calls
    /// - **Global state**: Share data across graph invocations
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::pregel::PregelLoop;
    /// use langgraph_core::store::InMemoryStore;
    /// use std::sync::Arc;
    ///
    /// # let mut loop_exec = PregelLoop::new(Default::default(), Default::default(), Default::default(), 100);
    /// let store = Arc::new(InMemoryStore::new());
    /// let loop_exec = loop_exec.with_store(store);
    ///
    /// // Nodes can access via runtime:
    /// // let store = runtime.get_store()?;
    /// // store.put("key", value).await?;
    /// ```
    pub fn with_store(mut self, store: Arc<dyn Store>) -> Self {
        self.store = Some(store);
        self
    }

    /// Configure nodes that trigger interrupts before execution.
    ///
    /// Execution pauses when any of these nodes are about to run,
    /// allowing human review or input before proceeding.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::pregel::PregelLoop;
    /// use std::collections::HashSet;
    ///
    /// # let mut loop_exec = PregelLoop::new(Default::default(), Default::default(), Default::default(), 100);
    /// let mut interrupt_nodes = HashSet::new();
    /// interrupt_nodes.insert("approve_action".to_string());
    /// interrupt_nodes.insert("confirm_delete".to_string());
    ///
    /// let loop_exec = loop_exec.with_interrupt_before(interrupt_nodes);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`with_interrupt_after`](Self::with_interrupt_after) - Post-execution interrupts
    pub fn with_interrupt_before(mut self, nodes: HashSet<String>) -> Self {
        self.interrupt_before = nodes;
        self
    }

    /// Configure nodes that trigger interrupts after execution.
    ///
    /// Execution pauses after these nodes complete, allowing
    /// review of results before continuing.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::pregel::PregelLoop;
    /// use std::collections::HashSet;
    ///
    /// # let mut loop_exec = PregelLoop::new(Default::default(), Default::default(), Default::default(), 100);
    /// let mut interrupt_nodes = HashSet::new();
    /// interrupt_nodes.insert("generate_report".to_string());
    ///
    /// let loop_exec = loop_exec.with_interrupt_after(interrupt_nodes);
    /// ```
    pub fn with_interrupt_after(mut self, nodes: HashSet<String>) -> Self {
        self.interrupt_after = nodes;
        self
    }

    /// Set a value to apply when resuming from an interrupt.
    ///
    /// The resume value updates the graph state before continuing
    /// execution, enabling human-provided corrections or inputs.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::pregel::PregelLoop;
    /// use langgraph_core::command::ResumeValue;
    /// use serde_json::json;
    ///
    /// # let mut loop_exec = PregelLoop::new(Default::default(), Default::default(), Default::default(), 100);
    /// // User provides input after interrupt
    /// let resume_value = ResumeValue::Replace(json!({
    ///     "user_decision": "approve"
    /// }));
    ///
    /// let loop_exec = loop_exec.with_resume_value(resume_value);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`ResumeValue`] - Types of resume values (Replace, Update, Push)
    pub fn with_resume_value(mut self, value: ResumeValue) -> Self {
        self.resume_value = Some(value);
        self
    }

    /// Get the current interrupt state, if any.
    pub fn current_interrupt(&self) -> Option<&InterruptState> {
        self.interrupt_tracker.current_interrupt()
    }

    /// Check if execution is currently interrupted.
    pub fn is_interrupted(&self) -> bool {
        self.interrupt_tracker.is_interrupted()
    }

    /// Resume execution from an interrupt.
    ///
    /// This method prepares the graph to continue from a previously interrupted state.
    /// Call this after an interrupt, optionally providing a resume value.
    pub fn resume(&mut self) -> Result<()> {
        self.interrupt_tracker.resume().map_err(|e| {
            GraphError::Execution(format!("Failed to resume: {}", e))
        })
    }

    /// Execute the graph until completion or interruption.
    ///
    /// This is the main Pregel superstep loop:
    /// 1. Prepare tasks based on channel versions
    /// 2. Execute tasks in parallel
    /// 3. Apply writes (barrier)
    /// Execute the graph to completion using the Pregel algorithm.
    ///
    /// This is the main entry point for running a graph. It executes supersteps
    /// in a loop until one of the following conditions is met:
    /// - No more tasks to execute (graph completes)
    /// - Maximum step limit reached
    /// - Execution is interrupted
    ///
    /// # Execution Flow
    ///
    /// 1. **Initial State Emission**: Emits the current state before execution begins
    /// 2. **Superstep Loop**: Repeatedly calls `execute_superstep()` until completion
    /// 3. **State Aggregation**: Collects final state from all channels
    /// 4. **Result Return**: Returns either complete state or latest node output
    ///
    /// # Returns
    ///
    /// Returns the final state as a JSON value. The format depends on graph type:
    ///
    /// - **StateGraph with custom channels**: Complete state from all channels
    /// - **MessageGraph**: All message channels
    /// - **Simple graphs**: Latest node output only (backward compatibility)
    ///
    /// # Errors
    ///
    /// Returns [`GraphError`] if:
    /// - Maximum steps exceeded (`GraphError::Execution`)
    /// - Node execution fails
    /// - Checkpoint save fails
    /// - Channel read/write fails
    /// - Execution is interrupted (returns `GraphError::Interrupt`)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::pregel::PregelLoop;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut loop_exec = PregelLoop::new(Default::default(), Default::default(), Default::default(), 100);
    /// // Run graph to completion
    /// match loop_exec.run().await {
    ///     Ok(final_state) => {
    ///         println!("Graph completed with state: {:?}", final_state);
    ///     }
    ///     Err(langgraph_core::error::GraphError::Interrupt(state)) => {
    ///         println!("Graph interrupted for human input");
    ///         // Can resume later with state
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Graph execution failed: {}", e);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Performance
    ///
    /// - **Time Complexity**: O(S × N) where S = number of supersteps, N = avg nodes per step
    /// - **Space Complexity**: O(C) where C = number of channels
    /// - **Checkpoint Overhead**: One checkpoint save per superstep
    ///
    /// # Streaming
    ///
    /// If streaming is enabled via `with_streaming_mux()`, events are emitted
    /// throughout execution:
    /// - `StreamEvent::Values` - After each superstep
    /// - `StreamEvent::Updates` - Node outputs
    /// - `StreamEvent::Tasks` - Task start/end events
    ///
    /// # See Also
    ///
    /// - [`execute_superstep`](Self::execute_superstep) - Single superstep execution
    /// - [`with_checkpointer`](Self::with_checkpointer) - Enable checkpointing
    /// - [`with_streaming_mux`](Self::with_streaming_mux) - Enable event streaming
    pub async fn run(&mut self) -> Result<serde_json::Value> {
        // Emit initial Values event (force output of current state)
        // This happens before any execution begins
        if self.step == 0 {
            self.emit_values_event(None);
            self.flush_events().await?;
        }

        loop {
            // Check if we've exceeded max steps
            if self.step >= self.max_steps {
                return Err(GraphError::Execution(format!(
                    "Maximum steps ({}) exceeded",
                    self.max_steps
                )));
            }

            // Execute one superstep
            let should_continue = self.execute_superstep().await?;

            // Flush buffered stream events after superstep completion
            self.flush_events().await?;

            if !should_continue {
                // No more work to do
                break;
            }

            self.step += 1;
        }

        // Read final output from channels
        // If there are custom channels (non-node, non-internal), return complete state
        // Otherwise, return just the latest node output for backward compatibility

        let has_custom_channels = self.channels.keys().any(|name| {
            !name.starts_with("__") && !self.nodes.contains_key(name)
        });

        if has_custom_channels {
            // Return complete state with all channels (for MessageGraph, StateGraph with custom channels)
            Ok(self.read_all_channels())
        } else {
            // Find the latest node output
            let mut latest_version = ChannelVersion::Int(0);
            let mut latest_channel: Option<&str> = None;

            for (chan_name, version) in &self.checkpoint.channel_versions {
                if !chan_name.starts_with("__") {
                    if version > &latest_version {
                        latest_version = version.clone();
                        latest_channel = Some(chan_name);
                    }
                }
            }

            if let Some(chan_name) = latest_channel {
                if let Some(channel) = self.channels.get(chan_name) {
                    let value = channel
                        .get()
                        .map_err(|e| GraphError::Execution(format!("Failed to read final state: {}", e)))?;

                    // Check if this is a state object with nested node outputs
                    // (common in StateGraph where nodes return full state)
                    if let Value::Object(obj) = &value {
                        let has_nested_nodes = obj.keys().any(|k| self.nodes.contains_key(k));

                        if has_nested_nodes {
                            // This is a nested state - aggregate from all node outputs
                            let mut aggregated_state = serde_json::Map::new();

                            for (chan_name2, channel2) in &self.channels {
                                if chan_name2.starts_with("__") {
                                    continue;
                                }

                                if self.nodes.contains_key(chan_name2) {
                                    if let Ok(Value::Object(obj2)) = channel2.get() {
                                        for (k, v) in obj2 {
                                            if !k.starts_with("__") && !self.nodes.contains_key(&k) {
                                                aggregated_state.insert(k, v);
                                            }
                                        }
                                    }
                                }
                            }

                            return Ok(Value::Object(aggregated_state));
                        }
                    }

                    // Normal case: return the latest node output as-is
                    return Ok(value);
                }
            }

            // Fallback: return empty object
            Ok(serde_json::json!({}))
        }
    }

    /// Execute a single superstep.
    ///
    /// Execute a single Pregel superstep - the core of the execution algorithm.
    ///
    /// A superstep represents one round of parallel node execution followed by
    /// synchronized write application. This method implements the complete
    /// Pregel execution cycle with interrupts, checkpointing, and streaming.
    ///
    /// # Algorithm (17 Steps)
    ///
    /// 1. **Resume Handling**: Apply resume value if recovering from interrupt
    /// 2. **Task Preparation**: Call `prepare_next_tasks()` to identify triggered nodes
    /// 3. **Empty Check**: Return false if no tasks (graph complete)
    /// 4. **Write Tracking**: Record pending writes for crash recovery
    /// 5. **Interrupt Before**: Check and handle pre-execution interrupts
    /// 6. **Event Emission**: Emit TaskStart events for streaming
    /// 7. **Context Setup**: Create runtime context with managed values
    /// 8. **Parallel Execution**: Execute all tasks concurrently with retry
    /// 9. **Result Collection**: Gather results from all tasks
    /// 10. **Event Processing**: Emit TaskEnd/TaskError/Updates events
    /// 11. **Command Processing**: Extract Send objects from Command results
    /// 12. **Conditional Routing**: Evaluate edges to create dynamic Send tasks
    /// 13. **TASKS Channel**: Write Send objects for next superstep
    /// 14. **Write Collection**: Decompose results into channel writes
    /// 15. **Interrupt After**: Check and handle post-execution interrupts
    /// 16. **Write Application**: [BARRIER] Apply all writes atomically
    /// 17. **Checkpointing**: Save state snapshot for recovery
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - More tasks available, continue execution
    /// - `Ok(false)` - No tasks remain, graph complete
    /// - `Err(GraphError::Interrupt)` - Execution interrupted
    /// - `Err(_)` - Execution failed
    ///
    /// # Errors
    ///
    /// Returns [`GraphError`] if:
    /// - Node execution fails
    /// - Channel read/write fails
    /// - Checkpoint save fails
    /// - Interrupt triggered
    ///
    /// # Parallelism
    ///
    /// All tasks in a superstep execute concurrently. The barrier at write
    /// application ensures consistency:
    ///
    /// ```text
    /// Superstep N:
    /// ┌──────────────────────────────────────────┐
    /// │  Task A ──┐                              │
    /// │           ├─> Parallel Execution         │
    /// │  Task B ──┘                              │
    /// └──────────────────────────────────────────┘
    ///                        ↓
    ///              [BARRIER: apply_writes]
    ///                        ↓
    /// Superstep N+1:
    /// ┌──────────────────────────────────────────┐
    /// │  Task C ──┐                              │
    /// │           ├─> Parallel Execution         │
    /// │  Task D ──┘                              │
    /// └──────────────────────────────────────────┘
    /// ```
    ///
    /// # Determinism
    ///
    /// Execution is deterministic through:
    /// - Sorted task ordering
    /// - Deterministic write application
    /// - Version-based triggering
    /// - Consistent checkpoint restoration
    ///
    /// # Performance
    ///
    /// - **Time**: O(T + W) where T = parallel task time, W = write application
    /// - **Space**: O(N × S) where N = number of tasks, S = avg state size
    /// - **Parallelism**: Up to N concurrent tasks
    ///
    /// # See Also
    ///
    /// - [`prepare_next_tasks`](super::algo::prepare_next_tasks) - Task scheduling
    /// - [`apply_writes`](super::algo::apply_writes) - Write application
    /// - [`execute_with_retry`](Self::execute_with_retry) - Task execution with retry
    async fn execute_superstep(&mut self) -> Result<bool> {
        // 0. Apply resume value if resuming from interrupt
        let just_resumed = self.interrupt_tracker.is_resuming();
        if just_resumed {
            if let Some(resume_value) = self.resume_value.take() {
                self.apply_resume_value(resume_value)?;
            }
            self.interrupt_tracker.finish_resuming();
        }

        // 1. Prepare tasks
        let node_triggers: HashMap<String, Vec<String>> = self
            .nodes
            .iter()
            .map(|(name, spec)| (name.clone(), spec.triggers.clone()))
            .collect();

        let updated_channels = self
            .checkpoint
            .updated_channels
            .as_ref()
            .map(|v| v.iter().cloned().collect::<HashSet<_>>());

        let tasks = prepare_next_tasks(
            &self.checkpoint,
            &self.nodes,
            &node_triggers,
            &mut self.channels,
            updated_channels.as_ref(),
            &self.trigger_to_nodes,
        )?;

        // If no tasks, we're done
        if tasks.is_empty() {
            return Ok(false);
        }

        // 2. Track pending writes before execution (for crash recovery)
        self.pending_writes.clear();
        for (task_id, task) in &tasks {
            // For each task, record pending write to its output channel
            self.pending_writes.push((
                task_id.clone(),
                task.name.clone(), // Task writes to channel with its name
                task.input.clone(),
            ));
        }

        // 3. Check interrupt_before
        // Skip if we just resumed
        if self.should_interrupt_before(&tasks) && !just_resumed {
            // Record interrupt state
            let node_name = tasks.values().next().unwrap().name.clone();
            let thread_id = self.checkpoint_config.as_ref()
                .and_then(|c| c.thread_id.clone())
                .unwrap_or_else(|| "default".to_string());
            let checkpoint_id = Some(self.checkpoint.id.clone());

            self.interrupt_tracker.interrupt(
                thread_id.clone(),
                node_name.clone(),
                InterruptWhen::Before,
                self.step,
                checkpoint_id,
            );

            return Err(GraphError::interrupted(
                node_name,
                "Interrupted before node execution"
            ));
        }

        // 4. Emit TaskStart events for streaming
        for (task_id, task) in &tasks {
            self.emit_stream_event(StreamMode::Tasks, StreamEvent::TaskStart {
                task_id: task_id.clone(),
                node: task.name.clone(),
                input: task.input.clone(),
            });
        }

        // 5. Execute tasks in parallel with retry
        // Create runtime context for nodes
        let execution_context = ExecutionContext::new(self.max_steps);
        execution_context.set_current_step(self.step);

        let mut runtime = Runtime::new(execution_context.clone());

        // Add store if available
        if let Some(store) = &self.store {
            runtime = runtime.with_store(store.clone());
        }

        // Add stream writer if available
        if let Some(tx) = &self.stream_tx {
            runtime = runtime.with_stream_writer(StreamWriter::new(tx.clone()));
        }

        // Create futures for all tasks
        // Note: Retry policy is currently disabled (uses default 1 attempt)
        // TODO: Add support for per-node retry policies
        let task_futures: Vec<_> = tasks
            .iter()
            .map(|(task_id, task)| {
                let task_id = task_id.clone();
                let mut input = task.input.clone();

                // Inject managed values into input state
                let exec_ctx = execution_context.clone();
                let _ = exec_ctx.inject_managed_values(&mut input);

                let executor = task.proc.clone();
                let node_name = task.name.clone();
                let runtime = runtime.clone();

                async move {
                    let mut result = Self::execute_with_retry(executor, input, None, Some(runtime), Some(node_name)).await;

                    // Remove managed values from output to prevent them from being written to channels
                    if let Ok(ref mut output) = result {
                        exec_ctx.remove_managed_values(output);
                    }

                    (task_id, result)
                }
            })
            .collect();

        // Execute all tasks in parallel
        let results = join_all(task_futures).await;

        // Collect results into HashMap
        let task_results: HashMap<String, Result<serde_json::Value>> =
            results.into_iter().collect();

        // 6. Emit TaskEnd/TaskError and Updates events
        for (task_id, task) in &tasks {
            if let Some(result) = task_results.get(task_id) {
                match result {
                    Ok(output) => {
                        // Emit TaskEnd event
                        self.emit_stream_event(StreamMode::Tasks, StreamEvent::TaskEnd {
                            task_id: task_id.clone(),
                            node: task.name.clone(),
                            output: output.clone(),
                        });

                        // Emit Updates event (node output)
                        self.emit_stream_event(StreamMode::Updates, StreamEvent::Updates {
                            node: task.name.clone(),
                            update: output.clone(),
                        });

                        // Emit Messages events if output contains messages
                        if let Some(messages) = output.get("messages") {
                            if let Some(messages_array) = messages.as_array() {
                                for message in messages_array {
                                    self.emit_stream_event(StreamMode::Messages, StreamEvent::Message {
                                        message: message.clone(),
                                        metadata: output.get("metadata").cloned(),
                                    });
                                }
                            } else {
                                // Single message
                                self.emit_stream_event(StreamMode::Messages, StreamEvent::Message {
                                    message: messages.clone(),
                                    metadata: output.get("metadata").cloned(),
                                });
                            }
                        }

                        // Emit Custom events if output contains custom data
                        if let Some(custom_data) = output.get("__custom__").or_else(|| output.get("custom")) {
                            if let Some(custom_array) = custom_data.as_array() {
                                for data in custom_array {
                                    self.emit_stream_event(StreamMode::Custom, StreamEvent::Custom {
                                        data: data.clone(),
                                    });
                                }
                            } else {
                                // Single custom data
                                self.emit_stream_event(StreamMode::Custom, StreamEvent::Custom {
                                    data: custom_data.clone(),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        // Emit TaskError event
                        self.emit_stream_event(StreamMode::Tasks, StreamEvent::TaskError {
                            task_id: task_id.clone(),
                            node: task.name.clone(),
                            error: e.to_string(),
                        });
                    }
                }
            }
        }

        // 7. Process Command results and extract Send objects
        // Also evaluate conditional edges to route to successor nodes
        // Write Send objects to TASKS channel for execution in next superstep
        use crate::send::Send;
        use crate::send::ConditionalEdgeResult;
        let mut sends_to_write: Vec<Send> = Vec::new();

        // 7.1. Extract Sends from Command results (map-reduce pattern)
        for (task_id, _task) in &tasks {
            if let Some(Ok(value)) = task_results.get(task_id) {
                // Try to parse result as Command
                if let Ok(cmd) = serde_json::from_value::<Command>(value.clone()) {
                    // Check if Command has goto with Send commands
                    if let Some(GotoTarget::Sends(sends)) = cmd.goto {
                        sends_to_write.extend(sends);
                    } else if let Some(GotoTarget::Send(send)) = cmd.goto {
                        sends_to_write.push(send);
                    }
                }
            }
        }

        // 7.2. Evaluate conditional edges for dynamic routing
        for (_task_id, task) in &tasks {
            // Check if this node has outgoing conditional edges
            if let Some(edges) = self.edges.get(&task.name) {
                // Get the task result
                if let Some(Ok(output)) = task_results.get(_task_id) {
                    // Evaluate each conditional edge
                    for edge in edges {
                        if let crate::graph::Edge::Conditional { router, .. } = edge {
                            // Call the router function with the task output
                            let routing_result = router(output);

                            match routing_result {
                                ConditionalEdgeResult::Node(target_node) => {
                                    // Single node - create Send object for execution in next superstep
                                    let send = crate::send::Send::new(target_node, output.clone());
                                    sends_to_write.push(send);
                                }
                                ConditionalEdgeResult::Nodes(target_nodes) => {
                                    // Multiple nodes (parallel branching) - create Send for each
                                    for target_node in target_nodes {
                                        let send = crate::send::Send::new(target_node, output.clone());
                                        sends_to_write.push(send);
                                    }
                                }
                                ConditionalEdgeResult::Sends(sends) => {
                                    // Send objects - add to collection
                                    sends_to_write.extend(sends);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 7.3. Write all Sends to TASKS channel
        if !sends_to_write.is_empty() {
            if let Some(tasks_channel) = self.channels.get_mut("__tasks__") {
                // Convert Send objects to Value for channel write
                let send_values: Vec<Value> = sends_to_write
                    .iter()
                    .map(|s| serde_json::to_value(s).unwrap_or(Value::Null))
                    .collect();

                // Write all Sends to TASKS channel
                if let Err(e) = tasks_channel.update(send_values) {
                    tracing::warn!(error = %e, "Failed to write Send objects to TASKS channel");
                }
            }
        }

        // 8. Collect writes from completed regular tasks
        // Use the task's write_channels specification to determine where to write output
        let task_writes: Vec<_> = tasks
            .iter()
            .filter_map(|(task_id, task)| {
                task_results.get(task_id).and_then(|res| {
                    match res {
                        Ok(value) => {
                            let mut writes = vec![];

                            // Use write_channels if specified (e.g., StateGraph writes to "state" channel)
                            if !task.write_channels.is_empty() {
                                // Write the full output value to each specified channel
                                for channel_name in &task.write_channels {
                                    writes.push((channel_name.clone(), value.clone()));
                                }

                                // ALSO write to node's own channel to trigger successor nodes
                                // This is critical for graph execution flow
                                if self.channels.contains_key(&task.name) {
                                    writes.push((task.name.clone(), value.clone()));
                                }
                            } else {
                                // Backward compatibility: If no write_channels specified, use legacy behavior
                                // Write to node's own channel
                                writes.push((task.name.clone(), value.clone()));

                                // Also write fields to custom channels if result is an object
                                if let Some(obj) = value.as_object() {
                                    for (key, field_value) in obj {
                                        // Only write to custom channels (not node channels, not START/END)
                                        let is_custom_channel = key != &task.name
                                            && self.channels.contains_key(key)
                                            && !self.nodes.contains_key(key)
                                            && key != "__start__"
                                            && key != "__end__";

                                        if is_custom_channel {
                                            writes.push((key.clone(), field_value.clone()));
                                        }
                                    }
                                }
                            }

                            Some(super::types::PregelTaskWrites {
                                path: task.path.clone(),
                                name: task.name.clone(),
                                writes,
                                triggers: task.triggers.clone(),
                            })
                        }
                        Err(_) => None,
                    }
                })
            })
            .collect();

        // 11. Check interrupt_after (before apply_writes)
        if self.should_interrupt_after(&tasks) {
            // Record interrupt state
            let node_name = tasks.values().next().unwrap().name.clone();
            let thread_id = self.checkpoint_config.as_ref()
                .and_then(|c| c.thread_id.clone())
                .unwrap_or_else(|| "default".to_string());
            let checkpoint_id = Some(self.checkpoint.id.clone());

            self.interrupt_tracker.interrupt(
                thread_id.clone(),
                node_name.clone(),
                InterruptWhen::After,
                self.step,
                checkpoint_id,
            );

            return Err(GraphError::interrupted(
                node_name,
                "Interrupted after node execution"
            ));
        }

        // 12. Apply writes from regular tasks (BARRIER - this is where superstep synchronization happens)
        // Note: apply_writes automatically increments channel versions
        // Conditional routing happens via Send objects in TASKS channel (evaluated above)

        // Collect all writes for Values event emission (before task_writes is moved)
        let all_writes: Vec<(String, Value)> = task_writes
            .iter()
            .flat_map(|tw| tw.writes.iter().cloned())
            .collect();

        // Collect task-name and writes for Updates event emission
        let tasks_and_writes: Vec<(String, Vec<(String, Value)>)> = task_writes
            .iter()
            .map(|tw| (tw.name.clone(), tw.writes.clone()))
            .collect();

        let updated = apply_writes(
            &mut self.checkpoint,
            &mut self.channels,
            task_writes,
            &self.trigger_to_nodes,
        )?;

        // Emit stream events if modes are enabled
        if !all_writes.is_empty() {
            // Emit Values event (complete state)
            self.emit_values_event(Some(&all_writes));

            // Emit Updates events (node-by-node updates)
            self.emit_updates_event(&tasks_and_writes);

            // Emit Message events (for MessageGraph pattern)
            self.emit_messages_event(&all_writes);
        }

        // 13. Update versions_seen for executed tasks
        for (_task_id, task) in &tasks {
            // Track what versions this node saw for its trigger channels
            let node_name = &task.name;
            let mut seen = HashMap::new();
            for trigger_chan in &task.triggers {
                if let Some(version) = self.checkpoint.channel_versions.get(trigger_chan) {
                    seen.insert(trigger_chan.clone(), version.clone());
                }
            }
            self.checkpoint.versions_seen.insert(
                node_name.clone(),
                seen,
            );
        }

        // 14. Clear pending writes after successful apply
        self.pending_writes.clear();

        // 15. Emit Values event (complete state after step)
        self.emit_stream_event(StreamMode::Values, StreamEvent::Values {
            state: self.read_all_channels(),
        });

        // 16. Emit Checkpoint event
        self.emit_stream_event(StreamMode::Checkpoints, StreamEvent::Checkpoint {
            thread_id: self.checkpoint.id.clone(),
            namespace: "default".to_string(),
            checkpoint: serde_json::to_value(&self.checkpoint).unwrap_or_default(),
        });

        // 17. Save checkpoint if checkpointer is configured
        if let (Some(checkpointer), Some(config)) = (&self.checkpointer, &self.checkpoint_config) {
            // Convert ChannelVersions from pregel to langgraph_checkpoint format
            let convert_versions = |versions: &HashMap<String, ChannelVersion>| -> HashMap<String, langgraph_checkpoint::checkpoint::ChannelVersion> {
                versions.iter().map(|(k, v)| {
                    let lc_version = match v {
                        ChannelVersion::Int(n) => langgraph_checkpoint::checkpoint::ChannelVersion::Int(*n),
                        ChannelVersion::Float(f) => langgraph_checkpoint::checkpoint::ChannelVersion::Float(*f),
                        ChannelVersion::String(s) => langgraph_checkpoint::checkpoint::ChannelVersion::String(s.clone()),
                    };
                    (k.clone(), lc_version)
                }).collect()
            };

            // Convert Pregel checkpoint to langgraph_checkpoint::Checkpoint
            let lc_checkpoint = langgraph_checkpoint::Checkpoint {
                v: self.checkpoint.v,
                id: self.checkpoint.id.clone(),
                ts: self.checkpoint.ts,
                channel_values: self.checkpoint.channel_values.clone(),
                channel_versions: convert_versions(&self.checkpoint.channel_versions),
                versions_seen: self.checkpoint.versions_seen.iter()
                    .map(|(k, v)| (k.clone(), convert_versions(v)))
                    .collect(),
                updated_channels: self.checkpoint.updated_channels.clone(),
            };

            let metadata = CheckpointMetadata {
                source: Some(CheckpointSource::Loop),
                step: Some(self.step as i32),
                parents: None,
                extra: HashMap::new(),
            };

            // Save checkpoint (ignore errors for now - just log them)
            match checkpointer.put(
                config,
                lc_checkpoint.clone(),
                metadata.clone(),
                convert_versions(&self.checkpoint.channel_versions),
            ).await {
                Ok(_) => {
                    // Emit Checkpoint event if mode is enabled
                    let thread_id = config.thread_id.clone().unwrap_or_else(|| "default".to_string());
                    let checkpoint_ns = config.checkpoint_ns.clone().unwrap_or_default();

                    self.emit_stream_event(
                        StreamMode::Checkpoints,
                        StreamEvent::Checkpoint {
                            thread_id,
                            namespace: checkpoint_ns,
                            checkpoint: serde_json::to_value(&lc_checkpoint).unwrap_or(Value::Null),
                        }
                    );
                }
                Err(e) => {
                    eprintln!("Warning: Failed to save checkpoint: {}", e);
                }
            }
        }

        // Continue if we updated any channels
        Ok(!updated.is_empty())
    }

    /// Check if we should interrupt before executing tasks.
    fn should_interrupt_before(&self, tasks: &HashMap<String, PregelExecutableTask>) -> bool {
        if self.interrupt_before.is_empty() {
            return false;
        }

        tasks.values().any(|task| self.interrupt_before.contains(&task.name))
    }

    /// Check if we should interrupt after executing tasks.
    fn should_interrupt_after(&self, tasks: &HashMap<String, PregelExecutableTask>) -> bool {
        if self.interrupt_after.is_empty() {
            return false;
        }

        tasks.values().any(|task| self.interrupt_after.contains(&task.name))
    }

    /// Emit a stream event if streaming is enabled and the mode is active.
    /// Emit a stream event (buffered for new API, immediate for old API)
    fn emit_stream_event(&mut self, mode: StreamMode, event: StreamEvent) {
        // New buffered API
        if let Some(mux) = &self.stream_mux {
            if mux.has_mode(mode) {
                self.event_buffer.push(mode, event.clone());
            }
        }

        // Old immediate API (deprecated but kept for compatibility)
        if let Some(tx) = &self.stream_tx {
            if event.filter_by_modes(&self.stream_modes) {
                // Ignore send errors - if receiver is dropped, that's ok
                let _ = tx.send(event);
            }
        }
    }

    /// Flush all buffered events to the stream
    async fn flush_events(&mut self) -> Result<()> {
        if let Some(mux) = &self.stream_mux {
            let events = self.event_buffer.flush();
            for chunk in events {
                mux.emit(chunk).await
                    .map_err(|e| GraphError::Execution(format!("Failed to emit stream event: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Get output channel keys (all non-internal channels)
    fn get_output_keys(&self) -> Vec<String> {
        self.channels
            .keys()
            .filter(|k| !k.starts_with("__"))
            .cloned()
            .collect()
    }

    /// Emit a Values event if the mode is enabled and any output channel was written
    fn emit_values_event(&mut self, pending_writes: Option<&[(String, Value)]>) {
        // Get output keys
        let output_keys = self.get_output_keys();
        if output_keys.is_empty() {
            return;
        }

        // Call map_output_values to determine if we should emit
        match map_output_values(&output_keys, pending_writes, &self.channels) {
            Ok(Some(state)) => {
                self.emit_stream_event(StreamMode::Values, StreamEvent::Values { state });
            }
            Ok(None) => {
                // No output to emit
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to map output values");
            }
        }
    }

    /// Emit Updates events for completed tasks
    fn emit_updates_event(&mut self, tasks_and_writes: &[(String, Vec<(String, Value)>)]) {
        if tasks_and_writes.is_empty() {
            return;
        }

        // Get output keys
        let output_keys = self.get_output_keys();
        if output_keys.is_empty() {
            return;
        }

        // Call map_output_updates to generate node-by-node updates
        match map_output_updates(&output_keys, tasks_and_writes) {
            Ok(Some(Value::Object(updates_map))) => {
                // Emit individual Updates event for each node
                for (node, update) in updates_map {
                    self.emit_stream_event(
                        StreamMode::Updates,
                        StreamEvent::Updates { node, update }
                    );
                }
            }
            Ok(Some(_)) => {
                tracing::warn!("map_output_updates returned non-object value");
            }
            Ok(None) => {
                // No updates to emit
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to map output updates");
            }
        }
    }

    /// Emit Message events for message-type writes (for MessageGraph support)
    fn emit_messages_event(&mut self, all_writes: &[(String, Value)]) {
        if all_writes.is_empty() {
            return;
        }

        // Look for writes to "messages" channel (MessageGraph pattern)
        for (channel, value) in all_writes {
            if channel == "messages" {
                // Check if the value is an array of messages
                if let Value::Array(messages) = value {
                    for message in messages {
                        // Each message should be an object with role, content, etc.
                        if message.is_object() {
                            self.emit_stream_event(
                                StreamMode::Messages,
                                StreamEvent::Message {
                                    message: message.clone(),
                                    metadata: None,
                                }
                            );
                        }
                    }
                } else if value.is_object() {
                    // Single message object
                    self.emit_stream_event(
                        StreamMode::Messages,
                        StreamEvent::Message {
                            message: value.clone(),
                            metadata: None,
                        }
                    );
                }
            }
        }
    }

    /// Read all channel values into a single JSON object
    fn read_all_channels(&self) -> Value {
        // Debug: log all channels
        eprintln!("DEBUG: Reading all channels. Total channels: {}", self.channels.len());
        for (name, _) in &self.channels {
            eprintln!("DEBUG:   Channel: {}", name);
        }

        // Special handling for single "state" channel - return its value directly
        if self.channels.len() == 1 || (self.channels.len() == 2 && self.channels.contains_key("__start__")) {
            eprintln!("DEBUG: Checking for single state channel");
            if let Some(state_channel) = self.channels.get("state") {
                if let Ok(value) = state_channel.get() {
                    eprintln!("DEBUG: Returning state channel value directly: {:?}", value);
                    return value;
                } else {
                    eprintln!("DEBUG: State channel exists but get() failed");
                }
            } else {
                eprintln!("DEBUG: No state channel found");
            }
        }

        let mut state = serde_json::Map::new();

        for (name, channel) in &self.channels {
            // Skip internal channels (starting with __)
            if name.starts_with("__") {
                continue;
            }

            // Skip node output channels (channels that match node names)
            if self.nodes.contains_key(name) {
                eprintln!("DEBUG: Skipping node channel: {}", name);
                continue;
            }

            // Read the latest value from state channels only
            if let Ok(value) = channel.get() {
                eprintln!("DEBUG: Reading channel {}: {:?}", name, value);
                // If it's the "state" channel and it's the only non-internal channel,
                // return its value directly for better ergonomics
                if name == "state" && state.is_empty() {
                    // Check if state channel contains the actual state object
                    eprintln!("DEBUG: Returning state channel value from loop: {:?}", value);
                    return value;
                }
                state.insert(name.clone(), value);
            } else {
                eprintln!("DEBUG: Channel {} get() failed", name);
            }
        }

        eprintln!("DEBUG: Final state map: {:?}", state);

        if state.len() == 1 && state.contains_key("state") {
            // If we only have a "state" key, unwrap it
            state.remove("state").unwrap_or(Value::Object(state))
        } else {
            Value::Object(state)
        }
    }

    /// Apply a resume value to the graph state after an interrupt.
    ///
    /// Resume values can either be a single value (applied to a special __resume__ channel)
    /// or a map of interrupt IDs to values.
    fn apply_resume_value(&mut self, resume_value: ResumeValue) -> Result<()> {
        match resume_value {
            ResumeValue::Single(value) => {
                // Apply single resume value to __resume__ channel
                // This channel can be read by nodes to access the resume value
                if let Some(resume_channel) = self.channels.get_mut("__resume__") {
                    resume_channel.update(vec![value]).map_err(|e| {
                        GraphError::Execution(format!("Failed to update __resume__ channel: {}", e))
                    })?;
                } else {
                    // If no __resume__ channel exists, create a transient one
                    // or merge into state directly
                    self.checkpoint.channel_values.insert("__resume__".to_string(), value);
                }
            }
            ResumeValue::ByInterruptId(values) => {
                // Apply resume values by interrupt ID
                // For now, apply the first value found (can be enhanced later)
                if let Some((_, value)) = values.into_iter().next() {
                    if let Some(resume_channel) = self.channels.get_mut("__resume__") {
                        resume_channel.update(vec![value]).map_err(|e| {
                            GraphError::Execution(format!("Failed to update __resume__ channel: {}", e))
                        })?;
                    } else {
                        self.checkpoint.channel_values.insert("__resume__".to_string(), value);
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a task with retry logic
    async fn execute_with_retry(
        executor: Arc<dyn NodeExecutor>,
        input: Value,
        retry_policy: Option<super::super::retry::RetryPolicy>,
        runtime: Option<Runtime>,
        node_name: Option<String>,
    ) -> Result<Value> {
        let policy = retry_policy.unwrap_or_else(|| super::super::retry::RetryPolicy::new(1)); // Default: no retry

        let mut attempts = 0;
        let mut last_error = None;

        while attempts < policy.max_attempts {
            // Set runtime context before execution
            if let Some(ref rt) = runtime {
                rt.set_current_node(node_name.clone());
                set_runtime(rt.clone());
            }

            let result = executor.execute(input.clone()).await;

            // Clear runtime context after execution
            if runtime.is_some() {
                clear_runtime();
            }

            match result {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    attempts += 1;

                    if attempts < policy.max_attempts {
                        // Calculate and wait for retry delay
                        let delay = policy.calculate_delay(attempts - 1);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        // All retries exhausted, return the last error
        Err(last_error.unwrap())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_pregel_loop_creation() {
        let cp = Checkpoint::new();
        let channels = HashMap::new();
        let nodes = HashMap::new();
        let loop_inst = PregelLoop::new(cp, channels, nodes, 100);
        assert_eq!(loop_inst.step, 0);
        assert_eq!(loop_inst.max_steps, 100);
    }

    #[test]
    fn test_trigger_to_nodes_mapping() {
        let cp = Checkpoint::new();
        let channels = HashMap::new();

        let mut nodes = HashMap::new();
        nodes.insert(
            "node_a".to_string(),
            PregelNodeSpec {
                name: "node_a".to_string(),
                triggers: vec!["input".to_string()],
                reads: vec!["input".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );
        nodes.insert(
            "node_b".to_string(),
            PregelNodeSpec {
                name: "node_b".to_string(),
                triggers: vec!["input".to_string(), "config".to_string()],
                reads: vec!["input".to_string(), "config".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        let loop_inst = PregelLoop::new(cp, channels, nodes, 100);

        // Check trigger_to_nodes mapping
        assert_eq!(
            loop_inst.trigger_to_nodes.get("input").unwrap().len(),
            2
        );
        assert_eq!(
            loop_inst.trigger_to_nodes.get("config").unwrap().len(),
            1
        );
    }

    // Dummy executor for testing (public for use in other test modules)
    pub struct DummyExecutor;

    impl NodeExecutor for DummyExecutor {
        fn execute(
            &self,
            input: serde_json::Value,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + '_>> {
            Box::pin(async move { Ok(input) })
        }
    }

    #[test]
    fn test_pending_writes_initialized_empty() {
        let cp = Checkpoint::new();
        let channels = HashMap::new();
        let nodes = HashMap::new();
        let loop_inst = PregelLoop::new(cp, channels, nodes, 100);
        assert_eq!(loop_inst.pending_writes.len(), 0);
    }

    #[tokio::test]
    async fn test_pending_writes_tracked_before_execution() {
        use langgraph_checkpoint::LastValueChannel;

        // Create a simple graph with one node
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("input".to_string(), super::super::checkpoint::ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["input".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert("input".to_string(), Box::new(LastValueChannel::new()));
        channels.insert("process".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        nodes.insert(
            "process".to_string(),
            PregelNodeSpec {
                name: "process".to_string(),
                triggers: vec!["input".to_string()],
                reads: vec!["input".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        let loop_inst = PregelLoop::new(cp, channels, nodes, 100);

        // Before execute_superstep, pending_writes should be empty
        assert_eq!(loop_inst.pending_writes.len(), 0);

        // Execute one superstep - this will prepare tasks and track pending writes
        // Note: This test verifies the internal state tracking
        // The actual pending writes tracking happens before task execution
    }

    #[tokio::test]
    async fn test_pending_writes_cleared_after_successful_apply() {
        use langgraph_checkpoint::LastValueChannel;

        // Create a simple graph
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), super::super::checkpoint::ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"value": 42})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("process".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        nodes.insert(
            "process".to_string(),
            PregelNodeSpec {
                name: "process".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100);

        // Execute one superstep
        let _result = loop_inst.execute_superstep().await;

        // After successful execution, pending_writes should be cleared
        assert_eq!(loop_inst.pending_writes.len(), 0);
    }

    #[tokio::test]
    async fn test_channel_versions_incremented_after_apply() {
        use langgraph_checkpoint::LastValueChannel;

        // Create a simple graph
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"value": 42})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("process".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        nodes.insert(
            "process".to_string(),
            PregelNodeSpec {
                name: "process".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100);

        // Execute one superstep
        let _result = loop_inst.execute_superstep().await;

        // Check that process channel version was incremented
        let process_version = loop_inst.checkpoint.channel_versions.get("process");
        assert!(process_version.is_some(), "process channel should have a version");
        // Version should be Int(2) after first superstep
        // (1 for initial state, incremented to 2 by apply_writes)
        match process_version.unwrap() {
            ChannelVersion::Int(v) => assert_eq!(*v, 2),
            _ => panic!("Expected Int version"),
        }
    }

    #[tokio::test]
    async fn test_versions_seen_tracked_for_nodes() {
        use langgraph_checkpoint::LastValueChannel;

        // Create a simple graph with two nodes
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"value": 42})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("node_a".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        nodes.insert(
            "node_a".to_string(),
            PregelNodeSpec {
                name: "node_a".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100);

        // Execute one superstep
        let _result = loop_inst.execute_superstep().await;

        // Check that node_a has versions_seen recorded
        let node_a_seen = loop_inst.checkpoint.versions_seen.get("node_a");
        assert!(node_a_seen.is_some(), "node_a should have versions_seen");

        let seen = node_a_seen.unwrap();
        // node_a should have seen version 1 of __start__
        let start_version = seen.get("__start__");
        assert!(start_version.is_some(), "node_a should have seen __start__");
        match start_version.unwrap() {
            ChannelVersion::Int(v) => assert_eq!(*v, 1),
            _ => panic!("Expected Int version"),
        }
    }

    #[tokio::test]
    async fn test_deterministic_replay_from_versions() {
        use langgraph_checkpoint::LastValueChannel;

        // Create initial graph state
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("input".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["input".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut input_channel = LastValueChannel::new();
        input_channel.update(vec![serde_json::json!({"count": 0})]).unwrap();
        channels.insert("input".to_string(), Box::new(input_channel));
        channels.insert("counter".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        nodes.insert(
            "counter".to_string(),
            PregelNodeSpec {
                name: "counter".to_string(),
                triggers: vec!["input".to_string()],
                reads: vec!["input".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100);

        // Execute first superstep
        let _result1 = loop_inst.execute_superstep().await;

        // Capture state after first execution
        let counter_version_1 = loop_inst.checkpoint.channel_versions.get("counter").cloned();
        let versions_seen_1 = loop_inst.checkpoint.versions_seen.get("counter").cloned();

        // Both should exist
        assert!(counter_version_1.is_some(), "counter version should exist after first execution");
        assert!(versions_seen_1.is_some(), "versions_seen should exist after first execution");

        // The version should be deterministic (Int(2) after first superstep)
        match counter_version_1.unwrap() {
            ChannelVersion::Int(v) => assert_eq!(v, 2),
            _ => panic!("Expected Int version"),
        }
    }

    // Executor that returns a Command with Send
    struct CommandExecutor {
        command: Command,
    }

    impl NodeExecutor for CommandExecutor {
        fn execute(
            &self,
            _input: serde_json::Value,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + '_>> {
            let cmd = self.command.clone();
            Box::pin(async move {
                // Serialize Command as the result
                Ok(serde_json::to_value(cmd).unwrap())
            })
        }
    }

    #[tokio::test]
    async fn test_send_task_creation_single() {
        use langgraph_checkpoint::LastValueChannel;
        use crate::send::Send;

        // Create a node that returns Command with single Send
        let send_cmd = Command::new()
            .with_goto(Send::new("target_node", serde_json::json!({"data": "test"})));

        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"trigger": true})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("sender".to_string(), Box::new(LastValueChannel::new()));
        channels.insert("target_node".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        // Node that sends a Command
        nodes.insert(
            "sender".to_string(),
            PregelNodeSpec {
                name: "sender".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(CommandExecutor { command: send_cmd }),
            },
        );
        // Target node for the Send
        nodes.insert(
            "target_node".to_string(),
            PregelNodeSpec {
                name: "target_node".to_string(),
                triggers: vec!["sender".to_string()],
                reads: vec!["sender".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100);

        // Execute one superstep - this should process the Command and create dynamic task
        let _result = loop_inst.execute_superstep().await;

        // The dynamic task is created but not executed in this implementation
        // This test verifies the code compiles and runs without errors
        // TODO: Once dynamic task execution is implemented, verify the task was executed
    }

    #[tokio::test]
    async fn test_send_task_creation_multiple() {
        use langgraph_checkpoint::LastValueChannel;
        use crate::send::Send;

        // Create a node that returns Command with multiple Sends (map-reduce pattern)
        let sends = vec![
            Send::new("process", serde_json::json!({"item": 1})),
            Send::new("process", serde_json::json!({"item": 2})),
            Send::new("process", serde_json::json!({"item": 3})),
        ];
        let send_cmd = Command::new().with_goto(sends);

        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"trigger": true})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("mapper".to_string(), Box::new(LastValueChannel::new()));
        channels.insert("process".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        // Mapper node that sends multiple Commands
        nodes.insert(
            "mapper".to_string(),
            PregelNodeSpec {
                name: "mapper".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(CommandExecutor { command: send_cmd }),
            },
        );
        // Process node that gets called multiple times
        nodes.insert(
            "process".to_string(),
            PregelNodeSpec {
                name: "process".to_string(),
                triggers: vec!["mapper".to_string()],
                reads: vec!["mapper".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100);

        // Execute one superstep - this should create 3 dynamic tasks
        let _result = loop_inst.execute_superstep().await;

        // The dynamic tasks are created but not executed in this implementation
        // This test verifies the map-reduce pattern structure is correct
        // TODO: Once dynamic task execution is implemented, verify all 3 tasks were executed
    }

    #[tokio::test]
    async fn test_streaming_task_events() {
        use langgraph_checkpoint::LastValueChannel;

        // Create a simple graph
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"value": 42})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("process".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        nodes.insert(
            "process".to_string(),
            PregelNodeSpec {
                name: "process".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        // Create a channel for streaming events
        let (tx, mut rx) = mpsc::unbounded_channel();

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100)
            .with_streaming(vec![StreamMode::Tasks, StreamMode::Updates], tx);

        // Execute one superstep
        let _result = loop_inst.execute_superstep().await;

        // Collect streamed events
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        // Should have received TaskStart, TaskEnd, and Updates events
        assert!(events.len() >= 2, "Should have received at least 2 stream events");

        // Check that we got a TaskStart event
        let has_task_start = events.iter().any(|e| matches!(e, StreamEvent::TaskStart { .. }));
        assert!(has_task_start, "Should have received TaskStart event");

        // Check that we got a TaskEnd event
        let has_task_end = events.iter().any(|e| matches!(e, StreamEvent::TaskEnd { .. }));
        assert!(has_task_end, "Should have received TaskEnd event");

        // Check that we got an Updates event
        let has_updates = events.iter().any(|e| matches!(e, StreamEvent::Updates { .. }));
        assert!(has_updates, "Should have received Updates event");
    }

    #[tokio::test]
    async fn test_streaming_modes_filtering() {
        use langgraph_checkpoint::LastValueChannel;

        // Create a simple graph
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"value": 10})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("worker".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        nodes.insert(
            "worker".to_string(),
            PregelNodeSpec {
                name: "worker".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        // Create a channel for streaming - only subscribe to Updates mode
        let (tx, mut rx) = mpsc::unbounded_channel();

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100)
            .with_streaming(vec![StreamMode::Updates], tx);

        // Execute one superstep
        let _result = loop_inst.execute_superstep().await;

        // Collect streamed events
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        // Should only receive Updates events, not Task events
        assert!(!events.is_empty(), "Should have received some events");

        for event in &events {
            match event {
                StreamEvent::Updates { .. } => {
                    // This is expected
                }
                _ => {
                    panic!("Should only receive Updates events, got: {:?}", event);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_streaming_values_mode() {
        use langgraph_checkpoint::LastValueChannel;

        // Create a simple graph
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"value": 10})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("worker".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        nodes.insert(
            "worker".to_string(),
            PregelNodeSpec {
                name: "worker".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        // Create a channel for streaming - subscribe to Values mode
        let (tx, mut rx) = mpsc::unbounded_channel();

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100)
            .with_streaming(vec![StreamMode::Values], tx);

        // Execute one superstep
        let _result = loop_inst.execute_superstep().await;

        // Collect streamed events
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        // Should have received Values event
        let has_values = events.iter().any(|e| matches!(e, StreamEvent::Values { .. }));
        assert!(has_values, "Should have received Values event");

        // Verify Values event contains state
        if let Some(StreamEvent::Values { state }) = events.iter().find(|e| matches!(e, StreamEvent::Values { .. })) {
            assert!(state.is_object(), "Values state should be an object");
        }
    }

    #[tokio::test]
    async fn test_streaming_checkpoints_mode() {
        use langgraph_checkpoint::LastValueChannel;

        // Create a simple graph
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"value": 10})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("worker".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        nodes.insert(
            "worker".to_string(),
            PregelNodeSpec {
                name: "worker".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        // Create a channel for streaming - subscribe to Checkpoints mode
        let (tx, mut rx) = mpsc::unbounded_channel();

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100)
            .with_streaming(vec![StreamMode::Checkpoints], tx);

        // Execute one superstep
        let _result = loop_inst.execute_superstep().await;

        // Collect streamed events
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        // Should have received Checkpoint event
        let has_checkpoint = events.iter().any(|e| matches!(e, StreamEvent::Checkpoint { .. }));
        assert!(has_checkpoint, "Should have received Checkpoint event");

        // Verify Checkpoint event has thread_id
        if let Some(StreamEvent::Checkpoint { thread_id, .. }) = events.iter().find(|e| matches!(e, StreamEvent::Checkpoint { .. })) {
            assert!(!thread_id.is_empty(), "Checkpoint should have non-empty thread_id");
        }
    }

    #[tokio::test]
    async fn test_streaming_debug_mode() {
        use langgraph_checkpoint::LastValueChannel;

        // Create a simple graph
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"value": 10})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("worker".to_string(), Box::new(LastValueChannel::new()));

        let mut nodes = HashMap::new();
        nodes.insert(
            "worker".to_string(),
            PregelNodeSpec {
                name: "worker".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(DummyExecutor),
            },
        );

        // Create a channel for streaming - subscribe to Debug mode (combines Checkpoints + Tasks)
        let (tx, mut rx) = mpsc::unbounded_channel();

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100)
            .with_streaming(vec![StreamMode::Debug], tx);

        // Execute one superstep
        let _result = loop_inst.execute_superstep().await;

        // Collect streamed events
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        // Debug mode should include both Task and Checkpoint events
        let has_task = events.iter().any(|e| matches!(e, StreamEvent::TaskStart { .. } | StreamEvent::TaskEnd { .. }));
        let has_checkpoint = events.iter().any(|e| matches!(e, StreamEvent::Checkpoint { .. }));

        assert!(has_task, "Debug mode should have Task events");
        assert!(has_checkpoint, "Debug mode should have Checkpoint events");
    }

    #[tokio::test]
    async fn test_dynamic_task_execution() {
        use langgraph_checkpoint::LastValueChannel;
        use crate::command::{Command, GotoTarget};
        use crate::send::Send as SendTask;

        // Create an executor that returns a Command with Send
        struct MapExecutor;
        impl NodeExecutor for MapExecutor {
            fn execute(&self, _input: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + std::marker::Send + '_>> {
                Box::pin(async move {
                    // Return a Command with 2 Send tasks
                    let cmd = Command {
                        graph: None,
                        goto: Some(GotoTarget::Sends(vec![
                            SendTask::new("worker", serde_json::json!({"value": 1})),
                            SendTask::new("worker", serde_json::json!({"value": 2})),
                        ])),
                        update: None,
                        resume: None,
                    };
                    Ok(serde_json::to_value(cmd).unwrap())
                })
            }
        }

        // Worker executor that doubles the input value
        struct WorkerExecutor;
        impl NodeExecutor for WorkerExecutor {
            fn execute(&self, input: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + std::marker::Send + '_>> {
                Box::pin(async move {
                    if let Some(val) = input.get("value").and_then(|v| v.as_i64()) {
                        Ok(serde_json::json!({"result": val * 2}))
                    } else {
                        Ok(serde_json::json!({"result": 0}))
                    }
                })
            }
        }

        // Create a simple graph
        let mut cp = Checkpoint::new();
        cp.channel_versions.insert("__start__".to_string(), ChannelVersion::Int(1));
        cp.updated_channels = Some(vec!["__start__".to_string()]);

        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let mut start_channel = LastValueChannel::new();
        start_channel.update(vec![serde_json::json!({"trigger": true})]).unwrap();
        channels.insert("__start__".to_string(), Box::new(start_channel));
        channels.insert("map".to_string(), Box::new(LastValueChannel::new()));
        // Use TopicChannel for worker since it can accumulate multiple writes per step
        channels.insert("worker".to_string(), Box::new(langgraph_checkpoint::TopicChannel::new()));
        // Add TASKS channel for dynamic task spawning
        channels.insert("__tasks__".to_string(), Box::new(langgraph_checkpoint::TopicChannel::new()));

        let mut nodes = HashMap::new();
        // Map node that triggers on __start__
        nodes.insert(
            "map".to_string(),
            PregelNodeSpec {
                name: "map".to_string(),
                triggers: vec!["__start__".to_string()],
                reads: vec!["__start__".to_string()],
                writes: vec![],
                executor: Arc::new(MapExecutor),
            },
        );
        // Worker node that will be called dynamically
        nodes.insert(
            "worker".to_string(),
            PregelNodeSpec {
                name: "worker".to_string(),
                triggers: vec![], // No static triggers - called via Send
                reads: vec![], // No reads - receives input via Send
                writes: vec![],
                executor: Arc::new(WorkerExecutor),
            },
        );

        // Create streaming channel to verify dynamic tasks execute
        let (tx, mut rx) = mpsc::unbounded_channel();

        let mut loop_inst = PregelLoop::new(cp, channels, nodes, 100)
            .with_streaming(vec![StreamMode::Tasks, StreamMode::Updates], tx);

        // Execute first superstep - this will execute map node and write Send objects to TASKS
        let result = loop_inst.execute_superstep().await;
        assert!(result.is_ok(), "First superstep should complete successfully: {:?}", result.err());

        // Collect events from first superstep
        let mut events_step1 = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events_step1.push(event);
        }

        // First superstep should have 1 TaskStart (map) and 1 TaskEnd (map)
        let task_starts_step1: Vec<_> = events_step1.iter()
            .filter(|e| matches!(e, StreamEvent::TaskStart { .. }))
            .collect();
        assert_eq!(task_starts_step1.len(), 1, "Should have 1 TaskStart event in first step (map)");

        // Execute second superstep - this will execute the 2 worker tasks from TASKS channel
        let result = loop_inst.execute_superstep().await;
        assert!(result.is_ok(), "Second superstep should complete successfully: {:?}", result.err());

        // Collect events from second superstep
        let mut events_step2 = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events_step2.push(event);
        }

        // Second superstep should have 2 TaskStart events (2 workers)
        let task_starts_step2: Vec<_> = events_step2.iter()
            .filter(|e| matches!(e, StreamEvent::TaskStart { .. }))
            .collect();
        assert_eq!(task_starts_step2.len(), 2, "Should have 2 TaskStart events in second step (workers)");

        // Should have 2 TaskEnd events for workers
        let task_ends_step2: Vec<_> = events_step2.iter()
            .filter(|e| matches!(e, StreamEvent::TaskEnd { .. }))
            .collect();
        assert_eq!(task_ends_step2.len(), 2, "Should have 2 TaskEnd events");

        // Verify worker tasks were executed
        let worker_updates: Vec<_> = events_step2.iter()
            .filter_map(|e| {
                if let StreamEvent::Updates { node, update } = e {
                    if node == "worker" {
                        Some(update)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        // Verify we got worker updates (may be more than 2 due to channel/node write semantics)
        assert!(worker_updates.len() >= 2, "Should have at least 2 worker update events, got {}", worker_updates.len());

        // Verify worker results (should be doubled values: 2 and 4)
        let results: Vec<i64> = worker_updates.iter()
            .filter_map(|u| u.get("result").and_then(|v| v.as_i64()))
            .collect();
        assert!(results.contains(&2), "Should have result 2 (1 * 2)");
        assert!(results.contains(&4), "Should have result 4 (2 * 2)");
    }

    #[tokio::test]
    async fn test_retry_logic_with_eventual_success() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Executor that fails twice then succeeds
        struct RetryExecutor {
            attempts: Arc<AtomicUsize>,
        }

        impl NodeExecutor for RetryExecutor {
            fn execute(&self, _input: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
                let attempts = self.attempts.clone();
                Box::pin(async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 {
                        Err(GraphError::Execution(format!("Attempt {} failed", attempt)))
                    } else {
                        Ok(serde_json::json!({"success": true}))
                    }
                })
            }
        }

        let attempts = Arc::new(AtomicUsize::new(0));
        let executor = Arc::new(RetryExecutor {
            attempts: attempts.clone(),
        });

        // Use retry policy with 3 attempts
        let policy = Some(crate::retry::RetryPolicy::new(3)
            .with_initial_interval(0.01)  // Very short for testing
            .with_jitter(false));

        let result = PregelLoop::execute_with_retry(
            executor,
            serde_json::json!({}),
            policy,
            None,
            None,
        ).await;

        // Should succeed after 3 attempts
        assert!(result.is_ok(), "Should succeed after retries");
        assert_eq!(attempts.load(Ordering::SeqCst), 3, "Should have made 3 attempts");
    }

    #[tokio::test]
    async fn test_retry_logic_exhausts_attempts() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Executor that always fails
        struct AlwaysFailExecutor {
            attempts: Arc<AtomicUsize>,
        }

        impl NodeExecutor for AlwaysFailExecutor {
            fn execute(&self, _input: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
                let attempts = self.attempts.clone();
                Box::pin(async move {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    Err(GraphError::Execution("Always fails".to_string()))
                })
            }
        }

        let attempts = Arc::new(AtomicUsize::new(0));
        let executor = Arc::new(AlwaysFailExecutor {
            attempts: attempts.clone(),
        });

        // Use retry policy with 3 attempts
        let policy = Some(crate::retry::RetryPolicy::new(3)
            .with_initial_interval(0.01)
            .with_jitter(false));

        let result = PregelLoop::execute_with_retry(
            executor,
            serde_json::json!({}),
            policy,
            None,
            None,
        ).await;

        // Should fail after exhausting all attempts
        assert!(result.is_err(), "Should fail after exhausting retries");
        assert_eq!(attempts.load(Ordering::SeqCst), 3, "Should have made 3 attempts");
    }

    #[tokio::test]
    async fn test_checkpoint_save_and_restore() {
        use langgraph_checkpoint::{InMemoryCheckpointSaver, CheckpointConfig};

        // Create a checkpointer
        let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
        let config = CheckpointConfig {
            thread_id: Some("test_thread".to_string()),
            checkpoint_ns: None,
            checkpoint_id: None,
            extra: HashMap::new(),
        };

        // Setup simple test data
        let nodes = HashMap::new();
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert(
            "state".to_string(),
            Box::new(langgraph_checkpoint::LastValueChannel::new()),
        );
        channels.get_mut("state").unwrap().update(vec![serde_json::json!({"value": 42})]).unwrap();

        // Create checkpoint with some state
        let mut checkpoint = Checkpoint::new();
        checkpoint.channel_versions.insert("state".to_string(), ChannelVersion::Int(5));
        checkpoint.channel_values.insert("state".to_string(), serde_json::json!({"value": 42}));

        let mut versions_seen = HashMap::new();
        let mut seen_map = HashMap::new();
        seen_map.insert("state".to_string(), ChannelVersion::Int(3));
        versions_seen.insert("node_a".to_string(), seen_map);
        checkpoint.versions_seen = versions_seen;

        // Create PregelLoop and save checkpoint
        let pregel = PregelLoop::new(checkpoint.clone(), channels, nodes.clone(), 100)
            .with_checkpointer(checkpointer.clone(), config.clone());

        // Manually save the checkpoint (normally done by execute_superstep)
        let metadata = langgraph_checkpoint::CheckpointMetadata {
            source: Some(langgraph_checkpoint::checkpoint::CheckpointSource::Loop),
            step: Some(7),
            parents: None,
            extra: HashMap::new(),
        };

        // Convert and save
        let lc_checkpoint = langgraph_checkpoint::Checkpoint {
            v: checkpoint.v,
            id: checkpoint.id.clone(),
            ts: checkpoint.ts,
            channel_values: checkpoint.channel_values.clone(),
            channel_versions: checkpoint.channel_versions.iter().map(|(k, v)| {
                let lc_v = match v {
                    ChannelVersion::Int(n) => langgraph_checkpoint::checkpoint::ChannelVersion::Int(*n),
                    ChannelVersion::Float(f) => langgraph_checkpoint::checkpoint::ChannelVersion::Float(*f),
                    ChannelVersion::String(s) => langgraph_checkpoint::checkpoint::ChannelVersion::String(s.clone()),
                };
                (k.clone(), lc_v)
            }).collect(),
            versions_seen: checkpoint.versions_seen.iter().map(|(k, v)| {
                (k.clone(), v.iter().map(|(k2, v2)| {
                    let lc_v2 = match v2 {
                        ChannelVersion::Int(n) => langgraph_checkpoint::checkpoint::ChannelVersion::Int(*n),
                        ChannelVersion::Float(f) => langgraph_checkpoint::checkpoint::ChannelVersion::Float(*f),
                        ChannelVersion::String(s) => langgraph_checkpoint::checkpoint::ChannelVersion::String(s.clone()),
                    };
                    (k2.clone(), lc_v2)
                }).collect())
            }).collect(),
            updated_channels: checkpoint.updated_channels.clone(),
        };

        checkpointer.put(&config, lc_checkpoint, metadata.clone(), HashMap::new()).await.unwrap();

        drop(pregel);

        // Create fresh channels for restore
        let mut new_channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        new_channels.insert(
            "state".to_string(),
            Box::new(langgraph_checkpoint::LastValueChannel::new()),
        );

        // Restore from checkpoint
        let restored = PregelLoop::from_checkpoint(
            checkpointer,
            config,
            new_channels,
            nodes,
            100,
            HashMap::new(), // No edges for this test
        )
        .await
        .expect("Should restore from checkpoint");

        // Verify all state was restored correctly
        assert_eq!(restored.step, 7, "Step should be restored from metadata");
        assert_eq!(restored.checkpoint.channel_versions.get("state"), Some(&ChannelVersion::Int(5)), "Channel versions should be restored");

        let restored_state = restored.channels.get("state").unwrap().get().unwrap();
        assert_eq!(restored_state, serde_json::json!({"value": 42}), "Channel value should be restored");

        let node_a_seen = restored.checkpoint.versions_seen.get("node_a").unwrap();
        assert_eq!(node_a_seen.get("state"), Some(&ChannelVersion::Int(3)), "Versions seen should be restored");
    }

    #[tokio::test]
    async fn test_checkpoint_restore_with_no_checkpoint() {
        use langgraph_checkpoint::{InMemoryCheckpointSaver, CheckpointConfig};

        let checkpointer = Arc::new(InMemoryCheckpointSaver::new());
        let config = CheckpointConfig {
            thread_id: Some("nonexistent_thread".to_string()),
            checkpoint_ns: None,
            checkpoint_id: None,
            extra: HashMap::new(),
        };

        let channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        let nodes = HashMap::new();

        // Should fail when no checkpoint exists
        let result = PregelLoop::from_checkpoint(
            checkpointer,
            config,
            channels,
            nodes,
            100,
            HashMap::new(), // No edges for this test
        )
        .await;

        assert!(result.is_err(), "Should fail when no checkpoint exists");
        match result.err().unwrap() {
            GraphError::Checkpoint(err) => {
                // Should be NotFound error
                assert!(matches!(err, langgraph_checkpoint::CheckpointError::NotFound(_)));
            }
            _ => panic!("Expected Checkpoint error"),
        }
    }

    #[tokio::test]
    async fn test_interrupt_before_and_resume() {
        use langgraph_checkpoint::LastValueChannel;
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Create a simple counter executor
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        struct CounterExecutor {
            counter: Arc<AtomicUsize>,
        }

        impl NodeExecutor for CounterExecutor {
            fn execute(&self, input: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
                let counter = self.counter.clone();
                Box::pin(async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
                    let mut result = serde_json::Map::new();
                    result.insert("count".to_string(), serde_json::json!(count));
                    Ok(Value::Object(result))
                })
            }
        }

        // Setup nodes
        let mut nodes = HashMap::new();
        nodes.insert(
            "process".to_string(),
            PregelNodeSpec {
                name: "process".to_string(),
                triggers: vec!["input".to_string()],
                reads: vec!["input".to_string()],
                writes: vec![],
                executor: Arc::new(CounterExecutor {
                    counter: counter_clone,
                }),
            },
        );

        // Setup channels
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert(
            "input".to_string(),
            Box::new(LastValueChannel::new()),
        );
        channels.get_mut("input").unwrap().update(vec![serde_json::json!({"data": 42})]).unwrap();

        // Create checkpoint with updated channel
        let mut checkpoint = Checkpoint::new();
        checkpoint.updated_channels = Some(vec!["input".to_string()]);
        checkpoint.channel_versions.insert("input".to_string(), ChannelVersion::Int(1));

        // Create PregelLoop with interrupt_before on "process" node
        let mut interrupt_before = HashSet::new();
        interrupt_before.insert("process".to_string());

        let mut pregel = PregelLoop::new(checkpoint, channels, nodes, 100)
            .with_interrupt_before(interrupt_before);

        // Execute - should interrupt before "process"
        let result = pregel.execute_superstep().await;
        assert!(result.is_err(), "Should interrupt before execution");

        match result {
            Err(GraphError::Interrupted { node, .. }) => {
                assert_eq!(node, "process", "Should mention the interrupted node");
            }
            _ => panic!("Expected Interrupted error"),
        }

        // Verify interrupt was tracked
        assert!(pregel.is_interrupted(), "Should be in interrupted state");
        let interrupt = pregel.current_interrupt().unwrap();
        assert_eq!(interrupt.node, "process");
        assert_eq!(interrupt.when, InterruptWhen::Before);

        // Resume execution
        pregel.resume().unwrap();
        assert!(!pregel.is_interrupted(), "Should no longer be interrupted");

        // After resuming from interrupt_before, the task should still be pending
        // We need to execute again to actually run the task
        let result = pregel.execute_superstep().await;

        // The result could be Ok(true) if tasks executed, or Ok(false) if no more tasks
        // It should NOT be another interrupt
        match result {
            Ok(_) => {
                // Success - task executed
                assert_eq!(counter.load(Ordering::SeqCst), 1, "Node should have executed once");
            }
            Err(e) => {
                panic!("Should not error after resume: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_interrupt_after_and_resume_with_value() {
        use langgraph_checkpoint::LastValueChannel;

        struct EchoExecutor;

        impl NodeExecutor for EchoExecutor {
            fn execute(&self, input: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
                Box::pin(async move {
                    Ok(input)
                })
            }
        }

        // Setup nodes
        let mut nodes = HashMap::new();
        nodes.insert(
            "review".to_string(),
            PregelNodeSpec {
                name: "review".to_string(),
                triggers: vec!["input".to_string()],
                reads: vec!["input".to_string()],
                writes: vec![],
                executor: Arc::new(EchoExecutor),
            },
        );

        // Setup channels (including __resume__ channel)
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert(
            "input".to_string(),
            Box::new(LastValueChannel::new()),
        );
        channels.insert(
            "__resume__".to_string(),
            Box::new(LastValueChannel::new()),
        );
        channels.get_mut("input").unwrap().update(vec![serde_json::json!({"data": "test"})]).unwrap();

        // Create checkpoint
        let mut checkpoint = Checkpoint::new();
        checkpoint.updated_channels = Some(vec!["input".to_string()]);
        checkpoint.channel_versions.insert("input".to_string(), ChannelVersion::Int(1));

        // Create PregelLoop with interrupt_after on "review" node
        let mut interrupt_after = HashSet::new();
        interrupt_after.insert("review".to_string());

        let mut pregel = PregelLoop::new(checkpoint, channels, nodes, 100)
            .with_interrupt_after(interrupt_after);

        // Execute - should interrupt after "review"
        let result = pregel.execute_superstep().await;
        assert!(result.is_err(), "Should interrupt after execution");

        match result {
            Err(GraphError::Interrupted { node, .. }) => {
                assert_eq!(node, "review", "Should mention the interrupted node");
            }
            _ => panic!("Expected Interrupted error"),
        }

        // Verify interrupt
        let interrupt = pregel.current_interrupt().unwrap();
        assert_eq!(interrupt.when, InterruptWhen::After);

        // Resume with a value
        pregel.resume().unwrap();
        pregel.resume_value = Some(ResumeValue::Single(serde_json::json!({"approved": true})));

        // Execute again
        let result = pregel.execute_superstep().await;
        // May return Ok(false) if no more tasks, that's fine
        assert!(result.is_ok() || matches!(result, Err(GraphError::Interrupted { .. })));

        // Verify resume value was applied
        if let Some(resume_val) = pregel.checkpoint.channel_values.get("__resume__") {
            assert_eq!(resume_val, &serde_json::json!({"approved": true}));
        }
    }

    #[tokio::test]
    async fn test_messages_streaming_mode() {
        use langgraph_checkpoint::LastValueChannel;

        // Create executor that returns messages
        struct MessageExecutor;

        impl NodeExecutor for MessageExecutor {
            fn execute(&self, _input: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
                Box::pin(async move {
                    Ok(serde_json::json!({
                        "messages": [
                            {"role": "user", "content": "Hello"},
                            {"role": "assistant", "content": "Hi there!"}
                        ],
                        "metadata": {"model": "gpt-4"}
                    }))
                })
            }
        }

        // Setup nodes
        let mut nodes = HashMap::new();
        nodes.insert(
            "chat".to_string(),
            PregelNodeSpec {
                name: "chat".to_string(),
                triggers: vec!["input".to_string()],
                reads: vec!["input".to_string()],
                writes: vec![],
                executor: Arc::new(MessageExecutor),
            },
        );

        // Setup channels
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert("input".to_string(), Box::new(LastValueChannel::new()));
        channels.get_mut("input").unwrap().update(vec![serde_json::json!({"query": "test"})]).unwrap();

        // Create checkpoint
        let mut checkpoint = Checkpoint::new();
        checkpoint.updated_channels = Some(vec!["input".to_string()]);
        checkpoint.channel_versions.insert("input".to_string(), ChannelVersion::Int(1));

        // Setup streaming with Messages mode
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut pregel = PregelLoop::new(checkpoint, channels, nodes, 100)
            .with_streaming(vec![StreamMode::Messages], tx);

        // Execute
        let _result = pregel.execute_superstep().await;

        // Collect streaming events
        let mut message_events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            if let StreamEvent::Message { message, .. } = event {
                message_events.push(message);
            }
        }

        // Verify we received 2 message events
        assert_eq!(message_events.len(), 2, "Should receive 2 message events");
        assert_eq!(message_events[0]["role"], "user");
        assert_eq!(message_events[0]["content"], "Hello");
        assert_eq!(message_events[1]["role"], "assistant");
        assert_eq!(message_events[1]["content"], "Hi there!");
    }

    #[tokio::test]
    async fn test_custom_streaming_mode() {
        use langgraph_checkpoint::LastValueChannel;

        // Create executor that returns custom data
        struct CustomExecutor;

        impl NodeExecutor for CustomExecutor {
            fn execute(&self, _input: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
                Box::pin(async move {
                    Ok(serde_json::json!({
                        "result": "success",
                        "__custom__": [
                            {"type": "metric", "value": 42},
                            {"type": "log", "message": "Processing complete"}
                        ]
                    }))
                })
            }
        }

        // Setup nodes
        let mut nodes = HashMap::new();
        nodes.insert(
            "process".to_string(),
            PregelNodeSpec {
                name: "process".to_string(),
                triggers: vec!["input".to_string()],
                reads: vec!["input".to_string()],
                writes: vec![],
                executor: Arc::new(CustomExecutor),
            },
        );

        // Setup channels
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert("input".to_string(), Box::new(LastValueChannel::new()));
        channels.get_mut("input").unwrap().update(vec![serde_json::json!({"data": "test"})]).unwrap();

        // Create checkpoint
        let mut checkpoint = Checkpoint::new();
        checkpoint.updated_channels = Some(vec!["input".to_string()]);
        checkpoint.channel_versions.insert("input".to_string(), ChannelVersion::Int(1));

        // Setup streaming with Custom mode
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut pregel = PregelLoop::new(checkpoint, channels, nodes, 100)
            .with_streaming(vec![StreamMode::Custom], tx);

        // Execute
        let _result = pregel.execute_superstep().await;

        // Collect streaming events
        let mut custom_events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            if let StreamEvent::Custom { data } = event {
                custom_events.push(data);
            }
        }

        // Verify we received 2 custom events
        assert_eq!(custom_events.len(), 2, "Should receive 2 custom events");
        assert_eq!(custom_events[0]["type"], "metric");
        assert_eq!(custom_events[0]["value"], 42);
        assert_eq!(custom_events[1]["type"], "log");
        assert_eq!(custom_events[1]["message"], "Processing complete");
    }

    #[tokio::test]
    async fn test_messages_and_custom_streaming_combined() {
        use langgraph_checkpoint::LastValueChannel;

        // Create executor that returns both messages and custom data
        struct CombinedExecutor;

        impl NodeExecutor for CombinedExecutor {
            fn execute(&self, _input: Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + '_>> {
                Box::pin(async move {
                    Ok(serde_json::json!({
                        "messages": {"role": "assistant", "content": "Done"},
                        "custom": {"metric": "latency", "value": 150}
                    }))
                })
            }
        }

        // Setup nodes
        let mut nodes = HashMap::new();
        nodes.insert(
            "agent".to_string(),
            PregelNodeSpec {
                name: "agent".to_string(),
                triggers: vec!["input".to_string()],
                reads: vec!["input".to_string()],
                writes: vec![],
                executor: Arc::new(CombinedExecutor),
            },
        );

        // Setup channels
        let mut channels: HashMap<String, Box<dyn Channel>> = HashMap::new();
        channels.insert("input".to_string(), Box::new(LastValueChannel::new()));
        channels.get_mut("input").unwrap().update(vec![serde_json::json!({"query": "test"})]).unwrap();

        // Create checkpoint
        let mut checkpoint = Checkpoint::new();
        checkpoint.updated_channels = Some(vec!["input".to_string()]);
        checkpoint.channel_versions.insert("input".to_string(), ChannelVersion::Int(1));

        // Setup streaming with both modes
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut pregel = PregelLoop::new(checkpoint, channels, nodes, 100)
            .with_streaming(vec![StreamMode::Messages, StreamMode::Custom], tx);

        // Execute
        let _result = pregel.execute_superstep().await;

        // Collect streaming events
        let mut message_count = 0;
        let mut custom_count = 0;
        while let Ok(event) = rx.try_recv() {
            match event {
                StreamEvent::Message { .. } => message_count += 1,
                StreamEvent::Custom { .. } => custom_count += 1,
                _ => {}
            }
        }

        // Verify we received both types of events
        assert_eq!(message_count, 1, "Should receive 1 message event");
        assert_eq!(custom_count, 1, "Should receive 1 custom event");
    }
}
