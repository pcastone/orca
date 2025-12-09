# Workflows & Templates Catalog
## Agentic Coding Orchestrator

This document catalogs all workflows, templates, and patterns needed for the system.

---

## 1. Task Plan Templates (Planning LLM Output)

These are example task plans that Planning LLM should learn from or generate.

### 1.1 Code Understanding Workflows

#### Template: Simple File Read
```yaml
name: simple_file_read
description: Read and understand a single file
complexity: low
typical_tasks: 1-2

example:
  plan_id: "read-{filename}"
  query: "Show me the {filename} file"
  goal: "Read and display file contents"
  tasks:
    - id: 1
      action: read_file
      target: "{filepath}"
      reason: "User wants to view the file"
      expected_outcome: "File contents displayed"
```

#### Template: Code Search & Discover
```yaml
name: code_search
description: Find code patterns across codebase
complexity: low
typical_tasks: 2-3

example:
  plan_id: "search-pattern"
  query: "Find all uses of {pattern}"
  goal: "Locate code matching pattern"
  tasks:
    - id: 1
      action: grep
      reason: "Search for pattern across codebase"
      params:
        pattern: "{search_term}"
        glob: "src/**/*.rs"
    
    - id: 2
      action: analyze_error
      reason: "Categorize and summarize findings"
      depends_on: [1]
```

#### Template: Understand Module Structure
```yaml
name: module_structure
description: Map out module organization and dependencies
complexity: medium
typical_tasks: 3-5

example:
  plan_id: "understand-module"
  query: "Explain how the {module} module works"
  goal: "Understand module structure and relationships"
  tasks:
    - id: 1
      action: read_file
      target: "src/{module}/mod.rs"
      reason: "See module entry point"
    
    - id: 2
      action: list_files
      reason: "Find all files in module"
      params:
        path: "src/{module}"
        recursive: true
    
    - id: 3
      action: grep
      reason: "Find imports and dependencies"
      depends_on: [2]
      params:
        pattern: "^use "
        glob: "src/{module}/**"
    
    - id: 4
      action: analyze_error
      reason: "Synthesize module overview"
      depends_on: [1, 3]
```

---

### 1.2 Debugging Workflows

#### Template: Test Failure Investigation
```yaml
name: debug_test_failure
description: Investigate and fix failing test
complexity: high
typical_tasks: 5-8

example:
  plan_id: "debug-test-{test_name}"
  query: "Why is {test_name} failing?"
  goal: "Identify root cause and fix the test failure"
  tasks:
    - id: 1
      action: read_file
      target: "tests/{test_file}"
      reason: "Understand what the test is checking"
    
    - id: 2
      action: run_test
      target: "{test_name}"
      reason: "Get actual error message and stack trace"
      depends_on: [1]
    
    - id: 3
      action: analyze_error
      reason: "Parse error to identify root cause"
      depends_on: [2]
    
    - id: 4
      action: read_file
      target: "src/{impl_file}"
      reason: "Examine implementation being tested"
      depends_on: [3]
    
    - id: 5
      action: grep
      reason: "Find related code that might be involved"
      depends_on: [3]
      params:
        pattern: "{error_related_term}"
    
    - id: 6
      action: edit_file
      target: "src/{impl_file}"
      reason: "Fix identified bug"
      depends_on: [4, 5]
    
    - id: 7
      action: run_test
      target: "{test_name}"
      reason: "Verify fix works"
      depends_on: [6]
```

#### Template: Build Failure Investigation
```yaml
name: debug_build_failure
description: Fix compilation errors
complexity: medium
typical_tasks: 4-6

example:
  plan_id: "fix-build"
  query: "Fix the build errors"
  goal: "Get code to compile successfully"
  tasks:
    - id: 1
      action: run_build
      reason: "Get compilation error messages"
    
    - id: 2
      action: analyze_error
      reason: "Parse compiler errors"
      depends_on: [1]
    
    - id: 3
      action: read_file
      target: "{error_file}"
      reason: "See code causing error"
      depends_on: [2]
    
    - id: 4
      action: edit_file
      target: "{error_file}"
      reason: "Fix compilation error"
      depends_on: [3]
    
    - id: 5
      action: run_build
      reason: "Verify fix"
      depends_on: [4]
```

