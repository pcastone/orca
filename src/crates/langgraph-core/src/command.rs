//! Dynamic graph control and execution flow management
//!
//! This module provides the [`Command`] system for fine-grained control over graph execution.
//! Commands allow nodes to dynamically update state, navigate to specific nodes, resume from
//! interrupts, and create parallel tasks - enabling advanced patterns like map-reduce,
//! conditional workflows, and human-in-the-loop interactions.
//!
//! # Overview
//!
//! The Command system provides:
//!
//! - **State Updates** - Modify graph state from within nodes
//! - **Navigation** - Dynamically choose which nodes to execute next
//! - **Interrupt Resumption** - Provide values to resume from HITL interrupts
//! - **Dynamic Task Creation** - Spawn parallel tasks with custom state (via [`Send`])
//! - **Subgraph Control** - Target commands to parent or named subgraphs
//! - **Fluent API** - Chainable builder pattern for composing commands
//!
//! # Core Types
//!
//! - [`Command`] - Main control structure for graph execution
//! - [`GotoTarget`] - Navigation destinations (nodes or Send commands)
//! - [`ResumeValue`] - Values to resume from interrupts
//! - [`CommandGraph`] - Target graph for command execution
//! - [`Send`] - Dynamic task creation with custom state (see [`send`](crate::send) module)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Node Returns Command                                        │
//! │  ┌────────────────────────────────────────────────────┐    │
//! │  │  Command::new()                                     │    │
//! │  │    .with_update(state_changes)                      │    │
//! │  │    .with_goto(next_node)                            │    │
//! │  │    .with_resume(interrupt_value)                    │    │
//! │  └────────────┬───────────────────────────────────────┘    │
//! └───────────────┼──────────────────────────────────────────────┘
//!                 ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Pregel Execution Engine                                     │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
//! │  │ 1. Apply     │  │ 2. Resume    │  │ 3. Navigate  │     │
//! │  │    update    │→ │    interrupts│→ │    to goto   │     │
//! │  └──────────────┘  └──────────────┘  └──────────────┘     │
//! │                                                              │
//! │  Special case: goto = Send/Sends                            │
//! │  ┌──────────────────────────────────────────────────┐      │
//! │  │  Create parallel tasks with custom state         │      │
//! │  │  • Map-reduce patterns                            │      │
//! │  │  • Dynamic fanout                                 │      │
//! │  │  • Conditional branching with state               │      │
//! │  └──────────────────────────────────────────────────┘      │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Basic Navigation
//!
//! ```rust,ignore
//! use langgraph_core::Command;
//! use serde_json::json;
//!
//! async fn router_node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     // Decide next node based on state
//!     let status = state["status"].as_str().unwrap();
//!
//!     let next = if status == "pending" {
//!         "process"
//!     } else {
//!         "finalize"
//!     };
//!
//!     Ok(Command::new().with_goto(next))
//! }
//! ```
//!
//! ## Update State and Navigate
//!
//! ```rust,ignore
//! use langgraph_core::Command;
//! use serde_json::json;
//!
//! async fn process_node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     // Process data
//!     let result = do_processing(&state).await?;
//!
//!     // Update state and move to next node
//!     Ok(Command::new()
//!         .with_update(json!({
//!             "result": result,
//!             "processed": true
//!         }))
//!         .with_goto("next_step"))
//! }
//! ```
//!
//! ## Resume from Interrupt
//!
//! ```rust,ignore
//! use langgraph_core::Command;
//! use serde_json::json;
//!
//! async fn approval_node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     // Wait for human approval (graph will interrupt here)
//!     // When resumed, the approval value will be in the interrupt's resume data
//!
//!     Ok(Command::new()
//!         .with_resume(json!({"approved": true}))
//!         .with_goto("continue"))
//! }
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Conditional Branching
//!
//! Dynamically route to different nodes based on runtime conditions:
//!
//! ```rust,ignore
//! use langgraph_core::Command;
//! use serde_json::json;
//!
//! async fn router(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     let score = state["confidence_score"].as_f64().unwrap();
//!
//!     let next = if score > 0.8 {
//!         "auto_approve"
//!     } else if score > 0.5 {
//!         "manual_review"
//!     } else {
//!         "reject"
//!     };
//!
//!     Ok(Command::new()
//!         .with_update(json!({"routing_decision": next}))
//!         .with_goto(next))
//! }
//! ```
//!
//! ## Pattern 2: Map-Reduce with Send
//!
//! Process multiple items in parallel, then collect results:
//!
//! ```rust,ignore
//! use langgraph_core::{Command, Send};
//! use serde_json::json;
//!
//! // Map phase: Create parallel tasks
//! async fn map_node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     let items = state["items"].as_array().unwrap();
//!
//!     let sends: Vec<Send> = items.iter().map(|item| {
//!         Send::new("process_item", json!({
//!             "item": item,
//!             "batch_id": state["batch_id"]
//!         }))
//!     }).collect();
//!
//!     Ok(Command::new()
//!         .with_update(json!({"status": "mapping"}))
//!         .with_goto(sends))
//! }
//!
//! // Reduce phase: Collect results
//! async fn reduce_node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     let results = state["results"].as_array().unwrap();
//!     let total = results.iter().filter_map(|v| v.as_i64()).sum::<i64>();
//!
//!     Ok(Command::new()
//!         .with_update(json!({
//!             "total": total,
//!             "status": "complete"
//!         }))
//!         .with_goto("finalize"))
//! }
//! ```
//!
//! ## Pattern 3: Human-in-the-Loop Approval
//!
//! Interrupt execution for human review, then resume with decision:
//!
//! ```rust,ignore
//! use langgraph_core::Command;
//! use serde_json::json;
//!
//! async fn review_node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     // Node requests approval by returning a command that will cause interrupt
//!     // The graph will pause here until resumed with approval data
//!
//!     Ok(Command::new()
//!         .with_update(json!({"awaiting_approval": true}))
//!         .with_goto("approval_gate"))
//! }
//!
//! async fn approval_gate(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     // This node is configured as an interrupt node
//!     // When resumed, check the approval value
//!
//!     let approved = state["approval"]["approved"].as_bool().unwrap_or(false);
//!
//!     let next = if approved {
//!         "execute_action"
//!     } else {
//!         "cancel_action"
//!     };
//!
//!     Ok(Command::new().with_goto(next))
//! }
//! ```
//!
//! ## Pattern 4: Multi-Path Fanout
//!
//! Execute multiple nodes in parallel with shared state:
//!
//! ```rust,ignore
//! use langgraph_core::Command;
//! use serde_json::json;
//!
//! async fn fanout_node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     // Execute multiple analysis nodes in parallel
//!     Ok(Command::new()
//!         .with_update(json!({"stage": "analysis"}))
//!         .with_goto(vec![
//!             "sentiment_analysis".to_string(),
//!             "entity_extraction".to_string(),
//!             "classification".to_string(),
//!         ]))
//! }
//! ```
//!
//! ## Pattern 5: Dynamic Retry with Backoff
//!
//! Implement retry logic with state tracking:
//!
//! ```rust,ignore
//! use langgraph_core::Command;
//! use serde_json::json;
//!
//! async fn retry_node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     let attempt = state["retry_count"].as_u64().unwrap_or(0);
//!     let max_retries = 3;
//!
//!     match perform_operation(&state).await {
//!         Ok(result) => {
//!             Ok(Command::new()
//!                 .with_update(json!({"result": result, "status": "success"}))
//!                 .with_goto("next_step"))
//!         }
//!         Err(err) if attempt < max_retries => {
//!             Ok(Command::new()
//!                 .with_update(json!({
//!                     "retry_count": attempt + 1,
//!                     "last_error": err.to_string()
//!                 }))
//!                 .with_goto("retry_node")) // Retry same node
//!         }
//!         Err(err) => {
//!             Ok(Command::new()
//!                 .with_update(json!({"error": err.to_string(), "status": "failed"}))
//!                 .with_goto("error_handler"))
//!         }
//!     }
//! }
//! ```
//!
//! ## Pattern 6: Subgraph Communication
//!
//! Send commands to parent or named subgraphs:
//!
//! ```rust,ignore
//! use langgraph_core::{Command, CommandGraph};
//! use serde_json::json;
//!
//! async fn subgraph_node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     // Process in subgraph, then update parent
//!     let result = process_locally(&state).await?;
//!
//!     Ok(Command::new()
//!         .with_graph(CommandGraph::Parent)
//!         .with_update(json!({"subgraph_result": result}))
//!         .with_goto("parent_continue"))
//! }
//! ```
//!
//! ## Pattern 7: State Accumulation
//!
//! Progressively build up state across multiple nodes:
//!
//! ```rust,ignore
//! use langgraph_core::Command;
//! use serde_json::json;
//!
//! async fn accumulator_node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     let mut history = state["history"].as_array()
//!         .map(|a| a.clone())
//!         .unwrap_or_default();
//!
//!     history.push(json!({
//!         "step": state["current_step"],
//!         "timestamp": chrono::Utc::now().to_rfc3339()
//!     }));
//!
//!     Ok(Command::new()
//!         .with_update(json!({
//!             "history": history,
//!             "step_count": history.len()
//!         }))
//!         .with_goto("next_step"))
//! }
//! ```
//!
//! # Command Fields
//!
//! ## Update Field
//!
//! The `update` field modifies the graph's state. It accepts any JSON value:
//!
//! - **Object**: Each field is merged into state (shallow merge)
//! - **Other**: Stored in special `__root__` field (rarely used)
//!
//! ```rust,ignore
//! // Object update (typical)
//! Command::new().with_update(json!({
//!     "field1": "value1",
//!     "field2": 42
//! }))
//!
//! // Single value (edge case)
//! Command::new().with_update(json!("some_value")) // Stored in __root__
//! ```
//!
//! ## Goto Field
//!
//! The `goto` field specifies navigation targets:
//!
//! | Type | Example | Behavior |
//! |------|---------|----------|
//! | Single node | `"next"` | Execute one node |
//! | Multiple nodes | `vec!["a", "b"]` | Execute in parallel |
//! | Single Send | `Send::new("n", state)` | Execute with custom state |
//! | Multiple Sends | `vec![Send::new(...), ...]` | Map-reduce pattern |
//!
//! ## Resume Field
//!
//! The `resume` field provides values to resume from interrupts:
//!
//! - **Single Value**: Resume next interrupt with this value
//! - **By Interrupt ID**: Resume specific interrupts by ID
//!
//! ```rust,ignore
//! // Resume next interrupt
//! Command::new().with_resume(json!({"approved": true}))
//!
//! // Resume specific interrupts
//! let mut resumes = HashMap::new();
//! resumes.insert("interrupt-1".to_string(), json!({"value": 1}));
//! resumes.insert("interrupt-2".to_string(), json!({"value": 2}));
//! Command::new().with_resume(resumes)
//! ```
//!
//! ## Graph Field
//!
//! The `graph` field targets which graph receives the command:
//!
//! - **Current** (default): Apply to current graph
//! - **Parent**: Apply to parent graph (from subgraph)
//! - **Named**: Apply to specific named subgraph
//!
//! # Integration with Send
//!
//! Commands integrate with [`Send`] to enable dynamic task creation. This is the foundation
//! for map-reduce patterns:
//!
//! ```rust,ignore
//! use langgraph_core::{Command, Send};
//! use serde_json::json;
//!
//! // Map phase: Split work
//! async fn map_phase(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     let items = vec![1, 2, 3, 4, 5];
//!
//!     let tasks: Vec<Send> = items.iter().map(|i| {
//!         Send::new("worker", json!({
//!             "item": i,
//!             "shared_context": state["context"]
//!         }))
//!     }).collect();
//!
//!     Ok(Command::new().with_goto(tasks))
//! }
//!
//! // Worker: Process individual items
//! async fn worker(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     let item = state["item"].as_i64().unwrap();
//!     let result = item * 2; // Example processing
//!
//!     Ok(Command::new()
//!         .with_update(json!({"result": result}))
//!         .with_goto("collect"))
//! }
//!
//! // Reduce phase: Aggregate results
//! async fn collect(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     // All worker results are collected here
//!     let total: i64 = state["results"].as_array()
//!         .unwrap()
//!         .iter()
//!         .filter_map(|v| v["result"].as_i64())
//!         .sum();
//!
//!     Ok(Command::new()
//!         .with_update(json!({"total": total}))
//!         .with_goto("finalize"))
//! }
//! ```
//!
//! See the [`send`](crate::send) module for more details on Send.
//!
//! # Performance Considerations
//!
//! ## Parallel Execution
//!
//! - **Multiple goto nodes**: Execute concurrently within same superstep
//! - **Send commands**: Each creates independent execution path
//! - **Batching**: Group related Sends to reduce coordination overhead
//!
//! ## State Updates
//!
//! - **Granular updates**: Only include changed fields in `update`
//! - **Shallow merge**: Updates are merged at top level only
//! - **Serialization cost**: Large state objects incur JSON serialization overhead
//!
//! ## Goto Target Selection
//!
//! | Pattern | When to Use | Performance |
//! |---------|-------------|-------------|
//! | Single node | Sequential flow | ⚡ Fast |
//! | Multiple nodes | Parallel independent work | ⚡⚡ Faster |
//! | Send commands | Parallel with custom state | ⚡⚡ Faster (+ overhead) |
//!
//! ## Memory Considerations
//!
//! - Each Send creates a new task with copied state
//! - Large fanouts (>100 Sends) may impact memory
//! - Consider batching or streaming for large datasets
//!
//! # Best Practices
//!
//! 1. **Prefer Commands for Control Flow** - Use Commands instead of conditional edges when you need dynamic routing based on runtime state
//!
//! 2. **Update Only What Changed** - Include only modified fields in `with_update()` to minimize serialization overhead
//!
//! 3. **Use Send for Map-Reduce** - Leverage Send commands when you need parallel processing with custom state per task
//!
//! 4. **Handle Empty Commands** - Check `is_empty()` if you conditionally build commands
//!
//! 5. **Type Safety** - Validate state structure before accessing nested fields
//!
//! 6. **Error Handling** - Use Result types and propagate errors properly instead of panicking
//!
//! 7. **Document Navigation** - Comment why specific goto targets are chosen for future maintainability
//!
//! # Comparison with Python LangGraph
//!
//! | Python LangGraph | rLangGraph | Notes |
//! |------------------|------------|-------|
//! | `Command(update={...})` | `Command::new().with_update(json!({...}))` | Rust uses fluent API |
//! | `Command(goto="node")` | `Command::new().with_goto("node")` | String or &str accepted |
//! | `Command(goto=["a", "b"])` | `Command::new().with_goto(vec!["a", "b"])` | Multiple nodes |
//! | `Command(goto=Send(...))` | `Command::new().with_goto(Send::new(...))` | Single Send |
//! | `Command(resume={...})` | `Command::new().with_resume(json!({...}))` | Interrupt resumption |
//! | `Command(graph="parent")` | `Command::new().with_graph(CommandGraph::Parent)` | Subgraph targeting |
//! | Chaining | `.with_update(...).with_goto(...)` | Same fluent pattern |
//!
//! # Errors
//!
//! Commands themselves don't fail - they're data structures. However, nodes returning Commands
//! should use Result types to propagate errors:
//!
//! ```rust,ignore
//! async fn node(state: Value) -> Result<Command, Box<dyn std::error::Error>> {
//!     // Processing that may fail
//!     let result = risky_operation(&state)?;
//!
//!     Ok(Command::new()
//!         .with_update(json!({"result": result}))
//!         .with_goto("next"))
//! }
//! ```
//!
//! # See Also
//!
//! - [`Send`](crate::send) - Dynamic task creation for map-reduce patterns
//! - [`StateGraph`](crate::builder::StateGraph) - Graph construction with nodes and edges
//! - [`CompiledGraph`](crate::compiled::CompiledGraph) - Execution runtime that processes Commands
//! - [`interrupt`](crate::interrupt) - Human-in-the-loop interrupt configuration
//! - [Pregel execution model](crate::pregel) - How Commands are processed in supersteps

