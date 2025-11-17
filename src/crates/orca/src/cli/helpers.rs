//! Helper functions for CLI budget and LLM profile loading

use crate::error::Result;
use crate::models::Budget;
use crate::models::LlmProfile;
use crate::repositories::{BudgetRepository, LlmProfileRepository};
use crate::DatabaseManager;
use std::sync::Arc;

/// Load the active budget from database
///
/// Returns the active budget if one is configured and active, otherwise None
pub async fn load_active_budget(db_manager: Arc<DatabaseManager>) -> Result<Option<Budget>> {
    let user_db = db_manager.user_db().clone();
    let repo = BudgetRepository::new(user_db);
    repo.get_active().await
}

/// Load the active LLM profile from database
///
/// Returns the active profile if one is configured and active, otherwise None
pub async fn load_active_llm_profile(db_manager: Arc<DatabaseManager>) -> Result<Option<LlmProfile>> {
    let user_db = db_manager.user_db().clone();
    let repo = LlmProfileRepository::new(user_db);
    repo.get_active().await
}

/// Load a budget by name
pub async fn load_budget_by_name(db_manager: Arc<DatabaseManager>, name: &str) -> Result<Option<Budget>> {
    let user_db = db_manager.user_db().clone();
    let repo = BudgetRepository::new(user_db);
    repo.get_by_name(name).await
}

/// Load an LLM profile by name
pub async fn load_llm_profile_by_name(db_manager: Arc<DatabaseManager>, name: &str) -> Result<Option<LlmProfile>> {
    let user_db = db_manager.user_db().clone();
    let repo = LlmProfileRepository::new(user_db);
    repo.get_by_name(name).await
}

/// List all available budgets
pub async fn list_all_budgets(db_manager: Arc<DatabaseManager>) -> Result<Vec<Budget>> {
    let user_db = db_manager.user_db().clone();
    let repo = BudgetRepository::new(user_db);
    repo.list_all().await
}

/// List all available LLM profiles
pub async fn list_all_llm_profiles(db_manager: Arc<DatabaseManager>) -> Result<Vec<LlmProfile>> {
    let user_db = db_manager.user_db().clone();
    let repo = LlmProfileRepository::new(user_db);
    repo.list_all().await
}
