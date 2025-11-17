//! Application state management for TUI

use crate::HealthReport;
use super::dialog::Dialog;
use std::collections::VecDeque;

/// Maximum number of conversation/log entries to keep
const MAX_ENTRIES: usize = 1000;

/// Which sidebar tab is currently active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    History,
    Todo,
    Bugs,
}

/// Which area is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedArea {
    Conversation,
    Prompts,
    Sidebar,
    Menu,
}

/// Menu bar state - which menu is open (if any)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuState {
    Closed,
    FileOpen,
    EditOpen,
    ConfigOpen,
    WorkflowOpen,
    HelpOpen,
}

/// Dialog state - what dialog is currently open
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogState {
    None,
    BudgetList,
    BudgetCreate,
    BudgetEdit,
    LlmProfileList,
    LlmProfileCreate,
    LlmProfileEdit,
    ConfigViewer,
    ExternalEditor,
}

/// Application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub should_quit: bool,
}

/// Main application structure
pub struct App {
    pub state: AppState,
    pub focused: FocusedArea,
    pub active_tab: SidebarTab,
    pub health_report: Option<HealthReport>,

    // Left side: conversation and prompts
    pub conversation: VecDeque<String>,
    pub prompt_lines: Vec<String>,
    pub prompt_cursor_line: usize,
    pub prompt_cursor_col: usize,
    pub conversation_scroll: u16,

    // Right sidebar content
    pub history: VecDeque<String>,
    pub todo_items: VecDeque<String>,
    pub bugs: VecDeque<String>,
    pub sidebar_selected: usize,
    pub sidebar_scroll: u16,

    // Status bar info
    pub current_model: String,
    pub tokens_used: u32,
    pub runtime: String,

    // Budget tracking
    pub active_budget: Option<String>,
    pub budget_usage: f64,
    pub budget_remaining: Option<f64>,
    pub budget_status: String,

    // LLM profile tracking
    pub llm_profile: Option<String>,
    pub planner_llm: Option<String>,
    pub worker_llm: Option<String>,

    // Menu management
    pub menu_state: MenuState,
    pub menu_selected_index: usize,
    pub dialog_state: DialogState,
    pub dialog: Option<Dialog>,
}

impl App {
    /// Create a new app instance
    pub fn new() -> Self {
        Self {
            state: AppState {
                should_quit: false,
            },
            focused: FocusedArea::Conversation,
            active_tab: SidebarTab::History,
            health_report: None,
            conversation: VecDeque::new(),
            prompt_lines: vec![String::new()],
            prompt_cursor_line: 0,
            prompt_cursor_col: 0,
            conversation_scroll: 0,
            history: VecDeque::new(),
            todo_items: VecDeque::new(),
            bugs: VecDeque::new(),
            sidebar_selected: 0,
            sidebar_scroll: 0,
            current_model: "claude-3-5-sonnet".to_string(),
            tokens_used: 0,
            runtime: "0ms".to_string(),
            active_budget: None,
            budget_usage: 0.0,
            budget_remaining: None,
            budget_status: "No budget".to_string(),
            llm_profile: None,
            planner_llm: None,
            worker_llm: None,
            menu_state: MenuState::Closed,
            menu_selected_index: 0,
            dialog_state: DialogState::None,
            dialog: None,
        }
    }

    /// Add a message to conversation
    pub fn add_message(&mut self, message: String) {
        self.conversation.push_back(message);
        while self.conversation.len() > MAX_ENTRIES {
            self.conversation.pop_front();
        }
    }

    /// Add to history
    pub fn add_history(&mut self, entry: String) {
        self.history.push_back(entry);
        while self.history.len() > MAX_ENTRIES {
            self.history.pop_front();
        }
    }

    /// Add todo item
    pub fn add_todo(&mut self, item: String) {
        self.todo_items.push_back(item);
    }

    /// Add bug
    pub fn add_bug(&mut self, bug: String) {
        self.bugs.push_back(bug);
    }

