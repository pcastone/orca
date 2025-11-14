//! Router module for dynamic pattern selection
//!
//! Provides routing and supervision capabilities to select patterns dynamically
//! based on input, context, and rules.

pub mod evaluator;
pub mod llm_router;
pub mod supervisor;

pub use evaluator::{EvaluationContext, RuleEvaluator};
pub use llm_router::LlmRouter;
pub use supervisor::Router;
