//! Tool Permission Enforcement
//!
//! Enforces security controls on tool execution based on database-configured permissions.

use crate::DatabaseManager;
use crate::error::{OrcaError, Result};
use crate::models::{PermissionLevel, ToolPermission};
use crate::repositories::ToolPermissionRepository;
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Tool execution decision
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionDecision {
    /// Tool execution is allowed
    Allow,
    /// Tool execution is denied
    Deny(String),
    /// Tool execution requires user approval
    RequiresApproval(String),
}

/// Result of tool execution with audit information
pub struct ExecutionResult {
    /// Tool execution output
    pub result: Result<Value>,
    /// Execution duration in milliseconds
    pub duration_ms: i64,
    /// Whether user approval was granted
    pub user_approval: bool,
}

/// Tool permission enforcer
///
/// Validates tool executions against database-configured permissions
/// and logs all execution attempts to the audit log.
pub struct ToolPermissionEnforcer {
    /// Database manager for accessing permissions
    db_manager: Arc<DatabaseManager>,

    /// Default behavior when no permission is configured
    default_behavior: PermissionLevel,
}

impl ToolPermissionEnforcer {
    /// Create a new tool permission enforcer
    ///
    /// # Arguments
    /// * `db_manager` - Database manager with project database access
    /// * `default_behavior` - Default permission level when no rule exists
    pub fn new(db_manager: Arc<DatabaseManager>, default_behavior: PermissionLevel) -> Self {
        Self {
            db_manager,
            default_behavior,
        }
    }

    /// Check if tool execution should be allowed
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool to execute
    /// * `args` - Tool execution arguments (JSON)
    ///
    /// # Returns
    /// ExecutionDecision indicating whether execution should proceed
    pub async fn check_permission(
        &self,
        tool_name: &str,
        args: &Value,
    ) -> Result<ExecutionDecision> {
        debug!(tool = tool_name, "Checking tool permission");

        // Get project database
        let project_db = match self.db_manager.project_db() {
            Some(db) => db,
            None => {
                // No project database - use default behavior
                debug!("No project database, using default behavior");
                return Ok(self.apply_default_behavior(tool_name));
            }
        };

        // Get permission repository
        let repo = ToolPermissionRepository::new(project_db.clone());

        // Try to find permission for this tool
        let permission = match repo.find_by_tool_name(tool_name).await {
            Ok(perm) => perm,
            Err(_) => {
                // No permission configured - use default behavior
                debug!(tool = tool_name, "No permission configured, using default");
                return Ok(self.apply_default_behavior(tool_name));
            }
        };

        // Check permission level
        let decision = self.evaluate_permission(&permission, tool_name, args)?;

        info!(
            tool = tool_name,
            decision = ?decision,
            "Permission check complete"
        );

        Ok(decision)
    }

    /// Evaluate a permission against tool execution request
    fn evaluate_permission(
        &self,
        permission: &ToolPermission,
        tool_name: &str,
        args: &Value,
    ) -> Result<ExecutionDecision> {
        // Check if denied
        if permission.is_denied() {
            return Ok(ExecutionDecision::Deny(format!(
                "Tool '{}' is explicitly denied by project permissions",
                tool_name
            )));
        }

        // Check if requires approval
        if permission.requires_approval() {
            return Ok(ExecutionDecision::RequiresApproval(format!(
                "Tool '{}' requires user approval before execution",
                tool_name
            )));
        }

        // Check path restrictions (if configured)
        if let Some(restrictions) = &permission.path_restrictions {
            if !self.check_path_restrictions(args, restrictions)? {
                return Ok(ExecutionDecision::Deny(format!(
                    "Tool '{}' execution violates path restrictions",
                    tool_name
                )));
            }
        }

        // Check argument whitelist (if configured)
        if let Some(whitelist) = &permission.arg_whitelist {
            if !self.check_arg_whitelist(args, whitelist)? {
                return Ok(ExecutionDecision::Deny(format!(
                    "Tool '{}' arguments not in whitelist",
                    tool_name
                )));
            }
        }

        // Check argument blacklist (if configured)
        if let Some(blacklist) = &permission.arg_blacklist {
            if !self.check_arg_blacklist(args, blacklist)? {
                return Ok(ExecutionDecision::Deny(format!(
                    "Tool '{}' arguments match blacklist pattern",
                    tool_name
                )));
            }
        }

        // All checks passed
        Ok(ExecutionDecision::Allow)
    }

    /// Apply default behavior when no permission is configured
    fn apply_default_behavior(&self, tool_name: &str) -> ExecutionDecision {
        match self.default_behavior {
            PermissionLevel::Allowed => ExecutionDecision::Allow,
            PermissionLevel::Denied => {
                ExecutionDecision::Deny(format!("Tool '{}' denied by default policy", tool_name))
            }
            PermissionLevel::RequiresApproval => {
                ExecutionDecision::RequiresApproval(format!(
                    "Tool '{}' requires approval (default policy)",
                    tool_name
                ))
            }
            PermissionLevel::Restricted => {
                // Restricted without explicit rules means deny
                ExecutionDecision::Deny(format!(
                    "Tool '{}' is restricted without explicit permission rules",
                    tool_name
                ))
            }
        }
    }

