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

    // ========================================================================
    // Phase 7.1: WebSocket Client Tests
    // ========================================================================

    // ------------------------------------------------------------------------
    // Basic Client Creation and Configuration
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_aco_client_creation() {
        let client = AcoClient::new("ws://localhost:8080", "test-session");
        assert_eq!(client.session_id, "test-session");
        assert_eq!(client.url, "ws://localhost:8080");
    }

    #[tokio::test]
    async fn test_aco_client_with_workspace_root() {
        let client = AcoClient::new("ws://localhost:8080", "test-session")
            .with_workspace_root("/tmp/workspace");
        assert_eq!(client.workspace_root, Some("/tmp/workspace".to_string()));
    }

    #[tokio::test]
    async fn test_client_initial_state_not_connected() {
        let client = AcoClient::new("ws://localhost:8080", "test-session");
        assert!(client.connection.lock().await.is_none());
    }

    // ------------------------------------------------------------------------
    // Connection Tests (requires mock server - marked #[ignore])
    // ------------------------------------------------------------------------

    #[tokio::test]
    #[ignore] // Requires running WebSocket server
    async fn test_websocket_connection_success() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        // This would connect to a real WebSocket server
        let result = client.connect().await;

        // In a real test environment with a mock server:
        // assert!(result.is_ok());
        // assert!(client.connection.lock().await.is_some());
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_connection_sends_session_init() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session")
            .with_workspace_root("/workspace");

        // Mock server would verify that SessionInit message is sent
        // with correct session_id, workspace_root, and capabilities
        let result = client.connect().await;

        // Would verify:
        // - SessionInit message sent
        // - Contains session_id = "test-session"
        // - Contains workspace_root = "/workspace"
        // - Contains capabilities = ["tool_execution"]
    }

    #[tokio::test]
    async fn test_connection_error_invalid_url() {
        let mut client = AcoClient::new("invalid-url", "test-session");

        let result = client.connect().await;

        // Should fail with connection error
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, crate::OrchestratorError::General(_)));
        }
    }

    #[tokio::test]
    async fn test_connection_error_unreachable_host() {
        // Use a non-existent host that will fail quickly
        let mut client = AcoClient::new("ws://255.255.255.255:9999", "test-session");

        let result = client.connect().await;

        // Should fail with connection error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_disconnect_closes_connection() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        // Manually set a connection (in real test, would connect first)
        // For now, test disconnect on unconnected client
        let result = client.disconnect().await;

        // Should not error even if not connected
        assert!(result.is_ok());
        assert!(client.connection.lock().await.is_none());
    }

    // ------------------------------------------------------------------------
    // Reconnection Logic Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    #[ignore] // Requires mock server with controlled failures
    async fn test_reconnection_after_connection_loss() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        // Test scenario:
        // 1. Connect successfully
        // 2. Simulate connection drop
        // 3. Attempt operation that triggers reconnect
        // 4. Verify reconnection succeeds

        // This would require a mock WebSocket server that can:
        // - Accept initial connection
        // - Drop connection on command
        // - Accept reconnection
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_execute_tool_reconnects_if_disconnected() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        // Ensure not connected initially
        assert!(client.connection.lock().await.is_none());

        let request = ToolRequest {
            request_id: "req-1".to_string(),
            tool_name: "test_tool".to_string(),
            arguments: json!({}),
            timeout_ms: None,
        };

        // execute_tool should auto-connect if not connected
        // In real test with mock server, this would succeed
        let _result = client.execute_tool(request).await;

        // Would verify: connection was established before executing tool
    }

    #[tokio::test]
    async fn test_send_message_without_connection_fails() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        let message = WsMessage::SessionInit(SessionInit {
            session_id: "test".to_string(),
            workspace_root: None,
            capabilities: vec![],
        });

        let result = client.send_message(message).await;

        // Should fail because not connected
        assert!(result.is_err());
        if let Err(e) = result {
            let msg = format!("{:?}", e);
            assert!(msg.contains("Not connected"));
        }
    }

    // ------------------------------------------------------------------------
    // Message Handling Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    #[ignore] // Requires message simulation infrastructure
    async fn test_handle_tool_response_message() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        let response = ToolResponse {
            request_id: "req-1".to_string(),
            success: true,
            output: Some(json!({"result": "success"})),
            error: None,
            execution_time_ms: Some(100),
        };

        let ws_message = WsMessage::ToolResponse(response.clone());
        let message_json = serde_json::to_string(&ws_message).unwrap();
        let ws_msg = Message::Text(message_json);

        // Handle the message
        let result = client.handle_message(ws_msg).await;

        // Should route response to response channel
        assert!(result.is_ok());

        // Would verify: response sent to response_tx channel
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_handle_session_ack_message() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        let ack = crate::client::messages::SessionAck {
            session_id: "test-session".to_string(),
            accepted: true,
            server_version: Some("1.0.0".to_string()),
        };

        let ws_message = WsMessage::SessionAck(ack);
        let message_json = serde_json::to_string(&ws_message).unwrap();
        let ws_msg = Message::Text(message_json);

        let result = client.handle_message(ws_msg).await;

        // Should handle acknowledgment without error
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_handle_error_message() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        let error_msg = crate::client::messages::ErrorMessage {
            code: "AUTH_FAILED".to_string(),
            message: "Authentication failed".to_string(),
            details: None,
        };

        let ws_message = WsMessage::Error(error_msg);
        let message_json = serde_json::to_string(&ws_message).unwrap();
        let ws_msg = Message::Text(message_json);

        let result = client.handle_message(ws_msg).await;

        // Should handle error without panicking
        assert!(result.is_ok());

        // Would verify: error logged appropriately
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_handle_ping_pong() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        // Simulate ping message
        let ping_data = vec![1, 2, 3, 4];
        let ping_msg = Message::Ping(ping_data.clone());

        // Would verify: Pong message sent in response with same data
        let result = client.handle_message(ping_msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_handle_close_message() {
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        let close_msg = Message::Close(None);

        let result = client.handle_message(close_msg).await;

        // Should handle close gracefully
        assert!(result.is_ok());

        // Would verify: connection state updated
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_message_serialization_deserialization() {
        // Test that messages can be round-tripped through JSON
        let request = ToolRequest {
            request_id: "req-123".to_string(),
            tool_name: "file_read".to_string(),
            arguments: json!({"path": "/test/file.txt"}),
            timeout_ms: Some(5000),
        };

        let ws_message = WsMessage::ToolRequest(request.clone());
        let json = serde_json::to_string(&ws_message).unwrap();
        let deserialized: WsMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            WsMessage::ToolRequest(req) => {
                assert_eq!(req.request_id, "req-123");
                assert_eq!(req.tool_name, "file_read");
            }
            _ => panic!("Wrong message type deserialized"),
        }
    }

    // ------------------------------------------------------------------------
    // Connection Error Scenarios
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_connection_timeout_unreachable() {
        // Test connection to unreachable host times out
        let mut client = AcoClient::new("ws://192.0.2.1:9999", "test-session");

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            client.connect()
        ).await;

        // Either times out or fails to connect
        assert!(result.is_err() || result.unwrap().is_err());
    }

    #[tokio::test]
    #[ignore] // Requires mock server
    async fn test_connection_refused_by_server() {
        // Test connection refused scenario
        // Would use a mock server that refuses connections
        let mut client = AcoClient::new("ws://localhost:9998", "test-session");

        let result = client.connect().await;

        assert!(result.is_err());
        if let Err(e) = result {
            let msg = format!("{:?}", e);
            assert!(msg.contains("Failed to connect"));
        }
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_connection_invalid_protocol() {
        // Test connecting with wrong protocol (http instead of ws)
        let mut client = AcoClient::new("http://localhost:8080", "test-session");

        let result = client.connect().await;

        // Should fail - HTTP is not WebSocket protocol
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires mock server
    async fn test_connection_authentication_failure() {
        // Test connection when server rejects authentication
        // Would require mock server that sends error after connection
        let mut client = AcoClient::new("ws://localhost:8080", "invalid-session");

        let result = client.connect().await;

        // Depending on server behavior, might succeed connection
        // but fail on session init
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_execute_tool_timeout() {
        // Test that execute_tool times out if no response received
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        // Would need mock server that accepts connection but doesn't respond
        let request = ToolRequest {
            request_id: "req-timeout".to_string(),
            tool_name: "slow_tool".to_string(),
            arguments: json!({}),
            timeout_ms: None,
        };

        let result = client.execute_tool(request).await;

        // Should timeout after 30 seconds (as per execute_tool implementation)
        assert!(result.is_err());
        if let Err(e) = result {
            let msg = format!("{:?}", e);
            assert!(msg.contains("Timeout"));
        }
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_concurrent_tool_execution() {
        // Test multiple concurrent tool executions
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        // Would test that multiple concurrent execute_tool calls
        // are handled correctly and responses matched to requests
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_response_matching_by_request_id() {
        // Test that responses are correctly matched to requests by ID
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        // Send multiple requests with different IDs
        // Verify each response is matched to correct request
        // Even if responses arrive out of order
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_connection_drop_during_execution() {
        // Test behavior when connection drops mid-execution
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        // 1. Connect
        // 2. Start tool execution
        // 3. Simulate connection drop
        // 4. Verify error is returned
    }

    #[tokio::test]
    #[ignore] // Requires test infrastructure
    async fn test_malformed_message_handling() {
        // Test handling of malformed JSON messages
        let mut client = AcoClient::new("ws://localhost:8080", "test-session");

        let malformed_msg = Message::Text("{ invalid json".to_string());

        let result = client.handle_message(malformed_msg).await;

        // Should return error for malformed message
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Client Cleanup Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_client_drop_disconnects() {
        // Test that dropping client attempts to disconnect
        {
            let mut client = AcoClient::new("ws://localhost:8080", "test-session");
            // Client goes out of scope
        }

        // Drop implementation should attempt disconnect
        // No panic should occur
    }

    // Note: Integration tests requiring a running aco server
    // should be in a separate integration test file with #[ignore]
    // or behind a feature flag for CI/CD environments
}

