# Workflow and Template Generation Plan

## Overview

This plan outlines the implementation of missing workflows and templates based on the comprehensive catalog defined in `/docs/workflows_and_templates.md`.

## Current State Analysis

### What Exists
- ✅ `/workflows/` directory with the following categories:
  - `examples/` - simple_react.yaml, plan_execute_example.yaml, reflection_example.yaml
  - `debugging/` - reactive_debug.yaml
  - `analysis/` - code_search.yaml, log_review.yaml
  - `planning/` - task_breakdown.yaml
  - `review/` - code_review.yaml
  - `validation/` - task_validation.yaml
  - README.md with comprehensive documentation

### What's Missing (from workflows_and_templates.md)

Based on the catalog, we need to create:

#### 1. Task Plan Templates (Workflow Definitions)
**Code Understanding Workflows:**
- [ ] `workflows/understanding/simple_file_read.yaml` - Read and display single file
- [ ] `workflows/understanding/module_structure.yaml` - Map module organization

**Debugging Workflows:**
- [ ] `workflows/debugging/test_failure.yaml` - Investigate failing tests
- [ ] `workflows/debugging/build_failure.yaml` - Fix compilation errors
- [ ] `workflows/debugging/runtime_error.yaml` - Debug runtime panics

**Code Modification Workflows:**
- [ ] `workflows/modification/add_feature.yaml` - Implement new feature
- [ ] `workflows/modification/refactor_function.yaml` - Refactor existing code
- [ ] `workflows/modification/fix_bug.yaml` - Locate and fix bugs

**Git Workflows:**
- [ ] `workflows/git/commit_changes.yaml` - Review and commit
- [ ] `workflows/git/create_branch.yaml` - Create feature branch
- [ ] `workflows/git/prepare_pr.yaml` - Prepare pull request

**Documentation Workflows:**
- [ ] `workflows/documentation/add_docs.yaml` - Document code/features

#### 2. Prompt Templates
- [ ] `templates/prompts/planning_llm_system.txt` - System prompt for Planning LLM
- [ ] `templates/prompts/execution_llm_system.txt` - System prompt for Execution LLM
- [ ] `templates/prompts/task_context.txt` - Task context template
- [ ] `templates/prompts/few_shot_examples.yaml` - Few-shot examples for Planning LLM

#### 3. Configuration Templates
- [ ] `templates/config/workspace.yaml` - Workspace configuration
- [ ] `templates/config/orchestrator.yaml` - Orchestrator configuration
- [ ] `templates/config/aco.yaml` - ACO configuration

#### 4. Pattern Templates
- [ ] `templates/patterns/react.yaml` - ReAct pattern configuration
- [ ] `templates/patterns/plan_execute.yaml` - Plan-Execute pattern
- [ ] `templates/patterns/reflection.yaml` - Reflection pattern

#### 5. Tool Definition Templates
- [ ] `templates/tools/tool_schema.yaml` - Generic tool schema definition
- [ ] `templates/tools/filesystem_tools.yaml` - Filesystem tools catalog
- [ ] `templates/tools/git_tools.yaml` - Git tools catalog
- [ ] `templates/tools/execution_tools.yaml` - Execution tools catalog

#### 6. State and Monitoring Templates
- [ ] `templates/state/execution_state.json` - Execution state schema
- [ ] `templates/monitoring/metrics.yaml` - Metrics collection config

#### 7. Testing Templates
- [ ] `templates/testing/task_plan_tests.yaml` - Task plan test cases

#### 8. Error Recovery Templates
- [ ] `templates/error_recovery/retry_strategies.yaml` - Retry strategies config

## Implementation Plan

### Phase 1: Create Directory Structure (Week 1, Priority 1)
```bash
mkdir -p workflows/understanding
mkdir -p workflows/modification
mkdir -p workflows/git
mkdir -p workflows/documentation
mkdir -p templates/prompts
mkdir -p templates/config
mkdir -p templates/patterns
mkdir -p templates/tools
mkdir -p templates/state
mkdir -p templates/monitoring
mkdir -p templates/testing
mkdir -p templates/error_recovery
```

### Phase 2: High-Priority Workflows (Week 1, Priority 1)

#### Task 1: Simple File Read Workflow
**File:** `workflows/understanding/simple_file_read.yaml`
**Complexity:** Low
**Value:** High
**Description:** Single-task workflow to read and display file contents

#### Task 2: Git Commit Workflow
**File:** `workflows/git/commit_changes.yaml`
**Complexity:** Low
**Value:** High
**Description:** Multi-step workflow: git status → diff → add → commit