    /// Check if execution satisfies path restrictions
    fn check_path_restrictions(&self, args: &Value, restrictions: &str) -> Result<bool> {
        // Parse restrictions as JSON array of path patterns
        let patterns: Vec<String> = serde_json::from_str(restrictions)
            .map_err(|e| OrcaError::Other(format!("Invalid path restrictions JSON: {}", e)))?;

        // Extract path from arguments (commonly 'path', 'file', 'directory', etc.)
        let path_value = args.get("path")
            .or_else(|| args.get("file"))
            .or_else(|| args.get("directory"))
            .or_else(|| args.get("source"))
            .or_else(|| args.get("target"));

        if let Some(path) = path_value.and_then(|v| v.as_str()) {
            // Check if path matches any allowed pattern
            for pattern in patterns {
                if path.starts_with(&pattern.replace("/*", "")) {
                    return Ok(true);
                }
            }
            return Ok(false);
        }

        // No path in arguments - allow (path restrictions don't apply)
        Ok(true)
    }

    /// Check if arguments satisfy whitelist
    fn check_arg_whitelist(&self, args: &Value, whitelist: &str) -> Result<bool> {
        // Parse whitelist as JSON array of patterns
        let patterns: Vec<String> = serde_json::from_str(whitelist)
            .map_err(|e| OrcaError::Other(format!("Invalid whitelist JSON: {}", e)))?;

        if patterns.is_empty() {
            return Ok(true); // Empty whitelist allows everything
        }

        // Convert args to string for pattern matching
        let args_str = args.to_string();

        // Check if any pattern matches
        for pattern in patterns {
            if args_str.contains(&pattern) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if arguments violate blacklist
    fn check_arg_blacklist(&self, args: &Value, blacklist: &str) -> Result<bool> {
        // Parse blacklist as JSON array of patterns
        let patterns: Vec<String> = serde_json::from_str(blacklist)
            .map_err(|e| OrcaError::Other(format!("Invalid blacklist JSON: {}", e)))?;

        // Convert args to string for pattern matching
        let args_str = args.to_string();

        // Check if any pattern matches (violation)
        for pattern in patterns {
            if args_str.contains(&pattern) {
                return Ok(false); // Match found - blacklist violated
            }
        }

        Ok(true) // No matches - blacklist satisfied
    }

    /// Log tool execution to audit log
    ///
    /// # Arguments
    /// * `tool_name` - Name of the executed tool
    /// * `args` - Tool execution arguments
    /// * `result` - Execution result
    /// * `duration_ms` - Execution duration in milliseconds
    /// * `user_approval` - Whether user approved the execution
    /// * `task_id` - Optional task ID that triggered the execution
    pub async fn log_execution(
        &self,
        tool_name: &str,
        args: &Value,
        result: &Result<Value>,
        duration_ms: i64,
        user_approval: bool,
        task_id: Option<&str>,
    ) -> Result<()> {
        // Get project database
        let project_db = match self.db_manager.project_db() {
            Some(db) => db,
            None => {
                warn!("No project database available for audit logging");
                return Ok(());
            }
        };

        // Determine status and result
        let (status, result_json) = match result {
            Ok(val) => ("success", serde_json::to_string(val).unwrap_or_default()),
            Err(e) => ("failure", format!("{{\"error\": \"{}\"}}", e)),
        };

        // Insert into tool_executions table
        let query = sqlx::query(
            "INSERT INTO tool_executions (id, tool_name, arguments, result, status,
                                          duration_ms, user_approval, task_id, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tool_name)
        .bind(serde_json::to_string(args).unwrap_or_default())
        .bind(result_json)
        .bind(status)
        .bind(duration_ms)
        .bind(if user_approval { 1 } else { 0 })
        .bind(task_id)
        .bind(Utc::now().timestamp());

        query
            .execute(project_db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to log tool execution: {}", e)))?;

        debug!(
            tool = tool_name,
            status = status,
            duration_ms = duration_ms,
            "Tool execution logged"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_decision_allow() {
        let decision = ExecutionDecision::Allow;
        assert_eq!(decision, ExecutionDecision::Allow);
    }

    #[test]
    fn test_execution_decision_deny() {
        let decision = ExecutionDecision::Deny("Test".to_string());
        match decision {
            ExecutionDecision::Deny(msg) => assert_eq!(msg, "Test"),
            _ => panic!("Expected Deny"),
        }
    }

    #[tokio::test]
    async fn test_check_path_restrictions_match() {
        let enforcer = ToolPermissionEnforcer::new(
            Arc::new(DatabaseManager::new(".").await.unwrap()),
            PermissionLevel::RequiresApproval,
        );

        let args = serde_json::json!({"path": "/project/src/main.rs"});
        let restrictions = r#"["/project/*"]"#;

        let result = enforcer.check_path_restrictions(&args, restrictions).unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_check_arg_blacklist_violation() {
        let enforcer = ToolPermissionEnforcer::new(
            Arc::new(DatabaseManager::new(".").await.unwrap()),
            PermissionLevel::RequiresApproval,
        );

        let args = serde_json::json!({"command": "rm -rf /"});
        let blacklist = r#"["rm -rf", "dd if="]"#;

        let result = enforcer.check_arg_blacklist(&args, blacklist).unwrap();
        assert!(!result); // Should return false (blacklist violated)
    }
}
