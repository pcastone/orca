//! Logs panel - displays system and execution logs

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};
use super::super::app::App;

/// Logs panel renderer
pub struct LogsPanel;

impl LogsPanel {
    /// Render the logs panel
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let block = Block::default()
            .title("Logs [4] - Use ↑↓/PgUp/PgDn to scroll, Ctrl+C to clear")
            .borders(Borders::ALL)
            .style(if matches!(app.focused, crate::tui::app::FocusedPanel::Logs) {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            });

        let logs: Vec<ListItem> = app
            .logs
            .iter()
            .rev()
            .skip(app.scroll_offset as usize)
            .take(area.height.saturating_sub(2) as usize)
            .rev()
            .map(|log| ListItem::new(log.clone()).style(Style::default().fg(Color::Gray)))
            .collect();

        let list = List::new(logs)
            .block(block)
            .style(Style::default());

        f.render_widget(list, area);
    }
}
