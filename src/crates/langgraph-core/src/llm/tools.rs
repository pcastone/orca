//! Tool calling abstractions for function-calling models.
//!
//! This module provides types for defining tools/functions that LLMs can call,
//! handling tool call requests, and returning tool results.
//!
//! # Function Calling Flow
//!
//! 1. **Define tools**: Create `ToolDefinition`s with name, description, parameters
//! 2. **Bind to request**: Add tools via `ChatRequest::with_tools()`
//! 3. **Model requests tool**: Response includes `tool_calls` in message
//! 4. **Execute tool**: Application calls the actual function
//! 5. **Return results**: Send tool results in next message
//! 6. **Model responds**: Final answer based on tool outputs
//!
//! # Example
//!
//! ```rust,ignore
//! use langgraph_core::llm::{ToolDefinition, ChatRequest, MessageRole};
//! use serde_json::json;
//!
//! // 1. Define a tool
//! let calculator = ToolDefinition::new(
//!     "calculator",
//!     "Perform arithmetic calculations",
//! ).with_parameters(json!({
//!     "type": "object",
//!     "properties": {
//!         "operation": {"type": "string", "enum": ["add", "subtract"]},
//!         "a": {"type": "number"},
//!         "b": {"type": "number"},
//!     },
//!     "required": ["operation", "a", "b"]
//! }));
//!
//! // 2. Send request with tool
//! let request = ChatRequest::new(vec![
//!     Message::human("What is 25 + 17?")
//! ]).with_tools(vec![calculator]);
//!
//! let response = model.chat(request).await?;
//!
//! // 3. Check if model wants to use tool
//! if let Some(tool_calls) = &response.message.tool_calls {
//!     for call in tool_calls {
//!         // 4. Execute the tool
//!         let result = execute_calculator(&call.arguments)?;
//!
//!         // 5. Return result to model
//!         let tool_message = Message::tool(
//!             &call.id,
//!             serde_json::to_string(&result)?
//!         );
//!
//!         let final_request = ChatRequest::new(vec![
//!             response.message.clone(),
//!             tool_message,
//!         ]);
//!
//!         // 6. Get final answer
//!         let final_response = model.chat(final_request).await?;
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Definition of a tool/function that an LLM can call.
///
/// Describes a function including its name, purpose, and parameter schema.
/// Models use this information to decide when and how to call the function.
///
/// # Parameter Schema
///
/// The `parameters` field should be a JSON Schema object describing the
/// function's parameters. Most models expect an object with:
/// - `type`: "object"
/// - `properties`: Map of parameter names to schemas
/// - `required`: List of required parameter names
///
/// # Example
///
/// ```rust,ignore
/// use langgraph_core::llm::ToolDefinition;
/// use serde_json::json;
///
/// let tool = ToolDefinition::new(
///     "get_weather",
///     "Get current weather for a location",
/// ).with_parameters(json!({
///     "type": "object",
///     "properties": {
///         "location": {
///             "type": "string",
///             "description": "City name (e.g., 'San Francisco')"
///         },
///         "unit": {
///             "type": "string",
///             "enum": ["celsius", "fahrenheit"],
///             "description": "Temperature unit"
///         }
///     },
///     "required": ["location"]
/// }));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// The unique name/identifier for this tool.
    ///
    /// Should be:
    /// - Descriptive (e.g., "get_weather", not "tool1")
    /// - Snake_case or camelCase
    /// - Unique within the tool list
    pub name: String,

    /// Human-readable description of what this tool does.
    ///
    /// The model uses this to decide when to call the tool.
    /// Should clearly explain:
    /// - What the tool does
    /// - When to use it
    /// - What it returns
    pub description: String,

    /// JSON Schema describing the function's parameters.
    ///
    /// Typically an object schema with properties and required fields.
    /// See module-level docs for example.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<JsonValue>,
}

impl ToolDefinition {
    /// Create a new tool definition with name and description.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: None,
        }
    }

    /// Add a JSON Schema for the tool's parameters.
    pub fn with_parameters(mut self, parameters: JsonValue) -> Self {
        self.parameters = Some(parameters);
        self
    }
}

