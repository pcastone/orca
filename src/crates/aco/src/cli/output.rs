//! Output formatting for CLI commands
//!
//! Provides formatters for JSON, table, and plain text output.

use serde_json::{json, Value};
use super::OutputFormat;

/// Output formatter trait
pub trait OutputFormatter {
    /// Format data as output
    fn format(&self, data: &Value) -> String;
}

/// JSON formatter
pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
    fn format(&self, data: &Value) -> String {
        match serde_json::to_string_pretty(data) {
            Ok(s) => s,
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}

/// Table formatter
pub struct TableFormatter;

impl OutputFormatter for TableFormatter {
    fn format(&self, data: &Value) -> String {
        match data {
            Value::Array(items) => {
                if items.is_empty() {
                    return "No items to display".to_string();
                }

                format_table(items)
            }
            Value::Object(_) => {
                format_object_as_table(data)
            }
            _ => data.to_string(),
        }
    }
}

/// Plain text formatter
pub struct PlainFormatter;

impl OutputFormatter for PlainFormatter {
    fn format(&self, data: &Value) -> String {
        match data {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(items) => {
                items
                    .iter()
                    .filter_map(|item| match item {
                        Value::String(s) => Some(s.clone()),
                        _ => Some(item.to_string()),
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Value::Object(obj) => {
                obj.iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }
}

/// Get appropriate formatter based on format type
pub fn get_formatter(format: OutputFormat) -> Box<dyn OutputFormatter> {
    match format {
        OutputFormat::Json => Box::new(JsonFormatter),
        OutputFormat::Table => Box::new(TableFormatter),
        OutputFormat::Plain => Box::new(PlainFormatter),
    }
}

/// Format an array of objects as a table
fn format_table(items: &[Value]) -> String {
    if items.is_empty() {
        return "No items to display".to_string();
    }

    let mut output = String::new();

    // Extract column names from first object
    let columns = if let Some(Value::Object(first)) = items.first() {
        first.keys().cloned().collect::<Vec<_>>()
    } else {
        return format!("{:?}", items);
    };

    // Header
    let header = columns
        .iter()
        .map(|col| format!("{:<20}", col))
        .collect::<Vec<_>>()
        .join("|");
    output.push_str(&header);
    output.push('\n');

    // Separator
    let separator = (0..columns.len())
        .map(|_| "-".repeat(20))
        .collect::<Vec<_>>()
        .join("+");
    output.push_str(&separator);
    output.push('\n');

    // Rows
    for item in items {
        if let Value::Object(obj) = item {
            let row = columns
                .iter()
                .map(|col| {
                    let value = obj.get(col).map(|v| v.to_string()).unwrap_or_default();
                    format!("{:<20}", truncate_string(&value, 18))
                })
                .collect::<Vec<_>>()
                .join("|");
            output.push_str(&row);
            output.push('\n');
        }
    }

    output
}

/// Format a single object as a table
fn format_object_as_table(obj: &Value) -> String {
    if let Value::Object(map) = obj {
        let mut output = String::new();

        for (key, value) in map {
            output.push_str(&format!("{:<20}: {}\n", key, value));
        }

        output
    } else {
        obj.to_string()
    }
}

/// Truncate string to specified length
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}

/// Build error response
pub fn error_response(message: &str) -> Value {
    json!({
        "error": message,
        "status": "error"
    })
}

/// Build success response
pub fn success_response(data: Value) -> Value {
    json!({
        "data": data,
        "status": "success"
    })
}

/// Build list response
pub fn list_response(items: Vec<Value>, total: u32) -> Value {
    json!({
        "items": items,
        "total": total,
        "status": "success"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_formatter() {
        let formatter = JsonFormatter;
        let data = json!({"name": "test", "value": 42});
        let output = formatter.format(&data);
        assert!(output.contains("name"));
        assert!(output.contains("test"));
    }

    #[test]
    fn test_plain_formatter_string() {
        let formatter = PlainFormatter;
        let data = json!("hello");
        let output = formatter.format(&data);
        assert_eq!(output, "hello");
    }

    #[test]
    fn test_plain_formatter_number() {
        let formatter = PlainFormatter;
        let data = json!(42);
        let output = formatter.format(&data);
        assert_eq!(output, "42");
    }

    #[test]
    fn test_plain_formatter_object() {
        let formatter = PlainFormatter;
        let data = json!({"key": "value"});
        let output = formatter.format(&data);
        assert!(output.contains("key"));
        assert!(output.contains("value"));
    }

    #[test]
    fn test_table_formatter_empty_array() {
        let formatter = TableFormatter;
        let data = json!([]);
        let output = formatter.format(&data);
        assert_eq!(output, "No items to display");
    }

    #[test]
    fn test_table_formatter_array_of_objects() {
        let formatter = TableFormatter;
        let data = json!([
            {"id": "1", "name": "task1"},
            {"id": "2", "name": "task2"}
        ]);
        let output = formatter.format(&data);
        assert!(output.contains("id"));
        assert!(output.contains("name"));
        assert!(output.contains("task1"));
    }

    #[test]
    fn test_get_formatter_json() {
        let formatter = get_formatter(OutputFormat::Json);
        let data = json!({"test": "data"});
        let output = formatter.format(&data);
        assert!(output.contains("test"));
    }

    #[test]
    fn test_get_formatter_table() {
        let formatter = get_formatter(OutputFormat::Table);
        let data = json!([{"col": "value"}]);
        let output = formatter.format(&data);
        assert!(output.contains("col"));
    }

    #[test]
    fn test_get_formatter_plain() {
        let formatter = get_formatter(OutputFormat::Plain);
        let data = json!("hello");
        let output = formatter.format(&data);
        assert_eq!(output, "hello");
    }

    #[test]
    fn test_error_response() {
        let response = error_response("test error");
        assert_eq!(response["error"], "test error");
        assert_eq!(response["status"], "error");
    }

    #[test]
    fn test_success_response() {
        let data = json!({"result": "success"});
        let response = success_response(data.clone());
        assert_eq!(response["data"], data);
        assert_eq!(response["status"], "success");
    }

    #[test]
    fn test_list_response() {
        let items = vec![json!({"id": "1"}), json!({"id": "2"})];
        let response = list_response(items, 2);
        assert_eq!(response["total"], 2);
        assert_eq!(response["status"], "success");
    }

    #[test]
    fn test_truncate_string() {
        let result = truncate_string("hello world", 5);
        assert_eq!(result.len(), 5);
        assert!(result.ends_with("..."));

        let result = truncate_string("hi", 5);
        assert_eq!(result, "hi");
    }
}
