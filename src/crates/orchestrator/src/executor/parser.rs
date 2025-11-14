//! LLM Response Parser
//!
//! This module provides robust parsing of LLM responses into structured task results.
//! It handles various response formats (JSON, markdown, plain text) and extracts
//! task status and data from LLM output.

use crate::{OrchestratorError, Result, TaskStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, warn};

/// Structured result parsed from LLM response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedResult {
    /// Task status determined from the response
    pub status: TaskStatus,

    /// Result data (if successful)
    pub result: Option<String>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Additional data extracted from response
    pub metadata: serde_json::Map<String, Value>,
}

impl ParsedResult {
    /// Create a new parsed result
    pub fn new(status: TaskStatus) -> Self {
        Self {
            status,
            result: None,
            error: None,
            metadata: serde_json::Map::new(),
        }
    }

    /// Set result data
    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(result.into());
        self
    }

    /// Set error message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Add metadata field
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Parser for LLM responses
pub struct ResponseParser {
    /// Whether to require strict JSON format
    strict_mode: bool,

    /// Whether to allow partial results
    allow_partial: bool,
}

impl Default for ResponseParser {
    fn default() -> Self {
        Self {
            strict_mode: false,
            allow_partial: true,
        }
    }
}

impl ResponseParser {
    /// Create a new response parser
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable strict mode
    ///
    /// In strict mode, only valid JSON responses are accepted.
    /// Non-JSON responses will result in an error.
    pub fn with_strict_mode(mut self, enabled: bool) -> Self {
        self.strict_mode = enabled;
        self
    }

    /// Enable or disable partial results
    ///
    /// When enabled, incomplete responses are accepted and marked as pending.
    /// When disabled, incomplete responses result in an error.
    pub fn with_allow_partial(mut self, enabled: bool) -> Self {
        self.allow_partial = enabled;
        self
    }

