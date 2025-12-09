//! Agent Service - LLM agent with tool execution via connected workers
//!
//! This service provides an agentic loop that:
//! 1. Sends prompts to an LLM with available tool definitions
//! 2. When the LLM requests tool calls, dispatches them to workers
//! 3. Returns tool results to the LLM
//! 4. Continues until the LLM provides a final response

use crate::config::LlmConfig;
use crate::grpc::{ToolDispatcher, WorkerRegistry};
use langgraph_core::llm::{ChatModel, ChatRequest, ToolDefinition};
use langgraph_core::messages::Message;
use langgraph_core::tool::ToolCall;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info, warn};

/// Maximum iterations in the agent loop to prevent infinite loops
const MAX_ITERATIONS: usize = 10;

/// Default timeout for tool execution (30 seconds)
const DEFAULT_TOOL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent not configured - LLM not enabled")]
    NotConfigured,
    #[error("No workers available for tool execution")]
    NoWorkers,
    #[error("LLM error: {0}")]
    LlmError(String),
    #[error("Tool execution error: {0}")]
    ToolError(String),
    #[error("Max iterations exceeded")]
    MaxIterationsExceeded,
    #[error("Missing API key: {0}")]
    MissingApiKey(String),
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),
}

/// Result of agent execution
#[derive(Debug, Clone)]
pub struct AgentResult {
    /// Final response from the agent
    pub response: String,
    /// Tool calls made during execution
    pub tool_calls: Vec<ToolCallRecord>,
    /// Number of iterations
    pub iterations: usize,
}

/// Record of a tool call made during agent execution
#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub arguments: Value,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub duration_ms: i64,
}

/// Agent service for agentic LLM interactions with tool execution
pub struct AgentService {
    /// LLM client
    provider: Box<dyn ChatModel>,
    /// Tool dispatcher for executing tools via workers
    dispatcher: Arc<ToolDispatcher>,
    /// Worker registry for getting tool definitions
    registry: Arc<WorkerRegistry>,
    /// System prompt for the agent
    system_prompt: Option<String>,
}

impl AgentService {
    /// Create a new AgentService
    pub fn new(
        config: &LlmConfig,
        dispatcher: Arc<ToolDispatcher>,
        registry: Arc<WorkerRegistry>,
    ) -> Result<Self, AgentError> {
        if !config.enabled {
            return Err(AgentError::NotConfigured);
        }

        let provider = Self::create_provider(config)?;

        Ok(Self {
            provider,
            dispatcher,
            registry,
            system_prompt: None,
        })
    }

