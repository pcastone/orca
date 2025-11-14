# Unit Testing Plan for Orca Project

## Overview
Comprehensive unit testing plan covering all 10 crates with ~500+ identified testing gaps across 326 source files.

**Current State:**
- Existing tests: ~1,313 test functions (846 #[test] + 467 #[tokio::test])
- Estimated coverage: 30-40%
- Target coverage: 80%+ for critical paths, 100% for security code

---

## Phase 1: Critical Security Tests (PRIORITY: CRITICAL)

### 1.1 Tooling Crate - Filesystem Security
- [ ] Test path traversal prevention in `tools/filesystem.rs`
- [ ] Test symlink handling and detection
- [ ] Test permission checks for file operations
- [ ] Test directory escape attempts
- [ ] Test file operation sandboxing

### 1.2 Orca - Permission Enforcement
- [ ] Test permission checks in `tools/permission_enforcer.rs`
- [ ] Test path traversal prevention
- [ ] Test whitelist validation
- [ ] Test audit logging for denied operations
- [ ] Test permission edge cases (symlinks, relative paths)

### 1.3 Orca - Tool Sandboxing
- [ ] Test sandboxing in `tools/direct_bridge.rs`
- [ ] Test resource isolation
- [ ] Test escape attempt detection
- [ ] Test error propagation from sandboxed tools

### 1.4 Aco - Workspace Security
- [ ] Test sandbox enforcement in `workspace/security.rs`
- [ ] Test escape attempt detection and blocking
- [ ] Test path validation with malicious inputs
- [ ] Test security rule enforcement across operations

---

## Phase 2: Core Execution Engine (PRIORITY: CRITICAL)

### 2.1 Langgraph-Core - Pregel Execution
- [ ] Test superstep execution with parallel nodes in `pregel/executor.rs`
- [ ] Test barrier synchronization correctness
- [ ] Test parallel node execution thread safety
- [ ] Test message passing between supersteps
- [ ] Test checkpoint creation after each superstep

### 2.2 Langgraph-Core - State Management
- [ ] Test concurrent reducer application in `state.rs`
- [ ] Test `MergeReducer` deep merge with complex objects
- [ ] Test reducer thread safety under concurrent updates
- [ ] Test schema validation with invalid types
- [ ] Test type coercion error handling

### 2.3 Langgraph-Core - Graph Compilation
- [ ] Test cycle detection in `builder.rs`
- [ ] Test orphaned node detection
- [ ] Test graph validation edge cases
- [ ] Test conditional edge routing logic
- [ ] Test invalid graph configurations

---

## Phase 3: Checkpoint & Recovery (PRIORITY: HIGH)

### 3.1 Langgraph-Checkpoint - Concurrent Access
- [ ] Test concurrent checkpoint writes in `memory.rs`
- [ ] Test concurrent channel updates in `channels.rs`
- [ ] Test checkpoint list operations under load
- [ ] Test memory cleanup strategies

### 3.2 Langgraph-Checkpoint - Channel Operations
- [ ] Test barrier signal ordering in `channels_extended.rs`
- [ ] Test ephemeral state persistence
- [ ] Test concurrent channel updates thread safety
- [ ] Test channel serialization edge cases

### 3.3 Langgraph-Core - Recovery Scenarios
- [ ] Test resume from checkpoint in `compiled/mod.rs`
- [ ] Test error recovery mechanisms
- [ ] Test state editing during interrupt
- [ ] Test multiple interrupt handling
- [ ] Test state snapshot consistency

---

## Phase 4: LLM Integration (PRIORITY: HIGH)

### 4.1 LLM - OpenAI Provider
- [ ] Test streaming in `remote/openai.rs`
- [ ] Test tool calling functionality
- [ ] Test function calling
- [ ] Test vision/multi-modal inputs
- [ ] Test o1 reasoning mode
- [ ] Test error handling and retries

### 4.2 LLM - Claude Provider
- [ ] Test streaming in `remote/claude.rs`
- [ ] Test tool use functionality
- [ ] Test vision support
- [ ] Test thinking tags extraction
- [ ] Test error handling

### 4.3 LLM - Deepseek Provider
- [ ] Test R1 reasoning mode in `remote/deepseek.rs`
- [ ] Test reasoning extraction
- [ ] Test streaming reasoning output
- [ ] Test error handling

### 4.4 LLM - Local Providers
- [ ] Test Ollama streaming in `local/ollama.rs`
- [ ] Test Ollama connection retry logic
- [ ] Test llama.cpp model loading in `local/llama_cpp.rs`
- [ ] Test llama.cpp streaming
- [ ] Test LM Studio API compatibility

### 4.5 LLM - Other Remote Providers
- [ ] Test Gemini multi-modal inputs in `remote/gemini.rs`
- [ ] Test Gemini streaming
- [ ] Test OpenRouter multi-provider routing
- [ ] Test OpenRouter model selection

---

## Phase 5: Database & Persistence (PRIORITY: HIGH)

### 5.1 Orca - Database Operations
- [ ] Test database migration handling in `db/manager.rs`
- [ ] Test transaction handling
- [ ] Test connection pooling
- [ ] Test concurrent access patterns

### 5.2 Orca - Repository Concurrent Access
- [ ] Test concurrent TaskRepository operations in `repositories/task.rs`
- [ ] Test concurrent WorkflowRepository operations in `repositories/workflow.rs`
- [ ] Test foreign key constraint enforcement
- [ ] Test query methods with edge cases

### 5.3 Orchestrator - Repository Operations
- [ ] Test transaction handling across repositories
- [ ] Test concurrent access in all repositories
- [ ] Test cascade delete operations
- [ ] Test query methods with complex filters

---

## Phase 6: Workflow & Task Execution (PRIORITY: HIGH)

### 6.1 Orca - Workflow Execution
- [ ] Test task coordination in `workflow.rs`
- [ ] Test state transitions
- [ ] Test failure recovery mechanisms
- [ ] Test workflow resumption after error

### 6.2 Orca - Task Executor
- [ ] Test resource cleanup in `executor/task_executor.rs`
- [ ] Test timeout enforcement edge cases
- [ ] Test retry logic with various error types
- [ ] Test error handling paths

### 6.3 Orchestrator - Workflow Engine
- [ ] Test multi-task coordination in `execution/workflow_engine.rs`
- [ ] Test checkpointing during execution
- [ ] Test resume from checkpoint
- [ ] Test error propagation across tasks

### 6.4 Orchestrator - Task Engine
- [ ] Test parallel execution in `execution/task_engine.rs`
- [ ] Test task cancellation
- [ ] Test error handling with partial failures

---

## Phase 7: Communication & Streaming (PRIORITY: HIGH)

### 7.1 Aco - WebSocket Client
- [ ] Test WebSocket connection in `client.rs`
- [ ] Test reconnection logic
- [ ] Test message handling
- [ ] Test connection error scenarios

### 7.2 Orchestrator - Streaming
- [ ] Test backpressure handling in `execution/streaming.rs`
- [ ] Test stream error recovery
- [ ] Test client disconnection handling
- [ ] Test concurrent stream consumers

### 7.3 Utils - HTTP Client Retry
- [ ] Test retry logic in `client/mod.rs`
- [ ] Test backoff calculation
- [ ] Test connection timeout scenarios
- [ ] Test HTTP methods (get, post_json)

---

## Phase 8: Agent Patterns (PRIORITY: HIGH)

### 8.1 Langgraph-Prebuilt - ReAct Agent
- [ ] Test ReAct agent creation in `agents/react.rs`
- [ ] Test tool selection logic
- [ ] Test loop termination conditions
- [ ] Test error recovery in ReAct cycle

### 8.2 Langgraph-Prebuilt - Plan-Execute
- [ ] Test plan generation in `agents/plan_execute.rs`
- [ ] Test step execution
- [ ] Test replanning logic
- [ ] Test plan validation

### 8.3 Langgraph-Prebuilt - Reflection
- [ ] Test generation-critique cycle in `agents/reflection.rs`
- [ ] Test quality assessment
- [ ] Test iteration limits

### 8.4 Langgraph-Prebuilt - Tool Node
- [ ] Test tool execution node in `tool_node.rs`
- [ ] Test parallel tool execution
- [ ] Test error handling in tool execution

---

## Phase 9: Configuration & Validation (PRIORITY: MEDIUM)

### 9.1 Orca - Configuration
- [ ] Test config merging in `config/loader.rs`
- [ ] Test user-level config loading
- [ ] Test project-level config loading
- [ ] Test file not found handling
- [ ] Test environment variable expansion

### 9.2 Utils - Config Loading
- [ ] Test YAML config loading in `config/mod.rs`
- [ ] Test JSON config loading
- [ ] Test auto-detect config file format
- [ ] Test invalid file paths
- [ ] Test malformed YAML/JSON
- [ ] Test env var parsing edge cases

### 9.3 Aco - Config Management
- [ ] Test config loading in `config/loader.rs`
- [ ] Test config merging priority
- [ ] Test validation rules

---

## Phase 10: Message & State Management (PRIORITY: MEDIUM)

### 10.1 Langgraph-Core - Messages
- [ ] Test tool call/result matching in `messages.rs`
- [ ] Test message ID collision handling
- [ ] Test `RemoveMessage` functionality
- [ ] Test `merge_consecutive_messages()` edge cases
- [ ] Test message deduplication

### 10.2 Langgraph-Core - Graph Operations
- [ ] Test conditional edge routing in `graph.rs`
- [ ] Test channel management
- [ ] Test nested graph execution in `subgraph.rs`
- [ ] Test parent-child communication
- [ ] Test state isolation in subgraphs

---

## Phase 11: Error Handling & Edge Cases (PRIORITY: MEDIUM)

### 11.1 Tooling - Async Utilities
- [ ] Test jitter calculation in `async_utils/retry.rs`
- [ ] Test retry predicate logic
- [ ] Test cancellation propagation in `async_utils/timeout.rs`
- [ ] Test early completion handling

### 11.2 Tooling - Rate Limiting
- [ ] Test concurrent access in `rate_limit/mod.rs`
- [ ] Test sliding window under load
- [ ] Test rate limit reset
- [ ] Test token bucket edge cases

### 11.3 Langgraph-Core - Execution Edge Cases
- [ ] Test infinite loop detection in `pregel/loop_impl.rs`
- [ ] Test max iterations limit
- [ ] Test interrupt handling edge cases
- [ ] Test concurrent cache access in `cache.rs`

### 11.4 Error Types Across Crates
- [ ] Test Utils error conversions in `utils/error.rs`
- [ ] Test error message formatting
- [ ] Test error propagation chains

---

## Phase 12: Advanced Features (PRIORITY: MEDIUM)

### 12.1 Orca - Pattern Execution
- [ ] Test ReAct pattern in `pattern.rs`
- [ ] Test Plan-Execute pattern
- [ ] Test Reflection pattern
- [ ] Test pattern selection logic

### 12.2 Orca - LLM Provider Management
- [ ] Test provider selection in `executor/llm_provider.rs`
- [ ] Test API key validation
- [ ] Test rate limiting per provider
- [ ] Test error retry strategies

### 12.3 Orchestrator - Advanced Features
- [ ] Test LLM routing in `router/llm_router.rs`
- [ ] Test load balancing
- [ ] Test failover mechanisms
- [ ] Test plan generation in `pattern/llm_planner.rs`
- [ ] Test plan validation
- [ ] Test plan execution

### 12.4 Orchestrator - Context Management
- [ ] Test concurrent contexts in `context/manager.rs`
- [ ] Test context cleanup
- [ ] Test context isolation

---

## Phase 13: CLI & TUI (PRIORITY: MEDIUM)

### 13.1 Aco - TUI Components
- [ ] Test input handling in `tui/app.rs`
- [ ] Test view switching
- [ ] Test async event stream in `tui/events.rs`
- [ ] Test event debouncing

### 13.2 Aco - CLI Handlers
- [ ] Test command handling in `cli/handlers.rs`
- [ ] Test error formatting
- [ ] Test output rendering

### 13.3 Aco - Session Management
- [ ] Test session management in `session.rs`
- [ ] Test session cleanup
- [ ] Test concurrent sessions

### 13.4 Langgraph-CLI - Project Operations
- [ ] Test `init_project()` in `main.rs`
- [ ] Test `create_graph()`
- [ ] Test `validate_yaml()`
- [ ] Test template generation
- [ ] Test file creation

---

## Phase 14: Workspace & Auth (PRIORITY: MEDIUM)

### 14.1 Aco - Workspace
- [ ] Test permission setup in `workspace/initializer.rs`
- [ ] Test Git integration
- [ ] Test directory structure creation

### 14.2 Aco - Authentication
- [ ] Test token refresh in `auth.rs`
- [ ] Test expiry handling edge cases
- [ ] Test token caching

### 14.3 Orchestrator - Authentication
- [ ] Test token expiry in `services/auth.rs`
- [ ] Test permission checks
- [ ] Test authentication edge cases

---

## Phase 15: Utilities & Tools (PRIORITY: LOW)

### 15.1 Tooling - Tools
- [ ] Test shell command execution in `tools/shell.rs`
- [ ] Test environment isolation
- [ ] Test output capture
- [ ] Test timeout handling
- [ ] Test Git operations in `tools/git.rs`
- [ ] Test repository detection

### 15.2 Langgraph-Core - Visualization
- [ ] Test ASCII rendering in `visualization.rs`
- [ ] Test edge cases in DOT format
- [ ] Test edge cases in Mermaid format

### 15.3 Langgraph-Core - YAML Support
- [ ] Test YAML parsing in `yaml.rs`
- [ ] Test graph construction from YAML
- [ ] Test validation
- [ ] Test error messages

### 15.4 Utils - Server Configuration
- [ ] Test invalid socket address handling in `server/mod.rs`
- [ ] Test `socket_addr()` edge cases
- [ ] Test `from_env()` with missing vars

---

## Testing Infrastructure Setup

### Test Utilities to Build
- [ ] Create mock LLM client for testing without API calls
- [ ] Create temporary database helper for SQLite tests
- [ ] Create async test helpers (timeout, concurrency)
- [ ] Create file system sandbox for safe testing
- [ ] Create WebSocket mock server for client testing
- [ ] Create checkpoint builder helpers
- [ ] Create graph builder helpers for common patterns

### Integration Test Suites
- [ ] End-to-end workflow execution test
- [ ] Multi-provider LLM switching test
- [ ] Checkpoint save/restore cycle test
- [ ] Tool permission enforcement integration test
- [ ] Concurrent task execution test
- [ ] Stream backpressure handling test
- [ ] Database migration compatibility test

---

## Success Metrics

- **Code Coverage Target**: 80%+ for critical paths
- **Security Coverage**: 100% for permission/sandbox code
- **Concurrency Coverage**: All shared state tested concurrently
- **Error Path Coverage**: 70%+ for error scenarios
- **Integration Tests**: 20+ end-to-end scenarios

---

## Notes

- Each checkbox represents a focused testing task
- Tasks are ordered by priority: Critical → High → Medium → Low
- Security-related tests (Phases 1, 2) should be completed first
- Each phase can be worked on incrementally
- Tests should follow existing patterns in each crate
- All new tests should include documentation
- Use `cargo test` after each task to ensure no regressions
- Commit after completing each sub-section

---

## Estimated Effort

- **Phase 1 (Security)**: 2 weeks
- **Phase 2 (Core Engine)**: 2 weeks
- **Phase 3 (Checkpoints)**: 1 week
- **Phase 4 (LLM)**: 2 weeks
- **Phase 5 (Database)**: 1 week
- **Phase 6 (Workflow)**: 1.5 weeks
- **Phase 7 (Communication)**: 1 week
- **Phase 8 (Agents)**: 1 week
- **Phase 9 (Config)**: 1 week
- **Phase 10 (Messages)**: 1 week
- **Phase 11 (Error Handling)**: 1 week
- **Phase 12 (Advanced)**: 1.5 weeks
- **Phase 13 (CLI/TUI)**: 1 week
- **Phase 14 (Workspace)**: 1 week
- **Phase 15 (Utilities)**: 1 week

**Total Estimated Time**: ~18-20 weeks for complete coverage

---

## Getting Started

1. Review this plan and confirm priorities
2. Set up testing infrastructure (mock utilities)
3. Begin with Phase 1 (Critical Security)
4. Test and commit after each task
5. Run `cargo test` frequently to catch regressions
6. Update this document as tasks are completed
