//! CLI framework for aco client
//!
//! Provides command-line interface with subcommands for tasks, workflows, and authentication.

pub mod handlers;
pub mod output;
pub mod workflow;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// aco - Orchestrator client application
#[derive(Parser, Debug)]
#[command(name = "aco")]
#[command(version = "0.1.0")]
#[command(author = "aco")]
#[command(about = "Orchestrator client for task and workflow management")]
#[command(long_about = None)]
pub struct Cli {
    /// Orchestrator server URL
    #[arg(long, global = true, env = "ACO_SERVER")]
    pub server: Option<String>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output format (json, table, plain)
    #[arg(long, global = true, default_value = "table")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Output format options
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Json,
    Table,
    Plain,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "table" => Ok(OutputFormat::Table),
            "plain" => Ok(OutputFormat::Plain),
            _ => Err(format!("Invalid output format: {}", s)),
        }
    }
}

/// CLI commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Task management commands
    Task {
        #[command(subcommand)]
        subcommand: TaskCommands,
    },

    /// Workflow management commands
    Workflow {
        #[command(subcommand)]
        subcommand: WorkflowCommands,
    },

    /// Authentication commands
    Auth {
        #[command(subcommand)]
        subcommand: AuthCommands,
    },

    /// Server and connection management
    Server {
        #[command(subcommand)]
        subcommand: ServerCommands,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        subcommand: ConfigCommands,
    },
}

/// Task subcommands
#[derive(Subcommand, Debug)]
pub enum TaskCommands {
    /// Create a new task
    Create {
        /// Task title
        #[arg(value_name = "TITLE")]
        title: String,

        /// Task description
        #[arg(short, long)]
        description: Option<String>,

        /// Task type
        #[arg(short, long)]
        r#type: Option<String>,

        /// Configuration JSON
        #[arg(short, long)]
        config: Option<String>,

        /// Metadata JSON
        #[arg(short, long)]
        metadata: Option<String>,
    },

    /// List all tasks
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,

        /// Limit results
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: u32,
    },

    /// Get task details
    Get {
        /// Task ID
        #[arg(value_name = "ID")]
        id: String,
    },

    /// Update a task
    Update {
        /// Task ID
        #[arg(value_name = "ID")]
        id: String,

        /// New task title
        #[arg(short, long)]
        title: Option<String>,

        /// New task description
        #[arg(short, long)]
        description: Option<String>,

        /// New task status
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Delete a task
    Delete {
        /// Task ID
        #[arg(value_name = "ID")]
        id: String,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Execute a task
    Execute {
        /// Task ID
        #[arg(value_name = "ID")]
        id: String,

        /// Parameters as JSON
        #[arg(short, long)]
        parameters: Option<String>,

        /// Stream execution output
        #[arg(short, long)]
        stream: bool,
    },
}

/// Workflow subcommands
#[derive(Subcommand, Debug)]
pub enum WorkflowCommands {
    /// Create a new workflow
    Create {
        /// Workflow name
        #[arg(value_name = "NAME")]
        name: String,

        /// Workflow description
        #[arg(short, long)]
        description: Option<String>,

        /// Workflow definition JSON or file
        #[arg(short, long)]
        definition: Option<String>,
    },

    /// List all workflows
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,

        /// Limit results
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: u32,
    },

    /// Get workflow details
    Get {
        /// Workflow ID
        #[arg(value_name = "ID")]
        id: String,
    },

    /// Update a workflow
    Update {
        /// Workflow ID
        #[arg(value_name = "ID")]
        id: String,

        /// New workflow name
        #[arg(short, long)]
        name: Option<String>,

        /// New workflow description
        #[arg(short, long)]
        description: Option<String>,

        /// New workflow definition
        #[arg(short, long)]
        definition: Option<String>,
    },

    /// Delete a workflow
    Delete {
        /// Workflow ID
        #[arg(value_name = "ID")]
        id: String,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Execute a workflow
    Execute {
        /// Workflow ID
        #[arg(value_name = "ID")]
        id: String,

        /// Parameters as JSON
        #[arg(short, long)]
        parameters: Option<String>,

        /// Stream execution output
        #[arg(short, long)]
        stream: bool,
    },
}

/// Authentication subcommands
#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Authenticate with username and password
    Login {
        /// Username
        #[arg(value_name = "USERNAME")]
        username: String,

        /// Password (interactive if not provided)
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Logout and clear authentication token
    Logout,

    /// Show current authentication status
    Status,

    /// Connect with authentication credentials
    Connect {
        /// Connection string (none, secret:key, user:pass, token:jwt)
        #[arg(value_name = "AUTH")]
        auth: String,
    },
}

/// Server and connection commands
#[derive(Subcommand, Debug)]
pub enum ServerCommands {
    /// Show server information
    Info,

    /// Check server health
    Health,

    /// Show server and client version
    Version,

    /// Connect to a server
    Connect {
        /// Server URL
        #[arg(value_name = "URL")]
        url: String,
    },

    /// Disconnect from server
    Disconnect,

    /// Show connection status
    Status,
}

/// Configuration commands
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Set configuration value
    Set {
        /// Configuration key
        #[arg(value_name = "KEY")]
        key: String,

        /// Configuration value
        #[arg(value_name = "VALUE")]
        value: String,
    },

    /// Initialize default configuration
    Init {
        /// Configuration file path
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_json() {
        let format: OutputFormat = "json".parse().unwrap();
        assert!(matches!(format, OutputFormat::Json));
    }

    #[test]
    fn test_output_format_table() {
        let format: OutputFormat = "table".parse().unwrap();
        assert!(matches!(format, OutputFormat::Table));
    }

    #[test]
    fn test_output_format_plain() {
        let format: OutputFormat = "plain".parse().unwrap();
        assert!(matches!(format, OutputFormat::Plain));
    }

    #[test]
    fn test_output_format_case_insensitive() {
        let format: OutputFormat = "JSON".parse().unwrap();
        assert!(matches!(format, OutputFormat::Json));

        let format: OutputFormat = "TaBlE".parse().unwrap();
        assert!(matches!(format, OutputFormat::Table));
    }

    #[test]
    fn test_output_format_invalid() {
        let result: Result<OutputFormat, _> = "invalid".parse();
        assert!(result.is_err());
    }
}
