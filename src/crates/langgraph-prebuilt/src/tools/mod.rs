//! Tools - Type-Safe Tool Abstractions for Agents
//!
//! This module provides a **trait-based tool system** that enables agents to interact
//! with external systems, APIs, and functions. Tools are the primary way agents take
//! actions in the world.
//!
//! # Overview
//!
//! The tool system provides:
//! - **[`Tool`] trait** - Define custom tools with async execution
//! - **[`ToolRegistry`]** - Manage and execute multiple tools
//! - **[`ToolValidation`]** - Schema validation and constraints (via validation module)
//! - **Type-safe I/O** - JSON-based input/output with validation
//!
//! **Common tool use cases:**
//! - Search engines (web search, document search)
//! - APIs (weather, news, databases)
//! - File operations (read, write, list)
//! - Calculators and data processors
//! - Code execution (sandboxed)
//!
//! # Quick Start
//!
//! ## Implementing a Simple Tool
//!
//! ```rust
//! use langgraph_prebuilt::{Tool, ToolInput, ToolOutput, Result};
//! use async_trait::async_trait;
//! use serde_json::json;
//!
//! #[derive(Clone)]
//! struct SearchTool;
//!
//! #[async_trait]
//! impl Tool for SearchTool {
//!     fn name(&self) -> &str {
//!         "search"
//!     }
//!
//!     fn description(&self) -> &str {
//!         "Search the web for information"
//!     }
//!
//!     async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
//!         let query = input["query"].as_str().unwrap_or("");
//!
//!         // Perform search (mock here)
//!         let results = format!("Results for: {}", query);
//!
//!         Ok(json!({"results": results}))
//!     }
//! }
//! ```
//!
//! ## Using ToolRegistry
//!
//! ```rust,ignore
//! use langgraph_prebuilt::ToolRegistry;
//!
//! let mut registry = ToolRegistry::new();
//! registry.register(Box::new(SearchTool));
//! registry.register(Box::new(CalculatorTool));
//!
//! // Execute tool by name
//! let result = registry.execute("search",
//!     json!({"query": "rust async"})).await?;
//!
//! // List available tools
//! let tools = registry.list_tools(); // ["search", "calculator"]
//! ```
//!
//! # Tool Implementation Patterns
//!
//! ## Pattern 1: HTTP API Tool
//!
//! ```rust,ignore
//! use reqwest::Client;
//!
//! struct WeatherTool {
//!     client: Client,
//!     api_key: String,
//! }
//!
//! #[async_trait]
//! impl Tool for WeatherTool {
//!     fn name(&self) -> &str { "weather" }
//!
//!     fn description(&self) -> &str {
//!         "Get current weather for a city"
//!     }
//!
//!     fn input_schema(&self) -> Option<Value> {
//!         Some(json!({
//!             "type": "object",
//!             "properties": {
//!                 "city": {"type": "string"}
//!             },
//!             "required": ["city"]
//!         }))
//!     }
//!
//!     async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
//!         let city = input["city"].as_str()
//!             .ok_or_else(|| PrebuiltError::InvalidInput("city required".into()))?;
//!
//!         let url = format!("https://api.weather.com/v1/{}?key={}",
//!                          city, self.api_key);
//!
//!         let response = self.client.get(&url).send().await
//!             .map_err(|e| PrebuiltError::ToolExecution(e.to_string()))?;
//!
//!         let data = response.json::<Value>().await
//!             .map_err(|e| PrebuiltError::ToolExecution(e.to_string()))?;
//!
//!         Ok(data)
//!     }
//! }
//! ```
//!
//! ## Pattern 2: Database Query Tool
//!
//! ```rust,ignore
//! struct DatabaseTool {
//!     pool: sqlx::PgPool,
//! }
//!
//! #[async_trait]
//! impl Tool for DatabaseTool {
//!     fn name(&self) -> &str { "db_query" }
//!
//!     fn description(&self) -> &str {
//!         "Execute read-only database queries"
//!     }
//!
//!     fn validate_input(&self, input: &ToolInput) -> Result<()> {
//!         let query = input["query"].as_str()
//!             .ok_or_else(|| PrebuiltError::InvalidInput("query required".into()))?;
//!
//!         // Security: Only allow SELECT
//!         if !query.trim().to_uppercase().starts_with("SELECT") {
//!             return Err(PrebuiltError::ToolValidation(
//!                 "Only SELECT queries allowed".into()
//!             ));
//!         }
//!
//!         Ok(())
//!     }
//!
//!     async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
//!         let query = input["query"].as_str().unwrap();
//!
//!         let rows = sqlx::query(query)
//!             .fetch_all(&self.pool)
//!             .await
//!             .map_err(|e| PrebuiltError::ToolExecution(e.to_string()))?;
//!
//!         // Convert rows to JSON
//!         let results = rows.iter().map(|row| {
//!             // Row to JSON conversion
//!             json!({"data": "..."})
//!         }).collect::<Vec<_>>();
//!
//!         Ok(json!({"rows": results}))
//!     }
//! }
//! ```
//!
//! ## Pattern 3: File System Tool
//!
//! ```rust,ignore
//! struct FileReadTool {
//!     allowed_dirs: Vec<PathBuf>,
//! }
//!
//! #[async_trait]
//! impl Tool for FileReadTool {
//!     fn name(&self) -> &str { "read_file" }
//!
//!     fn description(&self) -> &str {
//!         "Read contents of a file"
//!     }
//!
//!     fn validate_input(&self, input: &ToolInput) -> Result<()> {
//!         let path = Path::new(input["path"].as_str().unwrap_or(""));
//!
//!         // Security: Check path is in allowed directories
//!         let allowed = self.allowed_dirs.iter()
//!             .any(|dir| path.starts_with(dir));
//!
//!         if !allowed {
//!             return Err(PrebuiltError::ToolValidation(
//!                 "Path not in allowed directories".into()
//!             ));
//!         }
//!
//!         Ok(())
//!     }
//!
//!     async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
//!         let path = input["path"].as_str().unwrap();
//!         let content = tokio::fs::read_to_string(path).await
//!             .map_err(|e| PrebuiltError::ToolExecution(e.to_string()))?;
//!
//!         Ok(json!({"content": content}))
//!     }
//! }
//! ```
//!
//! ## Pattern 4: Calculator Tool with Schema
//!
//! ```rust,ignore
//! struct CalculatorTool;
//!
//! #[async_trait]
//! impl Tool for CalculatorTool {
//!     fn name(&self) -> &str { "calculator" }
//!
//!     fn description(&self) -> &str {
//!         "Perform arithmetic operations"
//!     }
//!
//!     fn input_schema(&self) -> Option<Value> {
//!         Some(json!({
//!             "type": "object",
//!             "properties": {
//!                 "operation": {
//!                     "type": "string",
//!                     "enum": ["add", "subtract", "multiply", "divide"]
//!                 },
//!                 "a": {"type": "number"},
//!                 "b": {"type": "number"}
//!             },
//!             "required": ["operation", "a", "b"]
//!         }))
//!     }
//!
//!     async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
//!         let op = input["operation"].as_str().unwrap();
//!         let a = input["a"].as_f64().unwrap();
//!         let b = input["b"].as_f64().unwrap();
//!
//!         let result = match op {
//!             "add" => a + b,
//!             "subtract" => a - b,
//!             "multiply" => a * b,
//!             "divide" => {
//!                 if b == 0.0 {
//!                     return Err(PrebuiltError::ToolExecution(
//!                         "Division by zero".into()
//!                     ));
//!                 }
//!                 a / b
//!             }
//!             _ => return Err(PrebuiltError::InvalidInput("Invalid operation".into())),
//!         };
//!
//!         Ok(json!({"result": result}))
//!     }
//! }
//! ```
//!
//! # Tool Best Practices
//!
//! 1. **Clear Descriptions**: LLMs use descriptions to decide when to call tools
//!    ```rust,ignore
//!    fn description(&self) -> &str {
//!        "Search for current weather in a city. \
//!         Input: city name (string). \
//!         Output: temperature, conditions, humidity."
//!    }
//!    ```
//!
//! 2. **Input Validation**: Always validate and sanitize inputs
//!    ```rust,ignore
//!    fn validate_input(&self, input: &ToolInput) -> Result<()> {
//!        // Check required fields
//!        // Validate types
//!        // Apply security constraints
//!        Ok(())
//!    }
//!    ```
//!
//! 3. **Error Handling**: Return descriptive errors
//!    ```rust,ignore
//!    .map_err(|e| PrebuiltError::ToolExecution(
//!        format!("API call failed: {}", e)
//!    ))
//!    ```
//!
//! 4. **JSON Schema**: Provide schemas for better LLM understanding
//!    ```rust,ignore
//!    fn input_schema(&self) -> Option<Value> {
//!        Some(json!({
//!            "type": "object",
//!            "properties": { /* ... */ }
//!        }))
//!    }
//!    ```
//!
//! 5. **Idempotency**: Tools should be idempotent when possible
//!    - Safe for retries
//!    - No unintended side effects
//!
//! # ToolRegistry Usage
//!
//! ## Organizing Tools
//!
//! ```rust,ignore
//! fn create_agent_tools() -> ToolRegistry {
//!     let mut registry = ToolRegistry::new();
//!
//!     // Search tools
//!     registry.register(Box::new(WebSearchTool::new()));
//!     registry.register(Box::new(DocumentSearchTool::new()));
//!
//!     // Data tools
//!     registry.register(Box::new(DatabaseTool::new()));
//!     registry.register(Box::new(CalculatorTool));
//!
//!     // IO tools
//!     registry.register(Box::new(FileReadTool::new()));
//!     registry.register(Box::new(HttpRequestTool::new()));
//!
//!     registry
//! }
//! ```
//!
//! ## Dynamic Tool Loading
//!
//! ```rust,ignore
//! fn load_tools_from_config(config: &ToolConfig) -> Result<ToolRegistry> {
//!     let mut registry = ToolRegistry::new();
//!
//!     for tool_spec in &config.enabled_tools {
//!         let tool: Box<dyn Tool> = match tool_spec.name.as_str() {
//!             "search" => Box::new(SearchTool::from_config(&tool_spec.config)?),
//!             "weather" => Box::new(WeatherTool::from_config(&tool_spec.config)?),
//!             _ => return Err(PrebuiltError::ToolValidation(
//!                 format!("Unknown tool: {}", tool_spec.name)
//!             )),
//!         };
//!         registry.register(tool);
//!     }
//!
//!     Ok(registry)
//! }
//! ```
//!
//! # Python LangGraph Comparison
//!
//! | Python LangGraph | rLangGraph (Rust) |
//! |------------------|-------------------|
//! | `@tool` decorator | `impl Tool` trait |
//! | `def tool_func(...)` | `async fn execute(...)` |
//! | `BaseTool` class | `Tool` trait |
//! | `tool.invoke(input)` | `tool.execute(input).await` |
//! | Dynamic typing | Strong typing with JSON Value |
//! | Sync by default | Async-first |
//!
//! **Python Example:**
//! ```python
//! from langchain_core.tools import tool
//!
//! @tool
//! def search(query: str) -> str:
//!     \"\"\"Search for information\"\"\"
//!     return "results..."
//!
//! result = search.invoke({"query": "rust"})
//! ```
//!
//! **Rust Equivalent:**
//! ```rust,ignore
//! struct SearchTool;
//!
//! #[async_trait]
//! impl Tool for SearchTool {
//!     fn name(&self) -> &str { "search" }
//!     fn description(&self) -> &str { "Search for information" }
//!
//!     async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
//!         Ok(json!({"results": "results..."}))
//!     }
//! }
//!
//! let tool = SearchTool;
//! let result = tool.execute(json!({"query": "rust"})).await?;
//! ```
//!
//! # See Also
//!
//! - [`Tool`] - Tool trait definition
//! - [`ToolRegistry`] - Tool management
//! - [`validation`] - Tool validation and constraints
//! - [`crate::tool_node::ToolNode`] - Graph node for tool execution
//! - [`crate::agents::create_react_agent`] - ReAct agent with tools
//! - [`crate::messages::ToolCall`] - Tool invocation from LLM

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod validation;

