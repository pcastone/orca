//! Expression evaluation engine for workflow conditions
//!
//! Supports expressions like:
//! - `result.success` - field access
//! - `result.status == 'pending'` - string comparison
//! - `result.score > 0.8` - numeric comparison
//! - `result.data.nested.field` - nested field access
//! - `!result.failed` - boolean negation
//! - `result.a && result.b` - logical AND
//! - `result.a || result.b` - logical OR

use crate::{OrchestratorError, Result};
use serde_json::Value;

/// Expression evaluator for workflow conditions
pub struct ExpressionEvaluator;

impl Default for ExpressionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpressionEvaluator {
    /// Create a new expression evaluator
    pub fn new() -> Self {
        Self
    }

    /// Evaluate an expression against a context value
    ///
    /// The context typically comes from the previous step's result.
    /// Expressions can reference fields via dot notation (e.g., `result.success`)
    pub fn evaluate(&self, expr: &str, context: &Value) -> Result<bool> {
        let expr = expr.trim();

        // Handle empty expression
        if expr.is_empty() {
            return Ok(true);
        }

        // Parse and evaluate the expression
        let mut parser = ExpressionParser::new(expr, context);
        parser.parse_or_expression()
    }

    /// Evaluate an expression with a named context
    ///
    /// Allows expressions like `result.success` where `result` is the context name
    pub fn evaluate_with_name(&self, expr: &str, name: &str, context: &Value) -> Result<bool> {
        let expr = expr.trim();

        // Handle empty expression
        if expr.is_empty() {
            return Ok(true);
        }

        // Create a wrapper context with the name
        let wrapper = serde_json::json!({ name: context });
        self.evaluate(expr, &wrapper)
    }
}

/// Simple expression parser using recursive descent
struct ExpressionParser<'a> {
    expr: &'a str,
    pos: usize,
    context: &'a Value,
}

impl<'a> ExpressionParser<'a> {
    fn new(expr: &'a str, context: &'a Value) -> Self {
        Self { expr, pos: 0, context }
    }

    fn peek(&self) -> Option<char> {
        self.expr[self.pos..].chars().next()
    }

    fn peek_str(&self, len: usize) -> &str {
        let end = (self.pos + len).min(self.expr.len());
        &self.expr[self.pos..end]
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn consume(&mut self, expected: &str) -> bool {
        self.skip_whitespace();
        if self.expr[self.pos..].starts_with(expected) {
            self.pos += expected.len();
            true
        } else {
            false
        }
    }

    /// Parse OR expression: expr || expr
    fn parse_or_expression(&mut self) -> Result<bool> {
        let mut left = self.parse_and_expression()?;

        while self.consume("||") {
            let right = self.parse_and_expression()?;
            left = left || right;
        }

        Ok(left)
    }

    /// Parse AND expression: expr && expr
    fn parse_and_expression(&mut self) -> Result<bool> {
        let mut left = self.parse_not_expression()?;

        while self.consume("&&") {
            let right = self.parse_not_expression()?;
            left = left && right;
        }

        Ok(left)
    }

    /// Parse NOT expression: !expr
    fn parse_not_expression(&mut self) -> Result<bool> {
        self.skip_whitespace();
        if self.consume("!") {
            let value = self.parse_comparison()?;
            Ok(!value)
        } else {
            self.parse_comparison()
        }
    }

    /// Parse comparison: value op value
    fn parse_comparison(&mut self) -> Result<bool> {
        self.skip_whitespace();

        // Handle parentheses
        if self.consume("(") {
            let result = self.parse_or_expression()?;
            self.consume(")");
            return Ok(result);
        }

        // Parse left operand
        let left = self.parse_value()?;

        self.skip_whitespace();

        // Check for comparison operators
        if self.consume("==") {
            let right = self.parse_value()?;
            return Ok(values_equal(&left, &right));
        } else if self.consume("!=") {
            let right = self.parse_value()?;
            return Ok(!values_equal(&left, &right));
        } else if self.consume(">=") {
            let right = self.parse_value()?;
            return compare_numbers(&left, &right, |a, b| a >= b);
        } else if self.consume("<=") {
            let right = self.parse_value()?;
            return compare_numbers(&left, &right, |a, b| a <= b);
        } else if self.consume(">") {
            let right = self.parse_value()?;
            return compare_numbers(&left, &right, |a, b| a > b);
        } else if self.consume("<") {
            let right = self.parse_value()?;
            return compare_numbers(&left, &right, |a, b| a < b);
        }

        // No comparison operator - treat as boolean
        Ok(value_to_bool(&left))
    }

    /// Parse a value: field path, string literal, number, or boolean
    fn parse_value(&mut self) -> Result<Value> {
        self.skip_whitespace();

        // String literal
        if self.consume("'") || self.consume("\"") {
            return self.parse_string_literal();
        }

        // Number
        if let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '-' {
                return self.parse_number();
            }
        }

        // Boolean literals
        if self.consume("true") {
            return Ok(Value::Bool(true));
        }
        if self.consume("false") {
            return Ok(Value::Bool(false));
        }
        if self.consume("null") {
            return Ok(Value::Null);
        }

        // Field path (e.g., result.success, result.data.field)
        self.parse_field_path()
    }

