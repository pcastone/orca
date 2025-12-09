//! Server module (stub for future server implementation)

use langgraph_prebuilt::Tool;
use std::sync::Arc;

pub struct AcoServer {
    address: String,
}

impl AcoServer {
    pub fn new() -> Self {
        Self {
            address: "0.0.0.0:50051".to_string(),
        }
    }

    pub fn with_address(mut self, address: &str) -> Self {
        self.address = address.to_string();
        self
    }

    pub async fn register_tool(&self, _tool: Arc<dyn Tool>) {
        // Stub implementation - will be implemented in future tasks
    }

    pub async fn start(&self) -> crate::error::Result<()> {
        // Stub implementation - will be implemented in future tasks
        Ok(())
    }
}
