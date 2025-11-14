//! Bug tracking command handlers

use crate::error::{OrcaError, Result};
use crate::models::{Bug, BugPriority, BugStatus};
use crate::repositories::BugRepository;
use crate::DatabaseManager;
use chrono::{DateTime, Utc};
use colored::Colorize;
use std::sync::Arc;
use tabled::{Table, Tabled};

/// Bug display row for table output
#[derive(Tabled)]
struct BugRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Priority")]
    priority: String,
    #[tabled(rename = "Assignee")]
    assignee: String,
    #[tabled(rename = "Created")]
    created: String,
}

/// Handle bug create command
pub async fn handle_create(
    db_manager: Arc<DatabaseManager>,
    title: String,
    description: Option<String>,
    priority: Option<u8>,
    assignee: Option<String>,
) -> Result<()> {
    // Ensure project database exists
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = BugRepository::new(project_db.clone());

    // Create new bug
    let mut bug = Bug::new(title);

    if let Some(desc) = description {
        bug = bug.with_description(desc);
    }

    if let Some(p) = priority {
        let priority = match p {
            1 => BugPriority::Critical,
            2 => BugPriority::High,
            3 => BugPriority::Medium,
            4 => BugPriority::Low,
            5 => BugPriority::Trivial,
            _ => BugPriority::Medium,
        };
        bug = bug.with_priority(priority);
    }

    if let Some(assignee_name) = assignee {
        bug = bug.with_assignee(assignee_name);
    }

    // Save to database
    repo.save(&bug).await?;

    println!("{}", "✓ Bug created successfully".green().bold());
    println!("  ID: {}", bug.id);
    println!("  Title: {}", bug.title);
    println!("  Status: {}", bug.status);
    println!("  Priority: {}", bug.priority);

    Ok(())
}

/// Handle bug list command
pub async fn handle_list(db_manager: Arc<DatabaseManager>) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = BugRepository::new(project_db.clone());
    let bugs = repo.list().await?;

    if bugs.is_empty() {
        println!("{}", "No bugs found".yellow());
        return Ok(());
    }

    // Convert to table rows
    let rows: Vec<BugRow> = bugs
        .into_iter()
        .map(|bug| {
            let created = DateTime::from_timestamp(bug.created_at, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let title = if bug.title.len() > 40 {
                format!("{}...", &bug.title[..37])
            } else {
                bug.title.clone()
            };

            BugRow {
                id: bug.id[..8].to_string(),
                title,
                status: bug.status,
                priority: format!("{}", bug.priority),
                assignee: bug.assignee.unwrap_or_else(|| "-".to_string()),
                created,
            }
        })
        .collect();

    let count = rows.len();
    let table = Table::new(rows).to_string();
    println!("{}", table);
    println!("\nTotal: {} bugs", count);

    Ok(())
}

/// Handle bug list by status command
pub async fn handle_list_status(
    db_manager: Arc<DatabaseManager>,
    status: String,
) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = BugRepository::new(project_db.clone());
    let bugs = repo.list_by_status(&status).await?;

    if bugs.is_empty() {
        println!("{}", format!("No bugs with status '{}'", status).yellow());
        return Ok(());
    }

    // Convert to table rows
    let rows: Vec<BugRow> = bugs
        .into_iter()
        .map(|bug| {
            let created = DateTime::from_timestamp(bug.created_at, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let title = if bug.title.len() > 40 {
                format!("{}...", &bug.title[..37])
            } else {
                bug.title.clone()
            };

            BugRow {
                id: bug.id[..8].to_string(),
                title,
                status: bug.status.clone(),
                priority: format!("{}", bug.priority),
                assignee: bug.assignee.unwrap_or_else(|| "-".to_string()),
                created,
            }
        })
        .collect();

    let count = rows.len();
    let table = Table::new(rows).to_string();
    println!("{}", table);
    println!("\nTotal: {} bugs", count);

    Ok(())
}

