# Agentic YAML Orchestrator Workflow Plan

## Project Goal
Create a comprehensive library of YAML workflow definitions for the orchestrator that enable agentic workflows for common software development tasks including debugging, log analysis, code search, task planning, validation, and code review.

## Architecture Overview

### Orchestrator System
- **Workflow Storage**: SQLite database with `workflows` table
- **Definition Format**: JSON (nodes/edges) or YAML (step-based)
- **Agent Patterns**: ReAct, Plan-Execute, Reflection
- **Tool System**: Filesystem, Shell, Git tools available

### Workflow Configuration Structure
The orchestrator supports two workflow formats:

1. **Node/Edge Graph Format** (JSON):
```json
{
  "nodes": [{"id": "node1", "type": "task", "config": {...}}],
  "edges": [{"source": "node1", "target": "node2"}]
}
```

2. **Step-Based Workflow Format** (YAML - preferred for agentic workflows):
```yaml
id: "workflow_id"
description: "Workflow description"
steps:
  - name: "step_name"
    pattern: "react_1|plan_execute_1|reflection_1"
    config: {...}
    on_success: "next_step"
    on_failure: {end: true}
settings:
  max_total_steps: 20
  enable_retries: true
```

### Available Agent Patterns

1. **ReAct Pattern** (`react_1`)
   - **Use Case**: Tool calling, Q&A, general tasks (90% of use cases)
   - **Flow**: Think → Act → Observe loop
   - **Best For**: Debugging, code search, log analysis
   - **LLM Calls**: 1-5 per task
   - **Latency**: Low (seconds)

2. **Plan-Execute Pattern** (`plan_execute_1`)
   - **Use Case**: Multi-step tasks requiring explicit planning
   - **Flow**: Plan → Execute steps → Replan if needed
   - **Best For**: Task breakdown, complex workflows
   - **LLM Calls**: 2-20 per task
   - **Latency**: Medium (tens of seconds)

3. **Reflection Pattern** (`reflection_1`)
   - **Use Case**: Quality-critical outputs requiring self-critique
   - **Flow**: Generate → Critique → Refine loop
   - **Best For**: Code review, validation
   - **LLM Calls**: 3-10 per task
   - **Latency**: High (minutes)

### Available Tools

**Filesystem Tools**:
- `file_read` - Read file contents
- `file_write` - Write to files
- `fs_list` - List directory contents
- `file_patch` - Apply patches to files
- `grep` - Search file contents with regex
- `fs_copy`, `fs_move`, `fs_delete` - File operations

**Shell Tools**:
- `shell_exec` - Execute shell commands

**Git Tools**:
- `git_status` - Check repository status
- `git_diff` - View changes
- `git_add` - Stage changes
- `git_commit` - Commit changes

## Implementation Plan

### Phase 1: Setup and Infrastructure
- [x] Research orchestrator workflow structure
- [x] Understand agent patterns (ReAct, Plan-Execute, Reflection)
- [x] Identify available tools
- [ ] Create `workflows/` directory structure
- [ ] Create main `workflows/README.md`

**Directory Structure**:
```
workflows/
├── README.md                    # Main documentation
├── debugging/
│   ├── reactive_debug.yaml     # ReAct pattern for debugging
│   └── debug_with_logs.yaml    # Debug + log analysis
├── analysis/
│   ├── log_review.yaml         # Analyze logs for errors
│   └── code_search.yaml        # Find code patterns
├── planning/
│   ├── task_breakdown.yaml     # Plan-Execute for complex tasks
│   └── multi_step_research.yaml
├── validation/
│   ├── task_validation.yaml    # Validate task completion
│   └── requirement_check.yaml  # Check requirements met
├── review/
│   ├── code_review.yaml        # Reflection pattern for code review
│   └── quality_check.yaml      # Quality validation
└── examples/
    ├── simple_react.yaml       # Simple ReAct example
    ├── plan_execute_example.yaml
    └── reflection_example.yaml
```

### Phase 2: Core Workflow Development

#### Task 1: Debugging Workflow (`workflows/debugging/reactive_debug.yaml`)
**Pattern**: ReAct (reactive tool calling)
**Purpose**: Debug errors by analyzing code, running tests, checking logs

**Workflow Steps**:
1. Analyze error message/stack trace
2. Read relevant source files
3. Execute diagnostic commands (tests, linters)
4. Identify root cause
5. Propose fix

**Tools Used**: `file_read`, `grep`, `shell_exec`, `git_diff`

