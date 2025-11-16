# Documentation Gaps Analysis

**Date**: 2025-11-16
**Analyzed By**: Claude Code
**Purpose**: Identify gaps, inaccuracies, and outdated information between `/docs` directory and actual codebase

---

## Executive Summary

Analysis of the `/docs` directory revealed **15 major discrepancies** between documentation and actual codebase implementation, including:
- Incorrect workspace configuration information
- Missing referenced files and directories
- Outdated crate counts and structure descriptions
- Inaccurate module organization details
- Environment-specific hardcoded paths

**Severity Breakdown**:
- **Critical**: 5 issues (fundamental inaccuracies)
- **High**: 6 issues (significant discrepancies)
- **Medium**: 4 issues (minor inaccuracies or missing details)

---

## Critical Issues

### 1. ❌ Workspace Configuration Contradiction (CLAUDE.md)

**Location**: `/CLAUDE.md` line 26
**Issue**: States "This is NOT a standard Cargo workspace - there is no root `Cargo.toml`"
**Reality**: Root `Cargo.toml` exists at project root with full workspace configuration
**Impact**: Misleads developers about build system architecture
**Fix Required**: Update CLAUDE.md to reflect actual workspace structure

```toml
# Actual root Cargo.toml exists with:
[workspace]
members = [
    "src/crates/aco",
    "src/crates/orca",
    "src/crates/orchestrator",
    # ... 10 total crates
]
```

### 2. ❌ Incorrect Crate Count (environment.md)

**Location**: `/docs/environment.md` line 41
**Issue**: States "This is a Cargo workspace with 6 member crates"
**Reality**: Workspace has **10 member crates**
**Impact**: Incomplete understanding of project structure

**Actual Crates**:
1. aco
2. orca
3. orchestrator
4. langgraph-core
5. langgraph-checkpoint
6. langgraph-prebuilt
7. langgraph-cli
8. llm
9. tooling
10. utils

**Missing from docs**: aco, llm, tooling, utils

### 3. ❌ Non-existent Directories Referenced (environment.md)

**Location**: `/docs/environment.md` lines 20-34
**Issue**: Documents `scripts/` and `release/` directories that don't exist
**Reality**: Neither directory exists in repository
**Impact**: Broken workflow instructions

**File System Layout Claims**:
```
acolib/
   scripts/                 # ❌ DOES NOT EXIST
   release/                 # ❌ DOES NOT EXIST
      *.dylib              # ❌ DOES NOT EXIST
      logs/                # ❌ DOES NOT EXIST
```

### 4. ❌ Missing Build Scripts (BUILD.md)

**Location**: `/docs/BUILD.md` lines 22-83
**Issue**: Extensively documents scripts that don't exist:
- `./scripts/build-orca.sh`
- `./scripts/build-dist.sh`

**Reality**: Scripts directory doesn't exist
**Impact**: Users cannot follow documented build procedures
**Fix Required**: Either create scripts or update docs to use direct cargo commands

### 5. ❌ Non-existent CONTRIBUTING.md Referenced

**Location**: `/docs/architecture.md` line 507
**Issue**: References `[CONTRIBUTING.md](../CONTRIBUTING.md)` that doesn't exist
**Reality**: File not found in repository
**Impact**: Broken documentation link

---

## High Priority Issues

### 6. ⚠️ Incorrect Pregel Module Structure (architecture.md)

**Location**: `/docs/architecture.md` lines 54-66
**Issue**: Documents `pregel/barrier.rs` as a separate file
**Reality**: Barrier functionality is integrated in other modules (loop_impl.rs, types.rs)

**Documented Structure**:
```
langgraph-core/
├── pregel/
│   ├── barrier.rs     # ❌ DOES NOT EXIST
```

**Actual Structure**:
```
langgraph-core/
├── pregel/
│   ├── mod.rs
│   ├── types.rs       # Contains barrier types
│   ├── loop_impl.rs   # Contains barrier logic
│   ├── executor.rs
│   ├── channel.rs
│   ├── checkpoint.rs
│   ├── algo.rs
│   └── io.rs
```

### 7. ⚠️ Incorrect ACO Crate Structure (architecture.md)

