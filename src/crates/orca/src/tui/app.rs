//! Application state management for TUI

use crate::HealthReport;
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
        };
    }

    /// Move focus to previous area
    pub fn prev_focus(&mut self) {
        self.focused = match self.focused {
            FocusedArea::Conversation => FocusedArea::Sidebar,
            FocusedArea::Prompts => FocusedArea::Conversation,
            FocusedArea::Sidebar => FocusedArea::Prompts,
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
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
