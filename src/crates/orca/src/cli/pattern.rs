//! Pattern command handlers
//!
//! CLI handlers for pattern configuration management.

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::{PatternConfig, PatternType as ModelPatternType};
use crate::repositories::PatternConfigRepository;
use crate::DatabaseManager;
use colored::Colorize;
use std::sync::Arc;

/// Handle pattern list command
pub async fn handle_list(db_manager: Arc<DatabaseManager>) -> Result<()> {
    let user_db = db_manager.user_db();
    let repo = PatternConfigRepository::new(user_db.clone());
    let patterns = repo.list().await?;

    if patterns.is_empty() {
        println!("{}", "No pattern configurations found".yellow());
        return Ok(());
    }

    println!("Pattern Configurations:");
    println!(
        "{:<24} {:<20} {:<14} {:<10} {:<8}",
        "ID", "Name", "Type", "Max Iter", "Default"
    );
    println!("{}", "-".repeat(80));

    for pattern in patterns {
        let default_marker = if pattern.is_default { "*" } else { "" };
        let name = if pattern.name.len() > 18 {
            format!("{}...", &pattern.name[..15])
        } else {
            pattern.name.clone()
        };

        println!(
            "{:<24} {:<20} {:<14} {:<10} {:<8}",
            pattern.id,
            name,
            pattern.pattern_type,
            pattern.max_iterations,
            default_marker
        );
    }

    println!();
    println!("{}", "* = default pattern".dimmed());

    Ok(())
}

/// Handle pattern show command
pub async fn handle_show(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let user_db = db_manager.user_db();
    let repo = PatternConfigRepository::new(user_db.clone());
    let pattern = repo.find_by_id(&id).await?;

    println!("Pattern Configuration Details:");
    println!("{}", "-".repeat(50));
    println!("ID:             {}", pattern.id);
    println!("Name:           {}", pattern.name);
    println!("Type:           {}", pattern.pattern_type);
    println!("Max Iterations: {}", pattern.max_iterations);
    println!("Is Default:     {}", if pattern.is_default { "Yes" } else { "No" });
    println!("Usage Count:    {}", pattern.usage_count);

    if let Some(ref prompt) = pattern.system_prompt {
        println!("System Prompt:");
        // Truncate long prompts
        if prompt.len() > 200 {
            println!("  {}...", &prompt[..200]);
        } else {
            println!("  {}", prompt);
        }
    }

    // tools is a String (JSON array), not Option<String>
    if !pattern.tools.is_empty() && pattern.tools != "[]" {
        println!("Tools:          {}", pattern.tools);
    }

    // config is a String (JSON object), not Option<String>
    if !pattern.config.is_empty() && pattern.config != "{}" {
        println!("Config:         {}", pattern.config);
    }

    Ok(())
}

/// Handle pattern create command
pub async fn handle_create(
    db_manager: Arc<DatabaseManager>,
    name: String,
    pattern_type: String,
    max_iterations: Option<i64>,
    system_prompt: Option<String>,
    tools: Option<String>,
    set_default: bool,
) -> Result<()> {
    let user_db = db_manager.user_db();
    let repo = PatternConfigRepository::new(user_db.clone());

    // Parse pattern type
    let ptype = parse_pattern_type(&pattern_type)?;

    // Create pattern config
    let mut pattern = PatternConfig::new(&name, ptype)
        .with_max_iterations(max_iterations.unwrap_or(10));

    if let Some(prompt) = system_prompt {
        pattern = pattern.with_system_prompt(&prompt);
    }

    if let Some(tool_list) = tools {
        let tools_vec: Vec<&str> = tool_list.split(',').map(|s| s.trim()).collect();
        pattern = pattern.with_tools(tools_vec);
    }

    // Save pattern
    repo.save(&pattern).await?;

    // Set as default if requested
    if set_default {
        repo.set_default(&pattern.id).await?;
    }

    println!("{}", "✓ Pattern created successfully".green().bold());
    println!("  ID: {}", pattern.id);
    println!("  Name: {}", pattern.name);
    println!("  Type: {}", pattern.pattern_type);
    println!("  Max Iterations: {}", pattern.max_iterations);

    if set_default {
        println!("  {} This pattern is now the default", "*".yellow());
    }

    Ok(())
}

