//! Input event handling for TUI

use super::app::{App, FocusedArea, MenuState};
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

        // Handle menu-specific keys first
        if app.focused == FocusedArea::Menu {
            match key_event.code {
                KeyCode::Up => {
                    app.menu_prev();
                    return;
                }
                KeyCode::Down => {
                    app.menu_next();
                    return;
                }
                KeyCode::Enter => {
                    // Menu action will be handled by UI layer
                    return;
                }
                KeyCode::Esc => {
                    app.close_menu();
                    return;
                }
                _ => {}
            }
        }

        // Handle Esc to close menu in any other focus area
        if matches!(key_event.code, KeyCode::Esc) && app.menu_state != MenuState::Closed {
            app.close_menu();
            return;
        }

        // Handle Alt+F/E/C/W/H for menu shortcuts
        if key_event.modifiers.contains(KeyModifiers::ALT) {
            match key_event.code {
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    app.open_menu(MenuState::FileOpen);
                    return;
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    app.open_menu(MenuState::EditOpen);
                    return;
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    app.open_menu(MenuState::ConfigOpen);
                    return;
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    app.open_menu(MenuState::WorkflowOpen);
                    return;
                }
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    app.open_menu(MenuState::HelpOpen);
                    return;
                }
                _ => {}
            }
        }

        match key_event.code {
            // Tab navigation between areas
            KeyCode::Tab => {
                app.next_focus();
            }
            KeyCode::BackTab => {
                app.prev_focus();
            }

            // PROMPTS AREA: Check prompts first to avoid conflicts
            KeyCode::Left if app.focused == FocusedArea::Prompts => {
                app.prompt_cursor_left();
            }
            KeyCode::Right if app.focused == FocusedArea::Prompts => {
                app.prompt_cursor_right();
            }
            KeyCode::Char(c) if app.focused == FocusedArea::Prompts => {
                app.add_prompt_char(c);
            }
            KeyCode::Backspace if app.focused == FocusedArea::Prompts => {
                app.backspace_prompt();
            }

            // Arrow keys and vim navigation
            KeyCode::Up | KeyCode::Char('k') => {
                match app.focused {
                    FocusedArea::Conversation => app.scroll_conversation_up(),
                    FocusedArea::Sidebar => app.sidebar_prev(),
                    FocusedArea::Prompts => {}
                    FocusedArea::Menu => {} // Menu navigation handled above
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match app.focused {
                    FocusedArea::Conversation => app.scroll_conversation_down(),
                    FocusedArea::Sidebar => app.sidebar_next(),
                    FocusedArea::Prompts => {}
                    FocusedArea::Menu => {} // Menu navigation handled above
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

            // Sidebar tab navigation (only for sidebar)
            KeyCode::Char('h') if app.focused == FocusedArea::Sidebar => {
                app.prev_tab();
            }
            KeyCode::Char('l') if app.focused == FocusedArea::Sidebar => {
                app.next_tab();
            }

            // Newline in prompt (max 3 lines)
            KeyCode::Enter if app.focused == FocusedArea::Prompts => {
                app.newline_prompt();
            }

            // Submit prompt with Ctrl+Enter
            KeyCode::Enter if key_event.modifiers.contains(KeyModifiers::CONTROL) && app.focused == FocusedArea::Prompts => {
                let prompt_text = app.get_prompt_text();
                if !prompt_text.trim().is_empty() {
                    app.add_message(format!("You:\n{}", prompt_text));
                    app.add_history(prompt_text);
                    app.clear_prompt();
                }
            }

            // Clear conversation
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                if app.focused == FocusedArea::Conversation {
                    app.clear_conversation();
                }
            }

            // Quit (q or Esc when menu is not open)
            KeyCode::Char('q') => {
                app.state.should_quit = true;
            }
            KeyCode::Esc if app.menu_state == MenuState::Closed => {
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
