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
        let content = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.id),
            ]),
            Line::from(vec![
                Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.title),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.status),
            ]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&task.created_at),
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
    let content = vec![
        Line::from("Execution Stream View"),
        Line::from(""),
        Line::from("Real-time execution output will be displayed here"),
        Line::from(format!("Status: {}", app.status())),
    ];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Execution Stream ")
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Draw the help view
fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "ACO TUI - Help",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  ↑/↓       - Navigate up/down"),
        Line::from("  Enter    - View details"),
        Line::from("  Ctrl+R   - Refresh"),
        Line::from(""),
        Line::from("Views:"),
        Line::from("  1        - Task List"),
        Line::from("  2        - Workflow List"),
        Line::from("  3        - Execution Stream"),
        Line::from(""),
        Line::from("General:"),
        Line::from("  q/Esc    - Quit"),
        Line::from("  ?/h/F1   - Help"),
        Line::from(""),
        Line::from(format!("Server: {}", app.server_url())),
        Line::from(format!("Auth: {}", app.auth())),
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
