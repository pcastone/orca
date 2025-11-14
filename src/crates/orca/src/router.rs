//! Workflow routing and execution coordination
//!
//! The Router module manages workflow state transitions and execution flow,
//! determining which task(s) to execute next based on the current state and
//! configured routing strategy.
//!
//! # Router Responsibilities
//!
//! - **Task Sequencing**: Determine the order of task execution
//! - **Conditional Branching**: Route to different tasks based on execution results
//! - **Parallel Coordination**: Manage concurrent task group execution
//! - **State Transitions**: Update workflow state between task executions
//! - **Flow Control**: Handle end-of-workflow conditions and loops
//!
//! # Routing Strategies
//!
//! ## Sequential (Default)
//! Tasks execute one after another in the order they were added to the workflow.
//! This is the simplest strategy and suitable for linear workflows.
//!
//! ```text
//! Task A → Task B → Task C → Complete
//! ```
//!
//! ## Conditional
//! Tasks route based on the result of the previous task execution.
//! Enables branching workflows with decision points.
//!
//! ```text
//!          ┌─ success ─→ Task B
//! Task A ──┤
//!          └─ failure ─→ Task C
//! ```
//!
//! ## Parallel
//! Multiple tasks execute concurrently, then wait for all to complete
//! before proceeding. Useful for independent operations.
//!
//! ```text
//!          ┌─→ Task B ─┐
//! Task A ──┼─→ Task C ─┼─→ Task E
//!          └─→ Task D ─┘
//! ```
//!
//! # Integration with Executor
//!
//! The TaskExecutor uses the Router to:
//! 1. Determine next task(s) to execute
//! 2. Check if workflow is complete
//! 3. Handle conditional logic and branching
//! 4. Coordinate parallel task groups
//!
//! The Router does NOT execute tasks itself - it only determines routing decisions.

use crate::error::Result;
use crate::workflow::{Task, Workflow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Routing decision for workflow execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingDecision {
    /// Continue to the next task(s)
    Continue(Vec<String>), // task IDs to execute next

    /// Workflow is complete
    Complete,

    /// Wait for parallel tasks to finish
    Wait,
}

/// Routing strategy type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// Sequential execution (default)
    Sequential,

    /// Conditional branching
    Conditional,

    /// Parallel task groups
    Parallel,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self::Sequential
    }
}

impl std::fmt::Display for RoutingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sequential => write!(f, "sequential"),
            Self::Conditional => write!(f, "conditional"),
            Self::Parallel => write!(f, "parallel"),
        }
    }
}

/// Routing context containing workflow state
#[derive(Debug, Clone)]
pub struct RoutingContext {
    /// Current workflow
    pub workflow: Workflow,

    /// All tasks in the workflow (in order)
    pub task_ids: Vec<String>,

    /// Currently executing task (if any)
    pub current_task: Option<Task>,

    /// Index of current task in task_ids
    pub current_index: Option<usize>,

    /// Whether the last task succeeded
    pub last_task_success: bool,

    /// Result from the last task
    pub last_task_result: Option<String>,
}

/// Router trait for workflow routing decisions
///
/// Implementors provide different routing strategies (sequential, conditional, parallel).
/// The Router is responsible for determining what task(s) to execute next based on
/// the current workflow state.
#[async_trait]
pub trait Router: Send + Sync {
    /// Determine the next routing decision
    ///
    /// # Arguments
    /// * `context` - Current routing context with workflow state
    ///
    /// # Returns
    /// A RoutingDecision indicating what to do next
    async fn route(&self, context: &RoutingContext) -> Result<RoutingDecision>;

    /// Get the router's strategy type
    fn strategy(&self) -> RoutingStrategy;

