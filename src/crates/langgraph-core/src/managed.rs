//! Managed values for graph execution
//!
//! Managed values are special values that are automatically injected into the state
//! during graph execution. They provide execution context like remaining steps,
//! current step number, and whether this is the last step.
//!
//! # Example
//!
//! ```rust,ignore
//! use langgraph_core::managed::{ManagedValueType, ExecutionContext};
//!
//! let context = ExecutionContext::new(10); // max 10 steps
//! context.increment_step();
//!
//! let remaining = context.get_managed_value(ManagedValueType::RemainingSteps);
//! let is_last = context.get_managed_value(ManagedValueType::IsLastStep);
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, RwLock};

/// Types of managed values that can be injected into state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManagedValueType {
    /// Number of steps remaining before max_steps is reached
    RemainingSteps,

    /// Boolean indicating if this is the last step (remaining_steps == 1)
    IsLastStep,

    /// Current step number (0-indexed)
    CurrentStep,
}

impl ManagedValueType {
    /// Get the state key for this managed value
    pub fn state_key(&self) -> &'static str {
        match self {
            ManagedValueType::RemainingSteps => "__remaining_steps__",
            ManagedValueType::IsLastStep => "__is_last_step__",
            ManagedValueType::CurrentStep => "__current_step__",
        }
    }
}

/// Execution context that tracks managed values
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Current step number (0-indexed)
    current_step: Arc<RwLock<usize>>,

    /// Maximum number of steps allowed
    max_steps: usize,
}

impl ExecutionContext {
    /// Create a new execution context with the given max steps
    pub fn new(max_steps: usize) -> Self {
        Self {
            current_step: Arc::new(RwLock::new(0)),
            max_steps,
        }
    }

    /// Get the current step number
    pub fn current_step(&self) -> usize {
        *self.current_step.read().unwrap()
    }

    /// Get the maximum number of steps
    pub fn max_steps(&self) -> usize {
        self.max_steps
    }

    /// Get remaining steps (max_steps - current_step)
    pub fn remaining_steps(&self) -> usize {
        self.max_steps.saturating_sub(self.current_step())
    }

    /// Check if this is the last step
    pub fn is_last_step(&self) -> bool {
        self.remaining_steps() <= 1
    }

    /// Increment the step counter
    pub fn increment_step(&self) {
        let mut step = self.current_step.write().unwrap();
        *step += 1;
    }

    /// Set the step counter to a specific value
    pub(crate) fn set_current_step(&self, value: usize) {
        let mut step = self.current_step.write().unwrap();
        *step = value;
    }

    /// Reset the step counter
    pub fn reset(&self) {
        let mut step = self.current_step.write().unwrap();
        *step = 0;
    }

    /// Get a managed value by type
    pub fn get_managed_value(&self, value_type: ManagedValueType) -> Value {
        match value_type {
            ManagedValueType::RemainingSteps => {
                serde_json::json!(self.remaining_steps())
            }
            ManagedValueType::IsLastStep => {
                serde_json::json!(self.is_last_step())
            }
            ManagedValueType::CurrentStep => {
                serde_json::json!(self.current_step())
            }
        }
    }

    /// Inject managed values into state
    pub fn inject_managed_values(&self, state: &mut Value) -> Result<(), String> {
        if let Some(obj) = state.as_object_mut() {
            obj.insert(
                ManagedValueType::RemainingSteps.state_key().to_string(),
                self.get_managed_value(ManagedValueType::RemainingSteps),
            );
            obj.insert(
                ManagedValueType::IsLastStep.state_key().to_string(),
                self.get_managed_value(ManagedValueType::IsLastStep),
            );
            obj.insert(
                ManagedValueType::CurrentStep.state_key().to_string(),
                self.get_managed_value(ManagedValueType::CurrentStep),
            );
            Ok(())
        } else {
            Err("State must be a JSON object to inject managed values".to_string())
        }
    }

    /// Remove managed values from state (for cleanup)
    pub fn remove_managed_values(&self, state: &mut Value) {
        if let Some(obj) = state.as_object_mut() {
            obj.remove(ManagedValueType::RemainingSteps.state_key());
            obj.remove(ManagedValueType::IsLastStep.state_key());
            obj.remove(ManagedValueType::CurrentStep.state_key());
        }
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new(25) // Default to 25 steps like Python
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_basic() {
        let context = ExecutionContext::new(10);

        assert_eq!(context.current_step(), 0);
        assert_eq!(context.max_steps(), 10);
        assert_eq!(context.remaining_steps(), 10);
        assert!(!context.is_last_step());
    }

    #[test]
    fn test_increment_step() {
        let context = ExecutionContext::new(5);

        context.increment_step();
        assert_eq!(context.current_step(), 1);
        assert_eq!(context.remaining_steps(), 4);

        context.increment_step();
        assert_eq!(context.current_step(), 2);
        assert_eq!(context.remaining_steps(), 3);
    }

    #[test]
    fn test_is_last_step() {
        let context = ExecutionContext::new(3);

        assert!(!context.is_last_step()); // step 0, remaining 3

        context.increment_step();
        assert!(!context.is_last_step()); // step 1, remaining 2

        context.increment_step();
        assert!(context.is_last_step()); // step 2, remaining 1
    }

    #[test]
    fn test_reset() {
        let context = ExecutionContext::new(10);

        context.increment_step();
        context.increment_step();
        assert_eq!(context.current_step(), 2);

        context.reset();
        assert_eq!(context.current_step(), 0);
    }

    #[test]
    fn test_get_managed_value() {
        let context = ExecutionContext::new(5);
        context.increment_step();
        context.increment_step(); // step 2

        let remaining = context.get_managed_value(ManagedValueType::RemainingSteps);
        assert_eq!(remaining, serde_json::json!(3));

        let is_last = context.get_managed_value(ManagedValueType::IsLastStep);
        assert_eq!(is_last, serde_json::json!(false));

        let current = context.get_managed_value(ManagedValueType::CurrentStep);
        assert_eq!(current, serde_json::json!(2));
    }

    #[test]
    fn test_inject_managed_values() {
        let context = ExecutionContext::new(10);
        context.increment_step();

        let mut state = serde_json::json!({
            "user_data": "test"
        });

        context.inject_managed_values(&mut state).unwrap();

        assert_eq!(state["__remaining_steps__"], 9);
        assert_eq!(state["__is_last_step__"], false);
        assert_eq!(state["__current_step__"], 1);
        assert_eq!(state["user_data"], "test");
    }

    #[test]
    fn test_remove_managed_values() {
        let context = ExecutionContext::new(10);

        let mut state = serde_json::json!({
            "user_data": "test",
            "__remaining_steps__": 5,
            "__is_last_step__": false,
            "__current_step__": 3
        });

        context.remove_managed_values(&mut state);

        assert!(state.get("__remaining_steps__").is_none());
        assert!(state.get("__is_last_step__").is_none());
        assert!(state.get("__current_step__").is_none());
        assert_eq!(state["user_data"], "test");
    }

    #[test]
    fn test_inject_non_object_fails() {
        let context = ExecutionContext::new(10);
        let mut state = serde_json::json!("not an object");

        let result = context.inject_managed_values(&mut state);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_context() {
        let context = ExecutionContext::default();
        assert_eq!(context.max_steps(), 25);
    }

    #[test]
    fn test_saturating_sub_when_exceeding_max() {
        let context = ExecutionContext::new(3);

        // Increment beyond max_steps
        for _ in 0..5 {
            context.increment_step();
        }

        // Should saturate at 0, not underflow
        assert_eq!(context.remaining_steps(), 0);
    }
}
