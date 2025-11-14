//! Rule evaluation engine for routing decisions
//!
//! Evaluates routing rules to determine which patterns should be executed
//! based on input text, context, and conditions.

use crate::config::{ConditionCheck, RouteRule, RuleCondition};
use crate::{OrchestratorError, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// Context for rule evaluation
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Input text to match against
    pub input_text: String,
    /// Context values for lookups
    pub context: HashMap<String, Value>,
}

impl EvaluationContext {
    /// Create a new evaluation context
    pub fn new(input_text: impl Into<String>) -> Self {
        Self {
            input_text: input_text.into(),
            context: HashMap::new(),
        }
    }

    /// Add a context value
    pub fn with_context(mut self, key: impl Into<String>, value: Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    /// Set multiple context values
    pub fn with_context_map(mut self, context: HashMap<String, Value>) -> Self {
        self.context.extend(context);
        self
    }
}

/// Rule evaluator for routing decisions
pub struct RuleEvaluator;

impl RuleEvaluator {
    /// Create a new rule evaluator
    pub fn new() -> Self {
        Self
    }

    /// Evaluate a routing rule against the context
    ///
    /// Returns `true` if the rule matches, `false` otherwise
    pub fn evaluate_rule(&self, rule: &RouteRule, context: &EvaluationContext) -> Result<bool> {
        self.evaluate_condition(&rule.when, context)
    }

