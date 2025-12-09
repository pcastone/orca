//! Input event handling for TUI

use super::app::{App, DialogState, FocusedArea, MenuState, SidebarTab, ViewMode, ConfigSection, ConfigEditorForm, LlmFocusArea, LlmProfileEntry};
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

        // Handle config editor mode
        if app.view_mode == ViewMode::ConfigEditor {
            self.handle_config_editor_key(key_event, app);
            return;
        }

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

            // Submit prompt with Enter - send to LLM
            KeyCode::Enter if app.focused == FocusedArea::Prompts => {
                let prompt_text = app.get_prompt_text();
                if !prompt_text.trim().is_empty() {
                    // Display user message
                    app.add_message(format!("You:\n{}", prompt_text));
                    app.add_history(prompt_text.clone());
                    app.clear_prompt();

                    // Queue prompt for async LLM processing
                    app.pending_prompt_text = prompt_text;
                    app.pending_prompt_submit = true;
                }
            }

            // Clear conversation
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                if app.focused == FocusedArea::Conversation {
                    app.clear_conversation();
                }
            }

            // Ctrl+L: Open config editor with LLM section focused
            KeyCode::Char('l') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.view_mode = ViewMode::ConfigEditor;
                app.config_editor = super::app::ConfigEditorForm::default();
                app.config_editor.section = ConfigSection::Llm;
                app.config_editor.field_index = 0;
            }

            // Ctrl+P: Open pattern selection
            KeyCode::Char('p') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.dialog_state = DialogState::PatternSelect;
                app.pending_pattern_load = true;
                app.focused = FocusedArea::Menu;
            }

            // Ctrl+1: Focus Conversation area
            KeyCode::Char('1') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.close_menu();
                app.focused = FocusedArea::Conversation;
            }
            // Ctrl+2: Focus Prompts area
            KeyCode::Char('2') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.close_menu();
                app.focused = FocusedArea::Prompts;
            }
            // Ctrl+3: Focus Sidebar
            KeyCode::Char('3') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.close_menu();
                app.focused = FocusedArea::Sidebar;
            }
            // Ctrl+4: Toggle Config Editor view
            KeyCode::Char('4') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.close_menu();
                if app.view_mode == ViewMode::ConfigEditor {
                    app.view_mode = ViewMode::Conversation;
                } else {
                    app.view_mode = ViewMode::ConfigEditor;
                    app.config_editor = super::app::ConfigEditorForm::default();
                }
            }
            // Ctrl+Q: Quit application
            KeyCode::Char('q') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.state.should_quit = true;
            }

            // Quit with 'q' only when NOT in Prompts focus (so user can type 'q')
            KeyCode::Char('q') if app.focused != FocusedArea::Prompts => {
                app.state.should_quit = true;
            }
            // Quit with Esc when menu is not open and not in Prompts
            KeyCode::Esc if app.menu_state == MenuState::Closed && app.focused != FocusedArea::Prompts => {
                app.state.should_quit = true;
            }

            _ => {}
        }
    }

    /// Handle key events in config editor mode
    fn handle_config_editor_key(&self, key_event: KeyEvent, app: &mut App) {
        let form = &mut app.config_editor;

        // If editing a field
        if form.editing {
            match key_event.code {
                KeyCode::Enter => {
                    // Commit the edit
                    if form.section == ConfigSection::Llm {
                        // LLM section - edit the selected profile's field
                        let new_value = form.edit_buffer.clone();
                        let field_idx = form.llm_detail_field;
                        if let Some(profile) = form.selected_llm_profile_mut() {
                            if let Some(field) = profile.get_field_mut(field_idx) {
                                *field = new_value;
                                form.modified = true;
                            }
                        }
                    } else {
                        // Other sections - clone buffer first to avoid borrow issues
                        let new_value = form.edit_buffer.clone();
                        if let Some(field) = form.get_field_mut() {
                            *field = new_value;
                            form.modified = true;
                        }
                    }
                    form.editing = false;
                    form.edit_buffer.clear();
                }
                KeyCode::Esc => {
                    // Cancel edit
                    form.editing = false;
                    form.edit_buffer.clear();
                }
                KeyCode::Backspace => {
                    form.edit_buffer.pop();
                }
                KeyCode::Char(c) => {
                    form.edit_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        // Handle LLM section specially (table + detail view)
        if form.section == ConfigSection::Llm {
            self.handle_llm_section_key(key_event, app);
            return;
        }

        // Handle dropdown navigation if open (for Workflow section)
        if form.dropdown_open {
            match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => form.dropdown_prev(),
                KeyCode::Down | KeyCode::Char('j') => form.dropdown_next(),
                KeyCode::Enter => {
                    if form.section == ConfigSection::Workflow {
                        form.apply_workflow_dropdown_selection();
                    }
                }
                KeyCode::Esc => form.close_dropdown(),
                _ => {}
            }
            return;
        }

        // Not editing - handle navigation for other sections
        match key_event.code {
            // Navigation between sections
            KeyCode::Tab => {
                form.section = form.section.next();
                form.field_index = 0;
                form.button_focus = 0;
            }
            KeyCode::BackTab => {
                form.section = form.section.prev();
                form.field_index = 0;
                form.button_focus = 0;
            }

            // Field navigation
            KeyCode::Up | KeyCode::Char('k') => {
                if form.button_focus > 0 {
                    // Move from buttons back to fields
                    form.button_focus = 0;
                    form.field_index = form.field_count().saturating_sub(1);
                } else if form.field_index > 0 {
                    form.field_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if form.button_focus == 0 {
                    if form.field_index < form.field_count() - 1 {
                        form.field_index += 1;
                    } else {
                        // Move to Save button
                        form.button_focus = 1;
                    }
                }
            }

            // Button navigation
            KeyCode::Left | KeyCode::Char('h') if form.button_focus > 0 => {
                if form.button_focus > 1 {
                    form.button_focus -= 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') if form.button_focus > 0 => {
                if form.button_focus < 2 {
                    form.button_focus += 1;
                }
            }

            // Space to toggle boolean fields
            KeyCode::Char(' ') if form.button_focus == 0 => {
                if form.is_bool_field(form.field_index) {
                    form.toggle_bool_field();
                }
            }

            // Enter to edit field or activate button
            KeyCode::Enter => {
                if form.button_focus == 1 {
                    // Save button - mark pending save
                    app.pending_config_save = true;
                } else if form.button_focus == 2 {
                    // Cancel button
                    app.close_config_editor();
                } else if form.section == ConfigSection::Workflow
                    && ConfigEditorForm::is_workflow_dropdown_field(form.field_index) {
                    // Open dropdown for workflow planner/worker fields
                    form.open_workflow_dropdown();
                } else if !form.is_bool_field(form.field_index) {
                    // Start editing the field
                    if let Some(field) = form.get_field_mut() {
                        form.edit_buffer = field.clone();
                        form.editing = true;
                    }
                } else {
                    // Toggle boolean field
                    form.toggle_bool_field();
                }
            }

            // Escape to cancel/close
            KeyCode::Esc => {
                app.close_config_editor();
            }

            // Ctrl+S to save
            KeyCode::Char('s') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.pending_config_save = true;
            }

            _ => {}
        }
    }

    /// Handle key events for LLM section (table + detail view)
    fn handle_llm_section_key(&self, key_event: KeyEvent, app: &mut App) {
        let form = &mut app.config_editor;

        // Handle dropdown navigation if dropdown is open
        if form.dropdown_open {
            match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    form.dropdown_prev();
                    return;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    form.dropdown_next();
                    return;
                }
                KeyCode::Enter => {
                    form.apply_dropdown_selection();
                    return;
                }
                KeyCode::Esc => {
                    form.close_dropdown();
                    return;
                }
                _ => return,  // Ignore other keys when dropdown is open
            }
        }

        match key_event.code {
            // Tab: Switch between table and detail, or to next section
            KeyCode::Tab => {
                if form.button_focus > 0 {
                    // From buttons to next section
                    form.section = form.section.next();
                    form.field_index = 0;
                    form.button_focus = 0;
                } else if form.llm_focus == LlmFocusArea::Table {
                    // Switch to detail view
                    form.llm_focus = LlmFocusArea::Detail;
                    form.llm_detail_field = 0;
                } else {
                    // From detail, move to buttons
                    form.button_focus = 1;
                    form.llm_focus = LlmFocusArea::Table;  // Reset for next Tab cycle
                }
            }
            KeyCode::BackTab => {
                if form.button_focus > 0 {
                    // From buttons back to detail
                    form.button_focus = 0;
                    form.llm_focus = LlmFocusArea::Detail;
                } else if form.llm_focus == LlmFocusArea::Detail {
                    // Switch back to table
                    form.llm_focus = LlmFocusArea::Table;
                } else {
                    // From table to previous section
                    form.section = form.section.prev();
                    form.field_index = 0;
                    form.button_focus = 0;
                }
            }

            // Up/Down navigation
            KeyCode::Up | KeyCode::Char('k') => {
                if form.button_focus > 0 {
                    // Move from buttons back to content
                    form.button_focus = 0;
                } else if form.llm_focus == LlmFocusArea::Table {
                    // Navigate up in table
                    if form.llm_selected_index > 0 {
                        form.llm_selected_index -= 1;
                    }
                } else {
                    // Navigate up in detail form
                    if form.llm_detail_field > 0 {
                        form.llm_detail_field -= 1;
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if form.button_focus == 0 {
                    if form.llm_focus == LlmFocusArea::Table {
                        // Navigate down in table
                        if form.llm_selected_index < form.llm_profiles.len().saturating_sub(1) {
                            form.llm_selected_index += 1;
                        }
                    } else {
                        // Navigate down in detail form
                        if form.llm_detail_field < LlmProfileEntry::field_count() - 1 {
                            form.llm_detail_field += 1;
                        } else {
                            // Move to Save button
                            form.button_focus = 1;
                        }
                    }
                }
            }

            // Button navigation
            KeyCode::Left | KeyCode::Char('h') if form.button_focus > 0 => {
                if form.button_focus > 1 {
                    form.button_focus -= 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') if form.button_focus > 0 => {
                if form.button_focus < 2 {
                    form.button_focus += 1;
                }
            }

            // Space to toggle boolean fields in detail view
            KeyCode::Char(' ') if form.button_focus == 0 => {
                if form.llm_focus == LlmFocusArea::Detail {
                    let field_idx = form.llm_detail_field;
                    if LlmProfileEntry::is_bool_field(field_idx) {
                        if let Some(profile) = form.selected_llm_profile_mut() {
                            profile.toggle_bool(field_idx);
                            form.modified = true;
                        }
                    }
                }
            }

            // 'n' to add new profile
            KeyCode::Char('n') if form.button_focus == 0 && form.llm_focus == LlmFocusArea::Table => {
                form.add_llm_profile();
            }

            // 'c' to copy selected profile
            KeyCode::Char('c') if form.button_focus == 0 && form.llm_focus == LlmFocusArea::Table => {
                form.copy_llm_profile();
            }

            // 'd' to delete selected profile
            KeyCode::Char('d') if form.button_focus == 0 && form.llm_focus == LlmFocusArea::Table => {
                form.delete_selected_llm_profile();
            }

            // Enter to edit field or activate button
            KeyCode::Enter => {
                if form.button_focus == 1 {
                    // Save button - LLM config goes to database, not file
                    app.pending_llm_save = true;
                } else if form.button_focus == 2 {
                    // Cancel button
                    app.close_config_editor();
                } else if form.llm_focus == LlmFocusArea::Table {
                    // From table, switch to detail to edit
                    form.llm_focus = LlmFocusArea::Detail;
                    form.llm_detail_field = 0;
                } else {
                    // In detail view
                    let field_idx = form.llm_detail_field;

                    if LlmProfileEntry::is_bool_field(field_idx) {
                        // Toggle boolean field
                        if let Some(profile) = form.selected_llm_profile_mut() {
                            profile.toggle_bool(field_idx);
                            form.modified = true;
                        }
                    } else if LlmProfileEntry::is_dropdown_field(field_idx) {
                        // Open dropdown for dropdown fields (Provider, Model, Workflow, Budget)
                        form.open_dropdown();
                        // For model field, trigger async query to provider
                        if field_idx == 2 {
                            app.pending_model_query = true;
                        }
                    } else {
                        // Start text editing for regular fields
                        if let Some(profile) = form.selected_llm_profile() {
                            form.edit_buffer = profile.field_value(field_idx);
                            // Remove display-only values
                            if form.edit_buffer == "(not set)" || form.edit_buffer == "(default)" || form.edit_buffer == "(none)" {
                                form.edit_buffer.clear();
                            }
                            form.editing = true;
                        }
                    }
                }
            }

            // Escape to cancel/close
            KeyCode::Esc => {
                app.close_config_editor();
            }

            // Ctrl+S to save LLM config to database
            KeyCode::Char('s') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                app.pending_llm_save = true;
            }

            _ => {}
        }
    }
}

/// Execute menu action
fn execute_menu_action(action: &str, app: &mut App) {
    match action {
        // File menu actions
        "file_init" => {
            // Initialize - scan project and set up database
            let dialog = super::dialog::Dialog::info("Init", "Scanning project and initializing database...\n\nRun 'orca_install init' to set up defaults.");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "file_update" => {
            // Update - rescan YAML files and update database
            let dialog = super::dialog::Dialog::info("Update", "Scanning YAML files and updating database...\n\nThis will sync workflows, patterns, and prompts.");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "file_save" => {
            // Show save dialog
            let dialog = super::dialog::Dialog::info("Save", "Conversation saved!");
            app.show_dialog(dialog);
            app.close_menu();
        }
        "file_backup" => {
            // Trigger backup operation
            app.pending_backup = true;
            app.close_menu();
        }
        "file_restore" => {
            // Trigger restore operation (show backup list first)
            app.pending_restore = true;
            app.close_menu();
        }
        "file_export" => {
            // Trigger export operation
            app.pending_export = true;
            app.close_menu();
        }
        "file_import" => {
            // Trigger import operation
            app.pending_import = true;
            app.close_menu();
        }
        "file_quit" => {
            app.state.should_quit = true;
            app.close_menu();
        }

        // Edit menu actions (AST operations)
        "edit_build" => {
            let dialog = super::dialog::Dialog::info(
                "AST Build",
                "Building initial AST index for project...\n\nThis will parse all supported source files\n(Rust, Python) and extract symbols, imports,\nand code structure."
            );
            app.show_dialog(dialog);
            app.close_menu();
            // TODO: Trigger actual AST build via AstService
        }
        "edit_update" => {
            let dialog = super::dialog::Dialog::info(
                "AST Update",
                "Scanning for modified files...\n\nThis will detect changed files by comparing\ncontent hashes and re-parse only those files."
            );
            app.show_dialog(dialog);
            app.close_menu();
            // TODO: Trigger actual AST update via AstService
        }
        "edit_refine" => {
            let dialog = super::dialog::Dialog::info(
                "AST Refine",
                "Performing deep semantic analysis...\n\nThis extracts:\n- Call graphs (what calls what)\n- Type information\n- Cross-references (where symbols are used)"
            );
            app.show_dialog(dialog);
            app.close_menu();
            // TODO: Trigger actual AST refinement via AstService
        }
        "edit_purge" => {
            let dialog = super::dialog::Dialog::info(
                "AST Purge",
                "Removing refined AST data...\n\nThis clears deep semantic analysis while\nkeeping the base AST index intact."
            );
            app.show_dialog(dialog);
            app.close_menu();
            // TODO: Trigger actual AST purge via AstService
        }
        "edit_search" => {
            let dialog = super::dialog::Dialog::info(
                "AST Search",
                "Search across indexed code...\n\nSupports:\n- Symbol search (exact name)\n- Fuzzy search (approximate match)\n- Semantic search (with cross-refs)"
            );
            app.show_dialog(dialog);
            app.close_menu();
            // TODO: Show search input dialog and trigger search
        }

        // Config menu actions
        "config_view" => {
            // Open config editor - mark as pending so async code can handle it
            app.pending_config_save = false;  // Reset pending flag
            app.view_mode = ViewMode::ConfigEditor;
            app.close_menu();
            // Note: Config will be loaded in the main loop when pending_config_save is checked
            // For now, reset the editor state
            app.config_editor = super::app::ConfigEditorForm::default();
            app.config_editor.field_index = 0;
            app.config_editor.button_focus = 0;
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
                "Navigation:\nCtrl+1 - Focus Conversation\nCtrl+2 - Focus Prompts\nCtrl+3 - Focus Sidebar\nCtrl+4 - Toggle Config Editor\nCtrl+Q - Quit\n\nMenus:\nAlt+F - File | Alt+E - Edit | Alt+C - Config\nAlt+W - Workflow | Alt+H - Help | F1-F5\n\nGeneral:\nTab - Switch focus | Up/Down - Navigate\nEnter - Select/Submit | Esc - Close/Quit\nCtrl+C - Clear | Ctrl+L - Config Editor | Ctrl+P - Pattern",
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

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}
