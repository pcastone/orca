//! Status panel - displays system health and metrics

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use super::super::app::App;

/// Status panel renderer
pub struct StatusPanel;

impl StatusPanel {
    /// Render the status panel
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let block = Block::default()
            .title("Status [1]")
            .borders(Borders::ALL)
            .style(if matches!(app.focused, crate::tui::app::FocusedPanel::Status) {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            });

        let content = if let Some(report) = &app.health_report {
            format!(
                "Overall Status: {}\nTotal Response Time: {}ms",
                report.status, report.total_response_time_ms
            )
        } else {
            "Loading health information...".to_string()
        };

        let paragraph = Paragraph::new(content)
            .block(block)
            .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, area);
    }
}
