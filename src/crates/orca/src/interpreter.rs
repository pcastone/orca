//! Action interpreter for tool execution
//!
//! The Interpreter module bridges LLM responses and tool execution by parsing,
//! validating, and coordinating tool calls from agent decisions.
//!
//! # Interpreter Responsibilities
//!
//! - **Parse Tool Calls**: Extract tool invocations from agent messages
//! - **Validate Actions**: Ensure tool calls are safe and well-formed
//! - **Coordinate Execution**: Manage tool execution lifecycle
//! - **Handle Results**: Process tool outputs for agent feedback
//! - **Error Recovery**: Gracefully handle tool failures
//!
//! # Interpretation Pipeline
//!
//! ```text
//! LLM Response → Parse → Validate → Execute → Results
//!     ↓            ↓         ↓          ↓         ↓
//! ToolCall   ActionCall  Policy   ToolBridge  ToolResult
//!  Message    Struct     Check     Execute     Message
//! ```
//!
//! ## Pipeline Stages
//!
//! 1. **Parse**: Extract tool calls from agent state (ToolCall messages)
//! 2. **Validate**: Check tool exists, validate parameters, apply policies
//! 3. **Execute**: Invoke tool via DirectToolBridge
//! 4. **Results**: Package execution results for agent consumption
//!
//! # Integration with TaskExecutor
//!
//! The TaskExecutor uses the Interpreter to:
//! - Extract tool calls from agent patterns (ReAct, Plan-Execute, etc.)
//! - Validate tool calls before execution
//! - Execute tools and feed results back to agents
//! - Handle execution errors gracefully
//!
//! The Interpreter is pattern-agnostic - it works with any agent pattern
//! that produces ToolCall messages.

use crate::error::{OrcaError, Result};
use crate::tools::DirectToolBridge;
use async_trait::async_trait;
use langgraph_core::messages::Message;
use langgraph_core::tool::ToolCall;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Parsed action call ready for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCall {
    /// Tool name to invoke
    pub tool_name: String,

    /// Tool parameters as JSON
    pub parameters: JsonValue,

    /// Tool call ID for result correlation
    pub call_id: String,
}

impl ActionCall {
    /// Create a new action call
    pub fn new(tool_name: impl Into<String>, parameters: JsonValue, call_id: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            parameters,
            call_id: call_id.into(),
        }
    }

    /// Create from a ToolCall message
    pub fn from_tool_call(tool_call: &ToolCall) -> Self {
        Self {
            tool_name: tool_call.name.clone(),
            parameters: tool_call.args.clone(),
            call_id: tool_call.id.clone(),
        }
    }
}

/// Action execution result
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// The original action call
    pub action: ActionCall,

    /// Execution success status
    pub success: bool,

    /// Result data (if successful)
    pub result: Option<JsonValue>,

    /// Error message (if failed)
    pub error: Option<String>,
}

impl ActionResult {
    /// Create a successful result
    pub fn success(action: ActionCall, result: JsonValue) -> Self {
        Self {
            action,
            success: true,
            result: Some(result),
            error: None,
        }
    }

    /// Create a failure result
    pub fn failure(action: ActionCall, error: impl Into<String>) -> Self {
        Self {
            action,
            success: false,
            result: None,
            error: Some(error.into()),
        }
    }

    /// Convert to a Message for agent feedback
    pub fn to_message(&self) -> Message {
        let content = if self.success {
            self.result
                .as_ref()
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string()
        } else {
            format!("Error: {}", self.error.as_deref().unwrap_or("Unknown error"))
        };

        // Create a tool result message
        Message::tool(content, &self.action.call_id)
            .with_name(&self.action.tool_name)
    }
}

/// Interpreter trait for action interpretation and execution
///
/// Implementors parse tool calls from agent messages, validate them,
/// and coordinate execution via DirectToolBridge.
#[async_trait]
pub trait Interpreter: Send + Sync {
    /// Parse tool calls from agent messages
    ///
    /// # Arguments
    /// * `messages` - Agent state messages to parse
    ///
    /// # Returns
    /// Vector of parsed ActionCall structs
    fn parse_actions(&self, messages: &[Message]) -> Result<Vec<ActionCall>>;

