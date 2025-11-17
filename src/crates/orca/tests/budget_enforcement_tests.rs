//! Integration tests for budget enforcement logic

mod common;

use orca::models::{Budget, BudgetEnforcement};
use orca::services::BudgetService;
use std::sync::Arc;

#[tokio::test]
async fn test_budget_status_ok() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = orca::repositories::BudgetRepository::new(db.clone());
    let service = BudgetService::new(std::sync::Arc::new(repo.clone()));

    let budget_input = Budget::new_credit("b1".to_string(), "Test".to_string(), 1000.0, None, BudgetEnforcement::Warn);

    // Create budget
    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    // Track some usage
    repo.track_usage(&budget.id, 500.0)
        .await
        .expect("Failed to track usage");

    // Check status
    let status = service
        .check_budget(&budget.id)
        .await
        .expect("Failed to check budget");

    assert!(!status.is_exceeded);
    assert!(!status.should_warn);
    assert_eq!(status.usage_percentage, 50.0);
}

#[tokio::test]
async fn test_budget_status_warn_threshold() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = orca::repositories::BudgetRepository::new(db.clone());
    let service = BudgetService::new(std::sync::Arc::new(repo.clone()));

    let budget_input = Budget::new_credit("b1".to_string(), "Test".to_string(), 1000.0, None, BudgetEnforcement::Warn);
    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    // Track usage past 80% threshold
    repo.track_usage(&budget.id, 850.0)
        .await
        .expect("Failed to track usage");

    let status = service
        .check_budget(&budget.id)
        .await
        .expect("Failed to check budget");

    assert!(!status.is_exceeded);
    assert!(status.should_warn); // Should warn at 85%
    assert_eq!(status.usage_percentage, 85.0);
}

#[tokio::test]
async fn test_budget_status_exceeded() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = orca::repositories::BudgetRepository::new(db.clone());
    let service = BudgetService::new(std::sync::Arc::new(repo.clone()));

    let budget_input = Budget::new_credit("b1".to_string(), "Test".to_string(), 1000.0, None, BudgetEnforcement::Block);
    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    // Exceed budget
    repo.track_usage(&budget.id, 1500.0)
        .await
        .expect("Failed to track usage");

    let status = service
        .check_budget(&budget.id)
        .await
        .expect("Failed to check budget");

    assert!(status.is_exceeded);
    assert!(status.should_warn);
    assert_eq!(status.usage_percentage, 150.0);
}

#[tokio::test]
async fn test_enforcement_mode_warn() {
    let (_temp, _db) = common::setup_test_db().await;

    let budget = Budget::new_credit("b1".to_string(), "Warn Test".to_string(), 1000.0, None, BudgetEnforcement::Warn);

    assert_eq!(budget.enforcement, BudgetEnforcement::Warn);
}

#[tokio::test]
async fn test_enforcement_mode_block() {
    let (_temp, _db) = common::setup_test_db().await;

    let budget = Budget::new_credit("b1".to_string(), "Block Test".to_string(), 1000.0, None, BudgetEnforcement::Block);

    assert_eq!(budget.enforcement, BudgetEnforcement::Block);
}

#[tokio::test]
async fn test_budget_remaining_calculation() {
    let (_temp, _db) = common::setup_test_db().await;

    let mut budget = Budget::new_credit("b1".to_string(), "Calc".to_string(), 1000.0, None, BudgetEnforcement::Warn);

    budget.current_usage = 250.0;

    let remaining = budget.remaining();

    assert_eq!(remaining, 750.0);
}

#[tokio::test]
async fn test_budget_usage_percentage() {
    let (_temp, _db) = common::setup_test_db().await;

    let mut budget = Budget::new_credit("b1".to_string(), "Perc".to_string(), 500.0, None, BudgetEnforcement::Warn);

    budget.current_usage = 125.0;

    let percentage = budget.usage_percentage();

    assert_eq!(percentage, 25.0);
}

#[tokio::test]
async fn test_allow_request_under_limit() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = orca::repositories::BudgetRepository::new(db.clone());
    let service = BudgetService::new(std::sync::Arc::new(repo.clone()));

    let budget_input = Budget::new_credit("b1".to_string(), "Test".to_string(), 1000.0, None, BudgetEnforcement::Block);
    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    repo.track_usage(&budget.id, 500.0)
        .await
        .expect("Failed to track usage");

    let allow = service
        .should_allow_request(&budget.id)
        .await
        .expect("Failed to check");

    assert!(allow); // 50% usage, should allow
}

#[tokio::test]
async fn test_block_request_over_limit() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = orca::repositories::BudgetRepository::new(db.clone());
    let service = BudgetService::new(std::sync::Arc::new(repo.clone()));

    let budget_input = Budget::new_credit("b1".to_string(), "Test".to_string(), 1000.0, None, BudgetEnforcement::Block);
    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    repo.track_usage(&budget.id, 1200.0)
        .await
        .expect("Failed to track usage");

    let result = service.should_allow_request(&budget.id).await;

    // Should return error or false for block mode
    match result {
        Ok(false) => {} // Expected
        Err(_) => {}    // Also acceptable (error response)
        _ => panic!("Expected block mode to prevent request"),
    }
}
