# How To: Setup and Build Instructions

Complete guide for setting up your development environment and building acolib.

## Prerequisites

### Required Software

**Rust 1.75.0 or higher**
```bash
# Install via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

**Git** (for version control)
```bash
# Verify git is installed
git --version
```

### Optional Tools

- **rust-analyzer** - IDE support for VS Code, IntelliJ, etc.
- **cargo-watch** - Auto-rebuild on file changes
- **cargo-nextest** - Faster test runner

```bash
# Install optional tools
cargo install cargo-watch
cargo install cargo-nextest
```

## Initial Setup

### 1. Clone or Navigate to Project

If you haven't already:
```bash
git clone <repository-url>
cd acolib
```

Or if you're already in the project:
```bash
cd path/to/acolib
```

### 2. Verify Workspace Structure

```bash
# Check workspace members
cargo metadata --format-version 1 | grep -A 15 workspace_members

# List all crates
ls -la src/crates/
```

You should see 10 crates:
- aco
- orca
- orchestrator
- langgraph-core
- langgraph-checkpoint
- langgraph-prebuilt
- langgraph-cli
- llm
- tooling
- utils

## Building the Project

### Option 1: Build Orca (Recommended for New Users)

Orca is the standalone orchestrator and primary user tool:

```bash
# Using the build script (easiest)
./scripts/build-orca.sh

# Or build directly
cargo build -p orca --release

# Or with specific options
./scripts/build-orca.sh --test --install
```

**Script options:**
- `--debug` - Build in debug mode (faster compile, larger binary)
- `--test` - Run test suite before building
- `--install` - Install to `~/.cargo/bin/` for global access

**Output:** `target/release/orca`

### Option 2: Build All Workspace Crates

Build everything at once:

```bash
# Release build (optimized)
cargo build --release

# Debug build (faster compilation)
cargo build
```

This builds all 10 crates including:
- **Binaries**: orca, aco, orchestrator-server, langgraph
- **Libraries**: All supporting libraries as .dylib/.so/.dll

**Output:** `target/release/` or `target/debug/`

### Option 3: Build Specific Crates

Build only what you need:

```bash
# Build just the orchestrator
cargo build -p orchestrator --release

# Build just the LLM crate
cargo build -p llm --release

# Build langgraph-core
cargo build -p langgraph-core --release

# Build ACO client
cargo build -p aco --release
```

### Build Artifacts Location

After building:

```bash
# Binaries
ls -lh target/release/orca
ls -lh target/release/aco
ls -lh target/release/langgraph

