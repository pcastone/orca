# Agent Routing Diagnostic Plan - LangGraph ReAct Bug

## Problem Summary
The ReAct agent graph is routing directly to the `tools` node without executing the `agent` node first. This prevents the LLM from being called and tool execution from working correctly.

**Evidence:**
- Agent node never fires (no "Node update node=agent" log)
- Tools node fires immediately (160μs after LLM function starts)
- LLM function starts but never completes the HTTP call
- Messages array remains empty (count=0) throughout execution

**Expected Flow:**
1. START → agent (LLM call)
2. agent → tools (if tool_calls) or END (if no tool_calls)
3. tools → agent (loop back)

**Actual Flow:**
1. START → tools (incorrect!)
2. tools → END

---

## Phase 1: Isolate the Issue - Build Minimal Reproduction [PRIORITY: CRITICAL]

### Task 1.1: Create Minimal StateGraph Test
**File:** `src/crates/langgraph-core/tests/test_state_graph_routing.rs`

**Purpose:** Verify basic StateGraph routing works correctly

```rust
#[tokio::test]
async fn test_state_graph_basic_routing() {
    let mut graph = StateGraph::new();

    // Track execution order
    let execution_order = Arc::new(Mutex::new(Vec::new()));
    let order_clone = execution_order.clone();

    // Add node A
    graph.add_node("node_a", move |state: Value| {
        let order = order_clone.clone();
        Box::pin(async move {
            order.lock().unwrap().push("node_a");
            Ok(state)
        })
    });

    // Add node B
    let order_clone = execution_order.clone();
    graph.add_node("node_b", move |state: Value| {
        let order = order_clone.clone();
        Box::pin(async move {
            order.lock().unwrap().push("node_b");
            Ok(state)
        })
    });

    // Route: START -> A -> B
    graph.add_edge("__start__", "node_a");
    graph.add_edge("node_a", "node_b");

    let compiled = graph.compile().unwrap();
    let initial_state = json!({"test": "value"});

    compiled.invoke(initial_state).await.unwrap();

    let order = execution_order.lock().unwrap();
    assert_eq!(*order, vec!["node_a", "node_b"], "Nodes should execute in order");
}
```

**Validation:**
- [ ] Run: `cargo test -p langgraph-core test_state_graph_basic_routing`
- [ ] Test passes: Basic routing works ✅
- [ ] Test fails: StateGraph has fundamental routing bug ❌

**Expected Outcome:** If this test **passes**, the issue is specific to ReAct agent. If it **fails**, StateGraph itself is broken.

---

### Task 1.2: Create ReAct Agent Minimal Test
**File:** `src/crates/langgraph-prebuilt/tests/test_react_routing.rs`

**Purpose:** Test ReAct agent routing in isolation

```rust
#[tokio::test]
async fn test_react_agent_calls_agent_node_first() {
    use langgraph_prebuilt::{create_react_agent, Message};
    use std::sync::Arc;
    use std::sync::Mutex;

    // Track which nodes execute
    let nodes_executed = Arc::new(Mutex::new(Vec::new()));
    let nodes_clone = nodes_executed.clone();

    // Create simple LLM function that returns immediately
    let llm_fn = Arc::new(move |state: Value| {
        nodes_clone.lock().unwrap().push("llm_called");
        Box::pin(async move {
            // Return simple AI message with NO tool calls
            Ok(Message::ai("Test response"))
        }) as Pin<Box<dyn Future<Output = _> + Send>>
    });

    // Create agent with no tools
    let agent = create_react_agent(llm_fn, vec![])
        .with_max_iterations(1)
        .build()
        .unwrap();

    // Execute
    let input = json!({
        "messages": vec![Message::human("test")]
    });

    let result = agent.invoke(input).await.unwrap();

    // Verify LLM was called
    let executed = nodes_executed.lock().unwrap();
    assert!(executed.contains(&"llm_called"),
        "LLM function should have been called. Executed: {:?}", *executed);

    // Verify AI message was added
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2, "Should have human + AI message");
    assert_eq!(messages[1]["type"], "ai");
}
```

**Validation:**
- [ ] Run: `cargo test -p langgraph-prebuilt test_react_agent_calls_agent_node_first`
- [ ] Check if LLM is called
- [ ] Check if messages are added to state

**Expected Outcome:** Identifies if the issue is in ReAct graph construction or execution.

---

### Task 1.3: Add Execution Tracing to StateGraph
**File:** `src/crates/langgraph-core/src/compiled/pregel_builder.rs`

**Purpose:** Add detailed logging to track graph execution flow

**Changes:**
1. Add debug logging at graph entry point
2. Log before/after each node execution
3. Log edge traversal
4. Log conditional routing decisions