    /// Validate the workflow structure for this routing strategy
    ///
    /// # Arguments
    /// * `workflow` - The workflow to validate
    /// * `task_ids` - All task IDs in the workflow
    ///
    /// # Returns
    /// Ok(()) if valid, Err with details if invalid
    fn validate_workflow(&self, _workflow: &Workflow, task_ids: &[String]) -> Result<()> {
        // Default implementation: basic validation
        if task_ids.is_empty() {
            return Err(crate::error::OrcaError::Workflow(
                "Workflow has no tasks".to_string()
            ));
        }
        Ok(())
    }
}

/// Create a router based on strategy
pub fn create_router(strategy: RoutingStrategy) -> Box<dyn Router> {
    match strategy {
        RoutingStrategy::Sequential => Box::new(SequentialRouter),
        RoutingStrategy::Conditional => Box::new(ConditionalRouter::default()),
        RoutingStrategy::Parallel => Box::new(ParallelRouter::default()),
    }
}

/// Sequential routing implementation
///
/// Routes tasks in the order they were added to the workflow.
/// This is the simplest routing strategy and suitable for linear workflows.
pub struct SequentialRouter;

#[async_trait]
impl Router for SequentialRouter {
    async fn route(&self, context: &RoutingContext) -> Result<RoutingDecision> {
        // Determine next task index
        let next_index = match context.current_index {
            None => 0, // Start at beginning
            Some(idx) => idx + 1, // Move to next task
        };

        // Check if we've reached the end
        if next_index >= context.task_ids.len() {
            return Ok(RoutingDecision::Complete);
        }

        // Get the next task ID
        let next_task_id = context.task_ids[next_index].clone();

        Ok(RoutingDecision::Continue(vec![next_task_id]))
    }

    fn strategy(&self) -> RoutingStrategy {
        RoutingStrategy::Sequential
    }
}

/// Conditional routing implementation
///
/// Routes tasks based on the result of the previous task execution.
/// Enables branching workflows with decision points.
///
/// Conditions are evaluated based on the last task's success/failure state.
/// If no explicit condition is met, falls back to sequential routing.
#[derive(Default)]
pub struct ConditionalRouter {
    // Future: condition rules could be stored here for more complex scenarios
}

/// Condition evaluation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionResult {
    /// Condition matched - proceed with specified task
    Match(String), // task ID to route to

    /// No condition matched - use default behavior
    NoMatch,
}

impl ConditionalRouter {
    /// Evaluate conditions based on context
    ///
    /// Current simple implementation:
    /// - If last task failed and there's a next task, route to it (error handling path)
    /// - If last task succeeded, route to next sequential task
    /// - Future: support metadata-based condition expressions
    fn evaluate_condition(&self, context: &RoutingContext) -> ConditionResult {
        // For now, use simple success/failure routing
        // Future versions will support metadata conditions like:
        // - "result.status == 'needs_review'" -> route to review task
        // - "result.score > 0.8" -> route to approval task

        let current_index = match context.current_index {
            Some(idx) => idx,
            None => return ConditionResult::NoMatch, // Start of workflow
        };

        let next_index = current_index + 1;

        // Check if there are more tasks
        if next_index >= context.task_ids.len() {
            return ConditionResult::NoMatch; // End of workflow
        }

        // Route to next task (conditional logic can be enhanced later)
        let next_task_id = context.task_ids[next_index].clone();
        ConditionResult::Match(next_task_id)
    }
}

#[async_trait]
impl Router for ConditionalRouter {
    async fn route(&self, context: &RoutingContext) -> Result<RoutingDecision> {
        // Determine next task index
        let next_index = match context.current_index {
            None => 0, // Start at beginning
            Some(idx) => idx + 1, // Move to next task
        };

        // Check if we've reached the end
        if next_index >= context.task_ids.len() {
            return Ok(RoutingDecision::Complete);
        }

        // Evaluate conditions
        match self.evaluate_condition(context) {
            ConditionResult::Match(task_id) => {
                Ok(RoutingDecision::Continue(vec![task_id]))
            }
            ConditionResult::NoMatch => {
                // Fall back to sequential routing if no condition matches
                if next_index < context.task_ids.len() {
                    let next_task_id = context.task_ids[next_index].clone();
                    Ok(RoutingDecision::Continue(vec![next_task_id]))
                } else {
                    Ok(RoutingDecision::Complete)
                }
            }
        }
    }

