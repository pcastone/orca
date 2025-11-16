# Quick Start: Running the Project

## Quick Build and Run

### Build Orca (Recommended)

The fastest way to get started with acolib:

```bash
# Build Orca (standalone orchestrator)
cargo build -p orca --release

# Or use the helper script
./scripts/build-orca.sh
```

This creates the `orca` binary in `target/release/`.

### Verify Build

```bash
# Check the binary
ls -lh target/release/orca

# Test it works
./target/release/orca --version
```

Expected output:
```
orca 0.1.0
```

## First Run

### Initialize Orca

```bash
# Create configuration directory and default config
./target/release/orca init
```

This creates:
- `~/.orca/` directory
- `~/.orca/orca.db` (SQLite database)
- `~/.orca/orca.toml` (configuration file)

### Configure LLM Provider

Edit `~/.orca/orca.toml`:

```toml
[llm]
provider = "anthropic"          # or "openai", "ollama", etc.
model = "claude-3-sonnet"       # or your preferred model
api_key = "${ANTHROPIC_API_KEY}" # reads from environment

[execution]
streaming = true                 # Enable real-time token streaming
```

Or set environment variable:
```bash
export ANTHROPIC_API_KEY="your-api-key-here"
```

### Create and Run Your First Task

```bash
# Create a task
./target/release/orca task create "List all files in the current directory"

# This returns a task ID, e.g., task_abc123

# Run the task
./target/release/orca task run task_abc123

# Or run immediately
./target/release/orca task create "What is 2+2?" --run
```

## Building All Workspace Crates

### Full Workspace Build

```bash
# Build everything in release mode
cargo build --release
```

This builds all 10 crates:
- **Binaries**: orca, aco, orchestrator-server, langgraph
- **Libraries**: langgraph-core, langgraph-checkpoint, langgraph-prebuilt, llm, tooling, utils, orchestrator

### Verify All Builds

```bash
# Check binaries
ls -lh target/release/orca
ls -lh target/release/aco
ls -lh target/release/langgraph

# Check dynamic libraries (platform-specific extensions)
ls -lh target/release/*.dylib  # macOS
ls -lh target/release/*.so     # Linux
ls -lh target/release/*.dll    # Windows
```

## Running Tests

### Quick Test

```bash
# Run all tests
cargo test
```

### Test Specific Crate

```bash
# Test Orca
cargo test -p orca

# Test langgraph-core
cargo test -p langgraph-core

# Test with output
cargo test -p orca -- --nocapture
```

## Using the CLI Tools

### Orca (Standalone Orchestrator)

Primary tool for local development:

```bash
# Show help
./target/release/orca --help

# Initialize
./target/release/orca init

# Create task
./target/release/orca task create "Your task description"

# List tasks
./target/release/orca task list

# Run task
./target/release/orca task run <task-id>

# Check health
./target/release/orca health
```

### ACO (Client with TUI)

Terminal UI for orchestrator:

```bash
# Launch TUI (requires running orchestrator-server)
./target/release/aco tui

# Or with specific server
./target/release/aco --server http://localhost:50051 tui
```

### LangGraph CLI

Development tools:

```bash
./target/release/langgraph --help
```

## Development Mode

### Fast Check (No Build)

```bash
# Quick syntax/type check
cargo check

# Check specific crate
cargo check -p orca
```

### Development Build (Faster, with Debug Symbols)

```bash
# Debug build (faster compilation)
cargo build

# Run from debug build
./target/debug/orca --help
```

## Common Commands

### Clean and Rebuild

```bash
# Remove build artifacts
cargo clean

# Rebuild everything
cargo build --release
```

### Format Code

```bash
# Format all workspace code
cargo fmt
```

### Run Linter

```bash
# Lint all workspace code
cargo clippy --all
```

## Quick Development Workflow

Typical workflow for development:

1. **Make changes to code**
2. **Quick check**: `cargo check -p orca`
3. **Run tests**: `cargo test -p orca`
4. **Build**: `cargo build -p orca --release`
5. **Test binary**: `./target/release/orca --help`

## Using Helper Scripts

### Build Orca Script

```bash
# Basic build
./scripts/build-orca.sh

# With tests
./scripts/build-orca.sh --test

# Debug mode
./scripts/build-orca.sh --debug

# Build and install to ~/.cargo/bin
./scripts/build-orca.sh --install

# After install, use anywhere:
orca --help
```

### Distribution Build Script

```bash
# Build complete distribution
./scripts/build-dist.sh
```

This creates `release/` directory with:
- All binaries
- All dynamic libraries
- Documentation
- Example configurations

## Configuration Locations

### Orca Configuration

Priority order:
1. `./.orca/orca.toml` (project-level) - checked first
2. `~/.orca/orca.toml` (user-level) - fallback

### Project-Level Configuration

For project-specific settings:

```bash
# Create project config directory
mkdir -p .orca

# Create project config
cat > .orca/orca.toml <<EOF
[llm]
provider = "ollama"
model = "llama2"

[execution]
streaming = true
EOF
```

## Troubleshooting

### Build Fails

```bash
# Update Rust toolchain
rustup update

# Clean and retry
cargo clean
cargo build --release
```

### Missing Dependencies

**macOS:**
```bash
xcode-select --install
```

**Linux:**
```bash
sudo apt-get install build-essential
```

**Windows:**
- Install Visual Studio Build Tools

### Orca Initialization Fails

```bash
# Check if directory exists
ls -la ~/.orca/

# Remove and reinitialize
rm -rf ~/.orca
./target/release/orca init
```

### LLM API Key Not Working

```bash
# Verify environment variable is set
echo $ANTHROPIC_API_KEY

# Or hardcode in config (not recommended for production)
# Edit ~/.orca/orca.toml and set api_key directly
```

## Build Times

Typical build times (release mode) on modern hardware:

- **Orca only**: ~1-2 minutes (first build), ~10-30 seconds (incremental)
- **Full workspace**: ~3-5 minutes (first build), ~30-60 seconds (incremental)
- **Tests**: ~10-30 seconds
- **Clean build**: ~3-5 minutes

## Output Locations

### Release Build
- Binaries: `target/release/orca`, `target/release/aco`, etc.
- Dynamic libraries: `target/release/*.{dylib,so,dll}`

### Debug Build
- Binaries: `target/debug/orca`, `target/debug/aco`, etc.
- Dynamic libraries: `target/debug/*.{dylib,so,dll}`

### Distribution Build
- All artifacts: `release/`
- Archives: `release/*.{zip,tar.gz}` (if created)

## Next Steps

Once you have Orca running:

1. **Read the Orca README**: `src/crates/orca/README.md`
2. **Explore agent patterns**: See langgraph-prebuilt documentation
3. **Configure LLM providers**: See `src/crates/llm/README.md`
4. **Try the TUI**: `src/crates/aco/TUI_GUIDE.md`

## Detailed Documentation

- **[BUILD.md](BUILD.md)** - Complete build instructions
- **[howto.md](howto.md)** - Detailed setup and build guide
- **[environment.md](environment.md)** - Project structure and architecture
- **[architecture.md](architecture.md)** - System design and components

---

**Quick Tips:**
- Use `orca` for local development (simplest)
- Use `orchestrator` for distributed/production deployments
- Start with ReAct pattern (default, works for 90% of cases)
- Enable streaming for better visibility into LLM thinking