    /// Switch sidebar tab
    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            SidebarTab::History => SidebarTab::Todo,
            SidebarTab::Todo => SidebarTab::Bugs,
            SidebarTab::Bugs => SidebarTab::History,
        };
        self.sidebar_selected = 0;
        self.sidebar_scroll = 0;
    }

    /// Switch to previous tab
    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            SidebarTab::History => SidebarTab::Bugs,
            SidebarTab::Todo => SidebarTab::History,
            SidebarTab::Bugs => SidebarTab::Todo,
        };
        self.sidebar_selected = 0;
        self.sidebar_scroll = 0;
    }

    /// Move focus between areas
    pub fn next_focus(&mut self) {
        self.focused = match self.focused {
            FocusedArea::Conversation => FocusedArea::Prompts,
            FocusedArea::Prompts => FocusedArea::Sidebar,
            FocusedArea::Sidebar => FocusedArea::Conversation,
            FocusedArea::Menu => FocusedArea::Conversation,
        };
    }

    /// Move focus to previous area
    pub fn prev_focus(&mut self) {
        self.focused = match self.focused {
            FocusedArea::Conversation => FocusedArea::Sidebar,
            FocusedArea::Prompts => FocusedArea::Conversation,
            FocusedArea::Sidebar => FocusedArea::Prompts,
            FocusedArea::Menu => FocusedArea::Sidebar,
        };
    }

    /// Scroll conversation down
    pub fn scroll_conversation_down(&mut self) {
        self.conversation_scroll = self.conversation_scroll.saturating_add(1);
    }

    /// Scroll conversation up
    pub fn scroll_conversation_up(&mut self) {
        self.conversation_scroll = self.conversation_scroll.saturating_sub(1);
    }

    /// Scroll sidebar down
    pub fn scroll_sidebar_down(&mut self) {
        self.sidebar_scroll = self.sidebar_scroll.saturating_add(1);
    }

    /// Scroll sidebar up
    pub fn scroll_sidebar_up(&mut self) {
        self.sidebar_scroll = self.sidebar_scroll.saturating_sub(1);
    }

    /// Move sidebar selection down
    pub fn sidebar_next(&mut self) {
        self.sidebar_selected = self.sidebar_selected.saturating_add(1);
    }

    /// Move sidebar selection up
    pub fn sidebar_prev(&mut self) {
        self.sidebar_selected = self.sidebar_selected.saturating_sub(1);
    }

    /// Clear conversation
    pub fn clear_conversation(&mut self) {
        self.conversation.clear();
        self.conversation_scroll = 0;
    }

    /// Get full prompt text
    pub fn get_prompt_text(&self) -> String {
        self.prompt_lines.join("\n")
    }

    /// Add character to prompt at cursor position
    pub fn add_prompt_char(&mut self, c: char) {
        if self.prompt_cursor_line < self.prompt_lines.len() {
            self.prompt_lines[self.prompt_cursor_line].insert(self.prompt_cursor_col, c);
            self.prompt_cursor_col += 1;
        }
    }

    /// Remove character before cursor in prompt
    pub fn backspace_prompt(&mut self) {
        if self.prompt_cursor_line < self.prompt_lines.len() {
            if self.prompt_cursor_col > 0 {
                self.prompt_lines[self.prompt_cursor_line].remove(self.prompt_cursor_col - 1);
                self.prompt_cursor_col -= 1;
            } else if self.prompt_cursor_line > 0 {
                // Move to end of previous line
                let line = self.prompt_lines.remove(self.prompt_cursor_line);
                self.prompt_cursor_line -= 1;
                self.prompt_cursor_col = self.prompt_lines[self.prompt_cursor_line].len();
                self.prompt_lines[self.prompt_cursor_line].push_str(&line);
            }
        }
    }

    /// Add newline in prompt (max 3 lines)
    pub fn newline_prompt(&mut self) {
        if self.prompt_lines.len() < 3 && self.prompt_cursor_line < self.prompt_lines.len() {
            let rest = self.prompt_lines[self.prompt_cursor_line].split_off(self.prompt_cursor_col);
            self.prompt_lines.insert(self.prompt_cursor_line + 1, rest);
            self.prompt_cursor_line += 1;
            self.prompt_cursor_col = 0;
        }
    }

    /// Move cursor left
    pub fn prompt_cursor_left(&mut self) {
        if self.prompt_cursor_col > 0 {
            self.prompt_cursor_col -= 1;
        } else if self.prompt_cursor_line > 0 {
            self.prompt_cursor_line -= 1;
            self.prompt_cursor_col = self.prompt_lines[self.prompt_cursor_line].len();
        }
    }

    /// Move cursor right
    pub fn prompt_cursor_right(&mut self) {
        if self.prompt_cursor_line < self.prompt_lines.len() {
            let line_len = self.prompt_lines[self.prompt_cursor_line].len();
            if self.prompt_cursor_col < line_len {
                self.prompt_cursor_col += 1;
            } else if self.prompt_cursor_line < self.prompt_lines.len() - 1 {
                self.prompt_cursor_line += 1;
                self.prompt_cursor_col = 0;
            }
        }
    }

    /// Clear prompt
    pub fn clear_prompt(&mut self) {
        self.prompt_lines = vec![String::new()];
        self.prompt_cursor_line = 0;
        self.prompt_cursor_col = 0;
    }

    /// Set active budget and usage information
    pub fn set_budget(&mut self, name: String, usage: f64, remaining: Option<f64>) {
        self.active_budget = Some(name);
        self.budget_usage = usage;
        self.budget_remaining = remaining;
        self.budget_status = if usage >= 100.0 {
            "Budget exceeded".to_string()
        } else if usage >= 80.0 {
            "Budget near limit".to_string()
        } else {
            "Budget OK".to_string()
        };
    }

    /// Clear active budget
    pub fn clear_budget(&mut self) {
        self.active_budget = None;
        self.budget_usage = 0.0;
        self.budget_remaining = None;
        self.budget_status = "No budget".to_string();
    }

    /// Set LLM profile configuration
    pub fn set_llm_profile(
        &mut self,
        profile_name: Option<String>,
        planner: Option<String>,
        worker: Option<String>,
    ) {
        self.llm_profile = profile_name;
        self.planner_llm = planner;
        self.worker_llm = worker;
    }

    /// Clear LLM profile configuration
    pub fn clear_llm_profile(&mut self) {
        self.llm_profile = None;
        self.planner_llm = None;
        self.worker_llm = None;
    }

    // === Menu Management Methods ===

    /// Open a menu
    pub fn open_menu(&mut self, menu: MenuState) {
        self.menu_state = menu;
        self.menu_selected_index = 0;
        self.focused = FocusedArea::Menu;
    }

    /// Close the current menu
    pub fn close_menu(&mut self) {
        self.menu_state = MenuState::Closed;
        self.menu_selected_index = 0;
        self.focused = FocusedArea::Conversation;
    }

    /// Move to next menu item
    pub fn menu_next(&mut self) {
        let max_items = self.get_menu_items_count();
        if max_items > 0 {
            self.menu_selected_index = (self.menu_selected_index + 1) % max_items;
        }
    }

    /// Move to previous menu item
    pub fn menu_prev(&mut self) {
        let max_items = self.get_menu_items_count();
        if max_items > 0 {
            self.menu_selected_index = if self.menu_selected_index > 0 {
                self.menu_selected_index - 1
            } else {
                max_items - 1
            };
        }
    }

    /// Get the count of items in the current menu
    fn get_menu_items_count(&self) -> usize {
        match self.menu_state {
            MenuState::Closed => 0,
            MenuState::FileOpen => 4,      // New, Open, Save, Quit
            MenuState::EditOpen => 3,      // Clear, Copy, Preferences
            MenuState::ConfigOpen => 4,    // View Config, Budget, LLM Profile, Editor
            MenuState::WorkflowOpen => 4,  // Run, View, Create, Manage
            MenuState::HelpOpen => 3,      // About, Shortcuts, Documentation
        }
    }

    /// Get the selected menu item action
    pub fn get_selected_menu_action(&self) -> Option<String> {
        match self.menu_state {
            MenuState::Closed => None,
            MenuState::FileOpen => match self.menu_selected_index {
                0 => Some("file_new".to_string()),
                1 => Some("file_open".to_string()),
                2 => Some("file_save".to_string()),
                3 => Some("file_quit".to_string()),
                _ => None,
            },
            MenuState::EditOpen => match self.menu_selected_index {
                0 => Some("edit_clear".to_string()),
                1 => Some("edit_copy".to_string()),
                2 => Some("edit_preferences".to_string()),
                _ => None,
            },
            MenuState::ConfigOpen => match self.menu_selected_index {
                0 => Some("config_view".to_string()),
                1 => Some("config_budget".to_string()),
                2 => Some("config_llm_profile".to_string()),
                3 => Some("config_editor".to_string()),
                _ => None,
            },
            MenuState::WorkflowOpen => match self.menu_selected_index {
                0 => Some("workflow_run".to_string()),
                1 => Some("workflow_view".to_string()),
                2 => Some("workflow_create".to_string()),
                3 => Some("workflow_manage".to_string()),
                _ => None,
            },
            MenuState::HelpOpen => match self.menu_selected_index {
                0 => Some("help_about".to_string()),
                1 => Some("help_shortcuts".to_string()),
                2 => Some("help_documentation".to_string()),
                _ => None,
            },
        }
    }

    // === Dialog Management Methods ===

    /// Show a dialog
    pub fn show_dialog(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
        self.focused = FocusedArea::Menu; // Change focus to dialog
    }

    /// Close the current dialog
    pub fn close_dialog(&mut self) {
        self.dialog = None;
        self.dialog_state = DialogState::None;
    }

    /// Navigate up in dialog (for list and confirmation dialogs)
    pub fn dialog_prev(&mut self) {
        if let Some(ref mut dialog) = self.dialog {
            dialog.select_prev();
        }
    }

    /// Navigate down in dialog (for list and confirmation dialogs)
    pub fn dialog_next(&mut self) {
        if let Some(ref mut dialog) = self.dialog {
            dialog.select_next();
        }
    }

    /// Add character to dialog input
    pub fn dialog_add_char(&mut self, c: char) {
        if let Some(ref mut dialog) = self.dialog {
            dialog.add_char(c);
        }
    }

    /// Backspace in dialog input
    pub fn dialog_backspace(&mut self) {
        if let Some(ref mut dialog) = self.dialog {
            dialog.backspace();
        }
    }

    /// Get selected option from dialog
    pub fn dialog_selected_option(&self) -> Option<&str> {
        self.dialog.as_ref().and_then(|d| d.selected_option())
    }

    /// Get input from text input dialog
    pub fn dialog_get_input(&self) -> Option<String> {
        self.dialog.as_ref().map(|d| d.get_input())
    }

    /// Check if dialog is open
    pub fn has_dialog(&self) -> bool {
        self.dialog.is_some()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
