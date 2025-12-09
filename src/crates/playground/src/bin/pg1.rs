//! pg1 - Mock Helpdesk Ticket Application
//!
//! A TUI application demonstrating ratatui keybindings with menus and enter keys.
//! Features:
//! - Main menu navigation with arrow keys
//! - Create new tickets with form input
//! - View and manage existing tickets
//! - Priority and status selection

use anyhow::Result;
use chrono::{DateTime, Local};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap,
    },
};
use std::io;
use uuid::Uuid;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Priority {
    fn as_str(&self) -> &'static str {
        match self {
            Priority::Low => "Low",
            Priority::Medium => "Medium",
            Priority::High => "High",
            Priority::Critical => "Critical",
        }
    }

    fn style(&self) -> Style {
        match self {
            Priority::Low => Style::default().fg(Color::Green),
            Priority::Medium => Style::default().fg(Color::Yellow),
            Priority::High => Style::default().fg(Color::Rgb(255, 165, 0)), // Orange
            Priority::Critical => Style::default().fg(Color::Red).bold(),
        }
    }

    fn all() -> Vec<Priority> {
        vec![
            Priority::Low,
            Priority::Medium,
            Priority::High,
            Priority::Critical,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Status {
    Open,
    InProgress,
    Pending,
    Resolved,
    Closed,
}

impl Status {
    fn as_str(&self) -> &'static str {
        match self {
            Status::Open => "Open",
            Status::InProgress => "In Progress",
            Status::Pending => "Pending",
            Status::Resolved => "Resolved",
            Status::Closed => "Closed",
        }
    }

    fn style(&self) -> Style {
        match self {
            Status::Open => Style::default().fg(Color::Cyan),
            Status::InProgress => Style::default().fg(Color::Yellow),
            Status::Pending => Style::default().fg(Color::Magenta),
            Status::Resolved => Style::default().fg(Color::Green),
            Status::Closed => Style::default().fg(Color::Gray),
        }
    }

    fn all() -> Vec<Status> {
        vec![
            Status::Open,
            Status::InProgress,
            Status::Pending,
            Status::Resolved,
            Status::Closed,
        ]
    }
}

#[derive(Debug, Clone)]
struct Ticket {
    id: String,
    title: String,
    description: String,
    priority: Priority,
    status: Status,
    created_at: DateTime<Local>,
    updated_at: DateTime<Local>,
}

impl Ticket {
    fn new(title: String, description: String, priority: Priority) -> Self {
        let now = Local::now();
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            title,
            description,
            priority,
            status: Status::Open,
            created_at: now,
            updated_at: now,
        }
    }
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum Screen {
    MainMenu,
    TicketList,
    CreateTicket,
    ViewTicket,
    HotkeyTest,
    Help,
}

#[derive(Debug, Clone, PartialEq)]
enum CreateTicketField {
    Title,
    Description,
    Priority,
}

struct App {
    // Navigation state
    current_screen: Screen,
    should_quit: bool,

    // Main menu
    menu_items: Vec<&'static str>,
    menu_state: ListState,

    // Ticket list
    tickets: Vec<Ticket>,
    ticket_table_state: TableState,

    // Create ticket form
    create_field: CreateTicketField,
    create_title: String,
    create_description: String,
    create_priority_index: usize,

    // View ticket
    selected_ticket_index: Option<usize>,

    // Status message
    status_message: Option<String>,

    // Hotkey test state
    hotkey_log: Vec<String>,
    last_key_pressed: Option<String>,
}

impl App {
    fn new() -> Self {
        let mut menu_state = ListState::default();
        menu_state.select(Some(0));

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        // Create some sample tickets
        let sample_tickets = vec![
            Ticket {
                id: "ABC12345".to_string(),
                title: "Login page not loading".to_string(),
                description: "Users report that the login page shows a blank screen on Firefox."
                    .to_string(),
                priority: Priority::High,
                status: Status::InProgress,
                created_at: Local::now(),
                updated_at: Local::now(),
            },
            Ticket {
                id: "DEF67890".to_string(),
                title: "Password reset email not received".to_string(),
                description: "Customer waited 30 minutes but never received the password reset email. Checked spam folder.".to_string(),
                priority: Priority::Medium,
                status: Status::Open,
                created_at: Local::now(),
                updated_at: Local::now(),
            },
            Ticket {
                id: "GHI11111".to_string(),
                title: "Dashboard charts not displaying".to_string(),
                description: "The analytics dashboard shows 'No data available' even though there should be data.".to_string(),
                priority: Priority::Low,
                status: Status::Pending,
                created_at: Local::now(),
                updated_at: Local::now(),
            },
        ];

        Self {
            current_screen: Screen::MainMenu,
            should_quit: false,
            menu_items: vec![
                "Create New Ticket",
                "View All Tickets",
                "Hotkey Test (Ctrl+ keys)",
                "Help",
                "Exit",
            ],
            menu_state,
            tickets: sample_tickets,
            ticket_table_state: table_state,
            create_field: CreateTicketField::Title,
            create_title: String::new(),
            create_description: String::new(),
            create_priority_index: 0,
            selected_ticket_index: None,
            status_message: None,
            hotkey_log: Vec::new(),
            last_key_pressed: None,
        }
    }

    fn log_hotkey(&mut self, key_desc: String) {
        let timestamp = Local::now().format("%H:%M:%S").to_string();
        let entry = format!("[{}] {}", timestamp, key_desc);
        self.hotkey_log.push(entry);
        self.last_key_pressed = Some(key_desc);
        // Keep only last 20 entries
        if self.hotkey_log.len() > 20 {
            self.hotkey_log.remove(0);
        }
    }

    fn clear_hotkey_log(&mut self) {
        self.hotkey_log.clear();
        self.last_key_pressed = None;
    }

    fn menu_next(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => {
                if i >= self.menu_items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    fn menu_previous(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.menu_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    fn ticket_next(&mut self) {
        if self.tickets.is_empty() {
            return;
        }
        let i = match self.ticket_table_state.selected() {
            Some(i) => {
                if i >= self.tickets.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.ticket_table_state.select(Some(i));
    }

    fn ticket_previous(&mut self) {
        if self.tickets.is_empty() {
            return;
        }
        let i = match self.ticket_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tickets.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.ticket_table_state.select(Some(i));
    }

    fn next_create_field(&mut self) {
        self.create_field = match self.create_field {
            CreateTicketField::Title => CreateTicketField::Description,
            CreateTicketField::Description => CreateTicketField::Priority,
            CreateTicketField::Priority => CreateTicketField::Title,
        };
    }

    fn prev_create_field(&mut self) {
        self.create_field = match self.create_field {
            CreateTicketField::Title => CreateTicketField::Priority,
            CreateTicketField::Description => CreateTicketField::Title,
            CreateTicketField::Priority => CreateTicketField::Description,
        };
    }

    fn submit_ticket(&mut self) {
        if self.create_title.trim().is_empty() {
            self.status_message = Some("Title cannot be empty!".to_string());
            return;
        }

        let priority = Priority::all()[self.create_priority_index].clone();
        let ticket = Ticket::new(
            self.create_title.clone(),
            self.create_description.clone(),
            priority,
        );
        let ticket_id = ticket.id.clone();
        self.tickets.push(ticket);

        // Reset form
        self.create_title.clear();
        self.create_description.clear();
        self.create_priority_index = 0;
        self.create_field = CreateTicketField::Title;

        self.status_message = Some(format!("Ticket {} created successfully!", ticket_id));
        self.current_screen = Screen::TicketList;

        // Select the new ticket
        if !self.tickets.is_empty() {
            self.ticket_table_state
                .select(Some(self.tickets.len() - 1));
        }
    }
}

// ============================================================================
// Event Handling
// ============================================================================

/// Handle global hotkeys that work from any screen
/// Returns true if a global hotkey was handled
fn handle_global_hotkeys(app: &mut App, key: KeyCode, modifiers: KeyModifiers) -> bool {
    if modifiers.contains(KeyModifiers::CONTROL) {
        match key {
            KeyCode::Char('1') => {
                app.current_screen = Screen::MainMenu;
                app.status_message = Some("Switched to Main Menu".to_string());
                true
            }
            KeyCode::Char('2') => {
                app.current_screen = Screen::TicketList;
                app.status_message = Some("Switched to Ticket List".to_string());
                true
            }
            KeyCode::Char('3') => {
                app.current_screen = Screen::HotkeyTest;
                app.status_message = Some("Switched to Hotkey Test".to_string());
                true
            }
            KeyCode::Char('4') => {
                app.current_screen = Screen::Help;
                app.status_message = Some("Switched to Help".to_string());
                true
            }
            KeyCode::Char('q') => {
                app.should_quit = true;
                true
            }
            KeyCode::Char('c') => {
                // Ctrl+C always quits
                app.should_quit = true;
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            // Only handle key press events (not release)
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            // Clear status message on any key press (except in hotkey test)
            if app.current_screen != Screen::HotkeyTest {
                app.status_message = None;
            }

            // Try global hotkeys first (except in hotkey test mode)
            if app.current_screen != Screen::HotkeyTest {
                if handle_global_hotkeys(app, key.code, key.modifiers) {
                    return Ok(());
                }
            }

            // Then screen-specific handlers
            match app.current_screen {
                Screen::MainMenu => handle_main_menu(app, key.code, key.modifiers),
                Screen::TicketList => handle_ticket_list(app, key.code, key.modifiers),
                Screen::CreateTicket => handle_create_ticket(app, key.code, key.modifiers),
                Screen::ViewTicket => handle_view_ticket(app, key.code, key.modifiers),
                Screen::HotkeyTest => handle_hotkey_test(app, key.code, key.modifiers),
                Screen::Help => handle_help(app, key.code, key.modifiers),
            }
        }
    }
    Ok(())
}

fn handle_main_menu(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    // Handle Ctrl+C to quit from anywhere
    if modifiers.contains(KeyModifiers::CONTROL) && key == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Up | KeyCode::Char('k') => app.menu_previous(),
        KeyCode::Down | KeyCode::Char('j') => app.menu_next(),
        KeyCode::Enter => {
            if let Some(i) = app.menu_state.selected() {
                match i {
                    0 => app.current_screen = Screen::CreateTicket,
                    1 => app.current_screen = Screen::TicketList,
                    2 => app.current_screen = Screen::HotkeyTest,
                    3 => app.current_screen = Screen::Help,
                    4 => app.should_quit = true,
                    _ => {}
                }
            }
        }
        KeyCode::Char('1') => app.current_screen = Screen::CreateTicket,
        KeyCode::Char('2') => app.current_screen = Screen::TicketList,
        KeyCode::Char('3') => app.current_screen = Screen::HotkeyTest,
        KeyCode::Char('4') => app.current_screen = Screen::Help,
        KeyCode::Char('5') => app.should_quit = true,
        _ => {}
    }
}

fn handle_ticket_list(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    // Handle Ctrl+ hotkeys
    if modifiers.contains(KeyModifiers::CONTROL) {
        match key {
            KeyCode::Char('n') => {
                app.current_screen = Screen::CreateTicket;
                app.status_message = Some("Creating new ticket...".to_string());
            }
            KeyCode::Char('d') => {
                if let Some(i) = app.ticket_table_state.selected() {
                    if i < app.tickets.len() {
                        let ticket_id = app.tickets[i].id.clone();
                        app.tickets.remove(i);
                        app.status_message = Some(format!("Ticket {} deleted", ticket_id));
                        if !app.tickets.is_empty() {
                            let new_i = if i >= app.tickets.len() {
                                app.tickets.len() - 1
                            } else {
                                i
                            };
                            app.ticket_table_state.select(Some(new_i));
                        }
                    }
                }
            }
            KeyCode::Char('r') => {
                app.status_message = Some("Refreshed ticket list".to_string());
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.current_screen = Screen::MainMenu,
        KeyCode::Up | KeyCode::Char('k') => app.ticket_previous(),
        KeyCode::Down | KeyCode::Char('j') => app.ticket_next(),
        KeyCode::Enter => {
            if let Some(i) = app.ticket_table_state.selected() {
                if i < app.tickets.len() {
                    app.selected_ticket_index = Some(i);
                    app.current_screen = Screen::ViewTicket;
                }
            }
        }
        KeyCode::Char('n') => app.current_screen = Screen::CreateTicket,
        KeyCode::Char('d') | KeyCode::Delete => {
            if let Some(i) = app.ticket_table_state.selected() {
                if i < app.tickets.len() {
                    let ticket_id = app.tickets[i].id.clone();
                    app.tickets.remove(i);
                    app.status_message = Some(format!("Ticket {} deleted", ticket_id));
                    if !app.tickets.is_empty() {
                        let new_i = if i >= app.tickets.len() {
                            app.tickets.len() - 1
                        } else {
                            i
                        };
                        app.ticket_table_state.select(Some(new_i));
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_create_ticket(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    // Handle Ctrl+ hotkeys
    if modifiers.contains(KeyModifiers::CONTROL) {
        match key {
            KeyCode::Char('s') => {
                // Submit the ticket
                app.submit_ticket();
            }
            KeyCode::Char('w') => {
                // Cancel and go back
                app.create_title.clear();
                app.create_description.clear();
                app.create_priority_index = 0;
                app.create_field = CreateTicketField::Title;
                app.current_screen = Screen::MainMenu;
                app.status_message = Some("Ticket creation cancelled".to_string());
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Esc => {
            // Reset form and go back
            app.create_title.clear();
            app.create_description.clear();
            app.create_priority_index = 0;
            app.create_field = CreateTicketField::Title;
            app.current_screen = Screen::MainMenu;
        }
        KeyCode::Tab => app.next_create_field(),
        KeyCode::BackTab => app.prev_create_field(),
        KeyCode::Enter => {
            match app.create_field {
                CreateTicketField::Priority => {
                    // Submit the ticket
                    app.submit_ticket();
                }
                _ => {
                    // Move to next field
                    app.next_create_field();
                }
            }
        }
        KeyCode::Char(c) => match app.create_field {
            CreateTicketField::Title => app.create_title.push(c),
            CreateTicketField::Description => app.create_description.push(c),
            CreateTicketField::Priority => {
                // Use number keys for priority
                if let Some(digit) = c.to_digit(10) {
                    let idx = digit as usize;
                    if (1..=4).contains(&idx) {
                        app.create_priority_index = idx - 1;
                    }
                }
            }
        },
        KeyCode::Backspace => match app.create_field {
            CreateTicketField::Title => {
                app.create_title.pop();
            }
            CreateTicketField::Description => {
                app.create_description.pop();
            }
            CreateTicketField::Priority => {}
        },
        KeyCode::Left if matches!(app.create_field, CreateTicketField::Priority) => {
            if app.create_priority_index > 0 {
                app.create_priority_index -= 1;
            }
        }
        KeyCode::Right if matches!(app.create_field, CreateTicketField::Priority) => {
            if app.create_priority_index < Priority::all().len() - 1 {
                app.create_priority_index += 1;
            }
        }
        _ => {}
    }
}

fn handle_view_ticket(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    // Handle Ctrl+ hotkeys for status and priority cycling
    if modifiers.contains(KeyModifiers::CONTROL) {
        match key {
            KeyCode::Char('s') => {
                // Cycle status
                if let Some(i) = app.selected_ticket_index {
                    if let Some(ticket) = app.tickets.get_mut(i) {
                        let statuses = Status::all();
                        let current_idx = statuses.iter().position(|s| s == &ticket.status).unwrap_or(0);
                        let next_idx = (current_idx + 1) % statuses.len();
                        ticket.status = statuses[next_idx].clone();
                        ticket.updated_at = Local::now();
                        app.status_message = Some(format!("Status changed to {}", ticket.status.as_str()));
                    }
                }
            }
            KeyCode::Char('p') => {
                // Cycle priority
                if let Some(i) = app.selected_ticket_index {
                    if let Some(ticket) = app.tickets.get_mut(i) {
                        let priorities = Priority::all();
                        let current_idx = priorities.iter().position(|p| p == &ticket.priority).unwrap_or(0);
                        let next_idx = (current_idx + 1) % priorities.len();
                        ticket.priority = priorities[next_idx].clone();
                        ticket.updated_at = Local::now();
                        app.status_message = Some(format!("Priority changed to {}", ticket.priority.as_str()));
                    }
                }
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
            app.selected_ticket_index = None;
            app.current_screen = Screen::TicketList;
        }
        KeyCode::Char('s') => {
            // Cycle status (also works without Ctrl for convenience)
            if let Some(i) = app.selected_ticket_index {
                if let Some(ticket) = app.tickets.get_mut(i) {
                    let statuses = Status::all();
                    let current_idx = statuses.iter().position(|s| s == &ticket.status).unwrap_or(0);
                    let next_idx = (current_idx + 1) % statuses.len();
                    ticket.status = statuses[next_idx].clone();
                    ticket.updated_at = Local::now();
                    app.status_message = Some(format!("Status changed to {}", ticket.status.as_str()));
                }
            }
        }
        KeyCode::Char('p') => {
            // Cycle priority (also works without Ctrl for convenience)
            if let Some(i) = app.selected_ticket_index {
                if let Some(ticket) = app.tickets.get_mut(i) {
                    let priorities = Priority::all();
                    let current_idx = priorities.iter().position(|p| p == &ticket.priority).unwrap_or(0);
                    let next_idx = (current_idx + 1) % priorities.len();
                    ticket.priority = priorities[next_idx].clone();
                    ticket.updated_at = Local::now();
                    app.status_message = Some(format!("Priority changed to {}", ticket.priority.as_str()));
                }
            }
        }
        _ => {}
    }
}

fn handle_help(app: &mut App, key: KeyCode, _modifiers: KeyModifiers) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
            app.current_screen = Screen::MainMenu;
        }
        _ => {}
    }
}

fn handle_hotkey_test(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    // Build description of the key press
    let mut mod_parts: Vec<&str> = Vec::new();

    if modifiers.contains(KeyModifiers::CONTROL) {
        mod_parts.push("Ctrl");
    }
    if modifiers.contains(KeyModifiers::ALT) {
        mod_parts.push("Alt");
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        mod_parts.push("Shift");
    }
    if modifiers.contains(KeyModifiers::SUPER) {
        mod_parts.push("Super");
    }
    if modifiers.contains(KeyModifiers::HYPER) {
        mod_parts.push("Hyper");
    }
    if modifiers.contains(KeyModifiers::META) {
        mod_parts.push("Meta");
    }

    let key_name = match key {
        KeyCode::Char(c) => {
            // Show both the char and its code for debugging
            if c.is_ascii_control() {
                format!("Char(0x{:02x})", c as u8)
            } else {
                format!("'{}'", c)
            }
        }
        KeyCode::F(n) => format!("F{}", n),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "BackTab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Null => "Null".to_string(),
        KeyCode::CapsLock => "CapsLock".to_string(),
        KeyCode::ScrollLock => "ScrollLock".to_string(),
        KeyCode::NumLock => "NumLock".to_string(),
        KeyCode::PrintScreen => "PrintScreen".to_string(),
        KeyCode::Pause => "Pause".to_string(),
        KeyCode::Menu => "Menu".to_string(),
        KeyCode::KeypadBegin => "KeypadBegin".to_string(),
        _ => format!("{:?}", key),
    };

    // Build the full description
    let key_desc = if mod_parts.is_empty() {
        key_name.clone()
    } else {
        format!("{}+{}", mod_parts.join("+"), key_name)
    };

    // Also show raw modifier bits for debugging
    let debug_info = format!("{} [mods: 0x{:02x}]", key_desc, modifiers.bits());

    // Handle escape to go back (only if no modifiers)
    if key == KeyCode::Esc && modifiers.is_empty() {
        app.current_screen = Screen::MainMenu;
        return;
    }

    // Handle 'c' key to clear log (only if no modifiers)
    if key == KeyCode::Char('c') && modifiers.is_empty() {
        app.clear_hotkey_log();
        app.status_message = Some("Log cleared".to_string());
        return;
    }

    // Log the key press with debug info
    app.log_hotkey(debug_info);
}

// ============================================================================
// UI Rendering
// ============================================================================

fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Footer/Status
        ])
        .split(frame.area());

    // Header
    let header = Paragraph::new("Helpdesk Ticket System")
        .style(Style::default().fg(Color::Cyan).bold())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(header, chunks[0]);

    // Content based on current screen
    match app.current_screen {
        Screen::MainMenu => render_main_menu(frame, app, chunks[1]),
        Screen::TicketList => render_ticket_list(frame, app, chunks[1]),
        Screen::CreateTicket => render_create_ticket(frame, app, chunks[1]),
        Screen::ViewTicket => render_view_ticket(frame, app, chunks[1]),
        Screen::HotkeyTest => render_hotkey_test(frame, app, chunks[1]),
        Screen::Help => render_help(frame, chunks[1]),
    }

    // Footer with status message and keybinds
    let footer_text = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else {
        get_keybind_hint(&app.current_screen)
    };

    let footer_style = if app.status_message.is_some() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let footer = Paragraph::new(footer_text)
        .style(footer_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

fn get_keybind_hint(screen: &Screen) -> String {
    match screen {
        Screen::MainMenu => "↑↓: Navigate | Enter: Select | 1-5: Quick | Ctrl+1-4: Go to | Ctrl+Q: Quit".to_string(),
        Screen::TicketList => "Ctrl+N: New | Ctrl+D: Delete | Ctrl+R: Refresh | Enter: View | Esc: Back".to_string(),
        Screen::CreateTicket => "Ctrl+S: Submit | Ctrl+W: Cancel | Tab/Shift+Tab: Fields | Esc: Cancel".to_string(),
        Screen::ViewTicket => "Ctrl+S: Status | Ctrl+P: Priority | Enter/Esc: Back".to_string(),
        Screen::HotkeyTest => "Press any key combo to test | c: Clear log | Esc: Back".to_string(),
        Screen::Help => "Ctrl+1-4: Navigation | Enter/Esc: Back to Menu".to_string(),
    }
}

fn render_main_menu(frame: &mut Frame, app: &mut App, area: Rect) {
    let menu_area = centered_rect(50, 60, area);

    let items: Vec<ListItem> = app
        .menu_items
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            let prefix = format!("[{}] ", i + 1);
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                Span::raw(item),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Main Menu ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .bold(),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, menu_area, &mut app.menu_state);
}

fn render_ticket_list(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.tickets.is_empty() {
        let empty_msg = Paragraph::new("No tickets found. Press 'n' to create a new ticket.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Tickets ")
                    .borders(Borders::ALL),
            );
        frame.render_widget(empty_msg, area);
        return;
    }

    let header_cells = ["ID", "Title", "Priority", "Status", "Created"]
        .iter()
        .map(|h| {
            ratatui::widgets::Cell::from(*h).style(Style::default().fg(Color::Yellow).bold())
        });
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app
        .tickets
        .iter()
        .map(|ticket| {
            let cells = vec![
                ratatui::widgets::Cell::from(ticket.id.clone()),
                ratatui::widgets::Cell::from(truncate_string(&ticket.title, 30)),
                ratatui::widgets::Cell::from(ticket.priority.as_str())
                    .style(ticket.priority.style()),
                ratatui::widgets::Cell::from(ticket.status.as_str()).style(ticket.status.style()),
                ratatui::widgets::Cell::from(ticket.created_at.format("%Y-%m-%d %H:%M").to_string()),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(18),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!(" Tickets ({}) ", app.tickets.len()))
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">> ");

    frame.render_stateful_widget(table, area, &mut app.ticket_table_state);
}

fn render_create_ticket(frame: &mut Frame, app: &mut App, area: Rect) {
    let form_area = centered_rect(70, 80, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(5), // Description
            Constraint::Length(3), // Priority
            Constraint::Min(0),    // Spacing
        ])
        .split(form_area);

    // Form container
    let form_block = Block::default()
        .title(" Create New Ticket ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(form_block, form_area);

    // Title field
    let title_style = if app.create_field == CreateTicketField::Title {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let title_block = Block::default()
        .title(" Title ")
        .borders(Borders::ALL)
        .border_style(title_style);
    let title_text = if app.create_title.is_empty() && app.create_field != CreateTicketField::Title {
        Paragraph::new("(enter title)").style(Style::default().fg(Color::DarkGray))
    } else {
        let display_text = if app.create_field == CreateTicketField::Title {
            format!("{}_", app.create_title)
        } else {
            app.create_title.clone()
        };
        Paragraph::new(display_text)
    };
    frame.render_widget(title_text.block(title_block), chunks[0]);

    // Description field
    let desc_style = if app.create_field == CreateTicketField::Description {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let desc_block = Block::default()
        .title(" Description ")
        .borders(Borders::ALL)
        .border_style(desc_style);
    let desc_text = if app.create_description.is_empty()
        && app.create_field != CreateTicketField::Description
    {
        Paragraph::new("(enter description)").style(Style::default().fg(Color::DarkGray))
    } else {
        let display_text = if app.create_field == CreateTicketField::Description {
            format!("{}_", app.create_description)
        } else {
            app.create_description.clone()
        };
        Paragraph::new(display_text).wrap(Wrap { trim: false })
    };
    frame.render_widget(desc_text.block(desc_block), chunks[1]);

    // Priority selection
    let priority_style = if app.create_field == CreateTicketField::Priority {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let priority_block = Block::default()
        .title(" Priority (1-4 or Left/Right) ")
        .borders(Borders::ALL)
        .border_style(priority_style);

    let priorities = Priority::all();
    let priority_spans: Vec<Span> = priorities
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let prefix = format!("[{}] ", i + 1);
            let selected = i == app.create_priority_index;
            if selected {
                Span::styled(
                    format!("{}{}  ", prefix, p.as_str()),
                    p.style().reversed(),
                )
            } else {
                Span::styled(format!("{}{}  ", prefix, p.as_str()), p.style())
            }
        })
        .collect();

    let priority_line = Line::from(priority_spans);
    let priority_para = Paragraph::new(priority_line).block(priority_block);
    frame.render_widget(priority_para, chunks[2]);
}

fn render_view_ticket(frame: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(80, 80, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    if let Some(idx) = app.selected_ticket_index {
        if let Some(ticket) = app.tickets.get(idx) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3), // Title + ID
                    Constraint::Length(3), // Status + Priority
                    Constraint::Min(5),    // Description
                    Constraint::Length(2), // Timestamps
                ])
                .split(popup_area);

            // Container block
            let container = Block::default()
                .title(format!(" Ticket: {} ", ticket.id))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            frame.render_widget(container, popup_area);

            // Title
            let title = Paragraph::new(ticket.title.clone())
                .style(Style::default().bold())
                .block(Block::default().title(" Title ").borders(Borders::ALL));
            frame.render_widget(title, chunks[0]);

            // Status and Priority
            let status_priority = Paragraph::new(Line::from(vec![
                Span::raw("Status: "),
                Span::styled(ticket.status.as_str(), ticket.status.style()),
                Span::raw("  |  Priority: "),
                Span::styled(ticket.priority.as_str(), ticket.priority.style()),
            ]))
            .block(
                Block::default()
                    .title(" Status & Priority ")
                    .borders(Borders::ALL),
            );
            frame.render_widget(status_priority, chunks[1]);

            // Description
            let description = Paragraph::new(ticket.description.clone())
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .title(" Description ")
                        .borders(Borders::ALL),
                );
            frame.render_widget(description, chunks[2]);

            // Timestamps
            let timestamps = Paragraph::new(format!(
                "Created: {}  |  Updated: {}",
                ticket.created_at.format("%Y-%m-%d %H:%M:%S"),
                ticket.updated_at.format("%Y-%m-%d %H:%M:%S")
            ))
            .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(timestamps, chunks[3]);
        }
    }
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_area = centered_rect(70, 80, area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled("Global Navigation (work anywhere):", Style::default().fg(Color::Yellow).bold())),
        Line::from("  Ctrl+1    - Go to Main Menu"),
        Line::from("  Ctrl+2    - Go to Ticket List"),
        Line::from("  Ctrl+3    - Go to Hotkey Test"),
        Line::from("  Ctrl+4    - Go to Help"),
        Line::from("  Ctrl+Q    - Quit application"),
        Line::from(""),
        Line::from(Span::styled("General:", Style::default().bold())),
        Line::from("  q, Esc    - Go back / Quit"),
        Line::from("  Enter     - Select / Confirm"),
        Line::from("  Up/Down   - Navigate (also j/k)"),
        Line::from(""),
        Line::from(Span::styled("Main Menu:", Style::default().bold())),
        Line::from("  1-5       - Quick select menu item"),
        Line::from(""),
        Line::from(Span::styled("Ticket List:", Style::default().bold())),
        Line::from("  Ctrl+N    - Create new ticket"),
        Line::from("  Ctrl+D    - Delete selected ticket"),
        Line::from("  Ctrl+R    - Refresh list"),
        Line::from("  n         - Create new ticket"),
        Line::from("  d/Delete  - Delete selected ticket"),
        Line::from("  Enter     - View ticket details"),
        Line::from(""),
        Line::from(Span::styled("Create Ticket:", Style::default().bold())),
        Line::from("  Ctrl+S    - Submit ticket"),
        Line::from("  Ctrl+W    - Cancel and go back"),
        Line::from("  Tab       - Next field"),
        Line::from("  Shift+Tab - Previous field"),
        Line::from("  1-4       - Select priority (in Priority field)"),
        Line::from("  Left/Right- Change priority"),
        Line::from(""),
        Line::from(Span::styled("View Ticket:", Style::default().bold())),
        Line::from("  Ctrl+S/s  - Cycle status"),
        Line::from("  Ctrl+P/p  - Cycle priority"),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(help, help_area);
}

fn render_hotkey_test(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Left panel - Instructions and last key
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(18),
            Constraint::Min(0),
        ])
        .split(chunks[0]);

    // Instructions
    let instructions = vec![
        Line::from(Span::styled(
            "Hotkey Test Mode",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from("Press any key combination to test detection."),
        Line::from("Shows raw modifier bits for debugging."),
        Line::from(""),
        Line::from(Span::styled("Working combos:", Style::default().fg(Color::Green).bold())),
        Line::from("  Ctrl+s, Ctrl+n, Ctrl+o, Ctrl+Shift+s"),
        Line::from(""),
        Line::from(Span::styled("Terminal Limitations:", Style::default().fg(Color::Red).bold())),
        Line::from("  - Ctrl+Enter often just sends Enter"),
        Line::from("  - Some Ctrl+key combos are intercepted"),
        Line::from("  - Alt combos may not work (OS captures)"),
        Line::from(""),
        Line::from(Span::styled("Note:", Style::default().fg(Color::Yellow))),
        Line::from("  Ctrl+Shift+S shows as Ctrl+Shift+'s'"),
        Line::from("  (lowercase char with Shift modifier)"),
    ];

    let instructions_widget = Paragraph::new(instructions)
        .block(
            Block::default()
                .title(" Instructions ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(instructions_widget, left_chunks[0]);

    // Last key pressed (large display)
    let last_key_display = if let Some(ref key) = app.last_key_pressed {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                key.clone(),
                Style::default()
                    .fg(Color::Green)
                    .bold()
                    .add_modifier(Modifier::REVERSED),
            )),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "Press a key...",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
        ]
    };

    let last_key_widget = Paragraph::new(last_key_display)
        .block(
            Block::default()
                .title(" Last Key Pressed ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .alignment(Alignment::Center);
    frame.render_widget(last_key_widget, left_chunks[1]);

    // Right panel - Key log
    let log_items: Vec<ListItem> = app
        .hotkey_log
        .iter()
        .rev()
        .map(|entry| {
            let style = if entry.contains("Ctrl") {
                Style::default().fg(Color::Yellow)
            } else if entry.contains("Alt") {
                Style::default().fg(Color::Magenta)
            } else if entry.contains("Shift") {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(entry.clone(), style)))
        })
        .collect();

    let log_widget = List::new(log_items)
        .block(
            Block::default()
                .title(format!(" Key Log ({}) ", app.hotkey_log.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        );
    frame.render_widget(log_widget, chunks[1]);
}

// ============================================================================
// Utility Functions
// ============================================================================

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        handle_events(app)?;

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