    /// Validate an action before execution
    ///
    /// # Arguments
    /// * `action` - The action to validate
    ///
    /// # Returns
    /// Ok(()) if valid, Err with details if invalid
    async fn validate_action(&self, action: &ActionCall) -> Result<()>;

    /// Execute an action via tool bridge
    ///
    /// # Arguments
    /// * `action` - The action to execute
    ///
    /// # Returns
    /// ActionResult with execution outcome
    async fn execute_action(&self, action: &ActionCall) -> Result<ActionResult>;

    /// Interpret and execute all actions from messages
    ///
    /// This is the main entry point that combines parse, validate, and execute.
    ///
    /// # Arguments
    /// * `messages` - Agent state messages
    ///
    /// # Returns
    /// Vector of ActionResults
    async fn interpret(&self, messages: &[Message]) -> Result<Vec<ActionResult>> {
        // Parse actions from messages
        let actions = self.parse_actions(messages)?;

        if actions.is_empty() {
            return Ok(Vec::new());
        }

        // Validate and execute each action
        let mut results = Vec::new();

        for action in actions {
            // Validate before execution
            if let Err(e) = self.validate_action(&action).await {
                results.push(ActionResult::failure(action, e.to_string()));
                continue;
            }

            // Execute the action
            match self.execute_action(&action).await {
                Ok(result) => results.push(result),
                Err(e) => results.push(ActionResult::failure(action, e.to_string())),
            }
        }

        Ok(results)
    }
}

/// Default interpreter implementation using DirectToolBridge
pub struct DefaultInterpreter {
    /// Tool bridge for execution
    tool_bridge: Arc<DirectToolBridge>,
}

impl DefaultInterpreter {
    /// Create a new default interpreter
    pub fn new(tool_bridge: Arc<DirectToolBridge>) -> Self {
        Self { tool_bridge }
    }
}

#[async_trait]
impl Interpreter for DefaultInterpreter {
    fn parse_actions(&self, messages: &[Message]) -> Result<Vec<ActionCall>> {
        let mut actions = Vec::new();

        // Iterate through messages looking for tool calls
        for message in messages {
            // Check if message has tool calls
            if let Some(tool_calls) = &message.tool_calls {
                // Parse each tool call in the message
                for tool_call in tool_calls {
                    // Validate JSON structure
                    if !tool_call.args.is_object() && !tool_call.args.is_null() {
                        return Err(OrcaError::ToolExecution(format!(
                            "Tool '{}' arguments must be a JSON object, got: {}",
                            tool_call.name,
                            tool_call.args
                        )));
                    }

                    // Create ActionCall from ToolCall
                    let action = ActionCall::from_tool_call(tool_call);
                    actions.push(action);
                }
            }
        }

        Ok(actions)
    }

    async fn validate_action(&self, action: &ActionCall) -> Result<()> {
        // 1. Validate tool exists in registry
        // This will return error if tool not found
        self.tool_bridge
            .get_tool_schema(&action.tool_name)
            .map_err(|e| {
                OrcaError::ToolExecution(format!(
                    "Tool '{}' not found in registry: {}",
                    action.tool_name, e
                ))
            })?;

        // 2. Validate arguments are properly formatted
        // Arguments must be a JSON object or null
        if !action.parameters.is_object() && !action.parameters.is_null() {
            return Err(OrcaError::ToolExecution(format!(
                "Tool '{}' requires object or null arguments, got: {}",
                action.tool_name,
                serde_json::to_string(&action.parameters).unwrap_or_else(|_| "invalid".to_string())
            )));
        }

        // 3. Policy constraints (allowlist/denylist) are enforced by the
        // ToolExecutor during execution via PolicyRegistry. The tool bridge
        // checks policies when actually running the tool.
        //
        // Additional pre-execution policy checks could be added here if needed,
        // but the current architecture handles this at the execution layer.

        Ok(())
    }

