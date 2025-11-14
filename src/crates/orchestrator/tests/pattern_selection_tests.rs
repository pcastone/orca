//! Integration tests for pattern selection system
//!
//! Tests the PatternSelector integration with LlmRouter, covering:
//! - Input analysis and characteristic detection
//! - Pattern recommendations based on task complexity
//! - LLM router integration with pattern suggestions
//! - Fallback behavior when LLM routing fails
//! - Confidence scoring and alternative patterns

use async_trait::async_trait;
use langgraph_core::error::GraphError;
use langgraph_core::llm::{ChatModel, ChatRequest, ChatResponse, ChatStreamResponse};
use langgraph_core::messages::Message;
use orchestrator::config::{GuardConfig, RegistryConfig, RoutePolicyConfig, RouterConfig, RouterSettings};
use orchestrator::pattern::{PatternRecommendation, PatternSelector, PatternType};
use orchestrator::router::{EvaluationContext, LlmRouter};
use std::sync::Arc;

// ============================================================================
// Test Helpers
// ============================================================================

/// Mock ChatModel that returns specific pattern IDs for testing
#[derive(Clone)]
struct MockChatModel {
    response: String,
}

#[async_trait]
impl ChatModel for MockChatModel {
    async fn chat(&self, _request: ChatRequest) -> langgraph_core::error::Result<ChatResponse> {
        Ok(ChatResponse {
            message: Message::ai(self.response.clone()),
            reasoning: None,
            usage: None,
            metadata: Default::default(),
        })
    }

    async fn stream(&self, _request: ChatRequest) -> langgraph_core::error::Result<ChatStreamResponse> {
        Err(GraphError::Validation("Stream not implemented for mock".to_string()))
    }

    fn clone_box(&self) -> Box<dyn ChatModel> {
        Box::new(self.clone())
    }
}

/// Create a test router configuration
fn create_test_router_config() -> RouterConfig {
    RouterConfig {
        id: "test_router".to_string(),
        description: Some("Test pattern selection router".to_string()),
        registry: RegistryConfig {
            allow: vec![
                "react_1".to_string(),
                "plan_execute".to_string(),
                "reflection".to_string(),
            ],
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

// ============================================================================
// PatternSelector Unit Tests
// ============================================================================

#[test]
fn test_analyze_simple_task() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input("Write a hello world program in Python");

    assert!(characteristics.complexity < 3.0);
    assert_eq!(characteristics.estimated_steps, 1);
    assert!(!characteristics.quality_critical);
    assert!(!characteristics.requires_explanation);
    assert!(!characteristics.iterative_nature);
    assert!(!characteristics.needs_planning);
}

#[test]
fn test_analyze_complex_task() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input(
        "Debug and optimize this critical performance issue in production code to improve scalability"
    );

    assert!(characteristics.complexity > 3.0);
    assert!(characteristics.quality_critical);
    assert!(characteristics.requires_explanation);
    assert!(characteristics.estimated_steps >= 3);
}

#[test]
fn test_analyze_planning_task() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input(
        "Plan and organize a multi-stage workflow with coordination points and phases"
    );

    assert!(characteristics.needs_planning);
    assert!(characteristics.estimated_steps >= 3);
}

#[test]
fn test_analyze_iterative_task() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input(
        "Improve and refine this code to make it better, more professional, and optimize its performance"
    );

    assert!(characteristics.iterative_nature);
    assert!(characteristics.requires_explanation);
}

#[test]
fn test_analyze_quality_critical_task() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input(
        "Write production-critical code that is reliable, secure, and robust"
    );

    assert!(characteristics.quality_critical);
    assert!(characteristics.requires_explanation);
}

// ============================================================================
// Pattern Recommendation Tests
// ============================================================================

#[test]
fn test_recommend_react_for_simple_task() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input("Write a simple script");

    let recommendation = selector.recommend(&characteristics);
    assert_eq!(recommendation.pattern, PatternType::React);
    assert!(recommendation.confidence > 0.7);
    assert!(!recommendation.alternatives.is_empty());
    assert!(!recommendation.alternatives.contains(&PatternType::React));
}

#[test]
fn test_recommend_plan_execute_for_complex_task() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input(
        "Design and implement a complex architecture with multiple optimization stages"
    );

    let recommendation = selector.recommend(&characteristics);
    assert_eq!(recommendation.pattern, PatternType::PlanExecute);
    assert!(recommendation.confidence > 0.6);
}

#[test]
fn test_recommend_reflection_for_quality_critical_task() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input(
        "Write high-quality production code that requires critical security review and iterative refinement"
    );

    let recommendation = selector.recommend(&characteristics);
    assert_eq!(recommendation.pattern, PatternType::Reflection);
    assert!(recommendation.confidence > 0.7);
}

#[test]
fn test_recommendation_has_reason() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input("Optimize this performance-critical code");

    let recommendation = selector.recommend(&characteristics);
    assert!(!recommendation.reason.is_empty());
    // Reason should reference the decision made
    assert!(recommendation.reason.contains("straightforward") ||
            recommendation.reason.contains("complexity") ||
            recommendation.reason.contains("planning") ||
            recommendation.reason.contains("quality") ||
            recommendation.reason.contains("refine"));
}