### Phase 3: Essential Prompts (Week 1, Priority 1)

#### Task 3: Planning LLM System Prompt
**File:** `templates/prompts/planning_llm_system.txt`
**Description:** Comprehensive system prompt for the Planning LLM with:
- Available actions catalog
- Planning guidelines
- Dependency rules
- Output requirements

#### Task 4: Execution LLM System Prompt
**File:** `templates/prompts/execution_llm_system.txt`
**Description:** System prompt for Execution LLM with:
- Available tools catalog
- Tool call format
- Precision requirements

### Phase 4: Medium-Priority Workflows (Week 2, Priority 2)

#### Task 5: Test Failure Debug Workflow
**File:** `workflows/debugging/test_failure.yaml`
**Complexity:** High
**Value:** High
**Description:** Plan-Execute pattern for debugging failing tests

#### Task 6: Fix Bug Workflow
**File:** `workflows/modification/fix_bug.yaml`
**Complexity:** Medium
**Value:** High
**Description:** ReAct pattern for locating and fixing bugs

#### Task 7: Build Failure Workflow
**File:** `workflows/debugging/build_failure.yaml`
**Complexity:** Medium
**Value:** High
**Description:** Iterative workflow for fixing compilation errors

### Phase 5: Configuration Templates (Week 2-3, Priority 2)

#### Task 8: Orchestrator Configuration
**File:** `templates/config/orchestrator.yaml`
**Description:** Template for orchestrator with:
- Planning LLM config
- Execution LLM config
- Action interpreter settings
- WebSocket client config

#### Task 9: ACO Configuration
**File:** `templates/config/aco.yaml`
**Description:** Template for ACO with:
- Server settings
- Tool runtime config
- Resource limits
- Policy enforcement

#### Task 10: Workspace Configuration
**File:** `templates/config/workspace.yaml`
**Description:** Template for workspace with:
- Tool policies
- Shell command allowlist
- Path restrictions
- Validation settings

### Phase 6: Pattern Templates (Week 3, Priority 2)

#### Task 11: ReAct Pattern Template
**File:** `templates/patterns/react.yaml`
**Description:** ReAct pattern configuration with use cases

#### Task 12: Plan-Execute Pattern Template
**File:** `templates/patterns/plan_execute.yaml`
**Description:** Plan-Execute pattern configuration

#### Task 13: Reflection Pattern Template
**File:** `templates/patterns/reflection.yaml`
**Description:** Reflection pattern configuration

### Phase 7: Additional Workflows (Week 3-4, Priority 3)

#### Task 14: Add Feature Workflow
**File:** `workflows/modification/add_feature.yaml`
**Complexity:** Very High
**Value:** Medium
**Description:** Complex multi-step feature implementation

#### Task 15: Refactor Function Workflow
**File:** `workflows/modification/refactor_function.yaml`
**Complexity:** Medium
**Value:** Medium

#### Task 16: Module Structure Workflow
**File:** `workflows/understanding/module_structure.yaml`
**Complexity:** Medium
**Value:** Low

### Phase 8: Tool Definition Templates (Week 4, Priority 3)

#### Task 17: Tool Schema Template
**File:** `templates/tools/tool_schema.yaml`
**Description:** Generic tool definition schema

#### Task 18: Filesystem Tools Catalog
**File:** `templates/tools/filesystem_tools.yaml`
**Description:** Catalog of all filesystem tools

#### Task 19: Git Tools Catalog
**File:** `templates/tools/git_tools.yaml`
**Description:** Catalog of all git tools

### Phase 9: Git and Documentation Workflows (Week 5, Priority 3)

#### Task 20: Create Branch Workflow
**File:** `workflows/git/create_branch.yaml`
**Complexity:** Low
**Value:** Medium

#### Task 21: Prepare PR Workflow
**File:** `workflows/git/prepare_pr.yaml`
**Complexity:** Medium
**Value:** Medium

#### Task 22: Add Documentation Workflow
**File:** `workflows/documentation/add_docs.yaml`
**Complexity:** Low-Medium
**Value:** Low

#### Task 23: Runtime Error Debug Workflow
**File:** `workflows/debugging/runtime_error.yaml`
**Complexity:** High
**Value:** High

### Phase 10: Supporting Templates (Week 5-6, Priority 4)

#### Task 24: Few-Shot Examples
**File:** `templates/prompts/few_shot_examples.yaml`
**Description:** Example task plans for Planning LLM

#### Task 25: Task Context Template
**File:** `templates/prompts/task_context.txt`
**Description:** Template for task context formatting

