//! Workflow command handlers

use crate::error::{OrcaError, Result};
use crate::repositories::{WorkflowRepository, TaskRepository};
use crate::workflow::Workflow;
use crate::DatabaseManager;
use chrono::Utc;
use colored::Colorize;
use std::sync::Arc;
use tracing::info;

/// Handle workflow create command
pub async fn handle_create(
    db_manager: Arc<DatabaseManager>,
    name: String,
    routing_strategy: Option<String>,
) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = WorkflowRepository::new(project_db.clone());

    // Create new workflow with default react pattern
    let mut workflow = Workflow::new(&name, "react");

    // Set routing strategy if provided, otherwise default to sequential
    let strategy = routing_strategy.unwrap_or_else(|| "sequential".to_string());
    workflow.set_routing_strategy(&strategy);

    // Save to database using repository
    repo.save(&workflow).await?;

    println!("{}", "✓ Workflow created successfully".green().bold());
    println!("  ID: {}", workflow.id);
    println!("  Name: {}", workflow.name);
    println!("  Status: {}", workflow.status);
    println!("  Routing Strategy: {}", workflow.routing_strategy());

    Ok(())
}

/// Handle workflow list command
pub async fn handle_list(db_manager: Arc<DatabaseManager>) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = WorkflowRepository::new(project_db.clone());

    // Query all workflows using repository
    let workflows = repo.list().await?;

    if workflows.is_empty() {
        println!("{}", "No workflows found".yellow());
        return Ok(());
    }

    println!("Workflows:");
    println!("{:<40} {:<12} {:<50}", "ID", "Status", "Name");
    println!("{}", "-".repeat(102));

    for workflow in workflows {
        let name = if workflow.name.len() > 47 {
            format!("{}...", &workflow.name[..47])
        } else {
            workflow.name.clone()
        };

        println!("{:<40} {:<12} {:<50}",
            workflow.id,
            workflow.status,
            name
        );
    }

    Ok(())
}

/// Handle workflow run command
pub async fn handle_run(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = WorkflowRepository::new(project_db.clone());

    // Load workflow from database using repository
    let mut workflow = repo.find_by_id(&id).await?;

    println!("Running workflow: {}", workflow.name);
    println!("Workflow ID: {}", workflow.id);
    println!();

    // Load workflow tasks using repository
    let task_ids = repo.get_task_ids(&workflow.id).await?;

    if task_ids.is_empty() {
        println!("{}", "✗ Workflow has no tasks".red());
        return Ok(());
    }

    println!("Found {} task(s) in workflow", task_ids.len());

    // Update workflow status to running
    workflow.status = "running".to_string();
    workflow.started_at = Some(Utc::now().timestamp());
    repo.update(&workflow).await?;

    println!("{}", "✓ Workflow execution started".green());
    println!("  Note: Full workflow execution requires ExecutionContext integration");
    println!("  Workflow status updated to 'running'");

    Ok(())
}

