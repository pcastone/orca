# Action Interpreter Specification
## Two-Tier LLM Architecture (Planning + Execution)

## Overview

The Action Interpreter bridges two LLMs with different capabilities:
- **Planning LLM (8B-20B):** Reasoning model that breaks complex goals into structured tasks
- **Execution LLM (4B-8B):** Instruction model that executes tasks and makes tool calls

**Core Responsibilities:**
1. Parse and validate Planning LLM's structured task list
2. Orchestrate task execution through Execution LLM
3. Validate tool calls from Execution LLM
4. Manage execution state and results
5. Format results back to LLMs

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    User Query                               │
│              "Debug this test failure"                       │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ↓
┌─────────────────────────────────────────────────────────────┐
│              Planning LLM (Deepseek R1 / o1)                │
│  • Analyzes problem                                         │
│  • Breaks into subtasks                                     │
│  • Outputs structured JSON task list                        │
└────────────────────────┬────────────────────────────────────┘
                         │
                         │ TaskPlan (JSON)
                         │
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                 Action Interpreter                          │
│  ┌──────────────────────────────────────────────────┐      │
│  │  Phase 1: Plan Validation                        │      │
│  │  • Parse JSON structure                          │      │
│  │  • Validate task schema                          │      │
│  │  • Check task dependencies                       │      │
│  │  • Resolve ambiguous targets (file paths, etc)   │      │
│  └──────────────────────────────────────────────────┘      │
└────────────────────────┬────────────────────────────────────┘
                         │
                         │ Validated TaskPlan
                         │
                         ↓
┌─────────────────────────────────────────────────────────────┐
│              Task Execution Loop (for each task)            │
│                                                             │
│  ┌──────────────────────────────────────────────────┐      │
│  │  1. Send Task to Execution LLM                   │      │
│  │     • Task description                           │      │
│  │     • Context from previous tasks                │      │
│  │     • Available tools                            │      │
│  └──────────────────────────────────────────────────┘      │
│                         │                                   │
│                         ↓                                   │
│  ┌──────────────────────────────────────────────────┐      │
│  │  Execution LLM (Llama3 / Mistral)                │      │
│  │  • Receives task + context                       │      │
│  │  • Outputs tool call(s) in JSON                  │      │
│  └──────────────────────────────────────────────────┘      │
│                         │                                   │
│                         │ ToolCall(s) JSON                  │
│                         ↓                                   │
│  ┌──────────────────────────────────────────────────┐      │
│  │  2. Action Interpreter Validation                │      │
│  │     • Parse tool call JSON                       │      │
│  │     • Validate against tool schema               │      │
│  │     • Check permissions (policy pre-check)       │      │
│  │     • Resolve paths/parameters                   │      │
│  └──────────────────────────────────────────────────┘      │
│                         │                                   │
│                         │ ToolRequest(s)                    │
│                         ↓                                   │
│  ┌──────────────────────────────────────────────────┐      │
│  │  3. Send to aco via WebSocket                    │      │
│  └──────────────────────────────────────────────────┘      │
│                         │                                   │
│                         ↓                                   │
│  ┌──────────────────────────────────────────────────┐      │
│  │  aco Tool Runtime                                │      │
│  │  • Policy enforcement                            │      │
│  │  • Tool execution                                │      │
│  │  • Returns ToolResponse                          │      │
│  └──────────────────────────────────────────────────┘      │
│                         │                                   │
│                         │ ToolResponse                      │
│                         ↓                                   │
│  ┌──────────────────────────────────────────────────┐      │
│  │  4. Result Formatting                            │      │
│  │     • Convert ToolResponse → natural language    │      │
│  │     • Truncate large outputs                     │      │
│  │     • Add to task result                         │      │
│  └──────────────────────────────────────────────────┘      │
│                         │                                   │
│                         │ Continue to next task...          │
│                         ↓                                   │
└─────────────────────────────────────────────────────────────┘
                         │
                         │ All tasks complete
                         ↓
┌─────────────────────────────────────────────────────────────┐
│              Final Result Assembly                          │
│  • Aggregate all task results                              │
│  • Optionally send to Planning LLM for review              │
│  • Return to user                                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Structures

### 1. Task Plan (Planning LLM Output)

```rust
/// Output from Planning LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    /// Unique plan identifier
    pub plan_id: String,
    
    /// Original user query
    pub query: String,
    
    /// High-level goal description
    pub goal: String,
    
    /// List of tasks to execute
    pub tasks: Vec<Task>,
    
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task ID (1-indexed for clarity)
    pub id: usize,
    
    /// Action type: "read_file", "run_test", "edit_file", etc.
    pub action: String,
    
    /// Target of the action (file path, command, etc)
    pub target: Option<String>,
    
    /// Human-readable reason for this task
    pub reason: String,
    
    /// Expected outcome / success criteria
    pub expected_outcome: Option<String>,
    
    /// Dependencies on other tasks (by ID)
    pub depends_on: Vec<usize>,
    
    /// Optional parameters for the action
    pub params: Option<serde_json::Value>,
}
```