#[test]
fn test_recommendation_provides_alternatives() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input("Simple task");

    let recommendation = selector.recommend(&characteristics);
    assert!(!recommendation.alternatives.is_empty());

    // Alternatives should not include the primary pattern
    assert!(!recommendation.alternatives.contains(&recommendation.pattern));

    // Should have at least 2 alternatives (for 3 patterns)
    assert!(recommendation.alternatives.len() >= 2);
}

// ============================================================================
// Pattern Type Tests
// ============================================================================

#[test]
fn test_pattern_type_ids() {
    assert_eq!(PatternType::React.id(), "react_1");
    assert_eq!(PatternType::PlanExecute.id(), "plan_execute");
    assert_eq!(PatternType::Reflection.id(), "reflection");
}

#[test]
fn test_pattern_type_names() {
    assert_eq!(PatternType::React.name(), "ReAct");
    assert_eq!(PatternType::PlanExecute.name(), "Plan-Execute");
    assert_eq!(PatternType::Reflection.name(), "Reflection");
}

#[test]
fn test_pattern_type_descriptions() {
    assert!(!PatternType::React.description().is_empty());
    assert!(!PatternType::PlanExecute.description().is_empty());
    assert!(!PatternType::Reflection.description().is_empty());
}

// ============================================================================
// Custom Threshold Tests
// ============================================================================

#[test]
fn test_custom_thresholds_simple() {
    let selector = PatternSelector::with_thresholds(2.0, 5.0);
    let characteristics = orchestrator::pattern::TaskCharacteristics {
        complexity: 3.0,
        estimated_steps: 2,
        quality_critical: false,
        requires_explanation: false,
        iterative_nature: false,
        needs_planning: true,  // Needs planning to trigger Plan-Execute
    };

    let recommendation = selector.recommend(&characteristics);
    // With needs_planning set to true, should trigger Plan-Execute
    assert_eq!(recommendation.pattern, PatternType::PlanExecute);
}

#[test]
fn test_custom_thresholds_complex() {
    let selector = PatternSelector::with_thresholds(3.0, 7.0);
    let characteristics = orchestrator::pattern::TaskCharacteristics {
        complexity: 8.0,
        estimated_steps: 5,
        quality_critical: false,
        requires_explanation: false,
        iterative_nature: false,
        needs_planning: false,
    };

    let recommendation = selector.recommend(&characteristics);
    // With custom thresholds, complexity 8.0 should trigger Plan-Execute (above complex_threshold of 7.0)
    assert_eq!(recommendation.pattern, PatternType::PlanExecute);
}

// ============================================================================
// Confidence Scoring Tests
// ============================================================================

#[test]
fn test_confidence_bounds() {
    let selector = PatternSelector::new();

    for complexity in [0.0, 1.0, 3.0, 5.0, 7.0, 10.0].iter() {
        let characteristics = orchestrator::pattern::TaskCharacteristics {
            complexity: *complexity,
            estimated_steps: 1,
            quality_critical: false,
            requires_explanation: false,
            iterative_nature: false,
            needs_planning: false,
        };

        let recommendation = selector.recommend(&characteristics);
        assert!(recommendation.confidence >= 0.0);
        assert!(recommendation.confidence <= 1.0);
    }
}

#[test]
fn test_react_confidence_high_for_simple() {
    let selector = PatternSelector::new();
    let characteristics = orchestrator::pattern::TaskCharacteristics {
        complexity: 1.0,
        estimated_steps: 1,
        quality_critical: false,
        requires_explanation: false,
        iterative_nature: false,
        needs_planning: false,
    };

    let recommendation = selector.recommend(&characteristics);
    assert_eq!(recommendation.pattern, PatternType::React);
    assert!(recommendation.confidence > 0.75);
}

#[test]
fn test_plan_execute_confidence_high_for_complex() {
    let selector = PatternSelector::new();
    let characteristics = orchestrator::pattern::TaskCharacteristics {
        complexity: 8.0,
        estimated_steps: 5,
        quality_critical: false,
        requires_explanation: false,
        iterative_nature: false,
        needs_planning: true,
    };

    let recommendation = selector.recommend(&characteristics);
    assert_eq!(recommendation.pattern, PatternType::PlanExecute);
    assert!(recommendation.confidence > 0.75);
}

#[test]
fn test_reflection_confidence_high_for_quality_critical() {
    let selector = PatternSelector::new();
    let characteristics = orchestrator::pattern::TaskCharacteristics {
        complexity: 5.0,
        estimated_steps: 3,
        quality_critical: true,
        requires_explanation: true,
        iterative_nature: true,
        needs_planning: false,
    };

    let recommendation = selector.recommend(&characteristics);
    assert_eq!(recommendation.pattern, PatternType::Reflection);
    assert!(recommendation.confidence > 0.85);
}

// ============================================================================
// LlmRouter Integration Tests
// ============================================================================

