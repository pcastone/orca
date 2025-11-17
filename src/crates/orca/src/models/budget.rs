use serde::{Deserialize, Serialize};

/// Budget type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetType {
    #[serde(rename = "recurring")]
    Recurring,
    #[serde(rename = "credit")]
    Credit,
}

impl BudgetType {
    pub fn as_str(&self) -> &str {
        match self {
            BudgetType::Recurring => "recurring",
            BudgetType::Credit => "credit",
        }
    }
}

/// Renewal interval for recurring budgets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenewalInterval {
    #[serde(rename = "days")]
    Days,
    #[serde(rename = "weeks")]
    Weeks,
    #[serde(rename = "months")]
    Months,
}

impl RenewalInterval {
    pub fn as_str(&self) -> &str {
        match self {
            RenewalInterval::Days => "days",
            RenewalInterval::Weeks => "weeks",
            RenewalInterval::Months => "months",
        }
    }

    /// Convert to days for calculation
    pub fn to_days(&self, interval_value: u32) -> u32 {
        match self {
            RenewalInterval::Days => interval_value,
            RenewalInterval::Weeks => interval_value * 7,
            RenewalInterval::Months => interval_value * 30, // Approximate
        }
    }
}

/// Budget enforcement strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetEnforcement {
    #[serde(rename = "block")]
    Block,
    #[serde(rename = "warn")]
    Warn,
}

impl BudgetEnforcement {
    pub fn as_str(&self) -> &str {
        match self {
            BudgetEnforcement::Block => "block",
            BudgetEnforcement::Warn => "warn",
        }
    }
}

/// Budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: String,
    pub name: String,
    pub budget_type: BudgetType,

    // Recurring budget fields
    pub renewal_interval_unit: Option<RenewalInterval>,
    pub renewal_interval_value: Option<u32>,
    pub last_renewal_date: Option<i64>,
    pub next_renewal_date: Option<i64>,

    // Credit budget fields
    pub credit_amount: Option<f64>,
    pub credit_cap: Option<f64>,

    // Tracking
    pub current_usage: f64,
    pub total_spent: f64,
    pub enforcement: BudgetEnforcement,
    pub active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Budget {
    /// Create a new recurring budget
    pub fn new_recurring(
        id: String,
        name: String,
        interval_unit: RenewalInterval,
        interval_value: u32,
        enforcement: BudgetEnforcement,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            name,
            budget_type: BudgetType::Recurring,
            renewal_interval_unit: Some(interval_unit),
            renewal_interval_value: Some(interval_value),
            last_renewal_date: Some(now),
            next_renewal_date: Some(now + (interval_unit.to_days(interval_value) as i64 * 86400)),
            credit_amount: None,
            credit_cap: None,
            current_usage: 0.0,
            total_spent: 0.0,
            enforcement,
            active: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new credit budget
    pub fn new_credit(
        id: String,
        name: String,
        amount: f64,
        cap: Option<f64>,
        enforcement: BudgetEnforcement,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            name,
            budget_type: BudgetType::Credit,
            renewal_interval_unit: None,
            renewal_interval_value: None,
            last_renewal_date: None,
            next_renewal_date: None,
            credit_amount: Some(amount),
            credit_cap: cap,
            current_usage: 0.0,
            total_spent: 0.0,
            enforcement,
            active: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get remaining budget
    pub fn remaining(&self) -> f64 {
        match self.budget_type {
            BudgetType::Credit => {
                self.credit_amount.unwrap_or(0.0) - self.current_usage
            }
            BudgetType::Recurring => {
                // For recurring, remaining is based on the interval amount
                // This would need additional config field to store per-interval limit
                0.0
            }
        }
    }

    /// Get usage percentage (0-100)
    pub fn usage_percentage(&self) -> f64 {
        match self.budget_type {
            BudgetType::Credit => {
                let amount = self.credit_amount.unwrap_or(0.0);
                if amount <= 0.0 {
                    0.0
                } else {
                    (self.current_usage / amount) * 100.0
                }
            }
            BudgetType::Recurring => {
                // Recurring budgets don't have a direct percentage without interval amount
                0.0
            }
        }
    }

    /// Check if budget is exceeded
    pub fn is_exceeded(&self) -> bool {
        match self.budget_type {
            BudgetType::Credit => {
                self.current_usage > self.credit_amount.unwrap_or(0.0)
            }
            BudgetType::Recurring => {
                // Check against cap if set
                if let Some(cap) = self.credit_cap {
                    self.total_spent > cap
                } else {
                    false
                }
            }
        }
    }

    /// Check if budget should warn
    pub fn should_warn(&self) -> bool {
        self.usage_percentage() >= 80.0
    }
}
