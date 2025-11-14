# Task 021: Set Up TUI Framework (ratatui)

## Objective
Create TUI foundation with ratatui, event loop, and app state.

## Dependencies
- Task 016 (Client infrastructure)

## Key Files
- `src/crates/aco/src/tui/mod.rs`
- `src/crates/aco/src/tui/app.rs` - App state
- `src/crates/aco/src/tui/ui.rs` - Rendering
- `src/crates/aco/src/tui/events.rs` - Event handling

## Implementation
- App struct with current view state
- Main event loop (60fps)
- Terminal setup/cleanup
- Keyboard event handling
- Layout: Header | Main | Footer

## Complexity: Moderate | Effort: 6-8 hours