    fn strategy(&self) -> RoutingStrategy {
        RoutingStrategy::Conditional
    }
}

/// Parallel routing implementation
///
/// Routes multiple tasks to execute concurrently, then waits for all to complete.
/// Tasks are grouped into parallel "batches" that execute simultaneously.
///
/// Current implementation uses a simple heuristic:
/// - Groups consecutive tasks into batches
/// - Each batch executes in parallel
/// - Waits for all tasks in batch to complete before next batch
///
/// Future: Support explicit parallel group definitions via metadata.
#[derive(Default)]
pub struct ParallelRouter {
    /// Batch size for parallel execution (default: 3)
    batch_size: Option<usize>,
}

impl ParallelRouter {
    /// Create a new parallel router with custom batch size
    pub fn with_batch_size(batch_size: usize) -> Self {
        Self {
            batch_size: Some(batch_size),
        }
    }

    /// Get the batch size (default: 3 if not specified)
    fn get_batch_size(&self) -> usize {
        self.batch_size.unwrap_or(3)
    }

    /// Determine which tasks should execute in the current parallel batch
    fn get_parallel_batch(&self, context: &RoutingContext) -> Vec<String> {
        let start_index = match context.current_index {
            None => 0, // Start of workflow
            Some(idx) => idx + 1, // After current task
        };

        let batch_size = self.get_batch_size();
        let end_index = (start_index + batch_size).min(context.task_ids.len());

        context.task_ids[start_index..end_index]
            .iter()
            .cloned()
            .collect()
    }
}

#[async_trait]
impl Router for ParallelRouter {
    async fn route(&self, context: &RoutingContext) -> Result<RoutingDecision> {
        // Determine starting index for next batch
        let start_index = match context.current_index {
            None => 0, // Start at beginning
            Some(idx) => idx + 1, // Move past current task
        };

        // Check if we've reached the end
        if start_index >= context.task_ids.len() {
            return Ok(RoutingDecision::Complete);
        }

        // Get the parallel batch of tasks
        let batch = self.get_parallel_batch(context);

        if batch.is_empty() {
            return Ok(RoutingDecision::Complete);
        }

        // Return all tasks in the batch for parallel execution
        Ok(RoutingDecision::Continue(batch))
    }

    fn strategy(&self) -> RoutingStrategy {
        RoutingStrategy::Parallel
    }