use crate::send::Send;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Target graph for a command
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandGraph {
    /// Current graph (default)
    Current,
    /// Parent graph
    Parent,
    /// Named subgraph
    Named(String),
}

impl Default for CommandGraph {
    fn default() -> Self {
        CommandGraph::Current
    }
}

/// Navigation target for goto field
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GotoTarget {
    /// Single node name
    Node(String),
    /// Multiple nodes to execute
    Nodes(Vec<String>),
    /// Single Send command
    Send(Send),
    /// Multiple Send commands
    Sends(Vec<Send>),
}

impl From<String> for GotoTarget {
    fn from(node: String) -> Self {
        GotoTarget::Node(node)
    }
}

impl From<&str> for GotoTarget {
    fn from(node: &str) -> Self {
        GotoTarget::Node(node.to_string())
    }
}

impl From<Vec<String>> for GotoTarget {
    fn from(nodes: Vec<String>) -> Self {
        GotoTarget::Nodes(nodes)
    }
}

impl From<Send> for GotoTarget {
    fn from(send: Send) -> Self {
        GotoTarget::Send(send)
    }
}

impl From<Vec<Send>> for GotoTarget {
    fn from(sends: Vec<Send>) -> Self {
        GotoTarget::Sends(sends)
    }
}

/// Resume value for interrupts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResumeValue {
    /// Resume with a single value for the next interrupt
    Single(Value),
    /// Resume specific interrupts by ID
    ByInterruptId(HashMap<String, Value>),
}

