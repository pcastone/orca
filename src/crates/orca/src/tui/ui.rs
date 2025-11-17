//! UI rendering for the TUI - Conversation-centric layout

use ratatui::{
    prelude::*,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
};
use super::app::{App, FocusedArea, SidebarTab};

/// Render the complete UI
pub fn render_ui(f: &mut Frame, app: &App) {
    // Create the main vertical layout: Menu | Main Area | Status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1),     // Menu bar (top)
            Constraint::Min(10),       // Main content area
            Constraint::Length(1),     // Status bar (bottom)
        ])
        .split(f.area());

    // Render menu bar
    render_menu(f, app, chunks[0]);

    // Main content area: Left (conversation + prompts) | Right (sidebar)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75),  // Left side (conversation + prompts)
            Constraint::Percentage(25),  // Right side (sidebar)
        ])
        .split(chunks[1]);

    // Left side: conversation and prompts
    render_left_side(f, app, main_chunks[0]);

    // Right side: sidebar with tabs
    render_sidebar(f, app, main_chunks[1]);

    // Status bar
    render_status_bar(f, app, chunks[2]);
}

/// Render the menu bar
fn render_menu(f: &mut Frame, _app: &App, area: Rect) {
    let menu_text = " File  Edit  View  Help ";
    let menu = Paragraph::new(menu_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .alignment(Alignment::Left);

    f.render_widget(menu, area);
}

/// Render left side (conversation + prompts)
fn render_left_side(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),     // Conversation
            Constraint::Length(6),  // Prompts (3 lines + 2 for borders + 1 padding)
        ])
        .split(area);

    render_conversation(f, app, chunks[0]);
    render_prompts(f, app, chunks[1]);
}

/// Render conversation area
fn render_conversation(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = matches!(app.focused, FocusedArea::Conversation);

    let block = Block::default()
        .title("Main conversation")
        .borders(Borders::ALL)
        .style(if is_focused {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default()
        });

    let messages: Vec<ListItem> = app
        .conversation
        .iter()
        .rev()
        .skip(app.conversation_scroll as usize)
        .take(area.height.saturating_sub(2) as usize)
        .rev()
        .map(|msg| ListItem::new(msg.clone()).style(Style::default().fg(Color::White)))
        .collect();

    let list = List::new(messages)
        .block(block)
        .style(Style::default());

    f.render_widget(list, area);
}

/// Render prompts input area (supports up to 3 lines)
fn render_prompts(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = matches!(app.focused, FocusedArea::Prompts);

    let block = Block::default()
        .title("Prompts (Enter for newline, Ctrl+Enter to submit)")
        .borders(Borders::ALL)
        .style(if is_focused {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default()
        });

    // Build display text with cursor
    let mut display_text = String::new();
    for (line_idx, line) in app.prompt_lines.iter().enumerate() {
        if line_idx > 0 {
            display_text.push('\n');
        }

        if is_focused && line_idx == app.prompt_cursor_line {
            // Insert cursor in the current line
            display_text.push_str(&line[..app.prompt_cursor_col]);
            display_text.push('│');
            display_text.push_str(&line[app.prompt_cursor_col..]);
        } else {
            display_text.push_str(line);
        }
    }

    // If in prompts and at end of last line, show cursor at end
    if is_focused && app.prompt_cursor_line == app.prompt_lines.len() - 1
        && app.prompt_cursor_col == app.prompt_lines[app.prompt_cursor_line].len() {
        display_text.push('│');
    }

    let paragraph = Paragraph::new(display_text)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Render sidebar with tabs
fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = matches!(app.focused, FocusedArea::Sidebar);

    // Tabs
    let tab_titles = vec!["History", "Todo", "Bugs"];
    let selected = match app.active_tab {
        SidebarTab::History => 0,
        SidebarTab::Todo => 1,
        SidebarTab::Bugs => 2,
    };

    let tabs = Tabs::new(tab_titles)
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .bold(),
        );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Tabs
            Constraint::Min(5),     // Content
        ])
        .split(area);

    f.render_widget(tabs, chunks[0]);

    // Sidebar content based on active tab
    let block = Block::default()
        .borders(Borders::ALL)
        .style(if is_focused {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default()
        });

    let items: Vec<ListItem> = match app.active_tab {
        SidebarTab::History => {
            app.history
                .iter()
                .rev()
                .skip(app.sidebar_scroll as usize)
                .take(chunks[1].height.saturating_sub(2) as usize)
                .rev()
                .enumerate()
                .map(|(i, item)| {
                    let style = if i == app.sidebar_selected {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    ListItem::new(format!("▸ {}", item)).style(style)
                })
                .collect()
        }
        SidebarTab::Todo => {
            app.todo_items
                .iter()
                .skip(app.sidebar_scroll as usize)
                .take(chunks[1].height.saturating_sub(2) as usize)
                .enumerate()
                .map(|(i, item)| {
                    let style = if i == app.sidebar_selected {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else {
                        Style::default().fg(Color::Yellow)
                    };
                    ListItem::new(format!("☐ {}", item)).style(style)
                })
                .collect()
        }
        SidebarTab::Bugs => {
            app.bugs
                .iter()
                .skip(app.sidebar_scroll as usize)
                .take(chunks[1].height.saturating_sub(2) as usize)
                .enumerate()
                .map(|(i, bug)| {
                    let style = if i == app.sidebar_selected {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else {
                        Style::default().fg(Color::Red)
                    };
                    ListItem::new(format!("✕ {}", bug)).style(style)
                })
                .collect()
        }
    };

    let list = List::new(items).block(block);
    f.render_widget(list, chunks[1]);
}

/// Render status bar
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let tokens_str = app.tokens_used.to_string();
    let status_parts = vec![
        ("Status", "Ready"),
        ("Model", app.current_model.as_str()),
        ("Runtime", app.runtime.as_str()),
        ("Tokens", tokens_str.as_str()),
    ];

    let mut status_text = String::new();
    for (label, value) in status_parts {
        status_text.push_str(&format!("{}: \"{}\" | ", label, value));
    }
    if status_text.ends_with(" | ") {
        status_text.pop();
        status_text.pop();
        status_text.pop();
    }

    let status = Paragraph::new(status_text)
        .style(Style::default().bg(Color::Black).fg(Color::DarkGray))
        .alignment(Alignment::Left);

    f.render_widget(status, area);
}
