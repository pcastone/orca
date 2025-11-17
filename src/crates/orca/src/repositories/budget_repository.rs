//! Budget repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::{Budget, BudgetEnforcement, BudgetType, RenewalInterval};
use chrono::Utc;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, FromRow};
use std::sync::Arc;
use uuid::Uuid;

/// Repository for budget database operations
#[derive(Clone, Debug)]
pub struct BudgetRepository {
    db: Arc<Database>,
}

impl BudgetRepository {
    /// Create a new budget repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Create a new budget
    pub async fn create(&self, mut budget: Budget) -> Result<Budget> {
        let now = Utc::now().timestamp();
        budget.id = Uuid::new_v4().to_string();
        budget.created_at = now;
        budget.updated_at = now;

        let enforcement_str = budget.enforcement.as_str();
        let type_str = budget.budget_type.as_str();

        let interval_unit = budget
            .renewal_interval_unit
            .map(|u| u.as_str().to_string());
        let interval_value = budget.renewal_interval_value;
        let credit_amount = budget.credit_amount;
        let credit_cap = budget.credit_cap;

        sqlx::query(
            "INSERT INTO budgets (id, name, type, renewal_interval_unit, renewal_interval_value,
                                 last_renewal_date, next_renewal_date, credit_amount, credit_cap,
                                 current_usage, total_spent, enforcement, active, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&budget.id)
        .bind(&budget.name)
        .bind(type_str)
        .bind(interval_unit)
        .bind(interval_value)
        .bind(budget.last_renewal_date)
        .bind(budget.next_renewal_date)
        .bind(credit_amount)
        .bind(credit_cap)
        .bind(budget.current_usage)
        .bind(budget.total_spent)
        .bind(enforcement_str)
        .bind(budget.active)
        .bind(now)
        .bind(now)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to create budget: {}", e)))?;

        Ok(budget)
    }

    /// Get budget by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Budget>> {
        let row = sqlx::query(
            "SELECT id, name, type, renewal_interval_unit, renewal_interval_value,
                    last_renewal_date, next_renewal_date, credit_amount, credit_cap,
                    current_usage, total_spent, enforcement, active, created_at, updated_at
             FROM budgets WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get budget: {}", e)))?;

        Ok(row.map(|r| self.row_to_budget(r)))
    }

