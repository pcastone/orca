//! Tool implementations for ACO worker
//!
//! These tools implement the `langgraph_prebuilt::Tool` trait and provide
//! actual functionality for file operations, shell commands, etc.

use async_trait::async_trait;
use langgraph_prebuilt::{Result as ToolResult, Tool, ToolInput, ToolOutput};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, warn};

/// File read tool - reads file contents
pub struct FileReadTool {
    /// Workspace root for path resolution
    pub workspace: String,
}

impl FileReadTool {
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }

    fn resolve_path(&self, path: &str) -> String {
        let p = Path::new(path);
        if p.is_absolute() {
            path.to_string()
        } else {
            Path::new(&self.workspace).join(path).to_string_lossy().to_string()
        }
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file"
    }

    fn input_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        }))
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let path = input.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| langgraph_prebuilt::PrebuiltError::ToolExecution(
                "Missing 'path' argument".into()
            ))?;

        let resolved = self.resolve_path(path);
        debug!("Reading file: {}", resolved);

        let content = fs::read_to_string(&resolved).await.map_err(|e| {
            langgraph_prebuilt::PrebuiltError::ToolExecution(
                format!("Failed to read file '{}': {}", resolved, e)
            )
        })?;

        Ok(json!({
            "path": path,
            "content": content,
            "size": content.len()
        }))
    }
}

/// File write tool - writes content to a file
pub struct FileWriteTool {
    pub workspace: String,
}

impl FileWriteTool {
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }

    fn resolve_path(&self, path: &str) -> String {
        let p = Path::new(path);
        if p.is_absolute() {
            path.to_string()
        } else {
            Path::new(&self.workspace).join(path).to_string_lossy().to_string()
        }
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file"
    }

    fn input_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        }))
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let path = input.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| langgraph_prebuilt::PrebuiltError::ToolExecution(
                "Missing 'path' argument".into()
            ))?;

        let content = input.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| langgraph_prebuilt::PrebuiltError::ToolExecution(
                "Missing 'content' argument".into()
            ))?;

        let resolved = self.resolve_path(path);
        debug!("Writing file: {}", resolved);

        // Create parent directories if needed
        if let Some(parent) = Path::new(&resolved).parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                langgraph_prebuilt::PrebuiltError::ToolExecution(
                    format!("Failed to create directory: {}", e)
                )
            })?;
        }

        fs::write(&resolved, content).await.map_err(|e| {
            langgraph_prebuilt::PrebuiltError::ToolExecution(
                format!("Failed to write file '{}': {}", resolved, e)
            )
        })?;

        Ok(json!({
            "path": path,
            "bytes_written": content.len(),
            "success": true
        }))
    }
}

/// File list tool - lists directory contents
pub struct FsListTool {
    pub workspace: String,
}

impl FsListTool {
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }

    fn resolve_path(&self, path: &str) -> String {
        let p = Path::new(path);
        if p.is_absolute() {
            path.to_string()
        } else {
            Path::new(&self.workspace).join(path).to_string_lossy().to_string()
        }
    }
}

#[async_trait]
impl Tool for FsListTool {
    fn name(&self) -> &str {
        "fs_list"
    }

    fn description(&self) -> &str {
        "List contents of a directory"
    }

    fn input_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to list (default: workspace root)"
                }
            }
        }))
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let path = input.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let resolved = self.resolve_path(path);
        debug!("Listing directory: {}", resolved);

        let mut entries = Vec::new();
        let mut dir = fs::read_dir(&resolved).await.map_err(|e| {
            langgraph_prebuilt::PrebuiltError::ToolExecution(
                format!("Failed to read directory '{}': {}", resolved, e)
            )
        })?;

        while let Some(entry) = dir.next_entry().await.map_err(|e| {
            langgraph_prebuilt::PrebuiltError::ToolExecution(
                format!("Failed to read entry: {}", e)
            )
        })? {
            let metadata = entry.metadata().await.ok();
            let file_name = entry.file_name().to_string_lossy().to_string();

            entries.push(json!({
                "name": file_name,
                "is_dir": metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                "size": metadata.as_ref().map(|m| m.len()).unwrap_or(0)
            }));
        }

        Ok(json!({
            "path": path,
            "entries": entries,
            "count": entries.len()
        }))
    }
}

/// Shell execution tool - runs shell commands
pub struct ShellExecTool {
    pub workspace: String,
}

impl ShellExecTool {
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }
}

#[async_trait]
impl Tool for ShellExecTool {
    fn name(&self) -> &str {
        "shell_exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command"
    }

    fn input_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory for the command (default: workspace)"
                }
            },
            "required": ["command"]
        }))
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let command = input.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| langgraph_prebuilt::PrebuiltError::ToolExecution(
                "Missing 'command' argument".into()
            ))?;

        let working_dir = input.get("working_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.workspace);

        debug!("Executing command: {} in {}", command, working_dir);

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                langgraph_prebuilt::PrebuiltError::ToolExecution(
                    format!("Failed to execute command: {}", e)
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        if !output.status.success() {
            warn!("Command exited with code {}: {}", exit_code, stderr);
        }

        Ok(json!({
            "command": command,
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
            "success": output.status.success()
        }))
    }
}