/// A request from the model to call a specific tool.
///
/// When a model decides it needs to use a function, it includes tool calls
/// in its response message. Each call specifies which tool to invoke and
/// with what arguments.
///
/// # Workflow
///
/// 1. Model responds with `tool_calls` in message
/// 2. Application validates and executes each call
/// 3. Application sends tool results back to model
/// 4. Model generates final response
///
/// # Example
///
/// ```rust,ignore
/// if let Some(tool_calls) = &response.message.tool_calls {
///     for call in tool_calls {
///         match call.name.as_str() {
///             "calculator" => {
///                 let args: CalculatorArgs = serde_json::from_value(call.arguments.clone())?;
///                 let result = perform_calculation(args);
///                 // Return result to model...
///             }
///             _ => eprintln!("Unknown tool: {}", call.name),
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this specific tool call.
    ///
    /// Used to associate tool results with their corresponding calls.
    /// Generated by the model (usually a UUID or similar).
    pub id: String,

    /// The name of the tool to call.
    ///
    /// Matches the `name` from a `ToolDefinition`.
    pub name: String,

    /// Arguments to pass to the tool.
    ///
    /// A JSON object with parameters matching the tool's schema.
    /// Should be validated before execution.
    pub arguments: JsonValue,
}

impl ToolCall {
    /// Create a new tool call.
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: JsonValue) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }
}

/// Result from executing a tool call.
///
/// After executing a function requested by the model, wrap the result
/// in this type and send it back via a tool message.
///
/// # Success vs Error
///
/// - **Success**: `result` contains the function's output
/// - **Error**: `error` contains an error message
///
/// The model will use either field to inform its final response.
///
/// # Example
///
/// ```rust,ignore
/// use langgraph_core::llm::ToolResult;
/// use langgraph_core::Message;
/// use serde_json::json;
///
/// // Success case
/// let result = ToolResult::success(
///     "call_123",
///     json!({"temperature": 72, "condition": "sunny"})
/// );
/// let message = Message::tool("call_123", result.to_json_string());
///
/// // Error case
/// let error_result = ToolResult::error(
///     "call_456",
///     "Location not found"
/// );
/// let error_message = Message::tool("call_456", error_result.to_json_string());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The ID of the tool call this result corresponds to.
    ///
    /// Must match a `ToolCall::id` from the model's request.
    pub call_id: String,

    /// The successful result of the tool execution.
    ///
    /// Contains the function's output as JSON. Mutually exclusive with `error`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<JsonValue>,

    /// An error message if the tool execution failed.
    ///
    /// Describes what went wrong. Mutually exclusive with `result`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResult {
    /// Create a successful tool result.
    pub fn success(call_id: impl Into<String>, result: JsonValue) -> Self {
        Self {
            call_id: call_id.into(),
            result: Some(result),
            error: None,
        }
    }

    /// Create an error tool result.
    pub fn error(call_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            result: None,
            error: Some(error.into()),
        }
    }

    /// Convert this result to a JSON string suitable for Message::tool().
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(r#"{{"error":"Failed to serialize tool result"}}"#)
        })
    }

    /// Check if this result represents a success.
    pub fn is_success(&self) -> bool {
        self.result.is_some()
    }

    /// Check if this result represents an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_definition_builder() {
        let tool = ToolDefinition::new("test_tool", "A test tool")
            .with_parameters(json!({"type": "object"}));

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");
        assert!(tool.parameters.is_some());
    }

    #[test]
    fn test_tool_call() {
        let call = ToolCall::new("call_1", "calculator", json!({"a": 5, "b": 3}));

        assert_eq!(call.id, "call_1");
        assert_eq!(call.name, "calculator");
        assert_eq!(call.arguments["a"], 5);
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("call_1", json!({"answer": 42}));

        assert!(result.is_success());
        assert!(!result.is_error());
        assert_eq!(result.result.unwrap()["answer"], 42);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("call_2", "Division by zero");

        assert!(!result.is_success());
        assert!(result.is_error());
        assert_eq!(result.error.unwrap(), "Division by zero");
    }

    #[test]
    fn test_tool_result_to_json_string() {
        let result = ToolResult::success("call_1", json!({"value": 100}));
        let json_str = result.to_json_string();

        assert!(json_str.contains("call_1"));
        assert!(json_str.contains("value"));
    }
}
