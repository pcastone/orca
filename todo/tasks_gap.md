# Gap Closure Plan - Orca Project

**Created**: 2025-11-15
**Purpose**: Address implementation gaps identified in project documentation comparison
**Target Completion**: ~60 hours of focused work

---

## Executive Summary

This plan addresses the gaps between documented features and actual implementation:
- **57 test compilation errors** (Priority: CRITICAL)
- **ContextManager enhancement** (Priority: HIGH)
- **TaskExecutor LLM integration** (Priority: HIGH)
- **Workspace initialization** (Priority: MEDIUM)
- **Orchestrator-LLM integration** (Priority: MEDIUM)
- **Web UI status clarification** (Priority: LOW)

---

## Section 1: Test Compilation Fixes (PRIORITY: CRITICAL)

**Goal**: Fix all 57 test compilation errors to enable full test suite execution
**Estimated Time**: 2.5 hours
**Dependencies**: None

### Task 1.1: Fix HashMap Import Errors (19 occurrences)
**Estimate**: 15 minutes
**Status**: PENDING

**Actions**:
- [ ] Identify all files with HashMap import errors
- [ ] Add `use std::collections::HashMap;` to affected files
- [ ] Run `cargo test --all` to verify fixes

**Files to check**:
- Search for test files using HashMap without import
- Likely in orchestrator, orca, aco crates

**Acceptance Criteria**:
- All HashMap-related compilation errors resolved
- Tests compile successfully

---

### Task 1.2: Fix Result Type Errors (8 occurrences)
**Estimate**: 20 minutes
**Status**: PENDING

**Actions**:
- [ ] Identify files using wrong Result type path
- [ ] Update to correct Result type:
  - `langgraph_core::error::Result` for core
  - `llm::Result` for LLM tests
  - `orchestrator::error::Result` for orchestrator
- [ ] Verify imports are correct

**Acceptance Criteria**:
- All Result type errors resolved
- Correct error types imported

---

### Task 1.3: Fix MockChatModel Missing Trait Methods (4 occurrences)
**Estimate**: 30 minutes
**Status**: PENDING

**Actions**:
- [ ] Identify test files with incomplete MockChatModel implementations
- [ ] Add missing methods to MockChatModel:
  ```rust
  fn stream(&self, _request: ChatRequest) -> Result<ChatResponseStream, GraphError> {
      Err(GraphError::NotSupported("MockChatModel does not support streaming".into()))
  }

  fn clone_box(&self) -> Box<dyn ChatModel> {
      Box::new(self.clone())
  }
  ```
- [ ] Add `#[derive(Clone)]` to MockChatModel structs
- [ ] Add required imports (GraphError)

**Files to check**:
- Tests in orchestrator crate with MockChatModel
- Tests in orca crate with MockChatModel

**Acceptance Criteria**:
- All MockChatModel implementations have required trait methods
- Tests compile successfully

---

### Task 1.4: Fix API Model Field Mismatches (13 occurrences)
**Estimate**: 45 minutes
**Status**: PENDING

**Actions**:
- [ ] Review `CreateWorkflowRequest` struct definition
- [ ] Add missing fields: `config`, `metadata`
- [ ] Review `UpdateWorkflowRequest` struct definition
- [ ] Add missing fields: `config`, `metadata`
- [ ] Review `ExecuteToolRequest` struct definition
- [ ] Add missing fields: `tool`, `input`, `timeout`, `metadata`
- [ ] Update test code to match new struct definitions
- [ ] Verify all API model tests compile

**Files to check**:
- `orchestrator/src/api/models.rs` (or similar)
- Test files using these request types

**Acceptance Criteria**:
- All API model structs have documented fields
- Test code uses correct field names
- API tests compile successfully

---

### Task 1.5: Fix Type Mismatch Errors (5 occurrences)
**Estimate**: 30 minutes
**Status**: PENDING

**Actions**:
- [ ] Fix `abs()` on `usize` (use conditional or cast to i64)
- [ ] Fix `contains()` on `Option` (use `is_some()` or pattern matching)
- [ ] Fix generic argument count mismatches
- [ ] Add missing imports for `Value` type
- [ ] Add missing imports for `Message` type

**Acceptance Criteria**:
- All type mismatch errors resolved
- Tests compile with correct types

