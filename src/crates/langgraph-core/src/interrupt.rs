//! Human-in-the-loop (HITL) interrupts for approval workflows and debugging
//!
//! This module provides the interrupt system that enables graph execution to pause at specific
//! points, allowing human review, approval, state modification, or debugging before resuming.
//! Interrupts are the foundation for building production-grade AI agents that require human
//! oversight and approval workflows.
//!
//! # Overview
//!
//! The interrupt system provides:
//!
//! - **Configurable Breakpoints** - Pause before or after specific nodes
//! - **State Inspection** - Examine graph state when paused
//! - **State Modification** - Update state before resuming
//! - **Checkpoint Integration** - Interrupts integrate with checkpoint system for persistence
//! - **Resumption Control** - Resume from specific interrupt points
//! - **Interrupt History** - Track all interrupts across execution
//! - **Metadata Support** - Attach custom data to interrupt states
//!
//! # Core Types
//!
//! - [`InterruptConfig`] - Configuration for when to interrupt (before/after nodes)
//! - [`InterruptState`] - State of a paused execution with metadata
//! - [`InterruptWhen`] - Timing: before or after node execution
//! - [`InterruptTracker`] - Runtime tracking of interrupt state
//! - [`InterruptError`] - Errors during interrupt operations
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Graph Execution with Interrupts                             │
//! │                                                               │
//! │  ┌──────────┐       ┌──────────┐       ┌──────────┐        │
//! │  │  Node A  │   ──► │  Node B  │   ──► │  Node C  │        │
//! │  └──────────┘       └────┬─────┘       └──────────┘        │
//! │                           │                                  │
//! │                           │ interrupt_before: ["Node B"]    │
//! │                           ↓                                  │
//! │  ┌───────────────────────────────────────────────┐          │
//! │  │  INTERRUPT - Execution Paused                 │          │
//! │  │  • Save checkpoint                             │          │
//! │  │  • Create InterruptState                       │          │
//! │  │  • Return control to user                      │          │
//! │  └───────────────────────────────────────────────┘          │
//! │                           ↓                                  │
//! │  ┌───────────────────────────────────────────────┐          │
//! │  │  Human Review                                  │          │
//! │  │  • Inspect state                               │          │
//! │  │  • Modify state (optional)                     │          │
//! │  │  • Approve/reject                              │          │
//! │  └───────────────────────────────────────────────┘          │
//! │                           ↓                                  │
//! │  ┌───────────────────────────────────────────────┐          │
//! │  │  RESUME - Continue Execution                   │          │
//! │  │  • Load checkpoint                             │          │
//! │  │  • Apply any state changes                     │          │
//! │  │  • Continue from interrupt point               │          │
//! │  └───────────────────────────────────────────────┘          │
//! │                           ↓                                  │
//! │                      [Execute Node B]                        │
//! │                           ↓                                  │
//! │                      [Continue to Node C]                    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Basic Approval Workflow
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, InterruptConfig};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut graph = StateGraph::new();
//!
//!     graph.add_node("analyze", |state| {
//!         Box::pin(async move {
//!             // Perform analysis
//!             Ok(json!({"analysis": "risk: medium"}))
//!         })
//!     });
//!
//!     graph.add_node("execute", |state| {
//!         Box::pin(async move {
//!             // Execute action
//!             Ok(json!({"status": "executed"}))
//!         })
//!     });
//!
//!     graph.add_edge("analyze", "execute");
//!
//!     // Pause before executing action (requires approval)
//!     let config = InterruptConfig::new()
//!         .with_interrupt_before(vec!["execute".to_string()]);
//!
//!     let compiled = graph.compile_with_interrupts(config)?;
//!
//!     // First run - executes until interrupt
//!     let paused = compiled.invoke(json!({"input": "data"})).await?;
//!     println!("Paused before execute. State: {:?}", paused.state);
//!
//!     // Human reviews and approves
//!     // ... approval logic ...
//!
//!     // Resume execution
//!     let final_result = compiled.resume().await?;
//!     println!("Completed: {:?}", final_result.state);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Interrupt After Node for Review
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, InterruptConfig};
//! use serde_json::json;
//!
//! // Pause after a node completes to review its output
//! let config = InterruptConfig::new()
//!     .with_interrupt_after(vec!["llm_response".to_string()]);
//!
//! let compiled = graph.compile_with_interrupts(config)?;
//!
//! // Execute until after llm_response completes
//! let paused = compiled.invoke(json!({"prompt": "..."})).await?;
//!
//! // Review LLM output
//! let response = &paused.state["response"];
//! println!("LLM said: {}", response);
//!
//! // Optionally modify state before continuing
//! let modified_state = json!({
//!     "response": "Edited response",
//!     "approved": true
//! });
//!
//! let result = compiled.resume_with_state(modified_state).await?;
//! ```
//!
//! ## Debug All Nodes
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, InterruptConfig};
//!
//! // Interrupt after every node for step-through debugging
//! let config = InterruptConfig::new()
//!     .with_interrupt_after_all();
//!
//! let compiled = graph.compile_with_interrupts(config)?;
//!
//! let mut state = initial_state;
//! loop {
//!     let result = compiled.resume_with_state(state).await?;
//!
//!     if result.is_interrupted {
//!         println!("Paused at: {}", result.interrupt.unwrap().node);
//!         println!("State: {:?}", result.state);
//!         state = result.state; // Continue with current state
//!     } else {
//!         println!("Execution complete!");
//!         break;
//!     }
//! }
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Multi-Stage Approval Workflow
//!
//! Require approval at multiple critical points:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, InterruptConfig};
//! use serde_json::json;
//!
//! let mut graph = StateGraph::new();
//!
//! graph.add_node("draft", |state| {
//!     Box::pin(async move {
//!         Ok(json!({"draft": "generated content"}))
//!     })
//! });
//!
//! graph.add_node("publish", |state| {
//!     Box::pin(async move {
//!         Ok(json!({"published": true}))
//!     })
//! });
//!
//! graph.add_node("notify", |state| {
//!     Box::pin(async move {
//!         Ok(json!({"notified": true}))
//!     })
//! });
//!
//! // Require approval before publishing AND before notifying
//! let config = InterruptConfig::new()
//!     .with_interrupt_before(vec![
//!         "publish".to_string(),
//!         "notify".to_string(),
//!     ]);
//!
//! let compiled = graph.compile_with_interrupts(config)?;
//!
//! // First interrupt: before publish
//! let paused1 = compiled.invoke(json!({"input": "..."})).await?;
//! // ... review draft ...
//! let paused2 = compiled.resume().await?;
//!
//! // Second interrupt: before notify
//! // ... review who will be notified ...
//! let final_result = compiled.resume().await?;
//! ```
//!
//! ## Pattern 2: Conditional Interrupt Based on State
//!
//! Only interrupt when certain conditions are met:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, InterruptConfig, Command};
//! use serde_json::json;
//!
//! graph.add_node("risk_check", |state| {
//!     Box::pin(async move {
//!         let risk_score = calculate_risk(&state);
//!
//!         if risk_score > 0.7 {
//!             // High risk - route to manual_review (which has interrupt)
//!             Ok(Command::new()
//!                 .with_update(json!({"risk": risk_score}))
//!                 .with_goto("manual_review"))
//!         } else {
//!             // Low risk - auto approve
//!             Ok(Command::new()
//!                 .with_update(json!({"risk": risk_score, "approved": true}))
//!                 .with_goto("execute"))
//!         }
//!     })
//! });
//!
//! // Only manual_review has interrupt configured
//! let config = InterruptConfig::new()
//!     .with_interrupt_before(vec!["manual_review".to_string()]);
//! ```
//!
//! ## Pattern 3: Interrupt with State Modification
//!
//! Modify state during the pause (e.g., add approval metadata):
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, InterruptConfig};
//! use serde_json::json;
//!
//! let config = InterruptConfig::new()
//!     .with_interrupt_before(vec!["execute_transaction".to_string()]);
//!
//! let compiled = graph.compile_with_interrupts(config)?;
//!
//! // Execute until interrupt
//! let paused = compiled.invoke(json!({
//!     "transaction": {"amount": 10000, "to": "account-123"}
//! })).await?;
//!
//! // Human reviews and modifies
//! let mut modified_state = paused.state.clone();
//! modified_state["approved_by"] = json!("user@example.com");
//! modified_state["approved_at"] = json!(chrono::Utc::now().to_rfc3339());
//! modified_state["transaction"]["amount"] = json!(9500); // Adjust amount
//!
//! // Resume with modified state
//! let result = compiled.resume_with_state(modified_state).await?;
//! ```
//!
//! ## Pattern 4: Time-Travel Debugging
//!
//! Use interrupts with checkpoints to replay and debug execution:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, InterruptConfig};
//!
//! // Interrupt after each node to create checkpoints
//! let config = InterruptConfig::new()
//!     .with_interrupt_after_all();
//!
//! let compiled = graph.compile_with_interrupts(config)?;
//!
//! let mut checkpoints = Vec::new();
//! let mut state = initial_state;
//!
//! loop {
//!     let result = compiled.resume_with_state(state.clone()).await?;
//!
//!     if result.is_interrupted {
//!         let interrupt = result.interrupt.unwrap();
//!         checkpoints.push((interrupt.checkpoint_id.clone(), result.state.clone()));
//!         println!("Checkpoint at {}: {:?}", interrupt.node, result.state);
//!         state = result.state;
//!     } else {
//!         break;
//!     }
//! }
//!
//! // Later: Load any checkpoint to replay from that point
//! let (checkpoint_id, checkpoint_state) = &checkpoints[2];
//! let replayed = compiled.load_checkpoint(checkpoint_id).await?;
//! ```
//!
//! ## Pattern 5: A/B Testing with Human Feedback
//!
//! Generate multiple options and let humans choose:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, InterruptConfig, Command, Send};
//! use serde_json::json;
//!
//! graph.add_node("generate_options", |state| {
//!     Box::pin(async move {
//!         // Generate 3 different approaches
//!         let sends = vec![
//!             Send::new("approach_a", json!({"method": "A"})),
//!             Send::new("approach_b", json!({"method": "B"})),
//!             Send::new("approach_c", json!({"method": "C"})),
//!         ];
//!         Ok(Command::new().with_goto(sends))
//!     })
//! });
//!
//! graph.add_node("select_best", |state| {
//!     Box::pin(async move {
//!         Ok(json!({
//!             "options": state["results"],
//!             "awaiting_selection": true
//!         }))
//!     })
//! });
//!
//! // Interrupt before selection to let human choose
//! let config = InterruptConfig::new()
//!     .with_interrupt_before(vec!["select_best".to_string()]);
//!
//! let compiled = graph.compile_with_interrupts(config)?;
//!
//! let paused = compiled.invoke(initial_state).await?;
//!
//! // Human reviews options and selects
//! let options = &paused.state["options"];
//! let selected = human_selects(options); // UI interaction
//!
//! let modified = json!({"selected": selected});
//! let result = compiled.resume_with_state(modified).await?;
//! ```
//!
//! # Configuration Options
//!
//! ## Interrupt Before Nodes
//!
//! Pause **before** a node executes (state is from previous node):
//!
//! ```rust,ignore
//! let config = InterruptConfig::new()
//!     .with_interrupt_before(vec![
//!         "critical_action".to_string(),
//!         "delete_data".to_string(),
//!     ]);
//! ```
//!
//! **Use cases:**
//! - Approval before executing dangerous operations
//! - Input validation before processing
//! - Gate-keeping sensitive actions
//!
//! ## Interrupt After Nodes
//!
//! Pause **after** a node completes (state includes node's output):
//!
//! ```rust,ignore
//! let config = InterruptConfig::new()
//!     .with_interrupt_after(vec![
//!         "llm_generation".to_string(),
//!         "data_transformation".to_string(),
//!     ]);
//! ```
//!
//! **Use cases:**
//! - Review LLM outputs before continuing
//! - Validate transformation results
//! - Quality assurance checkpoints
//!
//! ## Interrupt All Nodes
//!
//! Debug mode - pause at every node:
//!
//! ```rust,ignore
//! // Pause before every node
//! let debug_before = InterruptConfig::new()
//!     .with_interrupt_before_all();
//!
//! // Pause after every node
//! let debug_after = InterruptConfig::new()
//!     .with_interrupt_after_all();
//!
//! // Both before AND after (very granular)
//! let debug_all = InterruptConfig::new()
//!     .with_interrupt_before_all()
//!     .with_interrupt_after_all();
//! ```
//!
//! **Use cases:**
//! - Step-through debugging
//! - Understanding graph execution flow
//! - Development and testing
//!
//! ## Combined Configuration
//!
//! ```rust,ignore
//! let config = InterruptConfig::new()
//!     .with_interrupt_before(vec!["approve".to_string()])
//!     .with_interrupt_after(vec!["llm_call".to_string()]);
//! ```
//!
//! # Interrupt State and Metadata
//!
//! Each interrupt creates an [`InterruptState`] with rich metadata:
//!
//! ```rust,ignore
//! pub struct InterruptState {
//!     pub interrupt_id: String,        // Unique ID for this interrupt
//!     pub thread_id: String,            // Execution thread
//!     pub node: NodeId,                 // Node that triggered interrupt
//!     pub when: InterruptWhen,          // Before or After
//!     pub step: usize,                  // Superstep number
//!     pub checkpoint_id: Option<String>, // Associated checkpoint
//!     pub metadata: HashMap<String, Value>, // Custom metadata
//!     pub timestamp: DateTime<Utc>,     // When interrupted
//! }
//! ```
//!
//! **Add custom metadata:**
//!
//! ```rust,ignore
//! let interrupt = InterruptState::new(
//!     thread_id,
//!     "approval_node".to_string(),
//!     InterruptWhen::Before,
//!     step,
//!     checkpoint_id,
//! )
//! .with_metadata("reason".to_string(), json!("high risk transaction"))
//! .with_metadata("reviewer".to_string(), json!("security-team"));
//! ```
//!
//! # Resumption Strategies
//!
//! ## Resume with Same State
//!
//! Continue execution with unmodified state:
//!
//! ```rust,ignore
//! let paused = compiled.invoke(initial_state).await?;
//! // ... human reviews but doesn't change anything ...
//! let result = compiled.resume().await?;
//! ```
//!
//! ## Resume with Modified State
//!
//! Update state before continuing:
//!
//! ```rust,ignore
//! let paused = compiled.invoke(initial_state).await?;
//! let mut modified = paused.state.clone();
//! modified["approved"] = json!(true);
//! modified["approver"] = json!("user@example.com");
//! let result = compiled.resume_with_state(modified).await?;
//! ```
//!
//! ## Resume Specific Interrupt by ID
//!
//! When multiple interrupts exist, resume a specific one:
//!
//! ```rust,ignore
//! let interrupt_id = paused.interrupt.unwrap().interrupt_id;
//! let result = compiled.resume_interrupt(interrupt_id, modified_state).await?;
//! ```
//!
//! # Integration with Checkpointing
//!
//! Interrupts integrate seamlessly with the checkpoint system:
//!
//! - **Auto-checkpointing**: Checkpoint saved when interrupt occurs
//! - **Persistence**: Interrupted state persists across application restarts
//! - **Time-travel**: Load checkpoint to resume from any interrupt point
//! - **Replay**: Re-execute from checkpoint with different state
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, InterruptConfig};
//! use langgraph_checkpoint::InMemoryCheckpoint;
//!
//! let checkpointer = InMemoryCheckpoint::new();
//!
//! let config = InterruptConfig::new()
//!     .with_interrupt_before(vec!["approve".to_string()]);
//!
//! let compiled = graph
//!     .compile_with_interrupts(config)?
//!     .with_checkpointer(checkpointer);
//!
//! // Execute and interrupt
//! let paused = compiled.invoke(initial_state).await?;
//!
//! // Application can restart here...
//! // On restart, load checkpoint and resume
//! let checkpoint_id = paused.interrupt.unwrap().checkpoint_id.unwrap();
//! let restored = compiled.load_checkpoint(&checkpoint_id).await?;
//! let result = compiled.resume().await?;
//! ```
//!
//! # Performance Considerations
//!
//! ## Interrupt Overhead
//!
//! - **Minimal runtime cost**: Interrupts are checked only at node boundaries
//! - **Checkpoint cost**: Saving checkpoint on interrupt (depends on state size)
//! - **No overhead when not configured**: Zero cost if no interrupts configured
//!
//! ## Optimization Strategies
//!
//! 1. **Selective interrupts**: Only interrupt critical nodes, not all
//! 2. **Batch operations**: Group multiple approvals into single interrupt
//! 3. **Async checkpointing**: Use async checkpoint savers to avoid blocking
//! 4. **State size**: Keep state minimal to reduce checkpoint serialization cost
//!
//! ## Scalability
//!
//! | Scenario | Performance Impact |
//! |----------|-------------------|
//! | No interrupts configured | ⚡ Zero overhead |
//! | Interrupt 1-5 nodes | ⚡ Negligible (<1ms per check) |
//! | Interrupt all nodes | ⚡⚡ Low (adds checkpoint saves) |
//! | Large state (>1MB) | ⚡⚡⚡ Moderate (serialization cost) |
//!
//! # Best Practices
//!
//! 1. **Use Interrupts for Critical Actions** - Reserve interrupts for dangerous or high-value operations that need human oversight
//!
//! 2. **Combine with Checkpointing** - Always use a checkpointer with interrupts to enable persistence across restarts
//!
//! 3. **Add Metadata** - Include context in interrupt metadata (reason, reviewer, risk score) for audit trails
//!
//! 4. **Handle Resume Errors** - Properly handle cases where resume fails (invalid state, expired session)
//!
//! 5. **Timeout Interrupted Sessions** - Implement timeout logic for long-paused executions
//!
//! 6. **Test Both Paths** - Test both approval and rejection flows in your workflows
//!
//! 7. **Use interrupt_before for Gates** - Use `interrupt_before` when you need to prevent an action entirely
//!
//! 8. **Use interrupt_after for Review** - Use `interrupt_after` when you want to review node output before continuing
//!
//! # Comparison with Python LangGraph
//!
//! | Python LangGraph | rLangGraph | Notes |
//! |------------------|------------|-------|
//! | `interrupt_before=["node"]` | `InterruptConfig::new().with_interrupt_before(vec!["node"])` | Same concept |
//! | `interrupt_after=["node"]` | `InterruptConfig::new().with_interrupt_after(vec!["node"])` | Same concept |
//! | `interrupt_before="*"` | `.with_interrupt_before_all()` | Wildcard as method |
//! | `interrupt_after="*"` | `.with_interrupt_after_all()` | Wildcard as method |
//! | `graph.stream(...).get()` | `compiled.invoke(...).await?` | Async execution |
//! | Check `__interrupt__` | Check `result.is_interrupted` | Explicit field |
//! | `graph.update_state(...)` | `compiled.resume_with_state(...)` | Resume with modified state |
//! | Interrupt stored in checkpoint | Same - `InterruptState` in checkpoint | Compatible format |
//!
//! # Error Handling
//!
//! Common interrupt errors and how to handle them:
//!
//! ```rust,ignore
//! use langgraph_core::InterruptError;
//!
//! match compiled.resume().await {
//!     Ok(result) => println!("Resumed successfully"),
//!     Err(e) => match e.downcast_ref::<InterruptError>() {
//!         Some(InterruptError::NotInterrupted) => {
//!             println!("No active interrupt to resume from");
//!         }
//!         Some(InterruptError::NoInterruptedExecution { thread_id }) => {
//!             println!("No interrupted execution for thread: {}", thread_id);
//!         }
//!         Some(InterruptError::InvalidState(msg)) => {
//!             println!("Invalid interrupt state: {}", msg);
//!         }
//!         _ => println!("Other error: {}", e),
//!     }
//! }
//! ```
//!
//! # See Also
//!
//! - [`inline_interrupt`](crate::inline_interrupt) - Node-level interrupt function for fine-grained control
//! - [`Command`](crate::command) - Dynamic graph control with resume support
//! - [`CompiledGraph`](crate::compiled) - Graph execution runtime that processes interrupts
//! - [`StateGraph`](crate::builder::StateGraph) - Graph construction with interrupt configuration
//! - [Checkpoint system](langgraph_checkpoint) - Persistence layer for interrupted state

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

