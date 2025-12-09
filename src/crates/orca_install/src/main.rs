//! Orca Install - CLI entry point
//!
//! Initializes orca and aco applications from YAML configuration files.

use anyhow::Result;
use clap::{Parser, Subcommand};
use orca_install::installer::{InstallOptions, Installer};
use orca_install::schema::{AcoInstallConfig, OrcaInstallConfig};
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "orca_install")]
#[command(about = "Install and configure orca and aco applications")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to orca YAML config
    #[arg(long, default_value = "config/orca_base_install.yaml")]
    orca_yaml: PathBuf,

    /// Path to aco YAML config
    #[arg(long, default_value = "config/aco_base_install.yaml")]
    aco_yaml: PathBuf,

    /// User config directory
    #[arg(long)]
    user_dir: Option<PathBuf>,

    /// ACO config directory
    #[arg(long)]
    aco_dir: Option<PathBuf>,

    /// Show what would be done without making changes
    #[arg(long)]
    dry_run: bool,

    /// Overwrite existing files/data
    #[arg(long, short)]
    force: bool,

    /// Verbose output
    #[arg(long, short)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize fresh installation
    Init {
        /// Install target: orca, aco, or all
        #[arg(default_value = "all")]
        target: String,
    },
    /// Reset to base configuration (destructive)
    Reset {
        /// Reset target: orca, aco, or all
        #[arg(default_value = "all")]
        target: String,
    },
    /// Check installation integrity
    Check {
        /// Check target: orca, aco, or all
        #[arg(default_value = "all")]
        target: String,
    },
    /// Initialize project-level orca config
    Project {
        /// Project directory (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Build install options
    let mut options = InstallOptions::default();
    options.force = cli.force;
    options.dry_run = cli.dry_run;

    if let Some(user_dir) = cli.user_dir {
        options.user_dir = user_dir;
    }
    if let Some(aco_dir) = cli.aco_dir {
        options.aco_dir = aco_dir;
    }

    let installer = Installer::new(options);

    match cli.command {
        Commands::Init { target } => {
            run_init(&installer, &cli.orca_yaml, &cli.aco_yaml, &target).await?;
        }
        Commands::Reset { target } => {
            run_reset(&installer, &cli.orca_yaml, &cli.aco_yaml, &target).await?;
        }
        Commands::Check { target } => {
            run_check(&installer, &target)?;
        }
        Commands::Project { path } => {
            let orca_config = load_orca_config(&cli.orca_yaml)?;
            installer.init_orca_project(&path, &orca_config).await?;
        }
    }

    Ok(())
}

async fn run_init(
    installer: &Installer,
    orca_yaml: &PathBuf,
    aco_yaml: &PathBuf,
    target: &str,
) -> Result<()> {
    match target {
        "orca" => {
            let config = load_orca_config(orca_yaml)?;
            installer.init_orca(&config).await?;
        }
        "aco" => {
            let config = load_aco_config(aco_yaml)?;
            installer.init_aco(&config)?;
        }
        "all" => {
            if orca_yaml.exists() {
                let orca_config = load_orca_config(orca_yaml)?;
                installer.init_orca(&orca_config).await?;
            } else {
                info!("Skipping orca (config not found at {:?})", orca_yaml);
            }

            if aco_yaml.exists() {
                let aco_config = load_aco_config(aco_yaml)?;
                installer.init_aco(&aco_config)?;
            } else {
                info!("Skipping aco (config not found at {:?})", aco_yaml);
            }
        }
        _ => {
            error!("Unknown target: {}. Use 'orca', 'aco', or 'all'", target);
            anyhow::bail!("Unknown target: {}", target);
        }
    }
    Ok(())
}

async fn run_reset(
    installer: &Installer,
    orca_yaml: &PathBuf,
    aco_yaml: &PathBuf,
    target: &str,
) -> Result<()> {
    match target {
        "orca" => {
            let config = load_orca_config(orca_yaml)?;
            installer.reset_orca(&config).await?;
        }
        "aco" => {
            let config = load_aco_config(aco_yaml)?;
            installer.reset_aco(&config)?;
        }
        "all" => {
            let orca_config = load_orca_config(orca_yaml)?;
            installer.reset_orca(&orca_config).await?;

            let aco_config = load_aco_config(aco_yaml)?;
            installer.reset_aco(&aco_config)?;
        }
        _ => {
            error!("Unknown target: {}. Use 'orca', 'aco', or 'all'", target);
            anyhow::bail!("Unknown target: {}", target);
        }
    }
    Ok(())
}

fn run_check(installer: &Installer, target: &str) -> Result<()> {
    match target {
        "orca" => {
            let status = installer.check_orca()?;
            println!("Orca {}", status);
        }
        "aco" => {
            let status = installer.check_aco()?;
            println!("ACO {}", status);
        }
        "all" => {
            let orca_status = installer.check_orca()?;
            println!("Orca {}", orca_status);
            println!();
            let aco_status = installer.check_aco()?;
            println!("ACO {}", aco_status);
        }
        _ => {
            error!("Unknown target: {}. Use 'orca', 'aco', or 'all'", target);
            anyhow::bail!("Unknown target: {}", target);
        }
    }
    Ok(())
}

fn load_orca_config(path: &PathBuf) -> Result<OrcaInstallConfig> {
    info!("Loading orca config from {:?}", path);
    if !path.exists() {
        anyhow::bail!("Orca config file not found: {:?}", path);
    }
    OrcaInstallConfig::from_file(path)
}

fn load_aco_config(path: &PathBuf) -> Result<AcoInstallConfig> {
    info!("Loading aco config from {:?}", path);
    if !path.exists() {
        anyhow::bail!("ACO config file not found: {:?}", path);
    }
    AcoInstallConfig::from_file(path)
}