---

### Task 1.6: Verify All Tests Compile and Run
**Estimate**: 10 minutes
**Status**: PENDING

**Actions**:
- [ ] Run `cargo test --all --lib` (unit tests only)
- [ ] Run `cargo test --all` (all tests)
- [ ] Document any remaining test failures (not compilation errors)
- [ ] Update BUILD_STATUS.md with results

**Acceptance Criteria**:
- Zero test compilation errors
- All tests either pass or have documented failures
- Build status updated

---

## Section 2: ContextManager Enhancement (PRIORITY: HIGH)

**Goal**: Implement comprehensive context window management for LLM conversations
**Estimated Time**: 8 hours
**Dependencies**: None
**Reference**: Phase 14.1 in missing.md

### Task 2.1: Implement Token Counting
**Estimate**: 2 hours
**Status**: PENDING

**Actions**:
- [ ] Create `context/token_counter.rs` (already exists in orchestrator, enhance)
- [ ] Implement token counting for different message types
- [ ] Support multiple tokenizer types (tiktoken for OpenAI, character-based fallback)
- [ ] Add tests for token counting accuracy

**Files to modify**:
- `orchestrator/src/context/token_counter.rs`

**Implementation**:
```rust
pub struct TokenCounter {
    tokenizer: Box<dyn Tokenizer>,
}

pub trait Tokenizer: Send + Sync {
    fn count(&self, text: &str) -> usize;
    fn count_messages(&self, messages: &[Message]) -> usize;
}

impl TokenCounter {
    pub fn new(model: &str) -> Result<Self>;
    pub fn count_text(&self, text: &str) -> usize;
    pub fn count_messages(&self, messages: &[Message]) -> usize;
    pub fn count_tool_response(&self, response: &ToolResponse) -> usize;
}
```

**Acceptance Criteria**:
- Token counting works for messages and tool responses
- Tests verify accuracy within 5% margin
- Supports OpenAI and Claude token counting

---

### Task 2.2: Implement Context Window Sizing
**Estimate**: 2 hours
**Status**: PENDING

**Actions**:
- [ ] Enhance `context/manager.rs` with window sizing
- [ ] Add model-specific context limits configuration
- [ ] Implement window usage tracking
- [ ] Add warnings when approaching limits

**Implementation**:
```rust
pub struct ContextManager {
    token_counter: TokenCounter,
    max_tokens: usize,
    reserve_tokens: usize, // For response
}

impl ContextManager {
    pub fn new(model: &str, max_tokens: Option<usize>) -> Result<Self>;
    pub fn can_fit(&self, messages: &[Message]) -> bool;
    pub fn usage(&self, messages: &[Message]) -> ContextUsage;
    pub fn available_tokens(&self, messages: &[Message]) -> usize;
}

pub struct ContextUsage {
    pub used: usize,
    pub available: usize,
    pub percentage: f64,
}
```

**Acceptance Criteria**:
- Context limits respected for all models
- Usage tracking accurate
- Tests verify window sizing logic

---

### Task 2.3: Implement Message Truncation/Summarization
**Estimate**: 3 hours
**Status**: PENDING

**Actions**:
- [ ] Implement intelligent message truncation in `context/trimmer.rs` (already exists)
- [ ] Add strategies: sliding window, importance-based, summary-based
- [ ] Implement tool response summarization for large outputs
- [ ] Add tests for truncation strategies

**Implementation**:
```rust
pub trait TruncationStrategy: Send + Sync {
    fn truncate(&self, messages: &[Message], target_tokens: usize) -> Vec<Message>;
}

pub struct SlidingWindowStrategy;
pub struct ImportanceBasedStrategy {
    system_message_priority: usize,
    recent_message_weight: f64,
}

impl ContextManager {
    pub fn fit_to_window(&self, messages: Vec<Message>) -> Vec<Message>;
    pub fn summarize_tool_response(&self, response: &ToolResponse, max_tokens: usize) -> ToolResponse;
}
```

**Acceptance Criteria**:
- Multiple truncation strategies implemented
- System messages always preserved
- Recent messages prioritized
- Tests verify truncation quality

---

### Task 2.4: Implement Priority-Based Context Retention
**Estimate**: 1 hour
**Status**: PENDING