    /// Parse a string literal
    fn parse_string_literal(&mut self) -> Result<Value> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == '\'' || c == '"' {
                let value = &self.expr[start..self.pos];
                self.advance(); // consume closing quote
                return Ok(Value::String(value.to_string()));
            }
            self.advance();
        }
        Err(OrchestratorError::General("Unterminated string literal".to_string()))
    }

    /// Parse a number
    fn parse_number(&mut self) -> Result<Value> {
        let start = self.pos;
        let mut has_dot = false;

        // Handle negative sign
        if self.peek() == Some('-') {
            self.advance();
        }

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else if c == '.' && !has_dot {
                has_dot = true;
                self.advance();
            } else {
                break;
            }
        }

        let num_str = &self.expr[start..self.pos];
        if has_dot {
            let f: f64 = num_str.parse().map_err(|e| {
                OrchestratorError::General(format!("Invalid number '{}': {}", num_str, e))
            })?;
            Ok(serde_json::json!(f))
        } else {
            let i: i64 = num_str.parse().map_err(|e| {
                OrchestratorError::General(format!("Invalid number '{}': {}", num_str, e))
            })?;
            Ok(serde_json::json!(i))
        }
    }

    /// Parse a field path (e.g., result.success, result.data.nested)
    fn parse_field_path(&mut self) -> Result<Value> {
        let mut path_parts = Vec::new();
        let start = self.pos;

        // Parse first identifier
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        if self.pos == start {
            return Err(OrchestratorError::General(format!(
                "Expected identifier at position {}",
                self.pos
            )));
        }

        path_parts.push(&self.expr[start..self.pos]);

        // Parse additional path segments
        while self.consume(".") {
            let seg_start = self.pos;
            while let Some(c) = self.peek() {
                if c.is_alphanumeric() || c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.pos == seg_start {
                return Err(OrchestratorError::General(
                    "Expected identifier after '.'".to_string(),
                ));
            }
            path_parts.push(&self.expr[seg_start..self.pos]);
        }

        // Navigate the context to get the value
        let mut current = self.context;
        for part in &path_parts {
            match current {
                Value::Object(map) => {
                    current = map.get(*part).unwrap_or(&Value::Null);
                }
                Value::Array(arr) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        current = arr.get(idx).unwrap_or(&Value::Null);
                    } else {
                        return Ok(Value::Null);
                    }
                }
                _ => return Ok(Value::Null),
            }
        }

        Ok(current.clone())
    }
}

/// Compare two JSON values for equality
fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => {
            a.as_f64().unwrap_or(f64::NAN) == b.as_f64().unwrap_or(f64::NAN)
        }
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => left == right,
    }
}

/// Compare two numeric values
fn compare_numbers<F>(left: &Value, right: &Value, op: F) -> Result<bool>
where
    F: Fn(f64, f64) -> bool,
{
    let left_num = value_to_number(left).ok_or_else(|| {
        OrchestratorError::General(format!("Cannot compare non-numeric value: {:?}", left))
    })?;
    let right_num = value_to_number(right).ok_or_else(|| {
        OrchestratorError::General(format!("Cannot compare non-numeric value: {:?}", right))
    })?;
    Ok(op(left_num, right_num))
}

