//! Project rule command handlers

use crate::error::{OrcaError, Result};
use crate::models::ProjectRule;
use crate::repositories::ProjectRuleRepository;
use crate::DatabaseManager;
use chrono::DateTime;
use colored::Colorize;
use std::sync::Arc;
use tabled::{Table, Tabled};

/// Rule display row for table output
#[derive(Tabled)]
struct RuleRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Type")]
    rule_type: String,
    #[tabled(rename = "Severity")]
    severity: String,
    #[tabled(rename = "Enabled")]
    enabled: String,
    #[tabled(rename = "Created")]
    created: String,
}

/// Handle rule create command
pub async fn handle_create(
    db_manager: Arc<DatabaseManager>,
    name: String,
    rule_type: String,
    config: String,
    description: Option<String>,
    severity: Option<String>,
) -> Result<()> {
    // Ensure project database exists
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = ProjectRuleRepository::new(project_db.clone());

    // Validate rule type
    match rule_type.as_str() {
        "style" | "security" | "workflow" | "custom" => {}
        _ => {
            return Err(OrcaError::Other(
                format!("Invalid rule type: {}. Must be one of: style, security, workflow, custom", rule_type)
            ));
        }
    }

    // Validate severity if provided
    if let Some(ref sev) = severity {
        match sev.as_str() {
            "error" | "warning" | "info" => {}
            _ => {
                return Err(OrcaError::Other(
                    format!("Invalid severity: {}. Must be one of: error, warning, info", sev)
                ));
            }
        }
    }

    // Validate config is valid JSON
    if serde_json::from_str::<serde_json::Value>(&config).is_err() {
        return Err(OrcaError::Other("Config must be valid JSON".to_string()));
    }

    // Create new rule
    let mut rule = ProjectRule::new(name, rule_type, config);

    if let Some(desc) = description {
        rule = rule.with_description(desc);
    }

    if let Some(sev) = severity {
        rule = rule.with_severity(&sev);
    }

    // Save to database
    repo.save(&rule).await?;

    println!("{}", "✓ Project rule created successfully".green().bold());
    println!("  ID: {}", rule.id);
    println!("  Name: {}", rule.name);
    println!("  Type: {}", rule.rule_type);
    println!("  Severity: {}", colorize_severity(&rule.severity));
    println!("  Enabled: {}", if rule.enabled { "Yes" } else { "No" });

    Ok(())
}

/// Handle rule list command
pub async fn handle_list(db_manager: Arc<DatabaseManager>) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = ProjectRuleRepository::new(project_db.clone());
    let rules = repo.list().await?;

    if rules.is_empty() {
        println!("{}", "No project rules found".yellow());
        return Ok(());
    }

    // Convert to table rows
    let rows: Vec<RuleRow> = rules
        .into_iter()
        .map(|rule| {
            let created = DateTime::from_timestamp(rule.created_at, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let name = if rule.name.len() > 30 {
                format!("{}...", &rule.name[..27])
            } else {
                rule.name.clone()
            };

            RuleRow {
                id: rule.id[..8].to_string(),
                name,
                rule_type: rule.rule_type.clone(),
                severity: rule.severity.clone(),
                enabled: if rule.enabled { "✓" } else { "✗" }.to_string(),
                created,
            }
        })
        .collect();

    let count = rows.len();
    let table = Table::new(rows).to_string();
    println!("{}", table);
    println!("\nTotal: {} rules", count);

    Ok(())
}

/// Handle rule list by type command
pub async fn handle_list_type(
    db_manager: Arc<DatabaseManager>,
    rule_type: String,
) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = ProjectRuleRepository::new(project_db.clone());
    let rules = repo.list_by_type(&rule_type).await?;

    if rules.is_empty() {
        println!("{}", format!("No rules with type '{}'", rule_type).yellow());
        return Ok(());
    }

    // Convert to table rows
    let rows: Vec<RuleRow> = rules
        .into_iter()
        .map(|rule| {
            let created = DateTime::from_timestamp(rule.created_at, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let name = if rule.name.len() > 30 {
                format!("{}...", &rule.name[..27])
            } else {
                rule.name.clone()
            };

            RuleRow {
                id: rule.id[..8].to_string(),
                name,
                rule_type: rule.rule_type.clone(),
                severity: rule.severity.clone(),
                enabled: if rule.enabled { "✓" } else { "✗" }.to_string(),
                created,
            }
        })
        .collect();

    let count = rows.len();
    let table = Table::new(rows).to_string();
    println!("{}", table);
    println!("\nTotal: {} rules", count);

    Ok(())
}