**YAML Structure**:
```yaml
id: "reactive_debugging"
description: "Debug issues using reactive tool calling"
steps:
  - name: "analyze_and_debug"
    pattern: "react_1"
    config:
      max_iterations: 10
      tools:
        - file_read
        - grep
        - shell_exec
        - git_diff
      system_prompt: |
        You are a debugging assistant. Analyze the error, read relevant files,
        run diagnostic commands, and identify the root cause.
    on_success:
      end: true
    on_failure:
      end: true
settings:
  max_total_steps: 15
  enable_retries: true
  max_retries: 2
```

#### Task 2: Log Review Workflow (`workflows/analysis/log_review.yaml`)
**Pattern**: ReAct (search and analyze)
**Purpose**: Scan logs for errors, patterns, anomalies

**Workflow Steps**:
1. Read log files
2. Search for error patterns (ERROR, WARN, FATAL)
3. Extract relevant log entries
4. Categorize issues by severity
5. Generate summary report

**Tools Used**: `file_read`, `grep`, `fs_list`

**YAML Structure**:
```yaml
id: "log_analysis"
description: "Review logs for errors and patterns"
steps:
  - name: "scan_logs"
    pattern: "react_1"
    config:
      max_iterations: 8
      tools:
        - file_read
        - grep
        - fs_list
      system_prompt: |
        You are a log analysis assistant. Scan log files for errors,
        warnings, and anomalies. Categorize by severity and provide a summary.
    on_success: "generate_report"
  - name: "generate_report"
    pattern: "react_1"
    config:
      max_iterations: 3
      tools:
        - file_write
      system_prompt: |
        Generate a structured report of log findings with severity levels,
        timestamps, and recommended actions.
    on_success:
      end: true
settings:
  max_total_steps: 12
```

#### Task 3: Code Search Workflow (`workflows/analysis/code_search.yaml`)
**Pattern**: ReAct (search and extract)
**Purpose**: Find functions, classes, patterns in codebase

**Workflow Steps**:
1. Define search criteria (function name, pattern, etc.)
2. Use grep to search across files
3. Read matching files
4. Extract relevant code snippets
5. Present organized results

**Tools Used**: `grep`, `file_read`, `fs_list`

**YAML Structure**:
```yaml
id: "code_search"
description: "Find code patterns and definitions in codebase"
steps:
  - name: "search_codebase"
    pattern: "react_1"
    config:
      max_iterations: 10
      tools:
        - grep
        - file_read
        - fs_list
      system_prompt: |
        You are a code search assistant. Find the requested functions,
        classes, or patterns in the codebase. Provide file paths,
        line numbers, and relevant context.
    on_success:
      end: true
settings:
  max_total_steps: 15
```

#### Task 4: Task Breakdown Workflow (`workflows/planning/task_breakdown.yaml`)
**Pattern**: Plan-Execute (explicit planning)
**Purpose**: Break complex tasks into executable steps

**Workflow Steps**:
1. **Plan**: Analyze task and create step-by-step plan
2. **Execute**: Execute each planned step with tools
3. **Replan**: Adjust plan based on results (if needed)
4. **Finalize**: Verify completion

**Tools Used**: All available tools

**YAML Structure**:
```yaml
id: "task_breakdown_planning"
description: "Break down complex tasks into manageable steps"
steps:
  - name: "create_plan"
    pattern: "plan_execute_1"
    config:
      max_steps: 15
      replanning_enabled: true
      planner_config:
        model: "gpt-4"
        temperature: 0.7
      executor_config:
        model: "gpt-4"
        temperature: 0.5
      tools:
        - file_read
        - file_write
        - grep
        - shell_exec
        - git_status
        - git_diff
      system_prompt: |
        You are a task planning assistant. Break down the complex task
        into clear, executable steps. Monitor progress and replan if needed.
    on_success:
      end: true
    on_failure:
      if: "steps_completed > 3"
      then:
        end: true
      else: "retry_with_simpler_plan"
  - name: "retry_with_simpler_plan"
    pattern: "plan_execute_1"
    config:
      max_steps: 8
      replanning_enabled: false
    on_success:
      end: true
    on_failure:
      end: true
settings:
  max_total_steps: 25
  enable_retries: true
  max_retries: 1
```

#### Task 5: Task Validation Workflow (`workflows/validation/task_validation.yaml`)
**Pattern**: Reflection (generate → critique → refine)
**Purpose**: Validate task completion and requirements

**Workflow Steps**:
1. **Generate**: Check task outputs against requirements
2. **Critique**: Identify gaps or issues
3. **Refine**: Re-check with improvements
4. **Report**: Final validation status

