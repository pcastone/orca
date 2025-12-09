//! UI rendering for the TUI - Conversation-centric layout

use ratatui::{
    prelude::*,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};
use super::app::{App, DialogState, FocusedArea, SidebarTab, MenuState, ViewMode, ConfigSection, ConfigEditorForm, LlmFocusArea, LlmProfileEntry};
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

    // Render pattern selection dialog
    if app.dialog_state == DialogState::PatternSelect || app.dialog_state == DialogState::PatternList {
        render_pattern_select_dialog(f, app);
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

/// Render left side (conversation + prompts OR config editor)
fn render_left_side(f: &mut Frame, app: &App, area: Rect) {
    match app.view_mode {
        ViewMode::Conversation => {
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
        ViewMode::ConfigEditor => {
            render_config_editor(f, app, area);
        }
    }
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

    // Join all messages with double newlines for visual separation
    let content: String = app
        .conversation
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n\n");

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true })
        .scroll((app.conversation_scroll as u16, 0));

    f.render_widget(paragraph, area);
}

/// Render prompts input area (supports up to 3 lines)
fn render_prompts(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = matches!(app.focused, FocusedArea::Prompts);

    let block = Block::default()
        .title("Prompts (Enter to submit)")
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
        ("Status", app.status.clone()),
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
            ("🔍", "Init"),
            ("🔄", "Update"),
            ("💾", "Save"),
            ("📦", "Backup"),
            ("♻️", "Restore"),
            ("📤", "Export"),
            ("📥", "Import"),
            ("🚪", "Quit"),
        ],
        MenuState::EditOpen => vec![
            ("🔨", "Build"),
            ("🔄", "Update"),
            ("🔍", "Refine"),
            ("🗑️", "Purge"),
            ("🔎", "Search"),
        ],
        MenuState::ConfigOpen => vec![
            ("👁️", "View Config"),
            ("💰", "Budget"),
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

/// Render the config editor in the main content area
fn render_config_editor(f: &mut Frame, app: &App, area: Rect) {
    // Create main layout with header bar and content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header with Save/Cancel buttons
            Constraint::Min(5),     // Config content
        ])
        .split(area);

    // Render header with Save/Cancel buttons
    render_config_header(f, app, chunks[0]);

    // Render section tabs and content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14), // Section tabs
            Constraint::Min(20),    // Config fields
        ])
        .split(chunks[1]);

    // Render section tabs
    render_config_sections(f, app, content_chunks[0]);

    // Render fields for current section
    render_config_fields(f, app, content_chunks[1]);
}

/// Render config editor header with Save/Cancel buttons
fn render_config_header(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.config_editor;

    // Create layout for title and buttons
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),     // Title
            Constraint::Length(25),  // Buttons
        ])
        .split(area);

    // Title with modified indicator
    let title = if form.modified {
        "Configuration Editor *"
    } else {
        "Configuration Editor"
    };

    let title_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let help_text = "Tab: Section | Up/Down: Fields | Enter: Edit | Esc: Cancel";
    let help_para = Paragraph::new(help_text)
        .block(title_block)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_para, chunks[0]);

    // Buttons
    let button_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let save_style = if form.button_focus == 1 {
        Style::default().bg(Color::Green).fg(Color::Black).bold()
    } else {
        Style::default().fg(Color::Green)
    };

    let cancel_style = if form.button_focus == 2 {
        Style::default().bg(Color::Red).fg(Color::White).bold()
    } else {
        Style::default().fg(Color::Red)
    };

    let buttons = Line::from(vec![
        Span::styled(" [Save] ", save_style),
        Span::raw(" "),
        Span::styled(" [Cancel] ", cancel_style),
    ]);

    let buttons_para = Paragraph::new(buttons)
        .block(button_block)
        .alignment(Alignment::Right);
    f.render_widget(buttons_para, chunks[1]);
}

/// Render config section tabs on the left
fn render_config_sections(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.config_editor;

    let block = Block::default()
        .title("Sections")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let sections: Vec<ListItem> = ConfigSection::all()
        .iter()
        .map(|section| {
            let is_selected = *section == form.section;
            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };

            let marker = if is_selected { "▶ " } else { "  " };
            ListItem::new(format!("{}{}", marker, section.name())).style(style)
        })
        .collect();

    let list = List::new(sections).block(block);
    f.render_widget(list, area);
}

