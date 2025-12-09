# Agentic Coding Orchestrator - Project Plan

## Executive Summary

**Goal:** Build a distributed agentic coding system where an orchestrator (server) coordinates LLM-driven workflows and a client (aco) executes development tools in a secure, policy-governed environment.

**Architecture:** Two-component system communicating over WebSocket
- **Orchestrator (Server):** LLM coordination, pattern execution, action interpretation
- **aco (Client):** Tool execution, policy enforcement, workspace management

**Expected Load:** 40-100 tool calls per task, WebSocket transport, local LLM primary

**Status:** ~60% complete - Orchestrator crate exists, Tool Runtime SDK specified, integration layer needed

---

## Current State Assessment

### ✅ What You Have (Complete)

**Orchestrator Crate (~3,500 LOC)**
- ✅ YAML-based pattern configuration (9 pattern types)
- ✅ Pattern registry and factory
- ✅ Router/supervisor for dynamic pattern selection
- ✅ Workflow executor for multi-step orchestration
- ✅ Integration with langgraph-core
- ✅ 99 tests passing

**LLM Crate**
- ✅ Local providers (Ollama, llama.cpp, LM Studio)
- ✅ Remote providers (Claude, OpenAI, Gemini, etc.)
- ✅ Provider utilities (ping, model switching)
- ✅ Thinking model support (DeepSeek R1, o1)

**Tooling Crate**
- ✅ Configuration management
- ✅ Error handling and validation
- ✅ Async utilities (retry, timeout)
- ✅ Rate limiting
- ✅ Logging infrastructure

**Tool Runtime SDK (IMPLEMENTED)**
- ✅ Complete tool catalog (filesystem, git, AST, shell, network)
- ✅ Policy/validation rules defined
- ✅ JSON message protocol specified
- ✅ Registry schema defined
- ✅ **Full Rust implementation in `crates/tooling/`**
  - Tool registry and runtime
  - Filesystem tools (file_read, file_write, fs_list, etc.)
  - Git tools (git_status, git_diff, git_commit, etc.)
  - Shell tools (shell_exec)
  - AST tools
  - Network tools
  - Policy enforcement
  - Session management
  - Audit logging

### ⚠️ What's Missing (Implementation Gaps)

**Critical Missing Components:**

1. **Action Interpreter** (NEW - Core component)
   - Parse LLM natural language output
   - Map intent → structured ToolRequest
   - Handle ambiguity and validation
   - NOT IMPLEMENTED

2. **WebSocket Layer**
   - Orchestrator WebSocket client
   - aco WebSocket server
   - Session management
   - NOT IMPLEMENTED

3. **aco Client Application**
   - CLI/daemon that hosts Tool Runtime
   - Workspace initialization
   - WebSocket server integration
   - NOT IMPLEMENTED

4. **Result Formatter**
   - Convert ToolResponse → natural language for LLM
   - Context window management
   - NOT IMPLEMENTED

5. **Integration Layer**
   - PatternToolBridge to connect orchestrator patterns with aco client
   - NOT IMPLEMENTED

---

## Architecture Overview

### High-Level 4-Tier Architecture

The acolib system is designed as a four-tier architecture separating concerns between user interface, API layer, business logic, and persistence:

```
┌──────────────────────────────────────────────────────────────────┐
│  TIER 1: USER INTERFACES                                         │
│  ┌──────────────────────┐         ┌──────────────────────┐       │
│  │   TUI (ratatui)      │         │   Web UI (Svelte)    │       │
│  │  ┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈   │         │  ┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈   │       │
│  │ • Task Dashboard     │         │ • Task Dashboard     │       │
│  │ • Tool Monitor       │         │ • Workflow Builder   │       │
│  │ • Log Viewer         │         │ • Real-time Monitor  │       │
│  │ • Config Editor      │         │ • Analytics Charts 
        • LLM                         • Config Editor   
          LLM budget         │              • LLM 
        • prompt                            • LLM budget
        • ReAct COT/TOTO finetune           • prompt  
                                            • ReAct COT/TOTO finetune 
│  └──────────────────────┘                                        │       
└──────────────────────────────────────────────────────────────────┘
              │                                 │
              │ WebSocket                       │ REST + WebSocket
              ▼                                 ▼
┌──────────────────────────────────────────────────────────────────┐
│  TIER 2: API LAYER                                               │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │         orchestrator API Gateway (axum)                 │    │
│  │  ┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈  │    │
│  │  • REST Endpoints (Tasks, Workflows, Tools)            │    │
│  │  • WebSocket Server (Real-time updates)                │    │
│  │  • Request Validation                                  │    │
│  │  • Response Formatting                                 │    │
│  │  • Error Handling                                      │    │
│  └─────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────┐
│  TIER 3: BUSINESS LOGIC                                          │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  orchestrator Service (Extended)                          │  │
│  │  ┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈  │  │
│  │  ┌─────────────────────────┐  ┌─────────────────────┐  │  │
│  │  │  Pattern Executor       │  │ Action Interpreter  │  │  │
│  │  │  • Pattern Registry     │  │ • Intent Parser     │  │  │
│  │  │  • Router/Supervisor    │  │ • Tool Mapper       │  │  │
│  │  │  • Workflow Executor    │  │ • Result Formatter  │  │  │
│  │  │  • langgraph Runtime    │  │ • Validator         │  │  │
│  │  └─────────────────────────┘  └─────────────────────┘  │  │
│  │  ┌─────────────────────────┐  ┌─────────────────────┐  │  │
│  │  │  LLM Manager            │  │ Task Manager        │  │  │
│  │  │  • Provider Pool        │  │ • Lifecycle         │  │  │
│  │  │  • Context Manager      │  │ • State Tracking    │  │  │
│  │  │  • Prompt Templates     │  │ • Dependencies      │  │  │
│  │  └─────────────────────────┘  └─────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  aco Tool Execution Client                                │  │
│  │  ┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈  │  │
│  │  • WebSocket Server (Receives tool requests)             │  │
│  │  • Session Manager                                        │  │
│  │  • Workspace Context                                      │  │
│  │  ┌─────────────────────────────────────────────────────┐ │  │
│  │  │  Tool Runtime SDK (from tooling crate)              │ │  │
│  │  │  • 27 Production Tools (fs, git, AST, shell, net)  │ │  │
│  │  │  • Policy Enforcer (5 policy types)                │ │  │
│  │  │  • Validator Engine (17 validation rules)          │ │  │
│  │  │  • Lock Manager (resource locking)                 │ │  │
│  │  │  • Audit Logger                                     │ │  │
│  │  └─────────────────────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌──────────────────────────────────────────────────────────────────┐
│  TIER 4: PERSISTENCE LAYER                                       │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │         SQLite Database (via sqlx)                      │    │
│  │  ┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈┈  │    │
│  │  • tasks (task definitions, status, metadata)          │    │
│  │  • workflows (multi-step workflows, state)             │    │
│  │  • tool_executions (audit log, results, timing)        │    │
│  │  • sessions (active sessions, context)                 │    │
│  │  • configurations (system configs, policies)           │    │
│  └─────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────┘
```

### Communication Flows

**1. TUI → aco (Local Tool Execution)**
```
TUI (ratatui) → WebSocket → aco → Tool Runtime → Result → TUI
```
- Direct WebSocket connection for low-latency local tool execution
- Used for: file operations, git commands, shell execution
- No network overhead, ~5-50ms per tool call

**2. Web UI → orchestrator → aco (Remote Orchestration)**
```
Web UI (Svelte) → REST API → orchestrator → LLM → Action Interpreter
                                   ↓
                         WebSocket → aco → Tool Runtime
                                   ↓
                      Database ← Results ← orchestrator
```
- REST API for task creation, status queries
- WebSocket for real-time progress updates
- LLM-driven task planning and execution
- Results persisted to SQLite database

**3. Orchestrator → Database (Persistence)**
```
orchestrator → sqlx → SQLite → Disk
```
- All task state, results, and audit logs persisted
- Supports task resume after restart
- Historical analytics and reporting

