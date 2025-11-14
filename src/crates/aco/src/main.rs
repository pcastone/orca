//! aco client application - CLI entry point

use aco::{AcoConfig, AcoServer, ConfigLoader, Result, TuiConfig};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tooling::tools::{
    filesystem::{
        FileReadTool, FileWriteTool, FsCopyTool, FsDeleteTool, FsListTool, FsMoveTool,
        FilePatchTool, GrepTool,
    },
    git::{GitStatusTool, GitDiffTool, GitAddTool, GitCommitTool},
    shell::ShellExecTool,
};
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

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Initialize aco configuration for a project
    Init,
    /// Show current configuration
    Config,
    /// Connect to an orchestrator server
    Connect {
        /// Orchestrator server URL (e.g., http://localhost:8080)
        #[arg(value_name = "URL")]
        url: Option<String>,
    },
    /// Show connection status
    Status,
    /// Run as server (default behavior)
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize config directories (creates .aco dirs if they don't exist)
    if let Err(e) = aco::config::init_config_directories().await {
        warn!("Failed to initialize config directories: {}", e);
    }

    // Load configuration
    let config = load_config(&args).await;

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

    info!("Starting aco version {}", aco::version::VERSION);
    info!("Build {} at {} ({})",
          aco::version::BUILD_NUMBER,
          aco::version::BUILD_TIMESTAMP,
          aco::version::GIT_COMMIT_SHORT);

    match args.command {
        Some(Command::Init) => {
            info!("Initializing aco configuration...");
            aco::config::init_config_directories().await?;
            let config_path = aco::config::init_project_config().await?;
            info!("Created config file: {}", config_path.display());
            info!("Configuration initialized successfully!");
        }
        Some(Command::Config) => {
            show_config(&config).await?;
        }
        Some(Command::Connect { url }) => {
            let server_url = url.unwrap_or_else(|| {
                std::env::var("ORCHESTRATOR_URL")
                    .unwrap_or_else(|_| config.client.orchestrator_url.clone())
            });
            aco::client::connect_to_orchestrator(&server_url).await?;
        }
        Some(Command::Status) => {
            aco::client::show_status().await?;
        }
        Some(Command::Server { workspace, address, tui }) => {
            info!("Workspace: {}", workspace.display());

            // Use CLI args > config
            let server_address = address.unwrap_or_else(|| {
                format!("{}:{}", config.server.host, config.server.port)
            });
            let enable_tui = tui.unwrap_or(config.ui.enable_tui);

            info!("Address: {}", server_address);

            if enable_tui {
                // Run TUI mode
                info!("Starting TUI mode");
                let tui_config = TuiConfig::from_env(server_address, workspace, args.verbose);
                aco::tui::run(tui_config).await?;
            } else {
                // Run server mode
                let server = AcoServer::new().with_address(&server_address);

                // Register all tools
                register_tools(&server).await?;

                // Start server (this blocks)
                server.start().await?;
            }
        }
        None => {
            // Default to server mode with config values
            let workspace = PathBuf::from(".");
            let address = format!("{}:{}", config.server.host, config.server.port);
            info!("Workspace: {}", workspace.display());
            info!("Address: {}", address);

            let server = AcoServer::new().with_address(&address);
            register_tools(&server).await?;
            server.start().await?;
        }
    }

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

/// Register all available tools
async fn register_tools(server: &AcoServer) -> Result<()> {
    // Filesystem tools
    server.register_tool(Arc::new(FileReadTool)).await;
    server.register_tool(Arc::new(FileWriteTool)).await;
    server.register_tool(Arc::new(FsListTool)).await;
    server.register_tool(Arc::new(FsCopyTool)).await;
    server.register_tool(Arc::new(FsMoveTool)).await;
    server.register_tool(Arc::new(FsDeleteTool)).await;
    server.register_tool(Arc::new(FilePatchTool)).await;
    server.register_tool(Arc::new(GrepTool)).await;

    // Git tools
    server.register_tool(Arc::new(GitStatusTool)).await;
    server.register_tool(Arc::new(GitDiffTool)).await;
    server.register_tool(Arc::new(GitAddTool)).await;
    server.register_tool(Arc::new(GitCommitTool)).await;

    // Shell tools
    server.register_tool(Arc::new(ShellExecTool)).await;

    info!("Registered all tools");

    Ok(())
}

