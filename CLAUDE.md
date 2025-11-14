# CLAUDE.md

## project_rules
- Please follow File System Layout for putting file placements.
- First think through the problem,
   - read the codebase for relevant files,
   - Look for existing script, function, or implementation and build on them before create new script, function and implementation.
   - write a plan to todo/tasks.md
 - The plan should have a list of todo items that you can check off as you complete them
 - Alway try to re-use, repurpose, and extend before create new.  
 - Before you begin working, check in with me and I will verify the plan.
 - Then, begin working on the todo items, marking them as complete as you go.
 - At every step of the way please give me a high level explanation of what changes you made
 - Make every task and code change you do as simple as possible. We want to avoid making any massive or complex changes. Every changeshould impact as little code as possible. Everything is about simplicity.
 - Test your changes after each task compile and resolve errors.
 - Do local git commit with summary of the changes you made any time you make a change.

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

acolib is a Rust-based platform for building and executing stateful AI agent workflows. It implements the Pregel execution model adapted for LLM-driven applications, providing a comprehensive framework for orchestrating AI agents with explicit state management, checkpoint-based resilience, and streaming-first architecture.

## Build System

This is NOT a standard Cargo workspace - there is no root `Cargo.toml`. All crates are located in `src/crates/`.

### Quick Build Commands

```bash
# Build Orca (standalone orchestrator) - most common use case
cd src/crates/orca
cargo build --release

# Or use the helper script
./scripts/build-orca.sh --install

# Build all crates
cd src/crates/orca  # or any crate directory
cargo build --release

# Fast check without building
cargo check

# Run tests
cargo test

# Run tests for specific crate
cd src/crates/<crate-name>
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Crate Structure

Each crate in `src/crates/` has its own `Cargo.toml` and must be built independently. To work with any crate, `cd` into its directory first.

## Architecture

### Core Components

1. **langgraph-core** - Graph execution engine implementing Pregel model
   - `builder.rs` - StateGraph construction API
   - `graph.rs` - Compiled graph execution
   - `pregel/` - Pregel superstep-based executor
   - `state.rs` - State definition and reducers
   - `messages.rs` - Message types (System, Human, Assistant, Tool)
   - Key concept: Superstep-based execution with barrier synchronization

2. **langgraph-checkpoint** - Persistence abstraction
   - Trait-based checkpoint saving
   - Channel types: LastValue, Append, Merge, Sum
   - SQLite and PostgreSQL backends available
   - In-memory implementation for testing

3. **langgraph-prebuilt** - Pre-built agent patterns
   - ReAct: Think → Act → Observe (default, 90% of use cases)
   - Plan-Execute: Plan → Execute → Replan (complex multi-step)
   - Reflection: Generate → Critique → Refine (quality-critical)

4. **orca** - Standalone orchestrator (primary CLI tool)
   - Single-process architecture, no server dependency
   - SQLite database at `~/.orca/orca.db`
   - Config: `~/.orca/orca.toml` (user) or `./.orca/orca.toml` (project)
   - Direct tool execution (in-process)

5. **orchestrator** - Distributed orchestration engine
   - Multi-machine capable
   - WebSocket-based communication
   - Task and workflow lifecycle management
   - Database migrations in `migrations/` using sqlx

6. **llm** - LLM provider integrations
   - Local: Ollama, llama.cpp, LM Studio
   - Remote: Claude (Anthropic), OpenAI, Gemini, Grok, Deepseek, OpenRouter
   - Thinking model support (o1, R1 series) with reasoning extraction
   - Unified `ChatModel` trait

7. **tooling** - Utilities for acolib
   - Configuration management
   - Logging setup
   - Common utilities

8. **utils** - Shared utilities
   - Server and client configuration
   - HTTP client with retry/backoff
   - Environment variable and config file loading
   - Authentication helpers

9. **aco** - Web UI, TUI, and CLI binaries (if present)

### Execution Model: Pregel (BSP)

The system uses Google's Pregel Bulk Synchronous Parallel model:

```
Loop while active nodes exist:
  Superstep S:
    1. Active nodes receive messages
    2. Nodes execute in parallel
    3. Nodes emit messages to next nodes
    4. Barrier synchronization
    5. Checkpoint created
    6. Stream events emitted
    7. Mark nodes for next superstep
