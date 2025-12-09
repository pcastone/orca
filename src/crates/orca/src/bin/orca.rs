//! Orca CLI - Standalone orchestrator for AI agent workflows
//!
//! Main entry point for the orca command-line tool.

use clap::{Parser, Subcommand};
use orca::version_info;
use serde::Deserialize;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Minimal logging config for sync loading (without full config/database)
#[derive(Debug, Deserialize)]
struct LoggingConfigSync {
    #[serde(default = "default_level")]
    level: String,
    #[serde(default)]
    log_directory: Option<String>,
    #[serde(default = "default_prefix")]
    log_prefix: String,
}

fn default_level() -> String { "info".to_string() }
fn default_prefix() -> String { "orca".to_string() }

#[derive(Debug, Deserialize)]
struct PartialConfig {
    #[serde(default)]
    logging: Option<LoggingConfigSync>,
}

/// Load logging config synchronously from TOML files (no async, no database)
///
/// Checks project-level (./.orca/orca.toml) first, then user-level (~/.orca/orca.toml)
fn load_logging_config_sync() -> Option<LoggingConfigSync> {
    // Try project-level config first
    let project_config = std::path::PathBuf::from(".orca/orca.toml");
    if let Some(config) = try_load_toml_logging(&project_config) {
        return Some(config);
    }

    // Fall back to user-level config
    if let Some(home) = dirs::home_dir() {
        let user_config = home.join(".orca/orca.toml");
        if let Some(config) = try_load_toml_logging(&user_config) {
            return Some(config);
        }
    }

    None
}

fn try_load_toml_logging(path: &std::path::Path) -> Option<LoggingConfigSync> {
    let content = std::fs::read_to_string(path).ok()?;
    let partial: PartialConfig = toml::from_str(&content).ok()?;
    partial.logging
}

/// Initialize logging with file support
///
/// Reads logging configuration from:
/// 1. Environment variables (highest priority):
///    - RUST_LOG: Log level filter
///    - ORCA_LOG_DIR: Directory for log files
///    - ORCA_LOG_PREFIX: Log file prefix
/// 2. Config files (if env vars not set):
///    - ./.orca/orca.toml (project-level)
///    - ~/.orca/orca.toml (user-level)
///
/// If log_directory is set, logs are written to rolling daily files.
/// Otherwise, logs go to stderr only.
fn init_logging() {
    // Try to load logging config from TOML files (without async/database)
    let config = load_logging_config_sync();

    // Determine log level: env var > config > default
    let log_level = std::env::var("RUST_LOG").ok()
        .or_else(|| config.as_ref().map(|c| c.level.clone()))
        .unwrap_or_else(|| "info".to_string());

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&log_level));

    // Check if we should log to files: env var > config
    let log_directory = std::env::var("ORCA_LOG_DIR").ok()
        .or_else(|| config.as_ref().and_then(|c| c.log_directory.clone()));
    let log_prefix = std::env::var("ORCA_LOG_PREFIX").ok()
        .or_else(|| config.as_ref().map(|c| c.log_prefix.clone()))
        .unwrap_or_else(|| "orca".to_string());

    if let Some(log_dir) = log_directory {
        // Expand ~ to home directory
        let expanded_dir = if log_dir.starts_with("~/") {
            dirs::home_dir()
                .map(|h| h.join(&log_dir[2..]))
                .unwrap_or_else(|| std::path::PathBuf::from(&log_dir))
        } else {
            std::path::PathBuf::from(&log_dir)
        };

        // Create log directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&expanded_dir) {
            eprintln!("Warning: Failed to create log directory {:?}: {}", expanded_dir, e);
            // Fall back to stderr logging
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().with_writer(std::io::stderr))
                .init();
            return;
        }

        // Create file appender with rolling daily logs
        let file_appender = tracing_appender::rolling::daily(&expanded_dir, &log_prefix);
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        // Store the guard in a static to keep it alive
        // (otherwise the non-blocking writer will be dropped)
        static GUARD: std::sync::OnceLock<tracing_appender::non_blocking::WorkerGuard> = std::sync::OnceLock::new();
        let _ = GUARD.set(_guard);

        // Set up subscriber with file output
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)  // No ANSI codes in file
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true))
            .init();

        eprintln!("Logging to: {}/{}.YYYY-MM-DD", expanded_dir.display(), log_prefix);
    } else {
        // Default: log to stderr only
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_writer(std::io::stderr))
            .init();
    }
}

