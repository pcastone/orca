# Plan: Address Stub Functions and Unimplemented Features

## Overview
Complete implementation of 40+ stub functions across the codebase, organized by priority and dependency.

**Created:** 2025-11-24
**Status:** Planning

---

## Phase 1: LLM Streaming Support [PRIORITY: HIGH]
**Impact: All 9 LLM providers lack streaming**

### Task 1.1: Implement Base Streaming Infrastructure
- [ ] Create `ChatStreamResponse` handling in llm crate
- [ ] Define streaming chunk format and error handling

### Task 1.2: Implement Local Provider Streaming
- [ ] `src/crates/llm/src/local/ollama.rs:180` - Ollama streaming
- [ ] `src/crates/llm/src/local/lmstudio.rs:174` - LM Studio streaming
- [ ] `src/crates/llm/src/local/llama_cpp.rs:171` - Llama.cpp streaming

### Task 1.3: Implement Remote Provider Streaming
- [ ] `src/crates/llm/src/remote/openai.rs:210` - OpenAI streaming
- [ ] `src/crates/llm/src/remote/claude.rs:212` - Claude streaming
- [ ] `src/crates/llm/src/remote/deepseek.rs:212` - DeepSeek streaming
- [ ] `src/crates/llm/src/remote/gemini.rs:226` - Gemini streaming
- [ ] `src/crates/llm/src/remote/grok.rs:165` - Grok streaming
- [ ] `src/crates/llm/src/remote/openrouter.rs:193` - OpenRouter streaming

### Task 1.4: Add Streaming Tests
- [ ] Unit tests for each provider's stream() method
- [ ] Integration test with real API (Ollama)

---

## Phase 2: gRPC Client Integration [PRIORITY: HIGH]
**Impact: ACO TUI cannot communicate with orchestrator**

### Task 2.1: Fix Orchestrator Proto Client
- [ ] `src/crates/aco/src/tui/grpc_client.rs:52` - Replace mock `fetch_tasks()`
- [ ] `src/crates/aco/src/tui/grpc_client.rs:102` - Replace mock `fetch_workflows()`

### Task 2.2: Implement Real Execution Calls
- [ ] `src/crates/aco/src/tui/grpc_client.rs:125` - Real `execute_task()` gRPC call
- [ ] `src/crates/aco/src/tui/grpc_client.rs:186` - Real `execute_workflow()` gRPC call

### Task 2.3: Add Error Handling and Retry Logic
- [ ] Connection retry on failure
- [ ] Timeout handling
- [ ] Status code interpretation

---

## Phase 3: Expression Evaluation Engine [PRIORITY: MEDIUM]
**Impact: Router/workflow conditions don't work**

### Task 3.1: Create Expression Evaluator
- [ ] Design expression syntax (e.g., `result.success`, `state.count > 5`)
- [ ] Implement parser using nom or pest

### Task 3.2: Integrate with Router
- [ ] `src/crates/orchestrator/src/router/evaluator.rs:180` - Implement `evaluate_expression()`
- [ ] `src/crates/orchestrator/src/router/supervisor.rs:148` - Implement expression termination

### Task 3.3: Integrate with Workflow Executor
- [ ] `src/crates/orchestrator/src/workflow/executor.rs:166,211` - Full `evaluate_condition_expr()`

### Task 3.4: Add Expression Tests
- [ ] Test various expression patterns
- [ ] Test error cases (invalid syntax, missing fields)

---

## Phase 4: DirectToolBridge Implementation [PRIORITY: MEDIUM]
**Impact: Tool execution disabled in Orca**

### Task 4.1: Complete Tooling Crate Runtime
- [ ] Implement missing runtime module in tooling crate
- [ ] Implement tools module with base tool definitions

### Task 4.2: Implement DirectToolBridge Methods
- [ ] `src/crates/orca/src/tools/mod.rs:37` - `execute_tool()`
- [ ] `src/crates/orca/src/tools/mod.rs:42` - `list_tools()`
- [ ] `src/crates/orca/src/tools/mod.rs:57` - `get_tool_schema()`
- [ ] `src/crates/orca/src/tools/mod.rs:62` - `get_all_schemas()`

### Task 4.3: Enable Direct Bridge Module
- [ ] Uncomment `mod direct_bridge` in tools/mod.rs
- [ ] Wire up to application

### Task 4.4: Add Tool Execution Tests
- [ ] `src/crates/orca/src/tools/direct_bridge.rs:699,729` - Complete verification logic

---

## Phase 5: Authentication System [PRIORITY: MEDIUM]
**Impact: No security - all requests allowed**

### Task 5.1: Implement User Login Authentication
- [ ] `src/crates/orchestrator/src/config/server/security.rs:69`
- [ ] Token/session management
- [ ] Password hashing and verification

### Task 5.2: Implement LDAP Group Check
- [ ] `src/crates/orchestrator/src/config/server/ldap.rs:125` - Real `is_in_group()` query
- [ ] LDAP search with proper filter
- [ ] Group membership verification

### Task 5.3: Add Auth Middleware
- [ ] JWT or session-based auth
- [ ] Role-based access control

---

