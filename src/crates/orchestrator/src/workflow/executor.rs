//! Workflow executor for multi-step pattern orchestration
//!
//! Executes workflows defined by WorkflowConfig, managing state and transitions.

use crate::config::{StepCondition, StepTransition, WorkflowConfig, WorkflowState, WorkflowStatus, WorkflowStep};
use crate::{OrchestratorError, Result};
use serde_json::Value;

/// Workflow executor for running multi-step workflows
pub struct WorkflowExecutor {
    config: WorkflowConfig,
    state: WorkflowState,
}

impl WorkflowExecutor {
    /// Create a new workflow executor
    pub fn new(config: WorkflowConfig) -> Self {
        Self {
            config,
            state: WorkflowState::new(),
        }
    }

    /// Get the current workflow state
    pub fn state(&self) -> &WorkflowState {
        &self.state
    }

    /// Get mutable workflow state
    pub fn state_mut(&mut self) -> &mut WorkflowState {
        &mut self.state
    }

    /// Get the workflow configuration
    pub fn config(&self) -> &WorkflowConfig {
        &self.config
    }

    /// Get the current step
    pub fn current_step(&self) -> Option<&WorkflowStep> {
        self.config.steps.get(self.state.current_step)
    }

    /// Check if workflow is complete
    pub fn is_complete(&self) -> bool {
        matches!(
            self.state.status,
            WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled
        )
    }

    /// Start the workflow
    pub fn start(&mut self) -> Result<()> {
        if self.state.status != WorkflowStatus::Pending {
            return Err(OrchestratorError::General(
                "Workflow already started".to_string(),
            ));
        }

        self.state.status = WorkflowStatus::Running;
        Ok(())
    }

    /// Record a step result and determine next step
    ///
    /// Returns the name of the next step to execute, or None if workflow is complete
    pub fn record_step_result(&mut self, result: Value, success: bool) -> Result<Option<String>> {
        let current_step = self
            .current_step()
            .ok_or_else(|| OrchestratorError::General("No current step".to_string()))?
            .clone();

        // Record the result
        self.state.record_result(current_step.name.clone(), result.clone());

        // Check max steps
        if let Some(settings) = &self.config.settings {
            if self.state.steps_executed >= settings.max_total_steps {
                self.state.status = WorkflowStatus::Failed;
                return Err(OrchestratorError::General(format!(
                    "Exceeded max total steps: {}",
                    settings.max_total_steps
                )));
            }
        }

        // Determine next transition based on success/failure
        let transition = if success {
            current_step.on_success.as_ref()
        } else {
            current_step.on_failure.as_ref()
        };

        // If no transition specified, try to go to next step
        let next_step = if let Some(trans) = transition {
            self.evaluate_transition(trans, &result)?
        } else {
            // Auto-advance to next step
            let next_idx = self.state.current_step + 1;
            if next_idx < self.config.steps.len() {
                Some(self.config.steps[next_idx].name.clone())
            } else {
                None
            }
        };

        // Update current step index
        if let Some(ref next_name) = next_step {
            if let Some((idx, _)) = self
                .config
                .steps
                .iter()
                .enumerate()
                .find(|(_, s)| &s.name == next_name)
            {
                self.state.current_step = idx;
            } else {
                return Err(OrchestratorError::General(format!(
                    "Step not found: {}",
                    next_name
                )));
            }
        } else {
            // No next step - workflow complete
            self.state.status = WorkflowStatus::Completed;
        }

        Ok(next_step)
    }

    /// Evaluate a step transition to determine next step
    fn evaluate_transition(&self, transition: &StepTransition, result: &Value) -> Result<Option<String>> {
        match transition {
            StepTransition::Goto(step_name) => Ok(Some(step_name.clone())),
            StepTransition::End { end } => {
                if *end {
                    Ok(None)
                } else {
                    // Continue to next step
                    let next_idx = self.state.current_step + 1;
                    Ok(self.config.steps.get(next_idx).map(|s| s.name.clone()))
                }
            }
            StepTransition::Conditional {
                if_condition,
                then,
                else_transition,
            } => {
                // Evaluate condition
                if self.evaluate_condition_expr(if_condition, result)? {
                    self.evaluate_transition(then, result)
                } else if let Some(else_trans) = else_transition {
                    self.evaluate_transition(else_trans, result)
                } else {
                    // No else - continue to next step
                    let next_idx = self.state.current_step + 1;
                    Ok(self.config.steps.get(next_idx).map(|s| s.name.clone()))
                }
            }
        }
    }