    /// Evaluate a rule condition
    fn evaluate_condition(
        &self,
        condition: &RuleCondition,
        context: &EvaluationContext,
    ) -> Result<bool> {
        match condition {
            RuleCondition::Single(check) => self.evaluate_check(check, context),
            RuleCondition::All { all } => {
                // All conditions must match (AND)
                for check in all {
                    if !self.evaluate_check(check, context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            RuleCondition::Any { any } => {
                // Any condition must match (OR)
                for check in any {
                    if self.evaluate_check(check, context)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            RuleCondition::Not { not } => {
                // No conditions should match (NOT)
                for check in not {
                    if self.evaluate_check(check, context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }

    /// Evaluate an individual condition check
    fn evaluate_check(
        &self,
        check: &ConditionCheck,
        context: &EvaluationContext,
    ) -> Result<bool> {
        match check {
            ConditionCheck::Text { pattern } => self.evaluate_text_pattern(pattern, context),
            ConditionCheck::ContextHas { key } => Ok(context.context.contains_key(key)),
            ConditionCheck::ContextEquals { key, value } => {
                self.evaluate_context_equals(key, value, context)
            }
            ConditionCheck::Contains { keywords } => {
                self.evaluate_contains(keywords, context)
            }
            ConditionCheck::Expression { expr } => {
                self.evaluate_expression(expr, context)
            }
        }
    }

    /// Evaluate a text pattern (regex)
    fn evaluate_text_pattern(&self, pattern: &str, context: &EvaluationContext) -> Result<bool> {
        // Parse regex pattern (support JavaScript-style /pattern/flags)
        let (regex_pattern, case_insensitive) = self.parse_regex_pattern(pattern);

        let regex = if case_insensitive {
            Regex::new(&format!("(?i){}", regex_pattern))
        } else {
            Regex::new(&regex_pattern)
        }
        .map_err(|e| {
            OrchestratorError::General(format!("Invalid regex pattern '{}': {}", pattern, e))
        })?;

        Ok(regex.is_match(&context.input_text))
    }

    /// Parse regex pattern, extracting flags
    fn parse_regex_pattern(&self, pattern: &str) -> (String, bool) {
        // Support JavaScript-style regex: /pattern/flags
        if pattern.starts_with('/') {
            if let Some(end_pos) = pattern.rfind('/') {
                if end_pos > 0 {
                    let regex_pattern = pattern[1..end_pos].to_string();
                    let flags = &pattern[end_pos + 1..];
                    let case_insensitive = flags.contains('i');
                    return (regex_pattern, case_insensitive);
                }
            }
        }

        // Default: treat as plain regex pattern
        (pattern.to_string(), false)
    }

    /// Evaluate context equals check
    fn evaluate_context_equals(
        &self,
        key: &str,
        expected: &Value,
        context: &EvaluationContext,
    ) -> Result<bool> {
        if let Some(actual) = context.context.get(key) {
            Ok(actual == expected)
        } else {
            Ok(false)
        }
    }

    /// Evaluate contains check (case-insensitive keyword matching)
    fn evaluate_contains(&self, keywords: &[String], context: &EvaluationContext) -> Result<bool> {
        let input_lower = context.input_text.to_lowercase();
        for keyword in keywords {
            if input_lower.contains(&keyword.to_lowercase()) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Evaluate custom expression
    fn evaluate_expression(&self, _expr: &str, _context: &EvaluationContext) -> Result<bool> {
        // TODO: Implement expression evaluation
        // For now, return false as expressions are not yet supported
        Err(OrchestratorError::General(
            "Expression evaluation not yet implemented".to_string(),
        ))
    }
}

impl Default for RuleEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluation_context_creation() {
        let ctx = EvaluationContext::new("test input");
        assert_eq!(ctx.input_text, "test input");
        assert!(ctx.context.is_empty());
    }

    #[test]
    fn test_evaluation_context_with_context() {
        let ctx = EvaluationContext::new("test")
            .with_context("key1", Value::String("value1".to_string()))
            .with_context("key2", Value::Bool(true));

        assert_eq!(ctx.context.len(), 2);
        assert_eq!(ctx.context.get("key1"), Some(&Value::String("value1".to_string())));
        assert_eq!(ctx.context.get("key2"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_text_pattern_simple() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("debug mode enabled");

        let check = ConditionCheck::Text {
            pattern: "debug".to_string(),
        };

        assert!(evaluator.evaluate_check(&check, &ctx).unwrap());
    }

    #[test]
    fn test_text_pattern_regex() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("error code 404");

        let check = ConditionCheck::Text {
            pattern: r"\d+".to_string(),
        };

        assert!(evaluator.evaluate_check(&check, &ctx).unwrap());
    }

    #[test]
    fn test_text_pattern_case_insensitive() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("DEBUG mode");

        let check = ConditionCheck::Text {
            pattern: "/debug/i".to_string(),
        };

        assert!(evaluator.evaluate_check(&check, &ctx).unwrap());
    }

    #[test]
    fn test_context_has() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("test")
            .with_context("debug_mode", Value::Bool(true));

        let check = ConditionCheck::ContextHas {
            key: "debug_mode".to_string(),
        };

        assert!(evaluator.evaluate_check(&check, &ctx).unwrap());

        let check_missing = ConditionCheck::ContextHas {
            key: "missing_key".to_string(),
        };

        assert!(!evaluator.evaluate_check(&check_missing, &ctx).unwrap());
    }

    #[test]
    fn test_context_equals() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("test")
            .with_context("mode", Value::String("production".to_string()));

        let check = ConditionCheck::ContextEquals {
            key: "mode".to_string(),
            value: Value::String("production".to_string()),
        };

        assert!(evaluator.evaluate_check(&check, &ctx).unwrap());

        let check_wrong = ConditionCheck::ContextEquals {
            key: "mode".to_string(),
            value: Value::String("development".to_string()),
        };

        assert!(!evaluator.evaluate_check(&check_wrong, &ctx).unwrap());
    }

    #[test]
    fn test_contains() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("I need help with debugging");

        let check = ConditionCheck::Contains {
            keywords: vec!["help".to_string(), "debug".to_string()],
        };

        assert!(evaluator.evaluate_check(&check, &ctx).unwrap());
    }

    #[test]
    fn test_contains_case_insensitive() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("ERROR occurred");

        let check = ConditionCheck::Contains {
            keywords: vec!["error".to_string()],
        };

        assert!(evaluator.evaluate_check(&check, &ctx).unwrap());
    }

    #[test]
    fn test_condition_all() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("debug error")
            .with_context("mode", Value::String("debug".to_string()));

        let condition = RuleCondition::All {
            all: vec![
                ConditionCheck::Text {
                    pattern: "debug".to_string(),
                },
                ConditionCheck::ContextHas {
                    key: "mode".to_string(),
                },
            ],
        };

        assert!(evaluator.evaluate_condition(&condition, &ctx).unwrap());
    }

    #[test]
    fn test_condition_any() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("normal message");

        let condition = RuleCondition::Any {
            any: vec![
                ConditionCheck::Text {
                    pattern: "error".to_string(),
                },
                ConditionCheck::Text {
                    pattern: "normal".to_string(),
                },
            ],
        };

        assert!(evaluator.evaluate_condition(&condition, &ctx).unwrap());
    }

    #[test]
    fn test_condition_not() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("normal message");

        let condition = RuleCondition::Not {
            not: vec![
                ConditionCheck::Text {
                    pattern: "error".to_string(),
                },
                ConditionCheck::Text {
                    pattern: "warning".to_string(),
                },
            ],
        };

        assert!(evaluator.evaluate_condition(&condition, &ctx).unwrap());
    }

    #[test]
    fn test_evaluate_rule() {
        let evaluator = RuleEvaluator::new();
        let ctx = EvaluationContext::new("debug mode enabled")
            .with_context("debug_mode", Value::Bool(true));

        let rule = RouteRule {
            name: "debug_rule".to_string(),
            when: RuleCondition::Any {
                any: vec![
                    ConditionCheck::Text {
                        pattern: "/debug/i".to_string(),
                    },
                    ConditionCheck::ContextHas {
                        key: "debug_mode".to_string(),
                    },
                ],
            },
            prefer: vec!["debug_agent".to_string()],
            priority: 10,
        };

        assert!(evaluator.evaluate_rule(&rule, &ctx).unwrap());
    }
}