```rust
// In PregelLoop execution
debug!("Graph execution starting, entry node: {:?}", entry_node);

// Before node execution
debug!("Executing node: {}", node_name);

// After node execution
debug!("Node {} completed, output channels: {:?}", node_name, output);

// Edge routing
debug!("Routing from {} to next node", current_node);
```

**Validation:**
- [ ] Rebuild: `cargo build -p langgraph-core`
- [ ] Run with: `RUST_LOG=langgraph_core=trace ./target/release/orca -p "test"`
- [ ] Examine logs to see exact execution flow

---

## Phase 2: Analyze Graph Construction [PRIORITY: HIGH]

### Task 2.1: Dump ReAct Agent Graph Structure
**File:** `src/crates/langgraph-prebuilt/src/agents/react.rs`

**Purpose:** Add debugging to see compiled graph structure

**Add after graph compilation:**
```rust
// After: graph.compile().map_err(...)
let compiled = graph.compile().map_err(|e| PrebuiltError::ToolExecution(e.to_string()))?;

// DEBUG: Print graph structure
debug!("=== ReAct Agent Graph Structure ===");
debug!("Nodes: {:?}", compiled.get_node_names());
debug!("Entry node: {:?}", compiled.get_entry_node());
debug!("Edges: {:?}", compiled.get_edges());
debug!("=====================================");

Ok(compiled)
```

**Validation:**
- [ ] Add `get_node_names()`, `get_entry_node()`, `get_edges()` methods to CompiledGraph if missing
- [ ] Rebuild and run
- [ ] Verify graph structure matches expected topology

---

### Task 2.2: Test Graph Entry Point
**File:** `src/crates/langgraph-core/tests/test_entry_point.rs`

**Purpose:** Verify __start__ edge routing works

```rust
#[tokio::test]
async fn test_start_edge_routes_correctly() {
    let mut graph = StateGraph::new();
    let executed = Arc::new(Mutex::new(false));
    let exec_clone = executed.clone();

    graph.add_node("first_node", move |state: Value| {
        let exec = exec_clone.clone();
        Box::pin(async move {
            *exec.lock().unwrap() = true;
            Ok(state)
        })
    });

    graph.add_edge("__start__", "first_node");

    let compiled = graph.compile().unwrap();
    compiled.invoke(json!({})).await.unwrap();

    assert!(*executed.lock().unwrap(), "First node should have executed");
}
```

**Validation:**
- [ ] Run test
- [ ] If fails: __start__ routing is broken
- [ ] If passes: Issue is elsewhere

---

## Phase 3: Test Async Node Execution [PRIORITY: HIGH]

### Task 3.1: Test Async LLM Function Execution
**File:** `src/crates/orca/src/executor/tests/test_llm_function.rs`

**Purpose:** Verify LLM function completes correctly when awaited

```rust
#[tokio::test]
async fn test_llm_function_completes() {
    use crate::executor::create_llm_function;
    use crate::config::{OrcaConfig, LlmConfig};
    use std::sync::Arc;

    let config = OrcaConfig {
        llm: LlmConfig {
            provider: "ollama".to_string(),
            model: "gemma3:1b".to_string(),
            api_base: Some("http://localhost:11434".to_string()),
            api_key: None,
            temperature: 0.7,
            max_tokens: 100,
        },
        ..Default::default()
    };

    let provider = Arc::new(LlmProvider::from_config(&config).unwrap());
    let llm_fn = create_llm_function(provider);

    let state = json!({
        "messages": [
            {"type": "human", "content": "Say 'test'"}
        ]
    });

    let start = std::time::Instant::now();
    let result = llm_fn(state).await.unwrap();
    let duration = start.elapsed();

    // LLM call should take at least 100ms (real HTTP call)
    assert!(duration.as_millis() > 100,
        "LLM call took {}ms - too fast, might not be executing",
        duration.as_millis());

    // Should return AI message
    assert!(result.is_ai(), "Should return AI message");
}
```

**Validation:**
- [ ] Run: `cargo test -p orca test_llm_function_completes`
- [ ] Verify LLM function actually awaits HTTP call
- [ ] Check duration is realistic (>100ms)

---

### Task 3.2: Test Node Async Execution in Graph
**File:** `src/crates/langgraph-core/tests/test_async_nodes.rs`

**Purpose:** Verify graph waits for async node completion

```rust
#[tokio::test]
async fn test_graph_waits_for_async_node() {
    use std::time::Duration;

    let mut graph = StateGraph::new();
    let completed = Arc::new(Mutex::new(false));
    let comp_clone = completed.clone();

    // Async node that takes 100ms
    graph.add_node("slow_node", move |state: Value| {
        let comp = comp_clone.clone();
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            *comp.lock().unwrap() = true;
            Ok(state)
        })
    });

    graph.add_edge("__start__", "slow_node");

    let compiled = graph.compile().unwrap();
    let start = std::time::Instant::now();

    compiled.invoke(json!({})).await.unwrap();

    let duration = start.elapsed();

    assert!(*completed.lock().unwrap(), "Node should have completed");
    assert!(duration.as_millis() >= 100,
        "Graph should have waited for async node (took {}ms)",
        duration.as_millis());
}
```

