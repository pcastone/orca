//! Commands panel - displays available commands and workflows

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};
use super::super::app::App;

/// Commands available in the TUI
const AVAILABLE_COMMANDS: &[&str] = &[
    "Show System Health",
    "List Tasks",
    "List Workflows",
    "List Bugs",
    "List Rules",
    "Create Task",
    "Create Workflow",
    "Create Bug",
    "Create Rule",
    "Edit Configuration",
    "View Logs",
    "Clear Logs",
];

/// Commands panel renderer
pub struct CommandsPanel;

impl CommandsPanel {
    /// Render the commands panel
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let block = Block::default()
            .title("Commands [2] - Use ↑↓ to navigate, Enter to execute")
            .borders(Borders::ALL)
            .style(if matches!(app.focused, crate::tui::app::FocusedPanel::Commands) {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            });

        let items: Vec<ListItem> = AVAILABLE_COMMANDS
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let style = if i == app.selected_command {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                } else {
                    Style::default()
                };
                ListItem::new(*cmd).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);
    }
}