    async fn execute_action(&self, action: &ActionCall) -> Result<ActionResult> {
        // Execute tool via DirectToolBridge
        match self.tool_bridge.execute_tool(&action.tool_name, action.parameters.clone()).await {
            Ok(result) => {
                // Successful execution
                Ok(ActionResult::success(action.clone(), result))
            }
            Err(e) => {
                // Tool execution failed
                Ok(ActionResult::failure(action.clone(), e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_call_creation() {
        let action = ActionCall::new(
            "file_read",
            serde_json::json!({"path": "test.txt"}),
            "call-123",
        );

        assert_eq!(action.tool_name, "file_read");
        assert_eq!(action.call_id, "call-123");
        assert_eq!(action.parameters["path"], "test.txt");
    }

    #[test]
    fn test_action_call_from_tool_call() {
        let tool_call = ToolCall {
            id: "call-456".to_string(),
            name: "bash".to_string(),
            args: serde_json::json!({"command": "ls"}),
        };

        let action = ActionCall::from_tool_call(&tool_call);

        assert_eq!(action.tool_name, "bash");
        assert_eq!(action.call_id, "call-456");
        assert_eq!(action.parameters["command"], "ls");
    }

    #[test]
    fn test_action_result_success() {
        let action = ActionCall::new("test_tool", serde_json::json!({}), "call-1");
        let result = ActionResult::success(action.clone(), serde_json::json!("output"));

        assert!(result.success);
        assert!(result.result.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_action_result_failure() {
        let action = ActionCall::new("test_tool", serde_json::json!({}), "call-1");
        let result = ActionResult::failure(action.clone(), "Tool failed");

        assert!(!result.success);
        assert!(result.result.is_none());
        assert_eq!(result.error.as_deref(), Some("Tool failed"));
    }

    #[test]
    fn test_action_result_to_message_success() {
        let action = ActionCall::new("test_tool", serde_json::json!({}), "call-1");
        let result = ActionResult::success(action, serde_json::json!("success output"));

        let message = result.to_message();

        // Message will have an ID (auto-generated)
        assert!(message.id.is_some());
        assert_eq!(message.name, Some("test_tool".to_string()));
    }

    #[test]
    fn test_action_result_to_message_failure() {
        let action = ActionCall::new("test_tool", serde_json::json!({}), "call-1");
        let result = ActionResult::failure(action, "execution failed");

        let message = result.to_message();

        // Message will have an ID (auto-generated)
        assert!(message.id.is_some());

        // Check the message contains error information
        assert!(result.error.is_some());
        assert!(result.error.as_deref().unwrap().contains("execution failed"));
    }

    #[test]
    fn test_parse_actions_no_tool_calls() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        let messages = vec![
            Message::human("Hello"),
            Message::ai("Hi there"),
        ];

        let actions = interpreter.parse_actions(&messages).unwrap();
        assert_eq!(actions.len(), 0);
    }

    #[test]
    fn test_parse_actions_single_tool_call() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        let tool_call = ToolCall {
            id: "call-1".to_string(),
            name: "bash".to_string(),
            args: serde_json::json!({"command": "ls"}),
        };

        let message = Message::ai("Running command").with_tool_calls(vec![tool_call]);
        let messages = vec![message];

        let actions = interpreter.parse_actions(&messages).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].tool_name, "bash");
        assert_eq!(actions[0].call_id, "call-1");
        assert_eq!(actions[0].parameters["command"], "ls");
    }

    #[test]
    fn test_parse_actions_multiple_tool_calls_in_one_message() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        let tool_calls = vec![
            ToolCall {
                id: "call-1".to_string(),
                name: "bash".to_string(),
                args: serde_json::json!({"command": "ls"}),
            },
            ToolCall {
                id: "call-2".to_string(),
                name: "file_read".to_string(),
                args: serde_json::json!({"path": "test.txt"}),
            },
        ];

        let message = Message::ai("Running commands").with_tool_calls(tool_calls);
        let messages = vec![message];

        let actions = interpreter.parse_actions(&messages).unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].tool_name, "bash");
        assert_eq!(actions[1].tool_name, "file_read");
    }

