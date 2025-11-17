//! Budget management command handlers

use crate::error::{OrcaError, Result};
use crate::models::{Budget, BudgetEnforcement, BudgetType, RenewalInterval};
use crate::repositories::BudgetRepository;
use crate::DatabaseManager;
use colored::Colorize;
use std::sync::Arc;
use tabled::{Table, Tabled};
use uuid::Uuid;

/// Budget display row for table output
#[derive(Tabled)]
struct BudgetRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Type")]
    budget_type: String,
    #[tabled(rename = "Usage")]
    usage: String,
    #[tabled(rename = "Remaining")]
    remaining: String,
    #[tabled(rename = "Enforcement")]
    enforcement: String,
}

/// Handle budget create command
pub async fn handle_create(
    db_manager: Arc<DatabaseManager>,
    name: String,
    budget_type: String,
    interval: Option<String>,
    amount: Option<f64>,
    cap: Option<f64>,
    enforcement: Option<String>,
) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = BudgetRepository::new(user_db);

    // Determine enforcement
    let enforce = match enforcement.as_deref() {
        Some("block") => BudgetEnforcement::Block,
        Some("warn") => BudgetEnforcement::Warn,
        _ => BudgetEnforcement::Warn,
    };

    let budget = match budget_type.to_lowercase().as_str() {
        "recurring" => {
            let interval_unit = match interval.as_deref() {
                Some("days") => RenewalInterval::Days,
                Some("weeks") => RenewalInterval::Weeks,
                Some("months") => RenewalInterval::Months,
                _ => RenewalInterval::Months,
            };

            let interval_value = amount.unwrap_or(1.0) as u32;
            Budget::new_recurring(
                Uuid::new_v4().to_string(),
                name,
                interval_unit,
                interval_value,
                enforce,
            )
        }
        "credit" => {
            let amount = amount.ok_or_else(|| {
                OrcaError::Config("Credit amount required for credit budgets".to_string())
            })?;

            Budget::new_credit(Uuid::new_v4().to_string(), name, amount, cap, enforce)
        }
        _ => {
            return Err(OrcaError::Config(
                "Budget type must be 'recurring' or 'credit'".to_string(),
            ))
        }
    };

    repo.create(budget.clone()).await?;

    println!("{}", "✓ Budget created successfully".green().bold());
    println!("  Name: {}", budget.name);
    println!("  Type: {:?}", budget.budget_type);
    println!("  Enforcement: {:?}", budget.enforcement);

    Ok(())
}

/// Handle budget list command
pub async fn handle_list(db_manager: Arc<DatabaseManager>) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = BudgetRepository::new(user_db);
    let budgets = repo.list_all().await?;

    if budgets.is_empty() {
        println!("{}", "No budgets found".yellow());
        return Ok(());
    }

    let rows: Vec<BudgetRow> = budgets
        .into_iter()
        .map(|budget| {
            let usage = match budget.budget_type {
                BudgetType::Credit => format!("${:.2}", budget.current_usage),
                BudgetType::Recurring => format!("{:.2}", budget.current_usage),
            };

            let remaining = match budget.budget_type {
                BudgetType::Credit => format!("${:.2}", budget.remaining()),
                BudgetType::Recurring => "N/A".to_string(),
            };

            BudgetRow {
                name: budget.name,
                budget_type: format!("{:?}", budget.budget_type),
                usage,
                remaining,
                enforcement: format!("{:?}", budget.enforcement),
            }
        })
        .collect();

    println!("{}", Table::new(rows));

    Ok(())
}

/// Handle budget show command
pub async fn handle_show(db_manager: Arc<DatabaseManager>, name: String) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = BudgetRepository::new(user_db);
    let budget = repo
        .get_by_name(&name)
        .await?
        .ok_or_else(|| OrcaError::NotFound(format!("Budget not found: {}", name)))?;

    println!("{}", "Budget Details".bold().underline());
    println!("  Name: {}", budget.name);
    println!("  Type: {:?}", budget.budget_type);
    println!("  Enforcement: {:?}", budget.enforcement);
    println!("  Current Usage: {:.2}", budget.current_usage);
    println!("  Total Spent: {:.2}", budget.total_spent);

    match budget.budget_type {
        BudgetType::Credit => {
            if let Some(amount) = budget.credit_amount {
                println!("  Credit Amount: ${:.2}", amount);
                println!("  Remaining: ${:.2}", budget.remaining());
                println!("  Usage %: {:.1}%", budget.usage_percentage());
            }
            if let Some(cap) = budget.credit_cap {
                println!("  Cap: ${:.2}", cap);
            }
        }
        BudgetType::Recurring => {
            if let (Some(unit), Some(value)) = (budget.renewal_interval_unit, budget.renewal_interval_value) {
                println!("  Renewal Interval: {} {:?}", value, unit);
            }
            if let Some(next) = budget.next_renewal_date {
                println!("  Next Renewal: {}", next);
            }
        }
    }

    Ok(())
}

/// Handle budget update command
pub async fn handle_update(
    db_manager: Arc<DatabaseManager>,
    name: String,
    new_amount: Option<f64>,
    new_enforcement: Option<String>,
) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = BudgetRepository::new(user_db);
    let mut budget = repo
        .get_by_name(&name)
        .await?
        .ok_or_else(|| OrcaError::NotFound(format!("Budget not found: {}", name)))?;

    if let Some(amount) = new_amount {
        if let BudgetType::Credit = budget.budget_type {
            budget.credit_amount = Some(amount);
        }
    }

    if let Some(enforcement) = new_enforcement {
        budget.enforcement = match enforcement.to_lowercase().as_str() {
            "block" => BudgetEnforcement::Block,
            "warn" => BudgetEnforcement::Warn,
            _ => BudgetEnforcement::Warn,
        };
    }

    repo.update(&budget).await?;

    println!("{}", "✓ Budget updated successfully".green().bold());
    println!("  Name: {}", budget.name);

    Ok(())
}

/// Handle budget delete command
pub async fn handle_delete(db_manager: Arc<DatabaseManager>, name: String) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = BudgetRepository::new(user_db);
    let budget = repo
        .get_by_name(&name)
        .await?
        .ok_or_else(|| OrcaError::NotFound(format!("Budget not found: {}", name)))?;

    repo.delete(&budget.id).await?;

    println!("{}", "✓ Budget deleted successfully".green().bold());
    println!("  Name: {}", budget.name);

    Ok(())
}

/// Handle budget activate command
pub async fn handle_activate(db_manager: Arc<DatabaseManager>, name: String) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = BudgetRepository::new(user_db);
    let budget = repo
        .get_by_name(&name)
        .await?
        .ok_or_else(|| OrcaError::NotFound(format!("Budget not found: {}", name)))?;

    repo.activate(&budget.id).await?;

    println!("{}", "✓ Budget activated successfully".green().bold());
    println!("  Name: {}", budget.name);

    Ok(())
}

/// Handle budget reset command
pub async fn handle_reset(db_manager: Arc<DatabaseManager>, name: String) -> Result<()> {
    let user_db = db_manager.user_db().clone();
    let repo = BudgetRepository::new(user_db);
    let budget = repo
        .get_by_name(&name)
        .await?
        .ok_or_else(|| OrcaError::NotFound(format!("Budget not found: {}", name)))?;

    repo.reset_usage(&budget.id).await?;

    println!("{}", "✓ Budget usage reset successfully".green().bold());
    println!("  Name: {}", budget.name);

    Ok(())
}
