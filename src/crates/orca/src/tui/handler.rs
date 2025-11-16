//! Input event handling for TUI

use super::app::{App, FocusedPanel};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::debug;

/// Handles keyboard input events
pub struct InputHandler;

impl InputHandler {
    /// Create a new input handler
    pub fn new() -> Self {
        Self
    }

    /// Handle a keyboard event
    pub fn handle_key_event(&self, key_event: KeyEvent, app: &mut App) {
        debug!("Key event: {:?}", key_event);

        match key_event.code {
            // Tab navigation between panels
            KeyCode::Tab => {
                app.next_panel();
            }
            KeyCode::BackTab => {
                app.prev_panel();
            }

            // Arrow keys and vim navigation
            KeyCode::Up | KeyCode::Char('k') => {
                match app.focused {
                    FocusedPanel::Commands => app.prev_command(),
                    FocusedPanel::Logs => app.scroll_logs_up(),
                    _ => {}
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match app.focused {
                    FocusedPanel::Commands => app.next_command(),
                    FocusedPanel::Logs => app.scroll_logs_down(),
                    _ => {}
                }
            }

            // Page navigation
            KeyCode::PageUp => {
                if app.focused == FocusedPanel::Logs {
                    for _ in 0..10 {
                        app.scroll_logs_up();
                    }
                }
            }
            KeyCode::PageDown => {
                if app.focused == FocusedPanel::Logs {
                    for _ in 0..10 {
                        app.scroll_logs_down();
                    }
                }
            }

            // Numeric shortcuts for panels
            KeyCode::Char('1') => app.focused = FocusedPanel::Status,
            KeyCode::Char('2') => app.focused = FocusedPanel::Commands,
            KeyCode::Char('3') => app.focused = FocusedPanel::Output,
            KeyCode::Char('4') => app.focused = FocusedPanel::Logs,

            // Actions
            KeyCode::Enter => {
                if app.focused == FocusedPanel::Commands {
                    // Handle command execution - will be populated with actual command logic
                    app.add_log(format!("Executing command {}...", app.selected_command));
                }
            }

            // Clear actions
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                match app.focused {
                    FocusedPanel::Output => app.clear_output(),
                    FocusedPanel::Logs => app.clear_logs(),
                    _ => {}
                }
            }

            // Quit
            KeyCode::Char('q') => {
                app.state.should_quit = true;
            }

            _ => {}
        }
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}