    #[test]
    fn test_parse_actions_multiple_messages_with_tool_calls() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        let messages = vec![
            Message::ai("First call").with_tool_calls(vec![
                ToolCall {
                    id: "call-1".to_string(),
                    name: "tool1".to_string(),
                    args: serde_json::json!({"arg": "value1"}),
                },
            ]),
            Message::human("Continue"),
            Message::ai("Second call").with_tool_calls(vec![
                ToolCall {
                    id: "call-2".to_string(),
                    name: "tool2".to_string(),
                    args: serde_json::json!({"arg": "value2"}),
                },
            ]),
        ];

        let actions = interpreter.parse_actions(&messages).unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].tool_name, "tool1");
        assert_eq!(actions[1].tool_name, "tool2");
    }

    #[test]
    fn test_parse_actions_invalid_args_type() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        let tool_call = ToolCall {
            id: "call-1".to_string(),
            name: "bash".to_string(),
            args: serde_json::json!("invalid string args"), // Should be object
        };

        let message = Message::ai("Running command").with_tool_calls(vec![tool_call]);
        let messages = vec![message];

        let result = interpreter.parse_actions(&messages);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be a JSON object"));
    }

    #[test]
    fn test_parse_actions_null_args() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        let tool_call = ToolCall {
            id: "call-1".to_string(),
            name: "no_args_tool".to_string(),
            args: serde_json::json!(null), // Null args are allowed
        };

        let message = Message::ai("Running command").with_tool_calls(vec![tool_call]);
        let messages = vec![message];

        let actions = interpreter.parse_actions(&messages).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].tool_name, "no_args_tool");
    }

    // ORCA-042: Action validation tests

    #[tokio::test]
    async fn test_validate_action_valid_tool() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        // Create action with a valid tool that exists in the bridge
        let action = ActionCall::new(
            "file_read",  // This tool is registered in DirectToolBridge
            serde_json::json!({"path": "test.txt"}),
            "call-1",
        );

        // Validation should succeed
        let result = interpreter.validate_action(&action).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_action_tool_not_found() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        // Create action with a non-existent tool
        let action = ActionCall::new(
            "nonexistent_tool",
            serde_json::json!({}),
            "call-1",
        );

        // Validation should fail
        let result = interpreter.validate_action(&action).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_validate_action_invalid_args_type() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        // Create action with string args (should be object or null)
        let action = ActionCall::new(
            "file_read",
            serde_json::json!("invalid string args"),
            "call-1",
        );

        // Validation should fail
        let result = interpreter.validate_action(&action).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires object or null"));
    }

    #[tokio::test]
    async fn test_validate_action_null_args() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        // Create action with null args (should be allowed)
        let action = ActionCall::new(
            "git_status",  // This tool exists
            serde_json::json!(null),
            "call-1",
        );

        // Validation should succeed for null args
        let result = interpreter.validate_action(&action).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_action_array_args() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        // Create action with array args (should fail - must be object or null)
        let action = ActionCall::new(
            "file_read",
            serde_json::json!(["arg1", "arg2"]),
            "call-1",
        );

        // Validation should fail
        let result = interpreter.validate_action(&action).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires object or null"));
    }

    #[tokio::test]
    async fn test_interpret_with_validation_errors() {
        use std::path::PathBuf;

        let bridge = Arc::new(DirectToolBridge::new(
            PathBuf::from("/tmp"),
            "test-session".to_string(),
        ).unwrap());
        let interpreter = DefaultInterpreter::new(bridge);

        // Create messages with both valid and invalid tool calls
        let messages = vec![
            Message::ai("Running commands").with_tool_calls(vec![
                ToolCall {
                    id: "call-1".to_string(),
                    name: "file_read".to_string(),
                    args: serde_json::json!({"path": "test.txt"}),
                },
                ToolCall {
                    id: "call-2".to_string(),
                    name: "nonexistent_tool".to_string(),
                    args: serde_json::json!({}),
                },
            ]),
        ];

        // Interpret should handle validation errors gracefully
        let results = interpreter.interpret(&messages).await.unwrap();

        // Should have 2 results
        assert_eq!(results.len(), 2);

        // Second one should be a failure due to validation error
        assert!(!results[1].success);
        assert!(results[1].error.as_ref().unwrap().contains("not found"));
    }
}

