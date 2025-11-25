//! Direct tool execution bridge
//!
//! Provides direct in-process tool execution without requiring
//! a separate aco server or WebSocket communication.

mod permission_enforcer;
mod ast_cache_service;

pub use permission_enforcer::{ToolPermissionEnforcer, ExecutionDecision, ExecutionResult};
pub use ast_cache_service::{AstCacheService, CacheStats};

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, warn};

/// DirectToolBridge provides in-process tool execution
#[derive(Debug, Clone)]
pub struct DirectToolBridge {
    session_id: String,
    workspace_root: PathBuf,
}

impl DirectToolBridge {
    /// Create a new DirectToolBridge
    pub fn new(workspace_root: PathBuf, session_id: String) -> anyhow::Result<Self> {
        Ok(Self {
            session_id,
            workspace_root,
        })
    }

    /// Execute a tool by name with JSON arguments
    pub async fn execute_tool(&self, tool_name: &str, args: Value) -> anyhow::Result<Value> {
        debug!(tool = tool_name, session = %self.session_id, "Executing tool");

        match tool_name {
            "file_read" => self.tool_file_read(args).await,
            "file_write" => self.tool_file_write(args).await,
            "fs_list" => self.tool_fs_list(args).await,
            "git_status" => self.tool_git_status(args).await,
            "git_diff" => self.tool_git_diff(args).await,
            "shell_exec" => self.tool_shell_exec(args).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        }
    }

    /// List available tools
    pub fn list_tools(&self) -> Vec<String> {
        vec![
            "file_read".to_string(),
            "file_write".to_string(),
            "fs_list".to_string(),
            "git_status".to_string(),
            "git_diff".to_string(),
            "shell_exec".to_string(),
        ]
    }

    /// Get workspace root
    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get schema for a specific tool
    pub fn get_tool_schema(&self, tool_name: &str) -> anyhow::Result<Value> {
        let schema = match tool_name {
            "file_read" => json!({
                "name": "file_read",
                "description": "Read contents of a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        }
                    },
                    "required": ["path"]
                }
            }),
            "file_write" => json!({
                "name": "file_write",
                "description": "Write content to a file",
                "parameters": {
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
                }
            }),
            "fs_list" => json!({
                "name": "fs_list",
                "description": "List contents of a directory",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the directory to list"
                        }
                    },
                    "required": ["path"]
                }
            }),
            "git_status" => json!({
                "name": "git_status",
                "description": "Show git repository status",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to git repository (defaults to workspace root)"
                        }
                    }
                }
            }),
            "git_diff" => json!({
                "name": "git_diff",
                "description": "Show git diff",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to git repository (defaults to workspace root)"
                        },
                        "staged": {
                            "type": "boolean",
                            "description": "Show staged changes only"
                        }
                    }
                }
            }),
            "shell_exec" => json!({
                "name": "shell_exec",
                "description": "Execute a shell command",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Shell command to execute"
                        },
                        "cwd": {
                            "type": "string",
                            "description": "Working directory (defaults to workspace root)"
                        }
                    },
                    "required": ["command"]
                }
            }),
            _ => return Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        };

        Ok(schema)
    }

    /// Get all tool schemas
    pub fn get_all_schemas(&self) -> Vec<Value> {
        self.list_tools()
            .iter()
            .filter_map(|name| self.get_tool_schema(name).ok())
            .collect()
    }

    // Tool implementations

    async fn tool_file_read(&self, args: Value) -> anyhow::Result<Value> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let full_path = self.resolve_path(path)?;
        let content = tokio::fs::read_to_string(&full_path).await?;

        Ok(json!({
            "success": true,
            "path": full_path.display().to_string(),
            "content": content,
            "size": content.len()
        }))
    }

    async fn tool_file_write(&self, args: Value) -> anyhow::Result<Value> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' argument"))?;

        let full_path = self.resolve_path(path)?;

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&full_path, content).await?;

        Ok(json!({
            "success": true,
            "path": full_path.display().to_string(),
            "bytes_written": content.len()
        }))
    }

    async fn tool_fs_list(&self, args: Value) -> anyhow::Result<Value> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let full_path = self.resolve_path(path)?;
        let mut entries = Vec::new();

        let mut read_dir = tokio::fs::read_dir(&full_path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            entries.push(json!({
                "name": entry.file_name().to_string_lossy(),
                "type": if metadata.is_dir() { "directory" } else { "file" },
                "size": metadata.len()
            }));
        }

        Ok(json!({
            "success": true,
            "path": full_path.display().to_string(),
            "entries": entries
        }))
    }

    async fn tool_git_status(&self, args: Value) -> anyhow::Result<Value> {
        let path = args["path"]
            .as_str()
            .map(|p| self.resolve_path(p))
            .transpose()?
            .unwrap_or_else(|| self.workspace_root.clone());

        let output = Command::new("git")
            .args(&["status", "--porcelain"])
            .current_dir(&path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("git status failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(json!({
            "success": true,
            "output": stdout.to_string(),
            "path": path.display().to_string()
        }))
    }

    async fn tool_git_diff(&self, args: Value) -> anyhow::Result<Value> {
        let path = args["path"]
            .as_str()
            .map(|p| self.resolve_path(p))
            .transpose()?
            .unwrap_or_else(|| self.workspace_root.clone());

        let staged = args["staged"].as_bool().unwrap_or(false);

        let mut cmd = Command::new("git");
        cmd.arg("diff");
        if staged {
            cmd.arg("--staged");
        }
        cmd.current_dir(&path);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("git diff failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(json!({
            "success": true,
            "output": stdout.to_string(),
            "path": path.display().to_string(),
            "staged": staged
        }))
    }

    async fn tool_shell_exec(&self, args: Value) -> anyhow::Result<Value> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' argument"))?;

        let cwd = args["cwd"]
            .as_str()
            .map(|p| self.resolve_path(p))
            .transpose()?
            .unwrap_or_else(|| self.workspace_root.clone());

        debug!(command = command, cwd = %cwd.display(), "Executing shell command");

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&cwd)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let success = output.status.success();

        if !success {
            warn!(command = command, "Shell command failed");
        }

        Ok(json!({
            "success": success,
            "stdout": stdout.to_string(),
            "stderr": stderr.to_string(),
            "exit_code": output.status.code()
        }))
    }

    /// Resolve a path relative to workspace root
    fn resolve_path(&self, path: &str) -> anyhow::Result<PathBuf> {
        let path = Path::new(path);

        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        // Canonicalize to prevent path traversal attacks
        match resolved.canonicalize() {
            Ok(canonical) => {
                // Verify the resolved path is within workspace
                if canonical.starts_with(&self.workspace_root) {
                    Ok(canonical)
                } else {
                    Err(anyhow::anyhow!(
                        "Path '{}' is outside workspace root",
                        path.display()
                    ))
                }
            }
            Err(_) => {
                // If canonicalize fails (file doesn't exist yet), validate parent
                if let Some(parent) = resolved.parent() {
                    if parent.starts_with(&self.workspace_root) {
                        Ok(resolved)
                    } else {
                        Err(anyhow::anyhow!(
                            "Path '{}' is outside workspace root",
                            path.display()
                        ))
                    }
                } else {
                    Ok(resolved)
                }
            }
        }
    }
}
