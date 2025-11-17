//! Terminal User Interface (TUI) for Orca
//!
//! Provides an interactive multi-panel TUI for managing workflows, tasks, and viewing logs.

pub mod app;
pub mod dialog;
pub mod handler;
pub mod ui;

pub use app::{App, AppState};
pub use dialog::{Dialog, DialogType, render_dialog};
pub use handler::InputHandler;
pub use ui::render_ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::time::Duration;

/// Run the interactive TUI
pub async fn run_tui(app: &mut App) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initial draw
    terminal.draw(|f| {
        ui::render_ui(f, app);
    })?;

    // Main event loop
    loop {
        // Set timeout for event polling
        let timeout = Duration::from_millis(100);

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key_event) = event::read()? {
                if should_quit(&key_event) {
                    break;
                }

                // Handle input
                let handler = InputHandler::new();
                handler.handle_key_event(key_event, app);
            }
        }

        // Redraw
        terminal.draw(|f| {
            ui::render_ui(f, app);
        })?;
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// Check if quit key was pressed
fn should_quit(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('q') || key.code == KeyCode::Esc
}
