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
    // Count statuses based on current view
    let stats = match app.view() {
        View::TaskList | View::TaskDetail => {
            let mut pending = 0;
            let mut running = 0;
            let mut completed = 0;
            let mut failed = 0;

            for task in &app.tasks {
                match task.status.as_str() {
                    "pending" => pending += 1,
                    "running" => running += 1,
                    "completed" => completed += 1,
                    "failed" | "cancelled" => failed += 1,
                    _ => {}
                }
            }

            {
                let mut parts = vec![format!(" | {} Tasks:", app.tasks.len())];
                if pending > 0 {
                    parts.push(format!(" ⏸{}", pending));
                }
                if running > 0 {
                    parts.push(format!(" ▶{}", running));
                }
                if completed > 0 {
                    parts.push(format!(" ✔{}", completed));
                }
                if failed > 0 {
                    parts.push(format!(" ✗{}", failed));
                }
                parts.join("")
            }
        }
        View::WorkflowList | View::WorkflowDetail => {
            let mut draft = 0;
            let mut active = 0;
            let mut running = 0;

            for workflow in &app.workflows {
                match workflow.status.as_str() {
                    "draft" => draft += 1,
                    "active" => active += 1,
                    "running" => running += 1,
                    _ => {}
                }
            }

            {
                let mut parts = vec![format!(" | {} Workflows:", app.workflows.len())];
                if draft > 0 {
                    parts.push(format!(" ◯{}", draft));
                }
                if active > 0 {
                    parts.push(format!(" ◉{}", active));
                }
                if running > 0 {
                    parts.push(format!(" ▶{}", running));
                }
                parts.join("")
            }
        }
        View::BugList | View::BugDetail => {
            let mut open = 0;
            let mut in_progress = 0;
            let mut fixed = 0;

            for bug in &app.bugs {
                match bug.status.as_str() {
                    "open" => open += 1,
                    "in_progress" => in_progress += 1,
                    "fixed" => fixed += 1,
                    _ => {}
                }
            }

            {
                let mut parts = vec![format!(" | {} Bugs:", app.bugs.len())];
                if open > 0 {
                    parts.push(format!(" 🔴{}", open));
                }
                if in_progress > 0 {
                    parts.push(format!(" 🟡{}", in_progress));
                }
                if fixed > 0 {
                    parts.push(format!(" 🟢{}", fixed));
                }
                parts.join("")
            }
        }
        View::ExecutionStream => {
            if let Some(id) = app.executing_id() {
                format!(" | Executing: {} | {} events", id, app.execution_events.len())
            } else {
                String::new()
            }
        }
        _ => String::new(),
    };

    let header_text = format!("ACO - {}{}", app.view(), stats);
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
        View::BugList => draw_bug_list(f, app, area),
        View::BugDetail => draw_bug_detail(f, app, area),
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
            // Status icon and color
            let (status_icon, status_color) = match task.status.as_str() {
                "pending" => ("⏸", Color::Yellow),
                "running" => ("▶", Color::Cyan),
                "completed" => ("✔", Color::Green),
                "failed" => ("✗", Color::Red),
                "cancelled" => ("⊗", Color::DarkGray),
                _ => ("•", Color::White),
            };

            // Selection indicator
            let selector = if idx == app.selected { "▸ " } else { "  " };

            // Task type badge
            let type_badge = match task.task_type.as_str() {
                "execution" => "[EXEC]",
                "workflow" => "[FLOW]",
                "validation" => "[VALD]",
                _ => "[TASK]",
            };

            let line = Line::from(vec![
                Span::raw(selector),
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    type_badge,
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(" "),
                Span::styled(
                    &task.title,
                    if idx == app.selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]);

            let style = if idx == app.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let title = format!(" Tasks ({}) ", app.tasks.len());
    let list = List::new(items).block(
        Block::default()
            .title(title)
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
            // Status icon and color
            let (status_icon, status_color) = match workflow.status.as_str() {
                "draft" => ("◯", Color::DarkGray),
                "active" => ("◉", Color::Green),
                "running" => ("▶", Color::Cyan),
                "paused" => ("⏸", Color::Yellow),
                "completed" => ("✔", Color::Green),
                "failed" => ("✗", Color::Red),
                _ => ("•", Color::White),
            };

            // Selection indicator
            let selector = if idx == app.selected { "▸ " } else { "  " };

            let line = Line::from(vec![
                Span::raw(selector),
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "[WORKFLOW]",
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw(" "),
                Span::styled(
                    &workflow.name,
                    if idx == app.selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]);

            let style = if idx == app.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let title = format!(" Workflows ({}) ", app.workflows.len());
    let list = List::new(items).block(
        Block::default()
            .title(title)
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

/// Draw the bug list view
fn draw_bug_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .bugs
        .iter()
        .enumerate()
        .map(|(idx, bug)| {
            // Status icon and color
            let (status_icon, status_color) = match bug.status.as_str() {
                "open" => ("●", Color::Red),
                "in_progress" => ("◐", Color::Yellow),
                "fixed" => ("✔", Color::Green),
                "wontfix" => ("✗", Color::DarkGray),
                "duplicate" => ("≡", Color::DarkGray),
                _ => ("•", Color::White),
            };

            // Priority indicator
            let priority_color = match bug.priority {
                1 => Color::Red,      // Critical
                2 => Color::LightRed, // High
                3 => Color::Yellow,   // Medium
                4 => Color::Blue,     // Low
                5 => Color::DarkGray, // Trivial
                _ => Color::White,
            };

            // Selection indicator
            let selector = if idx == app.selected { "▸ " } else { "  " };

            let line = Line::from(vec![
                Span::raw(selector),
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("P{} ", bug.priority),
                    Style::default().fg(priority_color),
                ),
                Span::styled(
                    "[BUG]",
                    Style::default().fg(Color::Red),
                ),
                Span::raw(" "),
                Span::styled(
                    &bug.title,
                    if idx == app.selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
            ]);

            let style = if idx == app.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let title = format!(" Bugs ({}) ", app.bugs.len());
    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL),
    );

    f.render_widget(list, area);
}

/// Draw the bug detail view
fn draw_bug_detail(f: &mut Frame, app: &App, area: Rect) {
    if let Some(bug) = app.selected_bug() {
        // Status color
        let status_color = match bug.status.as_str() {
            "open" => Color::Red,
            "in_progress" => Color::Yellow,
            "fixed" => Color::Green,
            "wontfix" | "duplicate" => Color::DarkGray,
            _ => Color::White,
        };

        // Priority color and text
        let (priority_text, priority_color) = match bug.priority {
            1 => ("Critical", Color::Red),
            2 => ("High", Color::LightRed),
            3 => ("Medium", Color::Yellow),
            4 => ("Low", Color::Blue),
            5 => ("Trivial", Color::DarkGray),
            _ => ("Unknown", Color::White),
        };

        let mut content = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&bug.id),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&bug.title),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&bug.status, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Priority: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(priority_text, Style::default().fg(priority_color).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
        ];

        if let Some(severity) = &bug.severity {
            content.push(Line::from(vec![
                Span::styled("Severity: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(severity),
            ]));
            content.push(Line::from(""));
        }

        if let Some(assignee) = &bug.assignee {
            content.push(Line::from(vec![
                Span::styled("Assignee: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(assignee),
            ]));
            content.push(Line::from(""));
        }

        if let Some(reporter) = &bug.reporter {
            content.push(Line::from(vec![
                Span::styled("Reporter: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(reporter),
            ]));
            content.push(Line::from(""));
        }

        if let Some(description) = &bug.description {
            content.push(Line::from(vec![
                Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
            ]));
            content.push(Line::from(format!("  {}", description)));
            content.push(Line::from(""));
        }

        content.push(Line::from(vec![
            Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&bug.created_at),
        ]));
        content.push(Line::from(vec![
            Span::styled("Updated: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&bug.updated_at),
        ]));
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("Press ESC to return to bug list",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));

        let paragraph = Paragraph::new(content)
            .block(Block::default().title(" Bug Details ").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let empty_msg = Paragraph::new("No bug selected")
            .block(Block::default().title(" Bug Details ").borders(Borders::ALL))
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
                "reasoning" => ("💭", Color::Gray),
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
            "ACO TUI - Keyboard Shortcuts & Status Indicators",
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
        Line::from("  1, Ctrl+1   - Tasks List"),
        Line::from("  2, Ctrl+2   - Workflows List"),
        Line::from("  3, Ctrl+3   - Bugs List"),
        Line::from("  4, Ctrl+4   - Execution Stream"),
        Line::from("  5, Ctrl+5   - Help (also ?, h, F1)"),
        Line::from(""),
        Line::from(vec![Span::styled("Actions:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from("  e           - Execute selected task/workflow"),
        Line::from("  r           - Refresh data from server"),
        Line::from(""),
        Line::from(vec![Span::styled("Task Status Indicators:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("⏸", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" Pending   "),
            Span::styled("▶", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Running   "),
            Span::styled("✔", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" Completed"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("✗", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" Failed    "),
            Span::styled("⊗", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
            Span::raw(" Cancelled"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Workflow Status Indicators:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("◯", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
            Span::raw(" Draft     "),
            Span::styled("◉", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" Active    "),
            Span::styled("▶", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" Running"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("⏸", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" Paused    "),
            Span::styled("✔", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" Completed "),
            Span::styled("✗", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" Failed"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Bug Status Indicators:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("●", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" Open       "),
            Span::styled("◐", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" In Progress "),
            Span::styled("✔", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" Fixed"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("✗", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
            Span::raw(" Won't Fix  "),
            Span::styled("≡", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
            Span::raw(" Duplicate"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Type Badges:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[EXEC]", Style::default().fg(Color::Blue)),
            Span::raw(" Execution task  "),
            Span::styled("[FLOW]", Style::default().fg(Color::Blue)),
            Span::raw(" Workflow task"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[VALD]", Style::default().fg(Color::Blue)),
            Span::raw(" Validation task "),
            Span::styled("[WORKFLOW]", Style::default().fg(Color::Magenta)),
            Span::raw(" Workflow"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[BUG]", Style::default().fg(Color::Red)),
            Span::raw(" Bug report"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Data Operations:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from("  F5          - Backup databases"),
        Line::from("  F6          - Restore from latest backup"),
        Line::from("  F7          - Export all tables to SQL"),
        Line::from("  F8          - Import (CLI only: aco data import <file>)"),
        Line::from(""),
        Line::from(vec![Span::styled("General:", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from("  q, Ctrl+Q, Ctrl+C - Quit application"),
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