/// Handle rule show command
pub async fn handle_show(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = ProjectRuleRepository::new(project_db.clone());
    let rule = repo.find_by_id(&id).await?;

    // Format timestamps
    let created = DateTime::from_timestamp(rule.created_at, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let updated = DateTime::from_timestamp(rule.updated_at, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    println!("\n{}", "Rule Details".bold().underline());
    println!("\n{}: {}", "ID".bold(), rule.id);
    println!("{}: {}", "Name".bold(), rule.name);
    println!("{}: {}", "Type".bold(), rule.rule_type);
    println!("{}: {}", "Severity".bold(), colorize_severity(&rule.severity));
    println!("{}: {}", "Enabled".bold(), if rule.enabled { "Yes".green() } else { "No".red() });

    if let Some(description) = &rule.description {
        println!("\n{}:", "Description".bold());
        println!("{}", description);
    }

    println!("\n{}:", "Configuration".bold());
    // Pretty print JSON config
    if let Ok(config_value) = serde_json::from_str::<serde_json::Value>(&rule.config) {
        if let Ok(pretty_json) = serde_json::to_string_pretty(&config_value) {
            println!("{}", pretty_json);
        } else {
            println!("{}", rule.config);
        }
    } else {
        println!("{}", rule.config);
    }

    println!("\n{}: {}", "Created".bold(), created);
    println!("{}: {}", "Updated".bold(), updated);

    Ok(())
}

/// Handle rule enable command
pub async fn handle_enable(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = ProjectRuleRepository::new(project_db.clone());
    repo.enable(&id).await?;

    println!("{}", "✓ Rule enabled".green().bold());

    Ok(())
}

/// Handle rule disable command
pub async fn handle_disable(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = ProjectRuleRepository::new(project_db.clone());
    repo.disable(&id).await?;

    println!("{}", "✓ Rule disabled".green().bold());

    Ok(())
}

/// Handle rule delete command
pub async fn handle_delete(db_manager: Arc<DatabaseManager>, id: String) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = ProjectRuleRepository::new(project_db.clone());

    // Confirm deletion
    println!("{}", "⚠ Are you sure you want to delete this rule? (y/N)".yellow());
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "y" {
        println!("Cancelled");
        return Ok(());
    }

    repo.delete(&id).await?;

    println!("{}", "✓ Rule deleted".green().bold());

    Ok(())
}

/// Handle rule update command
pub async fn handle_update(
    db_manager: Arc<DatabaseManager>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    config: Option<String>,
    severity: Option<String>,
) -> Result<()> {
    let project_db = db_manager
        .project_db()
        .ok_or_else(|| OrcaError::Other("No project database. Run 'orca init' in a project directory.".to_string()))?;

    let repo = ProjectRuleRepository::new(project_db.clone());
    let mut rule = repo.find_by_id(&id).await?;

    // Update fields if provided
    if let Some(n) = name {
        rule.name = n;
    }

    if let Some(desc) = description {
        rule.description = Some(desc);
    }

    if let Some(cfg) = config {
        // Validate config is valid JSON
        if serde_json::from_str::<serde_json::Value>(&cfg).is_err() {
            return Err(OrcaError::Other("Config must be valid JSON".to_string()));
        }
        rule.config = cfg;
    }

    if let Some(sev) = severity {
        match sev.as_str() {
            "error" | "warning" | "info" => {
                rule.severity = sev;
            }
            _ => {
                return Err(OrcaError::Other(
                    format!("Invalid severity: {}. Must be one of: error, warning, info", sev)
                ));
            }
        }
    }

    repo.update(&rule).await?;

    println!("{}", "✓ Rule updated successfully".green().bold());
    println!("  Name: {}", rule.name);
    println!("  Severity: {}", colorize_severity(&rule.severity));

    Ok(())
}

/// Colorize severity for display
fn colorize_severity(severity: &str) -> colored::ColoredString {
    match severity {
        "error" => severity.red().bold(),
        "warning" => severity.yellow(),
        "info" => severity.blue(),
        _ => severity.normal(),
    }
}