#### Template: Runtime Error Investigation
```yaml
name: debug_runtime_error
description: Debug runtime panics or errors
complexity: high
typical_tasks: 6-9

example:
  plan_id: "debug-runtime"
  query: "App crashes with {error_message}"
  goal: "Find and fix runtime error"
  tasks:
    - id: 1
      action: grep
      reason: "Find where error occurs"
      params:
        pattern: "{error_keyword}"
        glob: "src/**"
    
    - id: 2
      action: read_file
      target: "{likely_file}"
      reason: "Examine error location"
      depends_on: [1]
    
    - id: 3
      action: search_code
      reason: "Find function callers"
      depends_on: [2]
      params:
        function: "{error_function}"
    
    - id: 4
      action: analyze_error
      reason: "Determine root cause"
      depends_on: [2, 3]
    
    - id: 5
      action: edit_file
      target: "{fix_file}"
      reason: "Add error handling or fix bug"
      depends_on: [4]
    
    - id: 6
      action: run_test
      reason: "Verify fix"
      depends_on: [5]
```

---

### 1.3 Code Modification Workflows

#### Template: Add New Feature
```yaml
name: add_feature
description: Implement a new feature from scratch
complexity: very_high
typical_tasks: 8-15

example:
  plan_id: "feature-{feature_name}"
  query: "Add feature: {feature_description}"
  goal: "Implement complete working feature"
  tasks:
    - id: 1
      action: read_file
      target: "src/{module}/mod.rs"
      reason: "Understand existing module structure"
    
    - id: 2
      action: search_code
      reason: "Find similar features for patterns"
      depends_on: [1]
    
    - id: 3
      action: write_file
      target: "src/{module}/{feature}.rs"
      reason: "Create new feature file"
      depends_on: [2]
    
    - id: 4
      action: edit_file
      target: "src/{module}/mod.rs"
      reason: "Add feature to module exports"
      depends_on: [3]
    
    - id: 5
      action: write_file
      target: "tests/{feature}_test.rs"
      reason: "Create tests for feature"
      depends_on: [3]
    
    - id: 6
      action: run_build
      reason: "Check compilation"
      depends_on: [4]
    
    - id: 7
      action: run_test
      target: "{feature}_test"
      reason: "Run new tests"
      depends_on: [5, 6]
    
    - id: 8
      action: edit_file
      target: "README.md"
      reason: "Document new feature"
      depends_on: [7]
```

#### Template: Refactor Function
```yaml
name: refactor_function
description: Refactor existing function
complexity: medium
typical_tasks: 5-7

example:
  plan_id: "refactor-{function}"
  query: "Refactor {function} to be more {improvement}"
  goal: "Improve function without breaking functionality"
  tasks:
    - id: 1
      action: read_file
      target: "src/{file}"
      reason: "See current implementation"
    
    - id: 2
      action: search_code
      reason: "Find all callers of function"
      depends_on: [1]
      params:
        function: "{function_name}"
    
    - id: 3
      action: read_file
      target: "tests/{test_file}"
      reason: "Understand expected behavior"
      depends_on: [1]
    
    - id: 4
      action: edit_file
      target: "src/{file}"
      reason: "Refactor implementation"
      depends_on: [2, 3]
    
    - id: 5
      action: run_test
      reason: "Ensure behavior unchanged"
      depends_on: [4]
    
    - id: 6
      action: run_build
      reason: "Check compilation"
      depends_on: [4]
```