**Example:**
```json
{
  "plan_id": "plan-abc123",
  "query": "Debug the failing integration test",
  "goal": "Identify and fix the bug causing test_login to fail",
  "tasks": [
    {
      "id": 1,
      "action": "read_file",
      "target": "tests/integration_test.rs",
      "reason": "Understand what the test is checking",
      "expected_outcome": "See test assertions and setup",
      "depends_on": [],
      "params": null
    },
    {
      "id": 2,
      "action": "run_test",
      "target": "integration_test::test_login",
      "reason": "See the actual error message",
      "expected_outcome": "Get failure details",
      "depends_on": [1],
      "params": null
    },
    {
      "id": 3,
      "action": "read_file",
      "target": "src/auth/login.rs",
      "reason": "Examine the login function being tested",
      "expected_outcome": "Find the bug in implementation",
      "depends_on": [2],
      "params": null
    },
    {
      "id": 4,
      "action": "edit_file",
      "target": "src/auth/login.rs",
      "reason": "Fix the null pointer dereference on line 42",
      "expected_outcome": "Code compiles without the bug",
      "depends_on": [3],
      "params": {
        "line_number": 42,
        "fix_type": "null_check"
      }
    },
    {
      "id": 5,
      "action": "run_test",
      "target": "integration_test::test_login",
      "reason": "Verify the fix works",
      "expected_outcome": "Test passes",
      "depends_on": [4],
      "params": null
    }
  ]
}
```

---

### 2. Tool Call (Execution LLM Output)

```rust
/// Output from Execution LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name from Tool Runtime SDK
    pub tool: String,
    
    /// Tool arguments
    pub args: serde_json::Value,
    
    /// Optional reasoning (why this tool?)
    pub reasoning: Option<String>,
}

/// Execution LLM can output single or multiple tool calls
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolCallResponse {
    Single(ToolCall),
    Multiple(Vec<ToolCall>),
}
```

**Example (Single):**
```json
{
  "tool": "file_read",
  "args": {
    "path": "tests/integration_test.rs",
    "max_bytes": 1048576
  },
  "reasoning": "Need to see what the test is checking"
}
```

**Example (Multiple - for complex tasks):**
```json
[
  {
    "tool": "file_read",
    "args": {"path": "src/auth/login.rs"}
  },
  {
    "tool": "grep",
    "args": {
      "pattern": "null",
      "glob": "src/auth/*.rs"
    }
  }
]
```

---

### 3. Execution State

```rust
/// Tracks execution progress
#[derive(Debug, Clone)]
pub struct ExecutionState {
    /// The task plan being executed
    pub plan: TaskPlan,
    
    /// Current task index (0-based)
    pub current_task_index: usize,
    
    /// Results from completed tasks
    pub task_results: HashMap<usize, TaskResult>,
    
    /// Execution start time
    pub started_at: Instant,
    
    /// Accumulated context for LLM
    pub context: String,
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: usize,
    pub success: bool,
    pub tool_calls: Vec<ToolCall>,
    pub tool_responses: Vec<ToolResponse>,
    pub summary: String,
    pub duration_ms: u64,
    pub error: Option<String>,
}
```

---

## Action Interpreter Components

### Module Structure

```
orchestrator/src/interpreter/
├── mod.rs                  # Public API
├── plan_validator.rs       # Phase 1: Validate Planning LLM output
├── task_executor.rs        # Phase 2: Execute tasks via Execution LLM
├── tool_validator.rs       # Phase 3: Validate tool calls
├── result_formatter.rs     # Phase 4: Format results
├── context_manager.rs      # Manage LLM context window
└── prompts.rs             # LLM prompt templates
```

---

### 1. Plan Validator (`plan_validator.rs`)

**Responsibility:** Parse and validate the Planning LLM's task list

