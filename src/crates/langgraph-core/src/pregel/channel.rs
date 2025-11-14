//! Channel implementations for Pregel execution.
//!
//! Channels are typed state containers with versioning that control
//! how state updates are applied and when nodes are triggered.

// Re-export all channel types from langgraph-checkpoint
pub use langgraph_checkpoint::{
    AnyValueChannel, BinaryOperatorChannel, Channel, EphemeralValueChannel, LastValueChannel,
    NamedBarrierValueChannel, TopicChannel, UntrackedValueChannel,
};
