//! TUI module for ratatui-based terminal interface
//!
//! Provides terminal UI with event loop, app state, and keyboard handling.

use crate::error::Result;
use std::path::PathBuf;

pub mod app;
pub mod events;
pub mod grpc_client;
pub mod ui;

pub use app::{App, AppState, View, TaskItem, WorkflowItem};
pub use events::{EventHandler, Event};
pub use grpc_client::TuiGrpcClient;

/// TUI configuration
#[derive(Debug, Clone)]
pub struct TuiConfig {
    pub server_url: String,
    pub workspace: PathBuf,
    pub verbose: bool,
}

impl TuiConfig {
    /// Create TUI config from environment variables
    pub fn from_env(server_url: String, workspace: PathBuf, verbose: bool) -> Self {
        Self {
            server_url,
            workspace,
            verbose,
        }
    }
}

/// Run the TUI application
pub async fn run(config: TuiConfig) -> Result<()> {
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io;
    use tracing::{debug, info};

    info!("Starting TUI mode");
    debug!("Server URL: {}", config.server_url);
    debug!("Workspace: {}", config.workspace.display());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app with config
    let mut app = App::new(config.clone());

    // Initial data load
    if let Err(e) = app.refresh_tasks().await {
        tracing::warn!("Initial task load failed: {}", e);
    }
    if let Err(e) = app.refresh_workflows().await {
        tracing::warn!("Initial workflow load failed: {}", e);
    }

    // Create event handler
    let event_handler = EventHandler::new(); // 60 FPS tick rate

    // Main event loop
    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Handle events
        match event_handler.next() {
            Ok(Event::Tick) => {
                // Auto-refresh on tick
                if app.should_refresh() {
                    if let Err(e) = app.refresh_tasks().await {
                        tracing::debug!("Task refresh failed: {}", e);
                    }
                    if let Err(e) = app.refresh_workflows().await {
                        tracing::debug!("Workflow refresh failed: {}", e);
                    }
                }
            }
            Ok(Event::Key(key)) => {
                use crossterm::event::KeyCode;
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if app.view() == View::TaskList || app.view() == View::WorkflowList || app.view() == View::Help {
                            break;
                        } else {
                            app.deselect_item();
                        }
                    }
                    KeyCode::Char('r') => {
                        // Manual refresh
                        app.refresh_tasks().await?;
                        app.refresh_workflows().await?;
                    }
                    KeyCode::Tab => app.next_view(),
                    KeyCode::BackTab => app.previous_view(),
                    KeyCode::Up => app.previous_item(),
                    KeyCode::Down => app.next_item(),
                    KeyCode::Enter => app.select_item(),
                    _ => {}
                }
            }
            Ok(Event::Resize(_, _)) => {
                // Terminal resized, will redraw on next loop
            }
            Ok(Event::Quit) => break,
            Ok(Event::Error(e)) => {
                tracing::error!("Event error: {}", e);
            }
            Err(e) => {
                tracing::error!("Event receive error: {}", e);
                break;
            }
        }

        if app.should_quit() {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    info!("TUI mode exited");
    Ok(())
}