/// Git status tool - shows git repository status
pub struct GitStatusTool {
    pub workspace: String,
}

impl GitStatusTool {
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }
}

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }

    fn description(&self) -> &str {
        "Get the git status of the repository"
    }

    async fn execute(&self, _input: ToolInput) -> ToolResult<ToolOutput> {
        debug!("Getting git status in {}", self.workspace);

        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                langgraph_prebuilt::PrebuiltError::ToolExecution(
                    format!("Failed to run git status: {}", e)
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(langgraph_prebuilt::PrebuiltError::ToolExecution(
                format!("Git status failed: {}", stderr)
            ));
        }

        // Parse porcelain output
        let changes: Vec<Value> = stdout.lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let status = &line[0..2];
                let file = line[3..].trim();
                json!({
                    "status": status.trim(),
                    "file": file
                })
            })
            .collect();

        Ok(json!({
            "changes": changes,
            "clean": changes.is_empty(),
            "change_count": changes.len()
        }))
    }
}

/// Git diff tool - shows diff of changes
pub struct GitDiffTool {
    pub workspace: String,
}

impl GitDiffTool {
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }
}

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }

    fn description(&self) -> &str {
        "Get the git diff of changes"
    }

    fn input_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "staged": {
                    "type": "boolean",
                    "description": "Show staged changes only (default: false)"
                },
                "file": {
                    "type": "string",
                    "description": "Specific file to diff (optional)"
                }
            }
        }))
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let staged = input.get("staged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let file = input.get("file")
            .and_then(|v| v.as_str());

        debug!("Getting git diff in {}", self.workspace);

        let mut args = vec!["diff"];
        if staged {
            args.push("--cached");
        }
        if let Some(f) = file {
            args.push("--");
            args.push(f);
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                langgraph_prebuilt::PrebuiltError::ToolExecution(
                    format!("Failed to run git diff: {}", e)
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(langgraph_prebuilt::PrebuiltError::ToolExecution(
                format!("Git diff failed: {}", stderr)
            ));
        }

        Ok(json!({
            "diff": stdout,
            "staged": staged,
            "has_changes": !stdout.is_empty()
        }))
    }
}

/// Grep tool - search file contents
pub struct GrepTool {
    pub workspace: String,
}

impl GrepTool {
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for patterns in files"
    }

    fn input_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search (default: workspace)"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Search recursively (default: true)"
                }
            },
            "required": ["pattern"]
        }))
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let pattern = input.get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| langgraph_prebuilt::PrebuiltError::ToolExecution(
                "Missing 'pattern' argument".into()
            ))?;

        let path = input.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let recursive = input.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let search_path = if Path::new(path).is_absolute() {
            path.to_string()
        } else {
            Path::new(&self.workspace).join(path).to_string_lossy().to_string()
        };

        debug!("Searching for '{}' in {}", pattern, search_path);

        let mut args = vec!["-n"]; // line numbers
        if recursive {
            args.push("-r");
        }
        args.push(pattern);
        args.push(&search_path);

        let output = Command::new("grep")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                langgraph_prebuilt::PrebuiltError::ToolExecution(
                    format!("Failed to run grep: {}", e)
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        // Parse grep output: file:line:content
        let matches: Vec<Value> = stdout.lines()
            .filter(|line| !line.is_empty())
            .take(100) // Limit results
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(3, ':').collect();
                if parts.len() >= 2 {
                    Some(json!({
                        "file": parts[0],
                        "line": parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0),
                        "content": parts.get(2).unwrap_or(&"")
                    }))
                } else {
                    None
                }
            })
            .collect();

        Ok(json!({
            "pattern": pattern,
            "matches": matches,
            "match_count": matches.len()
        }))
    }
}

/// Create all standard tools for a workspace
pub fn create_tools(workspace: &str) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(FileReadTool::new(workspace)),
        Box::new(FileWriteTool::new(workspace)),
        Box::new(FsListTool::new(workspace)),
        Box::new(ShellExecTool::new(workspace)),
        Box::new(GitStatusTool::new(workspace)),
        Box::new(GitDiffTool::new(workspace)),
        Box::new(GrepTool::new(workspace)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_read_tool() {
        let tool = FileReadTool::new("/tmp");
        assert_eq!(tool.name(), "file_read");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_shell_exec_tool() {
        let tool = ShellExecTool::new("/tmp");
        let result = tool.execute(json!({"command": "echo hello"})).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.get("stdout").unwrap().as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_fs_list_tool() {
        let tool = FsListTool::new("/tmp");
        let result = tool.execute(json!({})).await;
        assert!(result.is_ok());
    }
}
