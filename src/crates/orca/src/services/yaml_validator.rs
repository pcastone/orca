//! YAML validation service
//!
//! Validates YAML content for workflows, patterns, prompts, and other file types.

use crate::error::{OrcaError, Result};
use serde_json::Value;

/// Validated workflow data
#[derive(Debug, Clone)]
pub struct ValidatedWorkflow {
    /// Workflow ID/name
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Pattern type (react, plan_execute, etc.)
    pub pattern: String,
    /// Full definition as JSON
    pub definition: Value,
    /// Tags
    pub tags: Vec<String>,
}

/// Validated pattern configuration
#[derive(Debug, Clone)]
pub struct ValidatedPattern {
    /// Pattern name
    pub name: String,
    /// Pattern type
    pub pattern_type: String,
    /// Configuration object
    pub config: Value,
    /// Tools list
    pub tools: Value,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Maximum iterations
    pub max_iterations: i64,
    /// Whether this is the default pattern
    pub is_default: bool,
}

/// Validated prompt
#[derive(Debug, Clone)]
pub struct ValidatedPrompt {
    /// Prompt name
    pub name: String,
    /// Template content
    pub template: String,
    /// Category
    pub category: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Template variables
    pub variables: Vec<String>,
}

/// Validated tool definition
#[derive(Debug, Clone)]
pub struct ValidatedTool {
    /// Tool name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Tool configuration
    pub config: Value,
}

/// YAML content validator
pub struct YamlValidator;

impl YamlValidator {
    /// Validate workflow YAML structure
    pub fn validate_workflow(content: &Value) -> Result<ValidatedWorkflow> {
        // Extract ID - required
        let id = content
            .get("id")
            .and_then(|v| v.as_str())
            .or_else(|| content.get("name").and_then(|v| v.as_str()))
            .ok_or_else(|| {
                OrcaError::Validation("Workflow missing 'id' or 'name' field".into())
            })?
            .to_string();

        // Name defaults to ID
        let name = content
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();

        // Description is optional
        let description = content
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract pattern from first step or top-level, default to "react"
        let pattern = content
            .get("pattern")
            .and_then(|p| p.as_str())
            .or_else(|| {
                content
                    .get("steps")
                    .and_then(|s| s.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|step| step.get("pattern"))
                    .and_then(|p| p.as_str())
            })
            .unwrap_or("react")
            .to_string();

        // Extract tags
        let tags = content
            .get("tags")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(ValidatedWorkflow {
            id,
            name,
            description,
            pattern,
            definition: content.clone(),
            tags,
        })
    }

    /// Validate pattern configuration YAML
    pub fn validate_pattern(content: &Value) -> Result<ValidatedPattern> {
        // Name is required
        let name = content
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OrcaError::Validation("Pattern missing 'name' field".into()))?
            .to_string();

        // Type defaults to react
        let pattern_type = content
            .get("type")
            .or_else(|| content.get("pattern_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("react")
            .to_string();

        // Config object
        let config = content
            .get("config")
            .cloned()
            .unwrap_or(Value::Object(Default::default()));

        // Max iterations from config or top-level
        let max_iterations = config
            .get("max_iterations")
            .and_then(|v| v.as_i64())
            .or_else(|| content.get("max_iterations").and_then(|v| v.as_i64()))
            .unwrap_or(10);

        // Tools array
        let tools = content
            .get("tools")
            .or_else(|| config.get("tools"))
            .cloned()
            .unwrap_or(Value::Array(vec![]));

        // System prompt from prompts.system or top-level
        let system_prompt = content
            .get("prompts")
            .and_then(|p| p.get("system"))
            .and_then(|s| s.as_str())
            .or_else(|| content.get("system_prompt").and_then(|s| s.as_str()))
            .map(|s| s.to_string());

        // Is default
        let is_default = content
            .get("is_default")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(ValidatedPattern {
            name,
            pattern_type,
            config,
            tools,
            system_prompt,
            max_iterations,
            is_default,
        })
    }

    /// Validate prompt YAML
    pub fn validate_prompt(content: &Value) -> Result<ValidatedPrompt> {
        // Handle both single prompt and examples array format
        if let Some(_examples) = content.get("examples").and_then(|e| e.as_array()) {
            // Few-shot examples format - store entire content as template
            return Ok(ValidatedPrompt {
                name: content
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("few_shot_examples")
                    .to_string(),
                template: serde_json::to_string_pretty(content)
                    .map_err(|e| OrcaError::Validation(format!("Failed to serialize: {}", e)))?,
                category: Some("examples".to_string()),
                description: content
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                variables: vec![],
            });
        }

        // Standard prompt format
        let name = content
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OrcaError::Validation("Prompt missing 'name' field".into()))?
            .to_string();

        let template = content
            .get("template")
            .and_then(|v| v.as_str())
            .or_else(|| content.get("content").and_then(|v| v.as_str()))
            .ok_or_else(|| OrcaError::Validation("Prompt missing 'template' field".into()))?
            .to_string();

        let category = content
            .get("category")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let description = content
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract variables from template ({{var}} pattern)
        let variables = Self::extract_template_variables(&template);

        Ok(ValidatedPrompt {
            name,
            template,
            category,
            description,
            variables,
        })
    }

    /// Validate tool definition YAML
    pub fn validate_tool(content: &Value) -> Result<ValidatedTool> {
        let name = content
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OrcaError::Validation("Tool missing 'name' field".into()))?
            .to_string();

        let description = content
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(ValidatedTool {
            name,
            description,
            config: content.clone(),
        })
    }

    /// Extract template variables from a template string
    /// Looks for {{variable}} patterns
    fn extract_template_variables(template: &str) -> Vec<String> {
        let re = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();
        re.captures_iter(template)
            .filter_map(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .collect()
    }

    /// Validate YAML content is valid YAML
    pub fn validate_yaml_syntax(content: &str) -> Result<Value> {
        serde_yaml::from_str(content)
            .map_err(|e| OrcaError::Validation(format!("Invalid YAML syntax: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_workflow() {
        let content = json!({
            "id": "test_workflow",
            "description": "A test workflow",
            "pattern": "react",
            "tags": ["test", "example"]
        });

        let result = YamlValidator::validate_workflow(&content).unwrap();
        assert_eq!(result.id, "test_workflow");
        assert_eq!(result.name, "test_workflow");
        assert_eq!(result.pattern, "react");
        assert_eq!(result.tags, vec!["test", "example"]);
    }

    #[test]
    fn test_validate_pattern() {
        let content = json!({
            "name": "test_pattern",
            "type": "plan_execute",
            "max_iterations": 15,
            "tools": ["read_file", "write_file"]
        });

        let result = YamlValidator::validate_pattern(&content).unwrap();
        assert_eq!(result.name, "test_pattern");
        assert_eq!(result.pattern_type, "plan_execute");
        assert_eq!(result.max_iterations, 15);
    }

    #[test]
    fn test_validate_prompt() {
        let content = json!({
            "name": "test_prompt",
            "template": "Hello {{name}}, welcome to {{project}}!",
            "category": "greeting"
        });

        let result = YamlValidator::validate_prompt(&content).unwrap();
        assert_eq!(result.name, "test_prompt");
        assert_eq!(result.variables, vec!["name", "project"]);
    }

    #[test]
    fn test_extract_template_variables() {
        let template = "Hello {{name}}, your order {{order_id}} is ready!";
        let vars = YamlValidator::extract_template_variables(template);
        assert_eq!(vars, vec!["name", "order_id"]);
    }
}
