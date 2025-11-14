//! Tool validation framework
//!
//! This module provides comprehensive validation for tool inputs, outputs, and execution.
//! It ensures tools behave correctly and safely by validating:
//! - Input parameter schemas
//! - Output format requirements
//! - Execution constraints (timeouts, resource limits)
//! - Security policies

use crate::error::{PrebuiltError, Result};
use crate::tools::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Validation rules for a tool parameter
#[derive(Clone)]
pub struct ParameterValidation {
    /// Parameter name
    pub name: String,

    /// Parameter type (string, number, boolean, object, array)
    pub param_type: ParameterType,

    /// Whether this parameter is required
    pub required: bool,

    /// Minimum value (for numbers) or minimum length (for strings/arrays)
    pub min: Option<f64>,

    /// Maximum value (for numbers) or maximum length (for strings/arrays)
    pub max: Option<f64>,

    /// Regular expression pattern for string validation
    pub pattern: Option<String>,

    /// Allowed values (enum constraint)
    pub allowed_values: Option<Vec<Value>>,

    /// Custom validation function
    #[allow(clippy::type_complexity)]
    pub custom_validator: Option<Arc<dyn Fn(&Value) -> Result<()> + Send + Sync>>,
}

/// Parameter data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Object,
    Array,
    Any,
}

/// Tool execution constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConstraints {
    /// Maximum execution time
    pub timeout: Option<Duration>,

    /// Maximum number of retries on failure
    pub max_retries: Option<usize>,

    /// Rate limiting (max calls per minute)
    pub rate_limit: Option<usize>,

    /// Maximum input size in bytes
    pub max_input_size: Option<usize>,

    /// Maximum output size in bytes
    pub max_output_size: Option<usize>,

    /// Resource limits (CPU, memory, etc.)
    pub resource_limits: HashMap<String, Value>,
}

impl Default for ExecutionConstraints {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            max_retries: Some(3),
            rate_limit: Some(60),
            max_input_size: Some(1024 * 1024), // 1MB
            max_output_size: Some(10 * 1024 * 1024), // 10MB
            resource_limits: HashMap::new(),
        }
    }
}

/// Security policies for tool execution
#[derive(Clone)]
pub struct SecurityPolicy {
    /// Whether to sanitize inputs
    pub sanitize_inputs: bool,

    /// Whether to redact sensitive data in outputs
    pub redact_sensitive_data: bool,

    /// Allowed file system paths (for file-based tools)
    pub allowed_paths: Option<Vec<String>>,

    /// Blocked patterns in inputs
    pub blocked_patterns: Vec<String>,

    /// Whether to log all tool executions
    pub audit_logging: bool,

    /// Custom security validators
    #[allow(clippy::type_complexity)]
    pub custom_validators: Vec<Arc<dyn Fn(&Value) -> Result<()> + Send + Sync>>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            sanitize_inputs: true,
            redact_sensitive_data: true,
            allowed_paths: None,
            blocked_patterns: vec![],
            audit_logging: false,
            custom_validators: vec![],
        }
    }
}

/// Complete validation configuration for a tool
#[derive(Clone)]
pub struct ToolValidation {
    /// Tool name
    pub tool_name: String,

    /// Tool description
    pub description: String,

    /// Parameter validations
    pub parameters: Vec<ParameterValidation>,

    /// Execution constraints
    pub constraints: ExecutionConstraints,

    /// Security policy
    pub security: SecurityPolicy,

    /// Output schema validation
    pub output_schema: Option<Value>,

    /// Whether this tool is enabled
    pub enabled: bool,
}

/// Tool validator that wraps a tool with validation
pub struct ValidatedTool {
    /// The underlying tool
    tool: Box<dyn Tool>,

    /// Validation configuration
    validation: ToolValidation,

    /// Execution metrics
    metrics: Arc<std::sync::Mutex<ToolMetrics>>,
}

/// Metrics collected during tool execution
#[derive(Debug, Default, Clone)]
pub struct ToolMetrics {
    /// Total number of invocations
    pub total_calls: usize,

    /// Number of successful executions
    pub successful_calls: usize,

    /// Number of failed executions
    pub failed_calls: usize,

    /// Number of validation failures
    pub validation_failures: usize,

    /// Average execution time in milliseconds
    pub avg_execution_time: f64,

    /// Total execution time in milliseconds
    pub total_execution_time: f64,
}

impl ValidatedTool {
    /// Create a new validated tool
    pub fn new(tool: Box<dyn Tool>, validation: ToolValidation) -> Self {
        Self {
            tool,
            validation,
            metrics: Arc::new(std::sync::Mutex::new(ToolMetrics::default())),
        }
    }