**Validation:**
- [ ] Run test
- [ ] If fails: Graph not waiting for async nodes ❌
- [ ] If passes: Async execution works ✅

---

## Phase 4: Test Conditional Routing [PRIORITY: MEDIUM]

### Task 4.1: Test Conditional Edge Logic
**File:** `src/crates/langgraph-core/tests/test_conditional_edges.rs`

**Purpose:** Verify conditional routing works correctly

```rust
#[tokio::test]
async fn test_conditional_routing() {
    let mut graph = StateGraph::new();
    let path_taken = Arc::new(Mutex::new(String::new()));

    graph.add_node("start", |state: Value| {
        Box::pin(async move { Ok(state) })
    });

    let path_a = path_taken.clone();
    graph.add_node("path_a", move |state: Value| {
        let path = path_a.clone();
        Box::pin(async move {
            *path.lock().unwrap() = "a".to_string();
            Ok(state)
        })
    });

    let path_b = path_taken.clone();
    graph.add_node("path_b", move |state: Value| {
        let path = path_b.clone();
        Box::pin(async move {
            *path.lock().unwrap() = "b".to_string();
            Ok(state)
        })
    });

    graph.add_edge("__start__", "start");

    // Conditional: if state["go"] == "a" -> path_a, else -> path_b
    let condition = |state: &Value| {
        if state["go"].as_str() == Some("a") {
            ConditionalEdgeResult::Node("path_a".to_string())
        } else {
            ConditionalEdgeResult::Node("path_b".to_string())
        }
    };

    let mut branches = HashMap::new();
    branches.insert("path_a".to_string(), "path_a".to_string());
    branches.insert("path_b".to_string(), "path_b".to_string());
    graph.add_conditional_edge("start", condition, branches);

    let compiled = graph.compile().unwrap();

    // Test path A
    let result = compiled.invoke(json!({"go": "a"})).await.unwrap();
    assert_eq!(*path_taken.lock().unwrap(), "a");
}
```

**Validation:**
- [ ] Run test
- [ ] Verify conditional logic routes correctly
- [ ] Check if ReAct agent's `should_continue` function works

---

## Phase 5: Fix the Bug [PRIORITY: CRITICAL]

### Task 5.1: Identify Root Cause
Based on test results, determine if issue is:
- [ ] StateGraph entry point routing
- [ ] ReAct agent graph construction
- [ ] Async node execution not awaited
- [ ] Conditional routing logic
- [ ] State channel updates
- [ ] Streaming vs invoke execution

### Task 5.2: Implement Fix
**Location:** TBD based on root cause

**Possible Fixes:**
1. **If entry point issue:** Fix `add_edge("__start__", ...)` handling in `StateGraph::compile()`
2. **If async issue:** Ensure `PregelLoop` awaits node futures properly
3. **If routing issue:** Fix conditional edge evaluation in graph execution
4. **If state issue:** Fix state channel reading/writing

### Task 5.3: Add Regression Tests
Create comprehensive tests to prevent this bug from recurring:
- [ ] Test ReAct agent with tools
- [ ] Test ReAct agent without tools
- [ ] Test streaming execution
- [ ] Test non-streaming execution
- [ ] Test with real LLM calls

### Task 5.4: Validate Fix
**End-to-End Test:**
```bash
# Should now work with tools
./target/release/orca -p "List files in current directory"

# Should see agent node execution in logs
RUST_LOG=debug ./target/release/orca -p "test" 2>&1 | grep "Node update"
# Expected: "Node update node=agent" appears BEFORE "Node update node=tools"
```

**Validation:**
- [ ] Agent node executes first
- [ ] LLM is called and completes
- [ ] Tools are available to agent
- [ ] Response includes tool usage
- [ ] No "No response generated" error

---

## Success Criteria

✅ All unit tests pass
✅ Agent node executes before tools node
✅ LLM function completes HTTP call
✅ Messages are added to state correctly
✅ `orca -p "List files"` successfully uses tools
✅ No "No response generated" errors

---

## Current Status

**Completed:**
- [x] DirectToolBridge implementation (6 tools)
- [x] Tool schemas and execution
- [x] LLM provider integration
- [x] Debug logging added to LLM function

**In Progress:**
- [ ] Phase 1: Build minimal reproduction tests
- [ ] Phase 2: Analyze graph construction
- [ ] Phase 3: Test async execution
- [ ] Phase 4: Test conditional routing
- [ ] Phase 5: Fix the bug

**Next Steps:**
1. Start with Task 1.1 - Create minimal StateGraph test
2. Run tests to isolate the issue
3. Add execution tracing
4. Identify root cause
5. Implement fix
