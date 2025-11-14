//! AcoClient for WebSocket communication with aco server.

use crate::client::messages::{SessionInit, WsMessage};
use crate::Result;
use futures_util::SinkExt;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tooling::runtime::{ToolRequest, ToolResponse};
use tracing::{debug, error, info, warn};

/// WebSocket client for communicating with aco server
pub struct AcoClient {
    /// WebSocket URL
    url: String,
    /// Session ID
    session_id: String,
    /// Workspace root path
    workspace_root: Option<String>,
    /// WebSocket connection (mutex for async access)
    connection: Arc<Mutex<Option<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>>>,
    /// Response channel
    response_tx: Option<mpsc::Sender<ToolResponse>>,
    /// Response receiver
    response_rx: Option<mpsc::Receiver<ToolResponse>>,
}

impl AcoClient {
    /// Create a new AcoClient
    pub fn new(url: impl Into<String>, session_id: impl Into<String>) -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            url: url.into(),
            session_id: session_id.into(),
            workspace_root: None,
            connection: Arc::new(Mutex::new(None)),
            response_tx: Some(tx),
            response_rx: Some(rx),
        }
    }

    /// Set workspace root
    pub fn with_workspace_root(mut self, root: impl Into<String>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    /// Connect to aco server
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to aco server at {}", self.url);

        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| crate::OrchestratorError::General(format!(
                "Failed to connect to aco server: {}",
                e
            )))?;

        *self.connection.lock().await = Some(ws_stream);

        // Send session initialization
        let init = SessionInit {
            session_id: self.session_id.clone(),
            workspace_root: self.workspace_root.clone(),
            capabilities: vec!["tool_execution".to_string()],
        };

        self.send_message(WsMessage::SessionInit(init)).await?;

        // Wait for session acknowledgment
        // In a real implementation, we'd wait for SessionAck here
        info!("Connected to aco server");

        Ok(())
    }

    /// Disconnect from aco server
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut ws) = self.connection.lock().await.take() {
            ws.close(None).await.map_err(|e| {
                crate::OrchestratorError::General(format!("Failed to close connection: {}", e))
            })?;
        }
        Ok(())
    }

    /// Send a tool request and wait for response
    pub async fn execute_tool(&mut self, request: ToolRequest) -> Result<ToolResponse> {
        // Ensure connected
        if self.connection.lock().await.is_none() {
            self.connect().await?;
        }

        // Send request
        self.send_message(WsMessage::ToolRequest(request.clone())).await?;

        // Wait for response (with timeout)
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.wait_for_response(&request.request_id),
        )
        .await
        .map_err(|_| crate::OrchestratorError::General(
            "Timeout waiting for tool response".to_string(),
        ))?;

        response
    }

    /// Send a WebSocket message
    async fn send_message(&mut self, message: WsMessage) -> Result<()> {
        let mut conn = self.connection.lock().await;
        let ws = conn.as_mut().ok_or_else(|| {
            crate::OrchestratorError::General("Not connected to aco server".to_string())
        })?;

        let json = serde_json::to_string(&message)
            .map_err(|e| crate::OrchestratorError::Serialization(e))?;

        ws.send(Message::Text(json)).await.map_err(|e| {
            crate::OrchestratorError::General(format!("Failed to send message: {}", e))
        })?;

        Ok(())
    }

    /// Wait for a response with matching request ID
    async fn wait_for_response(&mut self, request_id: &str) -> Result<ToolResponse> {
        // Start message receiver task if not already running
        self.start_message_receiver().await;

        // Poll response channel
        if let Some(rx) = &mut self.response_rx {
            while let Some(response) = rx.recv().await {
                if response.request_id == request_id {
                    return Ok(response);
                }
            }
        }

        Err(crate::OrchestratorError::General(
            "Response channel closed".to_string(),
        ))
    }

    /// Start message receiver task
    async fn start_message_receiver(&mut self) {
        // This is a simplified version - in production, you'd spawn a task
        // that continuously reads from the WebSocket and routes messages
        // For now, we'll handle it synchronously in execute_tool
    }

    /// Handle incoming WebSocket messages
    async fn handle_message(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Text(text) => {
                let ws_msg: WsMessage = serde_json::from_str(&text)
                    .map_err(|e| crate::OrchestratorError::Serialization(e))?;

                match ws_msg {
                    WsMessage::ToolResponse(response) => {
                        if let Some(tx) = &self.response_tx {
                            tx.send(response).await.map_err(|e| {
                                crate::OrchestratorError::General(format!(
                                    "Failed to send response: {}",
                                    e
                                ))
                            })?;
                        }
                    }
                    WsMessage::SessionAck(ack) => {
                        info!("Session acknowledged: {}", ack.session_id);
                    }
                    WsMessage::Error(err) => {
                        error!("Error from aco server: {} - {}", err.code, err.message);
                    }
                    _ => {
                        debug!("Received message: {:?}", ws_msg);
                    }
                }
            }
            Message::Close(_) => {
                warn!("WebSocket connection closed by server");
            }
            Message::Ping(data) => {
                // Respond to ping
                if let Some(ws) = self.connection.lock().await.as_mut() {
                    ws.send(Message::Pong(data)).await.ok();
                }
            }
            _ => {}
        }

        Ok(())
    }
}

impl Drop for AcoClient {
    fn drop(&mut self) {
        // Attempt to disconnect on drop
        let rt = tokio::runtime::Runtime::new().ok();
        if let Some(rt) = rt {
            rt.block_on(self.disconnect()).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_aco_client_creation() {
        let client = AcoClient::new("ws://localhost:8080", "test-session");
        assert_eq!(client.session_id, "test-session");
    }

    // Note: Integration tests would require a running aco server
    // These would be in a separate integration test file
}

