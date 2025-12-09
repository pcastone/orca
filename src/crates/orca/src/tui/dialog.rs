//! Reusable dialog components for TUI

use ratatui::{
    prelude::*,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

/// Dialog types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogType {
    Information,
    Confirmation,
    TextInput,
    SelectList,
}

/// Reusable dialog component
#[derive(Debug, Clone)]
pub struct Dialog {
    pub title: String,
    pub dialog_type: DialogType,
    pub message: String,
    pub options: Vec<String>,
    pub selected_index: usize,
    pub input_text: String,
    pub input_cursor: usize,
}

impl Dialog {
    /// Create a new information dialog
    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            dialog_type: DialogType::Information,
            message: message.into(),
            options: vec!["OK".to_string()],
            selected_index: 0,
            input_text: String::new(),
            input_cursor: 0,
        }
    }

    /// Create a new confirmation dialog
    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            dialog_type: DialogType::Confirmation,
            message: message.into(),
            options: vec!["Yes".to_string(), "No".to_string()],
            selected_index: 0,
            input_text: String::new(),
            input_cursor: 0,
        }
    }

    /// Create a new text input dialog
    pub fn text_input(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            dialog_type: DialogType::TextInput,
            message: message.into(),
            options: vec!["OK".to_string(), "Cancel".to_string()],
            selected_index: 0,
            input_text: String::new(),
            input_cursor: 0,
        }
    }

    /// Create a new select list dialog
    pub fn select_list(title: impl Into<String>, items: Vec<String>) -> Self {
        Self {
            title: title.into(),
            dialog_type: DialogType::SelectList,
            message: String::new(),
            options: items,
            selected_index: 0,
            input_text: String::new(),
            input_cursor: 0,
        }
    }

    /// Move selection up in list dialogs
    pub fn select_prev(&mut self) {
        if !self.options.is_empty() {
            self.selected_index = if self.selected_index > 0 {
                self.selected_index - 1
            } else {
                self.options.len() - 1
            };
        }
    }

    /// Move selection down in list dialogs
    pub fn select_next(&mut self) {
        if !self.options.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.options.len();
        }
    }

    /// Add character to input text
    pub fn add_char(&mut self, c: char) {
        if self.dialog_type == DialogType::TextInput {
            self.input_text.insert(self.input_cursor, c);
            self.input_cursor += 1;
        }
    }

    /// Backspace in input text
    pub fn backspace(&mut self) {
        if self.dialog_type == DialogType::TextInput && self.input_cursor > 0 {
            self.input_text.remove(self.input_cursor - 1);
            self.input_cursor -= 1;
        }
    }

    /// Move cursor left in input text
    pub fn cursor_left(&mut self) {
        if self.dialog_type == DialogType::TextInput && self.input_cursor > 0 {
            self.input_cursor -= 1;
        }
    }

    /// Move cursor right in input text
    pub fn cursor_right(&mut self) {
        if self.dialog_type == DialogType::TextInput && self.input_cursor < self.input_text.len() {
            self.input_cursor += 1;
        }
    }

    /// Get the currently selected option
    pub fn selected_option(&self) -> Option<&str> {
        self.options.get(self.selected_index).map(|s| s.as_str())
    }

    /// Get the input text
    pub fn get_input(&self) -> String {
        self.input_text.clone()
    }
}

