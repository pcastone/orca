# Environment and Project Structure

## Overview
acolib is a standard Cargo workspace containing 10 member crates that provide a comprehensive AI agent orchestration platform. The workspace includes core execution engines, pre-built patterns, standalone tools, and provider integrations.

## File System Layout
```
acolib/
├── README.md                # Project overview and quick start
├── CLAUDE.md                # Claude Code assistant instructions
├── Cargo.toml               # Workspace configuration
├── .gitignore               # Git ignore rules
├── docs/                    # Documentation directory
│   ├── architecture.md      # System design and components
│   ├── BUILD.md             # Build instructions
│   ├── howto.md             # Setup and build guide
│   ├── running.md           # Quick start guide
│   ├── environment.md       # This file - project structure
│   ├── endpoints.md         # REST API reference
│   ├── project_prd.md       # Project requirements
│   └── status/              # Phase completion tracking
├── todo/                    # Task tracking
│   ├── tasks.md             # Current tasks
│   └── bugfix.md            # Bug tracking
├── logs/                    # Development logs
├── scripts/                 # Build and automation scripts
│   ├── build-orca.sh        # Quick Orca build script
│   └── build-dist.sh        # Full distribution build
├── src/crates/              # All workspace crates
│   ├── orca/                # Standalone orchestrator (⭐ primary tool)
│   ├── aco/                 # Client application with TUI
│   ├── orchestrator/        # Distributed orchestration engine
│   ├── langgraph-core/      # Core graph execution
│   ├── langgraph-checkpoint/ # Persistence layer
│   ├── langgraph-prebuilt/  # Agent patterns
│   ├── langgraph-cli/       # Development CLI tools
│   ├── llm/                 # LLM provider integrations
│   ├── tooling/             # Configuration and utilities
│   └── utils/               # Shared utilities
├── workflows/               # Example workflows
├── playground/              # Experimentation area
└── target/                  # Rust build directory (gitignored)
```

## Architecture

### Workspace Structure
This is a standard Cargo workspace with 10 member crates (all version 0.1.0):

#### Core Execution Engine

1. **langgraph-core**
   - Type: Library (cdylib + rlib)
   - Description: Core graph execution engine implementing Pregel model
   - Dependencies: langgraph-checkpoint
   - Output: `liblanggraph_core.dylib/.so/.dll`
   - Key modules: builder.rs, graph.rs, pregel/, state.rs, messages.rs

2. **langgraph-checkpoint**
   - Type: Library (cdylib + rlib)
   - Description: Checkpoint trait abstractions with SQLite/PostgreSQL backends
   - Output: `liblanggraph_checkpoint.dylib/.so/.dll`
   - Backends: Memory, SQLite, PostgreSQL

3. **langgraph-prebuilt**
   - Type: Library (cdylib + rlib)
   - Description: Pre-built agent patterns (ReAct, Plan-Execute, Reflection)
   - Dependencies: langgraph-core, langgraph-checkpoint
   - Output: `liblanggraph_prebuilt.dylib/.so/.dll`

4. **langgraph-cli**
   - Type: Binary
   - Description: Command-line development tools for langgraph
   - Dependencies: langgraph crates
   - Output: `langgraph` binary

#### Orchestration & Tools

5. **orca**
   - Type: Binary
   - Description: **Standalone orchestrator for local development (primary user tool)**
   - Features:
     - Direct tool execution (in-process)
     - SQLite database at `~/.orca/orca.db`
     - Config: `~/.orca/orca.toml` or `./.orca/orca.toml`
     - No server dependency
   - Output: `orca` binary
   - Use case: Local development, single-machine deployments

6. **orchestrator**
   - Type: Library (cdylib + rlib) + Binary
   - Description: Distributed orchestration engine for production
   - Features:
     - WebSocket-based communication
     - Multi-machine capable
     - Task and workflow lifecycle management
     - Database migrations with sqlx
   - Output: `liborchestrator.dylib/.so/.dll`, `orchestrator-server` binary
   - Use case: Production, distributed deployments

7. **aco**
   - Type: Binary
   - Description: Client application for tool execution with TUI and CLI
   - Features:
     - Terminal UI (ratatui)
     - CLI interface
     - WebSocket client for orchestrator
   - Output: `aco` binary

#### Integration & Utilities

