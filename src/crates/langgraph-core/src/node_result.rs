//! Node execution result types
//!
//! This module defines the result types that nodes can return from execution.
//! Nodes can return either a simple state update or a Command for advanced control.

use crate::command::Command;
use serde_json::Value;

/// Result that a node can return from execution
///
/// Nodes can return either:
/// - A simple state value (JSON) that will be merged into the graph state
/// - A Command for advanced control (update, goto, resume)
///
/// # Example: Simple State Update
///
/// ```rust
/// use langgraph_core::NodeResult;
/// use serde_json::json;
///
/// // Return a simple state update
/// let result = NodeResult::State(json!({"count": 42}));
/// ```
///
/// # Example: Command with Navigation
///
/// ```rust
/// use langgraph_core::{NodeResult, Command};
/// use serde_json::json;
///
/// // Return a command to update state and navigate
/// let result = NodeResult::Command(
///     Command::new()
///         .with_update(json!({"status": "processed"}))
///         .with_goto("next_step")
/// );
/// ```
#[derive(Debug, Clone)]
pub enum NodeResult {
    /// Simple state value to be merged into graph state
    State(Value),

    /// Command with advanced control
    Command(Command),
}

impl NodeResult {
    /// Extract the state update from this result
    ///
    /// For State variant, returns the value directly.
    /// For Command variant, returns the command's update field.
    pub fn get_state_update(&self) -> Option<Value> {
        match self {
            NodeResult::State(value) => Some(value.clone()),
            NodeResult::Command(cmd) => cmd.update.clone(),
        }
    }

    /// Extract the Command if present
    pub fn get_command(&self) -> Option<&Command> {
        match self {
            NodeResult::State(_) => None,
            NodeResult::Command(cmd) => Some(cmd),
        }
    }

    /// Check if this result contains a goto directive
    pub fn has_goto(&self) -> bool {
        matches!(self, NodeResult::Command(cmd) if cmd.goto.is_some())
    }

    /// Check if this result contains a resume directive
    pub fn has_resume(&self) -> bool {
        matches!(self, NodeResult::Command(cmd) if cmd.resume.is_some())
    }

    /// Convert into a Command, creating one if needed
    pub fn into_command(self) -> Command {
        match self {
            NodeResult::State(value) => Command::new().with_update(value),
            NodeResult::Command(cmd) => cmd,
        }
    }
}

impl From<Value> for NodeResult {
    fn from(value: Value) -> Self {
        NodeResult::State(value)
    }
}

impl From<Command> for NodeResult {
    fn from(cmd: Command) -> Self {
        NodeResult::Command(cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_node_result_state() {
        let result = NodeResult::State(json!({"count": 42}));

        assert!(matches!(result, NodeResult::State(_)));
        assert_eq!(result.get_state_update(), Some(json!({"count": 42})));
        assert!(result.get_command().is_none());
        assert!(!result.has_goto());
        assert!(!result.has_resume());
    }

    #[test]
    fn test_node_result_command() {
        let cmd = Command::new()
            .with_update(json!({"status": "done"}))
            .with_goto("next");
        let result = NodeResult::Command(cmd);

        assert!(matches!(result, NodeResult::Command(_)));
        assert_eq!(result.get_state_update(), Some(json!({"status": "done"})));
        assert!(result.get_command().is_some());
        assert!(result.has_goto());
        assert!(!result.has_resume());
    }

    #[test]
    fn test_node_result_from_value() {
        let result: NodeResult = json!({"key": "value"}).into();
        assert!(matches!(result, NodeResult::State(_)));
    }

    #[test]
    fn test_node_result_from_command() {
        let cmd = Command::new().with_goto("target");
        let result: NodeResult = cmd.into();
        assert!(matches!(result, NodeResult::Command(_)));
    }

    #[test]
    fn test_node_result_into_command() {
        // State variant
        let result = NodeResult::State(json!({"data": 123}));
        let cmd = result.into_command();
        assert_eq!(cmd.update, Some(json!({"data": 123})));
        assert!(cmd.goto.is_none());

        // Command variant
        let original = Command::new().with_goto("next");
        let result = NodeResult::Command(original.clone());
        let cmd = result.into_command();
        assert!(cmd.goto.is_some());
    }

    #[test]
    fn test_node_result_has_resume() {
        let cmd = Command::new().with_resume(json!({"approved": true}));
        let result = NodeResult::Command(cmd);

        assert!(result.has_resume());
        assert!(!result.has_goto());
    }

    #[test]
    fn test_node_result_get_state_update_from_command() {
        let cmd = Command::new()
            .with_update(json!({"field": "value"}))
            .with_goto("somewhere");
        let result = NodeResult::Command(cmd);

        assert_eq!(result.get_state_update(), Some(json!({"field": "value"})));
    }

    #[test]
    fn test_node_result_command_without_update() {
        let cmd = Command::new().with_goto("next");
        let result = NodeResult::Command(cmd);

        assert_eq!(result.get_state_update(), None);
        assert!(result.has_goto());
    }
}
