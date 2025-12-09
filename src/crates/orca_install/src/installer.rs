//! Main installer orchestration
//!
//! Coordinates database seeding and configuration file generation.

use crate::config::ConfigGenerator;
use crate::database::DatabaseSeeder;
use crate::schema::{AcoInstallConfig, OrcaInstallConfig};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// Installation options
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Force overwrite existing files/data
    pub force: bool,
    /// Dry run - show what would be done
    pub dry_run: bool,
    /// User config directory (default: ~/.orca)
    pub user_dir: PathBuf,
    /// ACO config directory (default: ~/.aco)
    pub aco_dir: PathBuf,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            force: false,
            dry_run: false,
            user_dir: dirs::home_dir()
                .map(|h| h.join(".orca"))
                .unwrap_or_else(|| PathBuf::from(".orca")),
            aco_dir: dirs::home_dir()
                .map(|h| h.join(".aco"))
                .unwrap_or_else(|| PathBuf::from(".aco")),
        }
    }
}

/// Main installer for orca and aco applications
pub struct Installer {
    options: InstallOptions,
}

impl Installer {
    pub fn new(options: InstallOptions) -> Self {
        Self { options }
    }

    /// Initialize fresh installation for orca
    pub async fn init_orca(&self, config: &OrcaInstallConfig) -> Result<()> {
        info!("Initializing orca installation");
        info!("User directory: {:?}", self.options.user_dir);

        if self.options.dry_run {
            self.dry_run_orca(config)?;
            return Ok(());
        }

        // Ensure directories exist
        std::fs::create_dir_all(&self.options.user_dir)?;

        // Seed user database
        let user_db_path = self.options.user_dir.join("user.db");
        let db_seeder = DatabaseSeeder::new(self.options.force);
        db_seeder
            .seed_user_db(&user_db_path, &config.orca.database)
            .await?;

        // Generate TOML config
        let toml_path = self.options.user_dir.join("orca.toml");
        let config_gen = ConfigGenerator::new(self.options.force);
        config_gen.generate_orca_toml(&toml_path, &config.orca.toml)?;

        info!("Orca installation complete");
        Ok(())
    }

    /// Initialize fresh installation for aco
    pub fn init_aco(&self, config: &AcoInstallConfig) -> Result<()> {
        info!("Initializing aco installation");
        info!("ACO directory: {:?}", self.options.aco_dir);

        if self.options.dry_run {
            self.dry_run_aco(config)?;
            return Ok(());
        }

        // Ensure directories exist
        std::fs::create_dir_all(&self.options.aco_dir)?;

        // Generate TOML config (ACO has no local database)
        let toml_path = self.options.aco_dir.join("aco.toml");
        let config_gen = ConfigGenerator::new(self.options.force);
        config_gen.generate_aco_toml(&toml_path, &config.aco.toml)?;

        info!("ACO installation complete");
        Ok(())
    }

    /// Initialize project-level orca database
    pub async fn init_orca_project(&self, project_dir: &Path, config: &OrcaInstallConfig) -> Result<()> {
        info!("Initializing orca project at {:?}", project_dir);

        if self.options.dry_run {
            info!("[DRY RUN] Would create project database at {:?}", project_dir.join(".orca/project.db"));
            return Ok(());
        }

        // Ensure .orca directory exists in project
        let orca_dir = project_dir.join(".orca");
        std::fs::create_dir_all(&orca_dir)?;

        // Seed project database
        let project_db_path = orca_dir.join("project.db");
        let db_seeder = DatabaseSeeder::new(self.options.force);
        db_seeder
            .seed_project_db(&project_db_path, &config.orca.database)
            .await?;

        info!("Orca project initialization complete");
        Ok(())
    }

    /// Reset to base configuration (destructive)
    pub async fn reset_orca(&self, config: &OrcaInstallConfig) -> Result<()> {
        info!("Resetting orca to base configuration");

        if !self.options.force {
            error!("Reset requires --force flag to confirm destructive operation");
            anyhow::bail!("Reset requires --force flag");
        }

        // Remove existing databases
        let user_db = self.options.user_dir.join("user.db");
        if user_db.exists() {
            std::fs::remove_file(&user_db)?;
            info!("Removed {:?}", user_db);
        }

        // Reinitialize
        self.init_orca(config).await?;

        info!("Orca reset complete");
        Ok(())
    }

