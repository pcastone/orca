//! Result Formatter for converting ToolResponses to natural language.
//!
//! Handles:
//! - Formatting single responses
//! - Summarizing multiple responses
//! - Context window management
//! - Error formatting

use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use tooling::runtime::ToolResponse;

/// Result Formatter for converting ToolResponses to natural language
pub struct ResultFormatter {
    /// Maximum length for formatted output (for context window management)
    max_output_length: Option<usize>,
}

impl ResultFormatter {
    /// Create a new Result Formatter
    pub fn new() -> Self {
        Self {
            max_output_length: None,
        }
    }

    /// Set maximum output length (for context window management)
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_output_length = Some(max_length);
        self
    }

    /// Format a single ToolResponse for LLM consumption
    pub fn format_for_llm(&self, response: &ToolResponse) -> String {
        if !response.ok {
            return self.format_error(response);
        }

        let mut formatted = format!("Tool '{}' executed successfully", response.tool);

        if let Some(data) = &response.data {
            formatted.push_str(&format!(":\n{}", self.format_data(data)));
        }

        if !response.warnings.is_empty() {
            formatted.push_str("\nWarnings:");
            for warning in &response.warnings {
                formatted.push_str(&format!("\n  - {}", warning));
            }
        }

        formatted.push_str(&format!("\n(Duration: {}ms)", response.duration_ms));

        // Apply length limit if set
        if let Some(max_len) = self.max_output_length {
            if formatted.len() > max_len {
                formatted.truncate(max_len - 3);
                formatted.push_str("...");
            }
        }

        formatted
    }

    /// Format an error response
    fn format_error(&self, response: &ToolResponse) -> String {
        let mut formatted = format!("Tool '{}' failed", response.tool);

        if !response.errors.is_empty() {
            formatted.push_str(":\n");
            for error in &response.errors {
                formatted.push_str(&format!("  - {}\n", error));
            }
        } else {
            formatted.push_str(" (unknown error)");
        }

        formatted
    }

    /// Format response data
    fn format_data(&self, data: &Value) -> String {
        match data {
            Value::String(s) => {
                // For large strings, truncate
                if let Some(max_len) = self.max_output_length {
                    if s.len() > max_len / 2 {
                        return format!("{}... (truncated, {} chars total)", 
                            &s[..max_len / 2], s.len());
                    }
                }
                s.clone()
            }
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                if arr.is_empty() {
                    return "[]".to_string();
                }
                let mut formatted = String::from("[\n");
                for (i, item) in arr.iter().enumerate() {
                    formatted.push_str(&format!("  [{}] {}\n", i, self.format_data(item)));
                    // Limit array items shown
                    if i >= 9 && arr.len() > 10 {
                        formatted.push_str(&format!("  ... ({} more items)\n", arr.len() - 10));
                        break;
                    }
                }
                formatted.push(']');
                formatted
            }
            Value::Object(obj) => {
                if obj.is_empty() {
                    return "{}".to_string();
                }
                let mut formatted = String::from("{\n");
                for (key, value) in obj.iter() {
                    formatted.push_str(&format!("  {}: {}\n", key, self.format_data(value)));
                    // Limit object keys shown
                    if formatted.len() > 1000 {
                        formatted.push_str("  ... (more fields)\n");
                        break;
                    }
                }
                formatted.push('}');
                formatted
            }
        }
    }

    /// Summarize multiple ToolResponses
    pub fn summarize(&self, responses: &[ToolResponse]) -> String {
        if responses.is_empty() {
            return "No tool executions".to_string();
        }

        let mut summary = format!("Executed {} tool(s):\n", responses.len());

        let mut success_count = 0;
        let mut failure_count = 0;

        for (i, response) in responses.iter().enumerate() {
            if response.ok {
                success_count += 1;
                summary.push_str(&format!("\n[{}] ✓ {} - {}ms", 
                    i + 1, response.tool, response.duration_ms));
            } else {
                failure_count += 1;
                summary.push_str(&format!("\n[{}] ✗ {} - FAILED", 
                    i + 1, response.tool));
                if !response.errors.is_empty() {
                    summary.push_str(&format!(": {}", response.errors[0]));
                }
            }
        }

        summary.push_str(&format!("\n\nSummary: {} succeeded, {} failed", 
            success_count, failure_count));

        // Apply length limit
        if let Some(max_len) = self.max_output_length {
            if summary.len() > max_len {
                summary.truncate(max_len - 3);
                summary.push_str("...");
            }
        }

        summary
    }

    /// Format response data as JSON (for structured output)
    pub fn format_as_json(&self, response: &ToolResponse) -> String {
        serde_json::to_string_pretty(response)
            .unwrap_or_else(|_| format!("{:?}", response))
    }

    /// Format for human-readable console output
    pub fn format_for_console(&self, response: &ToolResponse) -> String {
        use std::fmt::Write;

        let mut output = String::new();
        
        if response.ok {
            writeln!(output, "✓ Tool: {}", response.tool).ok();
        } else {
            writeln!(output, "✗ Tool: {} (FAILED)", response.tool).ok();
        }

        if let Some(data) = &response.data {
            writeln!(output, "  Data: {}", self.format_data(data)).ok();
        }

        if !response.errors.is_empty() {
            writeln!(output, "  Errors:").ok();
            for error in &response.errors {
                writeln!(output, "    - {}", error).ok();
            }
        }

        if !response.warnings.is_empty() {
            writeln!(output, "  Warnings:").ok();
            for warning in &response.warnings {
                writeln!(output, "    - {}", warning).ok();
            }
        }

        writeln!(output, "  Duration: {}ms", response.duration_ms).ok();

        output
    }
}

impl Default for ResultFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tooling::runtime::ToolResponse;

    fn create_success_response() -> ToolResponse {
        ToolResponse {
            ok: true,
            tool: "file_read".to_string(),
            request_id: "req-123".to_string(),
            duration_ms: 42,
            data: Some(json!({"content": "Hello, world!"})),
            errors: vec![],
            warnings: vec![],
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64,
        }
    }

    fn create_error_response() -> ToolResponse {
        ToolResponse {
            ok: false,
            tool: "file_read".to_string(),
            request_id: "req-123".to_string(),
            duration_ms: 10,
            data: None,
            errors: vec!["File not found".to_string()],
            warnings: vec![],
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64,
        }
    }

    #[test]
    fn test_format_success() {
        let formatter = ResultFormatter::new();
        let response = create_success_response();
        let formatted = formatter.format_for_llm(&response);

        assert!(formatted.contains("file_read"));
        assert!(formatted.contains("successfully"));
    }

    #[test]
    fn test_format_error() {
        let formatter = ResultFormatter::new();
        let response = create_error_response();
        let formatted = formatter.format_for_llm(&response);

        assert!(formatted.contains("failed"));
        assert!(formatted.contains("File not found"));
    }

    #[test]
    fn test_summarize() {
        let formatter = ResultFormatter::new();
        let responses = vec![create_success_response(), create_error_response()];
        let summary = formatter.summarize(&responses);

        assert!(summary.contains("2 tool(s)"));
        assert!(summary.contains("1 succeeded"));
    }

    #[test]
    fn test_max_length() {
        let formatter = ResultFormatter::new().with_max_length(50);
        let response = create_success_response();
        let formatted = formatter.format_for_llm(&response);

        assert!(formatted.len() <= 53); // 50 + "..."
    }
}

