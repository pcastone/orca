//! aco client application - CLI entry point

use aco::{AcoConfig, AcoServer, AcoWorker, ConfigLoader, Result, TuiConfig};
use aco::tools::{
    FileReadTool, FileWriteTool, FsListTool, ShellExecTool,
    GitStatusTool, GitDiffTool, GrepTool,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber;

/// aco client application
#[derive(Parser, Debug)]
#[command(name = "aco")]
#[command(version = aco::version::VERSION)]
#[command(long_version = aco::version::VERSION_INFO)]
#[command(about = "aco client application for tool execution", long_about = None)]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Connect to orchestrator and save for auto-connect
    #[arg(long, value_name = "URL")]
    connect: Option<String>,

    /// Disconnect and disable auto-connect
    #[arg(long)]
    disconnect: bool,

    /// Show connection status
    #[arg(long)]
    status: bool,

    /// Workspace directory (for worker mode)
    #[arg(short, long, default_value = ".")]
    workspace: PathBuf,

    /// Worker name (auto-generated if not provided)
    #[arg(short, long)]
    name: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Initialize aco configuration for a project
    Init,
    /// Show current configuration
    Config,
    /// Show connection status
    Status,
    /// Send a prompt to the LLM via orchestrator-server
    Prompt {
        /// The prompt to send
        #[arg(value_name = "PROMPT")]
        prompt: String,
        /// Orchestrator server URL (default from config or ORCHESTRATOR_URL env)
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Run as server
    Server {
        /// Workspace root directory
        #[arg(short, long, default_value = ".")]
        workspace: PathBuf,

        /// Server address (overrides config)
        #[arg(short, long)]
        address: Option<String>,

        /// Enable TUI mode (overrides config)
        #[arg(long)]
        tui: Option<bool>,
    },
    /// Data management commands (backup, restore, export, import via orchestrator)
    #[command(subcommand)]
    Data(DataCommand),
    /// Bug tracking commands
    #[command(subcommand)]
    Bug(BugCommand),
}

#[derive(Subcommand, Debug)]
enum DataCommand {
    /// Create a backup of databases
    Backup {
        /// Override backup directory
        #[arg(short, long)]
        dir: Option<PathBuf>,
        /// Include project database (default: true)
        #[arg(long, default_value = "true")]
        include_project: bool,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Restore from a backup
    Restore {
        /// Backup file to restore from
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// List available backups instead of restoring
        #[arg(long)]
        list: bool,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Export tables to SQL dump
    Export {
        /// Tables to export (comma-separated or "all")
        #[arg(short, long, default_value = "all")]
        tables: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Import from SQL dump
    Import {
        /// File to import
        file: PathBuf,
        /// Tables to import (comma-separated or "all")
        #[arg(short, long, default_value = "all")]
        tables: String,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum BugCommand {
    /// Create a new bug
    Create {
        /// Bug title
        title: String,
        /// Bug description
        #[arg(short, long)]
        description: Option<String>,
        /// Priority: 1=Critical, 2=High, 3=Medium, 4=Low, 5=Trivial
        #[arg(short, long)]
        priority: Option<i64>,
        /// Assignee name
        #[arg(short, long)]
        assignee: Option<String>,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// List all bugs
    List {
        /// Filter by status: open, in_progress, fixed, wontfix, duplicate
        #[arg(long)]
        status: Option<String>,
        /// Filter by assignee
        #[arg(long)]
        assignee: Option<String>,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Show bug details
    Show {
        /// Bug ID
        id: String,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Update bug status
    UpdateStatus {
        /// Bug ID
        id: String,
        /// New status: open, in_progress, fixed, wontfix, duplicate
        status: String,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Assign bug to someone
    Assign {
        /// Bug ID
        id: String,
        /// Assignee name
        assignee: String,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Close/fix a bug
    Close {
        /// Bug ID
        id: String,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Delete a bug
    Delete {
        /// Bug ID
        id: String,
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
    /// Show bug statistics
    Stats {
        /// Orchestrator server URL
        #[arg(short, long)]
        server: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize config directories (creates .aco dirs if they don't exist)
    if let Err(e) = aco::config::init_config_directories().await {
        warn!("Failed to initialize config directories: {}", e);
    }

    // Load configuration
    let mut config = load_config(&args).await;
    let loader = ConfigLoader::new();

    // Initialize logging based on config
    let log_level = if args.verbose {
        Level::DEBUG
    } else {
        match config.ui.log_level.as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        }
    };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    // Handle --disconnect first
    if args.disconnect {
        config.client.auto_connect = false;
        config.client.orchestrator_url = aco::config::ClientConfig::default().orchestrator_url;
        loader.save_user_config(&config).await?;
        println!("Disconnected. Auto-connect disabled.");
        return Ok(());
    }

    // Handle --status
    if args.status {
        show_status(&config, &args).await;
        return Ok(());
    }

    // Handle --connect
    if let Some(ref url) = args.connect {
        config.client.orchestrator_url = url.clone();
        config.client.auto_connect = true;
        loader.save_user_config(&config).await?;
        println!("Saved connection to {}. Starting worker...", url);
        return run_worker(&config, &args).await;
    }

    // Handle subcommands if present
    if let Some(ref cmd) = args.command {
        info!("Starting aco version {}", aco::version::VERSION);
        info!("Build {} at {} ({})",
              aco::version::BUILD_NUMBER,
              aco::version::BUILD_TIMESTAMP,
              aco::version::GIT_COMMIT_SHORT);

        return handle_command(cmd, &config, &args).await;
    }

    // Default behavior: auto-connect if enabled, otherwise show usage
    if config.client.auto_connect {
        println!("Auto-connecting to {}...", config.client.orchestrator_url);
        return run_worker(&config, &args).await;
    }

    // No auto-connect, no command - show usage
    println!("Usage: aco --connect <URL> to connect to orchestrator");
    println!("       aco --disconnect to disable auto-connect");
    println!("       aco server       to run in server mode");
    println!("       aco --help       for more options");
    Ok(())
}

/// Load configuration with fallback to defaults
async fn load_config(args: &Args) -> AcoConfig {
    match ConfigLoader::new().load().await {
        Ok(config) => {
            info!("Loaded configuration successfully");
            config
        }
        Err(e) => {
            if args.verbose {
                warn!("Failed to load config: {}, using defaults", e);
            }
            AcoConfig::default()
        }
    }
}

/// Show status information
async fn show_status(config: &AcoConfig, args: &Args) {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let workspace = args.workspace.canonicalize()
        .unwrap_or(args.workspace.clone())
        .display()
        .to_string();

    println!("ACO Status");
    println!("==========");
    println!();
    println!("Working Directory: {}", cwd);
    println!("Workspace:         {}", workspace);
    println!();
    println!("Connection:");
    if config.client.auto_connect {
        println!("  Auto-connect:    enabled");
        println!("  Orchestrator:    {}", config.client.orchestrator_url);
    } else {
        println!("  Auto-connect:    disabled");
    }
    println!();
    println!("Config Locations:");
    let loader = ConfigLoader::new();
    println!("  User:    {}", loader.get_user_config_path().display());
    println!("  Project: {}", loader.get_project_config_path().display());
}

/// Show current configuration
async fn show_config(config: &AcoConfig) -> Result<()> {
    let config_toml = toml::to_string_pretty(&config)
        .unwrap_or_else(|_| "Failed to serialize config".to_string());

    println!("Current Configuration:");
    println!("=====================");
    println!("{}", config_toml);
    println!();

    let loader = ConfigLoader::new();
    println!("Config Locations:");
    println!("  User:    {}", loader.get_user_config_path().display());
    println!("  Project: {}", loader.get_project_config_path().display());

    Ok(())
}

/// Run in worker mode
async fn run_worker(config: &AcoConfig, args: &Args) -> Result<()> {
    info!("Starting worker mode");
    info!("Orchestrator: {}", config.client.orchestrator_url);
    info!("Workspace: {}", args.workspace.display());

    let workspace_path = args.workspace.canonicalize()
        .unwrap_or(args.workspace.clone())
        .to_string_lossy()
        .to_string();

    let mut worker = AcoWorker::new(
        config.client.orchestrator_url.clone(),
        args.name.clone(),
        workspace_path.clone(),
    );

    register_worker_tools(&mut worker, &workspace_path);
    worker.start().await
}

/// Handle subcommands
async fn handle_command(cmd: &Command, config: &AcoConfig, args: &Args) -> Result<()> {
    match cmd {
        Command::Init => {
            info!("Initializing aco configuration...");
            aco::config::init_config_directories().await?;
            let config_path = aco::config::init_project_config().await?;
            info!("Created config file: {}", config_path.display());
            info!("Configuration initialized successfully!");
        }
        Command::Config => {
            show_config(config).await?;
        }
        Command::Status => {
            aco::client::show_status().await?;
        }
        Command::Prompt { prompt, server } => {
            let server_url = server.clone().unwrap_or_else(|| {
                std::env::var("ORCHESTRATOR_URL")
                    .unwrap_or_else(|_| config.client.orchestrator_url.clone())
            });
            send_prompt(&server_url, prompt).await?;
        }
        Command::Server { workspace, address, tui } => {
            info!("Workspace: {}", workspace.display());

            let server_address = address.clone().unwrap_or_else(|| {
                format!("{}:{}", config.server.host, config.server.port)
            });
            let enable_tui = tui.unwrap_or(config.ui.enable_tui);

            info!("Address: {}", server_address);

            if enable_tui {
                info!("Starting TUI mode");
                let tui_config = TuiConfig::from_env(server_address, workspace.clone(), args.verbose);
                aco::tui::run(tui_config).await?;
            } else {
                let workspace_path = workspace.canonicalize()
                    .unwrap_or(workspace.clone())
                    .to_string_lossy()
                    .to_string();
                let server = AcoServer::new().with_address(&server_address);

                register_tools(&server, &workspace_path).await?;
                server.start().await?;
            }
        }
        Command::Data(data_cmd) => {
            handle_data_command(data_cmd, config).await?;
        }
        Command::Bug(bug_cmd) => {
            handle_bug_command(bug_cmd, config).await?;
        }
    }
    Ok(())
}

/// Handle bug tracking subcommands
async fn handle_bug_command(cmd: &BugCommand, config: &AcoConfig) -> Result<()> {
    use aco::tui::grpc_client::{BugInfo, BugStats, CreateBugRequest, UpdateBugRequest, TuiGrpcClient};

    let get_server_url = |server: &Option<String>| {
        server.clone().unwrap_or_else(|| {
            std::env::var("ORCHESTRATOR_URL")
                .unwrap_or_else(|_| config.client.orchestrator_url.clone())
        })
    };

    let priority_name = |p: i64| -> &'static str {
        match p {
            1 => "Critical",
            2 => "High",
            3 => "Medium",
            4 => "Low",
            5 => "Trivial",
            _ => "Unknown",
        }
    };

    match cmd {
        BugCommand::Create { title, description, priority, assignee, server } => {
            let server_url = get_server_url(server);
            let client = TuiGrpcClient::new(server_url);

            let request = CreateBugRequest {
                title: title.clone(),
                description: description.clone(),
                priority: *priority,
                assignee: assignee.clone(),
            };

            let bug = client.create_bug(request).await?;
            println!("Bug created successfully!");
            println!("  ID: {}", bug.id);
            println!("  Title: {}", bug.title);
            println!("  Priority: {}", priority_name(bug.priority));
        }
        BugCommand::List { status, assignee, server } => {
            let server_url = get_server_url(server);
            let client = TuiGrpcClient::new(server_url);

            let bugs = client.fetch_bugs().await?;

            // Filter bugs
            let filtered: Vec<_> = bugs.into_iter()
                .filter(|b| status.as_ref().map(|s| &b.status == s).unwrap_or(true))
                .filter(|b| assignee.as_ref().map(|a| b.assignee.as_ref() == Some(a)).unwrap_or(true))
                .collect();

            if filtered.is_empty() {
                println!("No bugs found.");
                return Ok(());
            }

            println!("{:<10} {:<40} {:<12} {:<10} {:<15}", "ID", "Title", "Status", "Priority", "Assignee");
            println!("{}", "-".repeat(90));

            for bug in &filtered {
                let id_short = if bug.id.len() > 8 { &bug.id[..8] } else { &bug.id };
                let title = if bug.title.len() > 38 { format!("{}...", &bug.title[..35]) } else { bug.title.clone() };
                let assignee = bug.assignee.as_deref().unwrap_or("-");

                println!("{:<10} {:<40} {:<12} {:<10} {:<15}",
                    id_short, title, bug.status, priority_name(bug.priority), assignee);
            }

            println!("\nTotal: {} bugs", filtered.len());
        }
        BugCommand::Show { id, server } => {
            let server_url = get_server_url(server);
            let client = TuiGrpcClient::new(server_url);

            let bug = client.get_bug(id).await?;

            println!("\nBug Details");
            println!("===========");
            println!("ID:          {}", bug.id);
            println!("Title:       {}", bug.title);
            println!("Status:      {}", bug.status);
            println!("Priority:    {}", priority_name(bug.priority));
            if let Some(ref severity) = bug.severity {
                println!("Severity:    {}", severity);
            }
            if let Some(ref assignee) = bug.assignee {
                println!("Assignee:    {}", assignee);
            }
            if let Some(ref reporter) = bug.reporter {
                println!("Reporter:    {}", reporter);
            }
            println!("Created:     {}", bug.created_at);
            println!("Updated:     {}", bug.updated_at);
            if let Some(ref desc) = bug.description {
                println!("\nDescription:");
                println!("{}", desc);
            }
        }
        BugCommand::UpdateStatus { id, status, server } => {
            let server_url = get_server_url(server);
            let client = TuiGrpcClient::new(server_url);

            let request = UpdateBugRequest {
                title: None,
                description: None,
                status: Some(status.clone()),
                priority: None,
                assignee: None,
            };

            let bug = client.update_bug(id, request).await?;
            println!("Bug status updated!");
            println!("  ID: {}", bug.id);
            println!("  Status: {}", bug.status);
        }
        BugCommand::Assign { id, assignee, server } => {
            let server_url = get_server_url(server);
            let client = TuiGrpcClient::new(server_url);

            let request = UpdateBugRequest {
                title: None,
                description: None,
                status: None,
                priority: None,
                assignee: Some(assignee.clone()),
            };

            let bug = client.update_bug(id, request).await?;
            println!("Bug assigned!");
            println!("  ID: {}", bug.id);
            println!("  Assignee: {}", bug.assignee.unwrap_or_default());
        }
        BugCommand::Close { id, server } => {
            let server_url = get_server_url(server);
            let client = TuiGrpcClient::new(server_url);

            let request = UpdateBugRequest {
                title: None,
                description: None,
                status: Some("fixed".to_string()),
                priority: None,
                assignee: None,
            };

            let bug = client.update_bug(id, request).await?;
            println!("Bug closed!");
            println!("  ID: {}", bug.id);
            println!("  Status: {}", bug.status);
        }
        BugCommand::Delete { id, server } => {
            let server_url = get_server_url(server);
            let client = TuiGrpcClient::new(server_url);

            client.delete_bug(id).await?;
            println!("Bug deleted: {}", id);
        }
        BugCommand::Stats { server } => {
            let server_url = get_server_url(server);
            let client = TuiGrpcClient::new(server_url);

            let stats = client.get_bug_stats().await?;

            println!("\nBug Statistics");
            println!("==============");

            println!("\nBy Status:");
            println!("  Open:        {}", stats.open);
            println!("  In Progress: {}", stats.in_progress);
            println!("  Fixed:       {}", stats.fixed);
            println!("  Won't Fix:   {}", stats.wontfix);
            println!("  Duplicate:   {}", stats.duplicate);

            println!("\nBy Priority:");
            println!("  Critical:    {}", stats.critical);
            println!("  High:        {}", stats.high);
            println!("  Medium:      {}", stats.medium);
            println!("  Low:         {}", stats.low);
            println!("  Trivial:     {}", stats.trivial);

            println!("\nTotal: {}", stats.total);
        }
    }

    Ok(())
}

/// Handle data management subcommands
async fn handle_data_command(cmd: &DataCommand, config: &AcoConfig) -> Result<()> {
    let get_server_url = |server: &Option<String>| {
        server.clone().unwrap_or_else(|| {
            std::env::var("ORCHESTRATOR_URL")
                .unwrap_or_else(|_| config.client.orchestrator_url.clone())
        })
    };

    match cmd {
        DataCommand::Backup { dir, include_project, server } => {
            let server_url = get_server_url(server);
            handle_backup(&server_url, dir.clone(), *include_project).await?;
        }
        DataCommand::Restore { file, list, server } => {
            let server_url = get_server_url(server);
            handle_restore(&server_url, file.clone(), *list).await?;
        }
        DataCommand::Export { tables, output, server } => {
            let server_url = get_server_url(server);
            handle_export(&server_url, tables, output.clone()).await?;
        }
        DataCommand::Import { file, tables, server } => {
            let server_url = get_server_url(server);
            handle_import(&server_url, file, tables).await?;
        }
    }
    Ok(())
}

/// Register all available tools for AcoServer
async fn register_tools(server: &AcoServer, workspace: &str) -> Result<()> {
    // Filesystem tools
    server.register_tool(Arc::new(FileReadTool::new(workspace))).await;
    server.register_tool(Arc::new(FileWriteTool::new(workspace))).await;
    server.register_tool(Arc::new(FsListTool::new(workspace))).await;
    server.register_tool(Arc::new(GrepTool::new(workspace))).await;

    // Git tools
    server.register_tool(Arc::new(GitStatusTool::new(workspace))).await;
    server.register_tool(Arc::new(GitDiffTool::new(workspace))).await;

    // Shell tools
    server.register_tool(Arc::new(ShellExecTool::new(workspace))).await;

    info!("Registered all tools");

    Ok(())
}

/// Register all available tools for AcoWorker
fn register_worker_tools(worker: &mut AcoWorker, workspace: &str) {
    // Filesystem tools
    worker.register_tool(Box::new(FileReadTool::new(workspace)));
    worker.register_tool(Box::new(FileWriteTool::new(workspace)));
    worker.register_tool(Box::new(FsListTool::new(workspace)));
    worker.register_tool(Box::new(GrepTool::new(workspace)));

    // Git tools
    worker.register_tool(Box::new(GitStatusTool::new(workspace)));
    worker.register_tool(Box::new(GitDiffTool::new(workspace)));

    // Shell tools
    worker.register_tool(Box::new(ShellExecTool::new(workspace)));

    info!("Registered all tools for worker");
}

/// Send a prompt to the orchestrator-server's LLM endpoint
async fn send_prompt(server_url: &str, prompt: &str) -> Result<()> {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize)]
    struct PromptRequest {
        prompt: String,
    }

    #[derive(Deserialize)]
    struct PromptResponseWrapper {
        success: bool,
        data: PromptResponseData,
    }

    #[derive(Deserialize)]
    struct PromptResponseData {
        response: String,
    }

    #[derive(Deserialize)]
    struct ErrorResponse {
        success: bool,
        message: String,
    }

    // Build the URL
    let url = format!("{}/api/v1/prompt", server_url.trim_end_matches('/'));

    // Send the request
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&PromptRequest {
            prompt: prompt.to_string(),
        })
        .send()
        .await
        .map_err(|e| {
            aco::error::AcoError::Connection(format!("Failed to connect to orchestrator: {}", e))
        })?;

    // Check response status
    if response.status().is_success() {
        let result: PromptResponseWrapper = response.json().await.map_err(|e| {
            aco::error::AcoError::General(format!("Failed to parse response: {}", e))
        })?;
        println!("{}", result.data.response);
    } else {
        let error: ErrorResponse = response.json().await.map_err(|e| {
            aco::error::AcoError::General(format!("Failed to parse error response: {}", e))
        })?;
        eprintln!("Error: {}", error.message);
        return Err(aco::error::AcoError::General(error.message).into());
    }

    Ok(())
}

/// Handle backup via orchestrator API
async fn handle_backup(server_url: &str, dir: Option<PathBuf>, include_project: bool) -> Result<()> {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize)]
    struct BackupRequest {
        include_project: bool,
        backup_dir: Option<String>,
    }

    #[derive(Deserialize)]
    struct BackupResponse {
        path: String,
        timestamp: String,
        size_bytes: u64,
        includes_user_db: bool,
        includes_project_db: bool,
    }

    let url = format!("{}/api/v1/data/backup", server_url.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&BackupRequest {
            include_project,
            backup_dir: dir.map(|p| p.display().to_string()),
        })
        .send()
        .await
        .map_err(|e| aco::error::AcoError::Connection(format!("Failed to connect: {}", e)))?;

    if response.status().is_success() {
        let result: BackupResponse = response.json().await.map_err(|e| {
            aco::error::AcoError::General(format!("Failed to parse response: {}", e))
        })?;

        println!("Backup created successfully!");
        println!("  Path: {}", result.path);
        println!("  Size: {:.2} KB", result.size_bytes as f64 / 1024.0);
        println!("  Timestamp: {}", result.timestamp);
        println!("  User DB: {}", if result.includes_user_db { "Yes" } else { "No" });
        println!("  Project DB: {}", if result.includes_project_db { "Yes" } else { "No" });
    } else {
        let text = response.text().await.unwrap_or_default();
        eprintln!("Backup failed: {}", text);
        return Err(aco::error::AcoError::General(text).into());
    }

    Ok(())
}

/// Handle restore via orchestrator API
async fn handle_restore(server_url: &str, file: Option<PathBuf>, list: bool) -> Result<()> {
    use serde::Deserialize;

    if list {
        // List backups
        let url = format!("{}/api/v1/data/backups", server_url.trim_end_matches('/'));

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| aco::error::AcoError::Connection(format!("Failed to connect: {}", e)))?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct BackupInfo {
                filename: String,
                timestamp: String,
                size_bytes: u64,
                backup_type: String,
            }

            let backups: Vec<BackupInfo> = response.json().await.map_err(|e| {
                aco::error::AcoError::General(format!("Failed to parse response: {}", e))
            })?;

            if backups.is_empty() {
                println!("No backups found.");
            } else {
                println!("Available backups:");
                println!();
                for backup in backups {
                    println!("  {} - {} ({:.1} KB, {})",
                        backup.timestamp,
                        backup.filename,
                        backup.size_bytes as f64 / 1024.0,
                        backup.backup_type
                    );
                }
            }
        } else {
            let text = response.text().await.unwrap_or_default();
            eprintln!("Failed to list backups: {}", text);
        }
        return Ok(());
    }

    // Restore from file
    if let Some(backup_file) = file {
        use serde::Serialize;

        #[derive(Serialize)]
        struct RestoreRequest {
            backup_file: String,
        }

        let url = format!("{}/api/v1/data/restore", server_url.trim_end_matches('/'));

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&RestoreRequest {
                backup_file: backup_file.display().to_string(),
            })
            .send()
            .await
            .map_err(|e| aco::error::AcoError::Connection(format!("Failed to connect: {}", e)))?;

        if response.status().is_success() {
            println!("Restore completed successfully!");
            println!("Note: You may need to restart the orchestrator for changes to take effect.");
        } else {
            let text = response.text().await.unwrap_or_default();
            eprintln!("Restore failed: {}", text);
            return Err(aco::error::AcoError::General(text).into());
        }
    } else {
        println!("No backup file specified.");
        println!("Use: aco data restore --file <backup_file>");
        println!("Or:  aco data restore --list");
    }

    Ok(())
}

/// Handle export via orchestrator API
async fn handle_export(server_url: &str, tables: &str, output: Option<PathBuf>) -> Result<()> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct ExportRequest {
        tables: Vec<String>,
    }

    let url = format!("{}/api/v1/data/export", server_url.trim_end_matches('/'));

    let table_list: Vec<String> = tables
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&ExportRequest { tables: table_list })
        .send()
        .await
        .map_err(|e| aco::error::AcoError::Connection(format!("Failed to connect: {}", e)))?;

    if response.status().is_success() {
        let content = response.text().await.map_err(|e| {
            aco::error::AcoError::General(format!("Failed to read response: {}", e))
        })?;

        // Save to file or print to stdout
        if let Some(output_path) = output {
            std::fs::write(&output_path, &content).map_err(|e| {
                aco::error::AcoError::General(format!("Failed to write file: {}", e))
            })?;
            println!("Export saved to: {}", output_path.display());
        } else {
            // Generate default filename
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let output_path = format!("export_{}.sql", timestamp);
            std::fs::write(&output_path, &content).map_err(|e| {
                aco::error::AcoError::General(format!("Failed to write file: {}", e))
            })?;
            println!("Export saved to: {}", output_path);
        }
    } else {
        let text = response.text().await.unwrap_or_default();
        eprintln!("Export failed: {}", text);
        return Err(aco::error::AcoError::General(text).into());
    }

    Ok(())
}

/// Handle import via orchestrator API
async fn handle_import(server_url: &str, file: &PathBuf, _tables: &str) -> Result<()> {
    let url = format!("{}/api/v1/data/import", server_url.trim_end_matches('/'));

    // Read the file content
    let content = std::fs::read_to_string(file).map_err(|e| {
        aco::error::AcoError::General(format!("Failed to read file: {}", e))
    })?;

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "text/plain")
        .body(content)
        .send()
        .await
        .map_err(|e| aco::error::AcoError::Connection(format!("Failed to connect: {}", e)))?;

    if response.status().is_success() {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct ImportResponse {
            records_inserted: usize,
            records_updated: usize,
            records_skipped: usize,
            tables_imported: Vec<String>,
        }

        let result: ImportResponse = response.json().await.map_err(|e| {
            aco::error::AcoError::General(format!("Failed to parse response: {}", e))
        })?;

        println!("Import completed successfully!");
        println!("  Records inserted: {}", result.records_inserted);
        println!("  Records updated: {}", result.records_updated);
        println!("  Records skipped: {}", result.records_skipped);
        if !result.tables_imported.is_empty() {
            println!("  Tables: {}", result.tables_imported.join(", "));
        }
    } else {
        let text = response.text().await.unwrap_or_default();
        eprintln!("Import failed: {}", text);
        return Err(aco::error::AcoError::General(text).into());
    }

    Ok(())
}