---

## Critical Design Decision: Action Interpretation

### The Core Challenge

Your LLM emits natural language like:
- "Read the main.rs file"
- "Check if there are any uncommitted changes"
- "Run the tests to see what's failing"

But aco expects structured JSON:
```json
{
  "type": "ToolRequest",
  "tool": "file_read",
  "args": {"path": "src/main.rs"},
  "request_id": "uuid",
  "session_id": "sess-123"
}
```

**The gap:** How do you bridge natural language → structured tool calls?

### Three Approaches (Trade-offs)

#### Option 1: LLM Structured Output (Recommended ✅)

**How it works:**
- Prompt LLM to output JSON tool calls directly
- Use system prompt with tool schema definitions
- LLM generates: `{"tool": "file_read", "args": {"path": "src/main.rs"}}`
- Orchestrator validates and forwards to aco

**Pros:**
- ✅ Most deterministic and reliable
- ✅ No ambiguity in parsing
- ✅ Works with function calling APIs (GPT-4, Claude)
- ✅ Easy to validate before execution

**Cons:**
- ❌ Requires LLM that supports structured output
- ❌ Less flexible for local models
- ❌ Slightly more tokens in prompt (tool schemas)

**When it's "good enough":**
- Primary use case (80%+ of tasks)
- Local models that support JSON mode (llama3, mistral)
- Clear tool boundaries

#### Option 2: Parser-Based Interpretation

**How it works:**
- LLM outputs mixed natural language
- Parser extracts intent using regex/NLP
- Maps patterns to tool calls
- Falls back to asking clarification

**Pros:**
- ✅ Works with any LLM
- ✅ More natural for users
- ✅ Can handle ambiguous requests

**Cons:**
- ❌ Complex parser logic
- ❌ Brittle pattern matching
- ❌ Hard to test all edge cases
- ❌ Maintenance burden

**When it's "good enough":**
- Fallback for simpler local models
- User-initiated commands
- Debugging/exploration mode

#### Option 3: Hybrid (Best of Both)

**How it works:**
- Primary: LLM structured output (Option 1)
- Fallback: Parser for ambiguous cases (Option 2)
- Last resort: Ask user/LLM for clarification

**Pros:**
- ✅ Robust across LLM capabilities
- ✅ Graceful degradation
- ✅ Flexibility where needed

**Cons:**
- ❌ More code complexity
- ❌ Two systems to maintain

**Recommendation:** Start with Option 1 (structured output), add Option 2 only if needed.

---

## Component Specifications

### 1. Action Interpreter (NEW - Critical Component)

**Location:** `orchestrator/src/interpreter/`

**Responsibilities:**
1. Parse LLM output (JSON or natural language)
2. Validate tool call structure
3. Resolve ambiguities (missing paths, unclear targets)
4. Generate ToolRequest messages
5. Format ToolResponse back to natural language

**Key Modules:**

#### 1.1 Intent Parser (`interpreter/parser.rs`)
```rust
pub struct IntentParser {
    tool_schemas: HashMap<String, ToolSchema>,
}

pub enum ParsedIntent {
    StructuredTool(ToolCall),      // Direct JSON from LLM
    NaturalLanguage(String),        // Needs interpretation
    Ambiguous(Vec<ToolCall>),       // Multiple possibilities
}

impl IntentParser {
    pub fn parse(&self, llm_output: &str) -> Result<ParsedIntent>;
}
```

#### 1.2 Tool Mapper (`interpreter/mapper.rs`)
```rust
pub struct ToolMapper {
    workspace_context: WorkspaceContext,
    file_resolver: FileResolver,
}

impl ToolMapper {
    pub fn map_to_tool_request(
        &self, 
        intent: ParsedIntent,
        session: &Session
    ) -> Result<ToolRequest>;
    
    // Handle ambiguity
    pub fn resolve_path(&self, partial: &str) -> Result<PathBuf>;
    pub fn suggest_tool(&self, description: &str) -> Vec<String>;
}
```

#### 1.3 Result Formatter (`interpreter/formatter.rs`)
```rust
pub struct ResultFormatter;

impl ResultFormatter {
    pub fn format_for_llm(&self, response: ToolResponse) -> String;
    pub fn summarize(&self, responses: Vec<ToolResponse>) -> String;
    pub fn truncate_context(&self, text: &str, max_tokens: usize) -> String;
}
```

**Design Principles:**
- **Fail fast:** If ambiguous, ask clarification before execution
- **Context-aware:** Use workspace state to resolve paths
- **Reversible:** Always log original LLM output for debugging
- **Testable:** Mock LLM outputs for unit tests

### 2. aco Client Application (NEW)

**Location:** `aco/` (new crate)

**Responsibilities:**
1. WebSocket server for orchestrator connections
2. Session management and authentication
3. Tool execution via Tool Runtime SDK
4. Policy enforcement
5. Workspace initialization and context

**Key Modules:**

#### 2.1 WebSocket Server (`aco/src/server.rs`)
```rust
pub struct AcoServer {
    config: AcoConfig,
    runtime: ToolRuntime,
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
}

impl AcoServer {
    pub async fn start(&self, bind_addr: &str) -> Result<()>;
    pub async fn handle_connection(&self, ws: WebSocket) -> Result<()>;
    pub async fn handle_message(&self, msg: WsMessage, session: &Session) -> Result<()>;
}
```

#### 2.2 Session Manager (`aco/src/session.rs`)
```rust
pub struct Session {
    id: SessionId,
    workspace: PathBuf,
    policy: PolicyConfig,
    created_at: Instant,
    last_heartbeat: Instant,
    metrics: SessionMetrics,
}

impl Session {
    pub fn validate_tool_request(&self, req: &ToolRequest) -> Result<()>;
    pub fn update_metrics(&mut self, response: &ToolResponse);
}
```

#### 2.3 Tool Runtime (`aco/src/runtime/`) ⭐ Core Implementation
```rust
pub struct ToolRuntime {
    registry: ToolRegistry,
    policy: Arc<PolicyEnforcer>,
    validators: Vec<Box<dyn Validator>>,
}

impl ToolRuntime {
    pub async fn execute(&self, req: ToolRequest) -> Result<ToolResponse>;
    pub fn register_tool(&mut self, tool: Box<dyn Tool>);
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn schema(&self) -> ToolSchema;
    async fn execute(&self, args: Value, ctx: &ExecutionContext) -> Result<Value>;
}
```

### 3. Tool Runtime SDK Implementation (NEW)

**Location:** `aco/src/runtime/tools/`

**Structure:**
```
tools/
├── mod.rs              # Tool registry
├── filesystem.rs       # fs_list, file_read, file_write, etc.
├── git.rs             # git_status, git_commit, git_push, etc.
├── shell.rs           # shell_exec, proc_list
├── network.rs         # curl
├── ast.rs             # ast_generate, ast_query, ast_edit
└── validation.rs      # Policy enforcement
```

**Implementation Priority:**

**Phase 1 - Essential Tools (MVP):**
- `file_read` - Read file contents
- `file_write` - Write file contents
- `fs_list` - List files in workspace
- `git_status` - Check working tree status
- `shell_exec` - Run commands (with allowlist)

**Phase 2 - Developer Tools:**
- `git_diff`, `git_commit`, `git_push`
- `grep` - Search across files
- `file_patch` - Apply diffs
- `ast_generate` - Parse code structure

**Phase 3 - Advanced Tools:**
- `ast_query` - Query AST nodes
- `ast_edit` - Structural refactoring
- `curl` - HTTP requests
- Full git operations

### 4. Orchestrator WebSocket Client (NEW)

**Location:** `orchestrator/src/client/`

**Responsibilities:**
1. Connect to aco instances
2. Send ToolRequests
3. Handle ToolResponses
4. Reconnection logic
5. Request timeout handling

