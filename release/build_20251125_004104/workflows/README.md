# Agentic YAML Workflows for Orchestrator

This directory contains production-ready YAML workflow definitions for the Orca orchestrator system. These workflows implement agentic patterns for common software development tasks including debugging, code analysis, task planning, validation, and code review.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Agent Patterns](#agent-patterns)
- [Available Workflows](#available-workflows)
- [YAML Workflow Structure](#yaml-workflow-structure)
- [Available Tools](#available-tools)
- [Pattern Selection Guide](#pattern-selection-guide)
- [Examples](#examples)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

The orchestrator supports agentic workflows where LLM-powered agents use tools to complete complex tasks autonomously. This collection provides:

- **6 Core Workflows** for common development tasks
- **3 Agent Patterns** (ReAct, Plan-Execute, Reflection)
- **8+ Tools** for file operations, code search, and execution
- **Production-ready** configurations with error handling

### Architecture

```
User Request
     ↓
Orchestrator (loads YAML workflow)
     ↓
Agent Pattern (ReAct/Plan-Execute/Reflection)
     ↓
Tool Execution (file_read, grep, shell_exec, etc.)
     ↓
Results
```

## Quick Start

### 1. Load a Workflow

Workflows can be loaded into the orchestrator from these YAML files:

```bash
# Using the ACO CLI
aco workflow load workflows/debugging/reactive_debug.yaml

# Or using the orchestrator API
curl -X POST http://localhost:8080/api/workflows \
  -H "Content-Type: application/yaml" \
  --data-binary @workflows/debugging/reactive_debug.yaml
```

### 2. Execute a Workflow

```bash
# Execute debugging workflow
aco workflow execute reactive_debugging \
  --input "Debug test failure in test_user_auth"

# Execute code search
aco workflow execute code_search \
  --input "Find all implementations of the ChatModel trait"

# Execute task breakdown
aco workflow execute task_breakdown_planning \
  --input "Refactor the authentication module to use async/await"
```

### 3. Monitor Progress

```bash
# Check workflow status
aco workflow status <workflow-id>

# Stream workflow output
aco workflow stream <workflow-id>
```

## Agent Patterns

The orchestrator supports three proven agent patterns, each optimized for different use cases:

### 1. ReAct Pattern (`react_1`)

**Reasoning + Acting in a tight loop**

```
Think → Act → Observe → Think → Act → Observe → ...
```

- **Use Cases**: Debugging, code search, log analysis, Q&A
- **Strengths**: Fast, flexible, handles uncertainty well
- **LLM Calls**: 1-5 per task
- **Latency**: Low (seconds)
- **Best For**: 90% of use cases

**Example**:
```yaml
- name: "debug_issue"
  pattern: "react_1"
  config:
    max_iterations: 10
    tools: [file_read, grep, shell_exec]
```

### 2. Plan-Execute Pattern (`plan_execute_1`)

**Explicit planning before execution**

```
Plan (create steps) → Execute step 1 → Execute step 2 → ... → Replan (if needed)
```

- **Use Cases**: Multi-step tasks, research, complex workflows
- **Strengths**: Organized approach, handles dependencies
- **LLM Calls**: 2-20 per task
- **Latency**: Medium (tens of seconds)
- **Best For**: Complex tasks requiring coordination

**Example**:
```yaml
- name: "implement_feature"
  pattern: "plan_execute_1"
  config:
    max_steps: 15
    replanning_enabled: true
    tools: [file_read, file_write, grep, shell_exec]
```

### 3. Reflection Pattern (`reflection_1`)

**Self-critique and iterative improvement**

```
Generate → Critique → Refine → Critique → Refine → ...
```

- **Use Cases**: Code review, validation, quality-critical work
- **Strengths**: High quality output, catches own mistakes
- **LLM Calls**: 3-10 per task
- **Latency**: High (minutes)
- **Best For**: Quality-critical tasks

**Example**:
```yaml
- name: "review_code"
  pattern: "reflection_1"
  config:
    max_iterations: 3
    quality_threshold: 0.85
    tools: [file_read, shell_exec]
```

## Available Workflows

### Debugging

| Workflow | Pattern | Purpose | Use When |
|----------|---------|---------|----------|
| **reactive_debug.yaml** | ReAct | Debug errors by analyzing code and running diagnostics | Test failures, runtime errors, unexpected behavior |

### Analysis

| Workflow | Pattern | Purpose | Use When |
|----------|---------|---------|----------|
| **log_review.yaml** | ReAct | Scan logs for errors, categorize by severity | Investigating failures, monitoring system health |
| **code_search.yaml** | ReAct | Find functions, classes, patterns in codebase | Locating code, understanding structure |

### Planning

| Workflow | Pattern | Purpose | Use When |
|----------|---------|---------|----------|
| **task_breakdown.yaml** | Plan-Execute | Break complex tasks into executable steps | Multi-step implementations, refactoring |

### Validation

| Workflow | Pattern | Purpose | Use When |
|----------|---------|---------|----------|
| **task_validation.yaml** | Reflection | Validate task completion with self-critique | Quality gates, requirement verification |

### Review

| Workflow | Pattern | Purpose | Use When |
|----------|---------|---------|----------|
| **code_review.yaml** | Reflection | Comprehensive code review with quality assessment | Pre-commit reviews, pull requests |

## YAML Workflow Structure

### Basic Structure

```yaml
id: "unique_workflow_id"
description: "What this workflow does"

steps:
  - name: "step_name"
    pattern: "react_1|plan_execute_1|reflection_1"
    config:
      max_iterations: 10
      tools:
        - tool_name_1
        - tool_name_2
      system_prompt: |
        Instructions for the agent
    on_success: "next_step_name"  # or {end: true}
    on_failure: "error_handler"    # or {end: true}

settings:
  max_total_steps: 20
  enable_retries: true
  max_retries: 2
  timeout: 300  # seconds
```

### Step Transitions

Control workflow flow with `on_success` and `on_failure`:

```yaml
# Simple: go to next step
on_success: "next_step"

# Simple: end workflow
on_success:
  end: true

# Conditional: branch based on state
on_failure:
  if: "steps_completed > 3"
  then:
    end: true
  else: "retry_step"
```

### Pattern-Specific Configuration

**ReAct**:
```yaml
pattern: "react_1"
config:
  max_iterations: 10
  tools: [file_read, grep]
  system_prompt: |
    Agent instructions
```

**Plan-Execute**:
```yaml
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
  tools: [file_read, file_write]
```

**Reflection**:
```yaml
pattern: "reflection_1"
config:
  max_iterations: 3
  quality_threshold: 0.85
  generator_config:
    model: "gpt-4"
  critic_config:
    model: "gpt-4"
    temperature: 0.3
  tools: [file_read, shell_exec]
```

## Available Tools

### Filesystem Tools

| Tool | Purpose | Example |
|------|---------|---------|
| **file_read** | Read file contents | Read source code, config files |
| **file_write** | Write to files | Generate reports, create files |
| **file_patch** | Apply patches | Modify code sections |
| **fs_list** | List directory contents | Explore directory structure |
| **fs_copy** | Copy files | Backup, duplicate files |
| **fs_move** | Move/rename files | Reorganize files |
| **fs_delete** | Delete files | Clean up temporary files |

### Search Tools

| Tool | Purpose | Example |
|------|---------|---------|
| **grep** | Search file contents with regex | Find functions, error patterns |

### Execution Tools

| Tool | Purpose | Example |
|------|---------|---------|
| **shell_exec** | Execute shell commands | Run tests, linters, build commands |

### Git Tools

| Tool | Purpose | Example |
|------|---------|---------|
| **git_status** | Check repository status | See modified files |
| **git_diff** | View changes | Review code changes |
| **git_add** | Stage changes | Prepare commits |
| **git_commit** | Commit changes | Save changes |

## Pattern Selection Guide

Use this flowchart to choose the right pattern:

```
START: What type of task?
    │
    ├─ Need tool calling? ──────────[YES]───→ Use ReAct
    │                                         (debugging, search, analysis)
    ├─ Multi-step with planning? ───[YES]───→ Use Plan-Execute
    │                                         (feature implementation, research)
    ├─ Quality-critical output? ────[YES]───→ Use Reflection
    │                                         (code review, validation)
    └─ Custom requirements? ────────[YES]───→ Combine patterns or
                                              build custom workflow
```

### Pattern Characteristics

| Pattern | Speed | Flexibility | Quality | Complexity |
|---------|-------|-------------|---------|------------|
| ReAct | ⚡⚡⚡ | ⭐⭐⭐ | ⭐⭐ | Low |
| Plan-Execute | ⚡⚡ | ⭐⭐ | ⭐⭐⭐ | Medium |
| Reflection | ⚡ | ⭐ | ⭐⭐⭐ | High |

## Examples

### Example 1: Simple ReAct Workflow

Minimal example for learning the ReAct pattern:

```yaml
id: "simple_file_search"
description: "Find files matching a pattern"

steps:
  - name: "search"
    pattern: "react_1"
    config:
      max_iterations: 5
      tools:
        - grep
        - file_read
      system_prompt: |
        Search for the requested pattern and provide
        file paths with matching lines.
    on_success:
      end: true

settings:
  max_total_steps: 10
```

See `examples/simple_react.yaml` for full example.

### Example 2: Multi-Step Workflow

Combining multiple steps:

```yaml
id: "analyze_and_report"
description: "Analyze code and generate report"

steps:
  # Step 1: Analyze with ReAct
  - name: "analyze"
    pattern: "react_1"
    config:
      max_iterations: 8
      tools: [file_read, grep]
      system_prompt: "Analyze the codebase..."
    on_success: "generate_report"

  # Step 2: Generate report
  - name: "generate_report"
    pattern: "react_1"
    config:
      max_iterations: 2
      tools: [file_write]
      system_prompt: "Create a markdown report..."
    on_success:
      end: true

settings:
  max_total_steps: 15
```

### Example 3: Error Handling

Robust error handling with fallback:

```yaml
steps:
  - name: "primary_approach"
    pattern: "plan_execute_1"
    config:
      max_steps: 10
      tools: [file_read, file_write]
    on_success:
      end: true
    on_failure: "fallback_approach"

  - name: "fallback_approach"
    pattern: "react_1"
    config:
      max_iterations: 5
      tools: [file_read]
    on_success:
      end: true
    on_failure:
      end: true
```

## Best Practices

### 1. Tool Selection

✅ **DO**:
- Include only tools needed for the step
- Use file_read before file_write
- Use grep for searching, file_read for reading

❌ **DON'T**:
- Give all tools to every step
- Use shell_exec for file operations
- Over-specify tool usage in prompts

### 2. System Prompts

✅ **DO**:
- Be specific about the task
- Provide output format guidance
- Include examples when helpful
- Guide tool usage

❌ **DON'T**:
- Write overly long prompts
- Micromanage every step
- Assume knowledge not in the codebase

### 3. Iteration Limits

```yaml
# ReAct: 5-10 iterations
max_iterations: 10

# Plan-Execute: 10-15 steps
max_steps: 15

# Reflection: 2-4 iterations
max_iterations: 3
```

### 4. Error Handling

Always specify both success and failure paths:

```yaml
on_success:
  end: true
on_failure:
  end: true  # or fallback step
```

### 5. Workflow Settings

```yaml
settings:
  max_total_steps: 20       # Total across all steps
  enable_retries: true      # Retry transient failures
  max_retries: 2            # How many retries
  timeout: 300              # Seconds (5 minutes)
  enable_parallel: false    # Usually false for sequential
```

## Troubleshooting

### Workflow Won't Load

**Problem**: YAML parsing error

**Solutions**:
- Validate YAML syntax (use online validator)
- Check indentation (use spaces, not tabs)
- Verify all required fields present
- Ensure tool names are correct

### Agent Runs Out of Iterations

**Problem**: `max_iterations` or `max_steps` reached

**Solutions**:
- Increase iteration limit
- Simplify the task
- Improve system prompt clarity
- Break into multiple steps

### Tools Not Available

**Problem**: Agent tries to use tool not in `tools` list

**Solutions**:
- Add missing tool to tools list
- Update system prompt to mention available tools
- Use different tool for same purpose

### Quality Threshold Not Met (Reflection)

**Problem**: Reflection pattern fails to meet `quality_threshold`

**Solutions**:
- Lower quality threshold (e.g., 0.75 instead of 0.85)
- Increase max_iterations
- Improve system prompt for critic
- Add fallback step for manual review

### Workflow Times Out

**Problem**: Exceeds `timeout` setting

**Solutions**:
- Increase timeout value
- Reduce scope of task
- Use faster pattern (ReAct instead of Reflection)
- Split into multiple workflows

## Contributing

When creating new workflows:

1. Follow existing naming conventions
2. Include comprehensive documentation in YAML comments
3. Test with orchestrator before committing
4. Provide clear system prompts
5. Handle both success and failure cases
6. Set reasonable iteration/step limits

## See Also

- [Orchestrator Documentation](../docs/orchestrator.md)
- [Agent Patterns Guide](../docs/agent-patterns.md)
- [Tool Reference](../docs/tools.md)
- [CLAUDE.md](../CLAUDE.md) - Project rules and structure

---

**Version**: 1.0
**Last Updated**: 2025-11-15
**Maintained By**: Orca Development Team
