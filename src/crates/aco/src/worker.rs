//! ACO Worker - connects to orchestrator and executes tools
//!
//! The worker registers with the orchestrator, subscribes to an SSE event
//! stream, and executes tool requests as they arrive.

use crate::error::{AcoError, Result};
use langgraph_prebuilt::Tool;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Worker registration request
#[derive(Debug, Serialize)]
struct RegisterWorkerRequest {
    name: String,
    capabilities: Vec<String>,
    workspace_path: String,
}

/// Worker registration response
#[derive(Debug, Deserialize)]
struct RegisterWorkerResponse {
    worker_id: String,
    heartbeat_interval_ms: u64,
}

/// Tool execution request from orchestrator
#[derive(Debug, Clone, Deserialize)]
pub struct ToolRequest {
    pub request_id: String,
    pub worker_id: String,
    pub tool_name: String,
    pub arguments: Value,
    pub timeout_ms: u64,
}

/// Tool execution result to send back
#[derive(Debug, Serialize)]
struct ToolResult {
    request_id: String,
    worker_id: String,
    success: bool,
    output: Option<Value>,
    error: Option<String>,
    duration_ms: i64,
}

/// SSE event wrapper
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorkerEvent {
    ToolRequest(ToolRequest),
    Heartbeat { timestamp: String },
    Shutdown { reason: String },
}

/// ACO Worker that connects to orchestrator
pub struct AcoWorker {
    /// Orchestrator base URL
    orchestrator_url: String,
    /// Worker name
    name: String,
    /// Workspace root path
    workspace_path: String,
    /// Assigned worker ID (after registration)
    worker_id: Option<String>,
    /// Registered tools
    tools: HashMap<String, Box<dyn Tool>>,
    /// HTTP client
    http_client: Client,
    /// Heartbeat interval
    heartbeat_interval: Duration,
}

impl AcoWorker {
    /// Create a new worker
    pub fn new(orchestrator_url: String, name: Option<String>, workspace_path: String) -> Self {
        let worker_name = name.unwrap_or_else(|| {
            format!("aco-worker-{}", hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()))
        });

