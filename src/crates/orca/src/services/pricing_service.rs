//! Pricing service for LLM cost calculations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::{LlmPricing, default_pricing};
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// Service for managing LLM pricing and cost calculations
#[derive(Clone, Debug)]
pub struct PricingService {
    db: Arc<Database>,
}

impl PricingService {
    /// Create a new pricing service
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Initialize pricing database with default provider costs
    pub async fn initialize_pricing(&self) -> Result<()> {
        // Get existing pricing count
        let result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM llm_pricing")
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to count pricing: {}", e)))?;

        // If pricing already exists, don't reinitialize
        if result > 0 {
            return Ok(());
        }

        let now = Utc::now().timestamp();
        let default_data = default_pricing();

        for (provider, model, input_cost, output_cost, reasoning_cost) in default_data {
            let id = Uuid::new_v4().to_string();

            sqlx::query(
                "INSERT INTO llm_pricing (id, provider, model, cost_per_input_token,
                                        cost_per_output_token, cost_per_reasoning_token, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&id)
            .bind(provider)
            .bind(model)
            .bind(input_cost)
            .bind(output_cost)
            .bind(reasoning_cost)
            .bind(now)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to insert pricing: {}", e)))?;
        }

        Ok(())
    }

    /// Get pricing for a specific provider and model
    pub async fn get_pricing(&self, provider: &str, model: &str) -> Result<LlmPricing> {
        let row = sqlx::query(
            "SELECT id, provider, model, cost_per_input_token, cost_per_output_token,
                    cost_per_reasoning_token, updated_at
             FROM llm_pricing WHERE provider = ? AND model = ?"
        )
        .bind(provider)
        .bind(model)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to get pricing: {}", e)))?;

        match row {
            Some(r) => Ok(LlmPricing {
                id: r.get("id"),
                provider: r.get("provider"),
                model: r.get("model"),
                cost_per_input_token: r.get("cost_per_input_token"),
                cost_per_output_token: r.get("cost_per_output_token"),
                cost_per_reasoning_token: r.get("cost_per_reasoning_token"),
                updated_at: r.get("updated_at"),
            }),
            None => {
                // Return free pricing if not found
                Ok(LlmPricing {
                    id: "unknown".to_string(),
                    provider: provider.to_string(),
                    model: model.to_string(),
                    cost_per_input_token: 0.0,
                    cost_per_output_token: 0.0,
                    cost_per_reasoning_token: None,
                    updated_at: Utc::now().timestamp(),
                })
            }
        }
    }

    /// Calculate cost for a request
    pub async fn calculate_cost(
        &self,
        provider: &str,
        model: &str,
        input_tokens: usize,
        output_tokens: usize,
        reasoning_tokens: Option<usize>,
    ) -> Result<f64> {
        let pricing = self.get_pricing(provider, model).await?;
        Ok(pricing.calculate_cost(input_tokens, output_tokens, reasoning_tokens))
    }

    /// Update pricing for a model
    pub async fn update_pricing(
        &self,
        provider: &str,
        model: &str,
        cost_per_input_token: f64,
        cost_per_output_token: f64,
        cost_per_reasoning_token: Option<f64>,
    ) -> Result<()> {
        let now = Utc::now().timestamp();

        let existing = sqlx::query_scalar::<_, Option<String>>(
            "SELECT id FROM llm_pricing WHERE provider = ? AND model = ?"
        )
        .bind(provider)
        .bind(model)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to check pricing: {}", e)))?;

        match existing.flatten() {
            Some(_id) => {
                // Update existing
                sqlx::query(
                    "UPDATE llm_pricing SET cost_per_input_token = ?,
                                          cost_per_output_token = ?,
                                          cost_per_reasoning_token = ?,
                                          updated_at = ?
                     WHERE provider = ? AND model = ?"
                )
                .bind(cost_per_input_token)
                .bind(cost_per_output_token)
                .bind(cost_per_reasoning_token)
                .bind(now)
                .bind(provider)
                .bind(model)
                .execute(self.db.pool())
                .await
                .map_err(|e| OrcaError::Database(format!("Failed to update pricing: {}", e)))?;
            }
            None => {
                // Insert new
                let id = Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO llm_pricing (id, provider, model, cost_per_input_token,
                                            cost_per_output_token, cost_per_reasoning_token, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&id)
                .bind(provider)
                .bind(model)
                .bind(cost_per_input_token)
                .bind(cost_per_output_token)
                .bind(cost_per_reasoning_token)
                .bind(now)
                .execute(self.db.pool())
                .await
                .map_err(|e| OrcaError::Database(format!("Failed to insert pricing: {}", e)))?;
            }
        }

        Ok(())
    }

    /// List all pricing
    pub async fn list_all_pricing(&self) -> Result<Vec<LlmPricing>> {
        let rows = sqlx::query(
            "SELECT id, provider, model, cost_per_input_token, cost_per_output_token,
                    cost_per_reasoning_token, updated_at
             FROM llm_pricing ORDER BY provider, model"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list pricing: {}", e)))?;

        let pricing = rows
            .into_iter()
            .map(|r| LlmPricing {
                id: r.get("id"),
                provider: r.get("provider"),
                model: r.get("model"),
                cost_per_input_token: r.get("cost_per_input_token"),
                cost_per_output_token: r.get("cost_per_output_token"),
                cost_per_reasoning_token: r.get("cost_per_reasoning_token"),
                updated_at: r.get("updated_at"),
            })
            .collect();

        Ok(pricing)
    }
}
