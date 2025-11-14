//! Tool Validator for pre-execution validation of tool calls.
//!
//! Validates:
//! - Tool exists and is available
//! - Arguments match tool schema
//! - Paths are within workspace bounds
//! - Required arguments are present

use crate::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tooling::runtime::ToolRequest;

/// Tool Validator for pre-execution checks
pub struct ToolValidator {
    /// Workspace root for path validation
    workspace_root: Option<PathBuf>,
    /// Available tools registry (tool name -> schema)
    tool_registry: HashMap<String, ToolSchema>,
}

/// Tool schema for validation
#[derive(Debug, Clone)]
struct ToolSchema {
    name: String,
    required_args: Vec<String>,
    arg_types: HashMap<String, ArgType>,
}

/// Argument type for validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ArgType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Path, // Special: must be valid path
}

impl ToolValidator {
    /// Create a new Tool Validator
    pub fn new() -> Self {
        let mut validator = Self {
            workspace_root: None,
            tool_registry: HashMap::new(),
        };

        // Register common tools with their schemas
        validator.register_common_tools();
        validator
    }

    /// Set the workspace root for path validation
    pub fn with_workspace_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    /// Register a tool schema
    pub fn register_tool(
        &mut self,
        name: String,
        required_args: Vec<String>,
        arg_types: HashMap<String, ArgType>,
    ) {
        self.tool_registry.insert(
            name.clone(),
            ToolSchema {
                name,
                required_args,
                arg_types,
            },
        );
    }

    /// Validate a ToolRequest before execution
    pub fn validate(&self, request: &ToolRequest) -> Result<()> {
        // 1. Check tool exists
        let schema = self
            .tool_registry
            .get(&request.tool)
            .ok_or_else(|| {
                let available: Vec<String> = self.tool_registry.keys().cloned().collect();
                crate::OrchestratorError::General(format!(
                    "Unknown tool: {}. Available tools: {}",
                    request.tool,
                    available.join(", ")
                ))
            })?;

        // 2. Validate required arguments
        for required_arg in &schema.required_args {
            if !request.args.get(required_arg).is_some() {
                return Err(crate::OrchestratorError::General(format!(
                    "Missing required argument '{}' for tool '{}'",
                    required_arg, request.tool
                )));
            }
        }

        // 3. Validate argument types
        if let Some(obj) = request.args.as_object() {
            for (key, value) in obj {
                if let Some(expected_type) = schema.arg_types.get(key) {
                    if !self.validate_arg_type(value, *expected_type) {
                        return Err(crate::OrchestratorError::General(format!(
                            "Invalid type for argument '{}' in tool '{}': expected {:?}, got {:?}",
                            key,
                            request.tool,
                            expected_type,
                            self.get_value_type(value)
                        )));
                    }
                }
            }
        }

        // 4. Validate paths (if tool uses paths)
        self.validate_paths(&request.tool, &request.args)?;

        Ok(())
    }

    /// Validate argument type
    fn validate_arg_type(&self, value: &Value, expected: ArgType) -> bool {
        match (value, expected) {
            (Value::String(_), ArgType::String) => true,
            (Value::String(_), ArgType::Path) => true, // Paths are strings
            (Value::Number(_), ArgType::Number) => true,
            (Value::Bool(_), ArgType::Boolean) => true,
            (Value::Array(_), ArgType::Array) => true,
            (Value::Object(_), ArgType::Object) => true,
            _ => false,
        }
    }