8. **llm**
   - Type: Library (cdylib + rlib)
   - Description: LLM provider integrations
   - Features:
     - **Local**: Ollama, llama.cpp, LM Studio
     - **Remote**: Claude, OpenAI, Gemini, Grok, Deepseek, OpenRouter
     - Thinking model support (o1, R1 series)
     - Unified `ChatModel` trait
   - Output: `libllm.dylib/.so/.dll`

9. **tooling**
   - Type: Library (cdylib + rlib)
   - Description: Configuration management, logging, and utilities
   - Features: Config loading, logging setup, common utilities
   - Output: `libtooling.dylib/.so/.dll`

10. **utils**
   - Type: Library (cdylib + rlib)
   - Description: Shared utilities across workspace
   - Features: HTTP client, config loading, authentication helpers
   - Output: `libutils.dylib/.so/.dll`

### Library Configuration
Library crates are configured with:
```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

This produces:
- **cdylib**: C-compatible dynamic library
- **rlib**: Rust library for internal workspace dependencies

### Platform-Specific Libraries
- **macOS**: `.dylib` files
- **Linux**: `.so` files
- **Windows**: `.dll` files

## Dependencies

### Core Dependencies
- **tokio** (1.40): Async runtime with full features
- **async-trait** (0.1): Async trait support
- **futures** (0.3): Futures utilities
- **serde** (1.0): Serialization framework
- **serde_json** (1.0): JSON support
- **thiserror** (1.0): Error handling
- **anyhow** (1.0): Error context
- **uuid** (1.10): UUID generation
- **chrono** (0.4): Date/time handling
- **tracing** (0.1): Logging
- **clap** (4.5): CLI argument parsing
- **reqwest** (0.12): HTTP client

### Build Configuration
- **Rust version**: 1.75.0+
- **Edition**: 2021
- **Resolver**: Version 2

### Release Profile
```toml
[profile.release]
opt-level = 3           # Maximum optimization
lto = true             # Link-time optimization
codegen-units = 1      # Single codegen unit for better optimization
```

## Build Artifacts

### Binaries (in target/release/)
- `orca` - Standalone orchestrator (primary tool)
- `aco` - Client application with TUI
- `orchestrator-server` - Distributed orchestrator server
- `langgraph` - Development CLI

### Dynamic Libraries (in target/release/)
- `liblanggraph_core.{dylib,so,dll}`
- `liblanggraph_checkpoint.{dylib,so,dll}`
- `liblanggraph_prebuilt.{dylib,so,dll}`
- `libllm.{dylib,so,dll}`
- `libtooling.{dylib,so,dll}`
- `liborchestrator.{dylib,so,dll}`
- `libutils.{dylib,so,dll}`

## Workspace Features

### Standard Workspace Benefits
- Shared dependencies via `[workspace.dependencies]`
- Unified versioning (all crates at v0.1.0)
- Single `target/` directory for build artifacts
- Coordinated releases
- Cross-crate refactoring support

### Build Commands
```bash
# Build all workspace crates
cargo build --release

# Build specific crate
cargo build -p orca --release

# Test all crates
cargo test

# Test specific crate
cargo test -p langgraph-core
```

## Development Environment

### Recommended Tools
- **rust-analyzer**: IDE support
- **cargo-watch**: Auto-rebuild on changes
- **cargo-nextest**: Faster test runner

### Environment Variables
LLM provider API keys:
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GOOGLE_API_KEY`
- `DEEPSEEK_API_KEY`
- `XAI_API_KEY`

### Configuration Locations
- **Orca**: `~/.orca/orca.toml` (user) or `./.orca/orca.toml` (project)
- **Logs**: `logs/` (development) or configured path

## Architecture Highlights

### Execution Model
- **Pregel BSP**: Bulk Synchronous Parallel model from Google
- **Superstep-based**: Clear execution boundaries
- **Checkpointing**: Automatic state persistence after each step
- **Streaming**: Real-time event emission

### State Management
- **Reducer Pattern**: AppendReducer, OverwriteReducer, MergeReducer, SumReducer
- **Explicit State**: All changes trackable
- **Type Safety**: Rust's type system prevents errors

### Agent Patterns
- **ReAct**: Think → Act → Observe (default, 90% of use cases)
- **Plan-Execute**: Plan → Execute → Replan (complex tasks)
- **Reflection**: Generate → Critique → Refine (quality-critical)

---

**For detailed build instructions, see [BUILD.md](BUILD.md)**
**For quick start guide, see [running.md](running.md)**
**For architecture details, see [architecture.md](architecture.md)**