**Tools Used**: `file_read`, `grep`, `shell_exec`

**YAML Structure**:
```yaml
id: "task_validation"
description: "Validate task completion against requirements"
steps:
  - name: "validate_with_reflection"
    pattern: "reflection_1"
    config:
      max_iterations: 3
      quality_threshold: 0.85
      generator_config:
        model: "gpt-4"
      critic_config:
        model: "gpt-4"
        temperature: 0.3  # More deterministic for validation
      tools:
        - file_read
        - grep
        - shell_exec
      system_prompt: |
        You are a validation assistant. Check that all task requirements
        are met. Critique the completeness and quality. Iterate until
        quality threshold is reached or max iterations hit.
    on_success:
      end: true
    on_failure: "manual_review_required"
  - name: "manual_review_required"
    pattern: "react_1"
    config:
      max_iterations: 2
      tools:
        - file_write
      system_prompt: |
        Validation did not meet quality threshold. Generate a detailed
        report of issues found for manual review.
    on_success:
      end: true
settings:
  max_total_steps: 10
```

#### Task 6: Code Review Workflow (`workflows/review/code_review.yaml`)
**Pattern**: Reflection (quality-critical review)
**Purpose**: Review code for quality, bugs, style

**Workflow Steps**:
1. **Generate**: Initial code analysis
2. **Critique**: Identify issues (bugs, style, performance)
3. **Refine**: Deeper analysis based on critique
4. **Report**: Final review summary

**Tools Used**: `file_read`, `git_diff`, `grep`, `shell_exec`

**YAML Structure**:
```yaml
id: "code_review"
description: "Comprehensive code review with self-critique"
steps:
  - name: "analyze_changes"
    pattern: "react_1"
    config:
      max_iterations: 5
      tools:
        - git_diff
        - file_read
        - grep
      system_prompt: |
        Analyze the git diff to understand what changed.
        Read relevant files for context.
    on_success: "quality_review"
  - name: "quality_review"
    pattern: "reflection_1"
    config:
      max_iterations: 3
      quality_threshold: 0.80
      generator_config:
        model: "gpt-4"
      critic_config:
        model: "gpt-4"
        temperature: 0.2  # Very deterministic for code review
      tools:
        - file_read
        - shell_exec  # For running linters/tests
      system_prompt: |
        You are a code reviewer. Review the code for:
        - Correctness and bugs
        - Code style and conventions
        - Performance issues
        - Security vulnerabilities
        - Test coverage
        Provide detailed feedback and iterate to ensure thorough review.
    on_success: "generate_report"
  - name: "generate_report"
    pattern: "react_1"
    config:
      max_iterations: 2
      tools:
        - file_write
      system_prompt: |
        Generate a structured code review report with:
        - Issues found (categorized by severity)
        - Suggestions for improvement
        - Approval/rejection recommendation
    on_success:
      end: true
settings:
  max_total_steps: 15
  timeout: 300  # 5 minutes max
```

### Phase 3: Documentation and Testing

#### Task 7: Create Comprehensive README
**File**: `workflows/README.md`

**Contents**:
- Overview of workflow system
- Agent pattern selection guide
- Tool reference
- YAML syntax and structure
- Examples for each workflow type
- Troubleshooting guide
- Best practices

#### Task 8: Create Example Workflows
**Files**:
- `workflows/examples/simple_react.yaml` - Basic ReAct example
- `workflows/examples/plan_execute_example.yaml` - Planning example
- `workflows/examples/reflection_example.yaml` - Reflection example

These should be minimal, documented examples for learning.

#### Task 9: Validation Testing
- Parse each YAML file with orchestrator
- Verify schema validation passes
- Check for syntax errors
- Ensure all referenced tools exist
- Test step transitions work correctly

**Test Plan**:
```bash
# For each workflow YAML file
cd src/crates/orchestrator
cargo test workflow_config  # Validate YAML parsing

# Manual validation
# Load workflow into orchestrator
# Verify it appears in workflows table
# Check definition is valid JSON
```

### Phase 4: Advanced Workflows (Optional Extensions)

#### Additional Workflows to Consider:
1. **Multi-step Research** (`workflows/planning/research_workflow.yaml`)
   - Plan-Execute pattern
   - Research topic → Gather info → Synthesize → Report

2. **Debug with Log Analysis** (`workflows/debugging/debug_with_logs.yaml`)
   - Combined debugging + log review
   - Multi-step workflow

3. **Quality Gate** (`workflows/validation/quality_gate.yaml`)
   - Run tests, linters, coverage checks
   - Reflection pattern for quality assessment

