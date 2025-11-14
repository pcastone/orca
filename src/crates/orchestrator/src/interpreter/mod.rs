//! Action Interpreter for bridging LLM outputs to tool calls.
//!
//! This module provides components for:
//! - Parsing LLM output (JSON or natural language)
//! - Mapping intents to structured ToolRequests
//! - Validating tool calls before execution
//! - Formatting ToolResponses back to natural language

pub mod parser;
pub mod mapper;
pub mod validator;
pub mod formatter;

pub use parser::{IntentParser, ParsedIntent, ToolCall};
pub use mapper::ToolMapper;
pub use validator::ToolValidator;
pub use formatter::ResultFormatter;

/// Main Action Interpreter that coordinates all components
pub struct ActionInterpreter {
    parser: IntentParser,
    mapper: ToolMapper,
    validator: ToolValidator,
    formatter: ResultFormatter,
}

impl ActionInterpreter {
    /// Create a new Action Interpreter
    pub fn new() -> Self {
        Self {
            parser: IntentParser::new(),
            mapper: ToolMapper::new(),
            validator: ToolValidator::new(),
            formatter: ResultFormatter::new(),
        }
    }

    /// Parse LLM output and convert to ToolRequest
    pub async fn interpret(
        &self,
        llm_output: &str,
        session_id: &str,
    ) -> crate::Result<tooling::runtime::ToolRequest> {
        // 1. Parse LLM output
        let intent = self.parser.parse(llm_output)?;

        // 2. Map to ToolRequest
        let tool_request = self.mapper.map_to_tool_request(intent, session_id)?;

        // 3. Validate before execution
        self.validator.validate(&tool_request)?;

        Ok(tool_request)
    }

    /// Format ToolResponse to natural language for LLM
    pub fn format_response(
        &self,
        response: &tooling::runtime::ToolResponse,
    ) -> String {
        self.formatter.format_for_llm(response)
    }

    /// Format multiple responses with summarization
    pub fn format_responses(
        &self,
        responses: &[tooling::runtime::ToolResponse],
    ) -> String {
        self.formatter.summarize(responses)
    }
}

impl Default for ActionInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