    /// Get the type of a JSON value
    fn get_value_type(&self, value: &Value) -> &'static str {
        match value {
            Value::String(_) => "String",
            Value::Number(_) => "Number",
            Value::Bool(_) => "Boolean",
            Value::Array(_) => "Array",
            Value::Object(_) => "Object",
            Value::Null => "Null",
        }
    }

    /// Validate paths in tool arguments
    fn validate_paths(&self, tool: &str, args: &Value) -> Result<()> {
        let path_tools = [
            "file_read", "file_write", "file_patch", "fs_list", "fs_copy",
            "fs_move", "fs_delete", "grep",
        ];

        if !path_tools.contains(&tool) {
            return Ok(()); // Tool doesn't use paths
        }

        // Validate single path
        if let Some(path_value) = args.get("path") {
            if let Some(path_str) = path_value.as_str() {
                self.validate_path(path_str)?;
            }
        }

        // Validate path arrays
        if let Some(paths_value) = args.get("paths") {
            if let Some(paths_array) = paths_value.as_array() {
                for path_value in paths_array {
                    if let Some(path_str) = path_value.as_str() {
                        self.validate_path(path_str)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate a single path
    fn validate_path(&self, path: &str) -> Result<()> {
        let path_buf = PathBuf::from(path);

        // Check for path traversal attacks
        if path.contains("..") {
            let normalized = path_buf.canonicalize()
                .map_err(|_| crate::OrchestratorError::General(
                    format!("Invalid path: {}", path)
                ))?;

            if let Some(workspace) = &self.workspace_root {
                if !normalized.starts_with(workspace) {
                    return Err(crate::OrchestratorError::General(format!(
                        "Path '{}' is outside workspace root '{}'",
                        path,
                        workspace.display()
                    )));
                }
            }
        }

        // Check for absolute paths outside workspace
        if path_buf.is_absolute() {
            if let Some(workspace) = &self.workspace_root {
                if !path_buf.starts_with(workspace) {
                    return Err(crate::OrchestratorError::General(format!(
                        "Absolute path '{}' is outside workspace root '{}'",
                        path,
                        workspace.display()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Register common tools with their schemas
    fn register_common_tools(&mut self) {
        // file_read
        let mut args = HashMap::new();
        args.insert("path".to_string(), ArgType::Path);
        self.register_tool("file_read".to_string(), vec!["path".to_string()], args);

        // file_write
        let mut args = HashMap::new();
        args.insert("path".to_string(), ArgType::Path);
        args.insert("content".to_string(), ArgType::String);
        self.register_tool(
            "file_write".to_string(),
            vec!["path".to_string(), "content".to_string()],
            args,
        );

        // fs_list
        let mut args = HashMap::new();
        args.insert("path".to_string(), ArgType::Path);
        self.register_tool("fs_list".to_string(), vec!["path".to_string()], args);

        // git_status
        self.register_tool("git_status".to_string(), vec![], HashMap::new());

        // shell_exec
        let mut args = HashMap::new();
        args.insert("command".to_string(), ArgType::String);
        self.register_tool("shell_exec".to_string(), vec!["command".to_string()], args);

        // grep
        let mut args = HashMap::new();
        args.insert("pattern".to_string(), ArgType::String);
        args.insert("path".to_string(), ArgType::Path);
        self.register_tool(
            "grep".to_string(),
            vec!["pattern".to_string(), "path".to_string()],
            args,
        );
    }
}

impl Default for ToolValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tooling::runtime::ToolRequest;

    #[test]
    fn test_validate_valid_request() {
        let validator = ToolValidator::new();
        let request = ToolRequest::new(
            "file_read",
            json!({"path": "src/main.rs"}),
            "session-123",
        );

        assert!(validator.validate(&request).is_ok());
    }

    #[test]
    fn test_validate_missing_required_arg() {
        let validator = ToolValidator::new();
        let request = ToolRequest::new("file_read", json!({}), "session-123");

        assert!(validator.validate(&request).is_err());
    }

    #[test]
    fn test_validate_unknown_tool() {
        let validator = ToolValidator::new();
        let request = ToolRequest::new(
            "unknown_tool",
            json!({"arg": "value"}),
            "session-123",
        );

        assert!(validator.validate(&request).is_err());
    }

    #[test]
    fn test_validate_path_outside_workspace() {
        let validator = ToolValidator::new()
            .with_workspace_root("/workspace");
        let request = ToolRequest::new(
            "file_read",
            json!({"path": "/etc/passwd"}),
            "session-123",
        );

        assert!(validator.validate(&request).is_err());
    }
}

