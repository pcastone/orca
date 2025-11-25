# Orca - Standalone Orchestrator

A simplified, standalone orchestrator for AI agent workflows that operates without requiring a separate aco server. Orca uses direct tool execution for a streamlined, single-process architecture.

## Overview

Orca is designed for local development and single-machine deployments where simplicity is preferred over distributed architecture. It provides the full power of the acolib orchestration system without the complexity of WebSocket servers and network communication.

## Features

- **Standalone Operation** - No server dependency, runs as a single process
- **Direct Tool Execution** - Tools execute in-process without network overhead
- **SQLite Database** - Persistent state stored in `~/.orca/orca.db`
- **Dual-Location Config** - User-level (`~/.orca/orca.toml`) and project-level (`./.orca/orca.toml`)
- **Full CLI Interface** - Comprehensive command-line tools for task and workflow management
- **LLM Integration** - Support for OpenAI, Claude, Gemini, Ollama, and more
- **Agent Patterns** - ReAct, Plan-Execute, Reflection, and other patterns from langgraph-prebuilt

## Architecture

```
┌─────────────────┐
│      Orca       │
│   Standalone    │
│                 │
│  ┌───────────┐  │
│  │ Patterns  │  │
│  └─────┬─────┘  │
│        │        │
│  ┌─────▼──────┐ │
│  │ DirectTool │ │
│  │  Bridge    │ │
│  └─────┬──────┘ │
│        │        │
│  ┌─────▼──────┐ │
│  │   Tools    │ │
│  │  (direct)  │ │
│  └────────────┘ │
│                 │
│  ┌───────────┐  │
│  │~/.orca/   │  │
│  │  orca.db  │  │
│  └───────────┘  │
└─────────────────┘
```

## Installation

```bash
cargo install orca
```

Or build from source:

```bash
cd crates/orca
cargo build --release
```

## Quick Start

1. Initialize orca:
   ```bash
   orca init
   ```

2. Configure your LLM provider in `~/.orca/orca.toml`:
   ```toml
   [llm]
   provider = "anthropic"
   model = "claude-3-sonnet"
   api_key = "${ANTHROPIC_API_KEY}"
   ```

3. Send a quick prompt:
   ```bash
   orca -p "What is the capital of France?"
   ```

4. Or create and run a task:
   ```bash
   orca task create "Analyze project structure"
   orca task run <task-id>
   ```

## Quick Prompts

Send prompts directly to your configured LLM without creating tasks:

```bash
# Simple prompt
orca -p "Explain quantum computing in simple terms"

# With Ollama (local, no API key needed)
orca -p "Write a haiku about programming"
```

The `-p` flag sends the prompt to your configured LLM and prints the response directly to stdout.

## Interactive TUI

Launch the interactive terminal user interface:

```bash
orca
```

The TUI provides:
- **Multi-panel layout** - Conversation, prompts, and sidebar views
- **Real-time LLM interaction** - Type prompts and see responses immediately
- **LLM Configuration** - Press `Ctrl+L` to configure LLM provider settings
- **Keyboard navigation** - Tab between panels, arrow keys to navigate
- **History tracking** - View conversation history and task outputs

### TUI Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Switch between panels |
| `Enter` | Send prompt / Select item |
| `Ctrl+L` | Open LLM configuration |
| `Esc` / `q` | Quit |
| `Up/Down` | Navigate lists |

### Configuring LLM in TUI

1. Press `Ctrl+L` to open the LLM configuration form
2. Fill in provider, model, API key, etc.
3. Press `Enter` to save (saves to SQLite database)
4. Configuration is shared with orchestrator-server

## Agent Patterns

Orca supports three agent patterns from langgraph-prebuilt:

### ReAct (Default) - Best for Most Tasks (90%)

The ReAct pattern alternates between reasoning and acting, making it ideal for:
- General Q&A and conversational tasks
- Tool-using assistants
- Tasks requiring quick responses
- Simple to moderate complexity

**Advantages**: Low latency, token efficient, reliable

**Example**:
```bash
# Uses ReAct by default
orca task create "List all Python files in src/"
```

### Plan-Execute - For Complex Multi-Step Tasks

Creates an explicit plan before execution, ideal for:
- Multi-step research and analysis
- Complex problem decomposition
- Tasks requiring upfront planning

**Advantages**: Clear execution steps, can replan on failures

**Example**:
```bash
# Create task with plan_execute pattern
orca task create "Analyze codebase and identify test coverage gaps"
# Then set metadata: {"pattern": "plan_execute"}
```

### Reflection - For Quality-Critical Output

Generate → critique → refine loop for high-quality results:
- Code generation and refactoring
- Technical writing
- Output requiring multiple iterations

**Advantages**: Self-critique, iterative improvement, quality threshold support

**Example**:
```bash
# Create task with reflection pattern
orca task create "Write production-ready sorting function with tests"
# Then set metadata: {"pattern": "reflection"}
```

### Setting Patterns

Patterns are selected via task metadata:

```json
{"pattern": "react"}        // Default
{"pattern": "plan_execute"} // For complex tasks
{"pattern": "reflection"}   // For quality-critical output
```

## Streaming Output

Enable real-time streaming of LLM responses:

```toml
[execution]
streaming = true  # Enable token-by-token streaming
```

When enabled:
- See LLM responses in real-time (token streaming)
- Progress indicators for long-running tasks
- Better visibility into agent reasoning

## CLI Commands

See [CLI documentation](docs/cli.md) for complete command reference.

## Configuration

See [Configuration guide](docs/configuration.md) for detailed setup.

## Comparison with Orchestrator

| Feature | Orca | Orchestrator |
|---------|------|--------------|
| Architecture | Standalone | Distributed |
| Tool Execution | Direct (in-process) | Remote (via aco server) |
| Database | SQLite (~/.orca/) | SQLite (configurable) |
| Network | None | WebSocket |
| Use Case | Local development | Production, multi-machine |
| Complexity | Low | Higher |

## License

MIT OR Apache-2.0