#### Template: Fix Bug
```yaml
name: fix_bug
description: Locate and fix a reported bug
complexity: medium-high
typical_tasks: 4-7

example:
  plan_id: "bugfix-{bug_id}"
  query: "Fix bug: {bug_description}"
  goal: "Resolve reported issue"
  tasks:
    - id: 1
      action: grep
      reason: "Find code related to bug"
      params:
        pattern: "{bug_keyword}"
    
    - id: 2
      action: read_file
      target: "{likely_file}"
      reason: "Examine problematic code"
      depends_on: [1]
    
    - id: 3
      action: edit_file
      target: "{likely_file}"
      reason: "Apply fix"
      depends_on: [2]
    
    - id: 4
      action: run_test
      reason: "Verify fix doesn't break tests"
      depends_on: [3]
    
    - id: 5
      action: write_file
      target: "tests/{regression_test}.rs"
      reason: "Add regression test"
      depends_on: [4]
```

---

### 1.4 Git Workflows

#### Template: Commit Changes
```yaml
name: git_commit
description: Review and commit changes
complexity: low
typical_tasks: 3-4

example:
  plan_id: "commit-changes"
  query: "Commit my changes with message '{message}'"
  goal: "Safely commit current changes"
  tasks:
    - id: 1
      action: git_status
      reason: "Check what changed"
    
    - id: 2
      action: git_diff
      reason: "Review changes before committing"
      depends_on: [1]
    
    - id: 3
      action: git_add
      reason: "Stage changes"
      depends_on: [2]
      params:
        paths: ["."]
    
    - id: 4
      action: git_commit
      reason: "Commit with message"
      depends_on: [3]
      params:
        message: "{commit_message}"
```

#### Template: Create Feature Branch
```yaml
name: create_branch
description: Create and switch to feature branch
complexity: low
typical_tasks: 2-3

example:
  plan_id: "branch-{name}"
  query: "Create branch for {feature}"
  goal: "Set up feature branch"
  tasks:
    - id: 1
      action: git_status
      reason: "Ensure clean working tree"
    
    - id: 2
      action: git_checkout
      reason: "Create and switch to branch"
      depends_on: [1]
      params:
        target: "feature/{branch_name}"
        create: true
```

#### Template: Prepare Pull Request
```yaml
name: prepare_pr
description: Get code ready for pull request
complexity: medium
typical_tasks: 6-8

example:
  plan_id: "prep-pr"
  query: "Prepare code for pull request"
  goal: "Ensure code meets quality standards"
  tasks:
    - id: 1
      action: run_build
      reason: "Ensure code compiles"
    
    - id: 2
      action: run_test
      reason: "Run all tests"
      depends_on: [1]
    
    - id: 3
      action: shell_exec
      reason: "Run linter"
      depends_on: [1]
      params:
        cmd: "cargo clippy"
    
    - id: 4
      action: shell_exec
      reason: "Run formatter"
      depends_on: [1]
      params:
        cmd: "cargo fmt --check"
    
    - id: 5
      action: git_status
      reason: "Check uncommitted changes"
      depends_on: [2, 3, 4]
    
    - id: 6
      action: git_diff
      reason: "Review final changes"
      depends_on: [5]
```

---

### 1.5 Documentation Workflows

#### Template: Add Documentation
```yaml
name: add_docs
description: Document code or feature
complexity: low-medium
typical_tasks: 3-5

example:
  plan_id: "doc-{item}"
  query: "Add documentation for {item}"
  goal: "Document functionality"
  tasks:
    - id: 1
      action: read_file
      target: "src/{file}"
      reason: "Understand implementation"
    
    - id: 2
      action: edit_file
      target: "src/{file}"
      reason: "Add rustdoc comments"
      depends_on: [1]
    
    - id: 3
      action: edit_file
      target: "README.md"
      reason: "Update README with examples"
      depends_on: [1]
    
    - id: 4
      action: shell_exec
      reason: "Generate and check docs"
      depends_on: [2]
      params:
        cmd: "cargo doc --no-deps"
```

