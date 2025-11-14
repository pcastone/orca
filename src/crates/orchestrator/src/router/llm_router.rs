//! LLM-based Router for intelligent pattern selection
//!
//! Uses an LLM (Planning LLM) to select patterns based on user input,
//! with fallback to rule-based routing if LLM fails.

use crate::config::RouterConfig;
use crate::pattern::PatternSelector;
use crate::router::evaluator::EvaluationContext;
use crate::router::supervisor::Router as RuleBasedRouter;
use crate::{OrchestratorError, Result};
use langgraph_core::llm::{ChatModel, ChatRequest};
use langgraph_core::messages::Message;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Prompt template for pattern selection
const PATTERN_SELECTION_PROMPT: &str = r#"You are a routing assistant that selects the best pattern for handling user requests.

Available patterns:
{patterns}

User input: {input}

Context: {context}

Select the most appropriate pattern ID to handle this request. Respond with ONLY the pattern ID (e.g., "react_1", "plan_execute", etc.).
If multiple patterns could work, select the most specific one. If unsure, select "react_1" as the default.

Pattern ID:"#;

/// LLM-based router for pattern selection
pub struct LlmRouter {
    /// Planning LLM client
    llm: Arc<dyn ChatModel>,
    /// Pattern selector for input analysis
    pattern_selector: PatternSelector,
    /// Fallback rule-based router
    fallback_router: RuleBasedRouter,
    /// Router configuration
    config: RouterConfig,
}

impl LlmRouter {
    /// Create a new LLM router
    ///
    /// # Arguments
    /// * `llm` - Planning LLM client
    /// * `config` - Router configuration
    pub fn new(llm: Arc<dyn ChatModel>, config: RouterConfig) -> Self {
        let fallback_router = RuleBasedRouter::new(config.clone());
        let pattern_selector = PatternSelector::new();

        Self {
            llm,
            pattern_selector,
            fallback_router,
            config,
        }
    }

    /// Route using LLM with fallback to rules
    ///
    /// # Arguments
    /// * `context` - Evaluation context with input and context values
    ///
    /// # Returns
    /// * List of pattern IDs to execute
    pub async fn route(&self, context: &EvaluationContext) -> Result<Vec<String>> {
        debug!("Attempting LLM-based routing for input: {}", context.input_text);

        // Try LLM routing first
        match self.route_with_llm(context).await {
            Ok(patterns) if !patterns.is_empty() => {
                info!("LLM routing successful: selected {:?}", patterns);
                Ok(patterns)
            }
            Ok(_) => {
                warn!("LLM returned empty pattern list, falling back to rules");
                self.fallback_to_rules(context)
            }
            Err(e) => {
                warn!("LLM routing failed: {}, falling back to rules", e);
                self.fallback_to_rules(context)
            }
        }
    }

    /// Route using LLM
    async fn route_with_llm(&self, context: &EvaluationContext) -> Result<Vec<String>> {
        // Analyze input characteristics
        let characteristics = self.pattern_selector.analyze_input(&context.input_text);
        debug!("Input analysis: {:?}", characteristics);

        // Build pattern list description
        let patterns_desc = self.build_pattern_description();

        // Build context description
        let context_desc = if context.context.is_empty() {
            "None".to_string()
        } else {
            serde_json::to_string_pretty(&context.context)
                .unwrap_or_else(|_| "Unable to serialize context".to_string())
        };

        // Add input analysis to context
        let analysis_desc = format!(
            "Input Analysis:\n- Complexity: {:.1}/10\n- Estimated Steps: {}\n- Quality Critical: {}\n- Requires Explanation: {}\n- Iterative Nature: {}\n- Needs Planning: {}",
            characteristics.complexity,
            characteristics.estimated_steps,
            characteristics.quality_critical,
            characteristics.requires_explanation,
            characteristics.iterative_nature,
            characteristics.needs_planning
        );

        // Create prompt
        let enhanced_context = format!("{}\n\n{}", analysis_desc, context_desc);
        let prompt = PATTERN_SELECTION_PROMPT
            .replace("{patterns}", &patterns_desc)
            .replace("{input}", &context.input_text)
            .replace("{context}", &enhanced_context);

        // Call LLM
        let messages = vec![Message::human(prompt)];

        let request = ChatRequest::new(messages);

        let response = self.llm.chat(request).await.map_err(|e| {
            OrchestratorError::General(format!("LLM routing failed: {}", e))
        })?;

        // Parse response
        let response_text = response.message.text().unwrap_or("");
        let pattern_id = self.parse_llm_response(response_text)?;

        // Validate pattern exists
        if self.is_valid_pattern(&pattern_id) {
            info!("LLM selected pattern '{}' (complexity: {:.1})", pattern_id, characteristics.complexity);
            Ok(vec![pattern_id])
        } else {
            warn!("LLM selected invalid pattern: {}", pattern_id);
            Ok(vec![])
        }
    }

    /// Fallback to rule-based routing
    fn fallback_to_rules(&self, context: &EvaluationContext) -> Result<Vec<String>> {
        debug!("Using rule-based fallback routing");
        self.fallback_router.route(context)
    }