4. **Refactoring Assistant** (`workflows/review/refactoring.yaml`)
   - Analyze code structure
   - Suggest refactoring
   - Apply changes with validation

## Implementation Checklist

### Phase 1: Setup ✓ (Research Complete)
- [x] Research orchestrator workflow structure
- [x] Understand agent patterns (ReAct, Plan-Execute, Reflection)
- [x] Identify available tools
- [ ] Create `workflows/` directory structure
- [ ] Create main `workflows/README.md`

### Phase 2: Core Workflows
- [ ] Debugging workflow (`workflows/debugging/reactive_debug.yaml`)
- [ ] Log review workflow (`workflows/analysis/log_review.yaml`)
- [ ] Code search workflow (`workflows/analysis/code_search.yaml`)
- [ ] Task breakdown workflow (`workflows/planning/task_breakdown.yaml`)
- [ ] Task validation workflow (`workflows/validation/task_validation.yaml`)
- [ ] Code review workflow (`workflows/review/code_review.yaml`)

### Phase 3: Documentation
- [ ] Comprehensive README with usage guide
- [ ] Example workflows for learning
- [ ] Pattern selection flowchart
- [ ] Tool reference documentation

### Phase 4: Testing
- [ ] Parse all YAML files successfully
- [ ] Validate against WorkflowConfig schema
- [ ] Test step transitions
- [ ] Verify tool references exist

### Phase 5: Git & Deployment
- [ ] Commit changes with descriptive messages
- [ ] Push to feature branch: `claude/agentic-yaml-orchestrator-plan-01VSspposehwj3gerWhGmAh9`

## Key Design Decisions

### 1. YAML over JSON
- More human-readable
- Better for version control
- Easier to edit and maintain
- Supported by orchestrator via ACO CLI

### 2. Step-Based over Node/Edge
- More intuitive for sequential workflows
- Better error handling with on_success/on_failure
- Easier to understand workflow logic
- Natural fit for agentic patterns

### 3. Pattern Selection
- **Debugging**: ReAct (fast, tool-driven)
- **Log Analysis**: ReAct (search-focused)
- **Code Search**: ReAct (query-driven)
- **Task Breakdown**: Plan-Execute (needs planning)
- **Validation**: Reflection (quality-critical)
- **Code Review**: Reflection (quality-critical)

### 4. Tool Organization
- Keep tool lists minimal per step
- Only include tools needed for that step
- Use system prompts to guide tool selection

### 5. Error Handling
- Enable retries for transient failures
- Define clear on_failure transitions
- Set reasonable max_total_steps limits
- Include timeout for long-running workflows

## Success Criteria

1. ✅ All 6 core workflows created and documented
2. ✅ YAML files parse successfully with orchestrator
3. ✅ Comprehensive README with examples
4. ✅ Each workflow maps to correct agent pattern
5. ✅ Tool references are valid
6. ✅ Workflows committed to git feature branch

## Timeline Estimate

- **Phase 1 (Setup)**: 15-20 minutes
- **Phase 2 (Core Workflows)**: 40-50 minutes
- **Phase 3 (Documentation)**: 20-25 minutes
- **Phase 4 (Testing)**: 15-20 minutes
- **Phase 5 (Git)**: 5-10 minutes

**Total**: ~95-125 minutes (1.5-2 hours)

## Dependencies

- Orchestrator crate with WorkflowConfig support ✓
- ACO CLI with YAML parsing ✓
- Tool definitions (filesystem, shell, git) ✓
- Agent patterns (ReAct, Plan-Execute, Reflection) ✓

## Risks and Mitigations

1. **Risk**: Tool names don't match actual implementation
   - **Mitigation**: Reference tooling crate definitions

2. **Risk**: YAML schema changes
   - **Mitigation**: Follow existing test cases in workflow.rs

3. **Risk**: Workflows too complex for orchestrator
   - **Mitigation**: Start simple, test parsing first

4. **Risk**: Pattern IDs incorrect
   - **Mitigation**: Use exact IDs from tests: `react_1`, `plan_execute_1`, `reflection_1`

## Next Steps

1. **User Review**: Review this plan for approval
2. **Begin Implementation**: Start with Phase 1 (directory setup)
3. **Incremental Development**: Build one workflow at a time
4. **Test Each Workflow**: Validate YAML parsing after each
5. **Document as You Go**: Write README sections alongside workflows
6. **Git Commits**: Commit after each major milestone

---

**Ready to proceed?** Please review this plan and let me know if you'd like any adjustments before I begin implementation.
