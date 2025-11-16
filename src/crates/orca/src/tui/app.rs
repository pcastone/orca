//! Application state management for TUI

use crate::HealthReport;
use std::collections::VecDeque;

/// Maximum number of log entries to keep
const MAX_LOGS: usize = 1000;

/// Which panel is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Status,
    Commands,
    Output,
    Logs,
}

/// Application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub should_quit: bool,
}

/// Main application structure
pub struct App {
    pub state: AppState,
    pub focused: FocusedPanel,
    pub selected_command: usize,
    pub health_report: Option<HealthReport>,
    pub command_output: String,
    pub logs: VecDeque<String>,
    pub scroll_offset: u16,
}

impl App {
    /// Create a new app instance
    pub fn new() -> Self {
        Self {
            state: AppState {
                should_quit: false,
            },
            focused: FocusedPanel::Commands,
            selected_command: 0,
            health_report: None,
            command_output: String::new(),
            logs: VecDeque::new(),
            scroll_offset: 0,
        }
    }

    /// Add a log entry
    pub fn add_log(&mut self, message: String) {
        self.logs.push_back(message);
        // Keep only the most recent entries
        while self.logs.len() > MAX_LOGS {
            self.logs.pop_front();
        }
    }

    /// Move to next panel
    pub fn next_panel(&mut self) {
        self.focused = match self.focused {
            FocusedPanel::Status => FocusedPanel::Commands,
            FocusedPanel::Commands => FocusedPanel::Output,
            FocusedPanel::Output => FocusedPanel::Logs,
            FocusedPanel::Logs => FocusedPanel::Status,
        };
    }

    /// Move to previous panel
    pub fn prev_panel(&mut self) {
        self.focused = match self.focused {
            FocusedPanel::Status => FocusedPanel::Logs,
            FocusedPanel::Commands => FocusedPanel::Status,
            FocusedPanel::Output => FocusedPanel::Commands,
            FocusedPanel::Logs => FocusedPanel::Output,
        };
    }

    /// Select next command in list
    pub fn next_command(&mut self) {
        self.selected_command = self.selected_command.saturating_add(1);
    }

    /// Select previous command in list
    pub fn prev_command(&mut self) {
        self.selected_command = self.selected_command.saturating_sub(1);
    }

    /// Scroll logs down
    pub fn scroll_logs_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    /// Scroll logs up
    pub fn scroll_logs_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Clear command output
    pub fn clear_output(&mut self) {
        self.command_output.clear();
    }

    /// Clear logs
    pub fn clear_logs(&mut self) {
        self.logs.clear();
        self.scroll_offset = 0;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
