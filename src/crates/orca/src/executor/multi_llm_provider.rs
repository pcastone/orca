//! Multi-LLM provider for managing planner and worker LLMs
//!
//! LLM configuration is now database-only (llm_profiles table).
//! Use `from_params` to create providers with explicit parameters.

use crate::error::Result;
use crate::executor::llm_provider::LlmProvider;
use crate::services::{BudgetService, PricingService};
use std::sync::Arc;
use tracing::debug;

/// Multi-LLM configuration with separate planner and worker LLMs
pub struct MultiLlmProvider {
    /// Planner LLM (for planning/decomposition)
    pub planner: LlmProvider,
    /// Worker LLM (for execution)
    pub worker: LlmProvider,
}

impl MultiLlmProvider {
    /// Create a multi-LLM provider from explicit parameters
    ///
    /// LLM configuration is now database-only. Callers should load
    /// configuration from the llm_profiles table.
    pub fn from_params(
        planner_provider: &str,
        planner_model: &str,
        planner_api_key: Option<&str>,
        worker_provider: &str,
        worker_model: &str,
        worker_api_key: Option<&str>,
    ) -> Result<Self> {
        debug!(
            "Creating multi-LLM provider: planner={}:{}, worker={}:{}",
            planner_provider, planner_model, worker_provider, worker_model
        );

        let planner = LlmProvider::from_params(planner_provider, planner_model, planner_api_key, None)?;
        let worker = LlmProvider::from_params(worker_provider, worker_model, worker_api_key, None)?;

        Ok(Self { planner, worker })
    }

    /// Create a multi-LLM provider from two LlmProviderConfigs
    pub fn from_provider_configs(
        planner_config: &crate::models::LlmProviderConfig,
        worker_config: &crate::models::LlmProviderConfig,
    ) -> Result<Self> {
        debug!(
            "Creating multi-LLM provider from configs: planner={}:{}, worker={}:{}",
            planner_config.provider_type, planner_config.model,
            worker_config.provider_type, worker_config.model
        );

        let planner = LlmProvider::from_provider_config(planner_config)?;
        let worker = LlmProvider::from_provider_config(worker_config)?;

        Ok(Self { planner, worker })
    }

    /// Create with the same LLM for both planner and worker
    pub fn from_single_provider(provider: &str, model: &str, api_key: Option<&str>) -> Result<Self> {
        debug!(
            "Using same LLM for both planner and worker: {}:{}",
            provider, model
        );

        let planner = LlmProvider::from_params(provider, model, api_key, None)?;
        let worker = LlmProvider::from_params(provider, model, api_key, None)?;

        Ok(Self { planner, worker })
    }
}

/// Multi-LLM provider with budget tracking
pub struct BudgetTrackedMultiLlmProvider {
    /// Planner LLM with budget tracking
    pub planner: crate::executor::budget_tracked_llm::BudgetTrackedLlm,
    /// Worker LLM with budget tracking
    pub worker: crate::executor::budget_tracked_llm::BudgetTrackedLlm,
}

impl BudgetTrackedMultiLlmProvider {
    /// Create a budget-tracked multi-LLM provider from CLI flags
    pub fn from_cli_flags(
        planner_provider: &str,
        planner_model: &str,
        worker_provider: &str,
        worker_model: &str,
        budget_id: String,
        budget_service: Arc<BudgetService>,
        pricing_service: Arc<PricingService>,
    ) -> Result<Self> {
        use crate::executor::budget_tracked_llm::BudgetTrackedLlm;

        debug!(
            "Creating budget-tracked multi-LLM provider from CLI flags: planner={}:{}, worker={}:{}",
            planner_provider, planner_model, worker_provider, worker_model
        );

        let planner = BudgetTrackedLlm::new(
            budget_id.clone(),
            planner_provider.to_string(),
            planner_model.to_string(),
            budget_service.clone(),
            pricing_service.clone(),
        );

        let worker = BudgetTrackedLlm::new(
            budget_id,
            worker_provider.to_string(),
            worker_model.to_string(),
            budget_service,
            pricing_service,
        );

        Ok(Self { planner, worker })
    }
}
