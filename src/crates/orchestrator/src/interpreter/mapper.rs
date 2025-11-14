//! Tool Mapper for converting parsed intents to ToolRequests.
//!
//! Handles path resolution, ambiguity resolution, and workspace context.

use crate::interpreter::parser::{ParsedIntent, ToolCall};
use crate::Result;
use serde_json::Value;
use std::path::PathBuf;
use tooling::runtime::ToolRequest;

/// Tool Mapper for converting intents to ToolRequests
pub struct ToolMapper {
    /// Workspace root for path resolution
    workspace_root: Option<PathBuf>,
}

impl ToolMapper {
    /// Create a new Tool Mapper
    pub fn new() -> Self {
        Self {
            workspace_root: None,
        }
    }

    /// Set the workspace root for path resolution
    pub fn with_workspace_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    /// Map a parsed intent to a ToolRequest
    pub fn map_to_tool_request(
        &self,
        intent: ParsedIntent,
        session_id: &str,
    ) -> Result<ToolRequest> {
        match intent {
            ParsedIntent::StructuredTool(call) => {
                self.map_tool_call(call, session_id)
            }
            ParsedIntent::NaturalLanguage(text) => {
                // For now, return error - natural language parsing would require
                // more sophisticated NLP or asking LLM to reformat
                Err(crate::OrchestratorError::General(format!(
                    "Natural language interpretation not yet implemented. LLM output: {}",
                    text
                )))
            }
            ParsedIntent::Ambiguous(calls) => {
                // For ambiguous cases, we could:
                // 1. Return error asking for clarification
                // 2. Execute all (if safe)
                // 3. Pick first (risky)
                // For now, return error
                Err(crate::OrchestratorError::General(format!(
                    "Ambiguous tool call: {} possible tools. Please specify one.",
                    calls.len()
                )))
            }
        }
    }

    /// Map a single tool call to ToolRequest
    fn map_tool_call(&self, call: ToolCall, session_id: &str) -> Result<ToolRequest> {
        // Resolve paths in arguments if needed
        let args = self.resolve_paths_in_args(&call.tool, call.args)?;

        // Create ToolRequest
        let request = ToolRequest::new(&call.tool, args, session_id);

        Ok(request)
    }

    /// Resolve relative paths in tool arguments
    fn resolve_paths_in_args(&self, tool: &str, mut args: Value) -> Result<Value> {
        // Tools that use "path" argument
        let path_tools = ["file_read", "file_write", "file_patch", "fs_list", "fs_copy", "fs_move", "fs_delete", "grep"];

        if path_tools.contains(&tool) {
            if let Some(path_value) = args.get_mut("path") {
                if let Some(path_str) = path_value.as_str() {
                    let resolved = self.resolve_path(path_str)?;
                    *path_value = Value::String(resolved.to_string_lossy().to_string());
                }
            }
        }

        // Tools that use "paths" (array) argument
        if tool == "git_add" || tool == "file_patch" {
            if let Some(paths_value) = args.get_mut("paths") {
                if let Some(paths_array) = paths_value.as_array_mut() {
                    for path_value in paths_array {
                        if let Some(path_str) = path_value.as_str() {
                            let resolved = self.resolve_path(path_str)?;
                            *path_value = Value::String(resolved.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(args)
    }

    /// Resolve a path relative to workspace root
    fn resolve_path(&self, path: &str) -> Result<PathBuf> {
        let path_buf = PathBuf::from(path);

        // If absolute path, return as-is (but validate it's in workspace)
        if path_buf.is_absolute() {
            if let Some(workspace) = &self.workspace_root {
                if !path_buf.starts_with(workspace) {
                    return Err(crate::OrchestratorError::General(format!(
                        "Path {} is outside workspace root {}",
                        path,
                        workspace.display()
                    )));
                }
            }
            return Ok(path_buf);
        }

        // Relative path - resolve against workspace root
        if let Some(workspace) = &self.workspace_root {
            Ok(workspace.join(path_buf))
        } else {
            // No workspace root set, return as-is
            Ok(path_buf)
        }
    }

    /// Suggest tools based on natural language description
    pub fn suggest_tool(&self, description: &str) -> Vec<String> {
        let desc_lower = description.to_lowercase();
        let mut suggestions = Vec::new();

        // Simple keyword matching for tool suggestions
        if desc_lower.contains("read") || desc_lower.contains("file") && desc_lower.contains("content") {
            suggestions.push("file_read".to_string());
        }
        if desc_lower.contains("write") || desc_lower.contains("create") || desc_lower.contains("edit") {
            suggestions.push("file_write".to_string());
        }
        if desc_lower.contains("list") || desc_lower.contains("directory") || desc_lower.contains("files") {
            suggestions.push("fs_list".to_string());
        }
        if desc_lower.contains("git") && desc_lower.contains("status") {
            suggestions.push("git_status".to_string());
        }
        if desc_lower.contains("git") && desc_lower.contains("commit") {
            suggestions.push("git_commit".to_string());
        }
        if desc_lower.contains("run") || desc_lower.contains("execute") || desc_lower.contains("command") {
            suggestions.push("shell_exec".to_string());
        }
        if desc_lower.contains("search") || desc_lower.contains("grep") || desc_lower.contains("find") {
            suggestions.push("grep".to_string());
        }

        suggestions
    }
}

impl Default for ToolMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_map_tool_call() {
        let mapper = ToolMapper::new();
        let call = ToolCall {
            tool: "file_read".to_string(),
            args: json!({"path": "src/main.rs"}),
        };

        let request = mapper.map_tool_call(call, "session-123").unwrap();
        assert_eq!(request.tool, "file_read");
        assert_eq!(request.session_id, "session-123");
    }

    #[test]
    fn test_resolve_relative_path() {
        let mapper = ToolMapper::new()
            .with_workspace_root("/workspace");

        let resolved = mapper.resolve_path("src/main.rs").unwrap();
        assert!(resolved.to_string_lossy().contains("src/main.rs"));
    }

    #[test]
    fn test_suggest_tool() {
        let mapper = ToolMapper::new();
        
        let suggestions = mapper.suggest_tool("read the main file");
        assert!(suggestions.contains(&"file_read".to_string()));

        let suggestions = mapper.suggest_tool("check git status");
        assert!(suggestions.contains(&"git_status".to_string()));
    }
}