```rust
pub struct PlanValidator {
    /// Tool schemas for validation
    tool_schemas: HashMap<String, ToolSchema>,
    
    /// Workspace context for path resolution
    workspace: WorkspaceContext,
}

impl PlanValidator {
    /// Parse and validate a task plan
    pub fn validate(&self, json_output: &str) -> Result<TaskPlan> {
        // 1. Parse JSON
        let plan: TaskPlan = serde_json::from_str(json_output)
            .context("Failed to parse task plan JSON")?;
        
        // 2. Validate structure
        self.validate_structure(&plan)?;
        
        // 3. Check dependencies (no cycles, valid IDs)
        self.validate_dependencies(&plan)?;
        
        // 4. Resolve ambiguous targets
        let resolved_plan = self.resolve_targets(plan)?;
        
        Ok(resolved_plan)
    }
    
    /// Ensure plan structure is valid
    fn validate_structure(&self, plan: &TaskPlan) -> Result<()> {
        if plan.tasks.is_empty() {
            return Err(anyhow!("Task plan has no tasks"));
        }
        
        // Task IDs should be sequential 1, 2, 3...
        for (i, task) in plan.tasks.iter().enumerate() {
            if task.id != i + 1 {
                return Err(anyhow!(
                    "Task IDs must be sequential, expected {}, got {}",
                    i + 1,
                    task.id
                ));
            }
        }
        
        Ok(())
    }
    
    /// Check task dependencies are valid
    fn validate_dependencies(&self, plan: &TaskPlan) -> Result<()> {
        for task in &plan.tasks {
            for dep_id in &task.depends_on {
                // Dependency must reference earlier task
                if *dep_id >= task.id {
                    return Err(anyhow!(
                        "Task {} depends on task {}, but dependencies must be on earlier tasks",
                        task.id,
                        dep_id
                    ));
                }
                
                // Dependency must exist
                if !plan.tasks.iter().any(|t| t.id == *dep_id) {
                    return Err(anyhow!(
                        "Task {} depends on non-existent task {}",
                        task.id,
                        dep_id
                    ));
                }
            }
        }
        
        // TODO: Check for circular dependencies (shouldn't happen with forward-only deps)
        
        Ok(())
    }
    
    /// Resolve ambiguous targets (partial paths, glob patterns, etc)
    fn resolve_targets(&self, mut plan: TaskPlan) -> Result<TaskPlan> {
        for task in &mut plan.tasks {
            if let Some(target) = &task.target {
                // If target looks like a partial path, resolve it
                if target.contains("*") {
                    // It's a glob, leave as-is
                    continue;
                }
                
                // Try to resolve as a path
                if let Ok(resolved) = self.workspace.resolve_path(target) {
                    task.target = Some(resolved.to_string_lossy().to_string());
                }
            }
        }
        
        Ok(plan)
    }
}
```

---

### 2. Task Executor (`task_executor.rs`)

**Responsibility:** Execute tasks through Execution LLM and aco

