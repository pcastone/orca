//! Pregel-inspired execution engine for stateful graph workflows
//!
//! This module implements a Pregel-style execution model adapted for LangGraph's
//! requirements. The Pregel model, originally designed by Google for large-scale
//! graph processing, provides the foundation for deterministic, checkpointed execution
//! of stateful workflows.
//!
//! # Pregel Model Overview
//!
//! The Pregel computational model divides execution into **supersteps**:
//!
//! 1. **Input**: Nodes receive messages/state from the previous superstep
//! 2. **Compute**: Nodes execute their logic in parallel
//! 3. **Message Passing**: Nodes write outputs to channels
//! 4. **Barrier**: Wait for all active nodes to complete
//! 5. **Checkpoint**: Save complete state snapshot
//! 6. **Repeat**: Continue until no more work (graph reaches END or interrupts)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Pregel Loop Lifecycle                     │
//! │                                                             │
//! │  ┌────────────────────────────────────────────────────┐   │
//! │  │  Superstep N                                        │   │
//! │  │                                                     │   │
//! │  │  1. Read Channels (State)                          │   │
//! │  │     ↓                                               │   │
//! │  │  2. Execute Nodes (Parallel)                       │   │
//! │  │     ├── Node A  ├── Node B  ├── Node C             │   │
//! │  │     ↓                                               │   │
//! │  │  3. Write Channels (Updates)                       │   │
//! │  │     ↓                                               │   │
//! │  │  4. Barrier (Wait All Complete)                    │   │
//! │  │     ↓                                               │   │
//! │  │  5. Checkpoint (Save State)                        │   │
//! │  └────────────────────────────────────────────────────┘   │
//! │                     ↓                                      │
//! │                 More work?                                 │
//! │            Yes ↙         ↘ No                              │
//! │      Next Superstep      Done                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Components
//!
//! ## PregelLoop
//!
//! The main execution coordinator ([`PregelLoop`]) orchestrates supersteps,
//! manages checkpoints, and handles interrupts.
//!
//! ## Channels
//!
//! State storage primitives that persist values between supersteps:
//!
//! - [`LastValueChannel`] - Stores only the most recent value
//! - [`TopicChannel`] - Appends all values to a list
//! - [`BinaryOperatorChannel`] - Merges values with custom reducer
//! - [`EphemeralValueChannel`] - Temporary within-superstep communication
//! - [`AnyValueChannel`] - Type-erased storage
//! - [`NamedBarrierValueChannel`] - Synchronization primitive
//! - [`UntrackedValueChannel`] - No checkpoint tracking
//!
//! ## Tasks
//!
//! Execution units representing node invocations:
//!
//! - [`PregelTask`] - Scheduled task definition
//! - [`PregelExecutableTask`] - Task ready for execution
//! - [`TaskState`] - Execution lifecycle state
//!
//! ## Checkpoints
//!
//! State persistence for time-travel and recovery:
//!
//! - [`Checkpoint`] - Complete state snapshot
//! - [`ChannelVersions`] - Version tracking for channels
//! - [`ChannelVersion`] - Individual channel version
//!
//! # Execution Flow
//!
//! ## Normal Execution
//!
//! ```text
//! START
//!   │
//!   ├─> Superstep 0: Execute entry nodes
//!   │     └─> Checkpoint 0
//!   │
//!   ├─> Superstep 1: Execute triggered nodes
//!   │     └─> Checkpoint 1
//!   │
//!   ├─> Superstep 2: Continue execution
//!   │     └─> Checkpoint 2
//!   │
//!   └─> END (no more tasks)
//! ```
//!
//! ## With Interrupts (Human-in-the-Loop)
//!
//! ```text
//! START
//!   │
//!   ├─> Superstep 0: Execute nodes
//!   │     └─> Checkpoint 0
//!   │
//!   ├─> Superstep 1: Node triggers interrupt
//!   │     └─> Checkpoint 1 (INTERRUPTED)
//!   │     └─> Return control to user
//!   │
//! [User provides input]
//!   │
//!   ├─> Resume from Checkpoint 1
//!   │     └─> Superstep 2: Continue with user input
//!   │     └─> Checkpoint 2
//!   │
//!   └─> END
//! ```
//!
//! # Features
//!
//! ## Deterministic Replay
//!
//! All execution is deterministic from checkpoints. Given the same checkpoint
//! and input, execution always produces the same result.
//!
//! ## Parallel Execution
//!
//! Independent nodes in each superstep execute concurrently using Tokio tasks.
//! The barrier ensures all tasks complete before advancing.
//!
//! ## Checkpointing
//!
//! After each superstep, the complete graph state is saved. This enables:
//!
//! - **Time-travel debugging**: Inspect any historical state
//! - **Fault recovery**: Resume from last checkpoint after crash
//! - **Human-in-the-loop**: Pause for approval, resume later
//! - **A/B testing**: Branch from checkpoints to test alternatives
//!
//! ## Interrupt Support
//!
//! Nodes can trigger interrupts to pause execution:
//!
//! ```rust,no_run
//! use langgraph_core::Interrupt;
//!
//! async fn approval_node(state: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
//!     // Check if approval needed
//!     if state["needs_approval"].as_bool().unwrap_or(false) {
//!         return Err(Box::new(Interrupt::new("approval_needed")));
//!     }
//!     Ok(state)
//! }
//! ```
//!
//! ## State Management
//!
//! Channels provide flexible state storage:
//!
//! - **Reducers**: Merge concurrent writes deterministically
//! - **Versioning**: Track changes for efficient checkpointing
//! - **Isolation**: Each superstep sees consistent state snapshot
//!
//! # Performance Considerations
//!
//! ## Parallelism
//!
//! - Nodes with no dependencies execute in parallel within each superstep
//! - Parallelism limited by available Tokio worker threads
//! - Barrier synchronization required between supersteps
//!
//! ## Checkpointing Overhead
//!
//! - State serialized after every superstep
//! - Use checkpoint savers with batching for production
//! - Consider checkpoint frequency vs recovery time tradeoff
//!
//! ## Memory Usage
//!
//! - Full state kept in memory during execution
//! - Channels store complete history (Topic) or latest value (LastValue)
//! - Large states may require streaming or pagination patterns
//!
//! # Examples
//!
//! ## Basic Execution Loop
//!
//! ```rust,no_run
//! use langgraph_core::pregel::PregelLoop;
//! use serde_json::json;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     // PregelLoop created during graph compilation
//!     // Usually you don't create this directly
//!
//!     // Execution happens via CompiledGraph::invoke()
//!     // which internally uses PregelLoop
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Understanding Supersteps
//!
//! ```text
//! Given graph: A -> B -> C
//!
//! Superstep 0:
//!   - Execute A (reads initial state)
//!   - A writes to channel
//!   - Checkpoint
//!
//! Superstep 1:
//!   - Execute B (reads A's output)
//!   - B writes to channel
//!   - Checkpoint
//!
//! Superstep 2:
//!   - Execute C (reads B's output)
//!   - C writes to channel
//!   - Checkpoint
//!
//! Done (C edges to END)
//! ```
//!
//! # See Also
//!
//! - [Pregel Paper](https://research.google/pubs/pub37252/) - Original Google paper
//! - [`CompiledGraph`](crate::CompiledGraph) - User-facing execution API
//! - [`StateGraph`](crate::StateGraph) - Graph construction
//! - [`Checkpoint`] - State persistence
//!
//! # Module Organization
//!
//! - [`types`] - Core type definitions
//! - [`channel`] - Channel implementations
//! - [`algo`] - Pregel algorithm primitives
//! - [`executor`] - Task execution
//! - [`checkpoint`] - State persistence
//! - [`io`] - Input/output handling
//! - [`loop_impl`] - Main execution loop

pub mod types;
pub mod channel;
pub mod algo;
pub mod executor;
pub mod checkpoint;
pub mod io;
pub mod loop_impl;

pub use types::{
    PregelTask, PregelExecutableTask, PathSegment, TaskState,
    RetryPolicy, CachePolicy, CacheKey, Interrupt, NodeExecutor,
};
pub use channel::{
    Channel, LastValueChannel, TopicChannel, BinaryOperatorChannel,
    EphemeralValueChannel, AnyValueChannel, NamedBarrierValueChannel,
    UntrackedValueChannel,
};
pub use algo::{apply_writes, prepare_next_tasks, increment};
pub use executor::TaskExecutor;
pub use checkpoint::{Checkpoint, ChannelVersions, ChannelVersion};
pub use loop_impl::{PregelLoop, PregelNodeSpec};