```rust
pub struct AcoClient {
    url: String,
    connection: Arc<Mutex<Option<WebSocket>>>,
    pending_requests: Arc<Mutex<HashMap<RequestId, oneshot::Sender<ToolResponse>>>>,
}

impl AcoClient {
    pub async fn connect(&mut self) -> Result<()>;
    pub async fn execute_tool(&self, req: ToolRequest) -> Result<ToolResponse>;
    pub async fn execute_batch(&self, reqs: Vec<ToolRequest>) -> Result<Vec<ToolResponse>>;
}
```

### 5. Integration Layer (NEW)

**Location:** `orchestrator/src/integration/`

**Connects existing orchestrator patterns with aco client:**

```rust
pub struct PatternToolBridge {
    aco_client: AcoClient,
    interpreter: ActionInterpreter,
    formatter: ResultFormatter,
}

impl PatternToolBridge {
    // Called by langgraph patterns when they need tool execution
    pub async fn handle_action(&self, action: Action) -> Result<ActionResult>;
    
    // Batch multiple tool calls
    pub async fn handle_batch(&self, actions: Vec<Action>) -> Result<Vec<ActionResult>>;
}
```

---

## Data Flow: Typical Task Example

**Scenario:** LLM agent debugging a failing test

### Step-by-Step Flow

```
1. User Query
   Input: "The integration test is failing, can you debug it?"
   
2. Orchestrator - Pattern Selection
   Router evaluates query → selects "ReAct" pattern
   
3. Orchestrator - LLM Reasoning (Iteration 1)
   LLM (via ReAct pattern): 
   "I should first read the test file to understand what it's testing"
   
   Output: {"tool": "file_read", "args": {"path": "tests/integration_test.rs"}}
   
4. Orchestrator - Action Interpreter
   IntentParser: Detects structured tool call
   ToolMapper: Validates path exists in workspace
   Generates ToolRequest
   
5. WebSocket → aco Client
   ToolRequest sent over WebSocket:
   {
     "type": "ToolRequest",
     "tool": "file_read",
     "args": {"path": "tests/integration_test.rs"},
     "request_id": "req-001",
     "session_id": "sess-abc"
   }
   
6. aco - Policy Check
   PolicyEnforcer: 
   - Path is under workspace ✓
   - File exists ✓
   - Read permission ✓
   
7. aco - Tool Execution
   FileReadTool executes:
   - Reads file content
   - Computes SHA256
   - Measures duration (8ms)
   
8. aco - Response
   ToolResponse sent back:
   {
     "type": "ToolResponse",
     "ok": true,
     "tool": "file_read",
     "request_id": "req-001",
     "duration_ms": 8,
     "data": {"content": "...", "sha256": "..."}
   }
   
9. Orchestrator - Result Formatting
   ResultFormatter converts to natural language:
   "The test file contains 150 lines testing the authentication flow..."
   
10. Orchestrator - LLM Reasoning (Iteration 2)
    LLM sees formatted result, reasons:
    "I see the test is checking login. Let me run it to see the error."
    
    Output: {"tool": "shell_exec", "args": {"cmd": "cargo test integration_test"}}
    
11. [Steps 4-9 repeat with shell_exec tool]
    
12. Orchestrator - LLM Reasoning (Iteration 3)
    LLM: "The error shows a null pointer. Let me check the login function."
    Output: {"tool": "file_read", "args": {"path": "src/auth/login.rs"}}
    
13. [Steps 4-9 repeat]
    
14. Orchestrator - LLM Reasoning (Final)
    LLM: "Found the bug on line 42. Here's the fix..."
    Output: {"tool": "file_write", "args": {...}}
    
15. [Steps 4-9 repeat with file_write]
    
16. Orchestrator - Pattern Complete
    ReAct pattern detects completion
    Returns final result to user
    
Total Tool Calls: 4 (file_read × 2, shell_exec × 1, file_write × 1)
Total Time: ~2-5 seconds (mostly LLM inference)
```

### Key Observations

**Bottlenecks (Actual):**
1. LLM inference time: 1-3 seconds per iteration (dominant)
2. Tool execution time: 10-100ms per tool (negligible)
3. WebSocket overhead: 1-5ms per message (negligible)

**Bottlenecks (Potential but not likely given your constraints):**
1. Context window management (if many files read)
2. Action interpretation ambiguity (if LLM output unclear)
3. Policy validation overhead (if complex rule chains)

---

## Implementation Phases

### Phase 1: Core Infrastructure (2-3 weeks)

**Goal:** Get basic aco ↔ orchestrator communication working

**Deliverables:**
1. ✅ WebSocket protocol implementation
   - Message serialization/deserialization
   - Connection handling
   - Basic error handling

2. ✅ aco WebSocket server
   - Accept connections
   - Session management
   - Heartbeat handling

3. ✅ Orchestrator WebSocket client
   - Connect to aco
   - Send/receive messages
   - Request/response correlation

4. ✅ Basic Tool Runtime
   - Tool registry
   - `file_read` implementation
   - `file_write` implementation
   - Simple policy checks (path sandboxing)

**Success Criteria:**
- Can send ToolRequest from orchestrator
- aco executes file_read/file_write
- ToolResponse received by orchestrator
- Basic integration test passes

**Risk:** WebSocket connection stability
**Mitigation:** Add reconnection logic early, comprehensive error handling

---

### Phase 2: Action Interpreter (2 weeks)

**Goal:** Bridge LLM outputs to tool calls

**Deliverables:**
1. ✅ Structured output parser
   - JSON extraction from LLM output
   - Schema validation
   - Error messages for invalid structure

2. ✅ Tool schema definitions
   - Define all tools in YAML
   - Generate Rust types
   - Integrate with LLM prompts

3. ✅ Result formatter
   - ToolResponse → natural language
   - Context truncation
   - Summary generation

4. ✅ Integration with existing patterns
   - Modify ReAct pattern to use interpreter
   - Test with real LLM calls

**Success Criteria:**
- LLM can request file operations via JSON
- Results formatted and fed back to LLM
- End-to-end ReAct pattern works

**Risk:** LLM not following structured output format
**Mitigation:** Extensive prompt engineering, validation with retries

---

### Phase 3: Essential Tools (2-3 weeks)

**Goal:** Implement high-priority developer tools

**Deliverables:**
1. ✅ Filesystem tools
   - `fs_list` (with glob patterns)
   - `file_patch` (apply diffs)
   - `grep` (search across files)

2. ✅ Git tools
   - `git_status`
   - `git_diff`
   - `git_commit`

3. ✅ Shell execution
   - `shell_exec` (with command allowlist)
   - `proc_list`

4. ✅ Policy enforcement
   - Command allowlist checking
   - Path sandboxing
   - Network restrictions

**Success Criteria:**
- LLM can perform common coding tasks:
  - Read/edit files
  - Search code
  - Run tests
  - Check git status
  - Commit changes

**Risk:** Security vulnerabilities in shell execution
**Mitigation:** Strict allowlist, command parsing, env sanitization

---

### Phase 4: Advanced Patterns (2 weeks)

**Goal:** Enable complex multi-step workflows

**Deliverables:**
1. ✅ Workflow integration
   - Connect WorkflowExecutor to aco client
   - Multi-step tasks with tool calls
   - State management between steps

2. ✅ Router enhancements
   - Context-aware routing based on tool availability
   - Pattern selection based on task complexity

3. ✅ Batch execution
   - Send multiple ToolRequests in parallel
   - Aggregate results efficiently

4. ✅ Better error handling
   - Retry logic for transient failures
   - Graceful degradation
   - User-friendly error messages

**Success Criteria:**
- Can execute multi-file refactoring
- Complex debugging workflows work end-to-end
- Errors handled gracefully

---

### Phase 5: AST & Advanced Tools (3-4 weeks)

**Goal:** Structural code understanding and editing

**Deliverables:**
1. ✅ AST integration
   - Choose AST library (tree-sitter recommended)
   - `ast_generate` implementation
   - `ast_query` for code search

