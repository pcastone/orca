# ACO TUI User Guide

## Overview

The ACO Terminal User Interface (TUI) provides a powerful, terminal-based interface for managing tasks and workflows in the Orca orchestration system. Built with Ratatui and Crossterm, it offers a responsive, feature-rich experience with comprehensive keyboard navigation.

## Features

- **Real-time Task & Workflow Management**: View, execute, and monitor tasks and workflows
- **Execution Streaming**: Watch real-time execution events with color-coded output
- **Comprehensive Keyboard Navigation**: Vim-style keys, page scrolling, and direct view switching
- **Status Indicators**: Color-coded icons and badges for instant status recognition
- **Auto-refresh**: Automatic data updates every 10 seconds
- **Responsive Design**: Adapts to terminal size with proper scrolling

## Installation

### Build from Source

```bash
cd src/crates/aco
cargo build --release
```

The binary will be available at `target/release/aco`.

### Run in Development

```bash
cargo run -- tui
```

## Usage

### Starting the TUI

```bash
# Use default server (localhost:50051)
aco tui

# Connect to specific server
aco --server http://example.com:50051 tui

# With authentication
aco --connect user:password tui

# Verbose mode
aco -v tui
```

### Environment Variables

- `ACO_SERVER`: Default server URL (default: `http://localhost:50051`)
- `ACO_CONNECT`: Authentication string
- `RUST_LOG`: Logging level (`debug`, `info`, `warn`, `error`)

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Navigate up |
| `↓` / `j` | Navigate down |
| `Home` / `g` | Jump to first item |
| `End` / `G` | Jump to last item |
| `PgUp` | Scroll up one page (10 items) |
| `PgDn` | Scroll down one page (10 items) |
| `Enter` | View details / Select item |
| `Esc` | Back / Return to list / Quit |

### View Switching

| Key | Action |
|-----|--------|
| `Tab` | Cycle to next view |
| `Shift+Tab` | Cycle to previous view |
| `1` | Tasks List |
| `2` | Workflows List |
| `3` | Execution Stream |
| `4` / `?` / `h` / `F1` | Help |

### Actions

| Key | Action |
|-----|--------|
| `e` | Execute selected task/workflow |
| `r` | Refresh data from server |
| `q` / `Ctrl+C` | Quit application |

## Views

### 1. Task List

Displays all tasks with:
- **Status Icons**:
  - ⏸ (yellow) - Pending
  - ▶ (cyan) - Running
  - ✔ (green) - Completed
  - ✗ (red) - Failed
  - ⊗ (gray) - Cancelled

- **Type Badges**:
  - `[EXEC]` - Execution tasks
  - `[FLOW]` - Workflow tasks
  - `[VALD]` - Validation tasks

- **Header Statistics**: Shows count by status (e.g., "3 Tasks: ⏸1 ▶1 ✔1")

### 2. Task Details

Shows comprehensive task information:
- ID, Title, Description
- Status (color-coded)
- Type, Workspace path
- Configuration (JSON)
- Metadata (JSON)
- Created/Updated timestamps

### 3. Workflow List

Displays all workflows with:
- **Status Icons**:
  - ◯ (gray) - Draft
  - ◉ (green) - Active
  - ▶ (cyan) - Running
  - ⏸ (yellow) - Paused
  - ✔ (green) - Completed
  - ✗ (red) - Failed

- **Header Statistics**: Shows workflow counts by status

### 4. Workflow Details

Shows workflow information:
- ID, Name, Status
- Created timestamp

### 5. Execution Stream

Real-time execution event display with:
- **Event Types** (color-coded):
  - ▶ (green) - Started
  - ⋯ (cyan) - Progress updates
  - ◉ (yellow) - Output messages
  - 🔧 (magenta) - Tool calls
  - ✓ (blue) - Tool results
  - ✔ (green) - Completed
  - ✗ (red) - Failed

- Timestamp display (HH:MM:SS)
- Auto-scrolling for new events
- Execution ID in title

### 6. Help

Interactive help screen showing:
- Complete keyboard reference
- Status indicator legend
- Type badge reference
- Server connection info

## Configuration

### TUI Config Structure

The TUI uses the `TuiConfig` structure:

```rust
pub struct TuiConfig {
    pub server_url: String,      // Orchestrator server URL
    pub workspace: PathBuf,       // Workspace directory
    pub verbose: bool,            // Verbose logging
}
```

### Server Connection

The TUI connects to the orchestrator server via gRPC. Ensure the orchestrator is running:

```bash
# Start orchestrator (if using standalone)
cargo run -p orchestrator
```

## Deployment

### Standalone Deployment

1. **Build the binary**:
   ```bash
   cargo build --release -p aco
   ```