**Actions**:
- [ ] Add priority field to messages
- [ ] Implement priority-based retention algorithm
- [ ] Add configuration for priority rules
- [ ] Add tests for retention logic

**Implementation**:
```rust
pub enum MessagePriority {
    System,      // Never remove
    Critical,    // Remove last
    Normal,      // Remove based on age
    Low,         // Remove first
}

impl ContextManager {
    pub fn retain_by_priority(&self, messages: &[Message], target_tokens: usize) -> Vec<Message>;
}
```

**Acceptance Criteria**:
- Priority-based retention works correctly
- System messages always retained
- Tests verify priority ordering

---

### Task 2.5: Integration and Testing
**Estimate**: 30 minutes
**Status**: PENDING

**Actions**:
- [ ] Integrate ContextManager with TaskExecutor
- [ ] Integrate with WorkflowEngine
- [ ] Add integration tests
- [ ] Update documentation

**Acceptance Criteria**:
- ContextManager used in all LLM calls
- Integration tests pass
- Documentation updated

---

## Section 3: TaskExecutor LLM Integration (PRIORITY: HIGH)

**Goal**: Complete concrete implementation of TaskExecutor with full LLM integration
**Estimated Time**: 14 hours
**Dependencies**: Section 2 (ContextManager)
**Reference**: Phase 14.2 in missing.md

### Task 3.1: Review Existing TaskExecutor Implementation
**Estimate**: 1 hour
**Status**: PENDING

**Actions**:
- [ ] Review `orca/src/executor/task_executor.rs`
- [ ] Document what exists vs. what's needed
- [ ] Identify integration points with LLM crate
- [ ] Create implementation checklist

**Acceptance Criteria**:
- Clear understanding of existing code
- Gap analysis documented
- Implementation plan ready

---

### Task 3.2: Implement LLM Provider Integration
**Estimate**: 3 hours
**Status**: PENDING

**Actions**:
- [ ] Enhance `orca/src/executor/llm_provider.rs`
- [ ] Add concrete LLM client instantiation from config
- [ ] Implement provider selection logic
- [ ] Add provider fallback mechanism
- [ ] Add tests for provider selection

**Implementation**:
```rust
pub struct LlmProviderManager {
    providers: HashMap<String, Box<dyn ChatModel>>,
    default_provider: String,
}

impl LlmProviderManager {
    pub fn from_config(config: &LlmConfig) -> Result<Self>;
    pub fn get_provider(&self, name: Option<&str>) -> Result<&dyn ChatModel>;
    pub async fn chat(&self, provider: Option<&str>, request: ChatRequest) -> Result<ChatResponse>;
    pub async fn stream(&self, provider: Option<&str>, request: ChatRequest) -> Result<ChatResponseStream>;
}
```

**Acceptance Criteria**:
- LLM providers loaded from config
- Provider selection works
- Fallback logic tested
- Integration tests pass

---

### Task 3.3: Implement Streaming Execution Support
**Estimate**: 4 hours
**Status**: PENDING

**Actions**:
- [ ] Add streaming support to TaskExecutor
- [ ] Implement token-by-token emission
- [ ] Add progress callbacks for streaming
- [ ] Handle streaming errors gracefully
- [ ] Add tests for streaming execution

**Implementation**:
```rust
pub struct TaskExecutor {
    llm_manager: LlmProviderManager,
    context_manager: ContextManager,
}

impl TaskExecutor {
    pub async fn execute_streaming<F>(
        &self,
        task: &Task,
        progress_callback: F,
    ) -> Result<TaskResult>
    where
        F: Fn(StreamEvent) + Send + Sync;
}

pub enum StreamEvent {
    Token(String),
    ToolCall(ToolCall),
    ToolResult(ToolResponse),
    Thinking(String),
    Complete,
}
```

**Acceptance Criteria**:
- Streaming execution works with LLM providers
- Progress callbacks invoked correctly
- Error handling robust
- Tests verify streaming behavior

---

### Task 3.4: Implement Retry Logic with LLM
**Estimate**: 3 hours
**Status**: PENDING

**Actions**:
- [ ] Enhance `orca/src/executor/retry.rs`
- [ ] Add LLM-specific retry strategies
- [ ] Implement exponential backoff for rate limits
- [ ] Add retry budget tracking
- [ ] Handle different error types (rate limit, timeout, API error)
- [ ] Add tests for retry scenarios