2. ✅ AST editing
   - `ast_edit` for refactoring
   - Validation after edits
   - Format-on-save

3. ✅ Code validation
   - Build validation
   - Lint checks
   - Format checks
   - Test execution

4. ✅ Network tools
   - `curl` implementation
   - Domain allowlist enforcement

**Success Criteria:**
- LLM can perform semantic code search
- Structural refactoring (rename, extract function)
- Validation prevents broken commits

---

### Phase 6: Production Hardening (2 weeks)

**Goal:** Make system production-ready

**Deliverables:**
1. ✅ Comprehensive testing
   - Integration tests for all tools
   - End-to-end workflow tests
   - Security/penetration tests

2. ✅ Observability
   - Structured logging
   - Metrics collection
   - Distributed tracing (orchestrator → aco)

3. ✅ Configuration management
   - YAML configs for policies
   - Environment-specific settings
   - Hot-reload support

4. ✅ Documentation
   - API documentation
   - Tool usage examples
   - Deployment guide
   - Troubleshooting guide

**Success Criteria:**
- 90%+ test coverage
- Clear logs for debugging
- Production deployment successful

---

## Key Design Principles

### 1. Security First

**Policy enforcement at every layer:**
- aco validates all tool requests against policy
- Orchestrator doesn't trust LLM outputs blindly
- Command allowlists, path sandboxing, network restrictions

**When "good enough" is actually good enough:**
- Start with conservative allowlists, relax as needed
- Don't over-engineer sandboxing for trusted environments
- Focus on preventing accidents, not malicious actors (initially)

### 2. Fail Fast, Recover Gracefully

**Validation before execution:**
- Check tool arguments before sending to aco
- Validate paths exist before reading
- Confirm write permissions before modifying

**Graceful degradation:**
- If tool fails, LLM can try alternative approach
- Network issues → retry with backoff
- Ambiguous actions → ask for clarification

### 3. Observability Over Optimization

**Log everything:**
- Every ToolRequest/ToolResponse
- LLM reasoning steps
- Policy decisions

**When NOT to optimize:**
- Don't cache tool results (stale data risk)
- Don't pipeline requests (debugging harder)
- Don't compress messages (readability matters)

**When to optimize:**
- LLM context window (expensive)
- Tool execution (blocking operation)
- Repeated file reads (if proven bottleneck)

### 4. Simplicity Wins

**Avoid over-engineering:**
- ❌ Don't build custom protocol (WebSocket + JSON is fine)
- ❌ Don't build complex action parser (structured output preferred)
- ❌ Don't implement all 9 patterns initially (start with ReAct)
- ❌ Don't build AST editing initially (file_write is good enough)

**Start simple, add complexity only when needed:**
- ✅ Begin with file_read, file_write, shell_exec
- ✅ Add tools as LLM requests them
- ✅ Measure before optimizing
- ✅ User feedback drives features

---

## Pitfalls to Avoid

### 1. ❌ The "Smart Parser" Trap

**Pitfall:** Building a complex NLP parser to interpret arbitrary LLM outputs

**Why it fails:**
- LLM outputs are unpredictable
- Edge cases multiply exponentially
- Maintenance nightmare

**Solution:**
- Force structured output from LLM (Option 1)
- Use system prompts with tool schemas
- Validate before execution, not after

---

### 2. ❌ The "Stateless API" Trap

**Pitfall:** Treating each ToolRequest as independent

**Why it fails:**
- Need workspace context (current branch, file state)
- Session authentication
- Resource locking (prevent concurrent edits)

**Solution:**
- Session-based architecture (already in spec ✓)
- Maintain workspace state in aco
- Pass session_id with every request

---

### 3. ❌ The "LLM Will Figure It Out" Trap

**Pitfall:** Assuming LLM can recover from any error

**Why it fails:**
- Error messages propagate through iterations
- Context window fills with failures
- User frustration

**Solution:**
- Validate inputs before execution
- Clear, actionable error messages
- Prevent bad requests from reaching aco

---

### 4. ❌ The "Premature Optimization" Trap

**Pitfall:** Building batching, caching, streaming before measuring

**Why it fails:**
- Adds complexity without proven benefit
- 40-100 calls @ 50ms each = 2-5 seconds (acceptable)
- LLM inference dominates (1-3s per iteration)

**Solution:**
- Implement simple request/response first
- Measure actual bottlenecks
- Optimize only if proven necessary

---

### 5. ❌ The "Feature Completeness" Trap

**Pitfall:** Implementing all 30+ tools before testing end-to-end

**Why it fails:**
- Integration issues discovered late
- Wasted effort on unused tools
- Delayed user feedback

**Solution:**
- Start with 5 essential tools
- Get end-to-end flow working
- Add tools based on actual usage patterns

---

## Trade-offs Analysis

### Local LLM vs Remote LLM

| Aspect | Local (Ollama) | Remote (OpenAI) |
|--------|----------------|-----------------|
| Latency | 2-5s per call | 1-3s per call |
| Cost | Free | $0.01-0.10 per task |
| Privacy | Full control | Sends code to cloud |
| Capability | Good (70-80%) | Excellent (95%+) |
| Structured Output | Hit or miss | Reliable |

**Recommendation:** 
- Start with local LLM for development
- Use remote LLM for complex tasks
- Make it easy to switch (already have this ✓)

---

### Structured Output vs Parser

| Aspect | Structured Output | Parser |
|--------|-------------------|--------|
| Reliability | High (95%+) | Medium (60-70%) |
| LLM Compatibility | Requires capable model | Works with any |
| Maintenance | Low | High |
| Flexibility | Lower | Higher |

**Recommendation:** 
- Primary: Structured output (Option 1)
- Fallback: Basic parser for simpler models
- Don't build complex NLP parser

---

### WebSocket vs HTTP

| Aspect | WebSocket | HTTP |
|--------|-----------|------|
| Connection Overhead | Low (persistent) | Higher (per-request) |
| Real-time Events | Native | Polling/SSE needed |
| Complexity | Moderate | Low |
| Firewall-friendly | Sometimes blocked | Always works |

**Recommendation:** 
- WebSocket for main protocol (already chosen ✓)
- Consider HTTP fallback if deployment issues arise
- Keep protocol layer thin (easy to swap)

---

## Risk Assessment & Mitigations

### High Risk

**1. LLM Output Unpredictability**
- **Risk:** LLM doesn't follow structured output format
- **Impact:** Action interpreter fails frequently
- **Likelihood:** Medium (especially with local models)
- **Mitigation:** 
  - Extensive prompt engineering
  - Validation with retries (up to 3 attempts)
  - Fallback to simpler format
  - Monitor success rate, iterate on prompts

**2. Security Vulnerabilities**
- **Risk:** Command injection via shell_exec
- **Impact:** Critical (arbitrary code execution)
- **Likelihood:** Medium (adversarial prompts)
- **Mitigation:**
  - Strict command allowlist
  - Argument sanitization
  - No shell interpretation (direct exec)
  - Security audit before production

---

### Medium Risk

**3. Context Window Overflow**
- **Risk:** Reading large files exhausts LLM context
- **Impact:** Task failures, increased costs
- **Likelihood:** High (common with large codebases)
- **Mitigation:**
  - File size limits (1MB default)
  - Smart truncation in ResultFormatter
  - Chunk large files
  - Summarization for context

**4. Tool Execution Timeouts**
- **Risk:** Long-running commands block workflow
- **Impact:** Poor user experience, hung sessions
- **Likelihood:** Medium (tests, builds)
- **Mitigation:**
  - Per-tool timeout limits (already in spec ✓)
  - Progress events during execution
  - Ability to cancel long operations
  - Async execution for some tools

---

### Low Risk

**5. WebSocket Connection Failures**
- **Risk:** Network interruption during task
- **Impact:** Task state lost
- **Likelihood:** Low (local network)
- **Mitigation:**
  - Automatic reconnection
  - Request replay queue
  - Idempotent operations where possible
  - Session persistence

