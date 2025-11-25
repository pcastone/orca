//! Input event handling for TUI

use super::app::{App, DialogState, FocusedArea, LlmConfigForm, MenuState, SidebarTab};
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
        debug!("Key event: code={:?}, modifiers={:?}", key_event.code, key_event.modifiers);

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
                KeyCode::Left => {
                    app.prev_menu();
                    return;
                }
                KeyCode::Right => {
                    app.next_menu();
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

        // Handle pattern selection dialog input
        if app.dialog_state == DialogState::PatternSelect || app.dialog_state == DialogState::PatternList {
            match key_event.code {
                KeyCode::Up => {
                    app.pattern_select_prev();
                    return;
                }
                KeyCode::Down => {
                    app.pattern_select_next();
                    return;
                }
                KeyCode::Enter => {
                    app.confirm_pattern_selection();
                    return;
                }
                KeyCode::Esc => {
                    app.dialog_state = DialogState::None;
                    return;
                }
                _ => return,
            }
        }

        // Handle LLM config form input
        if app.dialog_state == DialogState::LlmProfileEdit || app.dialog_state == DialogState::LlmProfileCreate {
            match key_event.code {
                KeyCode::Tab => {
                    // Move to next field
                    let field_count = LlmConfigForm::field_count();
                    app.llm_config_form.selected_field = (app.llm_config_form.selected_field + 1) % field_count;
                    return;
                }
                KeyCode::BackTab => {
                    // Move to previous field
                    let field_count = LlmConfigForm::field_count();
                    if app.llm_config_form.selected_field == 0 {
                        app.llm_config_form.selected_field = field_count - 1;
                    } else {
                        app.llm_config_form.selected_field -= 1;
                    }
                    return;
                }
                KeyCode::Up => {
                    // Previous field
                    if app.llm_config_form.selected_field > 0 {
                        app.llm_config_form.selected_field -= 1;
                    }
                    return;
                }
                KeyCode::Down => {
                    // Next field
                    let field_count = LlmConfigForm::field_count();
                    if app.llm_config_form.selected_field < field_count - 1 {
                        app.llm_config_form.selected_field += 1;
                    }
                    return;
                }
                KeyCode::Left if app.llm_config_form.selected_field == 0 => {
                    // Cycle through providers
                    let providers = LlmConfigForm::providers();
                    let current_idx = providers.iter().position(|&p| p == app.llm_config_form.provider).unwrap_or(0);
                    let new_idx = if current_idx == 0 { providers.len() - 1 } else { current_idx - 1 };
                    app.llm_config_form.provider = providers[new_idx].to_string();
                    return;
                }
                KeyCode::Right if app.llm_config_form.selected_field == 0 => {
                    // Cycle through providers
                    let providers = LlmConfigForm::providers();
                    let current_idx = providers.iter().position(|&p| p == app.llm_config_form.provider).unwrap_or(0);
                    let new_idx = (current_idx + 1) % providers.len();
                    app.llm_config_form.provider = providers[new_idx].to_string();
                    return;
                }
                KeyCode::Char(c) if app.llm_config_form.selected_field > 0 => {
                    // Add character to current field
                    let field = app.llm_config_form.get_field_value_mut(app.llm_config_form.selected_field);
                    field.push(c);
                    return;
                }
                KeyCode::Backspace if app.llm_config_form.selected_field > 0 => {
                    // Remove character from current field
                    let field = app.llm_config_form.get_field_value_mut(app.llm_config_form.selected_field);
                    field.pop();
                    return;
                }
                KeyCode::Enter => {
                    // Mark for async save
                    app.pending_llm_save = true;
                    app.dialog_state = DialogState::None;
                    return;
                }
                KeyCode::Esc => {
                    app.dialog_state = DialogState::None;
                    app.add_message("LLM configuration cancelled".to_string());
                    return;
                }
                _ => return,
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

        // Handle function keys F1-F5 for menu shortcuts (works on all platforms)
        match key_event.code {
            KeyCode::F(1) => {
                app.open_menu(MenuState::FileOpen);
                return;
            }
            KeyCode::F(2) => {
                app.open_menu(MenuState::EditOpen);
                return;
            }
            KeyCode::F(3) => {
                app.open_menu(MenuState::ConfigOpen);
                return;
            }
            KeyCode::F(4) => {
                app.open_menu(MenuState::WorkflowOpen);
                return;
            }
            KeyCode::F(5) => {
                app.open_menu(MenuState::HelpOpen);
                return;
            }
            _ => {}
        }

        // Handle Alt+F/E/C/W/H for menu shortcuts
        // On Windows/Linux: Alt+F/E/C/W/H
        // On macOS: Ctrl+Shift+F/E/C/W/H (Cmd is usually intercepted by terminal)
        let has_menu_modifier = key_event.modifiers.contains(KeyModifiers::ALT) ||
                                (key_event.modifiers.contains(KeyModifiers::CONTROL) &&
                                 key_event.modifiers.contains(KeyModifiers::SHIFT));
        if has_menu_modifier {
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
                // Close menu if it's open before navigating
                if app.menu_state != MenuState::Closed {
                    app.close_menu();
                } else {
                    app.next_focus();
                }
            }
            KeyCode::BackTab => {
                // Close menu if it's open before navigating
                if app.menu_state != MenuState::Closed {
                    app.close_menu();
                } else {
                    app.prev_focus();
                }
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

            // Enter on Patterns tab selects the pattern
            KeyCode::Enter if app.focused == FocusedArea::Sidebar && app.active_tab == SidebarTab::Patterns => {
                if !app.patterns.is_empty() {
                    app.select_pattern(app.sidebar_selected);
                    if let Some(ref pattern) = app.active_pattern {
                        app.add_message(format!("Pattern activated: {} ({})", pattern.name, pattern.pattern_type));
                    }
                }
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

            // Ctrl+L: Open LLM config form
            KeyCode::Char('l') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.dialog_state = DialogState::LlmProfileEdit;
                app.focused = FocusedArea::Menu;
                // Form will be loaded with current config by UI
            }

            // Ctrl+P: Open pattern selection
            KeyCode::Char('p') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.dialog_state = DialogState::PatternSelect;
                app.pending_pattern_load = true;
                app.focused = FocusedArea::Menu;
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
            // Show budget management options
            let budget_options = vec![
                "List Budgets".to_string(),
                "Create Budget".to_string(),
                "Edit Budget".to_string(),
                "Activate Budget".to_string(),
            ];
            let dialog = super::dialog::Dialog::select_list("Budget Management", budget_options);
            app.show_dialog(dialog);
            app.close_menu();
        }
        "config_llm_profile" => {
            // Open LLM configuration form directly
            app.dialog_state = DialogState::LlmProfileEdit;
            app.focused = FocusedArea::Menu;
            app.close_menu();
        }
        "config_pattern" => {
            // Open pattern selection dialog
            app.dialog_state = DialogState::PatternSelect;
            app.pending_pattern_load = true;
            app.focused = FocusedArea::Menu;
            app.close_menu();
        }
        "config_editor" => {
            // Check for EDITOR environment variable
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
            let config_file = if std::path::Path::new("./.orca/orca.toml").exists() {
                "./.orca/orca.toml (project)"
            } else {
                "~/.orca/orca.toml (user)"
            };

            let msg = format!(
                "Open configuration file in external editor:\n\n\
                Editor: {}\n\
                Config file: {}\n\n\
                Close editor to return to Orca.",
                editor,
                config_file
            );
            let dialog = super::dialog::Dialog::info("Open Editor", msg);
            app.show_dialog(dialog);
            app.close_menu();
        }

        // Workflow menu actions
        "workflow_run" => {
            let workflows = vec![
                "ReAct Agent - Data Analysis".to_string(),
                "Plan-Execute - Research Task".to_string(),
                "Reflection - Code Review".to_string(),
            ];
            let dialog = super::dialog::Dialog::select_list("Select Workflow to Run", workflows);
            app.show_dialog(dialog);
            app.close_menu();
        }
        "workflow_view" => {
            let msg = "Active Workflows:\n\n\
                1. ReAct Agent (Running)\n\
                   Status: Processing step 3/5\n\
                   Tokens: 2,456 used\n\n\
                2. Plan-Execute (Queued)\n\
                   Status: Waiting to start\n\
                   Tokens: 0 used\n\n\
                3. Reflection (Completed)\n\
                   Status: Finished successfully\n\
                   Tokens: 1,890 used";
            let dialog = super::dialog::Dialog::info("Workflows", msg);
            app.show_dialog(dialog);
            app.close_menu();
        }
        "workflow_create" => {
            let patterns = vec![
                "ReAct - Think → Act → Observe".to_string(),
                "Plan-Execute - Plan → Execute → Replan".to_string(),
                "Reflection - Generate → Critique → Refine".to_string(),
            ];
            let dialog = super::dialog::Dialog::select_list("Create New Workflow", patterns);
            app.show_dialog(dialog);
            app.close_menu();
        }
        "workflow_manage" => {
            let msg = "Workflow Management:\n\n\
                Total Workflows: 3\n\
                Active: 2\n\
                Completed: 1\n\n\
                Options:\n\
                • Edit workflow definition\n\
                • Delete workflow\n\
                • View execution history\n\
                • Configure parameters\n\n\
                Use workflow menu to manage.";
            let dialog = super::dialog::Dialog::info("Workflow Management", msg);
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
                "Alt+F - File Menu\nAlt+E - Edit Menu\nAlt+C - Config Menu\nAlt+W - Workflow Menu\nAlt+H - Help Menu\n\nTab - Switch focus\nUp/Down - Navigate\nEnter - Select\nEsc - Close/Quit\nCtrl+Enter - Submit prompt\nCtrl+C - Clear conversation\nCtrl+L - LLM Config\nCtrl+P - Pattern Select",
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

/// Helper: Show budget operation confirmation
fn _show_budget_operation(operation: &str, app: &mut App) {
    let msg = format!(
        "Budget {}:\n\n\
        Name: Sample Budget\n\
        Type: Credit\n\
        Amount: $500.00\n\
        Current Usage: $145.67\n\
        Status: 29% used\n\n\
        {} this budget?",
        operation.to_lowercase(),
        if operation == "Create" { "Create" } else { "Confirm" }
    );
    let dialog = super::dialog::Dialog::confirm(&format!("{} Budget", operation), msg);
    app.show_dialog(dialog);
}

/// Helper: Show LLM profile operation confirmation
fn _show_llm_operation(operation: &str, app: &mut App) {
    let msg = format!(
        "LLM Profile {}:\n\n\
        Name: Multi-Model\n\
        Planner: Claude-3-Opus\n\
        Worker: GPT-4\n\n\
        {} this profile?",
        operation.to_lowercase(),
        if operation == "Create" { "Create" } else { "Confirm" }
    );
    let dialog = super::dialog::Dialog::confirm(&format!("{} Profile", operation), msg);
    app.show_dialog(dialog);
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}