## Phase 6: Task Execution Engine [PRIORITY: MEDIUM]
**Impact: Tasks simulated, not actually executed**

### Task 6.1: Implement Real Task Execution
- [ ] `src/crates/orchestrator/src/services/task.rs:294`
- [ ] Replace mock events with TaskExecutionEngine calls
- [ ] Wire up to workflow engine

### Task 6.2: Add Workflow LLM Streaming
- [ ] `src/crates/orchestrator/src/workflow/llm_executor.rs:207`
- [ ] Implement stream() for workflow context

---

## Phase 7: Advanced LLM Features [PRIORITY: LOW]
**Impact: Missing tool/image support in prompts**

### Task 7.1: Tool Definitions in Messages
- [ ] `src/crates/llm/src/remote/openai.rs:621` - Add tool definitions
- [ ] `src/crates/llm/src/remote/claude.rs:595` - Add tool definitions

### Task 7.2: Image Content Handling
- [ ] `src/crates/llm/src/remote/openai.rs:641` - Image message creation
- [ ] `src/crates/llm/src/remote/claude.rs:620` - Image message creation
- [ ] `src/crates/llm/src/remote/gemini.rs:642` - Image message creation

### Task 7.3: Provider-Specific Features
- [ ] `src/crates/llm/src/remote/claude.rs:648` - Thinking tags support
- [ ] `src/crates/llm/src/remote/deepseek.rs:718` - R1 streaming

---

## Phase 8: Testing & Documentation [PRIORITY: LOW]

### Task 8.1: Enable Ignored Tests
- [ ] `src/crates/orca/src/testing/mod.rs:111` - `test_database_creation()`
- [ ] `src/crates/orca/src/testing/mod.rs:119` - `test_database_cleanup()`
- [ ] Add proper test fixtures for ~/.orca directory

### Task 8.2: Pregel Advanced Features
- [ ] `src/crates/langgraph-core/src/pregel/loop_impl.rs:1130` - Per-node retry policies
- [ ] `src/crates/langgraph-core/src/pregel/loop_impl.rs:2164,2222` - Dynamic task execution

### Task 8.3: Documentation Updates
- [ ] Document new streaming APIs
- [ ] Document expression syntax
- [ ] Update auth configuration docs

---

## Implementation Order

1. **Phase 2 (gRPC)** - Unblocks ACO TUI
2. **Phase 1 (Streaming)** - High user demand
3. **Phase 3 (Expressions)** - Enables complex workflows
4. **Phase 6 (Task Execution)** - Core functionality
5. **Phase 5 (Authentication)** - Security requirement
6. **Phase 4 (Tools)** - Depends on tooling crate
7. **Phase 7 (Advanced LLM)** - Nice to have
8. **Phase 8 (Testing)** - Cleanup

---

## Success Criteria

- [ ] All `unimplemented!()` and `todo!()` macros removed
- [ ] All TODO comments addressed or converted to GitHub issues
- [ ] gRPC client connects to real orchestrator
- [ ] LLM streaming works for at least 3 providers
- [ ] Expression evaluation handles common patterns
- [ ] Authentication middleware functional
- [ ] All tests pass (including previously ignored)
- [ ] No mock data in production code paths

---

## Summary Statistics

| Category | Count |
|----------|-------|
| **`unimplemented!()` calls** | 7 |
| **`todo!()` macros** | 4 |
| **TODO comments** | 20+ |
| **Mock/Stub implementations** | 7 |
| **Ignored tests** | 2 |
| **Total stub items** | 40+ |

---

## Quick Reference - Files with Stubs

### Orchestrator
- `src/crates/orchestrator/src/router/evaluator.rs:180`
- `src/crates/orchestrator/src/router/supervisor.rs:148`
- `src/crates/orchestrator/src/workflow/executor.rs:166,211`
- `src/crates/orchestrator/src/workflow/llm_executor.rs:207`
- `src/crates/orchestrator/src/config/server/security.rs:69`
- `src/crates/orchestrator/src/config/server/ldap.rs:125`
- `src/crates/orchestrator/src/services/task.rs:294`

### ACO
- `src/crates/aco/src/tui/grpc_client.rs:52,102,125,186`

### Orca
- `src/crates/orca/src/tools/mod.rs:27-65`
- `src/crates/orca/src/tools/direct_bridge.rs:699,729`
- `src/crates/orca/src/testing/mod.rs:111,119`

### LLM (all providers)
- `src/crates/llm/src/local/ollama.rs:180`
- `src/crates/llm/src/local/lmstudio.rs:174`
- `src/crates/llm/src/local/llama_cpp.rs:171`
- `src/crates/llm/src/remote/openai.rs:210,621,641`
- `src/crates/llm/src/remote/claude.rs:212,595,620,648`
- `src/crates/llm/src/remote/deepseek.rs:212,718`
- `src/crates/llm/src/remote/gemini.rs:226,642`
- `src/crates/llm/src/remote/grok.rs:165`
- `src/crates/llm/src/remote/openrouter.rs:193`

### LangGraph
- `src/crates/langgraph-core/src/pregel/loop_impl.rs:1130,2164,2222`