/// Render config fields for the current section
fn render_config_fields(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.config_editor;

    // Special rendering for LLM section - table + detail view
    if form.section == ConfigSection::Llm {
        render_llm_section(f, app, area);
        return;
    }

    let title = format!("{} Settings", form.section.name());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    // Calculate inner area
    let inner_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(3),
    };

    f.render_widget(block, area);

    // Render each field
    let field_count = form.field_count();
    for i in 0..field_count {
        if i >= inner_area.height as usize {
            break;
        }

        let field_y = inner_area.y + i as u16;
        let is_selected = i == form.field_index && form.button_focus == 0;
        let is_editing = is_selected && form.editing;
        let is_bool = form.is_bool_field(i);

        // Field label
        let label = form.field_name(i);
        let label_width = 20.min(inner_area.width / 2) as usize;
        let label_text = format!("{:width$}", label, width = label_width);

        let label_style = if is_selected {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::White)
        };

        let label_area = Rect {
            x: inner_area.x,
            y: field_y,
            width: label_width as u16,
            height: 1,
        };
        f.render_widget(Paragraph::new(label_text).style(label_style), label_area);

        // Check if field is a dropdown (for workflow section)
        let is_dropdown = form.section == ConfigSection::Workflow
            && ConfigEditorForm::is_workflow_dropdown_field(i);

        // Field value
        let value = if is_editing {
            format!("{}│", form.edit_buffer)
        } else {
            form.field_value(i)
        };

        let value_style = if is_editing {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else if is_selected {
            Style::default().fg(Color::Cyan).bold()
        } else if is_bool {
            let bool_val = form.field_value(i);
            if bool_val == "true" {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            }
        } else {
            Style::default().fg(Color::Gray)
        };

        // Add toggle hint for boolean fields or dropdown indicator
        let display_value = if is_bool && is_selected && !is_editing {
            format!("{} (Space to toggle)", value)
        } else if is_dropdown {
            format!("{} ▼", value)
        } else {
            value
        };

        let value_area = Rect {
            x: inner_area.x + label_width as u16 + 1,
            y: field_y,
            width: inner_area.width.saturating_sub(label_width as u16 + 1),
            height: 1,
        };
        f.render_widget(Paragraph::new(display_value).style(value_style), value_area);
    }

    // Render dropdown overlay if open (for workflow section)
    if form.section == ConfigSection::Workflow && form.dropdown_open {
        render_workflow_dropdown(f, form, inner_area);
    }
}

/// Render dropdown overlay for workflow section (planner/worker LLM selection)
fn render_workflow_dropdown(f: &mut Frame, form: &ConfigEditorForm, form_area: Rect) {
    let Some(field_index) = form.dropdown_field else {
        return;
    };

    if form.dropdown_options.is_empty() {
        return;
    }

    // Get field name for title
    let field_name = form.field_name(field_index);

    // Calculate dropdown position - below the field
    let label_width = 20.min(form_area.width / 2) as u16;
    let dropdown_x = form_area.x + label_width + 1;
    let dropdown_y = form_area.y + field_index as u16 + 1;  // +1 to position below field

    // Calculate dropdown size
    let max_option_len = form.dropdown_options.iter()
        .map(|s| s.len())
        .max()
        .unwrap_or(10);
    let dropdown_width = (max_option_len + 4).min(40) as u16;  // +4 for padding and borders
    let dropdown_height = (form.dropdown_options.len() + 2).min(10) as u16;  // +2 for borders

    // Ensure dropdown fits on screen
    let available_height = f.area().height.saturating_sub(dropdown_y);
    let actual_height = dropdown_height.min(available_height);

    let dropdown_area = Rect {
        x: dropdown_x,
        y: dropdown_y,
        width: dropdown_width.min(form_area.width.saturating_sub(label_width + 1)),
        height: actual_height,
    };

    // Only render if we have space
    if dropdown_area.height < 3 || dropdown_area.width < 5 {
        return;
    }

    // Clear the area
    f.render_widget(Clear, dropdown_area);

    // Create dropdown block
    let block = Block::default()
        .title(format!("Select {}", field_name))
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    // Create list items with highlighting
    let visible_items = (actual_height - 2) as usize;  // -2 for borders
    let scroll_offset = if form.dropdown_selected >= visible_items {
        form.dropdown_selected - visible_items + 1
    } else {
        0
    };

    let list_items: Vec<ListItem> = form.dropdown_options
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_items)
        .map(|(idx, option)| {
            let is_selected = idx == form.dropdown_selected;
            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };

            let marker = if is_selected { "▶ " } else { "  " };
            ListItem::new(format!("{}{}", marker, option)).style(style)
        })
        .collect();

    let list = List::new(list_items).block(block);
    f.render_widget(list, dropdown_area);

    // Show scroll indicator if needed
    if form.dropdown_options.len() > visible_items {
        let scroll_info = format!("{}/{}", form.dropdown_selected + 1, form.dropdown_options.len());
        let scroll_area = Rect {
            x: dropdown_area.x + dropdown_area.width.saturating_sub(scroll_info.len() as u16 + 2),
            y: dropdown_area.y + dropdown_area.height - 1,
            width: scroll_info.len() as u16 + 1,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(scroll_info).style(Style::default().fg(Color::DarkGray)),
            scroll_area
        );
    }
}

