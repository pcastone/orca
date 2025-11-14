# Task 001: Define gRPC Protocol Buffers

## Objective
Define Protocol Buffer (.proto) files for the gRPC API contract between aco client and orchestrator server, covering task management, workflow execution, and real-time streaming.

## Priority
**CRITICAL** - Foundation for all client-server communication

## Dependencies
- None (this is the starting point)

## Implementation Details

### Files to Create

1. **`src/crates/orchestrator/proto/tasks.proto`**
```protobuf
syntax = "proto3";
package orchestrator.tasks;

service TaskService {
  rpc CreateTask(CreateTaskRequest) returns (Task);
  rpc GetTask(GetTaskRequest) returns (Task);
  rpc ListTasks(ListTasksRequest) returns (ListTasksResponse);
  rpc UpdateTask(UpdateTaskRequest) returns (Task);
  rpc DeleteTask(DeleteTaskRequest) returns (DeleteTaskResponse);
  rpc ExecuteTask(ExecuteTaskRequest) returns (stream ExecutionEvent);
}

message Task {
  string id = 1;
  string title = 2;
  string description = 3;
  string task_type = 4;
  TaskStatus status = 5;
  string config = 6;  // JSON
  string metadata = 7;  // JSON
  string workspace_path = 8;
  string created_at = 9;
  string updated_at = 10;
  optional string started_at = 11;
  optional string completed_at = 12;
  optional string error = 13;
}

enum TaskStatus {
  TASK_STATUS_UNSPECIFIED = 0;
  TASK_STATUS_PENDING = 1;
  TASK_STATUS_RUNNING = 2;
  TASK_STATUS_COMPLETED = 3;
  TASK_STATUS_FAILED = 4;
  TASK_STATUS_CANCELLED = 5;
}

message CreateTaskRequest {
  string title = 1;
  string description = 2;
  string task_type = 3;
  optional string config = 4;
  optional string metadata = 5;
  optional string workspace_path = 6;
}

message GetTaskRequest {
  string id = 1;
}

message ListTasksRequest {
  optional int32 limit = 1;
  optional int32 offset = 2;
  optional TaskStatus status = 3;
  optional string task_type = 4;
}

message ListTasksResponse {
  repeated Task tasks = 1;
  int32 total = 2;
}

message UpdateTaskRequest {
  string id = 1;
  optional string title = 2;
  optional string description = 3;
  optional TaskStatus status = 4;
  optional string config = 5;
  optional string metadata = 6;
}

message DeleteTaskRequest {
  string id = 1;
}

message DeleteTaskResponse {
  bool success = 1;
}

message ExecuteTaskRequest {
  string id = 1;
  optional string config_override = 2;
}

message ExecutionEvent {
  string task_id = 1;
  ExecutionEventType event_type = 2;
  string timestamp = 3;
  optional string data = 4;  // JSON payload
  optional string error = 5;
}

enum ExecutionEventType {
  EXECUTION_EVENT_TYPE_UNSPECIFIED = 0;
  EXECUTION_EVENT_TYPE_STARTED = 1;
  EXECUTION_EVENT_TYPE_PROGRESS = 2;
  EXECUTION_EVENT_TYPE_OUTPUT = 3;
  EXECUTION_EVENT_TYPE_COMPLETED = 4;
  EXECUTION_EVENT_TYPE_FAILED = 5;
  EXECUTION_EVENT_TYPE_TOOL_CALL = 6;
  EXECUTION_EVENT_TYPE_TOOL_RESULT = 7;
}
```

2. **`src/crates/orchestrator/proto/workflows.proto`**
```protobuf
syntax = "proto3";
package orchestrator.workflows;

service WorkflowService {
  rpc CreateWorkflow(CreateWorkflowRequest) returns (Workflow);
  rpc GetWorkflow(GetWorkflowRequest) returns (Workflow);
  rpc ListWorkflows(ListWorkflowsRequest) returns (ListWorkflowsResponse);
  rpc UpdateWorkflow(UpdateWorkflowRequest) returns (Workflow);
  rpc DeleteWorkflow(DeleteWorkflowRequest) returns (DeleteWorkflowResponse);
  rpc ExecuteWorkflow(ExecuteWorkflowRequest) returns (stream WorkflowEvent);
}

message Workflow {
  string id = 1;
  string name = 2;
  string description = 3;
  string definition = 4;  // JSON graph definition
  WorkflowStatus status = 5;
  string created_at = 6;
  string updated_at = 7;
  repeated string task_ids = 8;
}

enum WorkflowStatus {
  WORKFLOW_STATUS_UNSPECIFIED = 0;
  WORKFLOW_STATUS_DRAFT = 1;
  WORKFLOW_STATUS_ACTIVE = 2;
  WORKFLOW_STATUS_ARCHIVED = 3;
}

message CreateWorkflowRequest {
  string name = 1;
  string description = 2;
  string definition = 3;
}

message GetWorkflowRequest {
  string id = 1;
}

message ListWorkflowsRequest {
  optional int32 limit = 1;
  optional int32 offset = 2;
  optional WorkflowStatus status = 3;
}

message ListWorkflowsResponse {
  repeated Workflow workflows = 1;
  int32 total = 2;
}

message UpdateWorkflowRequest {
  string id = 1;
  optional string name = 2;
  optional string description = 3;
  optional string definition = 4;
  optional WorkflowStatus status = 5;
}

message DeleteWorkflowRequest {
  string id = 1;
}

message DeleteWorkflowResponse {
  bool success = 1;
}

message ExecuteWorkflowRequest {
  string id = 1;
  optional string input = 2;  // JSON
}

message WorkflowEvent {
  string workflow_id = 1;
  WorkflowEventType event_type = 2;
  string timestamp = 3;
  optional string data = 4;
  optional string current_node = 5;
  optional string error = 6;
}

enum WorkflowEventType {
  WORKFLOW_EVENT_TYPE_UNSPECIFIED = 0;
  WORKFLOW_EVENT_TYPE_STARTED = 1;
  WORKFLOW_EVENT_TYPE_NODE_ENTERED = 2;
  WORKFLOW_EVENT_TYPE_NODE_COMPLETED = 3;
  WORKFLOW_EVENT_TYPE_CHECKPOINT = 4;
  WORKFLOW_EVENT_TYPE_COMPLETED = 5;
  WORKFLOW_EVENT_TYPE_FAILED = 6;
}
```