---

### 1.6 Code Review Workflows

#### Template: Review Code Changes
```yaml
name: code_review
description: Analyze code changes for issues
complexity: medium
typical_tasks: 5-7

example:
  plan_id: "review-changes"
  query: "Review my recent changes"
  goal: "Identify potential issues before commit"
  tasks:
    - id: 1
      action: git_diff
      reason: "See what changed"
    
    - id: 2
      action: analyze_error
      reason: "Check for common issues"
      depends_on: [1]
    
    - id: 3
      action: grep
      reason: "Find TODO/FIXME comments"
      params:
        pattern: "TODO|FIXME"
    
    - id: 4
      action: run_build
      reason: "Ensure compiles"
      depends_on: [1]
    
    - id: 5
      action: run_test
      reason: "Run test suite"
      depends_on: [4]
    
    - id: 6
      action: shell_exec
      reason: "Run linter"
      depends_on: [4]
      params:
        cmd: "cargo clippy"
```

---

## 2. Prompt Templates

### 2.1 Planning LLM Prompts

#### System Prompt
```text
You are a planning assistant for a coding orchestration system. Your role is to break down complex coding tasks into structured, executable steps.

When given a user query, analyze the problem and create a JSON task plan with this structure:

{
  "plan_id": "unique-identifier",
  "query": "original user query",
  "goal": "high-level objective",
  "tasks": [
    {
      "id": 1,
      "action": "action_type",
      "target": "action target (optional)",
      "reason": "why this task is needed",
      "expected_outcome": "what success looks like",
      "depends_on": [],
      "params": {}
    }
  ]
}

AVAILABLE ACTIONS:
- read_file: Read file contents (requires target: filepath)
- write_file: Create/overwrite file (requires target: filepath)
- edit_file: Modify existing file (requires target: filepath)
- list_files: List files in directory
- grep: Search for pattern in files
- run_test: Execute tests (optional target: test name)
- run_build: Build the project
- git_status: Check git status
- git_diff: Show git changes
- git_add: Stage changes
- git_commit: Commit changes (requires params.message)
- git_checkout: Switch/create branch
- shell_exec: Run shell command (requires params.cmd)
- search_code: Search for code patterns
- analyze_error: Analyze error messages or results

PLANNING GUIDELINES:
1. Break complex tasks into 3-10 steps (optimal: 5-7)
2. Each task should be atomic and clear
3. Use dependencies to ensure proper ordering (tasks can only depend on earlier tasks)
4. Include clear reasoning for each step
5. Specify targets for file operations
6. Think step-by-step about what information is needed at each stage
7. Consider error cases (e.g., check before modifying)

DEPENDENCY RULES:
- Tasks can only depend on EARLIER tasks (lower IDs)
- Task 1 cannot have dependencies
- Task 2 can depend on [1]
- Task 3 can depend on [1], [2], or [1, 2]
- NEVER create forward dependencies (e.g., Task 2 depending on Task 3)

OUTPUT REQUIREMENTS:
- Output ONLY valid JSON
- No explanatory text before or after
- No markdown code blocks
- Ensure all required fields are present
- Use sequential task IDs starting from 1

Think carefully about the optimal sequence of steps. Output your plan now.
```

#### Few-Shot Examples (for Planning LLM)

**Example 1: Simple Read**
```json
USER: "Show me the main.rs file"

ASSISTANT: {
  "plan_id": "read-main-001",
  "query": "Show me the main.rs file",
  "goal": "Read and display main.rs contents",
  "tasks": [
    {
      "id": 1,
      "action": "read_file",
      "target": "src/main.rs",
      "reason": "User wants to view the main source file",
      "expected_outcome": "File contents displayed for user review"
    }
  ]
}
```