/// Render LLM section with table at top and detail form at bottom
fn render_llm_section(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.config_editor;

    // Split into table (top) and detail (bottom)
    let table_height = std::cmp::min(form.llm_profiles.len() as u16 + 3, 8);  // Header + profiles + borders
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(table_height),  // Profile table
            Constraint::Min(5),                 // Detail form
        ])
        .split(area);

    // Render profile table
    render_llm_profile_table(f, app, chunks[0]);

    // Render detail form for selected profile
    render_llm_detail_form(f, app, chunks[1]);
}

/// Render LLM profile table
fn render_llm_profile_table(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.config_editor;
    let is_table_focused = form.llm_focus == LlmFocusArea::Table && form.button_focus == 0;

    let border_color = if is_table_focused { Color::Yellow } else { Color::Cyan };
    let block = Block::default()
        .title("LLM Profiles [n=New c=Copy d=Delete Enter=Edit Tab=Switch]")
        .borders(Borders::ALL)
        .style(Style::default().fg(border_color));

    // Handle empty state - no profiles configured
    if form.llm_profiles.is_empty() {
        let empty_msg = vec![
            ListItem::new(""),
            ListItem::new("  No LLM profiles configured.").style(Style::default().fg(Color::Yellow)),
            ListItem::new(""),
            ListItem::new("  Run 'orca_install init' to set up defaults,").style(Style::default().fg(Color::Gray)),
            ListItem::new("  or press 'n' to create a new profile.").style(Style::default().fg(Color::Gray)),
        ];
        let list = List::new(empty_msg).block(block);
        f.render_widget(list, area);
        return;
    }

    // Create header row
    let header_style = Style::default().fg(Color::Cyan).bold();
    let header = format!("{:<15} {:<20} {:<12} {:<10}", "Name", "Model", "Provider", "Budget");

    // Create profile rows
    let mut rows: Vec<ListItem> = vec![ListItem::new(header).style(header_style)];

    for (idx, profile) in form.llm_profiles.iter().enumerate() {
        let is_selected = idx == form.llm_selected_index;
        let style = if is_selected && is_table_focused {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else if is_selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let default_marker = if profile.is_default { "*" } else { " " };
        let budget_display = if profile.budget.is_empty() { "-" } else { &profile.budget };
        let row = format!("{}{:<14} {:<20} {:<12} {:<10}",
            default_marker,
            truncate_str(&profile.name, 14),
            truncate_str(&profile.model, 20),
            truncate_str(&profile.provider, 12),
            truncate_str(budget_display, 10),
        );
        rows.push(ListItem::new(row).style(style));
    }

    let list = List::new(rows).block(block);
    f.render_widget(list, area);
}

/// Render detail form for selected LLM profile
fn render_llm_detail_form(f: &mut Frame, app: &App, area: Rect) {
    let form = &app.config_editor;
    let is_detail_focused = form.llm_focus == LlmFocusArea::Detail && form.button_focus == 0;

    let border_color = if is_detail_focused { Color::Yellow } else { Color::Cyan };
    let title = if let Some(profile) = form.selected_llm_profile() {
        format!("Profile: {} [Tab to switch]", profile.name)
    } else {
        "Profile Details".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(border_color));

    // Calculate inner area
    let inner_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(3),
    };

    f.render_widget(block, area);

    let Some(profile) = form.selected_llm_profile() else {
        return;
    };

    // Render profile fields
    let field_count = LlmProfileEntry::field_count();
    for i in 0..field_count {
        if i >= inner_area.height as usize {
            break;
        }

        let field_y = inner_area.y + i as u16;
        let is_selected = i == form.llm_detail_field && is_detail_focused;
        let is_editing = is_selected && form.editing;
        let is_bool = LlmProfileEntry::is_bool_field(i);

        // Field label
        let label = LlmProfileEntry::field_name(i);
        let label_width = 15.min(inner_area.width / 3) as usize;
        let label_text = format!("{:width$}", label, width = label_width);

        let label_style = if is_selected {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::White)
        };

        let label_area = Rect {
            x: inner_area.x,
            y: field_y,
            width: label_width as u16,
            height: 1,
        };
        f.render_widget(Paragraph::new(label_text).style(label_style), label_area);

        // Field value
        let value = if is_editing {
            format!("{}│", form.edit_buffer)
        } else {
            profile.field_value(i)
        };

        let value_style = if is_editing {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else if is_selected {
            Style::default().fg(Color::Cyan).bold()
        } else if is_bool {
            if profile.field_value(i) == "true" {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            }
        } else {
            Style::default().fg(Color::Gray)
        };

        // Check if this is a dropdown field
        let is_dropdown = LlmProfileEntry::is_dropdown_field(i);

        // Add hints and dropdown indicator for special fields
        let display_value = if is_selected && !is_editing {
            if is_bool {
                format!("{} (Space to toggle)", value)
            } else if is_dropdown {
                format!("{} ▼", value)  // Dropdown indicator
            } else {
                value
            }
        } else if is_dropdown && !is_editing {
            format!("{} ▼", value)  // Show dropdown indicator even when not selected
        } else {
            value
        };

        let value_area = Rect {
            x: inner_area.x + label_width as u16 + 1,
            y: field_y,
            width: inner_area.width.saturating_sub(label_width as u16 + 1),
            height: 1,
        };
        f.render_widget(Paragraph::new(display_value).style(value_style), value_area);
    }

    // Render dropdown overlay if open
    if form.dropdown_open {
        render_field_dropdown(f, app, inner_area);
    }
}

