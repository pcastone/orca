//! Multi-LLM provider for managing planner and worker LLMs

use crate::config::OrcaConfig;
use crate::error::Result;
use crate::models::LlmProfile;
use crate::executor::llm_provider::LlmProvider;
use crate::repositories::BudgetRepository;
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
    /// Create a multi-LLM provider from a profile
    pub fn from_profile(profile: &LlmProfile, base_config: &OrcaConfig) -> Result<Self> {
        // Create planner config
        let mut planner_config = base_config.clone();
        planner_config.llm.provider = profile.planner_provider.clone();
        planner_config.llm.model = profile.planner_model.clone();

        // Create worker config
        let mut worker_config = base_config.clone();
        worker_config.llm.provider = profile.worker_provider.clone();
        worker_config.llm.model = profile.worker_model.clone();

        debug!(
            "Creating multi-LLM provider: planner={}:{}, worker={}:{}",
            profile.planner_provider, profile.planner_model,
            profile.worker_provider, profile.worker_model
        );

        let planner = LlmProvider::from_config(&planner_config)?;
        let worker = LlmProvider::from_config(&worker_config)?;

        Ok(Self { planner, worker })
    }

    /// Create a multi-LLM provider from CLI flags
    pub fn from_cli_flags(
        planner_provider: &str,
        planner_model: &str,
        worker_provider: &str,
        worker_model: &str,
        base_config: &OrcaConfig,
    ) -> Result<Self> {
        // Create planner config
        let mut planner_config = base_config.clone();
        planner_config.llm.provider = planner_provider.to_string();
        planner_config.llm.model = planner_model.to_string();

        // Create worker config
        let mut worker_config = base_config.clone();
        worker_config.llm.provider = worker_provider.to_string();
        worker_config.llm.model = worker_model.to_string();

        debug!(
            "Creating multi-LLM provider from CLI: planner={}:{}, worker={}:{}",
            planner_provider, planner_model, worker_provider, worker_model
        );

        let planner = LlmProvider::from_config(&planner_config)?;
        let worker = LlmProvider::from_config(&worker_config)?;

        Ok(Self { planner, worker })
    }

    /// Use default config (same LLM for both planner and worker)
    pub fn from_default_config(config: &OrcaConfig) -> Result<Self> {
        let planner = LlmProvider::from_config(config)?;
        let worker = LlmProvider::from_config(config)?;

        debug!(
            "Using default LLM for both planner and worker: {}:{}",
            config.llm.provider, config.llm.model
        );

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
    /// Create a budget-tracked multi-LLM provider from a profile
    pub fn from_profile(
        profile: &LlmProfile,
        _base_config: &OrcaConfig,
        budget_id: String,
        budget_service: Arc<BudgetService>,
        pricing_service: Arc<PricingService>,
    ) -> Result<Self> {
        use crate::executor::budget_tracked_llm::BudgetTrackedLlm;

        debug!(
            "Creating budget-tracked multi-LLM provider from profile: {}",
            profile.name
        );

        let planner = BudgetTrackedLlm::new(
            budget_id.clone(),
            profile.planner_provider.clone(),
            profile.planner_model.clone(),
            budget_service.clone(),
            pricing_service.clone(),
        );

        let worker = BudgetTrackedLlm::new(
            budget_id,
            profile.worker_provider.clone(),
            profile.worker_model.clone(),
            budget_service,
            pricing_service,
        );

        Ok(Self { planner, worker })
    }

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
