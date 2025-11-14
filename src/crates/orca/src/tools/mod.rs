//! Direct tool execution bridge
//!
//! Provides direct in-process tool execution without requiring
//! a separate aco server or WebSocket communication.

// TODO: Enable when tooling crate has runtime and tools modules implemented
// mod direct_bridge;
mod permission_enforcer;
mod ast_cache_service;

// pub use direct_bridge::DirectToolBridge;
pub use permission_enforcer::{ToolPermissionEnforcer, ExecutionDecision, ExecutionResult};
pub use ast_cache_service::{AstCacheService, CacheStats};

// Placeholder stub for DirectToolBridge until tooling crate tools are implemented
use std::path::PathBuf;
use serde_json::Value;

/// Stub for DirectToolBridge - will be replaced with full implementation
/// when tooling crate has runtime and tools modules
#[derive(Debug, Clone)]
pub struct DirectToolBridge {
    session_id: String,
    workspace_root: PathBuf,
}

impl DirectToolBridge {
    /// Create a stub DirectToolBridge
    pub fn new(workspace_root: PathBuf, session_id: String) -> anyhow::Result<Self> {
        Ok(Self {
            session_id,
            workspace_root,
        })
    }

    /// Stub execute_tool - returns error
    pub async fn execute_tool(&self, _tool_name: &str, _args: Value) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("DirectToolBridge not yet implemented - requires tooling crate tools modules"))
    }

    /// Stub list_tools
    pub fn list_tools(&self) -> Vec<String> {
        vec![]
    }

    /// Stub workspace_root
    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    /// Stub session_id
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Stub get_tool_schema
    pub fn get_tool_schema(&self, _tool_name: &str) -> anyhow::Result<Value> {
        Err(anyhow::anyhow!("DirectToolBridge not yet implemented - requires tooling crate tools modules"))
    }

    /// Stub get_all_schemas
    pub fn get_all_schemas(&self) -> Vec<Value> {
        vec![]
    }
}