**Example 2: Debug Test**
```json
USER: "The test_login function is failing, can you fix it?"

ASSISTANT: {
  "plan_id": "debug-test-login-001",
  "query": "The test_login function is failing, can you fix it?",
  "goal": "Identify and fix the bug causing test_login to fail",
  "tasks": [
    {
      "id": 1,
      "action": "read_file",
      "target": "tests/auth_test.rs",
      "reason": "Understand what the test is checking and its assertions",
      "expected_outcome": "See test setup and expected behavior"
    },
    {
      "id": 2,
      "action": "run_test",
      "target": "test_login",
      "reason": "Get the actual error message and failure details",
      "expected_outcome": "Error message identifying the problem",
      "depends_on": [1]
    },
    {
      "id": 3,
      "action": "analyze_error",
      "reason": "Parse the error to identify root cause",
      "expected_outcome": "Understanding of what's failing and why",
      "depends_on": [2]
    },
    {
      "id": 4,
      "action": "read_file",
      "target": "src/auth/login.rs",
      "reason": "Examine the login implementation being tested",
      "expected_outcome": "Locate the buggy code",
      "depends_on": [3]
    },
    {
      "id": 5,
      "action": "edit_file",
      "target": "src/auth/login.rs",
      "reason": "Fix the identified bug in the login function",
      "expected_outcome": "Bug fixed, code should work correctly",
      "depends_on": [4]
    },
    {
      "id": 6,
      "action": "run_test",
      "target": "test_login",
      "reason": "Verify that the fix resolves the test failure",
      "expected_outcome": "Test passes successfully",
      "depends_on": [5]
    }
  ]
}
```

**Example 3: Git Commit**
```json
USER: "Commit my changes with message 'feat: add authentication'"

ASSISTANT: {
  "plan_id": "commit-auth-feat-001",
  "query": "Commit my changes with message 'feat: add authentication'",
  "goal": "Safely commit authentication changes to git",
  "tasks": [
    {
      "id": 1,
      "action": "git_status",
      "reason": "Check which files have been modified",
      "expected_outcome": "List of changed files to review"
    },
    {
      "id": 2,
      "action": "git_diff",
      "reason": "Review the actual changes before committing",
      "expected_outcome": "Diff showing all modifications",
      "depends_on": [1]
    },
    {
      "id": 3,
      "action": "run_test",
      "reason": "Ensure all tests pass before committing",
      "expected_outcome": "All tests pass",
      "depends_on": [1]
    },
    {
      "id": 4,
      "action": "git_add",
      "reason": "Stage all changes for commit",
      "expected_outcome": "Changes staged",
      "depends_on": [2, 3],
      "params": {
        "paths": ["."]
      }
    },
    {
      "id": 5,
      "action": "git_commit",
      "reason": "Commit the staged changes with provided message",
      "expected_outcome": "Changes committed to repository",
      "depends_on": [4],
      "params": {
        "message": "feat: add authentication",
        "signoff": true
      }
    }
  ]
}
```

---

### 2.2 Execution LLM Prompts

#### System Prompt
```text
You are a precise tool executor. Given a specific task, you output the exact tool call(s) needed to accomplish it.

AVAILABLE TOOLS:

FILESYSTEM:
- file_read: {"path": "file/path", "max_bytes": 1048576}
- file_write: {"path": "file/path", "content": "...", "create_dirs": true}
- file_patch: {"path": "file/path", "unified_diff": "..."}
- fs_list: {"glob": "pattern/**/*.rs", "max_results": 5000}
- grep: {"pattern": "regex", "glob": "src/**/*.rs", "case_sensitive": false}

GIT:
- git_status: {}
- git_diff: {"rev": "HEAD", "paths": ["src/**"]}
- git_add: {"paths": ["src/main.rs"]}
- git_commit: {"message": "fix: bug", "signoff": true}
- git_checkout: {"target": "branch-name", "create": false}

SHELL:
- shell_exec: {"cmd": "cargo test", "cwd": ".", "timeout_ms": 600000}
- proc_list: {"filter": "rust", "limit": 50}

NETWORK:
- curl: {"method": "GET", "url": "https://...", "timeout_ms": 15000}

INSTRUCTIONS:
- Output ONLY valid JSON
- No explanations or markdown
- Use exact file paths (no patterns for file operations)
- Be precise with arguments
- If multiple tools needed, output array

FORMAT:
Single tool: {"tool": "tool_name", "args": {...}}
Multiple tools: [{"tool": "tool1", "args": {...}}, {"tool": "tool2", "args": {...}}]
```