    /// Get budget by name
    pub async fn get_by_name(&self, name: &str) -> Result<Option<Budget>> {
        let row = sqlx::query(
            "SELECT id, name, type, renewal_interval_unit, renewal_interval_value,
                    last_renewal_date, next_renewal_date, credit_amount, credit_cap,
                    current_usage, total_spent, enforcement, active, created_at, updated_at
             FROM budgets WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get budget by name: {}", e)))?;

        Ok(row.map(|r| self.row_to_budget(r)))
    }

    /// List all budgets
    pub async fn list_all(&self) -> Result<Vec<Budget>> {
        let rows = sqlx::query(
            "SELECT id, name, type, renewal_interval_unit, renewal_interval_value,
                    last_renewal_date, next_renewal_date, credit_amount, credit_cap,
                    current_usage, total_spent, enforcement, active, created_at, updated_at
             FROM budgets ORDER BY created_at DESC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list budgets: {}", e)))?;

        Ok(rows.into_iter().map(|r| self.row_to_budget(r)).collect())
    }

    /// Get active budget
    pub async fn get_active(&self) -> Result<Option<Budget>> {
        let row = sqlx::query(
            "SELECT id, name, type, renewal_interval_unit, renewal_interval_value,
                    last_renewal_date, next_renewal_date, credit_amount, credit_cap,
                    current_usage, total_spent, enforcement, active, created_at, updated_at
             FROM budgets WHERE active = 1 LIMIT 1"
        )
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get active budget: {}", e)))?;

        Ok(row.map(|r| self.row_to_budget(r)))
    }

    /// Update budget
    pub async fn update(&self, budget: &Budget) -> Result<()> {
        let now = Utc::now().timestamp();
        let enforcement_str = budget.enforcement.as_str();
        let interval_unit = budget
            .renewal_interval_unit
            .map(|u| u.as_str().to_string());

        sqlx::query(
            "UPDATE budgets SET name = ?, renewal_interval_unit = ?, renewal_interval_value = ?,
                              last_renewal_date = ?, next_renewal_date = ?, credit_amount = ?,
                              credit_cap = ?, current_usage = ?, total_spent = ?,
                              enforcement = ?, active = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(&budget.name)
        .bind(interval_unit)
        .bind(budget.renewal_interval_value)
        .bind(budget.last_renewal_date)
        .bind(budget.next_renewal_date)
        .bind(budget.credit_amount)
        .bind(budget.credit_cap)
        .bind(budget.current_usage)
        .bind(budget.total_spent)
        .bind(enforcement_str)
        .bind(budget.active)
        .bind(now)
        .bind(&budget.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update budget: {}", e)))?;

        Ok(())
    }

    /// Delete budget
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM budgets WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete budget: {}", e)))?;

        Ok(())
    }

    /// Activate a budget and deactivate others
    pub async fn activate(&self, id: &str) -> Result<()> {
        let mut tx = self.db.pool().begin().await
            .map_err(|e| OrcaError::Database(format!("Failed to start transaction: {}", e)))?;

        sqlx::query("UPDATE budgets SET active = 0")
            .execute(&mut *tx)
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to deactivate budgets: {}", e)))?;

        sqlx::query("UPDATE budgets SET active = 1 WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to activate budget: {}", e)))?;

        tx.commit().await
            .map_err(|e| OrcaError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Track usage (increment current_usage and total_spent)
    pub async fn track_usage(&self, id: &str, cost: f64) -> Result<()> {
        sqlx::query(
            "UPDATE budgets SET current_usage = current_usage + ?, total_spent = total_spent + ?
             WHERE id = ?"
        )
        .bind(cost)
        .bind(cost)
        .bind(id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to track usage: {}", e)))?;

        Ok(())
    }

    /// Reset current usage (for recurring budgets)
    pub async fn reset_usage(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE budgets SET current_usage = 0 WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to reset usage: {}", e)))?;

        Ok(())
    }

    // Helper function to convert database row to Budget
    fn row_to_budget(&self, row: sqlx::sqlite::SqliteRow) -> Budget {
        let budget_type_str: String = row.get("type");
        let budget_type = match budget_type_str.as_str() {
            "recurring" => BudgetType::Recurring,
            "credit" => BudgetType::Credit,
            _ => BudgetType::Credit,
        };

        let renewal_interval_unit: Option<String> = row.get("renewal_interval_unit");
        let renewal_interval_unit = renewal_interval_unit.and_then(|s| match s.as_str() {
            "days" => Some(RenewalInterval::Days),
            "weeks" => Some(RenewalInterval::Weeks),
            "months" => Some(RenewalInterval::Months),
            _ => None,
        });

        let enforcement_str: String = row.get("enforcement");
        let enforcement = match enforcement_str.as_str() {
            "block" => BudgetEnforcement::Block,
            "warn" => BudgetEnforcement::Warn,
            _ => BudgetEnforcement::Warn,
        };

        Budget {
            id: row.get("id"),
            name: row.get("name"),
            budget_type,
            renewal_interval_unit,
            renewal_interval_value: row.get("renewal_interval_value"),
            last_renewal_date: row.get("last_renewal_date"),
            next_renewal_date: row.get("next_renewal_date"),
            credit_amount: row.get("credit_amount"),
            credit_cap: row.get("credit_cap"),
            current_usage: row.get("current_usage"),
            total_spent: row.get("total_spent"),
            enforcement,
            active: row.get::<bool, _>("active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