/// Handle bug show command
pub async fn handle_show(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = BugRepository::new(project_db.clone());
    let bug = repo.find_by_id(&id).await?;

    // Format timestamps
    let created = DateTime::from_timestamp(bug.created_at, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let updated = DateTime::from_timestamp(bug.updated_at, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let resolved = bug
        .resolved_at
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Not resolved".to_string());

    println!("\n{}", "Bug Details".bold().underline());
    println!("\n{}: {}", "ID".bold(), bug.id);
    println!("{}: {}", "Title".bold(), bug.title);
    println!("{}: {}", "Status".bold(), colorize_status(&bug.status));
    println!("{}: {}", "Priority".bold(), colorize_priority(bug.priority));

    if let Some(severity) = &bug.severity {
        println!("{}: {}", "Severity".bold(), severity);
    }

    if let Some(assignee) = &bug.assignee {
        println!("{}: {}", "Assignee".bold(), assignee);
    }

    if let Some(reporter) = &bug.reporter {
        println!("{}: {}", "Reporter".bold(), reporter);
    }

    println!("\n{}: {}", "Created".bold(), created);
    println!("{}: {}", "Updated".bold(), updated);
    println!("{}: {}", "Resolved".bold(), resolved);

    if let Some(description) = &bug.description {
        println!("\n{}:", "Description".bold());
        println!("{}", description);
    }

    // Parse and display labels if present
    if bug.labels != "[]" {
        if let Ok(labels) = serde_json::from_str::<Vec<String>>(&bug.labels) {
            if !labels.is_empty() {
                println!("\n{}:", "Labels".bold());
                for label in labels {
                    println!("  • {}", label);
                }
            }
        }
    }

    // Parse and display related files if present
    if bug.related_files != "[]" {
        if let Ok(files) = serde_json::from_str::<Vec<String>>(&bug.related_files) {
            if !files.is_empty() {
                println!("\n{}:", "Related Files".bold());
                for file in files {
                    println!("  • {}", file);
                }
            }
        }
    }

    Ok(())
}

/// Handle bug update status command
pub async fn handle_update_status(
    db_manager: Arc<DatabaseManager>,
    id: String,
    status: String,
) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = BugRepository::new(project_db.clone());
    let mut bug = repo.find_by_id(&id).await?;

    let old_status = bug.status.clone();
    bug.status = status.clone();

    // If marking as fixed, set resolved_at
    if status == BugStatus::Fixed.as_str() {
        bug.resolved_at = Some(Utc::now().timestamp());
    }

    bug.updated_at = Utc::now().timestamp();
    repo.update(&bug).await?;

    println!("{}", "✓ Bug status updated".green().bold());
    println!("  {} → {}", old_status, colorize_status(&status));

    Ok(())
}

/// Handle bug assign command
pub async fn handle_assign(
    db_manager: Arc<DatabaseManager>,
    id: String,
    assignee: String,
) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = BugRepository::new(project_db.clone());
    let mut bug = repo.find_by_id(&id).await?;

    bug.assignee = Some(assignee.clone());
    bug.updated_at = Utc::now().timestamp();
    repo.update(&bug).await?;

    println!("{}", "✓ Bug assigned".green().bold());
    println!("  Assignee: {}", assignee);

    Ok(())
}

/// Handle bug close command
pub async fn handle_close(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = BugRepository::new(project_db.clone());
    let mut bug = repo.find_by_id(&id).await?;

    bug.mark_fixed();
    repo.update(&bug).await?;

    println!("{}", "✓ Bug marked as fixed".green().bold());
    println!("  Status: {}", colorize_status(&bug.status));

    Ok(())
}

/// Handle bug delete command
pub async fn handle_delete(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = BugRepository::new(project_db.clone());

    // Confirm deletion
    println!("{}", "⚠ Are you sure you want to delete this bug? (y/N)".yellow());
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "y" {
        println!("Cancelled");
        return Ok(());
    }

    repo.delete(&id).await?;

    println!("{}", "✓ Bug deleted".green().bold());

    Ok(())
}

/// Colorize status for display
fn colorize_status(status: &str) -> colored::ColoredString {
    match status {
        "open" => status.red(),
        "in_progress" => status.yellow(),
        "fixed" => status.green(),
        "wontfix" => status.bright_black(),
        "duplicate" => status.bright_black(),
        _ => status.normal(),
    }
}

/// Colorize priority for display
fn colorize_priority(priority: i64) -> colored::ColoredString {
    let text = format!("{}", priority);
    match priority {
        1 => text.red().bold(),      // Critical
        2 => text.red(),             // High
        3 => text.yellow(),          // Medium
        4 => text.blue(),            // Low
        5 => text.bright_black(),    // Trivial
        _ => text.normal(),
    }
}