**6. Concurrent Modifications**
- **Risk:** Multiple agents editing same file
- **Impact:** Merge conflicts, data loss
- **Likelihood:** Low (single-user initially)
- **Mitigation:**
  - File-level locking in aco
  - Optimistic concurrency (compare SHA256)
  - Defer to later phases if single-user

---

## Testing Strategy

### Unit Tests
- Each tool implementation
- Action interpreter parsing
- Policy enforcement rules
- Result formatting

### Integration Tests
- Orchestrator ↔ aco communication
- End-to-end tool execution
- Pattern execution with real tool calls
- Error handling and recovery

### End-to-End Tests
- Complete user workflows
  - "Debug this test failure"
  - "Refactor this function"
  - "Add a new feature"
- Multiple patterns (ReAct, Plan-Execute)
- Real LLM calls (recorded/mocked for CI)

### Security Tests
- Command injection attempts
- Path traversal attacks
- Policy bypass attempts
- Rate limiting

---

## Metrics & Observability

### Key Metrics to Track

**Orchestrator:**
- Pattern selection accuracy
- LLM inference time (per call)
- Action interpretation success rate
- Tool calls per task
- Task completion time
- Error rate by category

**aco:**
- Tool execution time (per tool)
- Policy rejection rate
- WebSocket connection uptime
- Concurrent sessions
- Files modified per session

**System-wide:**
- End-to-end task latency
- Cost per task (if using remote LLM)
- Success rate by task type
- User satisfaction (explicit feedback)

---

## Project Timeline

**Total Estimated Time:** 13-17 weeks

| Phase | Duration | Dependencies | Deliverables |
|-------|----------|--------------|--------------|
| Phase 1: Core Infrastructure | 2-3 weeks | None | WebSocket layer, basic tools |
| Phase 2: Action Interpreter | 2 weeks | Phase 1 | LLM ↔ tool bridge |
| Phase 3: Essential Tools | 2-3 weeks | Phase 1 | Filesystem, git, shell |
| Phase 4: Advanced Patterns | 2 weeks | Phase 1-3 | Workflows, router |
| Phase 5: AST & Advanced | 3-4 weeks | Phase 1-3 | AST tools, validation |
| Phase 6: Production Hardening | 2 weeks | Phase 1-5 | Testing, docs, deployment |

**Critical Path:** Phase 1 → Phase 2 → Phase 3

**Parallelization Opportunities:**
- Phase 3 (tools) and Phase 4 (patterns) can partially overlap
- Phase 5 (AST) can start after Phase 3 tools are stable

---

## Success Criteria

### Minimum Viable Product (MVP)

**Must Have:**
- ✅ Orchestrator can connect to aco via WebSocket
- ✅ LLM can request file read/write operations
- ✅ aco executes tools with policy enforcement
- ✅ ReAct pattern works end-to-end
- ✅ Basic error handling and logging

**Tasks MVP Enables:**
- Read files in workspace
- Make simple edits based on LLM suggestions
- Run tests and see results
- Check git status

---

### Full Feature Set

**Should Have:**
- ✅ All essential tools implemented (filesystem, git, shell)
- ✅ Multiple patterns working (ReAct, Plan-Execute, Reflection)
- ✅ Workflow orchestration
- ✅ Advanced error recovery
- ✅ Comprehensive testing

**Tasks Enabled:**
- Complex multi-file refactoring
- Debug test failures with iteration
- Implement new features from scratch
- Code review and suggestions

---

### Production Ready

**Nice to Have:**
- ✅ AST-based code editing
- ✅ Full validation pipeline (build, lint, test, format)
- ✅ Metrics and monitoring
- ✅ Hot-reload configuration
- ✅ Complete documentation

**Tasks Enabled:**
- Semantic code search and refactoring
- Safe commits with validation
- Production deployments
- Team collaboration

---

## Next Steps

### Immediate Actions (Week 1)

1. **Create project structure**
   ```
   workspace/
   ├── orchestrator/     (existing)
   ├── aco/              (NEW crate)
   ├── tool-runtime/     (NEW crate - implements SDK)
   └── examples/         (end-to-end demos)
   ```

2. **Define WebSocket protocol**
   - Implement ToolRequest/ToolResponse types
   - Add serde serialization
   - Write protocol documentation

3. **Spike: Test LLM structured output**
   - Try Ollama with JSON mode
   - Test prompt engineering
   - Measure success rate

4. **Create basic aco server**
   - Accept WebSocket connections
   - Echo messages (no tool execution yet)
   - Session management

5. **Create orchestrator WebSocket client**
   - Connect to aco
   - Send/receive messages
   - Basic error handling

### First Deliverable (Week 2-3)

**Goal:** "Hello World" tool execution

- Orchestrator sends: `{"tool": "file_read", "args": {"path": "README.md"}}`
- aco receives, reads file, responds with content
- Orchestrator formats result and logs it
- Integration test passes

### Technical Debt to Avoid

- ❌ Don't build features without tests
- ❌ Don't optimize before measuring
- ❌ Don't implement tools that aren't used
- ❌ Don't skip documentation as you build
- ❌ Don't commit secrets or API keys

---

## Open Questions for Discussion

Before starting implementation, we should decide:

1. **Authentication:** How does orchestrator authenticate to aco?
   - API key in config?
   - Mutual TLS?
   - Trust localhost connections?

2. **Multi-workspace:** Can one aco instance serve multiple workspaces?
   - Single workspace per aco instance? (simpler)
   - Multiple workspaces with isolation? (more flexible)

3. **Tool Discovery:** How does orchestrator know what tools are available?
   - Hardcoded in prompts?
   - Query aco capabilities at startup?
   - YAML schema shared between both?

4. **Error Recovery:** When tool fails, should orchestrator:
   - Always retry?
   - Ask LLM to reformulate request?
   - Bubble error to user immediately?

5. **Rate Limiting:** Should we enforce rate limits?
   - Per session?
   - Per tool type?
   - Only if production deployment?

---

## Conclusion

You have a strong foundation with the orchestrator crate. The missing pieces are:

1. **Action Interpreter** - Bridge LLM → tools (new, critical)
2. **WebSocket Layer** - Connect orchestrator ↔ aco (new, straightforward)
3. **Tool Runtime Implementation** - Actual Rust code for tools (new, tedious but well-specified)
4. **aco Application** - CLI/daemon that hosts runtime (new, moderate complexity)

**The Real Complexity Lives In:**
- Action interpretation (ambiguity, validation)
- Security (policy enforcement, sandboxing)
- Robustness (error recovery, edge cases)

**Not in:**
- Network performance (WebSocket is fine)
- Optimization (40-100 calls is manageable)
- Protocol design (JSON is adequate)

**Philosophy:** Start simple, measure, iterate. The SDK spec is comprehensive, but you don't need all 30+ tools on day one. Get 5 tools working end-to-end, then expand based on real usage patterns.

**Estimated time to MVP:** 4-6 weeks with focused development
**Estimated time to production:** 13-17 weeks

Let me know which area you'd like to drill deeper into, or if you want me to generate starter code for any component.

---
---

# PART II - User Interface & API Architecture

## Overview

This section extends PART I's core acolib system with user interfaces (TUI and Web UI), REST API layer, and database persistence. The goal is to make the system accessible to both local terminal users and remote web users, with full state persistence and real-time monitoring capabilities.

**Status:** Design specification - future implementation roadmap

**Timeline:** 15-20 weeks of focused development

**Key Additions:**
- 🎨 **TUI (Terminal UI)** using ratatui - Local interactive dashboard
- 🌐 **Web UI** using Svelte - Remote web dashboard
- 🔌 **REST API** using axum - HTTP endpoints for web UI
- 💾 **Database Layer** using sqlx + SQLite - State persistence
- 📊 **Real-time Updates** using WebSocket - Live progress streaming

---

## Technology Stack

### Frontend Technologies