# Dynamic libraries (platform-specific)
ls -lh target/release/*.dylib    # macOS
ls -lh target/release/*.so       # Linux
ls -lh target/release/*.dll      # Windows
```

## Testing

### Run All Tests

```bash
# Run all workspace tests
cargo test

# With output visible
cargo test -- --nocapture

# Quiet mode (less verbose)
cargo test --quiet
```

### Run Tests for Specific Crate

```bash
# Test Orca
cargo test -p orca

# Test langgraph-core
cargo test -p langgraph-core

# Test LLM crate
cargo test -p llm

# Test with verbose output
cargo test -p orca -- --nocapture
```

### Run Integration Tests Only

```bash
# Integration tests for specific crate
cargo test -p orca --test '*'
```

### Run Unit Tests Only

```bash
# Unit tests for specific crate
cargo test -p orca --lib
```

### Using cargo-nextest (Faster)

```bash
# Install if not already installed
cargo install cargo-nextest

# Run tests with nextest
cargo nextest run

# Run tests for specific crate
cargo nextest run -p orca
```

## Code Quality

### Check Syntax and Types (Fast)

```bash
# Check all workspace crates
cargo check

# Check specific crate
cargo check -p orca

# With verbose output
cargo check --verbose
```

### Linting with Clippy

```bash
# Lint all crates
cargo clippy --all

# Lint with warnings as errors
cargo clippy --all -- -D warnings

# Lint specific crate
cargo clippy -p orca
```

### Format Code

```bash
# Format all workspace code
cargo fmt

# Check formatting without modifying
cargo fmt --all -- --check

# Format specific crate
cargo fmt -p orca
```

## Installation

### Install Orca Globally

```bash
# Option 1: Using build script
./scripts/build-orca.sh --install

# Option 2: Using cargo install
cargo install --path src/crates/orca

# Verify installation
which orca
orca --version
```

After installation, `orca` will be available globally:

```bash
# Use from anywhere
orca --help
orca init
orca task create "Test task"
```

### Install Other Binaries

```bash
# Install ACO
cargo install --path src/crates/aco

# Install langgraph CLI
cargo install --path src/crates/langgraph-cli
```

## First-Time Setup (Orca)

After building or installing Orca:

### 1. Initialize Orca

```bash
# Initialize (creates ~/.orca/ directory)
orca init
```

This creates:
- `~/.orca/` directory
- `~/.orca/orca.db` (SQLite database)
- `~/.orca/orca.toml` (configuration file)

### 2. Configure LLM Provider

Edit `~/.orca/orca.toml`:

```toml
[llm]
provider = "anthropic"          # Options: anthropic, openai, ollama, gemini, etc.
model = "claude-3-sonnet"       # Model name for your provider
api_key = "${ANTHROPIC_API_KEY}" # Reads from environment variable

[execution]
streaming = true                 # Enable real-time token streaming
```

### 3. Set Environment Variable

```bash
# For Anthropic Claude
export ANTHROPIC_API_KEY="your-api-key-here"

# For OpenAI
export OPENAI_API_KEY="your-api-key-here"

# For Google Gemini
export GOOGLE_API_KEY="your-api-key-here"

# Make permanent (add to ~/.bashrc or ~/.zshrc)
echo 'export ANTHROPIC_API_KEY="your-key"' >> ~/.bashrc
source ~/.bashrc
```

### 4. Verify Setup

```bash
# Check health
orca health

# Create test task
orca task create "What is 2+2?"

# List tasks
orca task list
```

## Development Workflow

### Typical Development Cycle

1. **Make code changes**
2. **Quick check**: `cargo check -p <crate-name>`
3. **Run tests**: `cargo test -p <crate-name>`
4. **Format code**: `cargo fmt`
5. **Lint**: `cargo clippy -p <crate-name>`
6. **Build release**: `cargo build -p <crate-name> --release`

### Watch Mode (Auto-Rebuild)

Install and use cargo-watch for automatic rebuilds:

```bash
# Install cargo-watch
cargo install cargo-watch

# Auto-rebuild on changes
cargo watch -x 'check -p orca'

# Auto-test on changes
cargo watch -x 'test -p orca'

# Auto-build release on changes
cargo watch -x 'build -p orca --release'

# Multiple commands
cargo watch -x check -x test -x build
```

## Platform-Specific Notes

### macOS

**Library Extension**: `.dylib`

**Additional Tools**:
```bash
# Install Xcode Command Line Tools (if needed)
xcode-select --install

# View library dependencies
otool -L target/release/liblanggraph_core.dylib

# Check library symbols
nm -gU target/release/liblanggraph_core.dylib
```

**Code Signing** (for distribution):
```bash
codesign -s "Developer ID Application" target/release/orca
```

### Linux

**Library Extension**: `.so`

**Additional Tools**:
```bash
# Install build essentials (if needed)
sudo apt-get update
sudo apt-get install build-essential

# View library dependencies
ldd target/release/liblanggraph_core.so

# Check library symbols
nm -D target/release/liblanggraph_core.so
```

### Windows

**Library Extension**: `.dll`

**Requirements**:
- Visual Studio Build Tools
- Or MinGW-w64 toolchain

**Install Build Tools**:
```powershell
# Download and install Visual Studio Build Tools
# https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
```

**Check Dependencies**:
```powershell
# Use dumpbin (comes with Visual Studio)
dumpbin /DEPENDENTS target\release\langgraph_core.dll
```

## Advanced: Cross-Compilation

### Compile for Linux (from macOS)

```bash
# Add target
rustup target add x86_64-unknown-linux-gnu

# Build for Linux
cargo build --release --target x86_64-unknown-linux-gnu

# Output in target/x86_64-unknown-linux-gnu/release/
```

### Compile for Windows (from macOS/Linux)

```bash
# Add target
rustup target add x86_64-pc-windows-gnu

# Build for Windows
cargo build --release --target x86_64-pc-windows-gnu

# Output in target/x86_64-pc-windows-gnu/release/
```

### Compile for macOS (from Linux)

```bash
# Add target (requires osxcross or similar)
rustup target add x86_64-apple-darwin

# Build for macOS
cargo build --release --target x86_64-apple-darwin
```

## Generating Documentation

### API Documentation

```bash
# Generate docs for all crates
cargo doc --no-deps

# Generate and open in browser
cargo doc --no-deps --open

# Include dependencies
cargo doc --open

# Generate docs for specific crate
cargo doc -p orca --no-deps --open
```

## Troubleshooting

### Issue: "command not found: cargo"

**Solution**: Install Rust via rustup:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Issue: Build fails with linking errors

**macOS**:
```bash
xcode-select --install
```

**Linux**:
```bash
sudo apt-get install build-essential
```

**Windows**:
- Install Visual Studio Build Tools

### Issue: "error: package requires edition 2021"

**Solution**: Update Rust:
```bash
rustup update stable
rustup default stable
```

### Issue: Slow compilation

**Solutions**:
```bash
# 1. Use more parallel jobs
export CARGO_BUILD_JOBS=8

# 2. Enable incremental compilation (usually default)
export CARGO_INCREMENTAL=1

# 3. Use faster linker (macOS)
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"

# 4. Use mold linker (Linux)
# Install mold first, then:
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

### Issue: Out of memory during build

**Solution**: Reduce parallelism:
```bash
# Build with only 2 parallel jobs
cargo build --release -j 2
```

### Issue: Tests failing

**Debugging**:
```bash
# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --nocapture

# Run tests with backtrace
RUST_BACKTRACE=1 cargo test
```

## Clean Build

### Remove Build Artifacts

```bash
# Clean all build artifacts
cargo clean

# Clean specific crate (not supported directly, clean rebuilds all)
cargo clean

# Rebuild from scratch
cargo clean && cargo build --release
```

### Deep Clean

```bash
# Remove target directory entirely
rm -rf target/

# Remove Cargo.lock (regenerates on next build)
rm Cargo.lock

# Rebuild
cargo build --release
```

## Performance Benchmarks

Typical build times on modern hardware (M1 Mac, 16GB RAM, or equivalent):

### First Build (Clean)
- **Orca only**: ~2-3 minutes
- **Full workspace**: ~5-7 minutes

### Incremental Build (Minor Changes)
- **Orca**: ~10-30 seconds
- **Full workspace**: ~30-60 seconds

### Test Execution
- **All tests**: ~10-30 seconds
- **Single crate tests**: ~5-10 seconds

## Additional Resources

- **[Rust Book - Cargo](https://doc.rust-lang.org/cargo/)** - Official Cargo documentation
- **[Rust Performance Book](https://nnethercote.github.io/perf-book/)** - Optimization guide
- **[BUILD.md](BUILD.md)** - Comprehensive build documentation
- **[running.md](running.md)** - Quick start guide

## Getting Help

If you encounter issues:

1. Check **[Troubleshooting](#troubleshooting)** section above
2. Review **[GitHub Issues](https://github.com/anthropics/acolib/issues)**
3. Check workspace documentation in `docs/`
4. File a new issue with:
   - Your platform (OS, Rust version)
   - Full error message
   - Build command used
   - Output of `rustc --version` and `cargo --version`

---

**Last Updated**: 2025-01-16