**Implementation**:
```rust
pub struct LlmRetryPolicy {
    max_retries: usize,
    base_delay: Duration,
    max_delay: Duration,
    retry_on_rate_limit: bool,
    retry_on_timeout: bool,
}

impl TaskExecutor {
    async fn execute_with_retry(
        &self,
        task: &Task,
        policy: &LlmRetryPolicy,
    ) -> Result<TaskResult>;
}
```

**Acceptance Criteria**:
- Retry logic handles all error types
- Exponential backoff implemented
- Rate limit errors retried appropriately
- Tests verify retry behavior

---

### Task 3.5: Implement Context Management Integration
**Estimate**: 2 hours
**Status**: PENDING

**Actions**:
- [ ] Integrate ContextManager with TaskExecutor
- [ ] Implement message history management
- [ ] Add context window checking before LLM calls
- [ ] Implement automatic truncation when needed
- [ ] Add tests for context integration

**Implementation**:
```rust
impl TaskExecutor {
    async fn prepare_llm_request(
        &self,
        task: &Task,
        history: Vec<Message>,
    ) -> Result<ChatRequest> {
        let mut messages = history;

        // Add task context
        messages.push(Message::system(&task.description));

        // Check and fit to context window
        if !self.context_manager.can_fit(&messages) {
            messages = self.context_manager.fit_to_window(messages);
        }

        Ok(ChatRequest::new(messages))
    }
}
```

**Acceptance Criteria**:
- Context management integrated
- Messages fit in context window
- Truncation works when needed
- Tests verify integration

---

### Task 3.6: End-to-End Integration Testing
**Estimate**: 1 hour
**Status**: PENDING

**Actions**:
- [ ] Create end-to-end tests with real LLM calls (mocked)
- [ ] Test complete task execution flow
- [ ] Test streaming execution
- [ ] Test retry scenarios
- [ ] Test context management
- [ ] Update documentation

**Acceptance Criteria**:
- End-to-end tests pass
- All integration points tested
- Documentation updated

---

## Section 4: Workspace Initialization (PRIORITY: MEDIUM)

**Goal**: Implement explicit workspace initialization logic and validation
**Estimated Time**: 6 hours
**Dependencies**: None
**Reference**: Phase 14.3 in missing.md

### Task 4.1: Review Existing Workspace Code
**Estimate**: 30 minutes
**Status**: PENDING

**Actions**:
- [ ] Review `aco/src/workspace/initializer.rs`
- [ ] Review `aco/src/workspace/security.rs`
- [ ] Review `aco/src/session.rs`
- [ ] Document existing functionality
- [ ] Identify gaps

**Acceptance Criteria**:
- Existing code understood
- Gaps identified
- Implementation plan ready

---

### Task 4.2: Implement Workspace Structure Creation
**Estimate**: 2 hours
**Status**: PENDING

**Actions**:
- [ ] Enhance `workspace/initializer.rs` with structure creation
- [ ] Create standard workspace directories (.orca/, .orca/cache/, .orca/logs/)
- [ ] Generate default configuration files
- [ ] Initialize Git repository if not exists
- [ ] Add validation for workspace structure

**Implementation**:
```rust
pub struct WorkspaceInitializer {
    root: PathBuf,
}

impl WorkspaceInitializer {
    pub fn new(root: PathBuf) -> Self;

    pub async fn initialize(&self) -> Result<Workspace> {
        // Create directory structure
        // Generate config files
        // Initialize Git
        // Set up permissions
        // Create metadata
    }

    pub fn validate(&self) -> Result<ValidationReport>;
}

pub struct Workspace {
    pub root: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub config: WorkspaceConfig,
}
```

**Acceptance Criteria**:
- Workspace directories created correctly
- Config files generated
- Git initialized if needed
- Tests verify structure creation

---

### Task 4.3: Implement Workspace Security Boundary Enforcement
**Estimate**: 2 hours
**Status**: PENDING

**Actions**:
- [ ] Enhance `workspace/security.rs` with boundary checks
- [ ] Implement path validation against workspace root
- [ ] Add symlink detection and handling
- [ ] Implement path traversal prevention
- [ ] Add comprehensive tests

