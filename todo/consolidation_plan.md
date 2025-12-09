# Crate Consolidation Plan

## Overview

Analysis of orca, aco, and orchestrator reveals significant code duplication. This plan consolidates shared logic into orchestrator as the central shared library.

## Identified Duplications

### 1. PromptService (HIGH PRIORITY)

| Crate | File | Purpose |
|-------|------|---------|
| **orca** | `src/services/prompt_service.rs` | Send prompts via TaskExecutor + ReAct agent |
| **orchestrator** | `src/services/prompt.rs` | Send prompts to various LLM providers |

**Differences:**
- Orca: Uses TaskExecutor with DirectToolBridge, agent-based execution
- Orchestrator: Simple LLM client wrapper, no tools

**Resolution:** Keep orchestrator's simple version for basic prompts, add orca's agent-based execution as separate method.

---

### 2. BackupService (HIGH PRIORITY)

| Crate | File | Purpose |
|-------|------|---------|
| **orca** | `src/services/backup.rs` | Full backup/restore/export/import with table groups |
| **orchestrator** | `src/api/handlers/data.rs` | API handlers that call orca's BackupService |

**Current State:** Orchestrator already imports orca's BackupService.

**Resolution:** Move BackupService to orchestrator, have orca re-export it. This requires breaking the circular dependency.

---

### 3. Tool Implementations (HIGH PRIORITY)

| Crate | File | Tools |
|-------|------|-------|
| **orca** | `src/tools/mod.rs` (DirectToolBridge) | file_read, file_write, fs_list, git_status, git_diff, shell_exec |
| **aco** | `src/tools.rs` | FileReadTool, FileWriteTool, FsListTool, GitStatusTool, GitDiffTool, ShellExecTool, GrepTool |

**Differences:**
- Orca: DirectToolBridge with execute_tool() interface
- ACO: Individual tool structs implementing langgraph Tool trait

**Resolution:** Create shared tool implementations in orchestrator, both orca and aco import them.

---

### 4. Configuration Loading (MEDIUM PRIORITY)

| Crate | File | Config Type |
|-------|------|-------------|
| **orca** | `src/config/loader.rs` | OrcaConfig from TOML (user + project) |
| **aco** | `src/config/loader.rs` | AcoConfig from TOML (user + project) |
| **orchestrator** | `src/config/loader.rs` | ServerConfig from YAML |

**Differences:**
- Same dual-location pattern (user + project)
- Different config structures
- Different file formats (TOML vs YAML)

**Resolution:** Keep separate configs but share the loading pattern via a trait.

---

### 5. TUI Architecture (MEDIUM PRIORITY)

| Crate | File | Features |
|-------|------|----------|
| **orca** | `src/tui/` | Full TUI with conversation, config editor, menus, dialogs |
| **aco** | `src/tui/` | Simpler TUI with task/workflow lists, execution stream |

**Shared Patterns:**
- App state management (View enum, AppState struct)
- Event handling (EventHandler, Event enum)
- UI rendering (ratatui-based)
- gRPC/HTTP client for server communication

**Resolution:** Extract common TUI components (event handler, base rendering) to orchestrator.

---

### 6. Task/Workflow Models (MEDIUM PRIORITY)

| Crate | Location | Models |
|-------|----------|--------|
| **orca** | `src/workflow.rs`, `src/models/` | Task, Workflow, TaskStatus, WorkflowStatus |
| **orchestrator** | `src/db/models/` | Task, Workflow, WorkflowTask, ToolExecution |

**Differences:**
- Orca: Simpler models for standalone use
- Orchestrator: Full models with database mapping

**Resolution:** Use orchestrator's models as the canonical source, orca maps to them.

---

### 7. Repositories (LOW PRIORITY)

| Crate | Count | Examples |
|-------|-------|----------|
| **orca** | 11 repos | TaskRepository, WorkflowRepository, BudgetRepository... |
| **orchestrator** | 11 repos | TaskRepository, WorkflowRepository, ToolExecutionRepository... |

**Differences:**
- Same pattern, different table schemas
- Orca has user/project split, orchestrator has single DB

**Resolution:** Keep separate for now, future consolidation possible.

---

### 8. Workspace/Path Security (LOW PRIORITY)

| Crate | File | Features |
|-------|------|----------|
| **orca** | DirectToolBridge | Basic path validation |
| **aco** | `src/workspace/security.rs` | PathValidator, symlink blocking, system path blocking |

**Resolution:** Use aco's more robust security in shared tools.

---

## Consolidation Strategy

### Phase 1: Break Circular Dependency

**Problem:** orchestrator → orca dependency prevents orca → orchestrator

