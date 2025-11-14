# Task 013: Implement Workflow Execution Engine

## Objective
Create WorkflowExecutor using langgraph-core for multi-task workflows.

## Dependencies
- Task 011 (Task Executor)
- langgraph-core crate

## Key Files
- `src/crates/orchestrator/src/execution/workflow_executor.rs`

## Implementation
- Parse WorkflowDefinition (nodes/edges) into StateGraph
- Compile graph with langgraph-core
- Execute workflow with Pregel model
- Emit checkpoints at each superstep
- Handle node failures and retries
- Update workflow status

## Integration
- Use TaskExecutor for task nodes
- Support conditional edges
- State reduction between nodes

## Complexity: High | Effort: 15-18 hours