3. **`src/crates/orchestrator/proto/health.proto`**
```protobuf
syntax = "proto3";
package orchestrator.health;

service HealthService {
  rpc Check(HealthCheckRequest) returns (HealthCheckResponse);
}

message HealthCheckRequest {
  string service = 1;
}

message HealthCheckResponse {
  HealthStatus status = 1;
  string version = 2;
  int64 uptime_seconds = 3;
  map<string, string> metadata = 4;
}

enum HealthStatus {
  HEALTH_STATUS_UNSPECIFIED = 0;
  HEALTH_STATUS_SERVING = 1;
  HEALTH_STATUS_NOT_SERVING = 2;
  HEALTH_STATUS_UNKNOWN = 3;
}
```

### Build Configuration

4. **Update `src/crates/orchestrator/Cargo.toml`**:
```toml
[dependencies]
tonic = "0.10"
prost = "0.12"
tokio = { workspace = true }
tokio-stream = "0.1"

[build-dependencies]
tonic-build = "0.10"
```

5. **Create `src/crates/orchestrator/build.rs`**:
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)  // Server doesn't need client
        .compile(
            &[
                "proto/tasks.proto",
                "proto/workflows.proto",
                "proto/health.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
```

6. **Create `src/crates/aco/build.rs`**:
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)  // Client doesn't need server
        .build_client(true)
        .compile(
            &[
                "../orchestrator/proto/tasks.proto",
                "../orchestrator/proto/workflows.proto",
                "../orchestrator/proto/health.proto",
            ],
            &["../orchestrator/proto"],
        )?;
    Ok(())
}
```

## Unit Tests

Create `src/crates/orchestrator/tests/proto_validation.rs`:
```rust
#[test]
fn test_task_status_enum_completeness() {
    // Verify all database TaskStatus values map to proto enum
    use orchestrator::tasks::TaskStatus;

    assert_eq!(TaskStatus::Pending as i32, 1);
    assert_eq!(TaskStatus::Running as i32, 2);
    assert_eq!(TaskStatus::Completed as i32, 3);
    assert_eq!(TaskStatus::Failed as i32, 4);
    assert_eq!(TaskStatus::Cancelled as i32, 5);
}

#[test]
fn test_execution_event_type_coverage() {
    // Ensure all event types are defined
    use orchestrator::tasks::ExecutionEventType;

    let types = vec![
        ExecutionEventType::Started,
        ExecutionEventType::Progress,
        ExecutionEventType::Output,
        ExecutionEventType::Completed,
        ExecutionEventType::Failed,
        ExecutionEventType::ToolCall,
        ExecutionEventType::ToolResult,
    ];

    assert_eq!(types.len(), 7);
}
```

## Acceptance Criteria

- [ ] All 3 .proto files created and compile successfully
- [ ] build.rs files generate Rust code without errors
- [ ] orchestrator generates server stubs
- [ ] aco generates client stubs
- [ ] All enums match database schema
- [ ] Streaming RPCs defined for ExecuteTask and ExecuteWorkflow
- [ ] Optional fields use `optional` keyword
- [ ] Tests validate enum completeness
- [ ] Documentation comments in proto files

## Complexity
**Moderate** - Requires understanding of Protocol Buffers, gRPC patterns, and mapping to existing database schema

## Estimated Effort
**4-6 hours**

## Notes
- Use proto3 syntax (modern, simpler than proto2)
- JSON fields for flexible data (config, metadata, definition)
- Streaming for real-time execution updates
- Follow protobuf naming conventions (snake_case)
- Keep service definitions focused (tasks vs workflows)