pub use validation::{
    ToolValidation, ValidatedTool, ToolValidationBuilder,
    ParameterValidation, ParameterType, ExecutionConstraints,
    SecurityPolicy, ToolMetrics, string_param, number_param
};

/// Tool input type
pub type ToolInput = Value;

/// Tool output type
pub type ToolOutput = Value;

/// Tool trait for implementing agent tools
///
/// # Example
///
/// ```rust
/// use langgraph_prebuilt::{Tool, ToolInput, ToolOutput};
/// use async_trait::async_trait;
///
/// struct CalculatorTool;
///
/// #[async_trait]
/// impl Tool for CalculatorTool {
///     fn name(&self) -> &str {
///         "calculator"
///     }
///
///     fn description(&self) -> &str {
///         "Performs basic arithmetic operations"
///     }
///
///     async fn execute(&self, input: ToolInput) -> langgraph_prebuilt::Result<ToolOutput> {
///         // Implementation
///         Ok(serde_json::json!({"result": 42}))
///     }
/// }
/// ```
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name
    fn name(&self) -> &str;

    /// Get the tool description
    fn description(&self) -> &str;

    /// Get the input schema (optional)
    fn input_schema(&self) -> Option<Value> {
        None
    }

    /// Execute the tool with the given input
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput>;

    /// Validate input (optional)
    fn validate_input(&self, _input: &ToolInput) -> Result<()> {
        Ok(())
    }
}