impl From<Value> for ResumeValue {
    fn from(value: Value) -> Self {
        ResumeValue::Single(value)
    }
}

impl From<HashMap<String, Value>> for ResumeValue {
    fn from(map: HashMap<String, Value>) -> Self {
        ResumeValue::ByInterruptId(map)
    }
}

/// Command to control graph execution
///
/// Commands provide fine-grained control over graph execution, allowing nodes to:
///
/// 1. **Update State**: Modify the graph's state
/// 2. **Resume Interrupts**: Provide values to resume from human-in-the-loop interrupts
/// 3. **Navigate**: Choose which nodes to execute next
/// 4. **Send Dynamic Messages**: Create tasks with custom state (map-reduce patterns)
///
/// Commands are typically returned by nodes to control what happens next in the graph.
///
/// # Example: Basic Navigation
///
/// ```rust
/// use langgraph_core::Command;
/// use serde_json::json;
///
/// // Simple goto
/// let cmd = Command::new().with_goto("next_node");
///
/// // Update state and goto
/// let cmd = Command::new()
///     .with_update(json!({"result": 42}))
///     .with_goto("process");
/// ```
///
/// # Example: Map-Reduce Pattern
///
/// ```rust
/// use langgraph_core::{Command, Send};
/// use serde_json::json;
///
/// // Process multiple items in parallel
/// let items = vec![1, 2, 3];
/// let sends: Vec<Send> = items.iter()
///     .map(|i| Send::new("process_item", json!({"value": i})))
///     .collect();
///
/// let cmd = Command::new().with_goto(sends);
/// ```
///
/// # Example: Resume from Interrupt
///
/// ```rust
/// use langgraph_core::Command;
/// use serde_json::json;
///
/// // Resume with approval
/// let cmd = Command::new()
///     .with_resume(json!({"approved": true}))
///     .with_goto("continue_processing");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Command {
    /// Target graph (current, parent, or named)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<CommandGraph>,

    /// State update to apply
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<Value>,

    /// Value to resume from interrupt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<ResumeValue>,

    /// Navigation target (node names or Send commands)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goto: Option<GotoTarget>,
}

