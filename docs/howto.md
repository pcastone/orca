# How To: Setup and Build Instructions

## Prerequisites

### Required Software
- **Rust** 1.75.0 or higher
  - Install via rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - Verify: `rustc --version`
- **Cargo** (comes with Rust)
  - Verify: `cargo --version`

### Optional Tools
- **git** - for version control
- **rust-analyzer** - for IDE support

## Initial Setup

### 1. Clone or Navigate to Project
```bash
cd /Users/pcastone/Projects/acolib
```

### 2. Verify Workspace Structure
```bash
# Check workspace members
cargo metadata --format-version 1 | grep -A 10 workspace_members

# List all crates
ls -la crates/
```

## Building the Project

### Development Build
Build all crates in debug mode:
```bash
cargo build
```

This will:
- Build all 4 workspace crates
- Generate debug symbols
- Place artifacts in `target/debug/`
- Create development versions of dylibs

### Release Build (Recommended for DLLs)
Build optimized dynamic libraries:
```bash
cargo build --release
```

This will:
- Apply LTO (Link-Time Optimization)
- Use maximum optimization (opt-level 3)
- Place artifacts in `target/release/`
- Create production-ready dylibs

Build artifacts location:
- Dynamic libraries: `target/release/*.dylib`
- CLI binary: `target/release/langgraph`

### Building Specific Crates
```bash
# Build only checkpoint library
cargo build --release -p langgraph-checkpoint

# Build only core library
cargo build --release -p langgraph-core

# Build only prebuilt library
cargo build --release -p langgraph-prebuilt

# Build only CLI
cargo build --release -p langgraph-cli
```

## Testing

### Run All Tests
```bash
cargo test
```

### Run Tests for Specific Crate
```bash
cargo test -p langgraph-core
cargo test -p langgraph-checkpoint
cargo test -p langgraph-prebuilt
```

### Run with Output
```bash
cargo test -- --nocapture
```

## Code Quality

### Check Code (Fast)
```bash
cargo check
```

### Run Clippy (Linter)
```bash
cargo clippy --all-targets --all-features
```

### Format Code
```bash
cargo fmt
```

## Working with Dynamic Libraries

### Locating Built Libraries
After `cargo build --release`:
```bash
# List all dynamic libraries
find target/release -name "*.dylib" -o -name "*.so" -o -name "*.dll"

# Or directly
ls -lh target/release/*.dylib
```

### Copy to Release Directory
The build process automatically copies libraries to the `release/` directory:
```bash
cp target/release/*.dylib release/
```

### Verify Library Symbols
On macOS:
```bash
nm -gU target/release/liblanggraph_core.dylib
```

On Linux:
```bash
nm -D target/release/liblanggraph_core.so
```

### Check Library Dependencies
On macOS:
```bash
otool -L target/release/liblanggraph_core.dylib
```

On Linux:
```bash
ldd target/release/liblanggraph_core.so
```

## Using the CLI Tool

### Run CLI
```bash
./target/release/langgraph --help
```

### Install CLI Globally
```bash
cargo install --path crates/langgraph-cli
```

## Clean Build

### Remove Build Artifacts
```bash
cargo clean
```

### Deep Clean (including dependencies)
```bash
rm -rf target/
cargo clean
```

## Benchmarking

Run benchmarks (if available):
```bash
cargo bench
```

## Troubleshooting

### Issue: "command not found: cargo"
**Solution**: Install Rust via rustup:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Issue: Build fails with linking errors
**Solution**: Ensure you have the required system libraries:
- macOS: `xcode-select --install`
- Linux: `sudo apt-get install build-essential`

### Issue: "error: package requires edition 2021"
**Solution**: Update Rust:
```bash
rustup update
```

### Issue: Slow compilation
**Solution**: Use parallel compilation:
```bash
# Set in .cargo/config.toml or environment
export CARGO_BUILD_JOBS=8
```

## Development Workflow

### Typical Development Cycle
1. Make code changes
2. Run quick check: `cargo check`
3. Run tests: `cargo test`
4. Format code: `cargo fmt`
5. Run linter: `cargo clippy`
6. Build release: `cargo build --release`
7. Copy artifacts: `cp target/release/*.dylib release/`

### Watch Mode (with cargo-watch)
Install cargo-watch:
```bash
cargo install cargo-watch
```

Auto-rebuild on changes:
```bash
cargo watch -x check
cargo watch -x test
cargo watch -x "build --release"
```

## Platform-Specific Notes

### macOS
- Libraries have `.dylib` extension
- May need to handle code signing for distribution
- Use `otool` for library inspection

### Linux
- Libraries have `.so` extension
- May need to set `LD_LIBRARY_PATH` for runtime loading
- Use `ldd` and `nm` for library inspection

### Windows
- Libraries have `.dll` extension
- May need Visual Studio Build Tools
- Use `dumpbin` for library inspection

## Advanced: Cross-Compilation

### For Linux (from macOS)
```bash
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

### For Windows (from macOS/Linux)
```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

## Documentation

### Generate Documentation
```bash
cargo doc --no-deps --open
```

### Generate Documentation with Dependencies
```bash
cargo doc --open
```