/// Convert a JSON value to a number
fn value_to_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        Value::Bool(true) => Some(1.0),
        Value::Bool(false) => Some(0.0),
        _ => None,
    }
}

/// Convert a JSON value to a boolean
fn value_to_bool(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty() && s != "false" && s != "0",
        Value::Array(arr) => !arr.is_empty(),
        Value::Object(obj) => !obj.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_field_access() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "result": { "success": true } });

        assert!(evaluator.evaluate("result.success", &context).unwrap());
    }

    #[test]
    fn test_field_access_false() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "result": { "success": false } });

        assert!(!evaluator.evaluate("result.success", &context).unwrap());
    }

    #[test]
    fn test_string_comparison() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "result": { "status": "pending" } });

        assert!(evaluator.evaluate("result.status == 'pending'", &context).unwrap());
        assert!(!evaluator.evaluate("result.status == 'completed'", &context).unwrap());
    }

    #[test]
    fn test_numeric_comparison() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "result": { "score": 0.85 } });

        assert!(evaluator.evaluate("result.score > 0.8", &context).unwrap());
        assert!(!evaluator.evaluate("result.score > 0.9", &context).unwrap());
        assert!(evaluator.evaluate("result.score >= 0.85", &context).unwrap());
        assert!(evaluator.evaluate("result.score < 0.9", &context).unwrap());
        assert!(evaluator.evaluate("result.score <= 0.85", &context).unwrap());
    }

    #[test]
    fn test_not_expression() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "result": { "failed": false } });

        assert!(evaluator.evaluate("!result.failed", &context).unwrap());
    }

    #[test]
    fn test_and_expression() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({
            "result": {
                "success": true,
                "score": 0.9
            }
        });

        assert!(evaluator.evaluate("result.success && result.score > 0.8", &context).unwrap());
        assert!(!evaluator.evaluate("result.success && result.score > 0.95", &context).unwrap());
    }

    #[test]
    fn test_or_expression() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({
            "result": {
                "success": false,
                "score": 0.9
            }
        });

        assert!(evaluator.evaluate("result.success || result.score > 0.8", &context).unwrap());
        assert!(!evaluator.evaluate("result.success || result.score > 0.95", &context).unwrap());
    }

    #[test]
    fn test_nested_field_access() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({
            "result": {
                "data": {
                    "user": {
                        "verified": true
                    }
                }
            }
        });

        assert!(evaluator.evaluate("result.data.user.verified", &context).unwrap());
    }

    #[test]
    fn test_not_equal() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "result": { "status": "pending" } });

        assert!(evaluator.evaluate("result.status != 'completed'", &context).unwrap());
        assert!(!evaluator.evaluate("result.status != 'pending'", &context).unwrap());
    }

    #[test]
    fn test_parentheses() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({
            "a": true,
            "b": false,
            "c": true
        });

        // Without parens: a || b && c = a || (b && c) = true || false = true
        // With parens: (a || b) && c = true && true = true
        assert!(evaluator.evaluate("(a || b) && c", &context).unwrap());
    }

    #[test]
    fn test_evaluate_with_name() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "success": true });

        assert!(evaluator.evaluate_with_name("result.success", "result", &context).unwrap());
    }

    #[test]
    fn test_missing_field_is_falsy() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "result": {} });

        assert!(!evaluator.evaluate("result.nonexistent", &context).unwrap());
    }

    #[test]
    fn test_null_is_falsy() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "result": { "value": null } });

        assert!(!evaluator.evaluate("result.value", &context).unwrap());
    }

    #[test]
    fn test_empty_string_is_falsy() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "result": { "name": "" } });

        assert!(!evaluator.evaluate("result.name", &context).unwrap());
    }

    #[test]
    fn test_non_empty_string_is_truthy() {
        let evaluator = ExpressionEvaluator::new();
        let context = serde_json::json!({ "result": { "name": "test" } });

        assert!(evaluator.evaluate("result.name", &context).unwrap());
    }
}
