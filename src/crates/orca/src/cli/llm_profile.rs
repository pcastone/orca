//! LLM profile management command handlers

use crate::error::{OrcaError, Result};
use crate::models::LlmProfile;
use crate::repositories::LlmProfileRepository;
use crate::DatabaseManager;
use colored::Colorize;
use std::sync::Arc;
use tabled::{Table, Tabled};
use uuid::Uuid;

/// LLM Profile display row for table output
#[derive(Tabled)]
struct LlmProfileRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Planner")]
    planner: String,
    #[tabled(rename = "Worker")]
    worker: String,
    #[tabled(rename = "Active")]
    active: String,
}

/// Handle profile create command
pub async fn handle_create(
    db_manager: Arc<DatabaseManager>,
    name: String,
    planner_provider: String,
    planner_model: String,
    worker_provider: String,
    worker_model: String,
    description: Option<String>,
) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = LlmProfileRepository::new(user_db);

    // Check if profile already exists
    if repo.get_by_name(&name).await?.is_some() {
        return Err(OrcaError::Other(format!("Profile already exists: {}", name)));
    }

    let mut profile = LlmProfile::new(
        Uuid::new_v4().to_string(),
        name,
        planner_provider,
        planner_model,
        worker_provider,
        worker_model,
    );

    if let Some(desc) = description {
        profile = profile.with_description(desc);
    }

    repo.create(profile.clone()).await?;

    println!("{}", "✓ Profile created successfully".green().bold());
    println!("  Name: {}", profile.name);
    println!("  Planner: {}:{}", profile.planner_provider, profile.planner_model);
    println!("  Worker: {}:{}", profile.worker_provider, profile.worker_model);

    Ok(())
}

/// Handle profile list command
pub async fn handle_list(db_manager: Arc<DatabaseManager>) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = LlmProfileRepository::new(user_db);
    let profiles = repo.list_all().await?;

    if profiles.is_empty() {
        println!("{}", "No profiles found".yellow());
        return Ok(());
    }

    let rows: Vec<LlmProfileRow> = profiles
        .into_iter()
        .map(|profile| {
            let planner = format!("{}:{}", profile.planner_provider, profile.planner_model);
            let worker = format!("{}:{}", profile.worker_provider, profile.worker_model);
            let active = if profile.active {
                "✓".green().to_string()
            } else {
                "-".to_string()
            };

            LlmProfileRow {
                name: profile.name,
                planner,
                worker,
                active,
            }
        })
        .collect();

    println!("{}", Table::new(rows));

    Ok(())
}

/// Handle profile show command
pub async fn handle_show(db_manager: Arc<DatabaseManager>, name: String) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = LlmProfileRepository::new(user_db);
    let profile = repo
        .get_by_name(&name)
        .await?
        .ok_or_else(|| OrcaError::NotFound(format!("Profile not found: {}", name)))?;

    println!("{}", "Profile Details".bold().underline());
    println!("  Name: {}", profile.name);
    println!("  Planner: {}:{}", profile.planner_provider, profile.planner_model);
    println!("  Worker: {}:{}", profile.worker_provider, profile.worker_model);
    println!("  Active: {}", if profile.active { "Yes" } else { "No" });

    if let Some(desc) = profile.description {
        println!("  Description: {}", desc);
    }

    Ok(())
}

/// Handle profile update command
pub async fn handle_update(
    db_manager: Arc<DatabaseManager>,
    name: String,
    new_planner: Option<String>,
    new_worker: Option<String>,
    new_description: Option<String>,
) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = LlmProfileRepository::new(user_db);
    let mut profile = repo
        .get_by_name(&name)
        .await?
        .ok_or_else(|| OrcaError::NotFound(format!("Profile not found: {}", name)))?;

    if let Some(planner_spec) = new_planner {
        let parts: Vec<&str> = planner_spec.split(':').collect();
        if parts.len() == 2 {
            profile.planner_provider = parts[0].to_string();
            profile.planner_model = parts[1].to_string();
        } else {
            return Err(OrcaError::Config(
                "Planner must be in format 'provider:model'".to_string(),
            ));
        }
    }

    if let Some(worker_spec) = new_worker {
        let parts: Vec<&str> = worker_spec.split(':').collect();
        if parts.len() == 2 {
            profile.worker_provider = parts[0].to_string();
            profile.worker_model = parts[1].to_string();
        } else {
            return Err(OrcaError::Config(
                "Worker must be in format 'provider:model'".to_string(),
            ));
        }
    }

    if let Some(desc) = new_description {
        profile.description = Some(desc);
    }

    repo.update(&profile).await?;

    println!("{}", "✓ Profile updated successfully".green().bold());
    println!("  Name: {}", profile.name);

    Ok(())
}

/// Handle profile delete command
pub async fn handle_delete(db_manager: Arc<DatabaseManager>, name: String) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = LlmProfileRepository::new(user_db);
    let profile = repo
        .get_by_name(&name)
        .await?
        .ok_or_else(|| OrcaError::NotFound(format!("Profile not found: {}", name)))?;

    repo.delete(&profile.id).await?;

    println!("{}", "✓ Profile deleted successfully".green().bold());
    println!("  Name: {}", profile.name);

    Ok(())
}

/// Handle profile activate command
pub async fn handle_activate(db_manager: Arc<DatabaseManager>, name: String) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = LlmProfileRepository::new(user_db);
    let profile = repo
        .get_by_name(&name)
        .await?
        .ok_or_else(|| OrcaError::NotFound(format!("Profile not found: {}", name)))?;

    repo.activate(&profile.id).await?;

    println!("{}", "✓ Profile activated successfully".green().bold());
    println!("  Name: {}", profile.name);

    Ok(())
}
