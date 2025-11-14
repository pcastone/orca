//! Task command handlers

use crate::error::{OrcaError, Result};
use crate::repositories::TaskRepository;
use crate::workflow::Task;
use crate::DatabaseManager;
use chrono::Utc;
use colored::Colorize;
use std::sync::Arc;
use tracing::info;

/// Handle task create command
pub async fn handle_create(db_manager: Arc<DatabaseManager>, description: String) -> Result<()> {
    // Ensure project database exists
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = TaskRepository::new(project_db.clone());

    // Create new task
    let task = Task::new(&description);

    // Save to database using repository
    repo.save(&task).await?;

    println!("{}", "✓ Task created successfully".green().bold());
    println!("  ID: {}", task.id);
    println!("  Description: {}", task.description);
    println!("  Status: {}", task.status);

    Ok(())
}

/// Handle task list command
pub async fn handle_list(db_manager: Arc<DatabaseManager>) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = TaskRepository::new(project_db.clone());

    // Query all tasks using repository
    let tasks = repo.list().await?;

    if tasks.is_empty() {
        println!("{}", "No tasks found".yellow());
        return Ok(());
    }

    println!("Tasks:");
    println!("{:<40} {:<12} {:<50}", "ID", "Status", "Description");
    println!("{}", "-".repeat(102));

    for task in tasks {
        let description = if task.description.len() > 47 {
            format!("{}...", &task.description[..47])
        } else {
            task.description.clone()
        };

        println!("{:<40} {:<12} {:<50}",
            task.id,
            task.status,
            description
        );
    }

    Ok(())
}

/// Handle task run command
pub async fn handle_run(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = TaskRepository::new(project_db.clone());

    // Load task from database using repository
    let mut task = repo.find_by_id(&id).await?;

    println!("Running task: {}", task.description);
    println!("Task ID: {}", task.id);
    println!();

    // Update status to running
    task.status = "running".to_string();
    task.started_at = Some(Utc::now().timestamp());
    repo.update(&task).await?;

    println!("{}", "✓ Task execution started".green());
    println!("  Note: Full task execution requires ExecutionContext integration");
    println!("  Task status updated to 'running'");

    Ok(())
}

/// Handle task cancel command
pub async fn handle_cancel(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = TaskRepository::new(project_db.clone());

    // Load task to display info
    let task = repo.find_by_id(&id).await?;

    println!("Cancelling task: {}", task.description);
    println!("Task ID: {}", task.id);
    println!("Current Status: {}", task.status);
    println!();

    // Cancel task using repository
    info!(task_id = %task.id, "Cancelling task");
    repo.cancel_task(&id).await?;

    println!("{}", "✓ Task cancelled successfully".green().bold());

    Ok(())
}