/// Render a dialog centered on screen
pub fn render_dialog(f: &mut Frame, dialog: &Dialog) {
    let screen_area = f.area();

    // Calculate dialog size based on content
    let title_len = dialog.title.len();

    // For multi-line messages, find the longest line
    let max_line_len = dialog.message
        .lines()
        .map(|l| l.len())
        .max()
        .unwrap_or(0);

    let _max_option_len = dialog.options.iter().map(|s| s.len()).max().unwrap_or(5);
    let buttons_width = dialog.options.iter().map(|s| s.len() + 2).sum::<usize>() + (dialog.options.len() - 1) * 3;

    // Calculate width but clamp to screen width minus margin
    let max_width = screen_area.width.saturating_sub(4);
    let desired_width = std::cmp::max(
        std::cmp::max(title_len + 4, max_line_len + 4),
        std::cmp::max(buttons_width + 4, 30),
    ) as u16;
    let width = std::cmp::min(desired_width, max_width);

    // Calculate height based on message lines for Information dialogs
    let message_lines = dialog.message.lines().count();
    let height = match dialog.dialog_type {
        DialogType::Information => {
            // Base height (borders + button) + message lines, clamped
            let base_height = 4u16;
            let content_height = (message_lines as u16).saturating_add(base_height);
            std::cmp::min(content_height, screen_area.height.saturating_sub(4))
        }
        DialogType::Confirmation => 7,
        DialogType::TextInput => 8,
        DialogType::SelectList => {
            std::cmp::min((dialog.options.len() as u16) + 4, 15)
        }
    };

    // Ensure minimum dimensions
    let width = std::cmp::max(width, 10);
    let height = std::cmp::max(height, 5);

    // Center dialog on screen, ensuring it fits
    let x = (screen_area.width.saturating_sub(width)) / 2;
    let y = (screen_area.height.saturating_sub(height)) / 2;

    // Final safety clamp to ensure we don't exceed screen bounds
    let final_width = std::cmp::min(width, screen_area.width.saturating_sub(x));
    let final_height = std::cmp::min(height, screen_area.height.saturating_sub(y));

    let dialog_area = Rect {
        x,
        y,
        width: final_width,
        height: final_height,
    };

    // Clear background
    f.render_widget(Clear, dialog_area);

    // Render dialog box with color-coded border
    let (border_color, icon) = match dialog.dialog_type {
        DialogType::Information => (Color::Green, "ℹ"),
        DialogType::Confirmation => (Color::Yellow, "?"),
        DialogType::TextInput => (Color::Cyan, "✎"),
        DialogType::SelectList => (Color::Blue, "◆"),
    };

    let title_with_icon = format!("{} {}", icon, dialog.title);
    let block = Block::default()
        .title(title_with_icon)
        .borders(Borders::ALL)
        .style(Style::default().fg(border_color).bold());

    f.render_widget(block, dialog_area);

    // Calculate inner area for content
    let inner_area = Rect {
        x: dialog_area.x + 1,
        y: dialog_area.y + 1,
        width: dialog_area.width.saturating_sub(2),
        height: dialog_area.height.saturating_sub(2),
    };

    match dialog.dialog_type {
        DialogType::Information => render_info_dialog(f, dialog, inner_area),
        DialogType::Confirmation => render_confirmation_dialog(f, dialog, inner_area),
        DialogType::TextInput => render_text_input_dialog(f, dialog, inner_area),
        DialogType::SelectList => render_select_list_dialog(f, dialog, inner_area),
    }
}

/// Render information dialog
fn render_info_dialog(f: &mut Frame, dialog: &Dialog, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),
            Constraint::Length(1),
        ])
        .split(area);

    // Message
    let message = Paragraph::new(dialog.message.as_str())
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(message, chunks[0]);

    // OK button with styling
    let ok_button = Paragraph::new("  [ OK ]  ")
        .style(Style::default().bg(Color::Green).fg(Color::Black).bold())
        .alignment(Alignment::Center);
    f.render_widget(ok_button, chunks[1]);
}

/// Render confirmation dialog
fn render_confirmation_dialog(f: &mut Frame, dialog: &Dialog, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),
            Constraint::Length(1),
        ])
        .split(area);

    // Message
    let message = Paragraph::new(dialog.message.as_str())
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(message, chunks[0]);

    // Buttons with color-coded selected state
    let mut spans = Vec::new();
    for (idx, option) in dialog.options.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw("   "));
        }
        if idx == dialog.selected_index {
            spans.push(Span::styled(
                format!("[{}]", option),
                Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", option),
                Style::default().fg(Color::White),
            ));
        }
    }

    let buttons = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center);
    f.render_widget(buttons, chunks[1]);
}

/// Render text input dialog
fn render_text_input_dialog(f: &mut Frame, dialog: &Dialog, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    // Message
    let message = Paragraph::new(dialog.message.as_str())
        .style(Style::default().fg(Color::Gray));
    f.render_widget(message, chunks[0]);

    // Input field with cursor
    let mut display_text = String::new();
    for (i, c) in dialog.input_text.chars().enumerate() {
        if i == dialog.input_cursor {
            display_text.push('│');
        }
        display_text.push(c);
    }
    if dialog.input_cursor == dialog.input_text.len() {
        display_text.push('│');
    }

    let input_field = Paragraph::new(display_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(input_field, chunks[1]);

    // Spacer
    f.render_widget(Paragraph::new(""), chunks[2]);

    // Buttons with color-coded selected state
    let mut spans = Vec::new();
    for (idx, option) in dialog.options.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw("   "));
        }
        if idx == dialog.selected_index {
            spans.push(Span::styled(
                format!("[{}]", option),
                Style::default().bg(Color::Cyan).fg(Color::Black).bold(),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", option),
                Style::default().fg(Color::White),
            ));
        }
    }

    let buttons = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center);
    f.render_widget(buttons, chunks[3]);
}

/// Render select list dialog
fn render_select_list_dialog(f: &mut Frame, dialog: &Dialog, area: Rect) {
    let list_items: Vec<ListItem> = dialog
        .options
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let style = if idx == dialog.selected_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(item.clone()).style(style)
        })
        .collect();

    let list = List::new(list_items).style(Style::default().fg(Color::White));
    f.render_widget(list, area);
}