impl Command {
    /// Create a new empty command
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the target graph
    pub fn with_graph(mut self, graph: CommandGraph) -> Self {
        self.graph = Some(graph);
        self
    }

    /// Set the state update
    pub fn with_update(mut self, update: Value) -> Self {
        self.update = Some(update);
        self
    }

    /// Set the resume value
    pub fn with_resume(mut self, resume: impl Into<ResumeValue>) -> Self {
        self.resume = Some(resume.into());
        self
    }

    /// Set the goto target
    pub fn with_goto(mut self, goto: impl Into<GotoTarget>) -> Self {
        self.goto = Some(goto.into());
        self
    }

    /// Check if command is empty (no operations)
    pub fn is_empty(&self) -> bool {
        self.graph.is_none()
            && self.update.is_none()
            && self.resume.is_none()
            && self.goto.is_none()
    }

    /// Get update as list of (field, value) tuples
    ///
    /// This is used internally by the Pregel loop to apply state updates.
    pub fn update_as_tuples(&self) -> Vec<(String, Value)> {
        match &self.update {
            Some(Value::Object(obj)) => obj
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            Some(value) => vec![("__root__".to_string(), value.clone())],
            None => vec![],
        }
    }
}

/// Special constant for targeting parent graph
pub const PARENT: &str = "__parent__";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_command_new() {
        let cmd = Command::new();
        assert!(cmd.is_empty());
        assert!(cmd.graph.is_none());
        assert!(cmd.update.is_none());
        assert!(cmd.resume.is_none());
        assert!(cmd.goto.is_none());
    }

    #[test]
    fn test_command_with_update() {
        let cmd = Command::new().with_update(json!({"status": "done"}));
        assert!(!cmd.is_empty());
        assert_eq!(cmd.update, Some(json!({"status": "done"})));
    }

    #[test]
    fn test_command_with_goto_node() {
        let cmd = Command::new().with_goto("next_node");
        assert!(!cmd.is_empty());
        match cmd.goto {
            Some(GotoTarget::Node(node)) => assert_eq!(node, "next_node"),
            _ => panic!("Expected Node variant"),
        }
    }

    #[test]
    fn test_command_with_goto_send() {
        let send = Send::new("process", json!({"item": 1}));
        let cmd = Command::new().with_goto(send);

        match cmd.goto {
            Some(GotoTarget::Send(_)) => {}
            _ => panic!("Expected Send variant"),
        }
    }

    #[test]
    fn test_command_with_goto_sends() {
        let sends = vec![
            Send::new("process", json!({"item": 1})),
            Send::new("process", json!({"item": 2})),
        ];
        let cmd = Command::new().with_goto(sends);

        match cmd.goto {
            Some(GotoTarget::Sends(sends)) => assert_eq!(sends.len(), 2),
            _ => panic!("Expected Sends variant"),
        }
    }

    #[test]
    fn test_command_with_resume_single() {
        let cmd = Command::new().with_resume(json!({"approved": true}));
        match cmd.resume {
            Some(ResumeValue::Single(value)) => {
                assert_eq!(value, json!({"approved": true}));
            }
            _ => panic!("Expected Single variant"),
        }
    }

    #[test]
    fn test_command_with_resume_by_id() {
        let mut resume_map = HashMap::new();
        resume_map.insert("int-1".to_string(), json!({"approved": true}));
        resume_map.insert("int-2".to_string(), json!({"rejected": false}));

        let cmd = Command::new().with_resume(resume_map);
        match cmd.resume {
            Some(ResumeValue::ByInterruptId(map)) => {
                assert_eq!(map.len(), 2);
                assert_eq!(map["int-1"], json!({"approved": true}));
            }
            _ => panic!("Expected ByInterruptId variant"),
        }
    }

    #[test]
    fn test_command_with_graph() {
        let cmd = Command::new().with_graph(CommandGraph::Parent);
        assert_eq!(cmd.graph, Some(CommandGraph::Parent));
    }

    #[test]
    fn test_command_chaining() {
        let cmd = Command::new()
            .with_update(json!({"count": 1}))
            .with_goto("next")
            .with_graph(CommandGraph::Current);

        assert!(!cmd.is_empty());
        assert_eq!(cmd.update, Some(json!({"count": 1})));
        assert!(matches!(cmd.goto, Some(GotoTarget::Node(_))));
        assert_eq!(cmd.graph, Some(CommandGraph::Current));
    }

    #[test]
    fn test_command_update_as_tuples() {
        let cmd = Command::new().with_update(json!({
            "field1": "value1",
            "field2": 42
        }));

        let tuples = cmd.update_as_tuples();
        assert_eq!(tuples.len(), 2);
        assert!(tuples.contains(&("field1".to_string(), json!("value1"))));
        assert!(tuples.contains(&("field2".to_string(), json!(42))));
    }

    #[test]
    fn test_command_update_as_tuples_non_object() {
        let cmd = Command::new().with_update(json!("single_value"));
        let tuples = cmd.update_as_tuples();
        assert_eq!(tuples.len(), 1);
        assert_eq!(tuples[0].0, "__root__");
        assert_eq!(tuples[0].1, json!("single_value"));
    }

    #[test]
    fn test_command_serialization() {
        let cmd = Command::new()
            .with_update(json!({"status": "ok"}))
            .with_goto("next");

        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: Command = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.update, cmd.update);
        match (deserialized.goto, cmd.goto) {
            (Some(GotoTarget::Node(a)), Some(GotoTarget::Node(b))) => assert_eq!(a, b),
            _ => panic!("Serialization failed"),
        }
    }

    #[test]
    fn test_goto_target_from_conversions() {
        // From &str
        let target: GotoTarget = "node1".into();
        assert!(matches!(target, GotoTarget::Node(_)));

        // From String
        let target: GotoTarget = "node2".to_string().into();
        assert!(matches!(target, GotoTarget::Node(_)));

        // From Vec<String>
        let target: GotoTarget = vec!["node1".to_string(), "node2".to_string()].into();
        assert!(matches!(target, GotoTarget::Nodes(_)));

        // From Send
        let send = Send::new("process", json!({}));
        let target: GotoTarget = send.into();
        assert!(matches!(target, GotoTarget::Send(_)));

        // From Vec<Send>
        let sends = vec![Send::new("p1", json!({})), Send::new("p2", json!({}))];
        let target: GotoTarget = sends.into();
        assert!(matches!(target, GotoTarget::Sends(_)));
    }

    #[test]
    fn test_map_reduce_pattern() {
        // Simulate a map-reduce workflow
        let items = vec![1, 2, 3, 4, 5];
        let sends: Vec<Send> = items
            .iter()
            .map(|i| Send::new("process_item", json!({"value": i})))
            .collect();

        let cmd = Command::new()
            .with_update(json!({"status": "mapping"}))
            .with_goto(sends);

        match cmd.goto {
            Some(GotoTarget::Sends(sends)) => {
                assert_eq!(sends.len(), 5);
                assert_eq!(sends[0].node(), "process_item");
            }
            _ => panic!("Expected Sends variant"),
        }
    }

    #[test]
    fn test_command_graph_targets() {
        let cmd1 = Command::new().with_graph(CommandGraph::Current);
        assert_eq!(cmd1.graph, Some(CommandGraph::Current));

        let cmd2 = Command::new().with_graph(CommandGraph::Parent);
        assert_eq!(cmd2.graph, Some(CommandGraph::Parent));

        let cmd3 = Command::new().with_graph(CommandGraph::Named("subgraph".to_string()));
        assert_eq!(
            cmd3.graph,
            Some(CommandGraph::Named("subgraph".to_string()))
        );
    }
}