#### Task Context Template
```text
# Task {id}: {action}

**Goal:** {reason}
**Target:** {target}
**Expected Outcome:** {expected_outcome}

## Previous Task Results

{context_from_previous_tasks}

## Your Task

Output the tool call(s) needed to accomplish this task. Remember:
- Use exact paths (not patterns)
- Be specific with arguments
- Output ONLY JSON
```

---

## 3. Configuration Templates

### 3.1 Workspace Configuration

```yaml
# workspace.yaml
workspace:
  root: "/path/to/project"
  
  # Tool policies
  tools:
    allowed_actions:
      - read_file
      - write_file
      - edit_file
      - list_files
      - grep
      - run_test
      - run_build
      - git_status
      - git_diff
      - git_add
      - git_commit
      - git_checkout
      - shell_exec
      - search_code
      - analyze_error
    
    # Shell command allowlist
    shell_allowlist:
      - "^cargo\\s"
      - "^npm\\s"
      - "^rustc\\s"
      - "^git\\s"
      - "^rg\\s"
    
    # Network restrictions
    network:
      allowed_domains:
        - "crates.io"
        - "docs.rs"
        - "github.com"
  
  # Validation settings
  validation:
    max_task_depth: 10
    strict_task_sequence: true
    require_targets_for:
      - read_file
      - write_file
      - edit_file
  
  # Path restrictions
  paths:
    sandbox_to_workspace: true
    allow_parent_dir: false
```

### 3.2 Orchestrator Configuration

```yaml
# orchestrator.yaml
orchestrator:
  # Planning LLM
  planning_llm:
    provider: "local"  # or "remote"
    model: "deepseek-r1"
    base_url: "http://localhost:11434"
    temperature: 0.3
    max_tokens: 4000
    timeout_ms: 30000
  
  # Execution LLM
  execution_llm:
    provider: "local"
    model: "llama3"
    base_url: "http://localhost:11434"
    temperature: 0.1  # More deterministic
    max_tokens: 2000
    timeout_ms: 10000
  
  # Action interpreter settings
  interpreter:
    max_retries: 3
    retry_delay_ms: 1000
    
    # Context management
    context:
      max_tokens: 8000
      compression_strategy: "keep_ends"
      keep_first_tasks: 2
      keep_last_tasks: 3
    
    # Result formatting
    formatting:
      max_content_length: 2000
      truncate_large_outputs: true
      include_reasoning: false
  
  # WebSocket client
  aco_client:
    url: "ws://localhost:8080"
    reconnect_attempts: 5
    reconnect_delay_ms: 2000
    request_timeout_ms: 60000
```

### 3.3 aco Configuration

```yaml
# aco.yaml
aco:
  # Server settings
  server:
    bind: "127.0.0.1:8080"
    max_connections: 10
    heartbeat_interval_ms: 30000
  
  # Tool runtime
  runtime:
    # Workspace path
    workspace: "/path/to/project"
    
    # Default timeouts
    timeouts:
      filesystem: 10000      # 10s
      git: 30000            # 30s
      shell: 600000         # 10m
      network: 15000        # 15s
    
    # Resource limits
    limits:
      max_file_size: 10485760      # 10MB
      max_output_size: 5242880     # 5MB
      max_concurrent_tools: 5
    
    # Policy enforcement
    policy:
      enable_sandbox: true
      enable_command_allowlist: true
      enable_network_restrictions: true
      
      # Validation rules
      validators:
        - rule: "sec.paths.sandbox"
          enforcement: "blocking"
        - rule: "sec.shell.allowlist"
          enforcement: "blocking"
        - rule: "sec.network.allowlist"
          enforcement: "blocking"
        - rule: "build.must.pass"
          enforcement: "warning"
```