use crate::graph::NodeId;

/// Error types for interrupt operations
#[derive(Debug, Error)]
pub enum InterruptError {
    #[error("Graph execution was interrupted before node: {node}")]
    InterruptedBefore { node: NodeId },

    #[error("Graph execution was interrupted after node: {node}")]
    InterruptedAfter { node: NodeId },

    #[error("No interrupted execution found for thread: {thread_id}")]
    NoInterruptedExecution { thread_id: String },

    #[error("Cannot resume: execution is not in interrupted state")]
    NotInterrupted,

    #[error("Invalid interrupt state: {0}")]
    InvalidState(String),
}

/// Configuration for graph interrupts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InterruptConfig {
    /// Nodes to interrupt before execution
    pub interrupt_before: Vec<NodeId>,

    /// Nodes to interrupt after execution
    pub interrupt_after: Vec<NodeId>,

    /// Whether to interrupt before all nodes
    pub interrupt_before_all: bool,

    /// Whether to interrupt after all nodes
    pub interrupt_after_all: bool,
}

impl InterruptConfig {
    /// Create a new interrupt configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set nodes to interrupt before
    pub fn with_interrupt_before(mut self, nodes: Vec<NodeId>) -> Self {
        self.interrupt_before = nodes;
        self
    }

    /// Set nodes to interrupt after
    pub fn with_interrupt_after(mut self, nodes: Vec<NodeId>) -> Self {
        self.interrupt_after = nodes;
        self
    }

