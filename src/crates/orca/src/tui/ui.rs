//! UI rendering for the TUI - Conversation-centric layout

use ratatui::{
    prelude::*,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};
use super::app::{App, DialogState, FocusedArea, SidebarTab, MenuState};
use super::dialog;

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

    // Render dropdown menu if one is open
    render_dropdown_menu(f, app, chunks[0]);

    // Render dialog if one is open
    if let Some(ref dlg) = app.dialog {
        dialog::render_dialog(f, dlg);
    }

    // Render LLM config form if dialog state is LlmProfileEdit or LlmProfileCreate
    if app.dialog_state == DialogState::LlmProfileEdit || app.dialog_state == DialogState::LlmProfileCreate {
        render_llm_config_form(f, app);
    }

    // Render pattern selection dialog
    if app.dialog_state == DialogState::PatternSelect || app.dialog_state == DialogState::PatternList {
        render_pattern_select_dialog(f, app);
    }
}

/// Render the LLM configuration form
fn render_llm_config_form(f: &mut Frame, app: &App) {
    let area = f.area();

    // Calculate form dimensions (centered popup)
    let form_width = 50.min(area.width.saturating_sub(4));
    let form_height = 16.min(area.height.saturating_sub(4)); // 7 fields + title + buttons
    let form_x = (area.width.saturating_sub(form_width)) / 2;
    let form_y = (area.height.saturating_sub(form_height)) / 2;

    let form_area = Rect {
        x: form_x,
        y: form_y,
        width: form_width,
        height: form_height,
    };

    // Clear the area
    f.render_widget(Clear, form_area);

    // Create form block
    let title = if app.dialog_state == DialogState::LlmProfileCreate {
        "Create LLM Configuration"
    } else {
        "Edit LLM Configuration"
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(block, form_area);

    // Calculate inner area for fields
    let inner_area = Rect {
        x: form_area.x + 2,
        y: form_area.y + 1,
        width: form_area.width.saturating_sub(4),
        height: form_area.height.saturating_sub(2),
    };

    // Render each field
    let form = &app.llm_config_form;
    let fields = [
        ("Name", form.name.clone(), false),
        ("Provider", form.provider.clone(), true),  // true = select type
        ("Model", form.model.clone(), false),
        ("API Key", if form.api_key.is_empty() { "(optional)".to_string() } else { "*".repeat(form.api_key.len().min(20)) }, false),
        ("API Base", if form.api_base.is_empty() { "(optional)".to_string() } else { form.api_base.clone() }, false),
        ("Temperature", form.temperature.clone(), false),
        ("Max Tokens", form.max_tokens.clone(), false),
    ];

    for (i, (label, value, is_select)) in fields.iter().enumerate() {
        if i >= inner_area.height as usize {
            break;
        }

        let field_y = inner_area.y + i as u16;
        let is_selected = i == form.selected_field;

        // Label
        let label_style = if is_selected {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::White)
        };

        let label_text = format!("{}: ", label);
        let label_para = Paragraph::new(label_text).style(label_style);
        let label_area = Rect {
            x: inner_area.x,
            y: field_y,
            width: 14,
            height: 1,
        };
        f.render_widget(label_para, label_area);

        // Value
        let value_style = if is_selected {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else if value.starts_with('(') || value.starts_with('*') {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Green)
        };

        let display_value = if *is_select && is_selected {
            format!("< {} >", value)
        } else {
            value.clone()
        };

        let value_para = Paragraph::new(display_value).style(value_style);
        let value_area = Rect {
            x: inner_area.x + 14,
            y: field_y,
            width: inner_area.width.saturating_sub(14),
            height: 1,
        };
        f.render_widget(value_para, value_area);
    }

    // Render help text at bottom
    if inner_area.height > 9 {
        let help_y = inner_area.y + inner_area.height - 2;
        let help_text = "Tab: Next | Up/Down: Navigate | Enter: Save | Esc: Cancel";
        let help_para = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        let help_area = Rect {
            x: inner_area.x,
            y: help_y,
            width: inner_area.width,
            height: 1,
        };
        f.render_widget(help_para, help_area);
    }
}