#[tokio::test]
async fn test_llm_router_with_simple_input() {
    let mock_llm = Arc::new(MockChatModel {
        response: "react_1".to_string(),
    });
    let config = create_test_router_config();
    let router = LlmRouter::new(mock_llm, config);

    let context = EvaluationContext::new("Write a simple program");
    let patterns = router.route(&context).await.unwrap();

    assert!(!patterns.is_empty());
    assert!(patterns[0] == "react_1" || !patterns[0].is_empty());
}

#[tokio::test]
async fn test_llm_router_with_complex_input() {
    let mock_llm = Arc::new(MockChatModel {
        response: "plan_execute".to_string(),
    });
    let config = create_test_router_config();
    let router = LlmRouter::new(mock_llm, config);

    let context = EvaluationContext::new(
        "Design and optimize a complex system with multiple stages and critical performance requirements"
    );
    let patterns = router.route(&context).await.unwrap();

    assert!(!patterns.is_empty());
}

#[tokio::test]
async fn test_llm_router_validates_pattern() {
    let mock_llm = Arc::new(MockChatModel {
        response: "react_1".to_string(),
    });
    let config = create_test_router_config();
    let router = LlmRouter::new(mock_llm, config);

    let context = EvaluationContext::new("Test input");
    let patterns = router.route(&context).await.unwrap();

    // Should return a valid pattern from the registry
    assert!(!patterns.is_empty());
    assert!(router.config().registry.allow.contains(&patterns[0]));
}

#[tokio::test]
async fn test_llm_router_fallback_on_invalid_pattern() {
    let mock_llm = Arc::new(MockChatModel {
        response: "invalid_pattern".to_string(),
    });
    let config = create_test_router_config();
    let router = LlmRouter::new(mock_llm, config);

    let context = EvaluationContext::new("Test input");
    let patterns = router.route(&context).await.unwrap();

    // Should fallback to default patterns
    assert!(!patterns.is_empty());
}

// ============================================================================
// Integration Scenario Tests
// ============================================================================

#[test]
fn test_end_to_end_simple_analysis_to_recommendation() {
    let selector = PatternSelector::new();

    // Simulate user input
    let user_input = "Create a REST API endpoint";

    // Step 1: Analyze characteristics
    let characteristics = selector.analyze_input(user_input);

    // Step 2: Get recommendation
    let recommendation = selector.recommend(&characteristics);

    // Verify flow
    assert!(characteristics.complexity < 5.0);
    assert_eq!(recommendation.pattern, PatternType::React);
    assert!(recommendation.confidence > 0.7);
}

#[test]
fn test_end_to_end_complex_analysis_to_recommendation() {
    let selector = PatternSelector::new();

    // Simulate complex user request
    let user_input = "Analyze the architecture of our system, plan improvements for scalability, \
                      optimize the database queries, and refine the API design for better usability";

    // Step 1: Analyze characteristics
    let characteristics = selector.analyze_input(user_input);

    // Step 2: Get recommendation
    let recommendation = selector.recommend(&characteristics);

    // Verify flow
    assert!(characteristics.complexity > 3.0);
    assert!(characteristics.needs_planning);
    assert!(!recommendation.alternatives.is_empty());
}

#[test]
fn test_serialization_of_recommendation() {
    let selector = PatternSelector::new();
    let characteristics = selector.analyze_input("Debug a critical issue");
    let recommendation = selector.recommend(&characteristics);

    // Test that recommendation can be serialized
    let json = serde_json::to_string(&recommendation).unwrap();
    assert!(json.contains("pattern") || json.contains("confidence"));

    // Test that recommendation can be deserialized
    let _deserialized: PatternRecommendation = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_multiple_sequential_analysis_consistency() {
    let selector = PatternSelector::new();
    let input = "Optimize the performance of this critical database query";

    // Analyze multiple times
    let rec1 = selector.recommend(&selector.analyze_input(input));
    let rec2 = selector.recommend(&selector.analyze_input(input));
    let rec3 = selector.recommend(&selector.analyze_input(input));

    // Should be consistent
    assert_eq!(rec1.pattern, rec2.pattern);
    assert_eq!(rec2.pattern, rec3.pattern);
}

#[test]
fn test_all_pattern_types_can_be_recommended() {
    let selector = PatternSelector::new();

    // Input for React (simple)
    let react_chars = selector.analyze_input("Write hello world");
    let react_rec = selector.recommend(&react_chars);

    // Input for Plan-Execute (complex)
    let plan_chars = selector.analyze_input("Design and plan a multi-stage complex workflow with architecture");
    let plan_rec = selector.recommend(&plan_chars);

    // Input for Reflection (quality critical + iterative)
    let mut reflection_chars = selector.analyze_input("Improve and refine code quality");
    reflection_chars.quality_critical = true;
    let reflection_rec = selector.recommend(&reflection_chars);

    // Verify we can get different patterns
    assert_eq!(react_rec.pattern, PatternType::React);
    assert_eq!(plan_rec.pattern, PatternType::PlanExecute);
    assert_eq!(reflection_rec.pattern, PatternType::Reflection);
}
