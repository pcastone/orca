# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Rules

- Build programs with scripts as much as possible; only use direct cargo commands for debugging/troubleshooting
- First think through the problem:
  - Read the codebase for relevant files
  - Look for existing scripts, functions, or implementations and build on them before creating new ones
  - Write a plan with todo items that can be checked off as completed
- Always try to re-use, repurpose, and extend before creating new code
- Before you begin working, check in with me to verify the plan
- Make every task and code change as simple as possible
- Every change should impact as little code as possible; simplicity is paramount
- Test your changes after each task and compile to resolve errors

# Goal 
- current goal build orca to support send a prompt to LLM.  

## Build Commands

```bash
# Primary build - creates release with all binaries
./scripts/build-release.sh

# Quick builds for specific crates
cargo build -p orca --release
cargo build -p aco --release
cargo build -p orchestrator --release

# Fast check without building
cargo check

# Run tests
cargo test                    # all workspace
cargo test -p orca           # specific crate
cargo test test_name         # specific test
cargo test -- --nocapture    # with output

# Code quality
cargo fmt                    # format all
cargo clippy --all           # lint all
```

## Architecture Overview

acolib is a Rust platform for building stateful AI agent workflows using the Pregel (BSP) execution model.

### Workspace Crates (src/crates/)

**Core Execution:**
- **langgraph-core** - Graph execution engine (Pregel model, StateGraph builder, state reducers)
- **langgraph-checkpoint** - Persistence (SQLite/PostgreSQL backends, channel types)
- **langgraph-prebuilt** - Agent patterns (ReAct, Plan-Execute, Reflection)

**Primary Tools:**
- **orca** - Standalone orchestrator CLI (most common use case)
- **orchestrator** - Distributed orchestration server (orchestrator-server binary)
- **aco** - Client with TUI/CLI for remote tool execution

**Support:**
- **llm** - LLM providers (Anthropic, OpenAI, Gemini, Ollama, Deepseek, etc.)
- **tooling** - Configuration, logging utilities
- **utils** - Shared utilities, HTTP client, config loading
- **rtoon** - TOON format serialization
- **langgraph-cli** - Development CLI tools

### Execution Model: Pregel

```
Loop while active nodes exist:
  Superstep S:
    1. Active nodes receive messages
    2. Nodes execute in parallel
    3. Nodes emit messages
    4. Barrier synchronization
    5. Checkpoint created
    6. Stream events emitted
```

### State Reducers

- **AppendReducer** - `[...old] + [...new]` for message history
- **OverwriteReducer** - `new` for status fields
- **MergeReducer** - `deep_merge(old, new)` for nested objects
- **SumReducer** - `old + new` for counters

## Configuration

### Orca Config Locations (priority order)
1. `./.orca/orca.toml` (project)
2. `~/.orca/orca.toml` (user)

### LLM Provider Environment Variables
- `ANTHROPIC_API_KEY`
- `OPENAI_API_KEY`
- `GOOGLE_API_KEY`
- `DEEPSEEK_API_KEY`

## Important Patterns

### Async Runtime
All async code uses tokio:
```rust
#[tokio::main]
async fn main() { }
```

### Error Handling
- `thiserror` for library errors (each crate has `error.rs`)
- `anyhow` for application errors

### Message Types
```rust
use langgraph_core::Message;
Message::system("...")
Message::human("...")
Message::assistant("...")
Message::tool_call("name", args)
Message::tool_result("name", result)
```

### Building Graphs
```rust
let graph = StateGraph::new()
    .add_node("agent", agent_fn)
    .add_node("tools", tool_fn)
    .add_edge("__start__", "agent")
    .add_conditional_edges("agent", should_continue)
    .compile()?;
```

## Database

Orchestrator uses SQLite with sqlx migrations in `src/crates/orchestrator/migrations/`.

```bash
cd src/crates/orchestrator
export DATABASE_URL="sqlite:orchestrator.db"
sqlx database create
sqlx migrate run
```

## Release Process

The build script creates releases in `release/`:
- `release/build_*/` - Build directories (keeps last 3)
- `release/dist/` - Tarballs (keeps last 3)
- `release/lastbuild` - Symlink to latest

Binaries produced: `orca`, `aco`, `orchestrator-server`

## Orca vs Orchestrator

| Feature | Orca | Orchestrator |
|---------|------|--------------|
| Architecture | Standalone | Distributed |
| Tool Execution | In-process | Remote (WebSocket) |
| Use Case | Local dev | Production |

Start with Orca; migrate to Orchestrator for distributed deployments.
