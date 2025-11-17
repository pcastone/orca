//! Budget tracking utilities for LLM calls

use crate::error::Result;
use crate::services::{BudgetService, PricingService};
use langgraph_core::llm::ChatResponse;
use std::sync::Arc;
use tracing::{debug, warn};

/// Budget tracker for LLM calls
pub struct BudgetTracker {
    /// Budget ID to track against
    budget_id: String,
    /// LLM provider name (for pricing lookup)
    provider: String,
    /// LLM model name (for pricing lookup)
    model: String,
    /// Budget service for enforcing limits
    budget_service: Arc<BudgetService>,
    /// Pricing service for cost calculation
    pricing_service: Arc<PricingService>,
}

impl BudgetTracker {
    /// Create a new budget tracker
    pub fn new(
        budget_id: String,
        provider: String,
        model: String,
        budget_service: Arc<BudgetService>,
        pricing_service: Arc<PricingService>,
    ) -> Self {
        Self {
            budget_id,
            provider,
            model,
            budget_service,
            pricing_service,
        }
    }

    /// Check if request is allowed based on budget
    pub async fn check_budget_allowed(&self) -> Result<bool> {
        self.budget_service.should_allow_request(&self.budget_id).await
    }

    /// Track usage from a response
    pub async fn track_response(&self, response: &ChatResponse) -> Result<f64> {
        let mut total_cost = 0.0;

        // Track usage and cost
        if let Some(usage) = &response.usage {
            let cost = self
                .pricing_service
                .calculate_cost(
                    &self.provider,
                    &self.model,
                    usage.input_tokens,
                    usage.output_tokens,
                    usage.reasoning_tokens,
                )
                .await?;

            debug!(
                "LLM usage - input: {}, output: {}, cost: ${:.6}",
                usage.input_tokens, usage.output_tokens, cost
            );

            // Track the cost against the budget
            self.budget_service.track_cost(&self.budget_id, cost).await?;
            total_cost = cost;
        } else {
            warn!("LLM response did not include usage information");
        }

        Ok(total_cost)
    }
}

/// Wrapper for budget-tracked LLM calls
pub struct BudgetTrackedLlm {
    tracker: BudgetTracker,
}

impl BudgetTrackedLlm {
    /// Create a new budget-tracked LLM wrapper
    pub fn new(
        budget_id: String,
        provider: String,
        model: String,
        budget_service: Arc<BudgetService>,
        pricing_service: Arc<PricingService>,
    ) -> Self {
        let tracker = BudgetTracker::new(
            budget_id,
            provider,
            model,
            budget_service,
            pricing_service,
        );

        Self { tracker }
    }

    /// Check if request is allowed based on budget
    pub async fn check_budget_allowed(&self) -> Result<bool> {
        self.tracker.check_budget_allowed().await
    }

    /// Track usage from a response
    pub async fn track_response(&self, response: &ChatResponse) -> Result<f64> {
        self.tracker.track_response(response).await
    }

    /// Track a cost directly (for testing or when usage info is not available)
    pub async fn track_cost(&self, cost: f64) -> Result<()> {
        self.tracker
            .budget_service
            .track_cost(&self.tracker.budget_id, cost)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_budget_tracker_creation() {
        // This is a basic test to ensure the tracker can be created
        // Full integration tests would require mock implementations
        println!("Budget tracker test placeholder");
    }
}