    /// Parse an LLM response into a structured result
    pub fn parse(&self, response: &str) -> Result<ParsedResult> {
        debug!("Parsing LLM response: {}", response);

        // Try to extract and parse JSON first
        if let Some(json_str) = Self::extract_json(response) {
            match self.parse_json_response(json_str) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if self.strict_mode {
                        return Err(e);
                    }
                    warn!("JSON parsing failed, falling back to text parsing: {}", e);
                }
            }
        }

        // Fallback to text-based parsing
        if self.strict_mode {
            return Err(OrchestratorError::General(
                "Strict mode enabled but no valid JSON found in response".to_string(),
            ));
        }

        self.parse_text_response(response)
    }

    /// Parse a JSON response
    fn parse_json_response(&self, json_str: &str) -> Result<ParsedResult> {
        let value: Value = serde_json::from_str(json_str).map_err(|e| {
            OrchestratorError::General(format!("Failed to parse JSON: {}", e))
        })?;

        let obj = value.as_object().ok_or_else(|| {
            OrchestratorError::General("JSON response is not an object".to_string())
        })?;

        // Extract status
        let status = self.parse_status(obj)?;

        // Build parsed result
        let mut parsed = ParsedResult::new(status);

        // Extract result field
        if let Some(result_value) = obj.get("result") {
            if let Some(result_str) = result_value.as_str() {
                parsed.result = Some(result_str.to_string());
            } else {
                // Convert non-string results to JSON string
                parsed.result = Some(serde_json::to_string(result_value).unwrap_or_default());
            }
        }

        // Extract error field
        if let Some(error_value) = obj.get("error") {
            if let Some(error_str) = error_value.as_str() {
                parsed.error = Some(error_str.to_string());
            }
        }

        // Extract needs_input field (for backward compatibility)
        if let Some(needs_input) = obj.get("needs_input") {
            if let Some(needs_input_str) = needs_input.as_str() {
                parsed.metadata.insert(
                    "needs_input".to_string(),
                    Value::String(needs_input_str.to_string()),
                );
            }
        }

        // Extract all other fields as metadata
        for (key, value) in obj {
            if key != "status" && key != "result" && key != "error" && key != "needs_input" {
                parsed.metadata.insert(key.clone(), value.clone());
            }
        }

        // Validate the parsed result
        self.validate_parsed_result(&parsed)?;

        Ok(parsed)
    }

    /// Parse a text response (fallback when JSON is not available)
    fn parse_text_response(&self, text: &str) -> Result<ParsedResult> {
        let text_lower = text.to_lowercase();

        // Check for explicit failure indicators
        if text_lower.contains("failed")
            || text_lower.contains("error")
            || text_lower.contains("failure")
        {
            return Ok(ParsedResult::new(TaskStatus::Failed).with_error(text));
        }

        // Check for completion indicators
        if text_lower.contains("completed")
            || text_lower.contains("success")
            || text_lower.contains("done")
        {
            return Ok(ParsedResult::new(TaskStatus::Completed).with_result(text));
        }

        // Check for pending/needs input indicators
        if text_lower.contains("need") || text_lower.contains("require") {
            return Ok(ParsedResult::new(TaskStatus::Pending)
                .with_result(text)
                .with_metadata("reason".to_string(), Value::String("needs_input".to_string())));
        }

        // If we got a non-empty response, consider it completed
        if !text.trim().is_empty() {
            if self.allow_partial {
                return Ok(ParsedResult::new(TaskStatus::Completed).with_result(text));
            }
        }

        // Empty response or no clear status
        Err(OrchestratorError::General(
            "Could not determine task status from response".to_string(),
        ))
    }

    /// Parse status from JSON object
    fn parse_status(&self, obj: &serde_json::Map<String, Value>) -> Result<TaskStatus> {
        let status_value = obj.get("status").ok_or_else(|| {
            OrchestratorError::General("Missing 'status' field in JSON response".to_string())
        })?;

        let status_str = status_value.as_str().ok_or_else(|| {
            OrchestratorError::General("'status' field is not a string".to_string())
        })?;

        match status_str.to_lowercase().as_str() {
            "completed" | "complete" | "success" | "done" => Ok(TaskStatus::Completed),
            "failed" | "fail" | "error" | "failure" => Ok(TaskStatus::Failed),
            "pending" | "needs_input" | "waiting" | "paused" => Ok(TaskStatus::Pending),
            "running" | "in_progress" => Ok(TaskStatus::Running),
            "cancelled" | "canceled" => Ok(TaskStatus::Cancelled),
            _ => Err(OrchestratorError::General(format!(
                "Unknown status: {}",
                status_str
            ))),
        }
    }

    /// Validate a parsed result
    fn validate_parsed_result(&self, result: &ParsedResult) -> Result<()> {
        match result.status {
            TaskStatus::Completed => {
                if result.result.is_none() && !self.allow_partial {
                    return Err(OrchestratorError::General(
                        "Completed status requires a result".to_string(),
                    ));
                }
            }
            TaskStatus::Failed => {
                if result.error.is_none() && !self.allow_partial {
                    return Err(OrchestratorError::General(
                        "Failed status requires an error message".to_string(),
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Extract JSON from a text response
    ///
    /// Looks for ```json...``` code blocks or raw JSON objects
    fn extract_json(text: &str) -> Option<&str> {
        // Try to find ```json ... ``` block
        if let Some(start) = text.find("```json") {
            let content = &text[start + 7..];
            if let Some(end) = content.find("```") {
                return Some(content[..end].trim());
            }
        }

        // Try to find ```JSON ... ``` block (uppercase)
        if let Some(start) = text.find("```JSON") {
            let content = &text[start + 7..];
            if let Some(end) = content.find("```") {
                return Some(content[..end].trim());
            }
        }

        // Try to find raw JSON object
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                if end > start {
                    return Some(text[start..=end].trim());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_completed() {
        let parser = ResponseParser::new();
        let response = r#"{"status": "completed", "result": "Task executed successfully"}"#;

        let parsed = parser.parse(response).unwrap();
        assert_eq!(parsed.status, TaskStatus::Completed);
        assert_eq!(parsed.result, Some("Task executed successfully".to_string()));
        assert!(parsed.error.is_none());
    }

    #[test]
    fn test_parse_json_failed() {
        let parser = ResponseParser::new();
        let response = r#"{"status": "failed", "error": "Task execution failed"}"#;

        let parsed = parser.parse(response).unwrap();
        assert_eq!(parsed.status, TaskStatus::Failed);
        assert_eq!(parsed.error, Some("Task execution failed".to_string()));
    }

    #[test]
    fn test_parse_json_with_code_block() {
        let parser = ResponseParser::new();
        let response = r#"Here's the result:
```json
{"status": "completed", "result": "Done"}
```
That's it!"#;

        let parsed = parser.parse(response).unwrap();
        assert_eq!(parsed.status, TaskStatus::Completed);
        assert_eq!(parsed.result, Some("Done".to_string()));
    }

    #[test]
    fn test_parse_json_needs_input() {
        let parser = ResponseParser::new();
        let response = r#"{"status": "needs_input", "needs_input": "Please provide API key"}"#;

        let parsed = parser.parse(response).unwrap();
        assert_eq!(parsed.status, TaskStatus::Pending);
        assert_eq!(
            parsed.metadata.get("needs_input"),
            Some(&Value::String("Please provide API key".to_string()))
        );
    }

    #[test]
    fn test_parse_json_with_metadata() {
        let parser = ResponseParser::new();
        let response = r#"{
            "status": "completed",
            "result": "Task done",
            "execution_time_ms": 150,
            "tokens_used": 42
        }"#;

        let parsed = parser.parse(response).unwrap();
        assert_eq!(parsed.status, TaskStatus::Completed);
        assert_eq!(parsed.metadata.get("execution_time_ms"), Some(&Value::from(150)));
        assert_eq!(parsed.metadata.get("tokens_used"), Some(&Value::from(42)));
    }

    #[test]
    fn test_parse_text_completed() {
        let parser = ResponseParser::new();
        let response = "The task completed successfully!";

        let parsed = parser.parse(response).unwrap();
        assert_eq!(parsed.status, TaskStatus::Completed);
        assert!(parsed.result.is_some());
    }

    #[test]
    fn test_parse_text_failed() {
        let parser = ResponseParser::new();
        let response = "The task failed due to an error.";

        let parsed = parser.parse(response).unwrap();
        assert_eq!(parsed.status, TaskStatus::Failed);
        assert!(parsed.error.is_some());
    }

    #[test]
    fn test_parse_text_needs_input() {
        let parser = ResponseParser::new();
        let response = "I need more information to proceed.";

        let parsed = parser.parse(response).unwrap();
        assert_eq!(parsed.status, TaskStatus::Pending);
    }

    #[test]
    fn test_parse_empty_response() {
        let parser = ResponseParser::new();
        let response = "";

        assert!(parser.parse(response).is_err());
    }

    #[test]
    fn test_strict_mode_requires_json() {
        let parser = ResponseParser::new().with_strict_mode(true);
        let response = "This is just plain text without JSON";

        assert!(parser.parse(response).is_err());
    }

    #[test]
    fn test_strict_mode_accepts_json() {
        let parser = ResponseParser::new().with_strict_mode(true);
        let response = r#"{"status": "completed", "result": "Success"}"#;

        let parsed = parser.parse(response).unwrap();
        assert_eq!(parsed.status, TaskStatus::Completed);
    }

    #[test]
    fn test_validation_completed_without_result() {
        let parser = ResponseParser::new().with_allow_partial(false);
        let response = r#"{"status": "completed"}"#;

        assert!(parser.parse(response).is_err());
    }

    #[test]
    fn test_validation_failed_without_error() {
        let parser = ResponseParser::new().with_allow_partial(false);
        let response = r#"{"status": "failed"}"#;

        assert!(parser.parse(response).is_err());
    }

    #[test]
    fn test_allow_partial() {
        let parser = ResponseParser::new().with_allow_partial(true);

        let response = r#"{"status": "completed"}"#;
        let parsed = parser.parse(response).unwrap();
        assert_eq!(parsed.status, TaskStatus::Completed);
        assert!(parsed.result.is_none());
    }

    #[test]
    fn test_extract_json_code_block() {
        let text = r#"Here's the result:
```json
{"status": "completed"}
```"#;

        let json = ResponseParser::extract_json(text).unwrap();
        assert!(json.contains("completed"));
    }

    #[test]
    fn test_extract_json_raw() {
        let text = r#"The result is {"status": "completed"} as you can see."#;

        let json = ResponseParser::extract_json(text).unwrap();
        assert!(json.contains("completed"));
    }

    #[test]
    fn test_extract_json_none() {
        let text = "No JSON here!";
        assert!(ResponseParser::extract_json(text).is_none());
    }

    #[test]
    fn test_status_aliases() {
        let parser = ResponseParser::new();

        // Test various status aliases
        let statuses = vec![
            ("completed", TaskStatus::Completed),
            ("complete", TaskStatus::Completed),
            ("success", TaskStatus::Completed),
            ("done", TaskStatus::Completed),
            ("failed", TaskStatus::Failed),
            ("fail", TaskStatus::Failed),
            ("error", TaskStatus::Failed),
            ("pending", TaskStatus::Pending),
            ("needs_input", TaskStatus::Pending),
            ("running", TaskStatus::Running),
            ("cancelled", TaskStatus::Cancelled),
        ];

        for (status_str, expected) in statuses {
            let response = format!(r#"{{"status": "{}"}}"#, status_str);
            let parsed = parser.parse(&response).unwrap();
            assert_eq!(parsed.status, expected, "Failed for status: {}", status_str);
        }
    }

    #[test]
    fn test_unknown_status() {
        let parser = ResponseParser::new();
        let response = r#"{"status": "unknown_status"}"#;

        assert!(parser.parse(response).is_err());
    }

    #[test]
    fn test_parsed_result_builder() {
        let result = ParsedResult::new(TaskStatus::Completed)
            .with_result("Test result")
            .with_error("Test error")
            .with_metadata("key1", Value::String("value1".to_string()))
            .with_metadata("key2", Value::from(42));

        assert_eq!(result.status, TaskStatus::Completed);
        assert_eq!(result.result, Some("Test result".to_string()));
        assert_eq!(result.error, Some("Test error".to_string()));
        assert_eq!(result.metadata.len(), 2);
    }
}