---

## 4. Tool Definition Templates

### 4.1 Tool Schema Definition

```yaml
tool: file_read
category: filesystem
description: Read contents of a file

parameters:
  - name: path
    type: string
    required: true
    description: Path to file relative to workspace
  
  - name: max_bytes
    type: integer
    required: false
    default: 1048576
    description: Maximum bytes to read

returns:
  type: object
  properties:
    content:
      type: string
      description: File contents
    sha256:
      type: string
      description: SHA256 hash of content
    size:
      type: integer
      description: File size in bytes

errors:
  - code: E_FILE_IO
    conditions:
      - File not found
      - Permission denied
      - File too large

examples:
  - input:
      path: "src/main.rs"
    output:
      content: "fn main() { ... }"
      sha256: "abc123..."
      size: 256
```

---

## 5. Pattern Configuration Templates

### 5.1 ReAct Pattern

```yaml
# patterns/react.yaml
name: react
type: react
description: Reasoning and Acting pattern for iterative problem solving

config:
  max_iterations: 10
  allow_tool_calling: true
  tools:
    - file_read
    - file_write
    - shell_exec
    - grep

prompts:
  system: |
    You are a coding assistant using the ReAct (Reasoning and Acting) pattern.
    
    For each step:
    1. Thought: Think about what to do next
    2. Action: Choose a tool and arguments
    3. Observation: Receive tool result
    
    Continue until task is complete, then provide final answer.

use_cases:
  - Exploratory debugging
  - Iterative development
  - Unknown problem scope
```

### 5.2 Plan-Execute Pattern

```yaml
# patterns/plan-execute.yaml
name: plan_execute
type: plan_execute
description: Plan first, then execute plan

config:
  planning_model: "deepseek-r1"
  execution_model: "llama3"
  allow_replanning: true
  max_replans: 2

prompts:
  planning: |
    Create a detailed plan to accomplish: {query}
    Output structured JSON task list.
  
  execution: |
    Execute task: {task}
    Previous results: {context}

use_cases:
  - Well-defined problems
  - Multi-step workflows
  - Need for upfront planning
```

---

## 6. Workflow State Templates

### 6.1 Execution State

```json
{
  "execution_id": "exec-abc123",
  "plan_id": "plan-xyz789",
  "status": "in_progress",
  "current_task": 3,
  "total_tasks": 7,
  "started_at": "2025-01-15T10:00:00Z",
  "tasks_completed": [
    {
      "task_id": 1,
      "status": "success",
      "duration_ms": 245,
      "tool_calls": 1
    },
    {
      "task_id": 2,
      "status": "success",
      "duration_ms": 1023,
      "tool_calls": 2
    }
  ],
  "current_task_state": {
    "task_id": 3,
    "started_at": "2025-01-15T10:00:05Z",
    "attempts": 1,
    "status": "running"
  }
}
```

---

## 7. Monitoring Templates

### 7.1 Metrics Collection

```yaml
metrics:
  # Task plan metrics
  plan_metrics:
    - plan_generation_time_ms
    - plan_validation_time_ms
    - plan_tasks_count
    - plan_complexity_score
  
  # Task execution metrics
  task_metrics:
    - task_execution_time_ms
    - task_success_rate
    - task_retry_count
    - tasks_per_plan
  
  # Tool metrics
  tool_metrics:
    - tool_calls_total
    - tool_execution_time_ms
    - tool_success_rate
    - tool_errors_by_type
  
  # LLM metrics
  llm_metrics:
    - llm_call_time_ms
    - llm_tokens_used
    - llm_cost_per_task
    - llm_error_rate
  
  # System metrics
  system_metrics:
    - websocket_latency_ms
    - context_window_usage
    - memory_usage_mb
    - active_sessions
```