#[derive(Parser)]
#[command(name = "orca")]
#[command(about = "Orca - Standalone orchestrator for AI agent workflows", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Send a quick prompt to the configured LLM
    #[arg(short = 'p', long = "prompt", value_name = "PROMPT")]
    prompt: Option<String>,

    /// Show LLM thinking/reasoning output (overrides config)
    #[arg(long = "show-thinking", global = true)]
    show_thinking: bool,

    /// Hide LLM thinking/reasoning output (overrides config)
    #[arg(long = "no-thinking", global = true, conflicts_with = "show_thinking")]
    no_thinking: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize orca configuration and database
    Init,

    /// Show version information
    Version,

    /// Check system health
    Health {
        /// Output format: text (default), json
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Task management commands (to be implemented)
    #[command(subcommand)]
    Task(TaskCommands),

    /// Workflow management commands (to be implemented)
    #[command(subcommand)]
    Workflow(WorkflowCommands),

    /// Bug tracking commands
    #[command(subcommand)]
    Bug(BugCommands),

    /// Project rule management commands
    #[command(subcommand)]
    Rule(RuleCommands),

    /// Budget management commands
    #[command(subcommand)]
    Budget(BudgetCommands),

    /// Pattern configuration management commands
    #[command(subcommand)]
    Pattern(PatternCommands),

    /// Data management commands (backup, restore, export, import)
    #[command(subcommand)]
    Data(DataCommands),

    /// LLM profile management commands
    #[command(subcommand)]
    LlmProfile(LlmProfileCommands),
}