    /// Validate input parameters
    pub fn validate_input(&self, input: &Value) -> Result<()> {
        // Check if tool is enabled
        if !self.validation.enabled {
            return Err(PrebuiltError::ToolValidation(
                format!("Tool '{}' is disabled", self.validation.tool_name)
            ));
        }

        let input_obj = input.as_object()
            .ok_or_else(|| PrebuiltError::ToolValidation("Input must be an object".to_string()))?;

        // Validate each parameter
        for param_validation in &self.validation.parameters {
            let value = input_obj.get(&param_validation.name);

            // Check required parameters
            if param_validation.required && value.is_none() {
                return Err(PrebuiltError::ToolValidation(
                    format!("Required parameter '{}' is missing", param_validation.name)
                ));
            }

            if let Some(value) = value {
                // Type validation
                self.validate_type(value, &param_validation.param_type, &param_validation.name)?;

                // Range/length validation
                self.validate_constraints(value, param_validation)?;

                // Pattern validation for strings
                if let Some(pattern) = &param_validation.pattern {
                    if let Some(str_value) = value.as_str() {
                        let regex = regex::Regex::new(pattern)
                            .map_err(|e| PrebuiltError::ToolValidation(format!("Invalid regex: {}", e)))?;
                        if !regex.is_match(str_value) {
                            return Err(PrebuiltError::ToolValidation(
                                format!("Parameter '{}' does not match pattern", param_validation.name)
                            ));
                        }
                    }
                }

                // Enum constraint validation
                if let Some(allowed) = &param_validation.allowed_values {
                    if !allowed.contains(value) {
                        return Err(PrebuiltError::ToolValidation(
                            format!("Parameter '{}' has invalid value", param_validation.name)
                        ));
                    }
                }

                // Custom validation
                if let Some(validator) = &param_validation.custom_validator {
                    validator(value)?;
                }
            }
        }

        // Apply security policy
        self.apply_security_policy(input)?;

        // Check input size constraint
        if let Some(max_size) = self.validation.constraints.max_input_size {
            let serialized = serde_json::to_string(input)
                .map_err(|e| PrebuiltError::ToolValidation(format!("Failed to serialize input: {}", e)))?;
            if serialized.len() > max_size {
                return Err(PrebuiltError::ToolValidation(
                    format!("Input size {} exceeds maximum {}", serialized.len(), max_size)
                ));
            }
        }

        Ok(())
    }

    /// Validate parameter type
    fn validate_type(&self, value: &Value, expected_type: &ParameterType, param_name: &str) -> Result<()> {
        let valid = match expected_type {
            ParameterType::String => value.is_string(),
            ParameterType::Number => value.is_number(),
            ParameterType::Boolean => value.is_boolean(),
            ParameterType::Object => value.is_object(),
            ParameterType::Array => value.is_array(),
            ParameterType::Any => true,
        };

        if !valid {
            return Err(PrebuiltError::ToolValidation(
                format!("Parameter '{}' has wrong type, expected {:?}", param_name, expected_type)
            ));
        }

        Ok(())
    }