/// Truncate string to fit width
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}

/// Render dropdown overlay for field selection
fn render_field_dropdown(f: &mut Frame, app: &App, form_area: Rect) {
    let form = &app.config_editor;

    let Some(field_index) = form.dropdown_field else {
        return;
    };

    if form.dropdown_options.is_empty() {
        return;
    }

    // Get field name for title
    let field_name = LlmProfileEntry::field_name(field_index);

    // Calculate dropdown position - below the field
    let label_width = 15.min(form_area.width / 3) as u16;
    let dropdown_x = form_area.x + label_width + 1;
    let dropdown_y = form_area.y + field_index as u16 + 1;  // +1 to position below field

    // Calculate dropdown size
    let max_option_len = form.dropdown_options.iter()
        .map(|s| s.len())
        .max()
        .unwrap_or(10);
    let dropdown_width = (max_option_len + 4).min(40) as u16;  // +4 for padding and borders
    let dropdown_height = (form.dropdown_options.len() + 2).min(10) as u16;  // +2 for borders

    // Ensure dropdown fits on screen
    let available_height = f.area().height.saturating_sub(dropdown_y);
    let actual_height = dropdown_height.min(available_height);

    let dropdown_area = Rect {
        x: dropdown_x,
        y: dropdown_y,
        width: dropdown_width.min(form_area.width.saturating_sub(label_width + 1)),
        height: actual_height,
    };

    // Only render if we have space
    if dropdown_area.height < 3 || dropdown_area.width < 5 {
        return;
    }

    // Clear the area
    f.render_widget(Clear, dropdown_area);

    // Create dropdown block
    let block = Block::default()
        .title(format!("Select {}", field_name))
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    // Create list items with highlighting
    let visible_items = (actual_height - 2) as usize;  // -2 for borders
    let scroll_offset = if form.dropdown_selected >= visible_items {
        form.dropdown_selected - visible_items + 1
    } else {
        0
    };

    let list_items: Vec<ListItem> = form.dropdown_options
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_items)
        .map(|(idx, option)| {
            let is_selected = idx == form.dropdown_selected;
            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White).bold()
            } else {
                Style::default().fg(Color::White)
            };

            let marker = if is_selected { "▶ " } else { "  " };
            ListItem::new(format!("{}{}", marker, option)).style(style)
        })
        .collect();

    let list = List::new(list_items).block(block);
    f.render_widget(list, dropdown_area);

    // Show scroll indicator if needed
    if form.dropdown_options.len() > visible_items {
        let scroll_info = format!("{}/{}", form.dropdown_selected + 1, form.dropdown_options.len());
        let scroll_area = Rect {
            x: dropdown_area.x + dropdown_area.width.saturating_sub(scroll_info.len() as u16 + 2),
            y: dropdown_area.y + dropdown_area.height - 1,
            width: scroll_info.len() as u16 + 1,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(scroll_info).style(Style::default().fg(Color::DarkGray)),
            scroll_area
        );
    }
}

