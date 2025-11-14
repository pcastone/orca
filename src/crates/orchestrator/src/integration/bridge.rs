//! PatternToolBridge for connecting orchestrator patterns with aco client

use crate::client::AcoClient;
use crate::interpreter::ActionInterpreter;
use crate::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tooling::runtime::ToolRequest;

/// Bridge between orchestrator patterns and aco client
pub struct PatternToolBridge {
    /// Action interpreter for parsing LLM output
    interpreter: Arc<ActionInterpreter>,
    /// aco client for tool execution
    aco_client: Arc<Mutex<AcoClient>>,
    /// Session ID
    session_id: String,
}

impl PatternToolBridge {
    /// Create a new PatternToolBridge
    pub fn new(aco_url: impl Into<String>, session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        let aco_client = Arc::new(Mutex::new(AcoClient::new(aco_url, &session_id)));
        Self {
            interpreter: Arc::new(ActionInterpreter::new()),
            aco_client,
            session_id,
        }
    }

    /// Execute a tool from LLM output
    pub async fn execute_from_llm_output(
        &self,
        llm_output: &str,
    ) -> Result<tooling::runtime::ToolResponse> {
        // 1. Parse and interpret LLM output
        let tool_request = self
            .interpreter
            .interpret(llm_output, &self.session_id)
            .await?;

        // 2. Execute via aco client
        let mut client = self.aco_client.lock().await;
        let response = client.execute_tool(tool_request).await?;

        Ok(response)
    }

    /// Execute a tool request directly
    pub async fn execute_tool(
        &self,
        request: ToolRequest,
    ) -> Result<tooling::runtime::ToolResponse> {
        let mut client = self.aco_client.lock().await;
        client.execute_tool(request).await
    }

    /// Format a tool response for LLM consumption
    pub fn format_response(
        &self,
        response: &tooling::runtime::ToolResponse,
    ) -> String {
        self.interpreter.format_response(response)
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

