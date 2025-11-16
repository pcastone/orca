//! UI rendering for the TUI

use ratatui::{
    prelude::*,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::Paragraph,
};
use super::app::App;
use super::panels::{CommandsPanel, LogsPanel, OutputPanel, StatusPanel};

/// Render the complete UI
pub fn render_ui(f: &mut Frame, app: &App) {
    // Create the main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(5),    // Status panel (top)
                Constraint::Min(10),      // Middle section
                Constraint::Length(8),    // Logs panel (bottom)
            ]
            .as_ref(),
        )
        .split(f.area());

    // Top: Status panel
    StatusPanel::render(f, app, chunks[0]);

    // Middle: Commands and Output side by side
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(40),  // Commands (left)
                Constraint::Percentage(60),  // Output (right)
            ]
            .as_ref(),
        )
        .split(chunks[1]);

    CommandsPanel::render(f, app, middle_chunks[0]);
    OutputPanel::render(f, app, middle_chunks[1]);

    // Bottom: Logs panel
    LogsPanel::render(f, app, chunks[2]);

    // Footer with help text
    render_footer(f, app);
}

/// Render footer with help text
fn render_footer(f: &mut Frame, _app: &App) {
    let area = f.area();
    let footer_area = Rect {
        x: 0,
        y: area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };

    let help_text = "q: quit | Tab: next panel | 1-4: jump to panel | j/k: navigate | PgUp/PgDn: scroll logs | Ctrl+C: clear";
    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray).bg(Color::Black))
        .alignment(Alignment::Left);

    f.render_widget(footer, footer_area);
}