```

**Key implications:**
- Within superstep: parallel execution
- Between supersteps: barrier sync, state reduction must complete
- Checkpoints created after each superstep
- Execution is deterministic and testable

### State Management

State uses reducer pattern:
- **AppendReducer**: `[...old] + [...new]` - for message history
- **OverwriteReducer**: `new` - for status fields
- **MergeReducer**: `deep_merge(old, new)` - for nested objects
- **SumReducer**: `old + new` - for counters

State fields must specify their reducer type. Multiple parallel nodes emit changes, then reducers merge them.

## Database

### Orchestrator Schema (SQLite)

6 main tables:
- **tasks** - Core task management (13 columns, 4 indexes, auto-timestamp trigger)
- **workflows** - Workflow definitions (7 columns, 3 indexes)
- **workflow_tasks** - M2M junction with CASCADE delete
- **tool_executions** - Audit log (11 columns, execution tracking)
- **sessions** - WebSocket connection tracking
- **configurations** - Key-value config store

Migrations located in `src/crates/orchestrator/migrations/` using sqlx format.

### Running Migrations

```bash
cd src/crates/orchestrator
export DATABASE_URL="sqlite:orchestrator.db"
sqlx database create
sqlx migrate run
```

## Testing

### Test Structure

Most crates have tests in:
- Unit tests: within `src/` files or `tests/` directory
- Integration tests: `tests/` directory
- Benchmarks: `benches/` directory (if present)

### Running Tests

```bash
# All tests for a crate
cd src/crates/<crate-name>
cargo test

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Integration tests only
cargo test --test '*'

# Unit tests only
cargo test --lib
```

## Configuration

### Orca Configuration

Location priority:
1. `./.orca/orca.toml` (project-level)
2. `~/.orca/orca.toml` (user-level)

Example structure:
```toml
[llm]
provider = "anthropic"  # or "openai", "ollama", etc.
model = "claude-3-sonnet"
api_key = "${ANTHROPIC_API_KEY}"  # env var expansion

[execution]
streaming = true  # Enable token streaming
```

### Environment Variables

All LLM providers support API keys via environment:
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GOOGLE_API_KEY`
- etc.

Use `utils::config` helpers for env var loading.

## Important Patterns

### 1. Async Runtime

All async code uses tokio:
```rust
#[tokio::main]
async fn main() { }

#[tokio::test]
async fn test_something() { }
```

### 2. Error Handling

- Use `thiserror` for library errors
- Use `anyhow` for application errors
- Each crate has `error.rs` with custom error types

### 3. Message Types

```rust
use langgraph_core::Message;

Message::system("You are a helpful assistant")
Message::human("What is the weather?")
Message::assistant("Let me check...")
Message::tool_call("get_weather", args)
Message::tool_result("get_weather", result)
```

### 4. Building Graphs

```rust
use langgraph_core::StateGraph;

let graph = StateGraph::new()
    .add_node("agent", agent_fn)
    .add_node("tools", tool_fn)
    .add_edge("__start__", "agent")
    .add_conditional_edges("agent", should_continue)
    .add_edge("tools", "agent")
    .compile()?;
```

### 5. LLM Integration

```rust
use llm::remote::OpenAiClient;
use llm::config::RemoteLlmConfig;
use langgraph_core::llm::{ChatModel, ChatRequest};

let config = RemoteLlmConfig::from_env("OPENAI_API_KEY",
    "https://api.openai.com/v1", "gpt-4");
let client = OpenAiClient::new(config);
let response = client.chat(request).await?;
```

## Common Tasks

### Adding a New LLM Provider

1. Create file in `src/crates/llm/src/local/` or `remote/`
2. Implement `ChatModel` trait
3. Add to module exports in `mod.rs`
4. Update README with usage example
5. Add tests

### Adding a New Agent Pattern

1. Create pattern in `src/crates/langgraph-prebuilt/src/agents/`
2. Use `StateGraph` builder
3. Define state schema and reducers
4. Add builder in `src/crates/langgraph-prebuilt/src/builders/`
5. Export from `lib.rs`
6. Add documentation and tests

### Adding Database Tables

1. Create migration files in `src/crates/orchestrator/migrations/`
2. Format: `YYYYMMDDHHMMSS_description.up.sql` and `.down.sql`
3. Use `IF NOT EXISTS` for idempotency
4. Add proper indexes and constraints
5. Test both UP and DOWN migrations

### Working with Checkpoints

Checkpoints are created automatically after each superstep. To use:
```rust
use langgraph_checkpoint::CheckpointSaver;

// Resume from checkpoint
let config = CheckpointConfig::new()
    .thread_id(thread_id)
    .checkpoint_id(checkpoint_id);
graph.stream(input, config).await?;
```

## Documentation

Key docs in `docs/`:
- `architecture.md` - High-level system design
- `environment.md` - Project structure and workspace layout
- `howto.md` - Complete build and setup instructions
- `running.md` - Quick start guide
- `BUILD.md` - Comprehensive build documentation
- `endpoints.md` - REST API reference

Per-crate docs in `src/crates/<crate>/README.md`.

## Development Workflow

1. Navigate to specific crate: `cd src/crates/<crate-name>`
2. Make changes
3. Quick check: `cargo check`
4. Run tests: `cargo test`
5. Format: `cargo fmt`
6. Lint: `cargo clippy`
7. Build: `cargo build --release`

## Known Constraints

- No root workspace `Cargo.toml` - each crate is independent
- Must `cd` into crate directory before running cargo commands
- Orca is the primary user-facing tool (standalone, simple)
- Orchestrator is for distributed/production use (complex)
- SQLite used for both (Orca and Orchestrator)
- Database migrations use sqlx format
- All crates target Rust edition 2021, version 1.75.0+