**Implementation**:
```rust
pub struct WorkspaceSecurity {
    workspace_root: PathBuf,
    allowed_paths: Vec<PathBuf>,
    forbidden_patterns: Vec<Regex>,
}

impl WorkspaceSecurity {
    pub fn validate_path(&self, path: &Path) -> Result<()>;
    pub fn is_within_workspace(&self, path: &Path) -> bool;
    pub fn resolve_safe_path(&self, path: &Path) -> Result<PathBuf>;
    pub fn check_symlink(&self, path: &Path) -> Result<SymlinkInfo>;
}
```

**Acceptance Criteria**:
- Path validation prevents escapes
- Symlinks handled correctly
- Tests verify security boundaries

---

### Task 4.4: Implement Workspace Validation
**Estimate**: 1 hour
**Status**: PENDING

**Actions**:
- [ ] Add validation methods to Workspace
- [ ] Check directory structure integrity
- [ ] Validate configuration files
- [ ] Check permissions
- [ ] Add repair functionality for corrupted workspaces

**Implementation**:
```rust
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl Workspace {
    pub fn validate(&self) -> ValidationReport;
    pub fn repair(&self) -> Result<()>;
}
```

**Acceptance Criteria**:
- Validation detects issues
- Repair fixes common problems
- Tests verify validation logic

---

### Task 4.5: Integration and Documentation
**Estimate**: 30 minutes
**Status**: PENDING

**Actions**:
- [ ] Integrate workspace initialization with aco startup
- [ ] Update CLI to use workspace initialization
- [ ] Add tests for integration
- [ ] Update documentation

**Acceptance Criteria**:
- Workspace initialized on aco startup
- CLI commands use workspace correctly
- Documentation updated

---

## Section 5: Orchestrator-LLM Integration (PRIORITY: MEDIUM)

**Goal**: Complete integration of LLM capabilities with orchestrator components
**Estimated Time**: 10 hours
**Dependencies**: Section 2 (ContextManager), Section 3 (TaskExecutor)
**Reference**: Phase 14.4 in missing.md

### Task 5.1: Implement Router LLM Integration
**Estimate**: 3 hours
**Status**: PENDING

**Actions**:
- [ ] Enhance `orchestrator/src/router/llm_router.rs`
- [ ] Implement LLM-based pattern selection
- [ ] Add confidence scoring for pattern selection
- [ ] Implement fallback logic when LLM fails
- [ ] Add tests for router decisions

**Implementation**:
```rust
pub struct LlmRouter {
    llm_client: Box<dyn ChatModel>,
    context_manager: ContextManager,
    patterns: Vec<PatternMetadata>,
}

impl LlmRouter {
    pub async fn select_pattern(
        &self,
        task: &Task,
        context: &TaskContext,
    ) -> Result<PatternSelection>;
}

pub struct PatternSelection {
    pub pattern: String,
    pub confidence: f64,
    pub reasoning: String,
}
```

**Acceptance Criteria**:
- LLM-based routing works
- Confidence scores accurate
- Fallback logic tested
- Tests verify routing decisions

---

### Task 5.2: Implement WorkflowExecutor LLM Integration
**Estimate**: 3 hours
**Status**: PENDING

**Actions**:
- [ ] Enhance `orchestrator/src/workflow/llm_executor.rs`
- [ ] Implement LLM-driven workflow step execution
- [ ] Add dynamic step generation
- [ ] Implement workflow replanning with LLM
- [ ] Add tests for workflow execution

**Implementation**:
```rust
pub struct LlmWorkflowExecutor {
    llm_client: Box<dyn ChatModel>,
    context_manager: ContextManager,
}

impl LlmWorkflowExecutor {
    pub async fn execute_step(
        &self,
        step: &WorkflowStep,
        context: &WorkflowContext,
    ) -> Result<StepResult>;

    pub async fn replan(
        &self,
        workflow: &Workflow,
        current_state: &WorkflowState,
    ) -> Result<Vec<WorkflowStep>>;
}
```

**Acceptance Criteria**:
- LLM-driven step execution works
- Dynamic step generation tested
- Replanning logic verified
- Tests pass

---

### Task 5.3: Implement Pattern Execution with LLM Planning
**Estimate**: 3 hours
**Status**: PENDING

