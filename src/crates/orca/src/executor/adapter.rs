//! Tool Adapter - Bridges DirectToolBridge to langgraph-prebuilt Tool trait
//!
//! Wraps DirectToolBridge tools to make them compatible with langgraph-prebuilt agents.

use crate::tools::DirectToolBridge;
use async_trait::async_trait;
use langgraph_prebuilt::tools::Tool;
use langgraph_prebuilt::Result as PrebuiltResult;
use serde_json::Value;
use std::sync::Arc;

/// Adapter that makes DirectToolBridge compatible with langgraph-prebuilt Tool trait
#[derive(Clone)]
pub struct ToolAdapter {
    bridge: Arc<DirectToolBridge>,
    tool_name: String,
    tool_description: String,
    tool_schema: Value,
}

impl ToolAdapter {
    /// Create a new tool adapter for a specific tool
    ///
    /// # Arguments
    /// * `bridge` - Shared DirectToolBridge instance
    /// * `tool_name` - Name of the tool to adapt
    ///
    /// # Returns
    /// A new ToolAdapter wrapped in a Box for use with langgraph-prebuilt
    pub fn new(bridge: Arc<DirectToolBridge>, tool_name: impl Into<String>) -> anyhow::Result<Self> {
        let tool_name = tool_name.into();

        // Get schema from bridge to validate tool exists
        let tool_schema = bridge.get_tool_schema(&tool_name)?;

        // Extract description from schema if available
        let tool_description = tool_schema
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or(&tool_name)
            .to_string();

        Ok(Self {
            bridge,
            tool_name,
            tool_description,
            tool_schema,
        })
    }

    /// Create adapters for all tools in the bridge
    ///
    /// # Arguments
    /// * `bridge` - Shared DirectToolBridge instance
    ///
    /// # Returns
    /// Vector of boxed Tool implementations for all available tools
    pub fn from_bridge(bridge: Arc<DirectToolBridge>) -> Vec<Box<dyn Tool>> {
        bridge
            .list_tools()
            .into_iter()
            .filter_map(|tool_name| {
                Self::new(bridge.clone(), tool_name)
                    .ok()
                    .map(|adapter| Box::new(adapter) as Box<dyn Tool>)
            })
            .collect()
    }
}

#[async_trait]
impl Tool for ToolAdapter {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn input_schema(&self) -> Option<Value> {
        Some(self.tool_schema.clone())
    }

    async fn execute(&self, input: Value) -> PrebuiltResult<Value> {
        self.bridge
            .execute_tool(&self.tool_name, input)
            .await
            .map_err(|e| langgraph_prebuilt::error::PrebuiltError::ToolExecution(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_tool_adapter_creation() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let adapter = ToolAdapter::new(bridge, "file_read").unwrap();
        assert_eq!(adapter.name(), "file_read");
        assert!(!adapter.description().is_empty());
        assert!(adapter.input_schema().is_some());
    }

    #[tokio::test]
    async fn test_from_bridge() {
        let temp_dir = TempDir::new().unwrap();
        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let tools = ToolAdapter::from_bridge(bridge);
        assert!(!tools.is_empty());

        // Verify we have common tools
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(tool_names.contains(&"file_read"));
        assert!(tool_names.contains(&"git_status"));
    }

    #[tokio::test]
    async fn test_tool_adapter_execution() {
        let temp_dir = TempDir::new().unwrap();

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();

        let bridge = Arc::new(
            DirectToolBridge::new(
                temp_dir.path().to_path_buf(),
                "test-session".to_string(),
            )
            .unwrap()
        );

        let adapter = ToolAdapter::new(bridge, "file_read").unwrap();

        let input = serde_json::json!({
            "path": test_file.to_str().unwrap()
        });

        let result = adapter.execute(input).await.unwrap();

        // Verify we got content back
        assert!(result.is_object());
    }
}
