//! Budget service for managing budgets, enforcing limits, and tracking usage

use crate::error::{OrcaError, Result};
use crate::models::{Budget, BudgetEnforcement, BudgetType, RenewalInterval};
use crate::repositories::BudgetRepository;
use chrono::Utc;
use std::sync::Arc;

/// Service for managing budgets and enforcing budget limits
#[derive(Clone, Debug)]
pub struct BudgetService {
    repository: Arc<BudgetRepository>,
}

impl BudgetService {
    /// Create a new budget service
    pub fn new(repository: Arc<BudgetRepository>) -> Self {
        Self { repository }
    }

    /// Check if budget is exceeded
    pub async fn check_budget(&self, budget_id: &str) -> Result<BudgetStatus> {
        let budget = self
            .repository
            .get_by_id(budget_id)
            .await?
            .ok_or_else(|| OrcaError::NotFound(format!("Budget not found: {}", budget_id)))?;

        Ok(BudgetStatus {
            is_exceeded: budget.is_exceeded(),
            should_warn: budget.should_warn(),
            usage_percentage: budget.usage_percentage(),
            remaining: budget.remaining(),
            enforcement: budget.enforcement,
        })
    }

    /// Track a cost against a budget
    pub async fn track_cost(&self, budget_id: &str, cost: f64) -> Result<()> {
        let budget = self
            .repository
            .get_by_id(budget_id)
            .await?
            .ok_or_else(|| OrcaError::NotFound(format!("Budget not found: {}", budget_id)))?;

        // Check enforcement before tracking
        if budget.enforcement == BudgetEnforcement::Block && budget.is_exceeded() {
            return Err(OrcaError::BudgetExceeded(format!(
                "Budget limit exceeded for: {}",
                budget.name
            )));
        }

        // Track usage
        self.repository.track_usage(budget_id, cost).await?;

        Ok(())
    }

    /// Process renewal for recurring budgets
    pub async fn process_renewal(&self, budget_id: &str) -> Result<()> {
        let mut budget = self
            .repository
            .get_by_id(budget_id)
            .await?
            .ok_or_else(|| OrcaError::NotFound(format!("Budget not found: {}", budget_id)))?;

        if budget.budget_type != BudgetType::Recurring {
            return Ok(());
        }

        let now = Utc::now().timestamp();
        if let Some(next_renewal) = budget.next_renewal_date {
            if now >= next_renewal {
                // Reset usage and calculate next renewal date
                budget.current_usage = 0.0;
                budget.last_renewal_date = Some(now);

                if let (Some(unit), Some(value)) =
                    (budget.renewal_interval_unit, budget.renewal_interval_value)
                {
                    let days = unit.to_days(value) as i64 * 86400;
                    budget.next_renewal_date = Some(now + days);
                }

                self.repository.update(&budget).await?;
            }
        }

        Ok(())
    }

    /// Get all budgets and process renewals
    pub async fn refresh_all_budgets(&self) -> Result<()> {
        let budgets = self.repository.list_all().await?;

        for budget in budgets {
            if budget.budget_type == BudgetType::Recurring {
                let _ = self.process_renewal(&budget.id).await;
            }
        }

        Ok(())
    }

    /// Check if a request should be allowed
    pub async fn should_allow_request(&self, budget_id: &str) -> Result<bool> {
        let budget = self
            .repository
            .get_by_id(budget_id)
            .await?
            .ok_or_else(|| OrcaError::NotFound(format!("Budget not found: {}", budget_id)))?;

        match budget.enforcement {
            BudgetEnforcement::Block => Ok(!budget.is_exceeded()),
            BudgetEnforcement::Warn => Ok(true),
        }
    }
}

/// Budget status information
#[derive(Debug, Clone)]
pub struct BudgetStatus {
    pub is_exceeded: bool,
    pub should_warn: bool,
    pub usage_percentage: f64,
    pub remaining: f64,
    pub enforcement: BudgetEnforcement,
}

impl BudgetStatus {
    /// Get status message
    pub fn message(&self) -> String {
        if self.should_warn {
            format!(
                "Budget usage at {:.1}% - {} remaining",
                self.usage_percentage, self.remaining
            )
        } else if self.is_exceeded {
            "Budget limit exceeded".to_string()
        } else {
            format!(
                "Budget usage at {:.1}% - {} remaining",
                self.usage_percentage, self.remaining
            )
        }
    }
}