/// Handle pattern update command
pub async fn handle_update(
    db_manager: Arc<DatabaseManager>,
    id: String,
    name: Option<String>,
    max_iterations: Option<i64>,
    system_prompt: Option<String>,
    tools: Option<String>,
) -> Result<()> {
    let user_db = db_manager.user_db();
    let repo = PatternConfigRepository::new(user_db.clone());

    // Load existing pattern
    let mut pattern = repo.find_by_id(&id).await?;

    // Update fields if provided
    if let Some(n) = name {
        pattern.name = n;
    }
    if let Some(iter) = max_iterations {
        pattern.max_iterations = iter;
    }
    if let Some(prompt) = system_prompt {
        pattern.system_prompt = Some(prompt);
    }
    if let Some(tool_list) = tools {
        // Convert comma-separated to JSON array
        let tools_vec: Vec<&str> = tool_list.split(',').map(|s| s.trim()).collect();
        pattern.tools = serde_json::to_string(&tools_vec).unwrap_or_else(|_| "[]".to_string());
    }

    // Save updates
    repo.update(&pattern).await?;

    println!("{}", "✓ Pattern updated successfully".green().bold());
    println!("  ID: {}", pattern.id);
    println!("  Name: {}", pattern.name);

    Ok(())
}

/// Handle pattern delete command
pub async fn handle_delete(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let user_db = db_manager.user_db();
    let repo = PatternConfigRepository::new(user_db.clone());

    // Check if pattern exists
    let pattern = repo.find_by_id(&id).await?;

    if pattern.is_default {
        return Err(OrcaError::Other(
            "Cannot delete the default pattern. Set another pattern as default first.".to_string(),
        ));
    }

    // Delete pattern
    repo.delete(&id).await?;

    println!("{}", "✓ Pattern deleted successfully".green().bold());
    println!("  ID: {}", id);

    Ok(())
}

/// Handle pattern set-default command
pub async fn handle_set_default(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let user_db = db_manager.user_db();
    let repo = PatternConfigRepository::new(user_db.clone());

    // Check if pattern exists
    let pattern = repo.find_by_id(&id).await?;

    // Set as default
    repo.set_default(&id).await?;

    println!("{}", "✓ Default pattern updated".green().bold());
    println!("  ID: {}", id);
    println!("  Name: {}", pattern.name);

    Ok(())
}

/// Handle pattern list-type command
pub async fn handle_list_type(db_manager: Arc<DatabaseManager>, pattern_type: String) -> Result<()> {
    let user_db = db_manager.user_db();
    let repo = PatternConfigRepository::new(user_db.clone());
    let patterns = repo.list_by_type(&pattern_type).await?;

    if patterns.is_empty() {
        println!("{}", format!("No {} pattern configurations found", pattern_type).yellow());
        return Ok(());
    }

    println!("{} Pattern Configurations:", pattern_type);
    println!(
        "{:<24} {:<20} {:<10} {:<8}",
        "ID", "Name", "Max Iter", "Default"
    );
    println!("{}", "-".repeat(65));

    for pattern in patterns {
        let default_marker = if pattern.is_default { "*" } else { "" };
        println!(
            "{:<24} {:<20} {:<10} {:<8}",
            pattern.id, pattern.name, pattern.max_iterations, default_marker
        );
    }

    Ok(())
}

/// Parse pattern type string to enum
fn parse_pattern_type(s: &str) -> Result<ModelPatternType> {
    match s.to_lowercase().as_str() {
        "react" => Ok(ModelPatternType::React),
        "plan_execute" | "plan-execute" | "planexecute" => Ok(ModelPatternType::PlanExecute),
        "reflection" => Ok(ModelPatternType::Reflection),
        "lats" => Ok(ModelPatternType::Lats),
        "storm" => Ok(ModelPatternType::Storm),
        "codeact" | "code_act" => Ok(ModelPatternType::CodeAct),
        "tot" | "tree_of_thought" => Ok(ModelPatternType::Tot),
        "cot" | "chain_of_thought" => Ok(ModelPatternType::Cot),
        "got" | "graph_of_thought" => Ok(ModelPatternType::Got),
        _ => Err(OrcaError::Other(format!(
            "Unknown pattern type: {}. Valid types: react, plan_execute, reflection, lats, storm, codeact, tot, cot, got",
            s
        ))),
    }
}

/// Look up a pattern by ID or name
pub async fn find_pattern(db: Arc<Database>, id_or_name: &str) -> Result<PatternConfig> {
    let repo = PatternConfigRepository::new(db);

    // Try by ID first
    match repo.find_by_id(id_or_name).await {
        Ok(pattern) => Ok(pattern),
        Err(_) => {
            // Try by name
            repo.find_by_name(id_or_name).await
        }
    }
}