/// Render the pattern selection dialog
fn render_pattern_select_dialog(f: &mut Frame, app: &App) {
    let area = f.area();

    // Calculate dialog dimensions (centered popup)
    let dialog_width = 50.min(area.width.saturating_sub(4));
    let dialog_height = (app.patterns.len() as u16 + 6).min(area.height.saturating_sub(4));
    let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect {
        x: dialog_x,
        y: dialog_y,
        width: dialog_width,
        height: dialog_height,
    };

    // Clear the area
    f.render_widget(Clear, dialog_area);

    // Create dialog block
    let title = if app.dialog_state == DialogState::PatternSelect {
        "Select Pattern"
    } else {
        "Pattern Configurations"
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Magenta));

    f.render_widget(block, dialog_area);

    // Calculate inner area for content
    let inner_area = Rect {
        x: dialog_area.x + 2,
        y: dialog_area.y + 1,
        width: dialog_area.width.saturating_sub(4),
        height: dialog_area.height.saturating_sub(2),
    };

    if app.patterns.is_empty() {
        let empty_msg = Paragraph::new("No patterns configured.\nUse 'orca pattern create' to add patterns.")
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: true });
        f.render_widget(empty_msg, inner_area);
        return;
    }

    // Create list of patterns
    let list_items: Vec<ListItem> = app
        .patterns
        .iter()
        .enumerate()
        .map(|(idx, pattern)| {
            let is_selected = app.selected_pattern_index == Some(idx);
            let is_default = pattern.is_default;
            let is_active = app.active_pattern.as_ref().map_or(false, |p| p.id == pattern.id);

            let mut line = format!("{:<20} {:<12}", pattern.name, pattern.pattern_type);
            if is_default {
                line.push_str(" *");
            }
            if is_active {
                line.push_str(" [active]");
            }

            let style = if is_selected {
                Style::default().bg(Color::Magenta).fg(Color::White)
            } else if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(line).style(style)
        })
        .collect();

    // Split inner area for list and help text
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(inner_area);

    let list = List::new(list_items)
        .style(Style::default().fg(Color::White));
    f.render_widget(list, chunks[0]);

    // Help text
    let help_text = "Up/Down: Navigate | Enter: Select | Esc: Cancel | * = default";
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[1]);
}

/// Render the menu bar
fn render_menu(f: &mut Frame, app: &App, area: Rect) {
    let menus = vec!["File", "Edit", "Config", "Workflow", "Help"];
    let mut menu_text = String::new();

    for (idx, menu_name) in menus.iter().enumerate() {
        let is_open = match (idx, app.menu_state) {
            (0, MenuState::FileOpen) => true,
            (1, MenuState::EditOpen) => true,
            (2, MenuState::ConfigOpen) => true,
            (3, MenuState::WorkflowOpen) => true,
            (4, MenuState::HelpOpen) => true,
            _ => false,
        };

        if idx > 0 {
            menu_text.push_str("  ");
        }

        if is_open {
            menu_text.push_str(&format!("[{}]", menu_name));
        } else {
            menu_text.push_str(menu_name);
        }
    }

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
    let tab_titles = vec!["History", "Todo", "Bugs", "Patterns"];
    let selected = match app.active_tab {
        SidebarTab::History => 0,
        SidebarTab::Todo => 1,
        SidebarTab::Bugs => 2,
        SidebarTab::Patterns => 3,
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
            Constraint::Length(1),  // Tabs
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
        SidebarTab::Patterns => {
            if app.patterns.is_empty() {
                vec![ListItem::new("No patterns").style(Style::default().fg(Color::DarkGray))]
            } else {
                app.patterns
                    .iter()
                    .skip(app.sidebar_scroll as usize)
                    .take(chunks[1].height.saturating_sub(2) as usize)
                    .enumerate()
                    .map(|(i, pattern)| {
                        let is_active = app.active_pattern.as_ref().map_or(false, |p| p.id == pattern.id);
                        let is_default = pattern.is_default;

                        let mut display = format!("{}", pattern.name);
                        if display.len() > 12 {
                            display = format!("{}...", &display[..9]);
                        }

                        let indicator = if is_active { "◆" } else if is_default { "*" } else { "○" };

                        let style = if i == app.sidebar_selected {
                            Style::default().bg(Color::Magenta).fg(Color::White)
                        } else if is_active {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default().fg(Color::Magenta)
                        };

                        ListItem::new(format!("{} {}", indicator, display)).style(style)
                    })
                    .collect()
            }
        }
    };

    let list = List::new(items).block(block);
    f.render_widget(list, chunks[1]);
}

