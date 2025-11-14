//! Intent Parser for extracting tool calls from LLM output.
//!
//! Supports both structured JSON output (preferred) and natural language parsing (fallback).

use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Parsed intent from LLM output
#[derive(Debug, Clone)]
pub enum ParsedIntent {
    /// Direct structured tool call from LLM (JSON)
    StructuredTool(ToolCall),
    /// Natural language that needs interpretation
    NaturalLanguage(String),
    /// Multiple possible tool calls (ambiguous)
    Ambiguous(Vec<ToolCall>),
}

/// Structured tool call representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name (e.g., "file_read", "git_status")
    pub tool: String,
    /// Tool arguments as JSON object
    pub args: Value,
}

/// Intent Parser for LLM outputs
pub struct IntentParser {
    /// Available tool schemas for validation
    tool_schemas: HashMap<String, ToolSchema>,
}

/// Tool schema information
#[derive(Debug, Clone)]
struct ToolSchema {
    name: String,
    description: String,
    input_schema: Value,
}

impl IntentParser {
    /// Create a new Intent Parser
    pub fn new() -> Self {
        Self {
            tool_schemas: HashMap::new(),
        }
    }

    /// Register a tool schema
    pub fn register_tool(&mut self, name: String, description: String, input_schema: Value) {
        self.tool_schemas.insert(
            name.clone(),
            ToolSchema {
                name,
                description,
                input_schema,
            },
        );
    }

    /// Parse LLM output to extract tool calls
    ///
    /// Attempts to parse as JSON first (structured output), then falls back
    /// to natural language interpretation if needed.
    pub fn parse(&self, llm_output: &str) -> Result<ParsedIntent> {
        let trimmed = llm_output.trim();

        // Try to extract JSON from the output
        // LLM might wrap JSON in markdown code blocks or include extra text
        if let Some(json_str) = self.extract_json(trimmed) {
            match self.parse_json_tool_call(&json_str) {
                Ok(tool_call) => return Ok(ParsedIntent::StructuredTool(tool_call)),
                Err(_) => {
                    // JSON found but invalid - try to parse as array (multiple tools)
                    if let Ok(tool_calls) = self.parse_json_tool_calls(&json_str) {
                        if tool_calls.len() > 1 {
                            return Ok(ParsedIntent::Ambiguous(tool_calls));
                        } else if tool_calls.len() == 1 {
                            return Ok(ParsedIntent::StructuredTool(tool_calls.into_iter().next().unwrap()));
                        }
                    }
                }
            }
        }

        // Fallback: treat as natural language
        Ok(ParsedIntent::NaturalLanguage(trimmed.to_string()))
    }

    /// Extract JSON from text (handles markdown code blocks, etc.)
    fn extract_json(&self, text: &str) -> Option<String> {
        // Try to find JSON object in markdown code block
        if let Some(start) = text.find("```json") {
            if let Some(end) = text[start..].find("```") {
                let json_content = &text[start + 7..start + end].trim();
                return Some(json_content.to_string());
            }
        }

        // Try to find JSON object in plain code block
        if let Some(start) = text.find("```") {
            if let Some(end) = text[start + 3..].find("```") {
                let json_content = &text[start + 3..start + 3 + end].trim();
                // Check if it looks like JSON
                if json_content.starts_with('{') || json_content.starts_with('[') {
                    return Some(json_content.to_string());
                }
            }
        }

        // Try to find JSON object directly
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                if end > start {
                    let json_content = &text[start..=end];
                    return Some(json_content.to_string());
                }
            }
        }

        None
    }

    /// Parse a single tool call from JSON
    fn parse_json_tool_call(&self, json_str: &str) -> Result<ToolCall> {
        let value: Value = serde_json::from_str(json_str)
            .map_err(|e| crate::OrchestratorError::Serialization(e))?;

        // Handle both direct object and wrapped formats
        let tool_call = if let Some(obj) = value.as_object() {
            // Direct format: {"tool": "...", "args": {...}}
            if obj.contains_key("tool") && obj.contains_key("args") {
                serde_json::from_value(value)
                    .map_err(|e| crate::OrchestratorError::Serialization(e))?
            } else {
                // Try to find nested structure
                return Err(crate::OrchestratorError::General(
                    "Invalid tool call format: missing 'tool' or 'args'".to_string(),
                ));
            }
        } else {
            return Err(crate::OrchestratorError::General(
                "Invalid tool call format: expected JSON object".to_string(),
            ));
        };

        Ok(tool_call)
    }

    /// Parse multiple tool calls from JSON array
    fn parse_json_tool_calls(&self, json_str: &str) -> Result<Vec<ToolCall>> {
        let value: Value = serde_json::from_str(json_str)
            .map_err(|e| crate::OrchestratorError::Serialization(e))?;

        match value {
            Value::Array(arr) => {
                let mut tool_calls = Vec::new();
                for item in arr {
                    let tool_call: ToolCall = serde_json::from_value(item)
                        .map_err(|e| crate::OrchestratorError::Serialization(e))?;
                    tool_calls.push(tool_call);
                }
                Ok(tool_calls)
            }
            _ => Err(crate::OrchestratorError::General(
                "Expected JSON array for multiple tool calls".to_string(),
            )),
        }
    }
}

impl Default for IntentParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_structured_tool_call() {
        let parser = IntentParser::new();
        let json = r#"{"tool": "file_read", "args": {"path": "src/main.rs"}}"#;
        
        let intent = parser.parse(json).unwrap();
        match intent {
            ParsedIntent::StructuredTool(call) => {
                assert_eq!(call.tool, "file_read");
                assert_eq!(call.args["path"], "src/main.rs");
            }
            _ => panic!("Expected StructuredTool"),
        }
    }

    #[test]
    fn test_parse_json_in_markdown() {
        let parser = IntentParser::new();
        let text = r#"Here's the tool call:
```json
{"tool": "file_read", "args": {"path": "src/main.rs"}}
```"#;
        
        let intent = parser.parse(text).unwrap();
        match intent {
            ParsedIntent::StructuredTool(call) => {
                assert_eq!(call.tool, "file_read");
            }
            _ => panic!("Expected StructuredTool"),
        }
    }

    #[test]
    fn test_parse_natural_language() {
        let parser = IntentParser::new();
        let text = "Read the main.rs file";
        
        let intent = parser.parse(text).unwrap();
        match intent {
            ParsedIntent::NaturalLanguage(_) => {}
            _ => panic!("Expected NaturalLanguage"),
        }
    }

    #[test]
    fn test_parse_multiple_tools() {
        let parser = IntentParser::new();
        let json = r#"[{"tool": "file_read", "args": {"path": "a.rs"}}, {"tool": "file_read", "args": {"path": "b.rs"}}]"#;
        
        let intent = parser.parse(json).unwrap();
        match intent {
            ParsedIntent::Ambiguous(calls) => {
                assert_eq!(calls.len(), 2);
            }
            _ => panic!("Expected Ambiguous"),
        }
    }
}