**Actions**:
- [ ] Enhance `orchestrator/src/pattern/llm_planner.rs`
- [ ] Implement LLM-based task planning
- [ ] Add plan validation
- [ ] Implement plan execution
- [ ] Add tests for planning logic

**Implementation**:
```rust
pub struct LlmPlanner {
    llm_client: Box<dyn ChatModel>,
    context_manager: ContextManager,
}

impl LlmPlanner {
    pub async fn create_plan(
        &self,
        task: &Task,
        pattern: &Pattern,
    ) -> Result<ExecutionPlan>;

    pub async fn validate_plan(&self, plan: &ExecutionPlan) -> Result<ValidationResult>;
}

pub struct ExecutionPlan {
    pub steps: Vec<PlanStep>,
    pub estimated_duration: Duration,
    pub reasoning: String,
}
```

**Acceptance Criteria**:
- LLM-based planning works
- Plans are validated
- Execution follows plan
- Tests verify planning

---

### Task 5.4: End-to-End Orchestrator-LLM Tests
**Estimate**: 1 hour
**Status**: PENDING

**Actions**:
- [ ] Create end-to-end tests for router + workflow + pattern
- [ ] Test complete orchestration flow with LLM
- [ ] Test error handling and recovery
- [ ] Verify context management across components
- [ ] Update documentation

**Acceptance Criteria**:
- End-to-end tests pass
- All components integrated
- Documentation updated

---

## Section 6: Web UI Status Clarification (PRIORITY: LOW)

**Goal**: Determine Web UI status and update documentation accordingly
**Estimated Time**: 2 hours
**Dependencies**: None

### Task 6.1: Investigate Web UI Implementation
**Estimate**: 1 hour
**Status**: PENDING

**Actions**:
- [ ] Search for web UI code in separate repositories
- [ ] Check project history for web UI references
- [ ] Review git branches for web UI work
- [ ] Check if web UI is in a separate mono-repo
- [ ] Document findings

**Acceptance Criteria**:
- Web UI status determined
- Location documented (if exists)
- Missing features identified

---

### Task 6.2: Update Documentation Based on Findings
**Estimate**: 1 hour
**Status**: PENDING

**Actions**:
- [ ] If Web UI exists: Add links and setup instructions
- [ ] If Web UI planned: Update docs to reflect future work
- [ ] If Web UI not planned: Remove or clearly mark as future work
- [ ] Update README.md with current status
- [ ] Update architecture.md
- [ ] Update aco_project_plan.md

**Acceptance Criteria**:
- Documentation accurately reflects Web UI status
- No misleading information about Web UI availability
- Future roadmap clear if Web UI is planned

---

## Progress Tracking

### Overall Status
- [ ] Section 1: Test Compilation Fixes (0/6 tasks)
- [ ] Section 2: ContextManager Enhancement (0/5 tasks)
- [ ] Section 3: TaskExecutor LLM Integration (0/6 tasks)
- [ ] Section 4: Workspace Initialization (0/5 tasks)
- [ ] Section 5: Orchestrator-LLM Integration (0/4 tasks)
- [ ] Section 6: Web UI Status Clarification (0/2 tasks)

**Total Tasks**: 28
**Completed**: 0
**In Progress**: 0
**Pending**: 28

### Time Tracking
- **Estimated Total**: ~42.5 hours
- **Actual Time**: TBD
- **Completion Date**: TBD

---

## Success Metrics

### Code Quality
- [ ] Zero test compilation errors
- [ ] All integration tests passing
- [ ] Code coverage >80% for new code
- [ ] No new clippy warnings

### Functionality
- [ ] ContextManager handles all documented scenarios
- [ ] TaskExecutor executes tasks with LLM successfully
- [ ] Workspace initialization creates valid workspaces
- [ ] Orchestrator-LLM integration works end-to-end

### Documentation
- [ ] All new features documented
- [ ] API documentation updated
- [ ] User guide updated
- [ ] Web UI status clarified

---

## Notes

- Tasks should be executed in order within each section
- Sections 2-5 can be worked on in parallel after Section 1
- Each task should include unit tests
- Commit after completing each task
- Run full test suite after each section completion
- Update this document as tasks are completed

---

**Last Updated**: 2025-11-15
**Next Review**: After Section 1 completion
