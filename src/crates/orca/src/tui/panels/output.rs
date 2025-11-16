//! Output panel - displays command execution results

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use super::super::app::App;

/// Output panel renderer
pub struct OutputPanel;

impl OutputPanel {
    /// Render the output panel
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let block = Block::default()
            .title("Output [3] - Ctrl+C to clear")
            .borders(Borders::ALL)
            .style(if matches!(app.focused, crate::tui::app::FocusedPanel::Output) {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            });

        let content = if app.command_output.is_empty() {
            "No command output yet. Select a command and press Enter to execute.".to_string()
        } else {
            app.command_output.clone()
        };

        let paragraph = Paragraph::new(content)
            .block(block)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }
}
