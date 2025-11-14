//! Router and supervisor configuration
//!
//! Defines configuration for dynamic pattern selection and routing based on
//! input messages and context.

use serde::{Deserialize, Serialize};

/// Router/Supervisor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Router identifier
    pub id: String,
    /// Description of the router's purpose
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Pattern registry configuration
    pub registry: RegistryConfig,
    /// Routing policy settings
    pub settings: RouterSettings,
}

/// Pattern registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Allowed pattern IDs (allowlist)
    #[serde(default)]
    pub allow: Vec<String>,
    /// Blocked pattern IDs (denylist)
    #[serde(default)]
    pub deny: Vec<String>,
}

/// Router settings for pattern selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterSettings {
    /// Route policy configuration
    pub route_policy: RoutePolicyConfig,
    /// Termination conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination: Option<TerminationConfig>,
    /// Guard configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guards: Option<GuardConfig>,
}

/// Route policy with rules for pattern selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePolicyConfig {
    /// Routing rules (evaluated in order)
    #[serde(default)]
    pub rules: Vec<RouteRule>,
    /// Default pattern IDs if no rules match
    #[serde(default)]
    pub default: Vec<String>,
}

/// A single routing rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    /// Rule name/identifier
    pub name: String,
    /// Conditions for this rule to match
    pub when: RuleCondition,
    /// Preferred patterns if rule matches
    pub prefer: Vec<String>,
    /// Rule priority (higher = evaluated first)
    #[serde(default)]
    pub priority: i32,
}

/// Conditions for rule matching
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleCondition {
    /// Single condition
    Single(ConditionCheck),
    /// All conditions must match (AND)
    All { all: Vec<ConditionCheck> },
    /// Any condition must match (OR)
    Any { any: Vec<ConditionCheck> },
    /// No conditions must match (NOT)
    Not { not: Vec<ConditionCheck> },
}

/// Individual condition check
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConditionCheck {
    /// Match text against regex pattern
    Text { pattern: String },
    /// Check if context has a specific key
    ContextHas { key: String },
    /// Check if context key equals value
    ContextEquals { key: String, value: serde_json::Value },
    /// Check if input contains keywords
    Contains { keywords: Vec<String> },
    /// Custom condition expression
    Expression { expr: String },
}

/// Termination conditions for routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminationConfig {
    /// Any of these conditions triggers termination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any: Option<Vec<TerminationCondition>>,
    /// All of these conditions must be met
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<TerminationCondition>>,
}

/// Individual termination condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum TerminationCondition {
    /// Maximum number of steps
    MaxSteps(usize),
    /// Output contains specific keywords
    Contains(Vec<String>),
    /// Pattern name matches
    PatternName(String),
    /// Custom termination expression
    Expression(String),
}

/// Guard configuration for router safety
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    /// Enforce registry allowlist
    #[serde(default = "default_true")]
    pub enforce_registry: bool,
    /// Fallback to default if no matches
    #[serde(default = "default_true")]
    pub fallback_to_default: bool,
    /// Maximum routing attempts
    #[serde(default = "default_max_routing_attempts")]
    pub max_routing_attempts: usize,
}

fn default_true() -> bool {
    true
}

fn default_max_routing_attempts() -> usize {
    10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_config_deserialization() {
        let yaml = r#"
            id: "main_router"
            description: "Main routing supervisor"
            registry:
              allow: ["react_1", "plan_execute_1"]
              deny: []
            settings:
              route_policy:
                rules:
                  - name: "debug_rule"
                    priority: 10
                    when:
                      any:
                        - type: text
                          pattern: "/debug|log/i"
                        - type: context_has
                          key: "debug_mode"
                    prefer: ["react_1"]
                default: ["plan_execute_1"]
              termination:
                any:
                  - type: max_steps
                    value: 8
                  - type: contains
                    value: ["complete", "done"]
              guards:
                enforce_registry: true
                fallback_to_default: true
        "#;

        let config: RouterConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.id, "main_router");
        assert_eq!(config.registry.allow, vec!["react_1", "plan_execute_1"]);
        assert_eq!(config.settings.route_policy.rules.len(), 1);
        assert_eq!(config.settings.route_policy.rules[0].name, "debug_rule");
        assert_eq!(config.settings.route_policy.rules[0].priority, 10);
    }

    #[test]
    fn test_rule_condition_any() {
        let yaml = r#"
            any:
              - type: text
                pattern: "/pattern/"
              - type: context_has
                key: "key"
        "#;

        let condition: RuleCondition = serde_yaml::from_str(yaml).unwrap();

        match condition {
            RuleCondition::Any { any } => {
                assert_eq!(any.len(), 2);
            }
            _ => panic!("Expected Any condition"),
        }
    }

    #[test]
    fn test_termination_config() {
        let yaml = r#"
            any:
              - type: max_steps
                value: 5
              - type: contains
                value: ["done"]
        "#;

        let config: TerminationConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.any.is_some());
        assert_eq!(config.any.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_guard_config_defaults() {
        let yaml = r#"
            enforce_registry: false
        "#;

        let config: GuardConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(!config.enforce_registry);
        assert!(config.fallback_to_default);
        assert_eq!(config.max_routing_attempts, 10);
    }

    #[test]
    fn test_context_equals_condition() {
        let yaml = r#"
            type: context_equals
            key: "mode"
            value: "production"
        "#;

        let condition: ConditionCheck = serde_yaml::from_str(yaml).unwrap();

        match condition {
            ConditionCheck::ContextEquals { key, value } => {
                assert_eq!(key, "mode");
                assert_eq!(value, serde_json::Value::String("production".to_string()));
            }
            _ => panic!("Expected ContextEquals"),
        }
    }
}
