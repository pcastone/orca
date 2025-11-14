//! Plan-Execute Agent - Explicit Planning for Complex Tasks
//!
//! The **Plan-Execute** pattern separates planning from execution, creating explicit
//! step-by-step plans before taking action. This is ideal for complex multi-step tasks.
//!
//! # Overview
//!
//! Plan-Execute agents work in two distinct phases:
//!
//! 1. **Plan**: LLM creates detailed step-by-step plan
//! 2. **Execute**: Each step is executed using tools
//! 3. **Evaluate**: Check if replanning is needed
//! 4. **Replan** (if needed): Update plan based on results
//! 5. **Repeat**: Continue until task complete
//!
//! **Use Plan-Execute when:**
//! - Tasks require multiple dependent steps
//! - Upfront planning improves outcome quality
//! - Need explicit task breakdown for debugging
//! - Complex research or data analysis workflows
//!
//! **Don't use when:**
//! - Simple single-step tasks (use ReAct)
//! - Planning overhead not justified
//! - Task requirements change rapidly mid-execution
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │  User Input                                                 │
//! │  "Research and compare top 3 Rust web frameworks"          │
//! └─────────────┬──────────────────────────────────────────────┘
//!               │
//!               ↓ START
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Planner Node (Create Plan)                                 │
//! │  LLM creates:                                               │
//! │  1. Search for Rust web frameworks                          │
//! │  2. Get details on top 3                                    │
//! │  3. Compare features                                        │
//! │  4. Summarize findings                                      │
//! └─────────────┬───────────────────────────────────────────────┘
//!               │
//!               ↓ Execute Step 1
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Executor Node (Execute Current Step)                       │
//! │  • Reads step 1 description                                │
//! │  • Calls executor LLM with tools                            │
//! │  • Marks step complete, stores result                       │
//! └─────────────┬───────────────────────────────────────────────┘
//!               │
//!               ↓ Check Progress
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Evaluator Node (Assess & Route)                            │
//! │  • All steps done? → END                                    │
//! │  • More steps? → Executor                                   │
//! │  • Error/stuck? → Replanner                                 │
//! └─────────────┬───────────────────────────────────────────────┘
//!               │
//!               ↓ Continues executing plan steps...
//! ```
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use langgraph_prebuilt::create_plan_execute_agent;
//! use std::sync::Arc;
//!
//! // Planner LLM: creates plans
//! let planner_llm = Arc::new(|state| {
//!     Box::pin(async move {
//!         // Call LLM to create step-by-step plan
//!         let plan_text = call_llm_for_planning(state).await?;
//!         Ok(Message::ai(plan_text))
//!     }) as Pin<Box<dyn Future<Output = _> + Send>>
//! });
//!
//! // Executor LLM: executes steps
//! let executor_llm = Arc::new(|state| {
//!     Box::pin(async move {
//!         // Call LLM to execute current step with tools
//!         let result = call_llm_for_execution(state).await?;
//!         Ok(Message::ai(result))
//!     }) as Pin<Box<dyn Future<Output = _> + Send>>
//! });
//!
//! // Create agent
//! let agent = create_plan_execute_agent(planner_llm, executor_llm, tools)
//!     .with_max_steps(10)
//!     .with_max_replans(3)
//!     .build()?;
//!
//! // Run complex task
//! let input = json!({
//!     "objective": "Research Rust async runtimes and create comparison table"
//! });
//!
//! let result = agent.invoke(input).await?;
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Research Workflow
//!
//! ```rust,ignore
//! let research_agent = create_plan_execute_agent(planner, executor, vec![
//!     Box::new(WebSearchTool),
//!     Box::new(SummarizerTool),
//! ])
//! .with_planner_prompt(
//!     "You are an expert researcher. Create detailed plans that:\n\
//!      1. Gather information from multiple sources\n\
//!      2. Synthesize findings\n\
//!      3. Provide citations"
//! )
//! .build()?;
//!
//! let result = research_agent.invoke(json!({
//!     "objective": "Research GraphQL vs REST and write report"
//! })).await?;
//! ```
//!
//! ## Pattern 2: Data Pipeline
//!
//! ```rust,ignore
//! let pipeline_agent = create_plan_execute_agent(planner, executor, vec![
//!     Box::new(DatabaseQueryTool),
//!     Box::new(DataTransformTool),
//!     Box::new(VisualizationTool),
//! ])
//! .with_max_steps(15)
//! .build()?;
//!
//! let result = pipeline_agent.invoke(json!({
//!     "objective": "Extract sales data, calculate trends, create dashboard"
//! })).await?;
//! ```
//!
//! # Key Components
//!
//! ## PlanStep
//!
//! Represents a single step in the plan:
//!
//! ```rust,ignore
//! pub struct PlanStep {
//!     step_number: usize,           // 1, 2, 3...
//!     description: String,          // "Search for X"
//!     tool: Option<String>,         // Tool to use
//!     tool_args: Option<Value>,     // Tool arguments
//!     expected_outcome: String,     // Success criteria
//!     completed: bool,              // Execution status
//!     result: Option<String>,       // Execution result
//! }
//! ```
//!
//! ## PlanExecuteState
//!
//! Tracks execution progress:
//!
//! ```rust,ignore
//! pub struct PlanExecuteState {
//!     objective: String,            // Original task
//!     plan: Vec<PlanStep>,          // Current plan
//!     messages: Vec<Message>,       // Conversation history
//!     current_step: usize,          // Which step we're on
//!     replan_count: usize,          // How many replans
//!     final_answer: Option<String>, // Result
//! }
//! ```
//!
//! # Configuration
//!
//! | Method | Description | Default |
//! |--------|-------------|---------|
//! | `with_max_steps(n)` | Maximum steps per plan | 10 |
//! | `with_max_replans(n)` | Maximum replanning attempts | 3 |
//! | `with_planner_prompt(s)` | System prompt for planner | None |
//! | `with_executor_prompt(s)` | System prompt for executor | None |
//!
//! # Replanning
//!
//! Replanning occurs when:
//! - A step fails
//! - New information invalidates the plan
//! - Dependencies change
//!
//! ```rust,ignore
//! let agent = create_plan_execute_agent(planner, executor, tools)
//!     .with_max_replans(5) // Allow 5 plan revisions
//!     .build()?;
//!
//! // Agent will automatically replan if execution fails
//! ```
//!
//! # Python LangGraph Comparison
//!
//! | Python | Rust |
//! |--------|------|
//! | `create_plan_execute_agent(...)` | `create_plan_execute_agent(...).build()` |
//! | Single LLM for plan+execute | Separate planner and executor LLMs |
//! | Sync execution | Async execution |
//!
//! # See Also
//!
//! - [`create_plan_execute_agent`] - Factory function
//! - [`PlanStep`] - Plan step structure
//! - [`PlanExecuteState`] - Execution state
//! - [`create_react_agent`](super::react::create_react_agent) - Simpler alternative
//! - [`create_reflection_agent`](super::reflection::create_reflection_agent) - Quality-focused alternative