**TUI (Terminal User Interface)**
- **ratatui 0.25+**: Modern Rust TUI framework (fork of tui-rs)
  - Rationale: Active maintenance, excellent performance, rich widget set
  - Alternatives considered: cursive (less flexible), termion (lower-level)
- **crossterm 0.27**: Terminal manipulation (cross-platform)
  - Rationale: Works on Windows/Mac/Linux, good ergonomics
- **tui-input**: Text input widgets
- **tui-textarea**: Multi-line text editing

**Web UI (Browser-based Dashboard)**
- **Svelte 4.2+**: Reactive UI framework
  - Rationale: Minimal bundle size, true reactivity, simple syntax
  - Alternatives considered: React (larger), Vue (more complex)
- **SvelteKit 2.0**: Full-stack Svelte framework
  - Server-side rendering
  - File-based routing
  - API routes
- **Tailwind CSS 3.4**: Utility-first CSS framework
- **shadcn-svelte**: High-quality component library
- **svelte-query**: Data fetching and caching
- **socket.io-client**: WebSocket client for real-time updates

### Backend Technologies

**Web Framework**
- **axum 0.7**: Web framework (hyper + tower)
  - Rationale: Type-safe, performant, excellent async support
  - Composable middleware with tower
  - WebSocket support built-in

**Database**
- **SQLite 3.40+**: File-based SQL database
  - Rationale: Zero-config, embedded, ACID compliant, fast
  - Single-file storage (`acolib.db`)
  - Perfect for local/small deployments
  - No separate database server needed
- **sqlx 0.7**: SQL toolkit and ORM
  - Compile-time query validation
  - Async support with tokio
  - Migration management
  - Connection pooling

**API & WebSocket**
- **axum**: HTTP server and routing
- **tokio-tungstenite 0.21**: WebSocket implementation
- **tower-http 0.5**: HTTP middleware (CORS, compression, tracing)

**Serialization**
- **serde 1.0**: Serialization framework
- **serde_json**: JSON support
- **serde_yaml**: YAML config files

**Observability**
- **tracing 0.1**: Structured logging
- **tracing-subscriber 0.3**: Log formatting and filtering
- **tracing-appender**: Log file rotation

**Testing**
- **tokio::test**: Async test runtime
- **sqlx::test**: Database integration tests
- **criterion 0.5**: Benchmarking

---

## Database Layer Architecture

### Design Philosophy

**SQLite-First Approach**
- Single-user local development focus
- File-based database (`~/.config/acolib/acolib.db`)
- No separate database server required
- Simple backup (copy file)
- Fast queries (<10ms p95 for typical operations)

**Schema Design Principles**
- Normalize data to reduce redundancy
- Use JSON columns for flexible metadata
- Index frequently queried columns
- Soft deletes for audit trail
- Timestamps on all tables (created_at, updated_at)

### Database Schema

**tasks table** - Task definitions and lifecycle
```sql
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,  -- UUID as TEXT
    title TEXT NOT NULL,
    description TEXT,
    task_type TEXT NOT NULL,  -- 'file_operation', 'git_operation', 'workflow', etc.
    status TEXT NOT NULL DEFAULT 'pending',  -- 'pending', 'running', 'completed', 'failed'
    config JSON NOT NULL,  -- Task-specific configuration
    metadata JSON,  -- Additional flexible data
    workspace_path TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT,
    error TEXT
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_created_at ON tasks(created_at DESC);
CREATE INDEX idx_tasks_task_type ON tasks(task_type);
```

**workflows table** - Multi-step workflow definitions
```sql
CREATE TABLE workflows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    pattern_type TEXT NOT NULL,  -- 'ReAct', 'Plan-Execute', 'Router', etc.
    config JSON NOT NULL,  -- Workflow configuration
    state JSON NOT NULL DEFAULT '{}',  -- Current workflow state
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE INDEX idx_workflows_status ON workflows(status);
CREATE INDEX idx_workflows_pattern_type ON workflows(pattern_type);
```

**workflow_tasks table** - Junction table for workflows and tasks
```sql
CREATE TABLE workflow_tasks (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    step_number INTEGER NOT NULL,
    depends_on TEXT,  -- task_id of dependency
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_workflow_tasks_workflow_id ON workflow_tasks(workflow_id);
CREATE INDEX idx_workflow_tasks_task_id ON workflow_tasks(task_id);
CREATE UNIQUE INDEX idx_workflow_tasks_step ON workflow_tasks(workflow_id, step_number);
```

**tool_executions table** - Audit log of all tool executions
```sql
CREATE TABLE tool_executions (
    id TEXT PRIMARY KEY,
    task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    tool_name TEXT NOT NULL,
    args JSON NOT NULL,
    result JSON,
    status TEXT NOT NULL,  -- 'success', 'failure'
    duration_ms INTEGER,
    error TEXT,
    executed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_tool_executions_task_id ON tool_executions(task_id);
CREATE INDEX idx_tool_executions_tool_name ON tool_executions(tool_name);
CREATE INDEX idx_tool_executions_executed_at ON tool_executions(executed_at DESC);
```

**sessions table** - Active WebSocket sessions
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    workspace_path TEXT NOT NULL,
    config JSON NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',  -- 'active', 'disconnected'
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_heartbeat TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT
);

CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_sessions_last_heartbeat ON sessions(last_heartbeat);
```

**configurations table** - System configuration and policies
```sql
CREATE TABLE configurations (
    key TEXT PRIMARY KEY,
    value JSON NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Migration Strategy

**Using sqlx-cli for migrations**
```bash
# Create new migration
sqlx migrate add create_tasks_table

# Apply migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

**Migration Files** (`migrations/`)
```
migrations/
├── 20250101000001_create_tasks_table.sql
├── 20250101000002_create_workflows_table.sql
├── 20250101000003_create_workflow_tasks_table.sql
├── 20250101000004_create_tool_executions_table.sql
├── 20250101000005_create_sessions_table.sql
└── 20250101000006_create_configurations_table.sql
```

**Example Migration** (`20250101000001_create_tasks_table.sql`):
```sql
-- Add migration (Up)
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    task_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    config JSON NOT NULL,
    metadata JSON,
    workspace_path TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT,
    error TEXT
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_created_at ON tasks(created_at DESC);
CREATE INDEX idx_tasks_task_type ON tasks(task_type);

-- Rollback (Down) - Add to separate .down.sql file
-- DROP TABLE tasks;
```

### Repository Pattern

**TaskRepository** (`orchestrator/src/persistence/task_repo.rs`):
```rust
use sqlx::{SqlitePool, query_as};
use serde_json::Value;
use crate::models::Task;

pub struct TaskRepository {
    pool: SqlitePool,
}

impl TaskRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, task: &Task) -> Result<Task, sqlx::Error> {
        query_as!(
            Task,
            r#"
            INSERT INTO tasks (id, title, description, task_type, status, config, metadata, workspace_path)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
            task.id,
            task.title,
            task.description,
            task.task_type,
            task.status,
            task.config,
            task.metadata,
            task.workspace_path
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get(&self, id: &str) -> Result<Option<Task>, sqlx::Error> {
        query_as!(
            Task,
            r#"SELECT * FROM tasks WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_by_status(
        &self,
        status: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Task>, sqlx::Error> {
        query_as!(
            Task,
            r#"
            SELECT * FROM tasks
            WHERE status = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            status,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn update_status(
        &self,
        id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<Task, sqlx::Error> {
        query_as!(
            Task,
            r#"
            UPDATE tasks
            SET status = $2, error = $3, updated_at = datetime('now'),
                completed_at = CASE WHEN $2 IN ('completed', 'failed') THEN datetime('now') ELSE completed_at END
            WHERE id = $1
            RETURNING *
            "#,
            id,
            status,
            error
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"DELETE FROM tasks WHERE id = $1"#,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
```

### Query Performance Targets

| Operation | Target Latency (p95) | Optimization |
|-----------|---------------------|--------------|
| Single task fetch | <5ms | Indexed primary key |
| List tasks (paginated) | <10ms | Composite index on status+created_at |
| Task search (full-text) | <50ms | SQLite FTS5 extension |
| Workflow with tasks (JOIN) | <20ms | Foreign key indexes |
| Tool execution insert | <5ms | Batched inserts |
| Session cleanup (DELETE) | <10ms | Indexed last_heartbeat |

---

## REST API Layer

### API Design Principles

**RESTful Conventions**
- Use HTTP verbs correctly (GET, POST, PUT, DELETE)
- Resource-based URLs (`/api/v1/tasks`, not `/api/v1/get-tasks`)
- Consistent response format
- Proper HTTP status codes
- Pagination for list endpoints

**No Authentication** (per requirements)
- Localhost-only binding recommended
- Trust local connections
- Future: Add optional API key support if needed

### Endpoint Specifications

**Base URL:** `http://localhost:8080/api/v1`

#### Tasks API

**POST `/api/v1/tasks`** - Create new task
```json
// Request
{
  "title": "Read main.rs",
  "description": "Read the main.rs file contents",
  "task_type": "file_operation",
  "config": {
    "tool": "file_read",
    "args": {"path": "src/main.rs"}
  },
  "workspace_path": "/home/user/project"
}

// Response (201 Created)
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "Read main.rs",
  "description": "Read the main.rs file contents",
  "task_type": "file_operation",
  "status": "pending",
  "config": {...},
  "metadata": null,
  "workspace_path": "/home/user/project",
  "created_at": "2025-01-15T10:30:00Z",
  "updated_at": "2025-01-15T10:30:00Z",
  "started_at": null,
  "completed_at": null,
  "error": null
}
```

**GET `/api/v1/tasks/:id`** - Get task by ID
```json
// Response (200 OK)
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "Read main.rs",
  "status": "completed",
  "...": "..."
}

// Error (404 Not Found)
{
  "error": "Task not found",
  "task_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**GET `/api/v1/tasks`** - List tasks (paginated)
```
Query Parameters:
  ?status=pending,running  (filter by status)
  ?task_type=file_operation  (filter by type)
  ?limit=20  (default: 20, max: 100)
  ?offset=0  (default: 0)
  ?sort=created_at:desc  (sort field and direction)

// Response (200 OK)
{
  "tasks": [
    {"id": "...", "title": "...", ...},
    ...
  ],
  "total": 150,
  "limit": 20,
  "offset": 0
}
```

**PUT `/api/v1/tasks/:id`** - Update task
```json
// Request
{
  "title": "Updated title",
  "description": "Updated description"
}

// Response (200 OK)
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "Updated title",
  "updated_at": "2025-01-15T10:35:00Z",
  ...
}
```

**DELETE `/api/v1/tasks/:id`** - Delete task
```
// Response (204 No Content)
```

**POST `/api/v1/tasks/:id/execute`** - Execute task
```
// Response (202 Accepted)
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "started_at": "2025-01-15T10:30:05Z"
}
```

**POST `/api/v1/tasks/:id/cancel`** - Cancel running task
```
// Response (200 OK)
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "cancelled"
}
```

#### Workflows API

**POST `/api/v1/workflows`** - Create workflow
```json
{
  "name": "Multi-file refactoring",
  "pattern_type": "Plan-Execute",
  "config": {
    "llm_provider": "ollama",
    "model": "llama3"
  },
  "tasks": [
    {"title": "Read files", "tool": "file_read", "args": {...}},
    {"title": "Apply changes", "tool": "file_write", "args": {...}, "depends_on": 0}
  ]
}
```

**GET `/api/v1/workflows/:id`** - Get workflow with tasks
```json
{
  "id": "...",
  "name": "Multi-file refactoring",
  "status": "running",
  "tasks": [
    {"id": "...", "title": "Read files", "status": "completed"},
    {"id": "...", "title": "Apply changes", "status": "running"}
  ],
  "created_at": "...",
  "updated_at": "..."
}
```

**GET `/api/v1/workflows`** - List workflows
**POST `/api/v1/workflows/:id/execute`** - Execute workflow
**DELETE `/api/v1/workflows/:id`** - Delete workflow

#### Tool Executions API

**GET `/api/v1/tool-executions`** - List tool execution history
```
Query Parameters:
  ?task_id=...  (filter by task)
  ?tool_name=file_read  (filter by tool)
  ?status=success,failure
  ?limit=50
  ?offset=0

// Response
{
  "executions": [
    {
      "id": "...",
      "task_id": "...",
      "tool_name": "file_read",
      "args": {"path": "src/main.rs"},
      "result": {"content": "...", "sha256": "..."},
      "status": "success",
      "duration_ms": 8,
      "executed_at": "2025-01-15T10:30:10Z"
    },
    ...
  ],
  "total": 250
}
```

**GET `/api/v1/tool-executions/stats`** - Tool execution statistics
```json
{
  "total_executions": 1250,
  "success_rate": 0.98,
  "avg_duration_ms": 35,
  "by_tool": {
    "file_read": {"count": 450, "avg_duration_ms": 12},
    "git_status": {"count": 200, "avg_duration_ms": 45},
    ...
  },
  "by_status": {
    "success": 1225,
    "failure": 25
  }
}
```

#### Health & System API

**GET `/health`** - Health check
```json
{
  "status": "healthy",
  "version": "0.2.0",
  "uptime_seconds": 3600,
  "database": "connected"
}
```

**GET `/api/v1/system/stats`** - System statistics
```json
{
  "tasks": {
    "total": 150,
    "pending": 5,
    "running": 2,
    "completed": 140,
    "failed": 3
  },
  "workflows": {
    "total": 25,
    "active": 1
  },
  "sessions": {
    "active": 3
  }
}
```

### Error Response Format

All API errors follow consistent format:
```json
{
  "error": "Human-readable error message",
  "code": "ERROR_CODE",
  "details": {
    "field": "validation_error",
    ...
  }
}
```

**HTTP Status Codes**
- `200 OK` - Success
- `201 Created` - Resource created
- `202 Accepted` - Async operation started
- `204 No Content` - Success with no response body
- `400 Bad Request` - Invalid input
- `404 Not Found` - Resource not found
- `409 Conflict` - Resource state conflict
- `500 Internal Server Error` - Server error

### API Handler Implementation Example

**Task Creation Handler** (`orchestrator/src/api/handlers/tasks.rs`):
```rust
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    models::Task,
    persistence::TaskRepository,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub task_type: String,
    pub config: serde_json::Value,
    pub workspace_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub task_type: String,
    pub status: String,
    pub config: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
    pub workspace_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

pub async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), ApiError> {
    // Validate request
    if req.title.is_empty() {
        return Err(ApiError::BadRequest("Title cannot be empty".into()));
    }

    // Create task
    let task = Task {
        id: Uuid::new_v4().to_string(),
        title: req.title,
        description: req.description,
        task_type: req.task_type,
        status: "pending".to_string(),
        config: req.config,
        metadata: None,
        workspace_path: req.workspace_path,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        started_at: None,
        completed_at: None,
        error: None,
    };

    // Save to database
    let saved_task = state.task_repo.create(&task).await
        .map_err(|e| ApiError::Database(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(TaskResponse::from(saved_task))))
}

pub async fn get_task(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<TaskResponse>, ApiError> {
    let task = state.task_repo.get(&id).await
        .map_err(|e| ApiError::Database(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Task {} not found", id)))?;

    Ok(Json(TaskResponse::from(task)))
}

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub status: Option<String>,
    pub task_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListTasksResponse {
    pub tasks: Vec<TaskResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list_tasks(
    Query(query): Query<ListTasksQuery>,
    State(state): State<AppState>,
) -> Result<Json<ListTasksResponse>, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let tasks = if let Some(status) = query.status {
        state.task_repo.list_by_status(&status, limit, offset).await
    } else {
        state.task_repo.list_all(limit, offset).await
    }.map_err(|e| ApiError::Database(e.to_string()))?;

    let total = state.task_repo.count().await
        .map_err(|e| ApiError::Database(e.to_string()))?;

    Ok(Json(ListTasksResponse {
        tasks: tasks.into_iter().map(TaskResponse::from).collect(),
        total,
        limit,
        offset,
    }))
}
```

### WebSocket Protocol for Real-time Updates

**Connection:** `ws://localhost:8080/ws`

**Message Types:**
```typescript
// Client → Server
{
  "type": "subscribe",
  "resource": "task",
  "id": "550e8400-e29b-41d4-a716-446655440000"
}

// Server → Client (updates)
{
  "type": "task_update",
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "progress": 0.45,
  "message": "Processing file 3 of 5"
}

// Server → Client (completion)
{
  "type": "task_complete",
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "result": {...}
}

// Heartbeat
{
  "type": "ping"
}

{
  "type": "pong"
}
```

---

## TUI Design (ratatui)

### Design Philosophy

**Terminal-First Interface**
- Keyboard-driven navigation (vim-style bindings)
- Mouse support as optional enhancement
- Works over SSH and tmux
- Minimal dependencies (runs on headless servers)
- Fast startup (<100ms)
- Low resource usage

**Target Users**
- Developers working in terminal environments
- Users running acolib on remote servers
- Power users who prefer keyboard interfaces
- CI/CD environments (monitoring mode)

### Layout Structure

**Full-Screen Layout** (80x24 minimum terminal size):
```
┌────────────────────────────────────────────────────────────────────────┐
│ acolib v0.2.0 │ Workspace: ~/project │ Session: 3 active │ [Q]uit    │ Header
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌─────────────────────────┐  ┌──────────────────────────────────────┤ Main
│  │ Tasks (15)             │  │ Task Details                          │
│  │ ┣ [R] Read main.rs     │  │ ┌────────────────────────────────────┤
│  │ ┣ [C] Git status       │  │ │ Title: Read main.rs               │
│  │ ┣ [P] Write tests      │  │ │ Type: file_operation              │
│  │ ┣ [F] Fix bug line 42  │  │ │ Status: ✓ Completed               │
│  │ ┗ [P] Deploy to prod   │  │ │ Duration: 125ms                   │
│  │                         │  │ │                                    │
│  │ [1] Pending: 2         │  │ │ Config:                           │
│  │ [2] Running: 1         │  │ │   tool: file_read                 │
│  │ [3] Completed: 10      │  │ │   args:                           │
│  │ [4] Failed: 2          │  │ │     path: src/main.rs             │
│  │                         │  │ │                                    │
│  └─────────────────────────┘  │ │ Result:                           │
│                                │ │   content: "fn main() {...}"     │
│  ┌─────────────────────────┐  │ │   lines: 150                      │
│  │ Tools (5 active)       │  │ │   sha256: a3f2...                 │
│  │ • file_read     (250)  │  │ └────────────────────────────────────┤
│  │ • git_status    (100)  │  │                                       │
│  │ • shell_exec    (50)   │  │ [E]xecute [C]ancel [D]elete [L]ogs   │
│  │ • file_write    (40)   │  └───────────────────────────────────────┤
│  │ • grep          (30)   │                                          │
│  └─────────────────────────┘                                          │
│                                                                        │
├────────────────────────────────────────────────────────────────────────┤
│ [h] Help [n] New Task [f] Filter [/] Search [q] Quit                  │ Footer
└────────────────────────────────────────────────────────────────────────┘
```

### Component Specifications

#### 1. Header Bar
- **Location**: Top row
- **Contents**: App name, version, workspace path, active sessions, quit button
- **Style**: Bold, inverted colors
- **Updates**: Static (redraws on workspace change)

#### 2. Task List Panel (Left, 30% width)
- **Shows**:
  - List of tasks with status icons
  - Task title (truncated to fit)
  - Status indicators: `[R]` Running, `[C]` Completed, `[P]` Pending, `[F]` Failed
- **Navigation**:
  - `j/k` or Arrow keys to move up/down
  - `Enter` to select task
  - `1-4` to filter by status
- **Style**:
  - Selected row highlighted
  - Status colors: Green (completed), Yellow (running), Gray (pending), Red (failed)

#### 3. Task Details Panel (Right, 70% width)
- **Shows**:
  - Full task details
  - Configuration (formatted JSON)
  - Result data (if completed)
  - Error message (if failed)
- **Navigation**:
  - `E` to execute task
  - `C` to cancel running task
  - `D` to delete task
  - `L` to view logs
- **Style**:
  - Syntax highlighting for JSON
  - Scrollable content (`PgUp/PgDn`)

#### 4. Tool Statistics Panel (Bottom left, 30% width)
- **Shows**:
  - List of tools with execution counts
  - Average duration per tool
  - Success rate
- **Updates**: Real-time via WebSocket
- **Style**: Compact list with aligned numbers

#### 5. Footer Bar
- **Location**: Bottom row
- **Contents**: Key bindings cheat sheet
- **Style**: Dimmed text
- **Updates**: Context-sensitive (changes based on active panel)

### Key Bindings

**Global**
- `q` - Quit application
- `h` - Show help overlay
- `?` - Show keybindings
- `Ctrl+C` - Force quit

**Navigation**
- `j` / `↓` - Move down
- `k` / `↑` - Move up
- `g` - Go to top
- `G` - Go to bottom
- `Tab` - Switch panel
- `/` - Search tasks
- `n` - Next search result
- `N` - Previous search result

**Task Actions**
- `n` - Create new task
- `e` - Execute selected task
- `c` - Cancel running task
- `d` - Delete task (with confirmation)
- `r` - Refresh task list
- `l` - View task logs
- `Enter` - View task details

**Filtering**
- `1` - Show pending tasks
- `2` - Show running tasks
- `3` - Show completed tasks
- `4` - Show failed tasks
- `0` - Show all tasks
- `f` - Custom filter (opens input)

**Sorting**
- `sc` - Sort by created time
- `ss` - Sort by status
- `st` - Sort by task type

### State Management

**TUI App State** (`aco/src/tui/app.rs`):
```rust
pub struct App {
    // UI state
    pub active_panel: Panel,
    pub task_list_state: ListState,
    pub selected_task_id: Option<String>,

    // Data
    pub tasks: Vec<Task>,
    pub tool_stats: ToolStats,
    pub sessions: Vec<Session>,

    // Filters and search
    pub filter: TaskFilter,
    pub search_query: Option<String>,
    pub sort_by: SortField,

    // WebSocket connection
    pub ws_client: WebSocketClient,

    // Configuration
    pub config: TuiConfig,
}

pub enum Panel {
    TaskList,
    TaskDetails,
    ToolStats,
}

pub struct TaskFilter {
    pub status: Option<TaskStatus>,
    pub task_type: Option<String>,
}

pub enum SortField {
    CreatedAt,
    Status,
    TaskType,
}
```

### Rendering Loop

**Main Loop** (`aco/src/tui/mod.rs`):
```rust
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

pub async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new().await?;

    // Main loop
    loop {
        // Render
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('j') | KeyCode::Down => app.next_task(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous_task(),
                    KeyCode::Enter => app.select_task(),
                    KeyCode::Char('e') => app.execute_task().await?,
                    _ => {}
                }
            }
        }

        // Process WebSocket messages
        app.process_ws_messages().await?;
    }

    // Cleanup
    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    Ok(())
}
```

### Integration with aco Backend

**TUI → aco WebSocket Flow**:
```
1. TUI starts, connects to aco WebSocket server (ws://localhost:9000)
2. TUI subscribes to task updates
3. User creates task via TUI → sends CreateTask message via WS
4. aco executes tool → sends progress updates via WS
5. TUI receives updates → redraws UI with new status
6. Task completes → TUI shows result in details panel
```

**No REST API needed for TUI** - Pure WebSocket communication for low latency.

---

(Continued in next section...)
