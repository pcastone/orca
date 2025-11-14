//! Direct Tool Execution Bridge
//!
//! Executes tools directly in-process without WebSocket communication.
//! Replaces the AcoClient WebSocket bridge used in the orchestrator crate.

use crate::DatabaseManager;
use crate::models::PermissionLevel;
use crate::tools::permission_enforcer::{ExecutionDecision, ToolPermissionEnforcer};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tooling::runtime::{PolicyRegistry, ToolRuntimeContext};
use tooling::tools::ast::{AstEditTool, AstExternalTool, AstGenerateTool, AstQueryTool, AstValidateTool};
use tooling::tools::filesystem::{
    FilePatchTool, FileReadTool, FileWriteTool, FsCopyTool, FsDeleteTool, FsListTool, FsMoveTool,
    GrepTool,
};
use tooling::tools::git::{GitAddTool, GitCommitTool, GitDiffTool, GitStatusTool};
use tooling::tools::network::CurlTool;
use tooling::tools::shell::ShellExecTool;
use tooling::tools::ToolExecutor;
use tracing::{debug, info, warn};

/// Direct tool execution bridge
///
/// Manages a registry of tools and executes them directly without
/// requiring network communication to an aco server.
pub struct DirectToolBridge {
    /// Tool executors indexed by name
    tools: Arc<HashMap<String, Arc<dyn ToolExecutor>>>,

    /// Runtime context for tool execution
    context: ToolRuntimeContext,

    /// Session identifier
    session_id: String,

    /// Permission enforcer (optional - only when project database exists)
    permission_enforcer: Option<Arc<ToolPermissionEnforcer>>,
}

impl std::fmt::Debug for DirectToolBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectToolBridge")
            .field("tools", &format!("{} tools", self.tools.len()))
            .field("session_id", &self.session_id)
            .field("workspace_root", &self.context.workspace_root)
            .finish()
    }
}

impl DirectToolBridge {
    /// Create a new DirectToolBridge
    ///
    /// # Arguments
    /// * `workspace_root` - Root directory for tool execution
    /// * `session_id` - Unique session identifier
    pub fn new(workspace_root: PathBuf, session_id: String) -> Result<Self> {
        let mut tools: HashMap<String, Arc<dyn ToolExecutor>> = HashMap::new();

        // Helper macro to register a tool
        macro_rules! register_tool {
            ($tool:expr) => {
                let tool: Arc<dyn ToolExecutor> = Arc::new($tool);
                tools.insert(tool.name().to_string(), tool);
            };
        }

        // Register filesystem tools
        register_tool!(FileReadTool);
        register_tool!(FileWriteTool);
        register_tool!(FsListTool);
        register_tool!(FsCopyTool);
        register_tool!(FsMoveTool);
        register_tool!(FsDeleteTool);
        register_tool!(FilePatchTool);
        register_tool!(GrepTool);

        // Register git tools
        register_tool!(GitStatusTool);
        register_tool!(GitDiffTool);
        register_tool!(GitAddTool);
        register_tool!(GitCommitTool);

        // Register shell tools
        register_tool!(ShellExecTool);

        // Register network tools
        register_tool!(CurlTool);

        // Register AST tools
        register_tool!(AstGenerateTool);
        register_tool!(AstQueryTool);
        register_tool!(AstEditTool);
        register_tool!(AstValidateTool);
        register_tool!(AstExternalTool);

        info!(
            tools = tools.len(),
            workspace = %workspace_root.display(),
            session = %session_id,
            "Initialized DirectToolBridge"
        );

        // Create runtime context with permissive policy for now
        let context = ToolRuntimeContext::new(
            session_id.clone(),
            workspace_root,
            Arc::new(PolicyRegistry::permissive()),
        );

        Ok(Self {
            tools: Arc::new(tools),
            context,
            session_id,
            permission_enforcer: None,
        })
    }

    /// Enable permission enforcement with database manager
    ///
    /// # Arguments
    /// * `db_manager` - Database manager with project database access
    /// * `default_behavior` - Default permission level when no rule exists
    pub fn with_permission_enforcement(
        mut self,
        db_manager: Arc<DatabaseManager>,
        default_behavior: PermissionLevel,
    ) -> Self {
        self.permission_enforcer = Some(Arc::new(ToolPermissionEnforcer::new(
            db_manager,
            default_behavior,
        )));
        info!("Permission enforcement enabled");
        self
    }

    /// Create with custom policy registry
    pub fn with_policy(mut self, policy: PolicyRegistry) -> Self {
        self.context = ToolRuntimeContext::new(
            self.session_id.clone(),
            self.context.workspace_root.clone(),
            Arc::new(policy),
        );
        self
    }

