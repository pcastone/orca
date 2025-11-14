//! Terminal UI rendering with ratatui

use super::app::{App, View};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Render the main TUI frame
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(1),     // Main content
            Constraint::Length(2),  // Footer/Status
        ])
        .split(f.size());

    draw_header(f, app, chunks[0]);
    draw_main_content(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

/// Draw the header bar
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let header_text = format!("ACO - {}", app.view());
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM))
        .alignment(Alignment::Center);

    f.render_widget(header, area);
}

/// Draw the main content area based on current view
fn draw_main_content(f: &mut Frame, app: &App, area: Rect) {
    match app.view() {
        View::TaskList => draw_task_list(f, app, area),
        View::TaskDetail => draw_task_detail(f, app, area),
        View::WorkflowList => draw_workflow_list(f, app, area),
        View::WorkflowDetail => draw_workflow_detail(f, app, area),
        View::ExecutionStream => draw_execution_stream(f, app, area),
        View::Help => draw_help(f, app, area),
    }
}

/// Draw the task list view
fn draw_task_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(idx, task)| {
            let style = if idx == app.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if idx == app.selected { "▶ " } else { "  " };
            let content = format!(
                "{}[{}] {}",
                prefix, task.status, task.title
            );
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Tasks ")
            .borders(Borders::ALL),
    );

    f.render_widget(list, area);
}

/// Draw the task detail view
fn draw_task_detail(f: &mut Frame, app: &App, area: Rect) {
    if let Some(task) = app.selected_task() {
        // Parse status color
        let status_color = match task.status.as_str() {
            "pending" => Color::Yellow,
            "running" => Color::Cyan,
            "completed" => Color::Green,
            "failed" => Color::Red,
            "cancelled" => Color::DarkGray,
            _ => Color::White,
        };

        let content = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.id),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.title),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(format!("  {}", task.description)),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&task.status, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.task_type),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Workspace: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(format!("  {}", task.workspace_path)),
            Line::from(""),
            Line::from(vec![
                Span::styled("Config: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(format!("  {}", task.config)),
            Line::from(""),
            Line::from(vec![
                Span::styled("Metadata: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(format!("  {}", task.metadata)),
            Line::from(""),
            Line::from(vec![
                Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.created_at),
            ]),
            Line::from(vec![
                Span::styled("Updated: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.updated_at),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ESC to return to task list",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]),
        ];

        let paragraph = Paragraph::new(content)
            .block(Block::default().title(" Task Details ").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let empty_msg = Paragraph::new("No task selected")
            .block(Block::default().title(" Task Details ").borders(Borders::ALL))
            .alignment(Alignment::Center);

        f.render_widget(empty_msg, area);
    }
}

/// Draw the workflow list view
fn draw_workflow_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .workflows
        .iter()
        .enumerate()
        .map(|(idx, workflow)| {
            let style = if idx == app.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if idx == app.selected { "▶ " } else { "  " };
            let content = format!(
                "{}[{}] {}",
                prefix, workflow.status, workflow.name
            );
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Workflows ")
            .borders(Borders::ALL),
    );

    f.render_widget(list, area);
}

/// Draw the workflow detail view
fn draw_workflow_detail(f: &mut Frame, app: &App, area: Rect) {
    if let Some(workflow) = app.selected_workflow() {
        let content = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&workflow.id),
            ]),
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&workflow.name),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&workflow.status),
            ]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&workflow.created_at),
            ]),
        ];

        let paragraph = Paragraph::new(content)
            .block(Block::default().title(" Workflow Details ").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let empty_msg = Paragraph::new("No workflow selected")
            .block(
                Block::default()
                    .title(" Workflow Details ")
                    .borders(Borders::ALL),
            )
            .alignment(Alignment::Center);

        f.render_widget(empty_msg, area);
    }
}

/// Draw the execution stream view
fn draw_execution_stream(f: &mut Frame, app: &App, area: Rect) {
    if app.execution_events.is_empty() {
        let empty_msg = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("No execution in progress",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press 'e' on a task or workflow to execute it",
                    Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let paragraph = Paragraph::new(empty_msg)
            .block(
                Block::default()
                    .title(" Execution Stream ")
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let title = if let Some(id) = app.executing_id() {
            format!(" Execution Stream - {} ", id)
        } else {
            " Execution Stream ".to_string()
        };

        let mut lines: Vec<Line> = Vec::new();

        for event in &app.execution_events {
            // Color code by event type
            let (icon, color) = match event.event_type.as_str() {
                "started" => ("▶", Color::Green),
                "progress" => ("⋯", Color::Cyan),
                "output" => ("◉", Color::Yellow),
                "tool_call" => ("🔧", Color::Magenta),
                "tool_result" => ("✓", Color::Blue),
                "completed" => ("✔", Color::Green),
                "failed" => ("✗", Color::Red),
                _ => ("•", Color::White),
            };

            // Extract timestamp (just time part)
            let time = event.timestamp
                .split('T')
                .nth(1)
                .and_then(|t| t.split('.').next())
                .unwrap_or("00:00:00");

            lines.push(Line::from(vec![
                Span::styled(
                    format!("[{}] ", time),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{} ", icon),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}: ", event.event_type.to_uppercase()),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(&event.message),
            ]));
        }

        // Add help text at the bottom
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Press ESC to return",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]));

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded),
            )
            .wrap(Wrap { trim: false })
            .scroll((app.scroll as u16, 0));

        f.render_widget(paragraph, area);
    }
}

/// Draw the help view
fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "ACO TUI - Keyboard Shortcuts",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled("Navigation:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from("  ↑/↓, j/k    - Navigate up/down (Vim-style also supported)"),
        Line::from("  Enter       - View details / Select item"),
        Line::from("  Esc         - Back / Return to list / Quit"),
        Line::from("  Home, g     - Jump to first item"),
        Line::from("  End, G      - Jump to last item"),
        Line::from("  PgUp        - Scroll up one page (10 items)"),
        Line::from("  PgDn        - Scroll down one page (10 items)"),
        Line::from(""),
        Line::from(vec![Span::styled("View Switching:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from("  Tab         - Cycle to next view"),
        Line::from("  Shift+Tab   - Cycle to previous view"),
        Line::from("  1           - Tasks List"),
        Line::from("  2           - Workflows List"),
        Line::from("  3           - Execution Stream"),
        Line::from("  4, ?, h, F1 - Help"),
        Line::from(""),
        Line::from(vec![Span::styled("Actions:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from("  e           - Execute selected task/workflow"),
        Line::from("  r           - Refresh data from server"),
        Line::from(""),
        Line::from(vec![Span::styled("General:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from("  q, Ctrl+C   - Quit application"),
        Line::from(""),
        Line::from(vec![Span::styled("Connection:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(format!("  Server: {}", app.server_url())),
        Line::from(format!("  Auth: {}", app.auth())),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Draw the footer with status and error messages
fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let status_text = if let Some(error) = app.error() {
        format!(" ERROR: {} ", error)
    } else {
        format!(" {} ", app.status())
    };

    let status_style = if app.error().is_some() {
        Style::default().bg(Color::Red).fg(Color::White)
    } else {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    };

    let status = Paragraph::new(status_text)
        .style(status_style)
        .alignment(Alignment::Left);

    f.render_widget(status, area);
}