```rust
pub struct TaskExecutor {
    /// Connection to aco
    aco_client: Arc<AcoClient>,
    
    /// Tool call validator
    tool_validator: ToolValidator,
    
    /// Result formatter
    result_formatter: ResultFormatter,
    
    /// Context manager (for LLM context window)
    context_manager: ContextManager,
    
    /// Execution LLM client
    execution_llm: Arc<dyn ChatModel>,
}

impl TaskExecutor {
    /// Execute an entire task plan
    pub async fn execute_plan(&self, plan: TaskPlan) -> Result<ExecutionResult> {
        let mut state = ExecutionState {
            plan: plan.clone(),
            current_task_index: 0,
            task_results: HashMap::new(),
            started_at: Instant::now(),
            context: String::new(),
        };
        
        // Execute each task in sequence
        for task_index in 0..plan.tasks.len() {
            state.current_task_index = task_index;
            let task = &plan.tasks[task_index];
            
            // Check dependencies
            self.check_dependencies(&state, task)?;
            
            // Execute the task
            let task_result = self.execute_task(&state, task).await?;
            
            // Update state
            state.task_results.insert(task.id, task_result.clone());
            
            // Update context for next task
            self.context_manager.add_task_result(&mut state.context, &task_result);
            
            // Check if task failed
            if !task_result.success {
                return Ok(ExecutionResult {
                    success: false,
                    final_state: state,
                    error: task_result.error,
                });
            }
        }
        
        Ok(ExecutionResult {
            success: true,
            final_state: state,
            error: None,
        })
    }
    
    /// Execute a single task
    async fn execute_task(
        &self,
        state: &ExecutionState,
        task: &Task,
    ) -> Result<TaskResult> {
        let start = Instant::now();
        
        // 1. Build prompt for Execution LLM
        let prompt = self.build_task_prompt(state, task);
        
        // 2. Call Execution LLM
        let llm_request = ChatRequest::new(vec![
            Message::system(include_str!("prompts/execution_system.txt")),
            Message::human(&prompt),
        ]);
        
        let llm_response = self.execution_llm.chat(llm_request).await?;
        let llm_output = llm_response.message.text()
            .ok_or_else(|| anyhow!("No text in LLM response"))?;
        
        // 3. Parse tool call(s) from LLM output
        let tool_calls = self.parse_tool_calls(llm_output)?;
        
        // 4. Validate and execute tool calls
        let mut tool_responses = Vec::new();
        for tool_call in &tool_calls {
            // Validate before sending
            self.tool_validator.validate(tool_call)?;
            
            // Convert to ToolRequest
            let tool_request = self.tool_call_to_request(tool_call)?;
            
            // Execute via aco
            let tool_response = self.aco_client.execute_tool(tool_request).await?;
            tool_responses.push(tool_response);
        }
        
        // 5. Format result
        let summary = self.result_formatter.format_task_result(
            task,
            &tool_calls,
            &tool_responses,
        );
        
        // 6. Determine success
        let success = tool_responses.iter().all(|r| r.ok);
        
        Ok(TaskResult {
            task_id: task.id,
            success,
            tool_calls,
            tool_responses,
            summary,
            duration_ms: start.elapsed().as_millis() as u64,
            error: if success { None } else { 
                Some("One or more tool calls failed".to_string()) 
            },
        })
    }
    
    /// Build prompt for Execution LLM
    fn build_task_prompt(&self, state: &ExecutionState, task: &Task) -> String {
        let mut prompt = String::new();
        
        // Task description
        prompt.push_str(&format!("# Task {}: {}\n\n", task.id, task.action));
        prompt.push_str(&format!("**Goal:** {}\n\n", task.reason));
        
        if let Some(target) = &task.target {
            prompt.push_str(&format!("**Target:** {}\n\n", target));
        }
        
        if let Some(expected) = &task.expected_outcome {
            prompt.push_str(&format!("**Expected Outcome:** {}\n\n", expected));
        }
        
        // Context from previous tasks
        if !state.context.is_empty() {
            prompt.push_str("\n## Previous Task Results\n\n");
            prompt.push_str(&state.context);
        }
        
        // Available tools (condensed list)
        prompt.push_str("\n## Available Tools\n\n");
        prompt.push_str(&self.get_available_tools_summary());
        
        // Instructions
        prompt.push_str("\n## Instructions\n\n");
        prompt.push_str("Output a JSON object with the tool call(s) needed to accomplish this task.\n");
        prompt.push_str("Format: {\"tool\": \"<tool_name>\", \"args\": {...}}\n");
        
        prompt
    }
    
    /// Parse tool call JSON from LLM output
    fn parse_tool_calls(&self, output: &str) -> Result<Vec<ToolCall>> {
        // Extract JSON from markdown code blocks if present
        let json_str = self.extract_json(output)?;
        
        // Try parsing as single or multiple tool calls
        let tool_calls: Vec<ToolCall> = match serde_json::from_str::<ToolCallResponse>(json_str) {
            Ok(ToolCallResponse::Single(call)) => vec![call],
            Ok(ToolCallResponse::Multiple(calls)) => calls,
            Err(e) => return Err(anyhow!("Failed to parse tool calls: {}", e)),
        };
        
        if tool_calls.is_empty() {
            return Err(anyhow!("No tool calls found in output"));
        }
        
        Ok(tool_calls)
    }
    
    /// Extract JSON from markdown code blocks
    fn extract_json<'a>(&self, text: &'a str) -> Result<&'a str> {
        // Look for ```json ... ``` blocks
        if let Some(start) = text.find("```json") {
            if let Some(end) = text[start..].find("```") {
                let json_start = start + 7; // "```json".len()
                let json_end = start + end;
                return Ok(text[json_start..json_end].trim());
            }
        }
        
        // Look for ``` ... ``` blocks (no language specifier)
        if let Some(start) = text.find("```") {
            if let Some(end) = text[start + 3..].find("```") {
                let json_start = start + 3;
                let json_end = start + 3 + end;
                return Ok(text[json_start..json_end].trim());
            }
        }
        
        // No code blocks, assume entire output is JSON
        Ok(text.trim())
    }
    
    /// Check task dependencies are satisfied
    fn check_dependencies(&self, state: &ExecutionState, task: &Task) -> Result<()> {
        for dep_id in &task.depends_on {
            match state.task_results.get(dep_id) {
                Some(result) if result.success => continue,
                Some(result) => {
                    return Err(anyhow!(
                        "Task {} depends on task {}, which failed",
                        task.id,
                        dep_id
                    ));
                }
                None => {
                    return Err(anyhow!(
                        "Task {} depends on task {}, which hasn't executed yet",
                        task.id,
                        dep_id
                    ));
                }
            }
        }
        Ok(())
    }
}
```

---

### 3. Tool Validator (`tool_validator.rs`)

**Responsibility:** Validate tool calls before sending to aco

```rust
pub struct ToolValidator {
    /// Tool schemas loaded from Tool Runtime SDK
    tool_schemas: HashMap<String, ToolSchema>,
    
    /// Workspace context
    workspace: WorkspaceContext,
}

