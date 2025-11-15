//! Execution context for Tool Runtime SDK
//!
//! Provides context information for tool execution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Tool runtime execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRuntimeContext {
    /// Session identifier
    pub session_id: String,

    /// Workspace root path
    pub workspace_root: PathBuf,

    /// Environment variables for tool execution
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Current working directory (relative to workspace_root)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,

    /// User metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ToolRuntimeContext {
    /// Create a new tool runtime context
    pub fn new(session_id: impl Into<String>, workspace_root: PathBuf) -> Self {
        Self {
            session_id: session_id.into(),
            workspace_root,
            env: HashMap::new(),
            cwd: None,
            metadata: HashMap::new(),
        }
    }

    /// Set environment variables
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Set current working directory
    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get the absolute path for a relative path within the workspace
    pub fn resolve_path(&self, path: &std::path::Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            let base = self.cwd.as_ref().unwrap_or(&self.workspace_root);
            base.join(path)
        }
    }

    /// Check if a path is within the workspace
    pub fn is_within_workspace(&self, path: &std::path::Path) -> bool {
        path.starts_with(&self.workspace_root)
    }

    /// Get environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }

    /// Set environment variable
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env.insert(key.into(), value.into());
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    /// Set metadata value
    pub fn set_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = ToolRuntimeContext::new("sess-123", PathBuf::from("/workspace"));

        assert_eq!(ctx.session_id, "sess-123");
        assert_eq!(ctx.workspace_root, PathBuf::from("/workspace"));
        assert!(ctx.env.is_empty());
        assert!(ctx.cwd.is_none());
    }

    #[test]
    fn test_context_builder() {
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "value".to_string());

        let ctx = ToolRuntimeContext::new("sess-123", PathBuf::from("/workspace"))
            .with_env(env.clone())
            .with_cwd(PathBuf::from("/workspace/subdir"))
            .with_metadata("user_id", serde_json::json!("user-456"));

        assert_eq!(ctx.env.get("KEY"), Some(&"value".to_string()));
        assert_eq!(ctx.cwd, Some(PathBuf::from("/workspace/subdir")));
        assert_eq!(ctx.get_metadata("user_id"), Some(&serde_json::json!("user-456")));
    }

    #[test]
    fn test_resolve_path_relative() {
        let ctx = ToolRuntimeContext::new("sess-123", PathBuf::from("/workspace"))
            .with_cwd(PathBuf::from("/workspace/src"));

        let resolved = ctx.resolve_path(&PathBuf::from("main.rs"));
        assert_eq!(resolved, PathBuf::from("/workspace/src/main.rs"));
    }

    #[test]
    fn test_resolve_path_absolute() {
        let ctx = ToolRuntimeContext::new("sess-123", PathBuf::from("/workspace"));

        let resolved = ctx.resolve_path(&PathBuf::from("/absolute/path"));
        assert_eq!(resolved, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_is_within_workspace() {
        let ctx = ToolRuntimeContext::new("sess-123", PathBuf::from("/workspace"));

        assert!(ctx.is_within_workspace(&PathBuf::from("/workspace/src/main.rs")));
        assert!(ctx.is_within_workspace(&PathBuf::from("/workspace")));
        assert!(!ctx.is_within_workspace(&PathBuf::from("/other/path")));
    }

    #[test]
    fn test_env_operations() {
        let mut ctx = ToolRuntimeContext::new("sess-123", PathBuf::from("/workspace"));

        ctx.set_env("KEY1", "value1");
        ctx.set_env("KEY2", "value2");

        assert_eq!(ctx.get_env("KEY1"), Some(&"value1".to_string()));
        assert_eq!(ctx.get_env("KEY2"), Some(&"value2".to_string()));
        assert_eq!(ctx.get_env("KEY3"), None);
    }

    #[test]
    fn test_metadata_operations() {
        let mut ctx = ToolRuntimeContext::new("sess-123", PathBuf::from("/workspace"));

        ctx.set_metadata("key1", serde_json::json!({"nested": "value"}));
        ctx.set_metadata("key2", serde_json::json!(42));

        assert_eq!(
            ctx.get_metadata("key1"),
            Some(&serde_json::json!({"nested": "value"}))
        );
        assert_eq!(ctx.get_metadata("key2"), Some(&serde_json::json!(42)));
        assert_eq!(ctx.get_metadata("key3"), None);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let ctx = ToolRuntimeContext::new("sess-123", PathBuf::from("/workspace"))
            .with_metadata("test", serde_json::json!("value"));

        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: ToolRuntimeContext = serde_json::from_str(&json).unwrap();

        assert_eq!(ctx.session_id, deserialized.session_id);
        assert_eq!(ctx.workspace_root, deserialized.workspace_root);
    }
}