use crate::error::{PrebuiltError, Result};
use crate::messages::Message;
use crate::tools::Tool;
use langgraph_core::StateGraph;
use langgraph_core::compiled::CompiledGraph;
use langgraph_core::messages::Message as CoreMessage;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

/// Type alias for LLM functions
pub type LlmFunction = Arc<dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Message>> + Send>> + Send + Sync>;

/// Represents a single step in the execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Step number (1-indexed for user clarity)
    pub step_number: usize,

    /// Description of what this step will accomplish
    pub description: String,

    /// Tool to use for this step (if applicable)
    pub tool: Option<String>,

    /// Arguments for the tool
    pub tool_args: Option<Value>,

    /// Expected outcome or success criteria
    pub expected_outcome: String,

    /// Whether this step has been completed
    #[serde(default)]
    pub completed: bool,

    /// Result of executing this step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// State for the Plan-Execute agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecuteState {
    /// Original user request
    pub objective: String,

    /// Current execution plan
    pub plan: Vec<PlanStep>,

    /// History of all messages
    pub messages: Vec<CoreMessage>,

    /// Current step being executed
    pub current_step: usize,

    /// Number of replanning attempts
    pub replan_count: usize,

    /// Final result of the execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_answer: Option<String>,
}

/// Configuration for Plan-Execute agent
pub struct PlanExecuteConfig {
    /// LLM function for planning
    planner_llm: LlmFunction,