    /// Evaluate a condition expression (simplified)
    fn evaluate_condition_expr(&self, expr: &str, result: &Value) -> Result<bool> {
        // Simple evaluation - check if result has a "success" field
        // TODO: Implement full expression evaluation
        if expr == "result.success" {
            if let Some(success) = result.get("success") {
                Ok(success.as_bool().unwrap_or(false))
            } else {
                Ok(false)
            }
        } else {
            // Default to true for now
            Ok(true)
        }
    }

    /// Check if a step should execute based on its condition
    pub fn should_execute_step(&self, step: &WorkflowStep) -> Result<bool> {
        if let Some(condition) = &step.condition {
            match condition {
                StepCondition::Always => Ok(true),
                StepCondition::OnSuccess => {
                    // Check if previous step succeeded
                    if self.state.current_step > 0 {
                        let prev_step = &self.config.steps[self.state.current_step - 1];
                        if let Some(result) = self.state.step_results.get(&prev_step.name) {
                            Ok(result.get("success").and_then(|v| v.as_bool()).unwrap_or(true))
                        } else {
                            Ok(true)
                        }
                    } else {
                        Ok(true)
                    }
                }
                StepCondition::OnFailure => {
                    // Check if previous step failed
                    if self.state.current_step > 0 {
                        let prev_step = &self.config.steps[self.state.current_step - 1];
                        if let Some(result) = self.state.step_results.get(&prev_step.name) {
                            Ok(!result.get("success").and_then(|v| v.as_bool()).unwrap_or(false))
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                }
                StepCondition::Expression(_expr) => {
                    // TODO: Implement expression evaluation
                    Ok(true)
                }
            }
        } else {
            Ok(true)
        }
    }

    /// Mark workflow as failed
    pub fn fail(&mut self, reason: impl Into<String>) -> Result<()> {
        self.state.status = WorkflowStatus::Failed;
        // Store failure reason in state
        self.state.step_results.insert(
            "_workflow_error".to_string(),
            serde_json::json!({ "error": reason.into() }),
        );
        Ok(())
    }

    /// Mark workflow as cancelled
    pub fn cancel(&mut self) -> Result<()> {
        self.state.status = WorkflowStatus::Cancelled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorkflowSettings;

    fn create_simple_workflow() -> WorkflowConfig {
        WorkflowConfig {
            id: "test_workflow".to_string(),
            description: Some("Test workflow".to_string()),
            steps: vec![
                WorkflowStep {
                    name: "step1".to_string(),
                    pattern: "react_1".to_string(),
                    config: None,
                    on_success: Some(StepTransition::Goto("step2".to_string())),
                    on_failure: Some(StepTransition::End { end: true }),
                    condition: None,
                },
                WorkflowStep {
                    name: "step2".to_string(),
                    pattern: "plan_execute_1".to_string(),
                    config: None,
                    on_success: Some(StepTransition::End { end: true }),
                    on_failure: None,
                    condition: None,
                },
            ],
            settings: Some(WorkflowSettings {
                max_total_steps: 20,
                enable_retries: false,
                max_retries: 3,
                timeout: None,
                enable_parallel: false,
            }),
        }
    }

    #[test]
    fn test_executor_creation() {
        let workflow = create_simple_workflow();
        let executor = WorkflowExecutor::new(workflow);

        assert_eq!(executor.state().status, WorkflowStatus::Pending);
        assert_eq!(executor.state().current_step, 0);
        assert_eq!(executor.state().steps_executed, 0);
    }

    #[test]
    fn test_start_workflow() {
        let workflow = create_simple_workflow();
        let mut executor = WorkflowExecutor::new(workflow);

        executor.start().unwrap();
        assert_eq!(executor.state().status, WorkflowStatus::Running);
    }

    #[test]
    fn test_current_step() {
        let workflow = create_simple_workflow();
        let executor = WorkflowExecutor::new(workflow);

        let step = executor.current_step().unwrap();
        assert_eq!(step.name, "step1");
        assert_eq!(step.pattern, "react_1");
    }

    #[test]
    fn test_record_step_success() {
        let workflow = create_simple_workflow();
        let mut executor = WorkflowExecutor::new(workflow);
        executor.start().unwrap();

        let result = serde_json::json!({ "output": "success" });
        let next_step = executor.record_step_result(result, true).unwrap();

        assert_eq!(next_step, Some("step2".to_string()));
        assert_eq!(executor.state().current_step, 1);
        assert_eq!(executor.state().steps_executed, 1);
    }

    #[test]
    fn test_record_step_failure() {
        let workflow = create_simple_workflow();
        let mut executor = WorkflowExecutor::new(workflow);
        executor.start().unwrap();

        let result = serde_json::json!({ "output": "failed" });
        let next_step = executor.record_step_result(result, false).unwrap();

        assert_eq!(next_step, None);
        assert_eq!(executor.state().status, WorkflowStatus::Completed);
    }

    #[test]
    fn test_workflow_completion() {
        let workflow = create_simple_workflow();
        let mut executor = WorkflowExecutor::new(workflow);
        executor.start().unwrap();

        // Step 1
        executor
            .record_step_result(serde_json::json!({ "output": "step1" }), true)
            .unwrap();

        assert!(!executor.is_complete());

        // Step 2
        executor
            .record_step_result(serde_json::json!({ "output": "step2" }), true)
            .unwrap();

        assert!(executor.is_complete());
        assert_eq!(executor.state().status, WorkflowStatus::Completed);
    }

    #[test]
    fn test_conditional_transition() {
        let workflow = WorkflowConfig {
            id: "conditional_workflow".to_string(),
            description: None,
            steps: vec![
                WorkflowStep {
                    name: "check".to_string(),
                    pattern: "react_1".to_string(),
                    config: None,
                    on_success: Some(StepTransition::Conditional {
                        if_condition: "result.success".to_string(),
                        then: Box::new(StepTransition::Goto("success_step".to_string())),
                        else_transition: Some(Box::new(StepTransition::Goto("failure_step".to_string()))),
                    }),
                    on_failure: None,
                    condition: None,
                },
                WorkflowStep {
                    name: "success_step".to_string(),
                    pattern: "react_2".to_string(),
                    config: None,
                    on_success: Some(StepTransition::End { end: true }),
                    on_failure: None,
                    condition: None,
                },
                WorkflowStep {
                    name: "failure_step".to_string(),
                    pattern: "react_3".to_string(),
                    config: None,
                    on_success: Some(StepTransition::End { end: true }),
                    on_failure: None,
                    condition: None,
                },
            ],
            settings: None,
        };

        let mut executor = WorkflowExecutor::new(workflow);
        executor.start().unwrap();

        // Simulate success condition
        let result = serde_json::json!({ "success": true });
        let next_step = executor.record_step_result(result, true).unwrap();

        assert_eq!(next_step, Some("success_step".to_string()));
    }

    #[test]
    fn test_max_steps_limit() {
        let workflow = WorkflowConfig {
            id: "limited_workflow".to_string(),
            description: None,
            steps: vec![
                WorkflowStep {
                    name: "step1".to_string(),
                    pattern: "react_1".to_string(),
                    config: None,
                    on_success: Some(StepTransition::Goto("step1".to_string())), // Loop
                    on_failure: None,
                    condition: None,
                },
            ],
            settings: Some(WorkflowSettings {
                max_total_steps: 2,
                enable_retries: false,
                max_retries: 0,
                timeout: None,
                enable_parallel: false,
            }),
        };

        let mut executor = WorkflowExecutor::new(workflow);
        executor.start().unwrap();

        // First execution - OK
        executor
            .record_step_result(serde_json::json!({}), true)
            .unwrap();

        // Second execution - should fail due to max steps
        let result = executor.record_step_result(serde_json::json!({}), true);
        assert!(result.is_err());
        assert_eq!(executor.state().status, WorkflowStatus::Failed);
    }

    #[test]
    fn test_fail_workflow() {
        let workflow = create_simple_workflow();
        let mut executor = WorkflowExecutor::new(workflow);
        executor.start().unwrap();

        executor.fail("Test error").unwrap();
        assert_eq!(executor.state().status, WorkflowStatus::Failed);
        assert!(executor.is_complete());
    }

    #[test]
    fn test_cancel_workflow() {
        let workflow = create_simple_workflow();
        let mut executor = WorkflowExecutor::new(workflow);
        executor.start().unwrap();

        executor.cancel().unwrap();
        assert_eq!(executor.state().status, WorkflowStatus::Cancelled);
        assert!(executor.is_complete());
    }
}
