//! TUI panel modules

pub mod status;
pub mod commands;
pub mod output;
pub mod logs;

pub use status::StatusPanel;
pub use commands::CommandsPanel;
pub use output::OutputPanel;
pub use logs::LogsPanel;