    /// Execute a tool by name with arguments
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool to execute
    /// * `args` - JSON arguments for the tool
    ///
    /// # Returns
    /// Result of tool execution
    pub async fn execute_tool(&self, tool_name: &str, args: Value) -> Result<Value> {
        let start_time = Instant::now();
        debug!(
            tool = tool_name,
            session = %self.session_id,
            "Executing tool"
        );

        // Check permissions if enforcer is enabled
        if let Some(enforcer) = &self.permission_enforcer {
            match enforcer.check_permission(tool_name, &args).await {
                Ok(ExecutionDecision::Allow) => {
                    debug!(tool = tool_name, "Permission granted");
                }
                Ok(ExecutionDecision::Deny(reason)) => {
                    warn!(tool = tool_name, reason = %reason, "Permission denied");

                    // Log denied execution
                    let duration_ms = start_time.elapsed().as_millis() as i64;
                    let error_result: crate::error::Result<Value> = Err(crate::error::OrcaError::Other(reason.clone()));
                    let _ = enforcer.log_execution(
                        tool_name,
                        &args,
                        &error_result,
                        duration_ms,
                        false,
                        None,
                    ).await;

                    return Err(anyhow::anyhow!("Permission denied: {}", reason));
                }
                Ok(ExecutionDecision::RequiresApproval(reason)) => {
                    warn!(tool = tool_name, reason = %reason, "Requires approval");

                    // Log approval request
                    let duration_ms = start_time.elapsed().as_millis() as i64;
                    let error_result: crate::error::Result<Value> = Err(crate::error::OrcaError::Other(reason.clone()));
                    let _ = enforcer.log_execution(
                        tool_name,
                        &args,
                        &error_result,
                        duration_ms,
                        false,
                        None,
                    ).await;

                    return Err(anyhow::anyhow!("Approval required: {}", reason));
                }
                Err(e) => {
                    warn!(tool = tool_name, error = %e, "Permission check failed");
                    return Err(anyhow::anyhow!("Permission check failed: {}", e));
                }
            }
        }

        // Get tool from registry
        let tool = self.tools
            .get(tool_name)
            .with_context(|| format!("Tool '{}' not found", tool_name))?;

        // Execute tool
        let result = tool
            .execute(args.clone(), &self.context)
            .await
            .with_context(|| format!("Failed to execute tool '{}'", tool_name));

        let duration_ms = start_time.elapsed().as_millis() as i64;

        // Log execution if enforcer is enabled
        if let Some(enforcer) = &self.permission_enforcer {
            let log_result = match &result {
                Ok(val) => Ok(val.clone()),
                Err(e) => Err(crate::error::OrcaError::Other(e.to_string())),
            };
            let _ = enforcer.log_execution(
                tool_name,
                &args,
                &log_result,
                duration_ms,
                false,
                None,
            ).await;
        }

        if result.is_ok() {
            debug!(
                tool = tool_name,
                session = %self.session_id,
                duration_ms = duration_ms,
                "Tool executed successfully"
            );
        }

        result
    }

    /// List all available tools
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get tool schema by name
    pub fn get_tool_schema(&self, tool_name: &str) -> Result<Value> {
        let tool = self.tools
            .get(tool_name)
            .with_context(|| format!("Tool '{}' not found", tool_name))?;

        Ok(tool.input_schema())
    }

    /// Get all tool schemas for LLM function calling
    pub fn get_all_schemas(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|tool| tool.input_schema())
            .collect()
    }

    /// Get the workspace root
    pub fn workspace_root(&self) -> &PathBuf {
        &self.context.workspace_root
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_bridge() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = DirectToolBridge::new(
            temp_dir.path().to_path_buf(),
            "test-session".to_string(),
        ).unwrap();

        assert_eq!(bridge.session_id(), "test-session");
        assert_eq!(bridge.workspace_root(), temp_dir.path());
        assert!(!bridge.list_tools().is_empty());
    }

    #[tokio::test]
    async fn test_list_tools() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = DirectToolBridge::new(
            temp_dir.path().to_path_buf(),
            "test-session".to_string(),
        ).unwrap();

        let tools = bridge.list_tools();
        assert!(tools.contains(&"file_read".to_string()));
        assert!(tools.contains(&"git_status".to_string()));
        assert!(tools.contains(&"shell_exec".to_string()));
    }

    #[tokio::test]
    async fn test_execute_tool_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = DirectToolBridge::new(
            temp_dir.path().to_path_buf(),
            "test-session".to_string(),
        ).unwrap();

        let result = bridge.execute_tool("nonexistent_tool", json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_get_tool_schema() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = DirectToolBridge::new(
            temp_dir.path().to_path_buf(),
            "test-session".to_string(),
        ).unwrap();

        let schema = bridge.get_tool_schema("file_read").unwrap();
        assert!(schema.is_object());
    }

    #[tokio::test]
    async fn test_get_all_schemas() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = DirectToolBridge::new(
            temp_dir.path().to_path_buf(),
            "test-session".to_string(),
        ).unwrap();

        let schemas = bridge.get_all_schemas();
        assert!(!schemas.is_empty());
    }
}
