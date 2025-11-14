//! Workflow configuration
//!
//! Defines multi-step workflow orchestration with conditional routing
//! between pattern executions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Workflow identifier
    pub id: String,
    /// Workflow description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Workflow steps
    pub steps: Vec<WorkflowStep>,
    /// Global workflow settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<WorkflowSettings>,
}

/// Individual workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step name/identifier
    pub name: String,
    /// Pattern ID to execute
    pub pattern: String,
    /// Step-specific configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, serde_json::Value>>,
    /// What to do on success
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_success: Option<StepTransition>,
    /// What to do on failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<StepTransition>,
    /// Conditions for executing this step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<StepCondition>,
}

/// Step transition (what to do next)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StepTransition {
    /// Go to a specific step
    Goto(String),
    /// End the workflow
    End { end: bool },
    /// Conditional routing
    Conditional {
        #[serde(rename = "if")]
        if_condition: String,
        then: Box<StepTransition>,
        #[serde(rename = "else", skip_serializing_if = "Option::is_none")]
        else_transition: Option<Box<StepTransition>>,
    },
}

/// Condition for executing a step
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepCondition {
    /// Always execute
    Always,
    /// Execute if previous step succeeded
    OnSuccess,
    /// Execute if previous step failed
    OnFailure,
    /// Custom expression
    Expression(String),
}

/// Global workflow settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSettings {
    /// Maximum total steps in workflow
    #[serde(default = "default_max_total_steps")]
    pub max_total_steps: usize,
    /// Enable step retries
    #[serde(default)]
    pub enable_retries: bool,
    /// Maximum retries per step
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    /// Timeout for entire workflow (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// Enable parallel execution where possible
    #[serde(default)]
    pub enable_parallel: bool,
}

fn default_max_total_steps() -> usize {
    20
}

fn default_max_retries() -> usize {
    3
}

/// Workflow execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Current step index
    pub current_step: usize,
    /// Total steps executed
    pub steps_executed: usize,
    /// Results from previous steps
    pub step_results: HashMap<String, serde_json::Value>,
    /// Workflow status
    pub status: WorkflowStatus,
}

/// Workflow execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// Workflow is pending
    Pending,
    /// Workflow is running
    Running,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed
    Failed,
    /// Workflow was cancelled
    Cancelled,
}

impl WorkflowState {
    /// Create a new workflow state
    pub fn new() -> Self {
        Self {
            current_step: 0,
            steps_executed: 0,
            step_results: HashMap::new(),
            status: WorkflowStatus::Pending,
        }
    }

    /// Record step result
    pub fn record_result(&mut self, step_name: String, result: serde_json::Value) {
        self.step_results.insert(step_name, result);
        self.steps_executed += 1;
    }
}

impl Default for WorkflowState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_config_deserialization() {
        let yaml = r#"
            id: "research_workflow"
            description: "Multi-step research workflow"
            steps:
              - name: "plan"
                pattern: "plan_execute_1"
                on_success: "execute"
                on_failure:
                  end: true
              - name: "execute"
                pattern: "react_1"
                on_success:
                  end: true
            settings:
              max_total_steps: 15
              enable_retries: true
              max_retries: 2
        "#;

        let config: WorkflowConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.id, "research_workflow");
        assert_eq!(config.steps.len(), 2);
        assert_eq!(config.steps[0].name, "plan");
        assert_eq!(config.steps[0].pattern, "plan_execute_1");

        let settings = config.settings.unwrap();
        assert_eq!(settings.max_total_steps, 15);
        assert!(settings.enable_retries);
        assert_eq!(settings.max_retries, 2);
    }

    #[test]
    fn test_step_transition_goto() {
        let yaml = r#""next_step""#;
        let transition: StepTransition = serde_yaml::from_str(yaml).unwrap();

        match transition {
            StepTransition::Goto(step) => assert_eq!(step, "next_step"),
            _ => panic!("Expected Goto transition"),
        }
    }

    #[test]
    fn test_step_transition_end() {
        let yaml = r#"
            end: true
        "#;
        let transition: StepTransition = serde_yaml::from_str(yaml).unwrap();

        match transition {
            StepTransition::End { end } => assert!(end),
            _ => panic!("Expected End transition"),
        }
    }

    #[test]
    fn test_step_transition_conditional() {
        let yaml = r#"
            if: "result.success"
            then: "success_step"
            else:
              end: true
        "#;
        let transition: StepTransition = serde_yaml::from_str(yaml).unwrap();

        match transition {
            StepTransition::Conditional { if_condition, then, else_transition } => {
                assert_eq!(if_condition, "result.success");
                assert!(else_transition.is_some());
            }
            _ => panic!("Expected Conditional transition"),
        }
    }

    #[test]
    fn test_workflow_state() {
        let mut state = WorkflowState::new();
        assert_eq!(state.status, WorkflowStatus::Pending);
        assert_eq!(state.steps_executed, 0);

        state.record_result("step1".to_string(), serde_json::json!({"result": "success"}));
        assert_eq!(state.steps_executed, 1);
        assert_eq!(state.step_results.len(), 1);
    }

    #[test]
    fn test_step_with_config_override() {
        let yaml = r#"
            name: "custom_step"
            pattern: "react_1"
            config:
              max_iterations: 15
              temperature: 0.9
        "#;

        let step: WorkflowStep = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(step.name, "custom_step");
        assert!(step.config.is_some());

        let config = step.config.unwrap();
        assert_eq!(config.get("max_iterations").unwrap(), &serde_json::json!(15));
        assert_eq!(config.get("temperature").unwrap(), &serde_json::json!(0.9));
    }
}