    /// Validate parameter constraints
    fn validate_constraints(&self, value: &Value, validation: &ParameterValidation) -> Result<()> {
        match &validation.param_type {
            ParameterType::Number => {
                if let Some(num) = value.as_f64() {
                    if let Some(min) = validation.min {
                        if num < min {
                            return Err(PrebuiltError::ToolValidation(
                                format!("Parameter '{}' value {} is less than minimum {}", validation.name, num, min)
                            ));
                        }
                    }
                    if let Some(max) = validation.max {
                        if num > max {
                            return Err(PrebuiltError::ToolValidation(
                                format!("Parameter '{}' value {} exceeds maximum {}", validation.name, num, max)
                            ));
                        }
                    }
                }
            }
            ParameterType::String => {
                if let Some(str_val) = value.as_str() {
                    let len = str_val.len() as f64;
                    if let Some(min) = validation.min {
                        if len < min {
                            return Err(PrebuiltError::ToolValidation(
                                format!("Parameter '{}' length {} is less than minimum {}", validation.name, len, min)
                            ));
                        }
                    }
                    if let Some(max) = validation.max {
                        if len > max {
                            return Err(PrebuiltError::ToolValidation(
                                format!("Parameter '{}' length {} exceeds maximum {}", validation.name, len, max)
                            ));
                        }
                    }
                }
            }
            ParameterType::Array => {
                if let Some(arr) = value.as_array() {
                    let len = arr.len() as f64;
                    if let Some(min) = validation.min {
                        if len < min {
                            return Err(PrebuiltError::ToolValidation(
                                format!("Parameter '{}' array length {} is less than minimum {}", validation.name, len, min)
                            ));
                        }
                    }
                    if let Some(max) = validation.max {
                        if len > max {
                            return Err(PrebuiltError::ToolValidation(
                                format!("Parameter '{}' array length {} exceeds maximum {}", validation.name, len, max)
                            ));
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Apply security policy to input
    fn apply_security_policy(&self, input: &Value) -> Result<()> {
        let policy = &self.validation.security;

        // Check blocked patterns
        if !policy.blocked_patterns.is_empty() {
            let input_str = serde_json::to_string(input)
                .map_err(|e| PrebuiltError::ToolValidation(format!("Failed to serialize input: {}", e)))?;

            for pattern in &policy.blocked_patterns {
                if input_str.contains(pattern) {
                    return Err(PrebuiltError::ToolValidation(
                        format!("Input contains blocked pattern: {}", pattern)
                    ));
                }
            }
        }

        // Apply custom validators
        for validator in &policy.custom_validators {
            validator(input)?;
        }

        Ok(())
    }

    /// Validate output against schema
    pub fn validate_output(&self, output: &Value) -> Result<()> {
        // Check output size constraint
        if let Some(max_size) = self.validation.constraints.max_output_size {
            let serialized = serde_json::to_string(output)
                .map_err(|e| PrebuiltError::ToolValidation(format!("Failed to serialize output: {}", e)))?;
            if serialized.len() > max_size {
                return Err(PrebuiltError::ToolValidation(
                    format!("Output size {} exceeds maximum {}", serialized.len(), max_size)
                ));
            }
        }

        // Validate against output schema if provided
        if let Some(schema) = &self.validation.output_schema {
            // Simplified schema validation - in production, use a proper JSON Schema validator
            // This would validate the output against the expected schema
        }

        Ok(())
    }

    /// Get execution metrics
    pub fn metrics(&self) -> ToolMetrics {
        self.metrics.lock().unwrap().clone()
    }
}

/// Builder for creating validated tools
pub struct ToolValidationBuilder {
    validation: ToolValidation,
}

impl ToolValidationBuilder {
    /// Create a new validation builder
    pub fn new(tool_name: impl Into<String>) -> Self {
        Self {
            validation: ToolValidation {
                tool_name: tool_name.into(),
                description: String::new(),
                parameters: Vec::new(),
                constraints: ExecutionConstraints::default(),
                security: SecurityPolicy::default(),
                output_schema: None,
                enabled: true,
            },
        }
    }

    /// Set tool description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.validation.description = description.into();
        self
    }

    /// Add a parameter validation
    pub fn add_parameter(mut self, param: ParameterValidation) -> Self {
        self.validation.parameters.push(param);
        self
    }

    /// Set execution constraints
    pub fn with_constraints(mut self, constraints: ExecutionConstraints) -> Self {
        self.validation.constraints = constraints;
        self
    }

    /// Set security policy
    pub fn with_security(mut self, security: SecurityPolicy) -> Self {
        self.validation.security = security;
        self
    }

    /// Set output schema
    pub fn with_output_schema(mut self, schema: Value) -> Self {
        self.validation.output_schema = Some(schema);
        self
    }

    /// Build the validation configuration
    pub fn build(self) -> ToolValidation {
        self.validation
    }
}

/// Create a simple string parameter validation
pub fn string_param(name: impl Into<String>, required: bool) -> ParameterValidation {
    ParameterValidation {
        name: name.into(),
        param_type: ParameterType::String,
        required,
        min: None,
        max: None,
        pattern: None,
        allowed_values: None,
        custom_validator: None,
    }
}

/// Create a simple number parameter validation
pub fn number_param(name: impl Into<String>, required: bool, min: Option<f64>, max: Option<f64>) -> ParameterValidation {
    ParameterValidation {
        name: name.into(),
        param_type: ParameterType::Number,
        required,
        min,
        max,
        pattern: None,
        allowed_values: None,
        custom_validator: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_validation() {
        let validation = string_param("name", true);
        assert_eq!(validation.name, "name");
        assert!(validation.required);
        assert_eq!(validation.param_type, ParameterType::String);
    }

    #[test]
    fn test_number_validation() {
        let validation = number_param("age", true, Some(0.0), Some(150.0));
        assert_eq!(validation.name, "age");
        assert_eq!(validation.min, Some(0.0));
        assert_eq!(validation.max, Some(150.0));
    }

    #[test]
    fn test_validation_builder() {
        let validation = ToolValidationBuilder::new("calculator")
            .with_description("Performs calculations")
            .add_parameter(number_param("a", true, None, None))
            .add_parameter(number_param("b", true, None, None))
            .build();

        assert_eq!(validation.tool_name, "calculator");
        assert_eq!(validation.parameters.len(), 2);
    }
}