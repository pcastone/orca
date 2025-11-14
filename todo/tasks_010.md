# Task 010: Implement Workflow Service (gRPC)

## Objective
Implement WorkflowService gRPC service for workflow CRUD operations.

## Dependencies
- Task 009 (Task Service pattern)

## Key Files
- `src/crates/orchestrator/src/services/workflow.rs` - WorkflowServiceImpl
- `src/crates/orchestrator/src/database/repository/workflow.rs` - WorkflowRepository

## Implementation
Similar to TaskService but for workflows:
- CreateWorkflow, GetWorkflow, ListWorkflows
- UpdateWorkflow, DeleteWorkflow
- Workflow definition validation (nodes/edges)
- Integration with WorkflowRepository

## Tests
Integration tests for all RPC methods

## Complexity: Moderate | Effort: 6-8 hours