/// Tool metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// Input schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,

    /// Additional metadata
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

/// Tool registry for managing multiple tools
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: std::collections::HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// List all tool names
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get metadata for all tools
    pub fn list_metadata(&self) -> Vec<ToolMetadata> {
        self.tools
            .values()
            .map(|tool| ToolMetadata {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.input_schema(),
                extra: std::collections::HashMap::new(),
            })
            .collect()
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, input: ToolInput) -> Result<ToolOutput> {
        let tool = self.get(name).ok_or_else(|| {
            crate::error::PrebuiltError::ToolExecution(format!("Tool not found: {}", name))
        })?;

        tool.validate_input(&input)?;
        tool.execute(input).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock"
        }

        fn description(&self) -> &str {
            "A mock tool for testing"
        }

        async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
            Ok(serde_json::json!({
                "echo": input
            }))
        }
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let tool = MockTool;
        let input = serde_json::json!({"test": "value"});
        let output = tool.execute(input.clone()).await.unwrap();

        assert_eq!(output["echo"], input);
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockTool));

        assert!(registry.get("mock").is_some());
        assert_eq!(registry.list_tools(), vec!["mock"]);

        let input = serde_json::json!({"test": "value"});
        let output = registry.execute("mock", input.clone()).await.unwrap();

        assert_eq!(output["echo"], input);
    }

    #[test]
    fn test_tool_metadata() {
        let registry = {
            let mut r = ToolRegistry::new();
            r.register(Box::new(MockTool));
            r
        };

        let metadata = registry.list_metadata();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].name, "mock");
        assert_eq!(metadata[0].description, "A mock tool for testing");
    }
}