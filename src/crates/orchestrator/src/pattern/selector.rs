//! Pattern Selection System
//!
//! Provides intelligent pattern selection based on task characteristics.
//! Analyzes input to recommend the best agent pattern:
//! - ReAct: Fast, simple reasoning
//! - Plan-Execute: Structured planning
//! - Reflection: Iterative refinement

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Available agent patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternType {
    /// Fast reactive pattern - good for simple tasks
    #[serde(rename = "react")]
    React,
    /// Planning pattern - good for multi-step tasks
    #[serde(rename = "plan_execute")]
    PlanExecute,
    /// Reflection pattern - good for quality-critical output
    #[serde(rename = "reflection")]
    Reflection,
}

impl PatternType {
    /// Get pattern ID string for routing
    pub fn id(&self) -> String {
        match self {
            PatternType::React => "react_1".to_string(),
            PatternType::PlanExecute => "plan_execute".to_string(),
            PatternType::Reflection => "reflection".to_string(),
        }
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            PatternType::React => "ReAct",
            PatternType::PlanExecute => "Plan-Execute",
            PatternType::Reflection => "Reflection",
        }
    }

    /// Get description of the pattern
    pub fn description(&self) -> &'static str {
        match self {
            PatternType::React => "Fast, simple reasoning with tool use",
            PatternType::PlanExecute => "Explicit planning followed by execution",
            PatternType::Reflection => "Generate, critique, and refine approach",
        }
    }
}

/// Characteristics of a task/input that influence pattern selection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskCharacteristics {
    /// Estimated complexity (0-10)
    pub complexity: f32,
    /// Number of steps likely required
    pub estimated_steps: usize,
    /// Whether quality/accuracy is critical
    pub quality_critical: bool,
    /// Whether reasoning needs to be explainable
    pub requires_explanation: bool,
    /// Whether iterative refinement would help
    pub iterative_nature: bool,
    /// Whether planning would be beneficial
    pub needs_planning: bool,
}

/// Pattern recommendation with reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRecommendation {
    /// Recommended pattern
    pub pattern: PatternType,
    /// Confidence score (0-1.0)
    pub confidence: f32,
    /// Reason for recommendation
    pub reason: String,
    /// Alternative patterns (in order of preference)
    pub alternatives: Vec<PatternType>,
}

/// Pattern selector for intelligent pattern choice
pub struct PatternSelector {
    /// Complexity thresholds
    simple_threshold: f32,
    complex_threshold: f32,
}