**Location**: `/docs/architecture.md` lines 145-154
**Issue**: Documents aco crate structure that doesn't match reality

**Documented**:
```
aco/
├── web/               # Web UI (React/Angular)
├── tui/               # Terminal UI (ratatui)
├── cli/               # Command-line interface
└── server.rs          # API server
```

**Actual**:
```
aco/src/
├── auth.rs
├── client.rs
├── config/
├── cli/               # ✓ EXISTS
├── tui/               # ✓ EXISTS
├── main.rs
├── server.rs          # ✓ EXISTS
├── session.rs
├── workspace/
└── version.rs
```

**Missing**: web/ subdirectory (no Web UI implementation)

### 8. ⚠️ Environment-Specific Hardcoded Paths

**Location**: Multiple files
**Issue**: Hardcoded paths to specific user directory

**Examples**:
- `/docs/howto.md` line 20: `cd /Users/pcastone/Projects/acolib`
- `/docs/running.md` line 7: `cd /Users/pcastone/Projects/acolib`
- `/docs/environment.md` line 133: `/Users/pcastone/Projects/rLangGraph/crates/`

**Impact**: Non-portable instructions, confusing for other users
**Fix**: Use relative paths or placeholder variables like `$PROJECT_ROOT`

### 9. ⚠️ Incorrect Library Count (running.md)

**Location**: `/docs/running.md` lines 11-12
**Issue**: States "3 dynamic libraries in `target/release/`"
**Reality**: Based on workspace, should be more libraries

**Crates with `[lib]` type**:
- langgraph-core
- langgraph-checkpoint
- langgraph-prebuilt
- orchestrator
- tooling
- llm
- utils

**Expected**: At least 7 dynamic libraries (depending on build configuration)

### 10. ⚠️ Missing LLM Provider Details (architecture.md)

**Location**: `/docs/architecture.md` lines 162-180
**Issue**: LLM section only lists OpenAI and Anthropic
**Reality**: LLM crate supports many more providers

**Documented**:
```
llm/
├── provider/
│   ├── openai.rs      # OpenAI GPT models
│   ├── anthropic.rs   # Claude models
│   └── generic.rs     # Generic HTTP provider
```

**Actual Additional Providers**:
- Gemini (Google)
- Grok (xAI)
- Deepseek
- OpenRouter
- Ollama (local)
- LM Studio (local)
- llama.cpp (local)

### 11. ⚠️ Incomplete Tooling Crate Documentation

**Location**: `/docs/architecture.md` lines 181-192
**Issue**: Minimal documentation for tooling crate
**Reality**: Tooling crate has extensive runtime SDK implementation (see `/docs/tools_runtime_sdk.md`)

**Missing Details**:
- Policy enforcement engine
- Tool runtime SDK
- Security validators
- AST operations
- Git operations
- Network policies

---

## Medium Priority Issues

### 12. ⚙️ Missing Orca Crate in Architecture Overview

**Location**: `/docs/architecture.md` Component Architecture section
**Issue**: No section for Orca crate despite being primary user-facing tool
**Reality**: Orca is the standalone orchestrator and main CLI

**Should Document**:
```
### Orca

**Standalone orchestrator with direct tool execution.**

orca/
├── bin/
│   └── orca.rs        # Main binary
├── config/            # Configuration
├── database/          # SQLite operations
├── cli/               # CLI commands
└── execution/         # Workflow execution
```

### 13. ⚙️ Utils Crate Not Documented

**Location**: `/docs/architecture.md` line 199-201
**Issue**: Utils crate mentioned but not documented
**Reality**: Utils crate exists with meaningful functionality

**Should Document**:
- Server and client configuration
- HTTP client with retry/backoff
- Environment variable handling
- Common utilities

### 14. ⚙️ Outdated API Base URL (endpoints.md vs api_specification.md)

**Location**: `/docs/endpoints.md` vs `/docs/api_specification.md`
**Issue**: Different base URLs documented

**endpoints.md**: `http://localhost:8080/api`
**api_specification.md**: `http://localhost:8080/api/v1`

**Impact**: Confusion about actual API paths
**Fix**: Standardize on single base URL format

### 15. ⚙️ Migration Count Not Updated