    /// Reset aco configuration
    pub fn reset_aco(&self, config: &AcoInstallConfig) -> Result<()> {
        info!("Resetting aco to base configuration");

        if !self.options.force {
            error!("Reset requires --force flag to confirm destructive operation");
            anyhow::bail!("Reset requires --force flag");
        }

        // Remove existing config
        let aco_toml = self.options.aco_dir.join("aco.toml");
        if aco_toml.exists() {
            std::fs::remove_file(&aco_toml)?;
            info!("Removed {:?}", aco_toml);
        }

        // Reinitialize
        self.init_aco(config)?;

        info!("ACO reset complete");
        Ok(())
    }

    /// Check installation integrity
    pub fn check_orca(&self) -> Result<InstallStatus> {
        info!("Checking orca installation");

        let mut status = InstallStatus::default();

        // Check user directory
        status.user_dir_exists = self.options.user_dir.exists();

        // Check user database
        let user_db = self.options.user_dir.join("user.db");
        status.user_db_exists = user_db.exists();

        // Check TOML config
        let toml_path = self.options.user_dir.join("orca.toml");
        status.toml_exists = toml_path.exists();

        Ok(status)
    }

    /// Check aco installation
    pub fn check_aco(&self) -> Result<InstallStatus> {
        info!("Checking aco installation");

        let mut status = InstallStatus::default();

        // Check aco directory
        status.user_dir_exists = self.options.aco_dir.exists();

        // Check TOML config
        let toml_path = self.options.aco_dir.join("aco.toml");
        status.toml_exists = toml_path.exists();

        // ACO has no database
        status.user_db_exists = true;

        Ok(status)
    }

    fn dry_run_orca(&self, config: &OrcaInstallConfig) -> Result<()> {
        info!("[DRY RUN] Would perform the following orca installation:");
        info!("  Create directory: {:?}", self.options.user_dir);
        info!(
            "  Create user database: {:?}",
            self.options.user_dir.join("user.db")
        );
        info!(
            "  Seed {} LLM providers",
            config.orca.database.llm_providers.len()
        );
        info!(
            "  Seed {} LLM pricing entries",
            config.orca.database.llm_pricing.len()
        );
        info!("  Seed {} budgets", config.orca.database.budgets.len());
        info!(
            "  Seed {} LLM profiles",
            config.orca.database.llm_profiles.len()
        );
        info!("  Seed {} prompts", config.orca.database.prompts.len());
        info!(
            "  Seed {} workflow templates",
            config.orca.database.workflow_templates.len()
        );
        info!(
            "  Seed {} pattern configs",
            config.orca.database.pattern_configs.len()
        );
        info!(
            "  Create TOML config: {:?}",
            self.options.user_dir.join("orca.toml")
        );
        Ok(())
    }

    fn dry_run_aco(&self, _config: &AcoInstallConfig) -> Result<()> {
        info!("[DRY RUN] Would perform the following aco installation:");
        info!("  Create directory: {:?}", self.options.aco_dir);
        info!(
            "  Create TOML config: {:?}",
            self.options.aco_dir.join("aco.toml")
        );
        Ok(())
    }
}

/// Installation status report
#[derive(Debug, Default)]
pub struct InstallStatus {
    pub user_dir_exists: bool,
    pub user_db_exists: bool,
    pub toml_exists: bool,
}

impl InstallStatus {
    pub fn is_complete(&self) -> bool {
        self.user_dir_exists && self.user_db_exists && self.toml_exists
    }
}

impl std::fmt::Display for InstallStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let check = |b: bool| if b { "✓" } else { "✗" };
        writeln!(f, "Installation Status:")?;
        writeln!(f, "  {} User directory", check(self.user_dir_exists))?;
        writeln!(f, "  {} User database", check(self.user_db_exists))?;
        writeln!(f, "  {} TOML config", check(self.toml_exists))?;
        if self.is_complete() {
            writeln!(f, "\nInstallation is complete.")?;
        } else {
            writeln!(f, "\nInstallation is incomplete. Run 'orca_install init' to complete.")?;
        }
        Ok(())
    }
}
