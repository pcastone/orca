//! Input event handling for TUI

use super::app::{App, FocusedArea};
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
            // Tab navigation between areas
            KeyCode::Tab => {
                app.next_focus();
            }
            KeyCode::BackTab => {
                app.prev_focus();
            }

            // Arrow keys and vim navigation
            KeyCode::Up | KeyCode::Char('k') => {
                match app.focused {
                    FocusedArea::Conversation => app.scroll_conversation_up(),
                    FocusedArea::Sidebar => app.sidebar_prev(),
                    FocusedArea::Prompts => {}
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match app.focused {
                    FocusedArea::Conversation => app.scroll_conversation_down(),
                    FocusedArea::Sidebar => app.sidebar_next(),
                    FocusedArea::Prompts => {}
                }
            }

            // Page navigation
            KeyCode::PageUp => {
                match app.focused {
                    FocusedArea::Conversation => {
                        for _ in 0..10 {
                            app.scroll_conversation_up();
                        }
                    }
                    FocusedArea::Sidebar => {
                        for _ in 0..10 {
                            app.scroll_sidebar_up();
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::PageDown => {
                match app.focused {
                    FocusedArea::Conversation => {
                        for _ in 0..10 {
                            app.scroll_conversation_down();
                        }
                    }
                    FocusedArea::Sidebar => {
                        for _ in 0..10 {
                            app.scroll_sidebar_down();
                        }
                    }
                    _ => {}
                }
            }

            // Sidebar tab navigation with left/right or h/l
            KeyCode::Left | KeyCode::Char('h') => {
                if app.focused == FocusedArea::Sidebar {
                    app.prev_tab();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if app.focused == FocusedArea::Sidebar {
                    app.next_tab();
                }
            }

            // Text input in prompts area
            KeyCode::Char(c) if app.focused == FocusedArea::Prompts => {
                app.prompt_input.push(c);
            }
            KeyCode::Backspace if app.focused == FocusedArea::Prompts => {
                app.prompt_input.pop();
            }

            // Actions
            KeyCode::Enter => {
                if app.focused == FocusedArea::Prompts && !app.prompt_input.is_empty() {
                    let prompt = app.prompt_input.clone();
                    app.add_message(format!("You: {}", prompt));
                    app.add_history(prompt.clone());
                    app.prompt_input.clear();
                }
            }

            // Clear conversation
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                if app.focused == FocusedArea::Conversation {
                    app.clear_conversation();
                }
            }

            // Quit
            KeyCode::Char('q') | KeyCode::Esc => {
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