    /// Create LLM provider from config
    fn create_provider(config: &LlmConfig) -> Result<Box<dyn ChatModel>, AgentError> {
        use llm::{LocalLlmConfig, RemoteLlmConfig};

        let provider: Box<dyn ChatModel> = match config.provider.to_lowercase().as_str() {
            "ollama" => {
                let api_base = config.api_base.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
                let local_config = LocalLlmConfig::new(&api_base, &config.model);
                Box::new(llm::local::OllamaClient::new(local_config))
            }
            "claude" | "anthropic" => {
                let api_key = config.get_api_key()
                    .ok_or_else(|| AgentError::MissingApiKey("claude".to_string()))?;
                let api_base = config.api_base.clone().unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
                let remote_config = RemoteLlmConfig::new(api_key, api_base, config.model.clone());
                Box::new(llm::remote::ClaudeClient::new(remote_config))
            }
            "openai" => {
                let api_key = config.get_api_key()
                    .ok_or_else(|| AgentError::MissingApiKey("openai".to_string()))?;
                let api_base = config.api_base.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                let remote_config = RemoteLlmConfig::new(api_key, api_base, config.model.clone());
                Box::new(llm::remote::OpenAiClient::new(remote_config))
            }
            other => return Err(AgentError::UnsupportedProvider(other.to_string())),
        };

        Ok(provider)
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Get tool definitions from connected workers
    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        let workers = self.registry.list_workers();

        let mut tools = Vec::new();
        let mut seen_tools = std::collections::HashSet::new();

        for worker in workers {
            for capability in &worker.capabilities {
                // Skip if we've already added this tool
                if seen_tools.contains(capability) {
                    continue;
                }
                seen_tools.insert(capability.clone());

                // Create tool definition using builder pattern
                let tool = ToolDefinition::new(
                    capability.clone(),
                    format!("Execute the {} tool", capability),
                ).with_parameters(json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": true
                }));
                tools.push(tool);
            }
        }

        tools
    }

    /// Run the agentic loop
    pub async fn run(&self, prompt: &str) -> Result<AgentResult, AgentError> {
        info!("Starting agent with prompt: {}...", &prompt[..prompt.len().min(50)]);

        // Check if we have any workers
        if self.registry.list_workers().is_empty() {
            warn!("No workers connected - agent will run without tools");
        }

        let mut messages = vec![];

        // Add system prompt if configured
        if let Some(ref system) = self.system_prompt {
            messages.push(Message::system(system.as_str()));
        } else {
            messages.push(Message::system(self.default_system_prompt()));
        }

        // Add user prompt
        messages.push(Message::human(prompt));

        // Get tool definitions
        let tools = self.get_tool_definitions();
        debug!("Available tools: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

        let mut tool_call_records = Vec::new();
        let mut iterations = 0;

        // Agentic loop
        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                return Err(AgentError::MaxIterationsExceeded);
            }

            debug!("Agent iteration {}", iterations);

            // Create request with tools
            let mut request = ChatRequest::new(messages.clone());
            if !tools.is_empty() {
                request = request.with_tools(tools.clone());
            }

            // Call LLM
            let response = self.provider.chat(request).await.map_err(|e| {
                AgentError::LlmError(e.to_string())
            })?;

            // Check for tool calls
            if let Some(ref tool_calls) = response.message.tool_calls {
                if !tool_calls.is_empty() {
                    debug!("LLM requested {} tool calls", tool_calls.len());

                    // Add assistant message with tool calls
                    messages.push(response.message.clone());

                    // Execute each tool call
                    for tool_call in tool_calls {
                        let record = self.execute_tool_call(tool_call).await;

                        // Add tool result message using Message::tool
                        let tool_content = record.result.clone()
                            .or_else(|| record.error.clone().map(|e| json!({"error": e})))
                            .unwrap_or(json!({"error": "unknown"}));
                        let tool_result_msg = Message::tool(
                            serde_json::to_string(&tool_content).unwrap_or_default(),
                            &tool_call.id,
                        ).with_name(&tool_call.name);
                        messages.push(tool_result_msg);

                        tool_call_records.push(record);
                    }

                    // Continue loop to get LLM response to tool results
                    continue;
                }
            }

            // No tool calls - this is the final response
            let final_response = response.message.text()
                .unwrap_or("No response")
                .to_string();

            info!("Agent completed in {} iterations", iterations);

            return Ok(AgentResult {
                response: final_response,
                tool_calls: tool_call_records,
                iterations,
            });
        }
    }

    /// Execute a single tool call via the dispatcher
    async fn execute_tool_call(&self, tool_call: &ToolCall) -> ToolCallRecord {
        debug!("Executing tool: {} with args: {}", tool_call.name, tool_call.args);

        let start = std::time::Instant::now();

        let result = self.dispatcher.execute_tool(
            &tool_call.name,
            tool_call.args.clone(),
            DEFAULT_TOOL_TIMEOUT,
        ).await;

        let duration_ms = start.elapsed().as_millis() as i64;

        match result {
            Ok(tool_result) => {
                debug!("Tool {} completed: success={}", tool_call.name, tool_result.success);
                ToolCallRecord {
                    tool_name: tool_call.name.clone(),
                    arguments: tool_call.args.clone(),
                    result: tool_result.output,
                    error: tool_result.error,
                    duration_ms,
                }
            }
            Err(e) => {
                warn!("Tool {} failed: {}", tool_call.name, e);
                ToolCallRecord {
                    tool_name: tool_call.name.clone(),
                    arguments: tool_call.args.clone(),
                    result: None,
                    error: Some(e.to_string()),
                    duration_ms,
                }
            }
        }
    }

    /// Default system prompt for the agent
    fn default_system_prompt(&self) -> String {
        r#"You are a helpful assistant that can use tools to accomplish tasks.

When you need to perform an action like reading files, executing commands, or searching,
use the available tools. Analyze tool results and provide helpful responses to the user.

Be concise and focus on completing the task effectively."#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_not_configured() {
        let config = LlmConfig::default(); // enabled = false
        let registry = Arc::new(WorkerRegistry::new());
        let dispatcher = Arc::new(ToolDispatcher::new(registry.clone()));

        let result = AgentService::new(&config, dispatcher, registry);
        assert!(matches!(result, Err(AgentError::NotConfigured)));
    }
}