/// Render status bar
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let tokens_str = app.tokens_used.to_string();
    let mut status_parts: Vec<(&str, String)> = vec![
        ("Status", "Ready".to_string()),
        ("Model", app.current_model.clone()),
        ("Runtime", app.runtime.clone()),
        ("Tokens", tokens_str),
    ];

    // Add budget information if active
    if let Some(ref budget) = app.active_budget {
        let budget_info = if let Some(remaining) = app.budget_remaining {
            format!("{} ({:.1}% used, ${:.2} left)", budget, app.budget_usage, remaining)
        } else {
            format!("{} ({:.1}% used)", budget, app.budget_usage)
        };
        status_parts.push(("Budget", budget_info));
    }

    // Add LLM profile information if configured
    if let Some(ref profile) = app.llm_profile {
        let llm_info = if let (Some(ref planner), Some(ref worker)) = (&app.planner_llm, &app.worker_llm) {
            format!("{} (P:{} W:{})", profile,
                planner.split(':').nth(1).unwrap_or("?"),
                worker.split(':').nth(1).unwrap_or("?"))
        } else {
            profile.clone()
        };
        status_parts.push(("LLM Profile", llm_info));
    }

    // Add active pattern information
    let pattern_display = app.get_active_pattern_display();
    status_parts.push(("Pattern", pattern_display));

    // Build status bar with colored spans
    let mut spans = Vec::new();

    for (i, (label, value)) in status_parts.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" | "));
        }

        // Color code labels based on content
        let label_color = match *label {
            "Status" => Color::Cyan,
            "Model" => Color::Magenta,
            "Runtime" => Color::Blue,
            "Tokens" => Color::Green,
            "Budget" => {
                if app.budget_status.as_str() == "Budget exceeded" {
                    Color::Red
                } else if app.budget_status.as_str() == "Budget near limit" {
                    Color::Yellow
                } else {
                    Color::Green
                }
            }
            "LLM Profile" => Color::Cyan,
            "Pattern" => Color::Magenta,
            _ => Color::White,
        };

        spans.push(Span::styled(
            format!("{}: ", label),
            Style::default().fg(label_color).bold(),
        ));
        spans.push(Span::raw(format!("\"{}\"", value)));
    }

    // Color code the status bar based on budget status
    let bar_style = if app.active_budget.is_some() {
        match app.budget_status.as_str() {
            "Budget exceeded" => Style::default().bg(Color::Red).fg(Color::White).bold(),
            "Budget near limit" => Style::default().bg(Color::Yellow).fg(Color::Black).bold(),
            _ => Style::default().bg(Color::Black).fg(Color::DarkGray),
        }
    } else {
        Style::default().bg(Color::Black).fg(Color::DarkGray)
    };

    let status = Paragraph::new(Line::from(spans))
        .style(bar_style)
        .alignment(Alignment::Left);

    f.render_widget(status, area);
}

/// Get menu items for the currently open menu
fn get_menu_items(menu_state: MenuState) -> Vec<(&'static str, &'static str)> {
    match menu_state {
        MenuState::Closed => vec![],
        MenuState::FileOpen => vec![
            ("📄", "New"),
            ("📂", "Open"),
            ("💾", "Save"),
            ("🚪", "Quit"),
        ],
        MenuState::EditOpen => vec![
            ("🗑️", "Clear"),
            ("📋", "Copy"),
            ("⚙️", "Preferences"),
        ],
        MenuState::ConfigOpen => vec![
            ("👁️", "View Config"),
            ("💰", "Budget"),
            ("🤖", "LLM Profile"),
            ("🔀", "Pattern"),
            ("✎", "Editor"),
        ],
        MenuState::WorkflowOpen => vec![
            ("▶️", "Run"),
            ("👀", "View"),
            ("➕", "Create"),
            ("🔧", "Manage"),
        ],
        MenuState::HelpOpen => vec![
            ("ℹ️", "About"),
            ("⌨️", "Shortcuts"),
            ("📚", "Documentation"),
        ],
    }
}

/// Render dropdown menu
fn render_dropdown_menu(f: &mut Frame, app: &App, menu_area: Rect) {
    if app.menu_state == MenuState::Closed {
        return;
    }

    let items = get_menu_items(app.menu_state);
    if items.is_empty() {
        return;
    }

    // Calculate popup size: width is max item length + 2, height is items count + 2
    let popup_width = items
        .iter()
        .map(|(icon, label)| icon.len() + label.len() + 1)
        .max()
        .unwrap_or(10)
        + 2;
    let popup_height = items.len() as u16 + 2;

    // Position popup below menu bar
    let popup_area = Rect {
        x: menu_area.x + 1,
        y: menu_area.y + 1,
        width: popup_width as u16,
        height: popup_height,
    };

    // Only render if there's space
    if popup_area.y + popup_area.height > f.area().height {
        return;
    }

    // Clear the area where popup will be rendered
    f.render_widget(Clear, popup_area);

    // Build list items with highlighting
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(idx, (icon, label))| {
            let style = if idx == app.menu_selected_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("{} {}", icon, label)).style(style)
        })
        .collect();

    // Create bordered list
    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Cyan)));

    f.render_widget(list, popup_area);
}

