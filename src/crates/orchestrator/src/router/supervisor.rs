//! Router/Supervisor for dynamic pattern selection
//!
//! Selects which pattern(s) to execute based on input, context, and routing rules.

use crate::config::{RouteRule, RouterConfig, TerminationCondition};
use crate::router::evaluator::{EvaluationContext, RuleEvaluator};
use crate::{OrchestratorError, Result};
use std::collections::HashSet;

/// Router for dynamic pattern selection
pub struct Router {
    config: RouterConfig,
    evaluator: RuleEvaluator,
}

impl Router {
    /// Create a new router from configuration
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            evaluator: RuleEvaluator::new(),
        }
    }

    /// Route an input to determine which pattern(s) to execute
    ///
    /// # Arguments
    /// * `context` - Evaluation context with input text and context values
    ///
    /// # Returns
    /// * List of pattern IDs to execute, in priority order
    pub fn route(&self, context: &EvaluationContext) -> Result<Vec<String>> {
        // Sort rules by priority (highest first)
        let mut sorted_rules: Vec<&RouteRule> = self.config.settings.route_policy.rules.iter().collect();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Evaluate rules in priority order
        let mut selected_patterns = Vec::new();
        let mut matched = false;

        for rule in sorted_rules {
            if self.evaluator.evaluate_rule(rule, context)? {
                // Rule matched - add preferred patterns
                for pattern_id in &rule.prefer {
                    if self.is_pattern_allowed(pattern_id) {
                        selected_patterns.push(pattern_id.clone());
                        matched = true;
                    }
                }
                // Use first matching rule (highest priority)
                break;
            }
        }

        // If no rules matched, use default patterns
        if !matched {
            for pattern_id in &self.config.settings.route_policy.default {
                if self.is_pattern_allowed(pattern_id) {
                    selected_patterns.push(pattern_id.clone());
                }
            }
        }

        // Apply guards
        if let Some(guards) = &self.config.settings.guards {
            if guards.enforce_registry && selected_patterns.is_empty() && guards.fallback_to_default {
                // Fallback to defaults if enforcement is on and nothing selected
                selected_patterns.extend(self.config.settings.route_policy.default.clone());
            }
        }

        // Remove duplicates while preserving order
        let mut seen = HashSet::new();
        selected_patterns.retain(|id| seen.insert(id.clone()));

        Ok(selected_patterns)
    }

    /// Check if a pattern is allowed by the registry configuration
    fn is_pattern_allowed(&self, pattern_id: &str) -> bool {
        // Check denylist first
        if self.config.registry.deny.contains(&pattern_id.to_string()) {
            return false;
        }

        // If allowlist is empty, allow all (except denied)
        if self.config.registry.allow.is_empty() {
            return true;
        }

        // Check allowlist
        self.config.registry.allow.contains(&pattern_id.to_string())
    }

    /// Check if execution should terminate
    ///
    /// # Arguments
    /// * `steps` - Number of steps executed
    /// * `output` - Latest output text
    /// * `current_pattern` - Currently executing pattern ID
    pub fn should_terminate(
        &self,
        steps: usize,
        output: &str,
        current_pattern: &str,
    ) -> Result<bool> {
        if let Some(termination) = &self.config.settings.termination {
            // Check "any" conditions (OR logic)
            if let Some(any_conditions) = &termination.any {
                for condition in any_conditions {
                    if self.check_termination_condition(condition, steps, output, current_pattern)? {
                        return Ok(true);
                    }
                }
            }

            // Check "all" conditions (AND logic)
            if let Some(all_conditions) = &termination.all {
                let all_met = all_conditions.iter().all(|condition| {
                    self.check_termination_condition(condition, steps, output, current_pattern)
                        .unwrap_or(false)
                });
                if all_met && !all_conditions.is_empty() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check an individual termination condition
    fn check_termination_condition(
        &self,
        condition: &TerminationCondition,
        steps: usize,
        output: &str,
        current_pattern: &str,
    ) -> Result<bool> {
        match condition {
            TerminationCondition::MaxSteps(max) => Ok(steps >= *max),
            TerminationCondition::Contains(keywords) => {
                let output_lower = output.to_lowercase();
                Ok(keywords.iter().any(|kw| output_lower.contains(&kw.to_lowercase())))
            }
            TerminationCondition::PatternName(name) => Ok(current_pattern == name),
            TerminationCondition::Expression(_expr) => {
                // TODO: Implement expression evaluation
                Err(OrchestratorError::General(
                    "Expression-based termination not yet implemented".to_string(),
                ))
            }
        }
    }

    /// Get the router configuration
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    /// Get the maximum routing attempts from guards
    pub fn max_routing_attempts(&self) -> usize {
        self.config
            .settings
            .guards
            .as_ref()
            .map(|g| g.max_routing_attempts)
            .unwrap_or(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ConditionCheck, GuardConfig, RegistryConfig, RoutePolicyConfig, RouterSettings,
        RuleCondition, TerminationConfig,
    };
    use serde_json::Value;
    use std::collections::HashMap;

    fn create_test_router() -> Router {
        let config = RouterConfig {
            id: "test_router".to_string(),
            description: None,
            registry: RegistryConfig {
                allow: vec!["react_1".to_string(), "plan_execute_1".to_string()],
                deny: vec![],
            },
            settings: RouterSettings {
                route_policy: RoutePolicyConfig {
                    rules: vec![
                        RouteRule {
                            name: "debug_rule".to_string(),
                            priority: 10,
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
                            prefer: vec!["react_1".to_string()],
                        },
                        RouteRule {
                            name: "plan_rule".to_string(),
                            priority: 5,
                            when: RuleCondition::Single(ConditionCheck::Contains {
                                keywords: vec!["plan".to_string(), "strategy".to_string()],
                            }),
                            prefer: vec!["plan_execute_1".to_string()],
                        },
                    ],
                    default: vec!["react_1".to_string()],
                },
                termination: Some(TerminationConfig {
                    any: Some(vec![
                        TerminationCondition::MaxSteps(10),
                        TerminationCondition::Contains(vec!["complete".to_string()]),
                    ]),
                    all: None,
                }),
                guards: Some(GuardConfig {
                    enforce_registry: true,
                    fallback_to_default: true,
                    max_routing_attempts: 10,
                }),
            },
        };

        Router::new(config)
    }

    #[test]
    fn test_router_creation() {
        let router = create_test_router();
        assert_eq!(router.config().id, "test_router");
    }

    #[test]
    fn test_route_with_matching_rule() {
        let router = create_test_router();
        let ctx = EvaluationContext::new("debug this issue");

        let patterns = router.route(&ctx).unwrap();
        assert_eq!(patterns, vec!["react_1"]);
    }

    #[test]
    fn test_route_with_context() {
        let router = create_test_router();
        let ctx = EvaluationContext::new("normal input")
            .with_context("debug_mode", Value::Bool(true));

        let patterns = router.route(&ctx).unwrap();
        assert_eq!(patterns, vec!["react_1"]);
    }

    #[test]
    fn test_route_with_default() {
        let router = create_test_router();
        let ctx = EvaluationContext::new("random input");

        let patterns = router.route(&ctx).unwrap();
        assert_eq!(patterns, vec!["react_1"]); // Should use default
    }

    #[test]
    fn test_route_respects_priority() {
        let router = create_test_router();
        // Both rules could match "plan debug", but debug has higher priority
        let ctx = EvaluationContext::new("debug and plan");

        let patterns = router.route(&ctx).unwrap();
        assert_eq!(patterns, vec!["react_1"]); // debug_rule wins (priority 10 > 5)
    }

    #[test]
    fn test_pattern_allowlist() {
        let config = RouterConfig {
            id: "test".to_string(),
            description: None,
            registry: RegistryConfig {
                allow: vec!["allowed_pattern".to_string()],
                deny: vec![],
            },
            settings: RouterSettings {
                route_policy: RoutePolicyConfig {
                    rules: vec![],
                    default: vec!["allowed_pattern".to_string(), "blocked_pattern".to_string()],
                },
                termination: None,
                guards: None,
            },
        };

        let router = Router::new(config);
        let ctx = EvaluationContext::new("test");

        let patterns = router.route(&ctx).unwrap();
        assert_eq!(patterns, vec!["allowed_pattern"]);
    }

    #[test]
    fn test_pattern_denylist() {
        let config = RouterConfig {
            id: "test".to_string(),
            description: None,
            registry: RegistryConfig {
                allow: vec![],
                deny: vec!["denied_pattern".to_string()],
            },
            settings: RouterSettings {
                route_policy: RoutePolicyConfig {
                    rules: vec![],
                    default: vec!["allowed_pattern".to_string(), "denied_pattern".to_string()],
                },
                termination: None,
                guards: None,
            },
        };

        let router = Router::new(config);
        let ctx = EvaluationContext::new("test");

        let patterns = router.route(&ctx).unwrap();
        assert_eq!(patterns, vec!["allowed_pattern"]);
    }

    #[test]
    fn test_termination_max_steps() {
        let router = create_test_router();

        assert!(!router.should_terminate(5, "", "react_1").unwrap());
        assert!(router.should_terminate(10, "", "react_1").unwrap());
        assert!(router.should_terminate(15, "", "react_1").unwrap());
    }

    #[test]
    fn test_termination_contains() {
        let router = create_test_router();

        assert!(!router.should_terminate(1, "in progress", "react_1").unwrap());
        assert!(router.should_terminate(1, "task complete", "react_1").unwrap());
    }

    #[test]
    fn test_max_routing_attempts() {
        let router = create_test_router();
        assert_eq!(router.max_routing_attempts(), 10);
    }

    #[test]
    fn test_route_removes_duplicates() {
        let config = RouterConfig {
            id: "test".to_string(),
            description: None,
            registry: RegistryConfig {
                allow: vec![],
                deny: vec![],
            },
            settings: RouterSettings {
                route_policy: RoutePolicyConfig {
                    rules: vec![RouteRule {
                        name: "multi_rule".to_string(),
                        priority: 10,
                        when: RuleCondition::Single(ConditionCheck::Text {
                            pattern: "test".to_string(),
                        }),
                        prefer: vec!["pattern1".to_string(), "pattern2".to_string(), "pattern1".to_string()],
                    }],
                    default: vec![],
                },
                termination: None,
                guards: None,
            },
        };

        let router = Router::new(config);
        let ctx = EvaluationContext::new("test");

        let patterns = router.route(&ctx).unwrap();
        assert_eq!(patterns, vec!["pattern1", "pattern2"]); // No duplicates
    }
}