    /// Build pattern description for LLM
    fn build_pattern_description(&self) -> String {
        // Get available patterns from config
        let mut descriptions = Vec::new();

        // Add patterns from registry allowlist
        for pattern_id in &self.config.registry.allow {
            descriptions.push(format!("- {}: General purpose pattern", pattern_id));
        }

        // Add default patterns if not already in list
        for pattern_id in &self.config.settings.route_policy.default {
            if !descriptions.iter().any(|d| d.contains(pattern_id)) {
                descriptions.push(format!("- {}: Default pattern", pattern_id));
            }
        }

        if descriptions.is_empty() {
            "- react_1: Default reactive pattern".to_string()
        } else {
            descriptions.join("\n")
        }
    }

    /// Parse LLM response to extract pattern ID
    fn parse_llm_response(&self, response: &str) -> Result<String> {
        // Clean up response (remove whitespace, quotes, etc.)
        let cleaned = response.trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_lowercase();

        // Extract first line if multiline
        let pattern_id = cleaned.lines().next().unwrap_or("").trim();

        if pattern_id.is_empty() {
            return Err(OrchestratorError::General(
                "LLM returned empty pattern ID".to_string(),
            ));
        }

        Ok(pattern_id.to_string())
    }

    /// Check if pattern ID is valid
    fn is_valid_pattern(&self, pattern_id: &str) -> bool {
        // Check against registry allowlist
        if !self.config.registry.allow.is_empty() {
            if self.config.registry.allow.contains(&pattern_id.to_string()) {
                return true;
            }
        }

        // Check against denylist
        if self.config.registry.deny.contains(&pattern_id.to_string()) {
            return false;
        }

        // Check against default patterns
        self.config.settings.route_policy.default.contains(&pattern_id.to_string())
    }

    /// Get router configuration
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GuardConfig, RegistryConfig, RoutePolicyConfig, RouterSettings, RouteRule};
    use async_trait::async_trait;
    use langgraph_core::error::{GraphError, Result as GraphResult};
    use langgraph_core::llm;

    // Mock ChatModel for testing
    #[derive(Clone)]
    struct MockChatModel {
        response: String,
    }

    #[async_trait]
    impl ChatModel for MockChatModel {
        async fn chat(&self, _request: ChatRequest) -> GraphResult<llm::ChatResponse> {
            Ok(llm::ChatResponse {
                message: Message::ai(self.response.clone()),
                reasoning: None,
                usage: None,
                metadata: Default::default(),
            })
        }

        async fn stream(&self, _request: ChatRequest) -> GraphResult<llm::ChatStreamResponse> {
            Err(GraphError::Validation("Stream not implemented for mock".to_string()))
        }

        fn clone_box(&self) -> Box<dyn ChatModel> {
            Box::new(self.clone())
        }
    }

    fn create_test_config() -> RouterConfig {
        RouterConfig {
            id: "test_router".to_string(),
            description: Some("Test router".to_string()),
            registry: RegistryConfig {
                allow: vec!["react_1".to_string(), "plan_execute".to_string()],
                deny: vec![],
            },
            settings: RouterSettings {
                route_policy: RoutePolicyConfig {
                    rules: vec![],
                    default: vec!["react_1".to_string()],
                },
                termination: None,
                guards: Some(GuardConfig {
                    enforce_registry: true,
                    fallback_to_default: true,
                    max_routing_attempts: 10,
                }),
            },
        }
    }

    #[tokio::test]
    async fn test_llm_router_success() {
        let mock_llm = Arc::new(MockChatModel {
            response: "react_1".to_string(),
        });
        let config = create_test_config();
        let router = LlmRouter::new(mock_llm, config);

        let context = EvaluationContext::new("Test input");
        let patterns = router.route(&context).await.unwrap();

        assert_eq!(patterns, vec!["react_1"]);
    }

    #[tokio::test]
    async fn test_llm_router_fallback() {
        // Mock LLM that returns invalid pattern
        let mock_llm = Arc::new(MockChatModel {
            response: "invalid_pattern".to_string(),
        });
        let config = create_test_config();
        let router = LlmRouter::new(mock_llm, config);

        let context = EvaluationContext::new("Test input");
        let patterns = router.route(&context).await.unwrap();

        // Should fallback to default
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_parse_llm_response() {
        let config = create_test_config();
        let mock_llm = Arc::new(MockChatModel {
            response: "test".to_string(),
        });
        let router = LlmRouter::new(mock_llm, config);

        // Test clean response
        assert_eq!(router.parse_llm_response("react_1").unwrap(), "react_1");

        // Test with quotes
        assert_eq!(router.parse_llm_response("\"react_1\"").unwrap(), "react_1");

        // Test with whitespace
        assert_eq!(router.parse_llm_response("  react_1  \n").unwrap(), "react_1");
    }

    #[test]
    fn test_is_valid_pattern() {
        let config = create_test_config();
        let mock_llm = Arc::new(MockChatModel {
            response: "test".to_string(),
        });
        let router = LlmRouter::new(mock_llm, config);

        assert!(router.is_valid_pattern("react_1"));
        assert!(router.is_valid_pattern("plan_execute"));
        assert!(!router.is_valid_pattern("invalid_pattern"));
    }
}