**Solution:** Create `orca-common` crate OR restructure dependencies

**Option A: New Shared Crate**
```
src/crates/
├── orca-common/     # NEW: Shared types, traits, utilities
│   ├── models/      # Task, Workflow, etc.
│   ├── tools/       # Tool trait, implementations
│   ├── services/    # BackupService, PricingService
│   └── config/      # Config traits
├── orca/            # Depends on orca-common
├── aco/             # Depends on orca-common
└── orchestrator/    # Depends on orca-common
```

**Option B: Invert Dependencies**
```
# Current: orchestrator → orca
# New: orca → orchestrator (for services)
#      orchestrator → orca (for specific integrations)
# Use feature flags to break cycle
```

### Phase 2: Move Services to Shared Location

1. **BackupService** → orchestrator/src/services/backup.rs
2. **Tool implementations** → orchestrator/src/tools/ OR orca-common/tools/
3. **Common TUI components** → orchestrator/src/tui/ OR orca-common/tui/

### Phase 3: Update Imports

1. Update orca to import from orchestrator/orca-common
2. Update aco to import from orchestrator/orca-common
3. Remove duplicate code from orca and aco

---

## Recommended Immediate Actions

### Action 1: Move Tools to Orchestrator

Create `orchestrator/src/tools/` with:
- `file_read.rs` - Read file contents
- `file_write.rs` - Write file contents
- `fs_list.rs` - List directory
- `git_status.rs` - Git status
- `git_diff.rs` - Git diff
- `shell_exec.rs` - Shell command execution
- `grep.rs` - Search tool (from aco)
- `mod.rs` - ToolBridge trait and exports

Both orca and aco import these.

### Action 2: Consolidate BackupService

1. Move `orca/src/services/backup.rs` to `orchestrator/src/services/backup.rs`
2. Update imports in orca to use `orchestrator::services::backup::*`
3. Update imports in aco to use `orchestrator::services::backup::*`

### Action 3: Create Shared Config Trait

```rust
// In orchestrator/src/config/traits.rs
pub trait DualLocationConfig {
    fn user_path() -> PathBuf;
    fn project_path() -> PathBuf;
    fn load() -> Result<Self>;
    fn save(&self) -> Result<()>;
}
```

Both OrcaConfig and AcoConfig implement this.

---

## Files to Create/Modify

### New Files in Orchestrator

| File | Purpose |
|------|---------|
| `src/tools/mod.rs` | Tool exports and ToolBridge trait |
| `src/tools/file_read.rs` | File read tool |
| `src/tools/file_write.rs` | File write tool |
| `src/tools/fs_list.rs` | Directory listing |
| `src/tools/git_status.rs` | Git status |
| `src/tools/git_diff.rs` | Git diff |
| `src/tools/shell_exec.rs` | Shell execution |
| `src/tools/grep.rs` | Search tool |
| `src/tools/path_validator.rs` | Security/path validation |

### Files to Modify in Orca

| File | Change |
|------|--------|
| `src/services/mod.rs` | Re-export from orchestrator |
| `src/tools/mod.rs` | Use orchestrator tools |
| `Cargo.toml` | Add orchestrator dependency (feature-gated) |

### Files to Modify in ACO

| File | Change |
|------|--------|
| `src/tools.rs` | Import from orchestrator |
| `Cargo.toml` | Keep orchestrator dependency |

---

## Dependency Resolution

### Current State
```
orchestrator ──depends on──► orca
aco ──────────depends on──► orchestrator
```

### After Consolidation (Option A)
```
orca-common (new shared crate)
    ▲
    │
┌───┴───┬───────────┐
│       │           │
orca    aco    orchestrator
```

### After Consolidation (Option B - Feature Flags)
```
orchestrator [with feature "orca-integration"]
    ▲
    │
┌───┴───┐
│       │
orca    aco
```

---

## Migration Order

1. **Phase 1** (This PR): Create shared tools in orchestrator
2. **Phase 2**: Move BackupService with feature flag
3. **Phase 3**: Consolidate TUI components
4. **Phase 4**: Unify models with mapping layer
5. **Phase 5**: Clean up and remove duplicates

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Circular dependency | High | Feature flags or new crate |
| Breaking changes | Medium | Incremental migration |
| Testing coverage | Medium | Add integration tests |
| Build time increase | Low | Feature flags for optional deps |

---

## Success Metrics

- [ ] Single source of truth for tool implementations
- [ ] Single BackupService used by all crates
- [ ] Reduced total lines of code
- [ ] No duplicate business logic
- [ ] All tests passing
- [ ] Build time not significantly increased
