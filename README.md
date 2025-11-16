# acolib - AI Agent Orchestration Library

A comprehensive Rust-based platform for building and executing stateful AI agent workflows. acolib implements the Pregel execution model adapted for LLM-driven applications, providing a robust framework for orchestrating AI agents with explicit state management, checkpoint-based resilience, and streaming-first architecture.

## Overview

acolib enables developers to build sophisticated AI agent systems with:

- **Explicit State Management** - All state changes are explicit and trackable
- **Checkpoint-Based Resilience** - Automatic persistence at each execution step
- **Streaming-First** - Real-time event emission during execution
- **Type Safety** - Rust's type system prevents entire classes of errors
- **Multiple Agent Patterns** - ReAct, Plan-Execute, Reflection, and custom patterns
- **LLM Provider Flexibility** - Support for OpenAI, Claude, Gemini, Ollama, and more

## Quick Start

### Installation

Build from source:

```bash
# Build Orca (standalone orchestrator)
cargo build -p orca --release

# Or use the helper script
./scripts/build-orca.sh --install
```

### First Steps

```bash
# Initialize Orca
./target/release/orca init

# Configure your LLM provider in ~/.orca/orca.toml
# Example:
# [llm]
# provider = "anthropic"
# model = "claude-3-sonnet"
# api_key = "${ANTHROPIC_API_KEY}"

# Create and run a task
./target/release/orca task create "Analyze project structure"
./target/release/orca task run <task-id>
```

## Architecture

acolib is organized as a Cargo workspace with 10 specialized crates:

### Core Execution Engine

- **langgraph-core** - Graph execution engine implementing Pregel model
- **langgraph-checkpoint** - Persistence abstraction with SQLite/PostgreSQL backends
- **langgraph-prebuilt** - Pre-built agent patterns (ReAct, Plan-Execute, Reflection)

### Orchestration & Tools

- **orca** - Standalone orchestrator for local development (primary CLI tool)
- **orchestrator** - Distributed orchestration engine for production deployments
- **aco** - Client application for tool execution with TUI and CLI interfaces

### Integration & Utilities

- **llm** - LLM provider integrations (OpenAI, Claude, Gemini, Ollama, etc.)
- **tooling** - Configuration management, logging, and utilities
- **utils** - Shared utilities across the workspace
- **langgraph-cli** - Command-line development tools

## Key Features

### Pregel Execution Model

acolib uses Google's Pregel Bulk Synchronous Parallel (BSP) model:

```
Loop while active nodes exist:
  Superstep S:
    1. Active nodes receive messages
    2. Nodes execute in parallel
    3. Nodes emit messages to next nodes
    4. Barrier synchronization
    5. Checkpoint created
    6. Stream events emitted
```

This provides:
- Clear execution boundaries
- Natural checkpoint points
- Deterministic, testable execution
- Built-in streaming support

### Agent Patterns

#### ReAct (Default - 90% of use cases)
Think → Act → Observe pattern for general tasks
- Low latency, token efficient
- Ideal for Q&A, tool-using assistants

#### Plan-Execute
Plan → Execute → Replan for complex multi-step tasks
- Explicit planning phase
- Can recover from failures

#### Reflection
Generate → Critique → Refine for quality-critical output
- Self-critique and improvement
- Ideal for code generation, technical writing

### LLM Provider Support

**Local Providers:**
- Ollama (default: `http://localhost:11434`)
- llama.cpp (`http://localhost:8080`)
- LM Studio (`http://localhost:1234/v1`)

**Remote Providers:**
- Anthropic Claude (Claude 3 Opus, Sonnet, Haiku, 3.5 Sonnet)
- OpenAI (GPT-4, GPT-4 Turbo, o1 series)
- Google Gemini (Pro, Pro Vision, 1.5 Pro/Flash)
- xAI Grok
- Deepseek (Chat, Coder, R1 reasoning model)
- OpenRouter (unified API for multiple providers)

## Project Structure