        let http_client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            orchestrator_url,
            name: worker_name,
            workspace_path,
            worker_id: None,
            tools: HashMap::new(),
            http_client,
            heartbeat_interval: Duration::from_secs(30),
        }
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        info!("Registering tool: {}", name);
        self.tools.insert(name, tool);
    }

    /// Get list of capabilities (tool names)
    fn capabilities(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Start the worker
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting worker {} connecting to {}", self.name, self.orchestrator_url);

        // Register with orchestrator
        self.register().await?;

        // Connect to SSE event stream and process events
        self.run_event_loop().await
    }

    /// Register with orchestrator
    async fn register(&mut self) -> Result<()> {
        let url = format!("{}/grpc/workers/register", self.orchestrator_url);

        let request = RegisterWorkerRequest {
            name: self.name.clone(),
            capabilities: self.capabilities(),
            workspace_path: self.workspace_path.clone(),
        };

        info!("Registering with orchestrator at {}", url);
        debug!("Registration request: {:?}", request);

        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to register: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AcoError::Connection(format!(
                "Registration failed: {} - {}", status, error_text
            )));
        }

        let reg_response: RegisterWorkerResponse = response.json().await
            .map_err(|e| AcoError::Connection(format!("Failed to parse registration response: {}", e)))?;

        self.worker_id = Some(reg_response.worker_id.clone());
        self.heartbeat_interval = Duration::from_millis(reg_response.heartbeat_interval_ms);

        info!("Registered as worker {} with {} tools",
              reg_response.worker_id,
              self.tools.len());

        Ok(())
    }

    /// Run the main event loop processing SSE events
    async fn run_event_loop(&self) -> Result<()> {
        let worker_id = self.worker_id.as_ref()
            .ok_or_else(|| AcoError::Connection("Worker not registered".to_string()))?;

        let url = format!("{}/grpc/workers/events?worker_id={}",
                         self.orchestrator_url, worker_id);

        info!("Connecting to event stream at {}", url);

        loop {
            match self.connect_and_process(&url).await {
                Ok(()) => {
                    info!("Event stream closed normally");
                    break;
                }
                Err(e) => {
                    error!("Event stream error: {}, reconnecting in 5s...", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }

        Ok(())
    }

    /// Connect to SSE stream and process events
    async fn connect_and_process(&self, url: &str) -> Result<()> {
        let response = self.http_client
            .get(url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to connect to event stream: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AcoError::Connection(format!(
                "Event stream connection failed: {}", status
            )));
        }

        info!("Connected to event stream");

        // Process SSE events
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AcoError::Connection(format!("Stream error: {}", e)))?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            // Process complete events
            while let Some(event) = self.parse_sse_event(&mut buffer) {
                self.handle_event(event).await?;
            }
        }

        Ok(())
    }

    /// Parse SSE event from buffer
    fn parse_sse_event(&self, buffer: &mut String) -> Option<WorkerEvent> {
        // Look for complete event (ends with double newline)
        if let Some(end_pos) = buffer.find("\n\n") {
            let event_str = buffer[..end_pos].to_string();
            buffer.drain(..=end_pos + 1);

            // Parse SSE format: event: type\ndata: json
            let mut event_type = None;
            let mut data = None;

            for line in event_str.lines() {
                if let Some(t) = line.strip_prefix("event:") {
                    event_type = Some(t.trim().to_string());
                } else if let Some(d) = line.strip_prefix("data:") {
                    data = Some(d.trim().to_string());
                }
            }

            if let Some(json_data) = data {
                match serde_json::from_str::<WorkerEvent>(&json_data) {
                    Ok(event) => return Some(event),
                    Err(e) => {
                        debug!("Failed to parse event: {} - data: {}", e, json_data);
                    }
                }
            }
        }
        None
    }

    /// Handle a single event
    async fn handle_event(&self, event: WorkerEvent) -> Result<()> {
        match event {
            WorkerEvent::ToolRequest(request) => {
                info!("Received tool request: {} ({})", request.tool_name, request.request_id);

                // Execute tool and send result
                let result = self.execute_tool(&request).await;
                self.send_result(result).await?;
            }
            WorkerEvent::Heartbeat { timestamp } => {
                debug!("Received heartbeat: {}", timestamp);
            }
            WorkerEvent::Shutdown { reason } => {
                warn!("Received shutdown: {}", reason);
                return Err(AcoError::Connection(format!("Server shutdown: {}", reason)));
            }
        }
        Ok(())
    }

    /// Execute a tool
    async fn execute_tool(&self, request: &ToolRequest) -> ToolResult {
        let worker_id = self.worker_id.clone().unwrap_or_default();
        let start = Instant::now();

        let tool = match self.tools.get(&request.tool_name) {
            Some(t) => t,
            None => {
                return ToolResult {
                    request_id: request.request_id.clone(),
                    worker_id,
                    success: false,
                    output: None,
                    error: Some(format!("Unknown tool: {}", request.tool_name)),
                    duration_ms: start.elapsed().as_millis() as i64,
                };
            }
        };

        info!("Executing tool: {} with args: {}", request.tool_name, request.arguments);

        // Execute with timeout
        let timeout = Duration::from_millis(request.timeout_ms);
        let result = tokio::time::timeout(
            timeout,
            tool.execute(request.arguments.clone())
        ).await;

        let duration_ms = start.elapsed().as_millis() as i64;

        match result {
            Ok(Ok(output)) => {
                info!("Tool {} completed successfully in {}ms", request.tool_name, duration_ms);
                ToolResult {
                    request_id: request.request_id.clone(),
                    worker_id,
                    success: true,
                    output: Some(output),
                    error: None,
                    duration_ms,
                }
            }
            Ok(Err(e)) => {
                warn!("Tool {} failed: {}", request.tool_name, e);
                ToolResult {
                    request_id: request.request_id.clone(),
                    worker_id,
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                    duration_ms,
                }
            }
            Err(_) => {
                warn!("Tool {} timed out after {:?}", request.tool_name, timeout);
                ToolResult {
                    request_id: request.request_id.clone(),
                    worker_id,
                    success: false,
                    output: None,
                    error: Some(format!("Timed out after {:?}", timeout)),
                    duration_ms,
                }
            }
        }
    }

    /// Send tool result back to orchestrator
    async fn send_result(&self, result: ToolResult) -> Result<()> {
        let url = format!("{}/grpc/workers/results", self.orchestrator_url);

        debug!("Sending result for request {}: success={}", result.request_id, result.success);

        let response = self.http_client
            .post(&url)
            .json(&result)
            .send()
            .await
            .map_err(|e| AcoError::Connection(format!("Failed to send result: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            warn!("Failed to send result: {} - {}", status, error_text);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_creation() {
        let worker = AcoWorker::new(
            "http://localhost:8080".to_string(),
            Some("test-worker".to_string()),
            "/tmp".to_string(),
        );
        assert_eq!(worker.name, "test-worker");
        assert!(worker.worker_id.is_none());
    }

    #[test]
    fn test_capabilities() {
        let worker = AcoWorker::new(
            "http://localhost:8080".to_string(),
            None,
            "/tmp".to_string(),
        );
        assert!(worker.capabilities().is_empty());
    }
}