---

## 8. Testing Templates

### 8.1 Task Plan Test Cases

```yaml
test_cases:
  - name: simple_file_read
    input_query: "Read the README.md file"
    expected_tasks: 1
    expected_actions:
      - read_file
    validates:
      - plan_structure
      - task_count
      - action_types
  
  - name: debug_workflow
    input_query: "Fix the failing test_auth test"
    expected_tasks: 5-7
    expected_actions:
      - read_file
      - run_test
      - analyze_error
      - edit_file
      - run_test
    validates:
      - plan_structure
      - dependencies
      - task_sequence
  
  - name: git_commit
    input_query: "Commit changes with message 'fix: bug'"
    expected_tasks: 3-5
    expected_actions:
      - git_status
      - git_diff
      - git_add
      - git_commit
    validates:
      - plan_structure
      - git_workflow
      - params_present
```

---

## 9. Error Recovery Templates

### 9.1 Retry Strategies

```yaml
retry_strategies:
  # Planning LLM failures
  planning_failure:
    max_retries: 3
    backoff: exponential
    initial_delay_ms: 1000
    recovery:
      - strategy: simplify_prompt
        description: "Remove examples, use simpler language"
      - strategy: alternative_model
        description: "Fall back to different planning model"
  
  # Execution LLM failures
  execution_failure:
    max_retries: 2
    backoff: linear
    initial_delay_ms: 500
    recovery:
      - strategy: retry_with_context
        description: "Retry with more context from previous tasks"
      - strategy: skip_task
        description: "Skip non-critical task"
  
  # Tool execution failures
  tool_failure:
    max_retries: 1
    backoff: none
    recovery:
      - strategy: alternative_tool
        description: "Try alternative tool if available"
      - strategy: fail_gracefully
        description: "Mark task as failed, continue if possible"
```

---

## 10. Priority Matrix

### Workflow Implementation Priority

| Priority | Workflow | Complexity | Value | Implementation Order |
|----------|----------|------------|-------|---------------------|
| 1 | Simple File Read | Low | High | Week 1 |
| 2 | Git Commit | Low | High | Week 1 |
| 3 | Test Failure Debug | High | High | Week 2 |
| 4 | Fix Bug | Medium | High | Week 2 |
| 5 | Code Search | Low | Medium | Week 3 |
| 6 | Build Failure | Medium | High | Week 3 |
| 7 | Add Feature | Very High | Medium | Week 4 |
| 8 | Refactor Function | Medium | Medium | Week 4 |
| 9 | Module Structure | Medium | Low | Week 5 |
| 10 | Code Review | Medium | Medium | Week 5 |
| 11 | Add Documentation | Low | Low | Week 6 |

---

## Summary

**Total Templates Defined:**
- 15 Task Plan Templates (workflows)
- 2 LLM System Prompts (Planning + Execution)
- 3 Few-Shot Examples
- 3 Configuration Templates (workspace, orchestrator, aco)
- 1 Tool Schema Template
- 2 Pattern Templates
- 4 State/Monitoring Templates
- 3 Testing Templates
- 3 Error Recovery Templates

**Immediate Needs (Week 1):**
1. Planning LLM system prompt
2. Execution LLM system prompt
3. Simple file read workflow
4. Git commit workflow
5. Basic orchestrator config
6. Basic aco config

**Future Additions:**
- More specific domain workflows (web dev, ML, etc.)
- Language-specific patterns (Python, JavaScript, etc.)
- Project-type specific workflows (CLI, web app, library)
- Custom user-defined workflows

This catalog should evolve as you discover new patterns from actual usage.
