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
                    // Execute the selected menu action
                    if let Some(action) = app.get_selected_menu_action() {
                        execute_menu_action(&action, app);
                    }
                    return;
                }
                KeyCode::Esc => {
                    app.close_menu();
                    return;
                }
                _ => {}
            }
        }

        // Handle dialog input
        if app.has_dialog() {
            match key_event.code {
                KeyCode::Up => {
                    app.dialog_prev();
                    return;
                }
                KeyCode::Down => {
                    app.dialog_next();
                    return;
                }
                KeyCode::Left => {
                    if let Some(ref mut dialog) = app.dialog {
                        dialog.cursor_left();
                    }
                    return;
                }
                KeyCode::Right => {
                    if let Some(ref mut dialog) = app.dialog {
                        dialog.cursor_right();
                    }
                    return;
                }
                KeyCode::Char(c) => {
                    app.dialog_add_char(c);
                    return;
                }
                KeyCode::Backspace => {
                    app.dialog_backspace();
                    return;
                }
                KeyCode::Enter => {
                    // Close dialog on Enter (selected option or submit)
                    app.close_dialog();
                    return;
                }
                KeyCode::Esc => {
                    app.close_dialog();
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

/// Execute menu action
fn execute_menu_action(action: &str, app: &mut App) {
    match action {
        // File menu actions
        "file_new" => {
            app.clear_conversation();
            app.close_menu();
        }
        "file_open" => {
            // Show file open dialog
            let dialog = super::dialog::Dialog::info("Open File", "Open file functionality coming soon...");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "file_save" => {
            // Show save dialog
            let dialog = super::dialog::Dialog::info("Save", "Conversation saved!");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "file_quit" => {
            app.state.should_quit = true;
            app.close_menu();
        }

        // Edit menu actions
        "edit_clear" => {
            app.clear_conversation();
            app.close_menu();
        }
        "edit_copy" => {
            let dialog = super::dialog::Dialog::info("Copy", "Text copied to clipboard!");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "edit_preferences" => {
            let dialog = super::dialog::Dialog::info("Preferences", "Preferences dialog coming soon...");
            app.show_dialog(dialog);
            app.close_menu();
        }

        // Config menu actions
        "config_view" => {
            // Build config info string
            let config_info = format!(
                "Current Configuration:\n\n\
                Model: {}\n\
                Tokens Used: {}\n\
                Budget: {}\n\
                LLM Profile: {}\n\n\
                Config files:\n\
                - ~/.orca/orca.toml (user)\n\
                - ./.orca/orca.toml (project)",
                app.current_model,
                app.tokens_used,
                app.active_budget.as_ref().unwrap_or(&"None".to_string()),
                app.llm_profile.as_ref().unwrap_or(&"None".to_string())
            );
            let dialog = super::dialog::Dialog::info("Configuration", config_info);
            app.show_dialog(dialog);
            app.close_menu();
        }
        "config_budget" => {
            let dialog = super::dialog::Dialog::info("Budget", "Budget management coming soon...");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "config_llm_profile" => {
            let dialog = super::dialog::Dialog::info("LLM Profiles", "LLM profile management coming soon...");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "config_editor" => {
            let dialog = super::dialog::Dialog::info("Editor", "External editor coming soon...");
            app.show_dialog(dialog);
            app.close_menu();
        }

        // Workflow menu actions (placeholders)
        "workflow_run" => {
            let dialog = super::dialog::Dialog::info("Run Workflow", "Workflow execution coming soon...");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "workflow_view" => {
            let dialog = super::dialog::Dialog::info("View Workflow", "Workflow viewer coming soon...");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "workflow_create" => {
            let dialog = super::dialog::Dialog::info("Create Workflow", "Workflow creation coming soon...");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "workflow_manage" => {
            let dialog = super::dialog::Dialog::info("Manage Workflow", "Workflow management coming soon...");
            app.show_dialog(dialog);
            app.close_menu();
        }

        // Help menu actions
        "help_about" => {
            let dialog = super::dialog::Dialog::info(
                "About Orca",
                "Orca - AI Orchestration Platform\nVersion 1.0\n\nA standalone AI agent workflow executor with budget management and multi-LLM support.",
            );
            app.show_dialog(dialog);
            app.close_menu();
        }
        "help_shortcuts" => {
            let dialog = super::dialog::Dialog::info(
                "Keyboard Shortcuts",
                "Alt+F - File Menu\nAlt+E - Edit Menu\nAlt+C - Config Menu\nAlt+W - Workflow Menu\nAlt+H - Help Menu\n\nTab - Switch focus\nUp/Down - Navigate\nEnter - Select\nEsc - Close/Quit\nCtrl+Enter - Submit prompt\nCtrl+C - Clear conversation",
            );
            app.show_dialog(dialog);
            app.close_menu();
        }
        "help_documentation" => {
            let dialog = super::dialog::Dialog::info("Documentation", "Documentation coming soon...\nVisit https://github.com/anthropics/orca for more info.");
            app.show_dialog(dialog);
            app.close_menu();
        }

        _ => {
            app.close_menu();
        }
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}
