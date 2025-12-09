//! Terminal User Interface (TUI) for Orca
//!
//! Provides an interactive multi-panel TUI for managing workflows, tasks, and viewing logs.

pub mod app;
pub mod dialog;
pub mod forms;
pub mod handler;
pub mod ui;

pub use app::{App, AppState};
pub use dialog::{Dialog, DialogType, render_dialog};
pub use forms::{Form, FormField, FieldType, render_form};
pub use handler::InputHandler;
pub use ui::render_ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event},
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
    // Initialize user database for LLM provider storage
    app.init_user_db().await;

    // Initialize prompt service for LLM interactions
    app.init_prompt_service().await;

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

    // Track previous states for loading config
    let mut prev_dialog_state = app.dialog_state;
    let mut prev_view_mode = app.view_mode;

    // Main event loop
    loop {
        // Set timeout for event polling
        let timeout = Duration::from_millis(100);

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key_event) = event::read()? {
                // Handle input - let handler manage quit via app.state.should_quit
                let handler = InputHandler::new();
                handler.handle_key_event(key_event, app);

                // Check if handler set should_quit
                if app.state.should_quit {
                    break;
                }
            }
        }

        // Track dialog state changes
        if app.dialog_state != prev_dialog_state {
            prev_dialog_state = app.dialog_state;
        }

        // Check if entering config editor mode - load config
        if app.view_mode != prev_view_mode {
            if app.view_mode == app::ViewMode::ConfigEditor {
                app.open_config_editor().await;
            }
            prev_view_mode = app.view_mode;
        }

        // Handle pending config editor save (Execution, Logging sections -> file)
        if app.pending_config_save {
            app.pending_config_save = false;
            app.save_and_close_config_editor().await;
        }

        // Handle pending LLM config save (LLM section -> database)
        if app.pending_llm_save {
            app.pending_llm_save = false;
            app.save_llm_config().await;
        }

        // Handle pending backup operation
        if app.pending_backup {
            app.pending_backup = false;
            app.handle_backup().await;
        }

        // Handle pending restore operation
        if app.pending_restore {
            app.pending_restore = false;
            app.handle_restore().await;
        }

        // Handle pending export operation
        if app.pending_export {
            app.pending_export = false;
            app.handle_export().await;
        }

        // Handle pending import operation
        if app.pending_import {
            app.pending_import = false;
            app.handle_import().await;
        }

        // Handle pending model query (async fetch from provider)
        if app.pending_model_query {
            app.query_provider_models().await;
        }

        // Handle pending prompt submission (send to LLM)
        if app.pending_prompt_submit {
            app.pending_prompt_submit = false;
            app.handle_prompt_submit().await;
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