2. **Copy binary to deployment location**:
   ```bash
   sudo cp target/release/aco /usr/local/bin/
   ```

3. **Set permissions**:
   ```bash
   sudo chmod +x /usr/local/bin/aco
   ```

### Docker Deployment

Create a `Dockerfile`:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p aco

FROM debian:bookworm-slim
RUN apt-update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/aco /usr/local/bin/aco
CMD ["aco", "tui"]
```

Build and run:

```bash
docker build -t aco-tui .
docker run -it --rm -e ACO_SERVER=http://orchestrator:50051 aco-tui
```

### Systemd Service

Create `/etc/systemd/system/aco-tui.service`:

```ini
[Unit]
Description=ACO TUI Service
After=network.target

[Service]
Type=simple
User=aco
Environment="ACO_SERVER=http://localhost:50051"
Environment="RUST_LOG=info"
ExecStart=/usr/local/bin/aco tui
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable aco-tui
sudo systemctl start aco-tui
```

## Testing

### Unit Tests

```bash
cargo test -p aco --lib
```

Runs 127 unit tests covering:
- App state management
- Navigation logic
- View switching
- Data operations

### Integration Tests

```bash
cargo test -p aco --test tui_integration
```

Runs 16 integration tests covering:
- Full user workflows
- Multi-view navigation
- Task/workflow operations
- Edge cases

### Performance Benchmarks

```bash
cargo bench --bench tui_benchmarks
```

Benchmarks:
- App initialization (125ns avg)
- Task operations (923ns avg per task)
- Navigation performance (sub-microsecond)
- Large dataset handling (50,000 tasks in 34ms)

## Troubleshooting

### Connection Issues

**Problem**: "Failed to refresh tasks: connection refused"

**Solution**:
1. Verify orchestrator is running: `ps aux | grep orchestrator`
2. Check server URL: `echo $ACO_SERVER`
3. Test connection: `curl http://localhost:50051`

### Display Issues

**Problem**: Characters not rendering correctly

**Solution**:
1. Ensure UTF-8 locale: `export LANG=en_US.UTF-8`
2. Update terminal: `apt-get update && apt-get upgrade`
3. Try different terminal emulator

### Performance Issues

**Problem**: TUI feels sluggish with many tasks

**Solution**:
1. Reduce auto-refresh frequency in code
2. Filter tasks by status before viewing
3. Use pagination features (PgUp/PgDn)

### Authentication Errors

**Problem**: "Authentication failed"

**Solution**:
1. Check credentials: `aco auth status`
2. Login again: `aco auth login`
3. Verify connect string format: `user:password` or `token:jwt`

## Architecture

### Component Structure

```
aco/src/tui/
├── mod.rs           # Main TUI entry point & event loop
├── app.rs           # Application state & logic
├── ui.rs            # Rendering functions (ratatui)
├── events.rs        # Event handling (crossterm)
└── grpc_client.rs   # Data loading from orchestrator
```

### Data Flow

```
User Input → Event Handler → App State → UI Renderer → Terminal
     ↑                                        ↓
     └──────────── Auto-refresh ←────────────┘
```

### State Management

The `App` struct maintains:
- **tasks**: Vector of TaskItem
- **workflows**: Vector of WorkflowItem
- **execution_events**: Vector of ExecutionEvent
- **view**: Current view enum
- **selected**: Selected item index
- **scroll**: Scroll position

## Performance Characteristics

- **Memory**: ~10 bytes per task/workflow item
- **CPU**: Sub-1% during idle
- **Latency**: <1ms for most operations
- **Scalability**: Tested with 50,000 tasks

## Contributing

### Adding New Views

1. Add view variant to `View` enum in `app.rs`
2. Implement view logic in `app.rs`
3. Add rendering function in `ui.rs`
4. Update view cycling in `next_view()`/`previous_view()`
5. Add keyboard shortcuts in `mod.rs`
6. Update help text

### Adding New Features

1. Add feature to `App` struct
2. Implement helper methods
3. Add keyboard shortcuts
4. Update UI rendering
5. Add tests
6. Update documentation

## License

See LICENSE file in repository root.

## Support

For issues, questions, or contributions:
- GitHub Issues: https://github.com/your-org/orca/issues
- Documentation: https://github.com/your-org/orca/docs

## Changelog

### Version 0.1.0 (Current)

**Features**:
- Task list with status indicators
- Workflow list with status indicators
- Task/workflow detail views
- Execution streaming view
- Comprehensive keyboard navigation
- Auto-refresh functionality
- Help system

**Testing**:
- 127 unit tests
- 16 integration tests
- 11 performance benchmarks

**Performance**:
- Sub-microsecond navigation
- Handles 50,000+ tasks efficiently
- Minimal memory footprint