```
acolib/
├── README.md                    # This file
├── CLAUDE.md                    # Claude Code assistant instructions
├── Cargo.toml                   # Workspace configuration
├── docs/                        # Documentation
│   ├── architecture.md          # System design and components
│   ├── BUILD.md                 # Build instructions
│   ├── howto.md                 # Setup and build guide
│   ├── running.md               # Quick start guide
│   ├── environment.md           # Project structure details
│   ├── endpoints.md             # REST API reference
│   └── status/                  # Phase completion tracking
├── src/crates/                  # All workspace crates
│   ├── orca/                    # Standalone orchestrator (⭐ primary tool)
│   ├── aco/                     # Client application with TUI
│   ├── orchestrator/            # Distributed orchestration engine
│   ├── langgraph-core/          # Core graph execution
│   ├── langgraph-checkpoint/    # Persistence layer
│   ├── langgraph-prebuilt/      # Agent patterns
│   ├── langgraph-cli/           # Development CLI
│   ├── llm/                     # LLM provider integrations
│   ├── tooling/                 # Configuration and utilities
│   └── utils/                   # Shared utilities
├── scripts/                     # Build and automation scripts
│   ├── build-orca.sh            # Quick Orca build script
│   └── build-dist.sh            # Full distribution build
├── workflows/                   # Example workflows
├── playground/                  # Experimentation area
└── target/                      # Build artifacts (gitignored)
```

## Building

### Build Orca (Recommended for Most Users)

```bash
# Quick build
./scripts/build-orca.sh

# Build with tests
./scripts/build-orca.sh --test

# Build and install to ~/.cargo/bin
./scripts/build-orca.sh --install
```

### Build All Crates

```bash
# Release build (optimized)
cargo build --release

# Debug build (faster compilation)
cargo build

# Build specific crate
cargo build -p langgraph-core --release
```

### Testing

```bash
# Run all tests
cargo test

# Test specific crate
cargo test -p orca

# With output
cargo test -- --nocapture
```

### Code Quality

```bash
# Fast syntax check
cargo check

# Linting
cargo clippy --all

# Format code
cargo fmt --all
```

## Documentation

- **[Architecture](docs/architecture.md)** - System design and components
- **[Build Guide](docs/BUILD.md)** - Comprehensive build instructions
- **[How To](docs/howto.md)** - Detailed setup and build guide
- **[Quick Start](docs/running.md)** - Fast path to running the project
- **[Environment](docs/environment.md)** - Project structure details
- **[API Endpoints](docs/endpoints.md)** - REST API reference
- **[Orca README](src/crates/orca/README.md)** - Standalone orchestrator docs
- **[LLM README](src/crates/llm/README.md)** - LLM provider documentation
- **[ACO TUI Guide](src/crates/aco/TUI_GUIDE.md)** - Terminal UI documentation

## Configuration

### Orca Configuration

Configuration locations (in priority order):
1. `./.orca/orca.toml` (project-level)
2. `~/.orca/orca.toml` (user-level)

Example configuration:

```toml
[llm]
provider = "anthropic"          # or "openai", "ollama", etc.
model = "claude-3-sonnet"
api_key = "${ANTHROPIC_API_KEY}" # supports env var expansion

[execution]
streaming = true                 # Enable token streaming
```

### Environment Variables

LLM provider API keys can be set via environment variables:
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GOOGLE_API_KEY`
- `DEEPSEEK_API_KEY`
- `XAI_API_KEY`

## Comparison: Orca vs Orchestrator

| Feature | Orca | Orchestrator |
|---------|------|--------------|
| Architecture | Standalone | Distributed |
| Tool Execution | Direct (in-process) | Remote (via aco server) |
| Database | SQLite (`~/.orca/`) | SQLite or PostgreSQL |
| Network | None | WebSocket |
| Use Case | Local development, single-machine | Production, multi-machine |
| Complexity | Low | Higher |
| Setup | Simple (one binary) | Requires server setup |

**Recommendation:** Start with Orca for development and simple deployments. Migrate to Orchestrator for production or distributed scenarios.

## Requirements

- **Rust**: 1.75.0 or later
- **Cargo**: Latest stable
- **OS**: macOS, Linux, or Windows
- **Memory**: 4GB+ RAM for building
- **Disk**: 2GB+ free space

## License

MIT OR Apache-2.0

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## Support

- Documentation: See `docs/` directory
- Issues: File on GitHub repository
- Development: See `CLAUDE.md` for AI assistant instructions

---

**Built with Rust 🦀 for reliable, high-performance AI agent orchestration.**
