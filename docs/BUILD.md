# Building acolib and Orca

This document provides comprehensive instructions for building acolib components, with a focus on the Orca standalone orchestrator.

## Quick Start: Build Orca

The fastest way to build and use Orca:

```bash
# Build in release mode
cargo build -p orca --release

# Run directly
./target/release/orca --help

# Or use the helper script
./scripts/build-orca.sh --install
```

## Build Scripts

### 1. Quick Orca Build (`scripts/build-orca.sh`)

**Purpose**: Fast, focused build of just the Orca binary.

```bash
# Basic build (release mode)
./scripts/build-orca.sh

# Debug build
./scripts/build-orca.sh --debug

# Build with tests
./scripts/build-orca.sh --test

# Build and install to ~/.cargo/bin
./scripts/build-orca.sh --install

# All options
./scripts/build-orca.sh --test --install
```

**Options**:
- `--debug`: Build in debug mode (faster compile, larger binary)
- `--test`: Run test suite before building
- `--install`: Copy binary to `~/.cargo/bin/orca`
- `--help`: Show usage information

**Output**:
- Release: `target/release/orca`
- Debug: `target/debug/orca`

### 2. Full Distribution Build (`scripts/build-dist.sh`)

**Purpose**: Build complete distribution with all binaries, libraries, and documentation.

```bash
# Run the distribution build
./scripts/build-dist.sh
```

**What it builds**:
- All workspace binaries (orca, aco, orchestrator-server)
- All shared libraries (.dylib, .dll, .so)
- Complete API documentation
- README, CHANGELOG, CONTRIBUTING files

**Output**: `release/` directory with:
```
release/
├── bin/           # Compiled binaries
│   ├── orca
│   ├── aco
│   └── orchestrator-server
├── lib/           # Shared libraries
│   ├── liborchestrator.dylib
│   └── ...
├── docs/          # Documentation
│   └── orca/      # Orca-specific docs
├── api-docs/      # Generated API documentation
└── config/        # Example configurations
```

**Optional**: Creates ZIP and TAR.GZ archives for distribution.

## Manual Build Commands

### Build Orca Only

```bash
# Debug build (fast, for development)
cargo build -p orca

# Release build (optimized, for production)
cargo build -p orca --release

# With verbose output
cargo build -p orca --release --verbose
```

### Build All Workspace Crates

```bash
# Build everything in debug mode
cargo build --all

# Build everything in release mode
cargo build --all --release
```

### Build Specific Components

```bash
# Build just the orchestrator
cargo build -p orchestrator --release

# Build just the tooling crate
cargo build -p tooling --release

# Build LangGraph components
cargo build -p langgraph-core --release
cargo build -p langgraph-prebuilt --release
```

## Testing

### Run Orca Tests

```bash
# All tests
cargo test -p orca

# Specific test
cargo test -p orca test_name

# With output
cargo test -p orca -- --nocapture

# Integration tests only
cargo test -p orca --test '*'

# Unit tests only
cargo test -p orca --lib
```

### Run All Tests

```bash
# All workspace tests
cargo test --all

# Quiet mode
cargo test --all --quiet
```

## Code Quality

### Check Syntax

```bash
# Fast syntax/type check
cargo check -p orca

# Check all
cargo check --all
```

### Linting

```bash
# Lint Orca
cargo clippy -p orca

# Lint all with warnings as errors
cargo clippy --all -- -D warnings
```

### Format Code

```bash
# Format Orca
cargo fmt -p orca

# Format all
cargo fmt --all

# Check formatting without modifying
cargo fmt --all -- --check
```

## Build Configurations

### Release Profile (Optimized)

```toml
[profile.release]
opt-level = 3              # Maximum optimization
lto = true                 # Link-time optimization
codegen-units = 1          # Single unit for better optimization
strip = false              # Keep debug symbols
debug = false              # No debug info
```

**Binary sizes**:
- Debug: ~50-100 MB (with symbols)
- Release: ~10-20 MB (optimized)

### Development Tips

```bash
# Fast incremental builds
cargo build -p orca

# Check without building
cargo check -p orca

# Watch mode (requires cargo-watch)
cargo watch -x 'check -p orca'

# Clean build artifacts
cargo clean

# Clean just Orca
cargo clean -p orca
```

## Installation

### Install from Source

```bash
# Build and install to ~/.cargo/bin
cargo install --path crates/orca

# Or use the script
./scripts/build-orca.sh --install
```

### Verify Installation

```bash
# Check version
orca --version

# Check installation location
which orca

# Run health check
orca health
```

## First-Time Setup

After building, initialize Orca:

```bash
# 1. Initialize (creates ~/.orca/ directory)
orca init

# 2. Edit configuration
nano ~/.orca/config.toml

# 3. Add your LLM API key
[llm]
provider = "openai"  # or "claude", "ollama", etc.
api_key = "your-api-key-here"

# 4. Check health
orca health

# 5. Create your first task
orca task create "List files in current directory"
orca task list
```

## Build Requirements

### System Requirements

- **Rust**: 1.75.0 or later
- **Cargo**: Latest stable
- **OS**: macOS, Linux, or Windows
- **Memory**: 4GB+ RAM for building
- **Disk**: 2GB+ free space

### Dependencies

All Rust dependencies are managed by Cargo and specified in `Cargo.toml` files.

**Runtime dependencies**:
- SQLite (embedded, no installation needed)
- tokio (async runtime, included)
- LLM provider (OpenAI, Anthropic Claude, Ollama, etc.)

### Platform-Specific Notes

**macOS**:
- Builds produce `.dylib` files
- Code signing may be required for distribution

**Linux**:
- Builds produce `.so` files
- May need `build-essential` package

**Windows**:
- Builds produce `.dll` files
- May need Visual Studio Build Tools

## Troubleshooting

### Build Failures

```bash
# Clean and rebuild
cargo clean
cargo build -p orca --release

# Update dependencies
cargo update

# Check for issues
cargo check -p orca
```

### Link Errors

```bash
# Update Rust toolchain
rustup update

# Check toolchain
rustup show
```

### Out of Memory

```bash
# Reduce parallelism
cargo build -p orca --release -j 2

# Or build in debug mode (less memory)
cargo build -p orca
```

### Slow Builds

```bash
# Use incremental compilation
export CARGO_INCREMENTAL=1

# Enable parallel compilation
export CARGO_BUILD_JOBS=8

# Use faster linker (macOS)
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build Orca

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build
        run: cargo build -p orca --release
      - name: Test
        run: cargo test -p orca
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: orca-binary
          path: target/release/orca
```

## Performance Benchmarks

Build times on typical development machine (M1 Mac, 16GB RAM):

- **Debug build**: ~2-3 minutes (first build), ~30 seconds (incremental)
- **Release build**: ~5-7 minutes (first build), ~1-2 minutes (incremental)
- **Full workspace**: ~10-15 minutes (first build)
- **Tests**: ~5-10 seconds

## Additional Resources

- [Rust Book - Cargo](https://doc.rust-lang.org/cargo/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Orca Documentation](./orca/)
- [Project README](../README.md)

## Getting Help

If you encounter build issues:

1. Check [Troubleshooting](#troubleshooting) section above
2. Review [GitHub Issues](https://github.com/your-org/acolib/issues)
3. Ask in project discussions
4. File a new issue with build logs

---

**Last Updated**: 2025-01-15
**Orca Version**: 0.1.0