/// Handle workflow show command
pub async fn handle_show(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let workflow_repo = WorkflowRepository::new(project_db.clone());
    let task_repo = TaskRepository::new(project_db.clone());

    // Load workflow using repository
    let workflow = workflow_repo.find_by_id(&id).await?;

    // Load workflow tasks
    let task_ids = workflow_repo.get_task_ids(&workflow.id).await?;

    // Display workflow details
    println!("\n{}", "Workflow Details".bold().underline());
    println!("\n{}: {}", "ID".bold(), workflow.id);
    println!("{}: {}", "Name".bold(), workflow.name);
    println!("{}: {}", "Status".bold(), workflow.status);
    println!("{}: {}", "Pattern".bold(), workflow.pattern);

    if let Some(desc) = &workflow.description {
        println!("{}: {}", "Description".bold(), desc);
    }

    println!("\n{}: {}", "Created".bold(),
        chrono::DateTime::from_timestamp(workflow.created_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    );

    if let Some(started) = workflow.started_at {
        println!("{}: {}", "Started".bold(),
            chrono::DateTime::from_timestamp(started, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        );
    }

    if let Some(completed) = workflow.completed_at {
        println!("{}: {}", "Completed".bold(),
            chrono::DateTime::from_timestamp(completed, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        );
    }

    println!("\n{}:", format!("Tasks ({})", task_ids.len()).bold());

    if task_ids.is_empty() {
        println!("  No tasks assigned");
    } else {
        for (idx, task_id) in task_ids.iter().enumerate() {
            // Load task details
            match task_repo.find_by_id(task_id).await {
                Ok(task) => {
                    println!("  {}. {} - {} ({})",
                        idx + 1,
                        task.id,
                        task.description,
                        task.status
                    );
                }
                Err(_) => {
                    println!("  {}. {} - [Task not found]", idx + 1, task_id);
                }
            }
        }
    }

    Ok(())
}

/// Handle workflow add-task command
pub async fn handle_add_task(
    db_manager: Arc<DatabaseManager>,
    workflow_id: String,
    task_id: String,
) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let workflow_repo = WorkflowRepository::new(project_db.clone());
    let task_repo = TaskRepository::new(project_db.clone());

    // Verify workflow exists
    if !workflow_repo.exists(&workflow_id).await? {
        println!("{}", format!("✗ Workflow not found: {}", workflow_id).red());
        return Ok(());
    }

    // Verify task exists
    if !task_repo.exists(&task_id).await? {
        println!("{}", format!("✗ Task not found: {}", task_id).red());
        return Ok(());
    }

    // Get current task count to determine sequence
    let task_count = workflow_repo.get_task_count(&workflow_id).await?;
    let sequence = task_count as i32;

    // Add task to workflow
    workflow_repo.add_task(&workflow_id, &task_id, sequence).await?;

    println!("{}", "✓ Task added to workflow".green().bold());
    println!("  Workflow: {}", workflow_id);
    println!("  Task: {}", task_id);
    println!("  Position: {}", sequence + 1);

    Ok(())
}

/// Handle workflow remove-task command
pub async fn handle_remove_task(
    db_manager: Arc<DatabaseManager>,
    workflow_id: String,
    task_id: String,
) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let workflow_repo = WorkflowRepository::new(project_db.clone());

    // Verify workflow exists
    if !workflow_repo.exists(&workflow_id).await? {
        println!("{}", format!("✗ Workflow not found: {}", workflow_id).red());
        return Ok(());
    }

    // Remove task from workflow
    workflow_repo.remove_task(&workflow_id, &task_id).await?;

    println!("{}", "✓ Task removed from workflow".green().bold());
    println!("  Workflow: {}", workflow_id);
    println!("  Task: {}", task_id);

    Ok(())
}

/// Handle workflow pause command
pub async fn handle_pause(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = WorkflowRepository::new(project_db.clone());

    // Load workflow to display info
    let workflow = repo.find_by_id(&id).await?;

    println!("Pausing workflow: {}", workflow.name);
    println!("Workflow ID: {}", workflow.id);
    println!("Current Status: {}", workflow.status);
    println!();

    // Pause workflow using repository
    info!(workflow_id = %workflow.id, "Pausing workflow");
    repo.pause_workflow(&id).await?;

    println!("{}", "✓ Workflow paused successfully".green().bold());

    Ok(())
}

/// Handle workflow resume command
pub async fn handle_resume(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = WorkflowRepository::new(project_db.clone());

    // Load workflow to display info
    let workflow = repo.find_by_id(&id).await?;

    println!("Resuming workflow: {}", workflow.name);
    println!("Workflow ID: {}", workflow.id);
    println!("Current Status: {}", workflow.status);
    println!();

    // Resume workflow using repository
    info!(workflow_id = %workflow.id, "Resuming workflow");
    repo.resume_workflow(&id).await?;

    println!("{}", "✓ Workflow resumed successfully".green().bold());
    println!("\nNote: Use 'orca workflow run {}' to execute the workflow", id);

    Ok(())
}