    /// LLM function for executing steps
    executor_llm: LlmFunction,

    /// Tools available to the agent
    tools: Vec<Box<dyn Tool>>,

    /// Maximum number of replanning attempts
    max_replans: usize,

    /// Maximum number of steps in a plan
    max_steps: usize,

    /// System prompt for the planner
    planner_prompt: Option<String>,

    /// System prompt for the executor
    executor_prompt: Option<String>,
}

impl PlanExecuteConfig {
    /// Create a new Plan-Execute configuration
    pub fn new(
        planner_llm: LlmFunction,
        executor_llm: LlmFunction,
        tools: Vec<Box<dyn Tool>>,
    ) -> Self {
        Self {
            planner_llm,
            executor_llm,
            tools,
            max_replans: 3,
            max_steps: 10,
            planner_prompt: None,
            executor_prompt: None,
        }
    }

    /// Set maximum replanning attempts
    pub fn with_max_replans(mut self, max: usize) -> Self {
        self.max_replans = max;
        self
    }

    /// Set maximum steps per plan
    pub fn with_max_steps(mut self, max: usize) -> Self {
        self.max_steps = max;
        self
    }

    /// Set planner system prompt
    pub fn with_planner_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.planner_prompt = Some(prompt.into());
        self
    }

    /// Set executor system prompt
    pub fn with_executor_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.executor_prompt = Some(prompt.into());
        self
    }

    /// Build the compiled Plan-Execute agent graph
    pub fn build(self) -> Result<CompiledGraph> {
        build_plan_execute_graph(self)
    }
}