    fn validate_workflow(&self, workflow: &Workflow, task_ids: &[String]) -> Result<()> {
        // Call parent validation first
        if task_ids.is_empty() {
            return Err(crate::error::OrcaError::Workflow(
                "Workflow has no tasks".to_string()
            ));
        }

        // Additional validation: ensure batch size is reasonable
        let batch_size = self.get_batch_size();
        if batch_size == 0 {
            return Err(crate::error::OrcaError::Workflow(
                "Parallel batch size must be greater than 0".to_string()
            ));
        }

        if batch_size > 10 {
            // Warn about large batch sizes
            tracing::warn!(
                workflow_id = %workflow.id,
                batch_size = batch_size,
                "Large parallel batch size may cause resource contention"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_strategy_display() {
        assert_eq!(RoutingStrategy::Sequential.to_string(), "sequential");
        assert_eq!(RoutingStrategy::Conditional.to_string(), "conditional");
        assert_eq!(RoutingStrategy::Parallel.to_string(), "parallel");
    }

    #[test]
    fn test_routing_strategy_default() {
        assert_eq!(RoutingStrategy::default(), RoutingStrategy::Sequential);
    }

    #[test]
    fn test_create_router() {
        let router = create_router(RoutingStrategy::Sequential);
        assert_eq!(router.strategy(), RoutingStrategy::Sequential);

        let router = create_router(RoutingStrategy::Conditional);
        assert_eq!(router.strategy(), RoutingStrategy::Conditional);

        let router = create_router(RoutingStrategy::Parallel);
        assert_eq!(router.strategy(), RoutingStrategy::Parallel);
    }

    #[test]
    fn test_routing_decision_equality() {
        assert_eq!(
            RoutingDecision::Continue(vec!["task-1".to_string()]),
            RoutingDecision::Continue(vec!["task-1".to_string()])
        );

        assert_eq!(RoutingDecision::Complete, RoutingDecision::Complete);
        assert_eq!(RoutingDecision::Wait, RoutingDecision::Wait);

        assert_ne!(RoutingDecision::Complete, RoutingDecision::Wait);
    }

    // Helper function to create test workflow
    fn create_test_workflow() -> Workflow {
        Workflow::new("test-workflow", "react")
    }

    // Helper function to create routing context
    fn create_routing_context(
        task_ids: Vec<String>,
        current_index: Option<usize>,
        last_success: bool,
    ) -> RoutingContext {
        RoutingContext {
            workflow: create_test_workflow(),
            task_ids,
            current_task: None,
            current_index,
            last_task_success: last_success,
            last_task_result: None,
        }
    }

    #[tokio::test]
    async fn test_sequential_router_start() {
        let router = SequentialRouter;
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
            None, // No current task (start of workflow)
            true,
        );

        let decision = router.route(&context).await.unwrap();

        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-1".to_string()])
        );
    }

    #[tokio::test]
    async fn test_sequential_router_middle() {
        let router = SequentialRouter;
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
            Some(0), // Currently at first task
            true,
        );

        let decision = router.route(&context).await.unwrap();

        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-2".to_string()])
        );
    }

    #[tokio::test]
    async fn test_sequential_router_complete() {
        let router = SequentialRouter;
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
            Some(2), // Currently at last task
            true,
        );

        let decision = router.route(&context).await.unwrap();

        assert_eq!(decision, RoutingDecision::Complete);
    }

    #[tokio::test]
    async fn test_sequential_router_single_task() {
        let router = SequentialRouter;
        let context = create_routing_context(
            vec!["task-1".to_string()],
            None, // Start of workflow
            true,
        );

        // First call should return the task
        let decision = router.route(&context).await.unwrap();
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-1".to_string()])
        );

        // After completing the single task, should be complete
        let context = create_routing_context(
            vec!["task-1".to_string()],
            Some(0), // After first (and only) task
            true,
        );

        let decision = router.route(&context).await.unwrap();
        assert_eq!(decision, RoutingDecision::Complete);
    }

    #[tokio::test]
    async fn test_sequential_router_ignores_task_result() {
        let router = SequentialRouter;

        // Test with successful task
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string()],
            Some(0),
            true, // Last task succeeded
        );

        let decision = router.route(&context).await.unwrap();
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-2".to_string()])
        );

        // Test with failed task (should still route to next)
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string()],
            Some(0),
            false, // Last task failed
        );

        let decision = router.route(&context).await.unwrap();
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-2".to_string()])
        );
    }

    #[tokio::test]
    async fn test_conditional_router_start() {
        let router = ConditionalRouter::default();
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
            None, // No current task (start of workflow)
            true,
        );

        let decision = router.route(&context).await.unwrap();

        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-1".to_string()])
        );
    }

    #[tokio::test]
    async fn test_conditional_router_success_path() {
        let router = ConditionalRouter::default();
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
            Some(0), // Currently at first task
            true,    // Last task succeeded
        );

        let decision = router.route(&context).await.unwrap();

        // Should route to next task
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-2".to_string()])
        );
    }

    #[tokio::test]
    async fn test_conditional_router_failure_path() {
        let router = ConditionalRouter::default();
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
            Some(0), // Currently at first task
            false,   // Last task failed
        );

        let decision = router.route(&context).await.unwrap();

        // In current simple implementation, still routes to next task
        // Future: could route to error handling task
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-2".to_string()])
        );
    }

    #[tokio::test]
    async fn test_conditional_router_complete() {
        let router = ConditionalRouter::default();
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
            Some(2), // Currently at last task
            true,
        );

        let decision = router.route(&context).await.unwrap();

        assert_eq!(decision, RoutingDecision::Complete);
    }

    #[tokio::test]
    async fn test_conditional_router_with_result() {
        let router = ConditionalRouter::default();
        let mut context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string()],
            Some(0),
            true,
        );

        // Add result data (for future condition evaluation)
        context.last_task_result = Some("{\"status\": \"needs_review\"}".to_string());

        let decision = router.route(&context).await.unwrap();

        // Currently routes sequentially, but infrastructure is ready for
        // metadata-based conditions in future
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-2".to_string()])
        );
    }

    #[tokio::test]
    async fn test_parallel_router_start() {
        let router = ParallelRouter::default();
        let context = create_routing_context(
            vec![
                "task-1".to_string(),
                "task-2".to_string(),
                "task-3".to_string(),
                "task-4".to_string(),
                "task-5".to_string(),
            ],
            None, // No current task (start of workflow)
            true,
        );

        let decision = router.route(&context).await.unwrap();

        // Should return first batch (default batch size = 3)
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec![
                "task-1".to_string(),
                "task-2".to_string(),
                "task-3".to_string()
            ])
        );
    }

    #[tokio::test]
    async fn test_parallel_router_second_batch() {
        let router = ParallelRouter::default();
        let context = create_routing_context(
            vec![
                "task-1".to_string(),
                "task-2".to_string(),
                "task-3".to_string(),
                "task-4".to_string(),
                "task-5".to_string(),
            ],
            Some(2), // After third task (index 2)
            true,
        );

        let decision = router.route(&context).await.unwrap();

        // Should return remaining tasks
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-4".to_string(), "task-5".to_string()])
        );
    }

    #[tokio::test]
    async fn test_parallel_router_custom_batch_size() {
        let router = ParallelRouter::with_batch_size(2);
        let context = create_routing_context(
            vec![
                "task-1".to_string(),
                "task-2".to_string(),
                "task-3".to_string(),
                "task-4".to_string(),
            ],
            None,
            true,
        );

        let decision = router.route(&context).await.unwrap();

        // Should return first batch with custom size (2)
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-1".to_string(), "task-2".to_string()])
        );
    }

    #[tokio::test]
    async fn test_parallel_router_complete() {
        let router = ParallelRouter::default();
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
            Some(2), // After last task
            true,
        );

        let decision = router.route(&context).await.unwrap();

        assert_eq!(decision, RoutingDecision::Complete);
    }

    #[tokio::test]
    async fn test_parallel_router_partial_batch() {
        let router = ParallelRouter::default(); // batch size = 3
        let context = create_routing_context(
            vec![
                "task-1".to_string(),
                "task-2".to_string(),
                "task-3".to_string(),
                "task-4".to_string(),
            ],
            Some(2), // After third task, only one task left
            true,
        );

        let decision = router.route(&context).await.unwrap();

        // Should return partial batch (only 1 task)
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-4".to_string()])
        );
    }

    #[tokio::test]
    async fn test_parallel_router_validation() {
        let router = ParallelRouter::default();
        let workflow = create_test_workflow();

        // Valid workflow
        let result = router.validate_workflow(&workflow, &["task-1".to_string()]);
        assert!(result.is_ok());

        // Empty workflow
        let result = router.validate_workflow(&workflow, &[]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parallel_router_with_failures() {
        let router = ParallelRouter::default();

        // Test with failed task
        let context = create_routing_context(
            vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
            Some(0),
            false, // Last task failed
        );

        let decision = router.route(&context).await.unwrap();

        // Should still route to next batch (failure handling is executor's responsibility)
        assert_eq!(
            decision,
            RoutingDecision::Continue(vec!["task-2".to_string(), "task-3".to_string()])
        );
    }
}