**Location**: Various references to database migrations
**Issue**: Docs don't specify number of migrations
**Reality**: 14 migration files exist in `src/crates/orchestrator/migrations/`

---

## Documentation Gaps (Missing Docs)

### Missing High-Value Documentation

1. **Orca User Guide** - No comprehensive guide for primary CLI tool
2. **Tool Runtime SDK Integration Guide** - How to use the extensive SDK documented in tools_runtime_sdk.md
3. **LLM Provider Configuration Guide** - How to configure each of the 10+ providers
4. **Testing Guide** - How to run tests, write tests, test structure
5. **Deployment Guide** - Production deployment instructions
6. **Troubleshooting Guide** - Common issues and solutions
7. **API Examples** - Working code examples for REST API usage
8. **WebSocket Protocol Examples** - How to use WebSocket streaming
9. **Migration Guide** - How to run database migrations
10. **Configuration Reference** - Complete reference for all config options

---

## Recommendations

### Immediate Actions (High Priority)

1. **Fix CLAUDE.md workspace contradiction** - Update to reflect actual workspace structure
2. **Update environment.md crate count** - Change from 6 to 10 crates with complete list
3. **Remove or create build scripts** - Either implement scripts or remove references
4. **Fix hardcoded paths** - Replace with relative paths or variables
5. **Create or remove CONTRIBUTING.md reference** - Add file or remove broken link

### Short-term Improvements

6. **Update architecture.md module structures** - Reflect actual file organization
7. **Document Orca crate** - Add comprehensive section for primary tool
8. **Expand LLM provider documentation** - Include all 10+ providers
9. **Document Utils crate** - Add architecture section
10. **Standardize API documentation** - Consolidate endpoints.md and api_specification.md

### Long-term Enhancements

11. **Create missing documentation** - Add guides for Orca, testing, deployment, etc.
12. **Add working examples** - Code examples for all major features
13. **Setup documentation CI** - Automated checks for broken links and outdated info
14. **Create visual diagrams** - Architecture diagrams, flow charts, sequence diagrams
15. **Add troubleshooting section** - Common issues and resolutions

---

## Positive Findings

### Well-Documented Areas

1. ✅ **Pregel execution model** - Excellent explanation in architecture.md and pregel/mod.rs
2. ✅ **Tool Runtime SDK** - Comprehensive specification in tools_runtime_sdk.md
3. ✅ **API endpoints** - Detailed REST API documentation in endpoints.md
4. ✅ **Message types** - Clear message flow documentation
5. ✅ **missing.md** - Good tracking of incomplete features

### Documentation Strengths

- Good use of code examples
- Clear table of contents
- Comprehensive API specifications
- Detailed architectural explanations
- Good coverage of design decisions

---

## Action Plan Priority Matrix

| Priority | Issue | Effort | Impact | Action |
|----------|-------|--------|--------|--------|
| P0 | Workspace contradiction | Low | High | Fix CLAUDE.md immediately |
| P0 | Crate count error | Low | High | Update environment.md |
| P1 | Missing scripts | High | High | Create scripts or update docs |
| P1 | Hardcoded paths | Low | Medium | Find/replace with variables |
| P1 | Module structure errors | Medium | Medium | Update architecture.md |
| P2 | Missing Orca docs | High | High | Create new section |
| P2 | LLM provider list | Low | Medium | Expand architecture.md |
| P3 | API URL inconsistency | Low | Low | Standardize base URL |
| P3 | Missing user guides | Very High | High | Create new docs (long-term) |

---

## Appendix: Files Analyzed

### Documentation Files
- /docs/architecture.md
- /docs/environment.md
- /docs/howto.md
- /docs/running.md
- /docs/BUILD.md
- /docs/endpoints.md
- /docs/api_specification.md
- /docs/tools_runtime_sdk.md
- /docs/missing.md
- /CLAUDE.md

### Codebase References
- /Cargo.toml (workspace)
- /src/crates/*/Cargo.toml (all 10 crates)
- /src/crates/langgraph-core/src/pregel/
- /src/crates/llm/src/
- /src/crates/aco/src/
- /src/crates/orchestrator/migrations/

---

**Next Steps**: Review this analysis, prioritize fixes, and begin updating documentation to match codebase reality.
