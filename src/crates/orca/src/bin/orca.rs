//! Orca CLI - Standalone orchestrator for AI agent workflows
//!
//! Main entry point for the orca command-line tool.

use clap::{Parser, Subcommand};
use orca::version_info;

#[derive(Parser)]
#[command(name = "orca")]
#[command(about = "Orca - Standalone orchestrator for AI agent workflows", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
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
}

#[derive(Subcommand)]
enum TaskCommands {
    /// Create a new task
    Create {
        /// Task description
        description: String,
    },
    /// List all tasks
    List,
    /// Run a task
    Run {
        /// Task ID
        id: String,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create shutdown coordinator and install signal handlers
    let shutdown_coordinator = std::sync::Arc::new(orca::ShutdownCoordinator::new());
    let _signal_handler = shutdown_coordinator.install_signal_handlers();

    let cli = Cli::parse();

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
                TaskCommands::Create { description } => {
                    orca::cli::task::handle_create(db_manager, description).await?;
                }
                TaskCommands::List => {
                    orca::cli::task::handle_list(db_manager).await?;
                }
                TaskCommands::Run { id } => {
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
                WorkflowCommands::Run { id } => {
                    orca::cli::workflow::handle_run(db_manager, id).await?;
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
        None => {
            println!("{}", version_info());
            println!("\nUse --help to see available commands");
            Ok(())
        }
    }
}