impl ToolValidator {
    /// Validate a tool call
    pub fn validate(&self, tool_call: &ToolCall) -> Result<()> {
        // 1. Check tool exists
        let schema = self.tool_schemas.get(&tool_call.tool)
            .ok_or_else(|| anyhow!("Unknown tool: {}", tool_call.tool))?;
        
        // 2. Validate arguments against schema
        self.validate_args(schema, &tool_call.args)?;
        
        // 3. Perform semantic validation (paths, permissions, etc)
        self.validate_semantics(tool_call)?;
        
        Ok(())
    }
    
    /// Validate arguments match schema
    fn validate_args(&self, schema: &ToolSchema, args: &serde_json::Value) -> Result<()> {
        // Check required fields are present
        for required_field in &schema.required_fields {
            if !args.get(required_field).is_some() {
                return Err(anyhow!(
                    "Missing required field '{}' for tool '{}'",
                    required_field,
                    schema.name
                ));
            }
        }
        
        // Check field types
        for (field_name, field_value) in args.as_object().unwrap() {
            if let Some(expected_type) = schema.fields.get(field_name) {
                if !self.check_type(field_value, expected_type) {
                    return Err(anyhow!(
                        "Field '{}' has wrong type for tool '{}'",
                        field_name,
                        schema.name
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate semantic constraints (paths exist, permissions, etc)
    fn validate_semantics(&self, tool_call: &ToolCall) -> Result<()> {
        match tool_call.tool.as_str() {
            "file_read" | "file_write" | "file_patch" => {
                // Check path is valid and within workspace
                if let Some(path) = tool_call.args.get("path").and_then(|p| p.as_str()) {
                    self.workspace.validate_path(path)?;
                }
            }
            "shell_exec" => {
                // Check command is allowed (pre-check before aco does it)
                if let Some(cmd) = tool_call.args.get("cmd").and_then(|c| c.as_str()) {
                    if !self.workspace.is_command_allowed(cmd) {
                        return Err(anyhow!(
                            "Command '{}' not in allowlist",
                            cmd
                        ));
                    }
                }
            }
            "curl" => {
                // Check URL domain is allowed
                if let Some(url) = tool_call.args.get("url").and_then(|u| u.as_str()) {
                    self.workspace.validate_url(url)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}
```

---

### 4. Result Formatter (`result_formatter.rs`)

**Responsibility:** Format tool responses into natural language for LLM context

```rust
pub struct ResultFormatter {
    /// Max length for individual file contents
    max_content_length: usize,
}

impl ResultFormatter {
    /// Format task result for LLM context
    pub fn format_task_result(
        &self,
        task: &Task,
        tool_calls: &[ToolCall],
        tool_responses: &[ToolResponse],
    ) -> String {
        let mut result = String::new();
        
        result.push_str(&format!("### Task {} Results\n\n", task.id));
        
        for (call, response) in tool_calls.iter().zip(tool_responses.iter()) {
            result.push_str(&format!("**Tool:** {}\n", call.tool));
            
            if response.ok {
                result.push_str(&self.format_success(call, response));
            } else {
                result.push_str(&self.format_error(response));
            }
            
            result.push_str("\n");
        }
        
        result
    }
    
    /// Format successful tool response
    fn format_success(&self, call: &ToolCall, response: &ToolResponse) -> String {
        match call.tool.as_str() {
            "file_read" => {
                if let Some(content) = response.data.get("content").and_then(|c| c.as_str()) {
                    let truncated = self.truncate(content, self.max_content_length);
                    format!("**File Content:**\n```\n{}\n```\n", truncated)
                } else {
                    "**Result:** File read successfully\n".to_string()
                }
            }
            "file_write" => {
                let bytes = response.data.get("bytes").and_then(|b| b.as_u64()).unwrap_or(0);
                format!("**Result:** Wrote {} bytes to file\n", bytes)
            }
            "shell_exec" => {
                let stdout = response.data.get("stdout").and_then(|s| s.as_str()).unwrap_or("");
                let stderr = response.data.get("stderr").and_then(|s| s.as_str()).unwrap_or("");
                let code = response.data.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
                
                let mut result = format!("**Exit Code:** {}\n", code);
                
                if !stdout.is_empty() {
                    let truncated = self.truncate(stdout, 2000);
                    result.push_str(&format!("**Output:**\n```\n{}\n```\n", truncated));
                }
                
                if !stderr.is_empty() {
                    let truncated = self.truncate(stderr, 2000);
                    result.push_str(&format!("**Errors:**\n```\n{}\n```\n", truncated));
                }
                
                result
            }
            "git_status" => {
                let branch = response.data.get("branch").and_then(|b| b.as_str()).unwrap_or("unknown");
                let changes = response.data.get("changes").and_then(|c| c.as_array()).map(|a| a.len()).unwrap_or(0);
                
                format!("**Result:** On branch '{}', {} change(s)\n", branch, changes)
            }
            _ => {
                format!("**Result:** {}\n", serde_json::to_string_pretty(&response.data).unwrap_or_default())
            }
        }
    }
    
    /// Format error response
    fn format_error(&self, response: &ToolResponse) -> String {
        let errors = response.errors.join("; ");
        format!("**Error:** {}\n", errors)
    }
    
    /// Truncate content to max length
    fn truncate(&self, content: &str, max_len: usize) -> String {
        if content.len() <= max_len {
            return content.to_string();
        }
        
        let half = max_len / 2;
        format!(
            "{}...\n[{} bytes truncated]\n...{}",
            &content[..half],
            content.len() - max_len,
            &content[content.len() - half..]
        )
    }
}
```

---

### 5. Context Manager (`context_manager.rs`)

**Responsibility:** Manage LLM context window to prevent overflow

```rust
pub struct ContextManager {
    /// Max tokens for context (estimated)
    max_context_tokens: usize,
    
    /// Compression strategy
    strategy: CompressionStrategy,
}

#[derive(Debug, Clone)]
pub enum CompressionStrategy {
    /// Keep first N and last N tasks, summarize middle
    KeepEnds { keep_first: usize, keep_last: usize },
    
    /// Keep only essential information
    SummaryOnly,
    
    /// No compression (risky!)
    None,
}

impl ContextManager {
    /// Add task result to context, compressing if needed
    pub fn add_task_result(&self, context: &mut String, result: &TaskResult) {
        let new_content = format!("\n{}\n", result.summary);
        
        // Estimate current token count (rough: 1 token ≈ 4 chars)
        let estimated_tokens = (context.len() + new_content.len()) / 4;
        
        if estimated_tokens > self.max_context_tokens {
            // Compress existing context
            *context = self.compress(context);
        }
        
        context.push_str(&new_content);
    }
    
    /// Compress context to fit within limits
    fn compress(&self, context: &str) -> String {
        match self.strategy {
            CompressionStrategy::KeepEnds { keep_first, keep_last } => {
                // Split into task sections
                let sections: Vec<&str> = context.split("### Task").collect();
                
                if sections.len() <= keep_first + keep_last + 1 {
                    return context.to_string();
                }
                
                let mut compressed = String::new();
                
                // Keep first N
                for section in sections.iter().take(keep_first + 1) {
                    compressed.push_str(section);
                }
                
                // Add summary
                compressed.push_str(&format!(
                    "\n... [{} tasks omitted for brevity] ...\n",
                    sections.len() - keep_first - keep_last - 1
                ));
                
                // Keep last N
                for section in sections.iter().skip(sections.len() - keep_last) {
                    compressed.push_str(section);
                }
                
                compressed
            }
            CompressionStrategy::SummaryOnly => {
                "Previous tasks executed successfully. Details omitted for space.".to_string()
            }
            CompressionStrategy::None => context.to_string(),
        }
    }
}
```

---

## Prompt Templates

### Planning LLM Prompt (`prompts/planning_system.txt`)

```text
You are a planning assistant that breaks down complex coding tasks into structured, executable steps.

When given a user query, analyze the problem and create a JSON task plan with the following structure:

{
  "plan_id": "unique-id",
  "query": "original user query",
  "goal": "high-level goal description",
  "tasks": [
    {
      "id": 1,
      "action": "action_type",
      "target": "target of action (optional)",
      "reason": "why this task is needed",
      "expected_outcome": "what success looks like (optional)",
      "depends_on": [],
      "params": {}
    }
  ]
}

Available action types:
- read_file: Read a file to understand its contents
- write_file: Write or overwrite a file
- edit_file: Modify an existing file
- run_test: Execute tests
- run_build: Build the project
- search_code: Search for patterns in code
- git_status: Check git status
- git_commit: Commit changes
- analyze_error: Analyze an error message

Guidelines:
1. Break complex tasks into 3-7 steps
2. Each task should be atomic and clear
3. Use dependencies to ensure proper ordering
4. Include clear reasoning for each step
5. Think step-by-step about what information is needed

Output ONLY valid JSON, no additional text.
```

---

### Execution LLM Prompt (`prompts/execution_system.txt`)

```text
You are a precise tool executor. Given a specific task, you output the exact tool call(s) needed to accomplish it.

You have access to these tools:

FILESYSTEM:
- file_read: {"path": "file/path", "max_bytes": 1048576}
- file_write: {"path": "file/path", "content": "...", "create_dirs": true}
- file_patch: {"path": "file/path", "unified_diff": "..."}
- fs_list: {"glob": "pattern", "max_results": 5000}
- grep: {"pattern": "regex", "glob": "pattern", "case_sensitive": false}

GIT:
- git_status: {}
- git_diff: {"rev": "HEAD", "paths": ["..."]}
- git_add: {"paths": ["..."]}
- git_commit: {"message": "...", "signoff": true}

SHELL:
- shell_exec: {"cmd": "command", "cwd": ".", "timeout_ms": 600000}
- proc_list: {"filter": "pattern", "limit": 50}

NETWORK:
- curl: {"method": "GET", "url": "...", "timeout_ms": 15000}

For each task, output a JSON object with the tool call(s):

Single tool:
{
  "tool": "tool_name",
  "args": {...}
}

Multiple tools:
[
  {"tool": "tool1", "args": {...}},
  {"tool": "tool2", "args": {...}}
]

Be precise with arguments. Use exact file paths, not patterns.
Output ONLY valid JSON, no explanations or markdown.
```

---

## Integration with Orchestrator

### Main Orchestrator Flow

```rust
// orchestrator/src/lib.rs

pub struct Orchestrator {
    /// Planning LLM (reasoning model)
    planning_llm: Arc<dyn ChatModel>,
    
    /// Execution LLM (instruction model)
    execution_llm: Arc<dyn ChatModel>,
    
    /// Action interpreter
    interpreter: ActionInterpreter,
    
    /// Pattern registry (for advanced patterns)
    pattern_registry: Arc<PatternRegistry>,
}

impl Orchestrator {
    /// Main entry point: Execute user query
    pub async fn execute_query(&self, query: &str) -> Result<String> {
        // Step 1: Planning Phase
        let task_plan = self.planning_phase(query).await?;
        
        // Step 2: Execution Phase
        let result = self.interpreter.execute_plan(task_plan).await?;
        
        // Step 3: Format final result
        let summary = self.format_final_result(result);
        
        Ok(summary)
    }
    
    /// Planning phase: Get structured task plan from Planning LLM
    async fn planning_phase(&self, query: &str) -> Result<TaskPlan> {
        let planning_request = ChatRequest::new(vec![
            Message::system(include_str!("interpreter/prompts/planning_system.txt")),
            Message::human(query),
        ]);
        
        let response = self.planning_llm.chat(planning_request).await?;
        let json_output = response.message.text()
            .ok_or_else(|| anyhow!("No text in planning LLM response"))?;
        
        // Validate and parse plan
        self.interpreter.plan_validator.validate(json_output)
    }
    
    /// Format final execution result for user
    fn format_final_result(&self, result: ExecutionResult) -> String {
        if result.success {
            let mut summary = String::from("✅ Task completed successfully\n\n");
            
            for (task_id, task_result) in &result.final_state.task_results {
                summary.push_str(&format!(
                    "Task {}: {} ({} ms)\n",
                    task_id,
                    if task_result.success { "✓" } else { "✗" },
                    task_result.duration_ms
                ));
            }
            
            summary
        } else {
            format!("❌ Task failed: {}", result.error.unwrap_or_default())
        }
    }
}
```

---

## Error Handling Strategy

### 1. Planning Phase Errors

**Error:** Planning LLM outputs invalid JSON
**Recovery:**
- Retry up to 3 times with clarified prompt
- Fall back to simpler prompt (fewer instructions)
- If all retries fail, ask user for clarification

**Error:** Planning LLM creates impossible dependencies
**Recovery:**
- PlanValidator catches this
- Return error to user with explanation
- Suggest reformulating the query

---

### 2. Execution Phase Errors

**Error:** Execution LLM outputs invalid tool call JSON
**Recovery:**
- Retry with clearer instructions
- If retry fails, skip task and report error
- Continue with next task if dependencies allow

**Error:** Tool execution fails (file not found, command fails, etc)
**Recovery:**
- Log the error
- Mark task as failed
- Check if later tasks depend on this one
  - If yes: abort remaining tasks
  - If no: continue with independent tasks

**Error:** Context window overflow
**Recovery:**
- ContextManager compresses earlier results
- Summarize instead of full detail
- Continue execution

---

### 3. Critical Errors (Abort Execution)

- WebSocket connection lost to aco
- Security policy violation
- Workspace corruption detected
- User cancellation

---

## Testing Strategy

### Unit Tests

**Plan Validator:**
```rust
#[test]
fn test_validate_valid_plan() {
    let json = r#"{
        "plan_id": "test",
        "query": "test query",
        "goal": "test goal",
        "tasks": [
            {"id": 1, "action": "read_file", "target": "test.rs", "reason": "test", "depends_on": []}
        ]
    }"#;
    
    let validator = PlanValidator::new(/* ... */);
    let result = validator.validate(json);
    assert!(result.is_ok());
}

#[test]
fn test_detect_invalid_dependencies() {
    let json = r#"{
        "tasks": [
            {"id": 1, "depends_on": [2]},
            {"id": 2, "depends_on": []}
        ]
    }"#;
    
    // Task 1 depends on task 2, but task 1 comes first!
    let validator = PlanValidator::new(/* ... */);
    let result = validator.validate(json);
    assert!(result.is_err());
}
```

**Tool Validator:**
```rust
#[test]
fn test_validate_file_read() {
    let tool_call = ToolCall {
        tool: "file_read".to_string(),
        args: json!({"path": "src/main.rs"}),
        reasoning: None,
    };
    
    let validator = ToolValidator::new(/* ... */);
    assert!(validator.validate(&tool_call).is_ok());
}

#[test]
fn test_reject_invalid_command() {
    let tool_call = ToolCall {
        tool: "shell_exec".to_string(),
        args: json!({"cmd": "rm -rf /"}),  // Not in allowlist!
        reasoning: None,
    };
    
    let validator = ToolValidator::new(/* ... */);
    assert!(validator.validate(&tool_call).is_err());
}
```

---

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_simple_task() {
    // Setup
    let orchestrator = create_test_orchestrator().await;
    
    // Execute query
    let result = orchestrator.execute_query("Read the main.rs file").await;
    
    // Verify
    assert!(result.is_ok());
    let summary = result.unwrap();
    assert!(summary.contains("completed successfully"));
}

#[tokio::test]
async fn test_multi_task_plan() {
    let orchestrator = create_test_orchestrator().await;
    
    let result = orchestrator.execute_query(
        "Check if there are uncommitted changes, and if so, commit them"
    ).await;
    
    assert!(result.is_ok());
}
```

---

### Mock LLM for Testing

```rust
pub struct MockPlanningLLM {
    responses: Vec<String>,
    current_index: AtomicUsize,
}

impl MockPlanningLLM {
    pub fn with_responses(responses: Vec<String>) -> Self {
        Self {
            responses,
            current_index: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl ChatModel for MockPlanningLLM {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
        let index = self.current_index.fetch_add(1, Ordering::SeqCst);
        let response = self.responses.get(index)
            .ok_or_else(|| anyhow!("No more mock responses"))?;
        
        Ok(ChatResponse {
            message: Message::assistant(response),
            reasoning: None,
            metadata: None,
        })
    }
}
```

---

## Performance Considerations

### Expected Latencies

| Component | Expected Time | Notes |
|-----------|---------------|-------|
| Planning LLM call | 2-5 seconds | Reasoning models are slow |
| Execution LLM call (per task) | 0.5-2 seconds | Faster instruction models |
| Tool execution (per call) | 10-100ms | Local file ops are fast |
| WebSocket round-trip | 1-5ms | Negligible on localhost |
| **Total for 5-task plan** | **5-15 seconds** | Dominated by LLM calls |

### Optimization Opportunities (If Needed)

**NOT RECOMMENDED INITIALLY:**
- Parallel task execution (breaks dependencies)
- Caching tool results (stale data risk)
- Batch tool calls (complicates error handling)

**CONSIDER IF PROVEN BOTTLENECK:**
- Streaming LLM responses (faster perceived latency)
- Smaller execution LLM (trade quality for speed)
- Pre-compute common patterns (e.g., "read this file")

---

## Configuration

```yaml
# orchestrator_config.yaml

action_interpreter:
  # Planning LLM configuration
  planning_llm:
    provider: "local"  # or "remote"
    model: "deepseek-r1"
    base_url: "http://localhost:11434"
    temperature: 0.3
    max_tokens: 4000
  
  # Execution LLM configuration
  execution_llm:
    provider: "local"
    model: "llama3"
    base_url: "http://localhost:11434"
    temperature: 0.1  # More deterministic for tool calls
    max_tokens: 2000
  
  # Validation settings
  validation:
    max_task_depth: 10
    allow_tool_fallback: true
    strict_dependencies: true
  
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
```

---

## Summary

The Action Interpreter for your two-tier architecture has these key characteristics:

**✅ Strengths:**
- Clear separation of concerns (planning vs execution)
- Structured data at every stage (minimal parsing ambiguity)
- Deterministic validation before tool execution
- Graceful error handling at each phase
- Context management to prevent overflow

**⚠️ Challenges:**
- Dependent on LLM following structured output format
- Requires good prompt engineering
- Context compression may lose important details
- Two LLM calls adds latency (but you said it's acceptable)

**🎯 Next Steps:**
1. Implement PlanValidator first (pure Rust, no LLM needed)
2. Mock the LLMs to test TaskExecutor logic
3. Integrate real local LLMs (Ollama)
4. Test end-to-end with simple queries
5. Iterate on prompts based on failure modes

Would you like me to:
1. **Generate the Rust code** for any of these components?
2. **Create example prompts** for the Planning/Execution LLMs?
3. **Design the WebSocket protocol** between orchestrator and aco?
4. **Build a simple prototype** to validate the architecture?

Let me know which piece to tackle next.