#[derive(Subcommand)]
enum DataCommands {
    /// Create a backup of databases
    Backup {
        /// Override backup directory
        #[arg(short, long)]
        dir: Option<std::path::PathBuf>,
        /// Include project database (default: true)
        #[arg(long, default_value = "true")]
        include_project: bool,
    },
    /// Restore from a backup
    Restore {
        /// Backup file to restore from
        #[arg(short, long)]
        file: Option<std::path::PathBuf>,
        /// List available backups instead of restoring
        #[arg(long)]
        list: bool,
    },
    /// Export tables to SQL dump
    Export {
        /// Tables to export (comma-separated or "all", groups: llm, budgets, bugs, tasks, patterns, ast)
        #[arg(short, long, default_value = "all")]
        tables: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Import from SQL dump or backup
    Import {
        /// File to import
        file: std::path::PathBuf,
        /// Tables to import (comma-separated or "all")
        #[arg(short, long, default_value = "all")]
        tables: String,
    },
}

#[derive(Subcommand)]
enum RuleCommands {
    /// Create a new project rule
    Create {
        /// Rule name
        name: String,
        /// Rule type: style, security, workflow, custom
        #[arg(short = 't', long)]
        rule_type: String,
        /// JSON configuration for the rule
        #[arg(short, long)]
        config: String,
        /// Rule description
        #[arg(short, long)]
        description: Option<String>,
        /// Severity: error, warning, info (default: warning)
        #[arg(short, long)]
        severity: Option<String>,
    },
    /// List all project rules
    List,
    /// List rules by type
    ListType {
        /// Rule type to filter by: style, security, workflow, custom
        rule_type: String,
    },
    /// Show rule details
    Show {
        /// Rule ID
        id: String,
    },
    /// Update a rule
    Update {
        /// Rule ID
        id: String,
        /// New name
        #[arg(short, long)]
        name: Option<String>,
        /// New description
        #[arg(short, long)]
        description: Option<String>,
        /// New JSON configuration
        #[arg(short, long)]
        config: Option<String>,
        /// New severity: error, warning, info
        #[arg(short, long)]
        severity: Option<String>,
    },
    /// Enable a rule
    Enable {
        /// Rule ID
        id: String,
    },
    /// Disable a rule
    Disable {
        /// Rule ID
        id: String,
    },
    /// Delete a rule
    Delete {
        /// Rule ID
        id: String,
    },
}

#[derive(Subcommand)]
enum BudgetCommands {
    /// Create a new budget
    Create {
        /// Budget name
        name: String,
        /// Budget type: recurring or credit
        #[arg(short = 't', long)]
        budget_type: String,
        /// Renewal interval (days, weeks, months) - for recurring budgets
        #[arg(short, long)]
        interval: Option<String>,
        /// Budget amount - for recurring interval count, for credit the amount
        #[arg(short, long)]
        amount: Option<f64>,
        /// Credit cap (maximum amount) - for credit budgets
        #[arg(short, long)]
        cap: Option<f64>,
        /// Enforcement mode: block or warn
        #[arg(short, long)]
        enforcement: Option<String>,
    },
    /// List all budgets
    List,
    /// Show budget details
    Show {
        /// Budget name
        name: String,
    },
    /// Update a budget
    Update {
        /// Budget name
        name: String,
        /// New budget amount
        #[arg(short, long)]
        amount: Option<f64>,
        /// New enforcement mode: block or warn
        #[arg(short, long)]
        enforcement: Option<String>,
    },
    /// Delete a budget
    Delete {
        /// Budget name
        name: String,
    },
    /// Activate a budget
    Activate {
        /// Budget name
        name: String,
    },
    /// Reset budget usage
    Reset {
        /// Budget name
        name: String,
    },
}

#[derive(Subcommand)]
enum BugCommands {
    /// Create a new bug
    Create {
        /// Bug title
        title: String,
        /// Bug description
        #[arg(short, long)]
        description: Option<String>,
        /// Priority: 1=Critical, 2=High, 3=Medium, 4=Low, 5=Trivial
        #[arg(short, long)]
        priority: Option<u8>,
        /// Assignee name
        #[arg(short, long)]
        assignee: Option<String>,
    },
    /// List all bugs
    List,
    /// List bugs by status
    ListStatus {
        /// Status to filter by: open, in_progress, fixed, wontfix, duplicate
        status: String,
    },
    /// Show bug details
    Show {
        /// Bug ID
        id: String,
    },
    /// Update bug status
    UpdateStatus {
        /// Bug ID
        id: String,
        /// New status: open, in_progress, fixed, wontfix, duplicate
        status: String,
    },
    /// Assign bug to someone
    Assign {
        /// Bug ID
        id: String,
        /// Assignee name
        assignee: String,
    },
    /// Close/fix a bug
    Close {
        /// Bug ID
        id: String,
    },
    /// Delete a bug
    Delete {
        /// Bug ID
        id: String,
    },
    /// Show bug statistics
    Stats,
}

#[derive(Subcommand)]
enum TaskCommands {
    /// Create a new task
    Create {
        /// Task description
        description: String,
        /// Pattern config ID or name to use for this task
        #[arg(long, value_name = "PATTERN")]
        pattern: Option<String>,
    },
    /// List all tasks
    List,
    /// Run a task
    Run {
        /// Task ID
        id: String,
        /// Override pattern config for this run
        #[arg(long, value_name = "PATTERN")]
        pattern: Option<String>,
    },
    /// Cancel a running or pending task
    Cancel {
        /// Task ID
        id: String,
    },
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// Create a new workflow
    Create {
        /// Workflow name
        name: String,
        /// Routing strategy: sequential (default), parallel, or conditional
        #[arg(short, long)]
        strategy: Option<String>,
    },
    /// List all workflows
    List,
    /// Run a workflow
    Run {
        /// Workflow ID
        id: String,
        /// Planner LLM provider and model (format: provider:model)
        #[arg(long)]
        planner: Option<String>,
        /// Worker LLM provider and model (format: provider:model)
        #[arg(long)]
        worker: Option<String>,
        /// Budget ID to use for this workflow run
        #[arg(long)]
        budget: Option<String>,
    },
    /// Show workflow details
    Show {
        /// Workflow ID
        id: String,
    },
    /// Add a task to a workflow
    AddTask {
        /// Workflow ID
        workflow_id: String,
        /// Task ID
        task_id: String,
    },
    /// Remove a task from a workflow
    RemoveTask {
        /// Workflow ID
        workflow_id: String,
        /// Task ID
        task_id: String,
    },
    /// Pause a running workflow
    Pause {
        /// Workflow ID
        id: String,
    },
    /// Resume a paused workflow
    Resume {
        /// Workflow ID
        id: String,
    },
}

#[derive(Subcommand)]
enum PatternCommands {
    /// List all pattern configurations
    List,
    /// List patterns by type
    ListType {
        /// Pattern type to filter by: react, plan_execute, reflection
        pattern_type: String,
    },
    /// Show pattern details
    Show {
        /// Pattern ID or name
        id: String,
    },
    /// Create a new pattern configuration
    Create {
        /// Pattern name
        name: String,
        /// Pattern type: react, plan_execute, reflection
        #[arg(short = 't', long)]
        pattern_type: String,
        /// Maximum iterations (default: 10)
        #[arg(short = 'i', long)]
        max_iterations: Option<i64>,
        /// System prompt for the agent
        #[arg(short = 's', long)]
        system_prompt: Option<String>,
        /// Comma-separated list of allowed tools
        #[arg(long)]
        tools: Option<String>,
        /// Set this pattern as the default
        #[arg(long)]
        default: bool,
    },
    /// Update a pattern configuration
    Update {
        /// Pattern ID
        id: String,
        /// New name
        #[arg(short, long)]
        name: Option<String>,
        /// New maximum iterations
        #[arg(short = 'i', long)]
        max_iterations: Option<i64>,
        /// New system prompt
        #[arg(short = 's', long)]
        system_prompt: Option<String>,
        /// New comma-separated tool list
        #[arg(long)]
        tools: Option<String>,
    },
    /// Delete a pattern configuration
    Delete {
        /// Pattern ID
        id: String,
    },
    /// Set a pattern as the default
    SetDefault {
        /// Pattern ID
        id: String,
    },
}

#[derive(Subcommand)]
enum LlmProfileCommands {
    /// Create a new LLM profile
    Create {
        /// Profile name
        name: String,
        /// Planner provider (e.g., anthropic, openai)
        #[arg(long)]
        planner_provider: String,
        /// Planner model (e.g., claude-sonnet-4-20250514, gpt-4)
        #[arg(long)]
        planner_model: String,
        /// Worker provider (e.g., anthropic, openai)
        #[arg(long)]
        worker_provider: String,
        /// Worker model (e.g., claude-sonnet-4-20250514, gpt-4)
        #[arg(long)]
        worker_model: String,
        /// Profile description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// List all LLM profiles
    List,
    /// Show profile details
    Show {
        /// Profile name
        name: String,
    },
    /// Update a profile
    Update {
        /// Profile name
        name: String,
        /// New planner (format: provider:model)
        #[arg(long)]
        planner: Option<String>,
        /// New worker (format: provider:model)
        #[arg(long)]
        worker: Option<String>,
        /// New description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a profile
    Delete {
        /// Profile name
        name: String,
    },
    /// Activate a profile (makes it the default)
    Activate {
        /// Profile name
        name: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing with file support
    init_logging();

    // Create shutdown coordinator and install signal handlers
    let shutdown_coordinator = std::sync::Arc::new(orca::ShutdownCoordinator::new());
    let _signal_handler = shutdown_coordinator.install_signal_handlers();

    let cli = Cli::parse();

    // Handle -p flag first (quick prompt)
    if let Some(prompt) = cli.prompt {
        // Check if initialized
        if !orca::cli::is_initialized() {
            eprintln!("{}", orca::cli::get_init_instructions());
            return Err(anyhow::anyhow!("Orca not initialized"));
        }

        // Load configuration and apply CLI overrides
        let mut config = orca::load_config().await?;

        // Apply thinking flag overrides
        if cli.show_thinking {
            config.execution.show_thinking = true;
        } else if cli.no_thinking {
            config.execution.show_thinking = false;
        }

        // Create database manager (workspace_root = current directory)
        let db_manager = std::sync::Arc::new(
            orca::DatabaseManager::new(".").await?
        );

        // Load active LLM provider from database
        let active_provider = orca::cli::load_active_llm_profile(db_manager.clone()).await?;
        let provider_config = active_provider.ok_or_else(|| {
            anyhow::anyhow!("No active LLM profile configured. Use 'orca llm-profile create' to add one.")
        })?;

        // Create LlmProvider from the profile
        let llm_provider = std::sync::Arc::new(
            orca::executor::LlmProvider::from_params(
                &provider_config.worker_provider,
                &provider_config.worker_model,
                std::env::var(format!("{}_API_KEY", provider_config.worker_provider.to_uppercase())).ok().as_deref(),
                None,
            )?
        );

        let service = orca::PromptService::new(&config, llm_provider)?;

        match service.send_prompt(&prompt).await {
            Ok(response) => {
                println!("{}", response);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                return Err(anyhow::anyhow!("Failed to get response from LLM: {}", e));
            }
        }
    }

    match cli.command {
        Some(Commands::Init) => {
            println!("Initializing Orca...");
            match orca::init::initialize(false) {
                Ok(_) => {
                    println!("✓ Orca initialized successfully");
                    println!("  Configuration: {}", orca::init::get_user_config_path()?.display());
                    println!("  Database: {}", orca::init::get_database_path()?.display());
                    println!("\nEdit the configuration file to set your LLM API key.");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("✗ Initialization failed: {}", e);
                    Err(e.into())
                }
            }
        }
        Some(Commands::Version) => {
            println!("{}", version_info());
            Ok(())
        }
        Some(Commands::Health { format }) => {
            // Check if initialized
            if !orca::cli::is_initialized() {
                eprintln!("{}", orca::cli::get_init_instructions());
                return Err(anyhow::anyhow!("Orca not initialized"));
            }

            // Get context and run health check
            let context = orca::cli::get_or_create_context().await?;
            let report = orca::HealthChecker::check_context(&context).await?;

            // Output report
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                // Text format
                println!("System Health Check");
                println!("==================");
                println!();
                println!("Overall Status: {}", match report.status {
                    orca::HealthStatus::Healthy => "✓ Healthy",
                    orca::HealthStatus::Degraded => "⚠ Degraded",
                    orca::HealthStatus::Unhealthy => "✗ Unhealthy",
                });
                println!("Total Response Time: {}ms", report.total_response_time_ms);
                println!();
                println!("Component Checks:");
                println!("{:<20} {:<12} {:<10} {}", "Component", "Status", "Time (ms)", "Message");
                println!("{}", "-".repeat(80));

                for check in &report.checks {
                    let status_icon = match check.status {
                        orca::HealthStatus::Healthy => "✓",
                        orca::HealthStatus::Degraded => "⚠",
                        orca::HealthStatus::Unhealthy => "✗",
                    };
                    let message = check.message.as_deref().unwrap_or("N/A");
                    println!("{:<20} {:<12} {:<10} {}",
                        check.name,
                        format!("{} {}", status_icon, check.status),
                        check.response_time_ms,
                        message
                    );
                }
            }

            Ok(())
        }
        Some(Commands::Task(task_cmd)) => {
            // Check if initialized
            if !orca::cli::is_initialized() {
                eprintln!("{}", orca::cli::get_init_instructions());
                return Err(anyhow::anyhow!("Orca not initialized"));
            }

            // Create database manager (workspace_root = current directory)
            let db_manager = std::sync::Arc::new(
                orca::DatabaseManager::new(".").await?
            );

            match task_cmd {
                TaskCommands::Create { description, pattern } => {
                    orca::cli::task::handle_create(db_manager, description, pattern).await?;
                }
                TaskCommands::List => {
                    orca::cli::task::handle_list(db_manager).await?;
                }
                TaskCommands::Run { id, pattern: _ } => {
                    // TODO: Pass pattern to handle_run when executor integration is ready
                    orca::cli::task::handle_run(db_manager, id).await?;
                }
                TaskCommands::Cancel { id } => {
                    orca::cli::task::handle_cancel(db_manager, id).await?;
                }
            }
            Ok(())
        }
        Some(Commands::Workflow(workflow_cmd)) => {
            // Check if initialized
            if !orca::cli::is_initialized() {
                eprintln!("{}", orca::cli::get_init_instructions());
                return Err(anyhow::anyhow!("Orca not initialized"));
            }

            // Create database manager (workspace_root = current directory)
            let db_manager = std::sync::Arc::new(
                orca::DatabaseManager::new(".").await?
            );

            match workflow_cmd {
                WorkflowCommands::Create { name, strategy } => {
                    orca::cli::workflow::handle_create(db_manager, name, strategy).await?;
                }
                WorkflowCommands::List => {
                    orca::cli::workflow::handle_list(db_manager).await?;
                }
                WorkflowCommands::Run { id, planner, worker, budget } => {
                    orca::cli::workflow::handle_run(db_manager, id, planner, worker, budget).await?;
                }
                WorkflowCommands::Show { id } => {
                    orca::cli::workflow::handle_show(db_manager, id).await?;
                }
                WorkflowCommands::AddTask { workflow_id, task_id } => {
                    orca::cli::workflow::handle_add_task(db_manager, workflow_id, task_id).await?;
                }
                WorkflowCommands::RemoveTask { workflow_id, task_id } => {
                    orca::cli::workflow::handle_remove_task(db_manager, workflow_id, task_id).await?;
                }
                WorkflowCommands::Pause { id } => {
                    orca::cli::workflow::handle_pause(db_manager, id).await?;
                }
                WorkflowCommands::Resume { id } => {
                    orca::cli::workflow::handle_resume(db_manager, id).await?;
                }
            }
            Ok(())
        }
        Some(Commands::Bug(bug_cmd)) => {
            // Check if initialized
            if !orca::cli::is_initialized() {
                eprintln!("{}", orca::cli::get_init_instructions());
                return Err(anyhow::anyhow!("Orca not initialized"));
            }

            // Create database manager (workspace_root = current directory)
            let db_manager = std::sync::Arc::new(
                orca::DatabaseManager::new(".").await?
            );

            match bug_cmd {
                BugCommands::Create { title, description, priority, assignee } => {
                    orca::cli::bug::handle_create(db_manager, title, description, priority, assignee).await?;
                }
                BugCommands::List => {
                    orca::cli::bug::handle_list(db_manager).await?;
                }
                BugCommands::ListStatus { status } => {
                    orca::cli::bug::handle_list_status(db_manager, status).await?;
                }
                BugCommands::Show { id } => {
                    orca::cli::bug::handle_show(db_manager, id).await?;
                }
                BugCommands::UpdateStatus { id, status } => {
                    orca::cli::bug::handle_update_status(db_manager, id, status).await?;
                }
                BugCommands::Assign { id, assignee } => {
                    orca::cli::bug::handle_assign(db_manager, id, assignee).await?;
                }
                BugCommands::Close { id } => {
                    orca::cli::bug::handle_close(db_manager, id).await?;
                }
                BugCommands::Delete { id } => {
                    orca::cli::bug::handle_delete(db_manager, id).await?;
                }
                BugCommands::Stats => {
                    orca::cli::bug::handle_stats(db_manager).await?;
                }
            }
            Ok(())
        }
        Some(Commands::Rule(rule_cmd)) => {
            // Check if initialized
            if !orca::cli::is_initialized() {
                eprintln!("{}", orca::cli::get_init_instructions());
                return Err(anyhow::anyhow!("Orca not initialized"));
            }

            // Create database manager (workspace_root = current directory)
            let db_manager = std::sync::Arc::new(
                orca::DatabaseManager::new(".").await?
            );

            match rule_cmd {
                RuleCommands::Create { name, rule_type, config, description, severity } => {
                    orca::cli::rule::handle_create(db_manager, name, rule_type, config, description, severity).await?;
                }
                RuleCommands::List => {
                    orca::cli::rule::handle_list(db_manager).await?;
                }
                RuleCommands::ListType { rule_type } => {
                    orca::cli::rule::handle_list_type(db_manager, rule_type).await?;
                }
                RuleCommands::Show { id } => {
                    orca::cli::rule::handle_show(db_manager, id).await?;
                }
                RuleCommands::Update { id, name, description, config, severity } => {
                    orca::cli::rule::handle_update(db_manager, id, name, description, config, severity).await?;
                }
                RuleCommands::Enable { id } => {
                    orca::cli::rule::handle_enable(db_manager, id).await?;
                }
                RuleCommands::Disable { id } => {
                    orca::cli::rule::handle_disable(db_manager, id).await?;
                }
                RuleCommands::Delete { id } => {
                    orca::cli::rule::handle_delete(db_manager, id).await?;
                }
            }
            Ok(())
        }
        Some(Commands::Budget(budget_cmd)) => {
            // Check if initialized
            if !orca::cli::is_initialized() {
                eprintln!("{}", orca::cli::get_init_instructions());
                return Err(anyhow::anyhow!("Orca not initialized"));
            }

            // Create database manager (workspace_root = current directory)
            let db_manager = std::sync::Arc::new(
                orca::DatabaseManager::new(".").await?
            );

            match budget_cmd {
                BudgetCommands::Create { name, budget_type, interval, amount, cap, enforcement } => {
                    orca::cli::budget::handle_create(db_manager, name, budget_type, interval, amount, cap, enforcement).await?;
                }
                BudgetCommands::List => {
                    orca::cli::budget::handle_list(db_manager).await?;
                }
                BudgetCommands::Show { name } => {
                    orca::cli::budget::handle_show(db_manager, name).await?;
                }
                BudgetCommands::Update { name, amount, enforcement } => {
                    orca::cli::budget::handle_update(db_manager, name, amount, enforcement).await?;
                }
                BudgetCommands::Delete { name } => {
                    orca::cli::budget::handle_delete(db_manager, name).await?;
                }
                BudgetCommands::Activate { name } => {
                    orca::cli::budget::handle_activate(db_manager, name).await?;
                }
                BudgetCommands::Reset { name } => {
                    orca::cli::budget::handle_reset(db_manager, name).await?;
                }
            }
            Ok(())
        }
        Some(Commands::Pattern(pattern_cmd)) => {
            // Check if initialized
            if !orca::cli::is_initialized() {
                eprintln!("{}", orca::cli::get_init_instructions());
                return Err(anyhow::anyhow!("Orca not initialized"));
            }

            // Create database manager (workspace_root = current directory)
            let db_manager = std::sync::Arc::new(
                orca::DatabaseManager::new(".").await?
            );

            match pattern_cmd {
                PatternCommands::List => {
                    orca::cli::pattern::handle_list(db_manager).await?;
                }
                PatternCommands::ListType { pattern_type } => {
                    orca::cli::pattern::handle_list_type(db_manager, pattern_type).await?;
                }
                PatternCommands::Show { id } => {
                    orca::cli::pattern::handle_show(db_manager, id).await?;
                }
                PatternCommands::Create { name, pattern_type, max_iterations, system_prompt, tools, default } => {
                    orca::cli::pattern::handle_create(
                        db_manager, name, pattern_type, max_iterations, system_prompt, tools, default
                    ).await?;
                }
                PatternCommands::Update { id, name, max_iterations, system_prompt, tools } => {
                    orca::cli::pattern::handle_update(db_manager, id, name, max_iterations, system_prompt, tools).await?;
                }
                PatternCommands::Delete { id } => {
                    orca::cli::pattern::handle_delete(db_manager, id).await?;
                }
                PatternCommands::SetDefault { id } => {
                    orca::cli::pattern::handle_set_default(db_manager, id).await?;
                }
            }
            Ok(())
        }
        Some(Commands::Data(data_cmd)) => {
            // Check if initialized
            if !orca::cli::is_initialized() {
                eprintln!("{}", orca::cli::get_init_instructions());
                return Err(anyhow::anyhow!("Orca not initialized"));
            }

            // Create database manager (workspace_root = current directory)
            let db_manager = std::sync::Arc::new(
                orca::DatabaseManager::new(".").await?
            );

            // Load config for backup directory
            let config = orca::load_config().await?;

            match data_cmd {
                DataCommands::Backup { dir, include_project } => {
                    orca::cli::data::handle_backup(db_manager, &config, dir, include_project).await?;
                }
                DataCommands::Restore { file, list } => {
                    orca::cli::data::handle_restore(db_manager, &config, file, list).await?;
                }
                DataCommands::Export { tables, output } => {
                    orca::cli::data::handle_export(db_manager, &config, tables, output).await?;
                }
                DataCommands::Import { file, tables } => {
                    orca::cli::data::handle_import(db_manager, &config, file, tables).await?;
                }
            }
            Ok(())
        }
        Some(Commands::LlmProfile(profile_cmd)) => {
            // Check if initialized
            if !orca::cli::is_initialized() {
                eprintln!("{}", orca::cli::get_init_instructions());
                return Err(anyhow::anyhow!("Orca not initialized"));
            }

            // Create database manager (workspace_root = current directory)
            let db_manager = std::sync::Arc::new(
                orca::DatabaseManager::new(".").await?
            );

            match profile_cmd {
                LlmProfileCommands::Create { name, planner_provider, planner_model, worker_provider, worker_model, description } => {
                    orca::cli::llm_profile::handle_create(db_manager, name, planner_provider, planner_model, worker_provider, worker_model, description).await?;
                }
                LlmProfileCommands::List => {
                    orca::cli::llm_profile::handle_list(db_manager).await?;
                }
                LlmProfileCommands::Show { name } => {
                    orca::cli::llm_profile::handle_show(db_manager, name).await?;
                }
                LlmProfileCommands::Update { name, planner, worker, description } => {
                    orca::cli::llm_profile::handle_update(db_manager, name, planner, worker, description).await?;
                }
                LlmProfileCommands::Delete { name } => {
                    orca::cli::llm_profile::handle_delete(db_manager, name).await?;
                }
                LlmProfileCommands::Activate { name } => {
                    orca::cli::llm_profile::handle_activate(db_manager, name).await?;
                }
            }
            Ok(())
        }
        None => {
            // Check if initialized
            if !orca::cli::is_initialized() {
                eprintln!("{}", orca::cli::get_init_instructions());
                return Err(anyhow::anyhow!("Orca not initialized"));
            }

            // Launch interactive TUI
            println!("Launching Orca TUI...");
            let mut app = orca::tui::App::new();

            // Try to load health report
            if let Ok(context) = orca::cli::get_or_create_context().await {
                if let Ok(report) = orca::HealthChecker::check_context(&context).await {
                    app.health_report = Some(report);
                }
            }

            app.add_message("Welcome to Orca TUI! Type in the prompts section to start.".to_string());

            // Run the TUI
            orca::tui::run_tui(&mut app).await?;

            Ok(())
        }
    }
}