#### Task 26: Execution State Schema
**File:** `templates/state/execution_state.json`
**Description:** JSON schema for execution state

#### Task 27: Metrics Configuration
**File:** `templates/monitoring/metrics.yaml`
**Description:** Metrics collection configuration

#### Task 28: Retry Strategies
**File:** `templates/error_recovery/retry_strategies.yaml`
**Description:** Error recovery and retry strategies

#### Task 29: Test Cases Template
**File:** `templates/testing/task_plan_tests.yaml`
**Description:** Test cases for task plan validation

### Phase 11: Execution Tools Catalog (Week 6, Priority 4)

#### Task 30: Execution Tools Catalog
**File:** `templates/tools/execution_tools.yaml`
**Description:** Catalog of shell and execution tools

## File System Layout

```
/home/user/orca/
├── workflows/
│   ├── README.md (existing)
│   ├── examples/ (existing)
│   ├── debugging/ (existing + new files)
│   │   ├── reactive_debug.yaml (existing)
│   │   ├── test_failure.yaml (new)
│   │   ├── build_failure.yaml (new)
│   │   └── runtime_error.yaml (new)
│   ├── analysis/ (existing)
│   ├── planning/ (existing)
│   ├── review/ (existing)
│   ├── validation/ (existing)
│   ├── understanding/ (new)
│   │   ├── simple_file_read.yaml
│   │   └── module_structure.yaml
│   ├── modification/ (new)
│   │   ├── add_feature.yaml
│   │   ├── refactor_function.yaml
│   │   └── fix_bug.yaml
│   ├── git/ (new)
│   │   ├── commit_changes.yaml
│   │   ├── create_branch.yaml
│   │   └── prepare_pr.yaml
│   └── documentation/ (new)
│       └── add_docs.yaml
│
└── templates/ (new)
    ├── prompts/
    │   ├── planning_llm_system.txt
    │   ├── execution_llm_system.txt
    │   ├── task_context.txt
    │   └── few_shot_examples.yaml
    ├── config/
    │   ├── workspace.yaml
    │   ├── orchestrator.yaml
    │   └── aco.yaml
    ├── patterns/
    │   ├── react.yaml
    │   ├── plan_execute.yaml
    │   └── reflection.yaml
    ├── tools/
    │   ├── tool_schema.yaml
    │   ├── filesystem_tools.yaml
    │   ├── git_tools.yaml
    │   └── execution_tools.yaml
    ├── state/
    │   └── execution_state.json
    ├── monitoring/
    │   └── metrics.yaml
    ├── testing/
    │   └── task_plan_tests.yaml
    └── error_recovery/
        └── retry_strategies.yaml
```

## Success Criteria

- [ ] All workflow YAML files are valid and follow the established structure
- [ ] All templates are complete and match the specifications in workflows_and_templates.md
- [ ] Each workflow has:
  - Clear description
  - Appropriate pattern selection
  - Correct tool lists
  - Comprehensive system prompts
  - Proper error handling
- [ ] All configuration templates are usable
- [ ] Directory structure is organized and consistent
- [ ] All files are committed to git with clear messages

## Testing Strategy

For each workflow created:
1. Validate YAML syntax
2. Verify all required fields present
3. Check tool names against available tools
4. Ensure system prompts are clear and actionable
5. Test success/failure path logic

## Priority Summary

**Week 1 (Immediate):**
- Simple file read workflow
- Git commit workflow
- Planning LLM system prompt
- Execution LLM system prompt
- Basic configuration templates

**Week 2-3 (High Priority):**
- Debugging workflows (test failure, build failure, bug fix)
- Configuration templates (orchestrator, aco, workspace)
- Pattern templates

**Week 4-5 (Medium Priority):**
- Code modification workflows
- Git workflows
- Tool definition templates
- Documentation workflows

**Week 6 (Lower Priority):**
- Supporting templates (monitoring, testing, error recovery)
- Advanced tool catalogs

## Notes

- Follow existing YAML workflow structure in `/workflows/` directory
- Re-use and extend existing patterns before creating new ones
- Keep workflows simple and focused on single responsibilities
- All system prompts should be clear, specific, and actionable
- Configuration templates should have sensible defaults
- Tool definitions should match the actual implementation in the codebase

## Next Steps

1. ✅ Review this plan with the user
2. Get approval before implementation
3. Begin with Phase 1: Create directory structure
4. Proceed with high-priority workflows and templates
5. Test each file after creation
6. Commit changes incrementally with clear messages