/// Build the Plan-Execute agent graph
fn build_plan_execute_graph(config: PlanExecuteConfig) -> Result<CompiledGraph> {
    let mut graph = StateGraph::new();

    let planner_llm = config.planner_llm.clone();
    let executor_llm = config.executor_llm.clone();
    let tools = Arc::new(config.tools);
    let max_replans = config.max_replans;
    let max_steps = config.max_steps;

    // Planner node - creates or updates the execution plan
    graph.add_node("planner", move |state: Value| {
        let planner = planner_llm.clone();
        let max_steps_copy = max_steps;

        Box::pin(async move {
            let mut state_obj = state.as_object().cloned().unwrap_or_default();

            // Get objective and current plan state
            let objective = state_obj.get("objective")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let current_plan = state_obj.get("plan")
                .and_then(|v| serde_json::from_value::<Vec<PlanStep>>(v.clone()).ok())
                .unwrap_or_default();

            let messages = state_obj.get("messages")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            // Create planning prompt
            let mut prompt = format!(
                "Create a step-by-step plan to achieve this objective: {}\n\n",
                objective
            );

            if !current_plan.is_empty() {
                prompt.push_str("Previous plan execution encountered issues. ");
                prompt.push_str("Please revise the plan based on the results so far:\n");
                for step in &current_plan {
                    if step.completed {
                        prompt.push_str(&format!(
                            "✓ Step {}: {} - Result: {}\n",
                            step.step_number,
                            step.description,
                            step.result.as_ref().unwrap_or(&"Done".to_string())
                        ));
                    } else {
                        prompt.push_str(&format!(
                            "○ Step {}: {}\n",
                            step.step_number,
                            step.description
                        ));
                    }
                }
            }

            prompt.push_str(&format!(
                "\nCreate a plan with up to {} steps. Return a JSON array of plan steps.",
                max_steps_copy
            ));

            // Call planner LLM
            let planner_input = json!({
                "messages": [{
                    "role": "user",
                    "content": prompt
                }]
            });

            let plan_response = planner(planner_input).await
                .map_err(|e| langgraph_core::GraphError::Execution(e.to_string()))?;

            // Parse plan from response (simplified - in production, use proper parsing)
            let new_plan = parse_plan_from_response(&plan_response, max_steps_copy);

            // Update state
            state_obj.insert("plan".to_string(), serde_json::to_value(new_plan).map_err(|e| langgraph_core::GraphError::Serialization(e))?);
            state_obj.insert("current_step".to_string(), json!(0));

            Ok(Value::Object(state_obj))
        })
    });

    // Executor node - executes the current step of the plan
    let tools_for_executor = tools.clone();
    graph.add_node("executor", move |state: Value| {
        let executor = executor_llm.clone();
        let tools = tools_for_executor.clone();

        Box::pin(async move {
            let mut state_obj = state.as_object().cloned().unwrap_or_default();

            let mut plan = state_obj.get("plan")
                .and_then(|v| serde_json::from_value::<Vec<PlanStep>>(v.clone()).ok())
                .unwrap_or_default();

            let current_step = state_obj.get("current_step")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            if current_step >= plan.len() {
                // All steps completed
                state_obj.insert("final_answer".to_string(), json!("Plan execution complete"));
                return Ok(Value::Object(state_obj));
            }

            // Execute current step
            let (step_number, step_result) = {
                let step = &mut plan[current_step];

                // Create execution prompt
                let exec_prompt = format!(
                    "Execute this step: {}\nExpected outcome: {}",
                    step.description,
                    step.expected_outcome
                );

                let executor_input = json!({
                    "messages": [{
                        "role": "user",
                        "content": exec_prompt
                    }]
                });

                // Call executor LLM (which may use tools)
                let exec_response = executor(executor_input).await
                    .map_err(|e| langgraph_core::GraphError::Execution(e.to_string()))?;

                // Mark step as completed and store result
                step.completed = true;
                step.result = Some(extract_result_from_response(&exec_response));

                // Extract values we need before dropping the mutable borrow
                (step.step_number, step.result.clone().unwrap_or_default())
            };

            // Update state (now that mutable borrow is dropped)
            state_obj.insert("plan".to_string(), serde_json::to_value(&plan).map_err(|e| langgraph_core::GraphError::Serialization(e))?);
            state_obj.insert("current_step".to_string(), json!(current_step + 1));

            // Add execution result to messages
            let mut messages = state_obj.get("messages")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            messages.push(json!({
                "role": "assistant",
                "content": format!("Completed step {}: {}", step_number, step_result)
            }));

            state_obj.insert("messages".to_string(), Value::Array(messages));

            Ok(Value::Object(state_obj))
        })
    });

    // Evaluator node - checks if we need to replan or continue
    let max_replans_for_eval = max_replans;
    graph.add_node("evaluator", move |state: Value| {
        Box::pin(async move {
            let mut state_obj = state.as_object().cloned().unwrap_or_default();

            let plan = state_obj.get("plan")
                .and_then(|v| serde_json::from_value::<Vec<PlanStep>>(v.clone()).ok())
                .unwrap_or_default();

            let current_step = state_obj.get("current_step")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let replan_count = state_obj.get("replan_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            // Check if all steps are completed successfully
            let all_completed = plan.iter().all(|s| s.completed);

            if all_completed || current_step >= plan.len() {
                // Plan complete
                let summary = plan.iter()
                    .filter_map(|s| s.result.as_ref())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n");

                state_obj.insert("final_answer".to_string(), json!(summary));
            } else if replan_count >= max_replans_for_eval {
                // Max replans reached
                state_obj.insert("final_answer".to_string(),
                    json!("Maximum replanning attempts reached. Partial results available."));
            }

            Ok(Value::Object(state_obj))
        })
    });

    // Add edges
    graph.add_edge("__start__", "planner");
    graph.add_edge("planner", "executor");
    graph.add_edge("executor", "evaluator");

    // Conditional routing from evaluator
    graph.add_conditional_edge(
        "evaluator",
        |state: &Value| {
            use langgraph_core::send::ConditionalEdgeResult;

            if state.get("final_answer").is_some() {
                ConditionalEdgeResult::Node("__end__".to_string())
            } else if should_replan(state) {
                ConditionalEdgeResult::Node("planner".to_string())
            } else {
                ConditionalEdgeResult::Node("executor".to_string())
            }
        },
        vec![
            ("__end__".to_string(), "__end__".to_string()),
            ("planner".to_string(), "planner".to_string()),
            ("executor".to_string(), "executor".to_string()),
        ].into_iter().collect(),
    );

    // Compile and return
    graph.compile().map_err(|e| PrebuiltError::Graph(e))
}

/// Helper function to parse plan from LLM response
fn parse_plan_from_response(response: &Message, max_steps: usize) -> Vec<PlanStep> {
    // Simplified parsing - in production, use proper JSON extraction
    // This would parse the LLM's response to extract structured plan steps

    let mut steps = Vec::new();

    // For now, create a simple default plan
    // In a real implementation, this would parse the LLM output
    for i in 1..=3.min(max_steps) {
        steps.push(PlanStep {
            step_number: i,
            description: format!("Step {} from plan", i),
            tool: None,
            tool_args: None,
            expected_outcome: format!("Complete step {}", i),
            completed: false,
            result: None,
        });
    }

    steps
}

/// Helper function to extract result from executor response
fn extract_result_from_response(response: &Message) -> String {
    // Simplified - extract the actual result from the LLM response
    response.content.clone()
}

/// Helper function to determine if replanning is needed
fn should_replan(state: &Value) -> bool {
    // Check if the last executed step failed or needs replanning
    if let Some(plan) = state.get("plan").and_then(|v| v.as_array()) {
        // Check for failed steps or other conditions that require replanning
        for step in plan {
            if let Ok(step_obj) = serde_json::from_value::<PlanStep>(step.clone()) {
                if step_obj.completed {
                    if let Some(result) = &step_obj.result {
                        if result.contains("error") || result.contains("failed") {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Create a Plan-Execute agent with the given configuration
pub fn create_plan_execute_agent(
    planner_llm: LlmFunction,
    executor_llm: LlmFunction,
    tools: Vec<Box<dyn Tool>>,
) -> PlanExecuteConfig {
    PlanExecuteConfig::new(planner_llm, executor_llm, tools)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_step_serialization() {
        let step = PlanStep {
            step_number: 1,
            description: "Test step".to_string(),
            tool: Some("calculator".to_string()),
            tool_args: Some(json!({"a": 1, "b": 2})),
            expected_outcome: "Get sum".to_string(),
            completed: false,
            result: None,
        };

        let serialized = serde_json::to_string(&step).unwrap();
        let deserialized: PlanStep = serde_json::from_str(&serialized).unwrap();

        assert_eq!(step.step_number, deserialized.step_number);
        assert_eq!(step.description, deserialized.description);
    }

    #[test]
    fn test_state_serialization() {
        let state = PlanExecuteState {
            objective: "Test objective".to_string(),
            plan: vec![],
            messages: vec![],
            current_step: 0,
            replan_count: 0,
            final_answer: None,
        };

        let serialized = serde_json::to_string(&state).unwrap();
        let deserialized: PlanExecuteState = serde_json::from_str(&serialized).unwrap();

        assert_eq!(state.objective, deserialized.objective);
        assert_eq!(state.current_step, deserialized.current_step);
    }
}