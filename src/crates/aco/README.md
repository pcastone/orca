# Aco - Agent Client for Orchestrator

A client application for connecting to the orchestrator-server, executing tools, and sending prompts to LLMs.

## Overview

Aco serves as the client component of the orchestrator ecosystem. It can:
- Connect to an orchestrator-server for distributed task execution
- Run as a tool server with filesystem, git, and shell tools
- Send prompts directly to LLMs via the orchestrator's API
- Provide a TUI interface for interactive use

## Installation

```bash
cargo install aco
```

Or build from source:

```bash
cd crates/aco
cargo build --release
```

## Quick Start

1. Initialize aco configuration:
   ```bash
   aco init
   ```

2. Send a prompt via orchestrator:
   ```bash
   aco prompt "What is the capital of France?" --server http://localhost:8080
   ```

3. Or start as a tool server:
   ```bash
   aco server
   ```

## Commands

### `aco init`

Initialize aco configuration for a project. Creates config files in:
- User-level: `~/.aco/aco.toml`
- Project-level: `./.aco/aco.toml`

### `aco config`

Display current configuration including merged user and project settings.

### `aco connect <URL>`

Connect to an orchestrator server for task coordination.

```bash
aco connect http://localhost:8080
```

### `aco status`

Show current connection status to the orchestrator.

### `aco prompt <PROMPT>`

Send a prompt to the LLM via orchestrator-server.

```bash
# With explicit server URL
aco prompt "Explain quantum computing" --server http://localhost:8080

# Using ORCHESTRATOR_URL environment variable
export ORCHESTRATOR_URL=http://localhost:8080
aco prompt "Write a haiku about Rust"

# Using default from config
aco prompt "What is 2+2?"
```

**Options:**
- `-s, --server <URL>` - Orchestrator server URL

**Note:** Requires orchestrator-server to be running with LLM configured.

### `aco server`

Run aco as a tool server (default behavior).

```bash
# Basic server
aco server

# With workspace directory
aco server --workspace /path/to/project

# With custom address
aco server --address 127.0.0.1:9000

# Enable TUI mode
aco server --tui
```

**Options:**
- `-w, --workspace <PATH>` - Workspace root directory (default: `.`)
- `-a, --address <ADDR>` - Server address (overrides config)
- `--tui` - Enable TUI mode

## Available Tools

When running as a server, aco registers these tools:

### Filesystem Tools
- `file_read` - Read file contents
- `file_write` - Write to files
- `fs_list` - List directory contents
- `fs_copy` - Copy files/directories
- `fs_move` - Move files/directories
- `fs_delete` - Delete files/directories
- `file_patch` - Patch files with diffs
- `grep` - Search file contents

### Git Tools
- `git_status` - Show repository status
- `git_diff` - Show changes
- `git_add` - Stage files
- `git_commit` - Create commits

### Shell Tools
- `shell_exec` - Execute shell commands

## Configuration

Configuration is loaded from (in order of precedence):
1. Command-line arguments
2. Environment variables
3. Project config (`./.aco/aco.toml`)
4. User config (`~/.aco/aco.toml`)

### Example Configuration

```toml
[server]
host = "127.0.0.1"
port = 8765

[client]
orchestrator_url = "http://localhost:8080"

[ui]
enable_tui = false
log_level = "info"
```

### Environment Variables

- `ORCHESTRATOR_URL` - Default orchestrator server URL
- `RUST_LOG` - Log level (trace, debug, info, warn, error)

## Architecture

```
┌─────────────────────────────────┐
│           Aco Client            │
│                                 │
│  ┌──────────┐    ┌───────────┐  │
│  │  Prompt  │    │   Tools   │  │
│  │  Client  │    │  Server   │  │
│  └────┬─────┘    └─────┬─────┘  │
│       │                │        │
└───────┼────────────────┼────────┘
        │                │
        ▼                ▼
┌───────────────┐  ┌────────────┐
│ Orchestrator  │  │  External  │
│    Server     │  │   Clients  │
└───────────────┘  └────────────┘
```

## Usage with Orchestrator

1. Start orchestrator-server:
   ```bash
   ./target/release/orchestrator-server
   ```

2. Connect aco:
   ```bash
   aco connect http://localhost:8080
   ```

3. Send prompts:
   ```bash
   aco prompt "Analyze the codebase structure"
   ```

## Verbose Mode

Enable detailed logging with `-v`:

```bash
aco -v prompt "Hello"
aco -v server
```

## License

MIT OR Apache-2.0