impl PatternSelector {
    /// Create a new pattern selector with default thresholds
    pub fn new() -> Self {
        Self {
            simple_threshold: 3.0,      // Complexity 0-3: simple
            complex_threshold: 7.0,     // Complexity 3-7: medium
            // Complexity 7-10: complex
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(simple_threshold: f32, complex_threshold: f32) -> Self {
        Self {
            simple_threshold,
            complex_threshold,
        }
    }

    /// Analyze task input and extract characteristics
    ///
    /// Uses heuristics to estimate task complexity and nature
    pub fn analyze_input(&self, input: &str) -> TaskCharacteristics {
        let mut characteristics = TaskCharacteristics::default();

        // Analyze input length and keywords
        let lower_input = input.to_lowercase();

        // Estimate complexity based on keywords
        let complex_words = [
            "debug", "optimize", "analyze", "improve", "enhance", "refactor",
            "design", "architecture", "performance", "scalability",
        ];

        let quality_words = [
            "quality", "production", "critical", "important", "robust",
            "reliable", "professional", "code review", "security",
        ];

        let planning_words = [
            "plan", "schedule", "organize", "structure", "coordinate",
            "sequence", "steps", "phases", "stages", "workflow",
        ];

        let iterative_words = [
            "improve", "refine", "iterate", "adjust", "enhance",
            "optimize", "better", "revise", "review",
        ];

        // Count keyword occurrences
        let complex_count = complex_words.iter().filter(|w| lower_input.contains(*w)).count() as f32;
        let quality_count = quality_words.iter().filter(|w| lower_input.contains(*w)).count() as f32;
        let planning_count = planning_words.iter().filter(|w| lower_input.contains(*w)).count() as f32;
        let iterative_count = iterative_words.iter().filter(|w| lower_input.contains(*w)).count() as f32;

        // Calculate complexity (0-10)
        let word_complexity = (complex_count * 1.5) + (quality_count * 0.8) + (planning_count * 0.6);
        let length_complexity = ((input.len() as f32).log2() / 10.0).min(3.0);
        characteristics.complexity = (word_complexity + length_complexity).min(10.0);

        // Estimate steps
        characteristics.estimated_steps = if characteristics.complexity < 3.0 {
            1
        } else if characteristics.complexity < 6.0 {
            3
        } else {
            5
        };

        // Set flags
        characteristics.quality_critical = quality_count > 0.0;
        characteristics.requires_explanation = complex_count > 1.0 || quality_count > 0.0;
        characteristics.iterative_nature = iterative_count > 0.0;
        characteristics.needs_planning = planning_count > 0.0 || characteristics.estimated_steps > 2;

        debug!("Analyzed input characteristics: {:?}", characteristics);
        characteristics
    }

    /// Recommend a pattern based on characteristics
    pub fn recommend(&self, characteristics: &TaskCharacteristics) -> PatternRecommendation {
        let pattern = self.select_pattern(characteristics);
        let confidence = self.calculate_confidence(characteristics, pattern);
        let reason = self.build_reasoning(characteristics, pattern);
        let alternatives = self.find_alternatives(characteristics, pattern);

        let recommendation = PatternRecommendation {
            pattern,
            confidence,
            reason,
            alternatives,
        };

        info!(
            "Pattern recommendation: {} (confidence: {:.2})",
            pattern.name(),
            confidence
        );
        recommendation
    }

    /// Select the best pattern for the characteristics
    fn select_pattern(&self, characteristics: &TaskCharacteristics) -> PatternType {
        // Reflection: Quality-critical or needs refinement
        if characteristics.quality_critical && characteristics.iterative_nature {
            return PatternType::Reflection;
        }

        // Plan-Execute: Needs planning or complex
        if characteristics.needs_planning || characteristics.complexity >= self.complex_threshold {
            return PatternType::PlanExecute;
        }

        // ReAct: Default for simple tasks
        PatternType::React
    }

    /// Calculate confidence in recommendation
    fn calculate_confidence(&self, characteristics: &TaskCharacteristics, pattern: PatternType) -> f32 {
        let mut score: f32 = 0.5; // Base confidence

        match pattern {
            PatternType::React => {
                // High confidence for simple, straightforward tasks
                if characteristics.complexity < self.simple_threshold {
                    score += 0.35;
                } else if characteristics.complexity < self.complex_threshold {
                    score += 0.15;
                } else {
                    score -= 0.2; // Lower confidence for complex tasks
                }
            }
            PatternType::PlanExecute => {
                // High confidence for complex tasks that need planning
                if characteristics.needs_planning {
                    score += 0.35;
                }
                if characteristics.complexity >= self.complex_threshold {
                    score += 0.2;
                }
                if characteristics.estimated_steps >= 3 {
                    score += 0.15;
                }
            }
            PatternType::Reflection => {
                // High confidence for quality-critical tasks
                if characteristics.quality_critical {
                    score += 0.35;
                }
                if characteristics.iterative_nature {
                    score += 0.2;
                }
                if characteristics.requires_explanation {
                    score += 0.15;
                }
            }
        }

        score.max(0.1).min(1.0)
    }

    /// Build human-readable reasoning
    fn build_reasoning(
        &self,
        characteristics: &TaskCharacteristics,
        pattern: PatternType,
    ) -> String {
        match pattern {
            PatternType::React => {
                format!(
                    "Simple, straightforward task (complexity: {:.1}). \
                    ReAct is fast and efficient for direct reasoning.",
                    characteristics.complexity
                )
            }
            PatternType::PlanExecute => {
                let mut reasons = vec![];
                if characteristics.needs_planning {
                    reasons.push("explicit planning needed");
                }
                if characteristics.complexity >= self.complex_threshold {
                    reasons.push("task is complex");
                }
                if characteristics.estimated_steps > 2 {
                    reasons.push("multiple steps required");
                }

                format!(
                    "Plan-Execute recommended because: {}",
                    reasons.join(", ")
                )
            }
            PatternType::Reflection => {
                let mut reasons = vec![];
                if characteristics.quality_critical {
                    reasons.push("quality is critical");
                }
                if characteristics.iterative_nature {
                    reasons.push("iterative refinement helpful");
                }
                if characteristics.requires_explanation {
                    reasons.push("explanation required");
                }

                format!(
                    "Reflection pattern for high-quality output: {}",
                    reasons.join(", ")
                )
            }
        }
    }

    /// Find alternative patterns
    fn find_alternatives(
        &self,
        _characteristics: &TaskCharacteristics,
        primary: PatternType,
    ) -> Vec<PatternType> {
        match primary {
            PatternType::React => vec![PatternType::PlanExecute, PatternType::Reflection],
            PatternType::PlanExecute => vec![PatternType::React, PatternType::Reflection],
            PatternType::Reflection => vec![PatternType::PlanExecute, PatternType::React],
        }
    }
}

impl Default for PatternSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_type_ids() {
        assert_eq!(PatternType::React.id(), "react_1");
        assert_eq!(PatternType::PlanExecute.id(), "plan_execute");
        assert_eq!(PatternType::Reflection.id(), "reflection");
    }

    #[test]
    fn test_analyze_simple_input() {
        let selector = PatternSelector::new();
        let characteristics = selector.analyze_input("Write a hello world program");

        assert!(characteristics.complexity < 3.0);
        assert_eq!(characteristics.estimated_steps, 1);
        assert!(!characteristics.quality_critical);
    }

    #[test]
    fn test_analyze_complex_input() {
        let selector = PatternSelector::new();
        let characteristics = selector.analyze_input(
            "Debug and optimize this critical performance issue in production code",
        );

        assert!(characteristics.complexity > 3.0);
        assert!(characteristics.quality_critical);
        assert!(!characteristics.estimated_steps < 3);
    }

    #[test]
    fn test_analyze_planning_input() {
        let selector = PatternSelector::new();
        let characteristics = selector.analyze_input(
            "Plan and organize a workflow with multiple stages and coordination points",
        );

        assert!(characteristics.needs_planning);
        assert!(characteristics.estimated_steps >= 3);
    }

    #[test]
    fn test_analyze_iterative_input() {
        let selector = PatternSelector::new();
        let characteristics = selector.analyze_input(
            "Improve and refine this code to make it better and more professional",
        );

        assert!(characteristics.iterative_nature);
    }

    #[test]
    fn test_recommend_simple_task() {
        let selector = PatternSelector::new();
        let characteristics = TaskCharacteristics {
            complexity: 1.0,
            estimated_steps: 1,
            quality_critical: false,
            requires_explanation: false,
            iterative_nature: false,
            needs_planning: false,
        };

        let recommendation = selector.recommend(&characteristics);
        assert_eq!(recommendation.pattern, PatternType::React);
        assert!(recommendation.confidence > 0.7);
    }

    #[test]
    fn test_recommend_complex_task() {
        let selector = PatternSelector::new();
        let characteristics = TaskCharacteristics {
            complexity: 8.0,
            estimated_steps: 5,
            quality_critical: false,
            requires_explanation: true,
            iterative_nature: false,
            needs_planning: true,
        };

        let recommendation = selector.recommend(&characteristics);
        assert_eq!(recommendation.pattern, PatternType::PlanExecute);
        assert!(recommendation.confidence > 0.6);
    }

    #[test]
    fn test_recommend_quality_critical_task() {
        let selector = PatternSelector::new();
        let characteristics = TaskCharacteristics {
            complexity: 5.0,
            estimated_steps: 3,
            quality_critical: true,
            requires_explanation: true,
            iterative_nature: true,
            needs_planning: false,
        };

        let recommendation = selector.recommend(&characteristics);
        assert_eq!(recommendation.pattern, PatternType::Reflection);
        assert!(recommendation.confidence > 0.7);
    }

    #[test]
    fn test_alternatives_provided() {
        let selector = PatternSelector::new();
        let characteristics = TaskCharacteristics {
            complexity: 2.0,
            estimated_steps: 1,
            quality_critical: false,
            requires_explanation: false,
            iterative_nature: false,
            needs_planning: false,
        };

        let recommendation = selector.recommend(&characteristics);
        assert!(!recommendation.alternatives.is_empty());
        assert!(!recommendation.alternatives.contains(&recommendation.pattern));
    }

    #[test]
    fn test_confidence_bounds() {
        let selector = PatternSelector::new();

        for complexity in [0.0, 3.0, 5.0, 7.0, 10.0] {
            let characteristics = TaskCharacteristics {
                complexity,
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
    fn test_custom_thresholds() {
        let selector = PatternSelector::with_thresholds(2.0, 5.0);
        let characteristics = TaskCharacteristics {
            complexity: 3.0,
            estimated_steps: 2,
            quality_critical: false,
            requires_explanation: false,
            iterative_nature: false,
            needs_planning: false,
        };

        let recommendation = selector.recommend(&characteristics);
        // With custom thresholds, complexity 3.0 should trigger Plan-Execute
        assert_eq!(recommendation.pattern, PatternType::PlanExecute);
    }
}
