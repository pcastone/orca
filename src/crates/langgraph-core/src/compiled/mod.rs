//! CompiledGraph execution engine for running stateful workflows
//!
//! This module provides the execution runtime for compiled graphs. Once a graph is built
//! using [`StateGraph`](crate::StateGraph) and compiled, it becomes a [`CompiledGraph`]
//! that can be executed multiple times with different inputs, providing deterministic,
//! checkpointed, and observable execution.
//!
//! # Overview
//!
//! `CompiledGraph` is the runtime execution engine that takes a graph definition and runs it
//! using a Pregel-inspired model. It provides:
//!
//! - **Deterministic Execution** - Same input + checkpoint = same output
//! - **Parallel Processing** - Independent nodes run concurrently
//! - **State Persistence** - Automatic checkpointing at each step
//! - **Real-time Streaming** - 7 streaming modes for observability
//! - **Human-in-the-Loop** - Pause/resume with interrupts
//! - **Time Travel** - Resume from any historical checkpoint
//!
//! # Key Types
//!
//! - [`CompiledGraph`] - The executable graph runtime
//! - [`ExecutionEvent`] - Legacy execution events
//! - [`StateSnapshot`] - Point-in-time state snapshots
//! - [`EventStream`] - Async stream of execution events
//! - [`StreamChunkStream`] - Stream of state/debug chunks
//!
//! # Execution Modes
//!
//! ## Invoke - One-Shot Execution
//!
//! Run the graph to completion and return final state.
//!
//! ## Stream - Real-Time Updates
//!
//! Stream execution events in real-time using 7 different modes.
//!
//! See module documentation for detailed examples and usage patterns.

mod types;
mod graph;
mod execution;
mod state;
mod streaming;
mod composition;
mod introspection;
mod pregel_builder;
#[cfg(test)]
mod tests;

// Re-export public types
pub use types::{ExecutionEvent, StateSnapshot, EventStream, StreamChunkStream, StateSnapshotStream};
pub use graph::CompiledGraph;