    /// Interrupt before all nodes (equivalent to Python's interrupt_before="*")
    pub fn with_interrupt_before_all(mut self) -> Self {
        self.interrupt_before_all = true;
        self
    }

    /// Interrupt after all nodes (equivalent to Python's interrupt_after="*")
    pub fn with_interrupt_after_all(mut self) -> Self {
        self.interrupt_after_all = true;
        self
    }

    /// Check if should interrupt before a specific node
    pub fn should_interrupt_before(&self, node: &str) -> bool {
        self.interrupt_before_all || self.interrupt_before.iter().any(|n| n == node)
    }

    /// Check if should interrupt after a specific node
    pub fn should_interrupt_after(&self, node: &str) -> bool {
        self.interrupt_after_all || self.interrupt_after.iter().any(|n| n == node)
    }
}

/// State of an interrupted execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptState {
    /// Unique ID for this interrupt
    pub interrupt_id: String,

    /// Thread ID this interrupt belongs to
    pub thread_id: String,

    /// The node that caused the interrupt
    pub node: NodeId,

    /// Whether the interrupt occurred before or after the node
    pub when: InterruptWhen,

    /// Current step number when interrupted
    pub step: usize,

    /// Checkpoint ID at the time of interrupt
    pub checkpoint_id: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Timestamp when interrupted
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl InterruptState {
    /// Create a new interrupt state
    pub fn new(
        thread_id: String,
        node: NodeId,
        when: InterruptWhen,
        step: usize,
        checkpoint_id: Option<String>,
    ) -> Self {
        Self {
            interrupt_id: Uuid::new_v4().to_string(),
            thread_id,
            node,
            when,
            step,
            checkpoint_id,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Add metadata to the interrupt state
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// When the interrupt occurred
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InterruptWhen {
    /// Interrupted before node execution
    Before,
    /// Interrupted after node execution
    After,
}

/// Tracks interrupt state across graph execution
#[derive(Debug, Default)]
pub struct InterruptTracker {
    /// Current interrupt state, if any
    current_interrupt: Option<InterruptState>,

    /// History of interrupts for this execution
    interrupt_history: Vec<InterruptState>,

    /// Whether we're currently resuming from an interrupt
    resuming: bool,
}

impl InterruptTracker {
    /// Create a new interrupt tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if execution is currently interrupted
    pub fn is_interrupted(&self) -> bool {
        self.current_interrupt.is_some()
    }

    /// Check if we're resuming from an interrupt
    pub fn is_resuming(&self) -> bool {
        self.resuming
    }

    /// Record an interrupt
    pub fn interrupt(
        &mut self,
        thread_id: String,
        node: NodeId,
        when: InterruptWhen,
        step: usize,
        checkpoint_id: Option<String>,
    ) {
        let interrupt = InterruptState::new(thread_id, node, when, step, checkpoint_id);
        self.interrupt_history.push(interrupt.clone());
        self.current_interrupt = Some(interrupt);
    }

    /// Get the current interrupt state
    pub fn current_interrupt(&self) -> Option<&InterruptState> {
        self.current_interrupt.as_ref()
    }

    /// Clear the current interrupt and prepare to resume
    pub fn resume(&mut self) -> Result<(), InterruptError> {
        if self.current_interrupt.is_none() {
            return Err(InterruptError::NotInterrupted);
        }

        self.current_interrupt = None;
        self.resuming = true;
        Ok(())
    }

    /// Mark resumption as complete
    pub fn finish_resuming(&mut self) {
        self.resuming = false;
    }

    /// Get the interrupt history
    pub fn history(&self) -> &[InterruptState] {
        &self.interrupt_history
    }

    /// Clear all interrupt state
    pub fn reset(&mut self) {
        self.current_interrupt = None;
        self.interrupt_history.clear();
        self.resuming = false;
    }
}

/// Helper to determine if a node should be interrupted
pub fn should_interrupt(
    config: &InterruptConfig,
    node: &str,
    when: InterruptWhen,
    any_updates_since_last_interrupt: bool,
) -> bool {
    if !any_updates_since_last_interrupt {
        return false;
    }

    match when {
        InterruptWhen::Before => config.should_interrupt_before(node),
        InterruptWhen::After => config.should_interrupt_after(node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_config_builder() {
        let config = InterruptConfig::new()
            .with_interrupt_before(vec!["node1".to_string(), "node2".to_string()])
            .with_interrupt_after(vec!["node3".to_string()]);

        assert!(config.should_interrupt_before("node1"));
        assert!(config.should_interrupt_before("node2"));
        assert!(!config.should_interrupt_before("node3"));

        assert!(config.should_interrupt_after("node3"));
        assert!(!config.should_interrupt_after("node1"));
    }

    #[test]
    fn test_interrupt_before_all() {
        let config = InterruptConfig::new().with_interrupt_before_all();

        assert!(config.should_interrupt_before("any_node"));
        assert!(config.should_interrupt_before("another_node"));
    }

    #[test]
    fn test_interrupt_after_all() {
        let config = InterruptConfig::new().with_interrupt_after_all();

        assert!(config.should_interrupt_after("any_node"));
        assert!(config.should_interrupt_after("another_node"));
    }

    #[test]
    fn test_interrupt_state_creation() {
        let state = InterruptState::new(
            "thread-1".to_string(),
            "test_node".to_string(),
            InterruptWhen::Before,
            5,
            Some("checkpoint-123".to_string()),
        );

        assert_eq!(state.thread_id, "thread-1");
        assert_eq!(state.node, "test_node");
        assert_eq!(state.when, InterruptWhen::Before);
        assert_eq!(state.step, 5);
        assert_eq!(state.checkpoint_id, Some("checkpoint-123".to_string()));
        assert!(!state.interrupt_id.is_empty());
    }

    #[test]
    fn test_interrupt_state_with_metadata() {
        let state = InterruptState::new(
            "thread-1".to_string(),
            "test_node".to_string(),
            InterruptWhen::After,
            3,
            None,
        )
        .with_metadata("reason".to_string(), serde_json::json!("manual review"));

        assert_eq!(
            state.metadata.get("reason"),
            Some(&serde_json::json!("manual review"))
        );
    }

    #[test]
    fn test_interrupt_tracker() {
        let mut tracker = InterruptTracker::new();

        assert!(!tracker.is_interrupted());
        assert!(!tracker.is_resuming());

        // Record an interrupt
        tracker.interrupt(
            "thread-1".to_string(),
            "node1".to_string(),
            InterruptWhen::Before,
            2,
            Some("checkpoint-1".to_string()),
        );

        assert!(tracker.is_interrupted());
        assert!(!tracker.is_resuming());

        let interrupt = tracker.current_interrupt().unwrap();
        assert_eq!(interrupt.node, "node1");
        assert_eq!(interrupt.when, InterruptWhen::Before);

        // Resume
        tracker.resume().unwrap();
        assert!(!tracker.is_interrupted());
        assert!(tracker.is_resuming());

        // Finish resuming
        tracker.finish_resuming();
        assert!(!tracker.is_resuming());

        // Check history
        assert_eq!(tracker.history().len(), 1);
    }

    #[test]
    fn test_interrupt_tracker_multiple_interrupts() {
        let mut tracker = InterruptTracker::new();

        tracker.interrupt(
            "thread-1".to_string(),
            "node1".to_string(),
            InterruptWhen::Before,
            1,
            None,
        );
        tracker.resume().unwrap();
        tracker.finish_resuming();

        tracker.interrupt(
            "thread-1".to_string(),
            "node2".to_string(),
            InterruptWhen::After,
            3,
            None,
        );

        assert_eq!(tracker.history().len(), 2);
        assert_eq!(tracker.history()[0].node, "node1");
        assert_eq!(tracker.history()[1].node, "node2");
    }

    #[test]
    fn test_resume_without_interrupt_fails() {
        let mut tracker = InterruptTracker::new();

        let result = tracker.resume();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), InterruptError::NotInterrupted));
    }

    #[test]
    fn test_should_interrupt_helper() {
        let config = InterruptConfig::new()
            .with_interrupt_before(vec!["node1".to_string()])
            .with_interrupt_after(vec!["node2".to_string()]);

        // With updates
        assert!(should_interrupt(
            &config,
            "node1",
            InterruptWhen::Before,
            true
        ));
        assert!(should_interrupt(
            &config,
            "node2",
            InterruptWhen::After,
            true
        ));

        // Without updates
        assert!(!should_interrupt(
            &config,
            "node1",
            InterruptWhen::Before,
            false
        ));
        assert!(!should_interrupt(
            &config,
            "node2",
            InterruptWhen::After,
            false
        ));

        // Wrong timing
        assert!(!should_interrupt(
            &config,
            "node1",
            InterruptWhen::After,
            true
        ));
        assert!(!should_interrupt(
            &config,
            "node2",
            InterruptWhen::Before,
            true
        ));
    }

    #[test]
    fn test_interrupt_tracker_reset() {
        let mut tracker = InterruptTracker::new();

        tracker.interrupt(
            "thread-1".to_string(),
            "node1".to_string(),
            InterruptWhen::Before,
            1,
            None,
        );
        tracker.resume().unwrap();

        assert_eq!(tracker.history().len(), 1);

        tracker.reset();

        assert!(!tracker.is_interrupted());
        assert!(!tracker.is_resuming());
        assert_eq!(tracker.history().len(), 0);
    }

    #[test]
    fn test_interrupt_serialization() {
        let state = InterruptState::new(
            "thread-1".to_string(),
            "test_node".to_string(),
            InterruptWhen::Before,
            5,
            Some("checkpoint-123".to_string()),
        );

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: InterruptState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.thread_id, state.thread_id);
        assert_eq!(deserialized.node, state.node);
        assert_eq!(deserialized.when, state.when);
        assert_eq!(deserialized.step, state.step);
    }
}
