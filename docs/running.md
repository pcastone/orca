# Quick Start: Running the Project

## Quick Build and Run

### Build Everything (Release Mode)
```bash
cd /Users/pcastone/Projects/acolib
cargo build --release
```

This will create:
- 3 dynamic libraries in `target/release/`
- 1 CLI binary in `target/release/`

### Verify Build
```bash
# Check dynamic libraries
ls -lh target/release/*.dylib

# Check CLI binary
ls -lh target/release/langgraph
```

Expected output:
```
-rwxr-xr-x  1 user  staff   16K  liblanggraph_checkpoint.dylib
-rwxr-xr-x  1 user  staff   16K  liblanggraph_core.dylib
-rwxr-xr-x  1 user  staff   16K  liblanggraph_prebuilt.dylib
-rwxr-xr-x  1 user  staff  1.0M  langgraph
```

## Using the Dynamic Libraries

### Copy to Release Directory
```bash
cp target/release/*.dylib release/
```

### View Library Information
```bash
# macOS
otool -L release/liblanggraph_core.dylib

# Linux
ldd release/liblanggraph_core.so
```

## Running Tests

### Quick Test
```bash
cargo test
```

### Test Specific Crate
```bash
cargo test -p langgraph-core
```

## Using the CLI

### Run CLI Help
```bash
./target/release/langgraph --help
```

### Install CLI (Optional)
```bash
cargo install --path crates/langgraph-cli
langgraph --help
```

## Development Mode

### Fast Check (No Build)
```bash
cargo check
```

### Development Build (Faster, with Debug Symbols)
```bash
cargo build
ls -lh target/debug/*.dylib
```

## Common Commands

### Clean and Rebuild
```bash
cargo clean
cargo build --release
```

### Format Code
```bash
cargo fmt
```

### Run Linter
```bash
cargo clippy
```

## Quick Development Workflow

1. **Make changes to code**
2. **Quick check**: `cargo check`
3. **Run tests**: `cargo test`
4. **Build release**: `cargo build --release`
5. **Copy DLLs**: `cp target/release/*.dylib release/`

## Troubleshooting

### Build Fails
```bash
# Update Rust
rustup update

# Clean and retry
cargo clean
cargo build --release
```

### Missing Dependencies
```bash
# macOS
xcode-select --install

# Linux
sudo apt-get install build-essential
```

## Build Times

Typical build times (release mode):
- **First build**: ~25-30 seconds (downloads and compiles all dependencies)
- **Incremental builds**: ~5-10 seconds (only changed files)
- **Clean build**: ~25 seconds

## Output Locations

### Release Build
- Dynamic libraries: `target/release/*.dylib`
- CLI binary: `target/release/langgraph`
- Copied DLLs: `release/*.dylib`

### Debug Build
- Dynamic libraries: `target/debug/*.dylib`
- CLI binary: `target/debug/langgraph`

## Next Steps

See detailed documentation:
- `docs/howto.md` - Complete setup and build instructions
- `docs/environment.md` - Project structure and architecture
- `docs/project_prd.md` - Project requirements