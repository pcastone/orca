//! Integration tests for budget system

mod common;

use orca::models::{Budget, BudgetEnforcement, BudgetType, RenewalInterval};
use orca::repositories::BudgetRepository;

#[tokio::test]
async fn test_budget_create_and_retrieve() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = BudgetRepository::new(db);

    // Create a new credit budget
    let budget_input = Budget::new_credit("budget-1".to_string(), "Test Budget".to_string(), 1000.0, None, BudgetEnforcement::Warn);

    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    // Retrieve and verify
    let retrieved = repo
        .get_by_id(&budget.id)
        .await
        .expect("Failed to retrieve budget")
        .expect("Budget not found");

    assert_eq!(retrieved.name, "Test Budget");
    assert_eq!(retrieved.credit_amount, Some(1000.0));
    assert_eq!(retrieved.current_usage, 0.0);
}

#[tokio::test]
async fn test_budget_get_by_name() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = BudgetRepository::new(db);

    let budget_input = Budget::new_credit("budget-1".to_string(), "Api Budget".to_string(), 500.0, None, BudgetEnforcement::Block);

    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    // Retrieve by name
    let found = repo
        .get_by_name("Api Budget")
        .await
        .expect("Failed to get by name")
        .expect("Budget not found");

    assert_eq!(found.id, budget.id);
    assert_eq!(found.credit_amount, Some(500.0));
}

#[tokio::test]
async fn test_budget_list_all() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = BudgetRepository::new(db);

    // Create multiple budgets
    let budget1 = Budget::new_credit("b1".to_string(), "Budget 1".to_string(), 100.0, None, BudgetEnforcement::Warn);
    let budget2 = Budget::new_credit("b2".to_string(), "Budget 2".to_string(), 200.0, None, BudgetEnforcement::Block);

    repo.create(budget1).await.expect("Failed to create budget 1");
    repo.create(budget2).await.expect("Failed to create budget 2");

    // List all
    let budgets = repo.list_all().await.expect("Failed to list budgets");

    assert_eq!(budgets.len(), 2);
    assert!(budgets.iter().any(|b| b.name == "Budget 1"));
    assert!(budgets.iter().any(|b| b.name == "Budget 2"));
}

#[tokio::test]
async fn test_budget_update() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = BudgetRepository::new(db);

    let budget_input = Budget::new_credit("b1".to_string(), "Original".to_string(), 1000.0, None, BudgetEnforcement::Warn);

    let mut budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    // Update
    budget.credit_amount = Some(2000.0);
    budget.enforcement = BudgetEnforcement::Block;

    repo.update(&budget)
        .await
        .expect("Failed to update budget");

    // Verify
    let updated = repo
        .get_by_id(&budget.id)
        .await
        .expect("Failed to retrieve")
        .expect("Not found");

    assert_eq!(updated.credit_amount, Some(2000.0));
    assert_eq!(updated.enforcement, BudgetEnforcement::Block);
}

#[tokio::test]
async fn test_budget_delete() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = BudgetRepository::new(db);

    let budget_input = Budget::new_credit("b1".to_string(), "ToDelete".to_string(), 100.0, None, BudgetEnforcement::Warn);

    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    // Delete
    repo.delete(&budget.id)
        .await
        .expect("Failed to delete budget");

    // Verify deletion
    let found = repo
        .get_by_id(&budget.id)
        .await
        .expect("Failed to check");

    assert!(found.is_none());
}

#[tokio::test]
async fn test_budget_activate() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = BudgetRepository::new(db);

    let budget_input = Budget::new_credit("b1".to_string(), "ToActivate".to_string(), 100.0, None, BudgetEnforcement::Warn);

    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    // Activate
    repo.activate(&budget.id)
        .await
        .expect("Failed to activate");

    // Verify active
    let active = repo
        .get_active()
        .await
        .expect("Failed to get active")
        .expect("No active budget");

    assert_eq!(active.id, budget.id);
    assert!(active.active);
}

#[tokio::test]
async fn test_recurring_budget_creation() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = BudgetRepository::new(db);

    let budget_input = Budget::new_recurring(
        "recurring-1".to_string(),
        "Monthly Budget".to_string(),
        RenewalInterval::Months,
        1,
        BudgetEnforcement::Warn,
    );

    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create recurring budget");

    let retrieved = repo
        .get_by_id(&budget.id)
        .await
        .expect("Failed to retrieve")
        .expect("Not found");

    assert_eq!(retrieved.renewal_interval_unit, Some(RenewalInterval::Months));
    assert_eq!(retrieved.renewal_interval_value, Some(1));
    assert!(matches!(retrieved.budget_type, BudgetType::Recurring));
}

#[tokio::test]
async fn test_budget_track_usage() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = BudgetRepository::new(db);

    let budget_input = Budget::new_credit("b1".to_string(), "Usage Test".to_string(), 100.0, None, BudgetEnforcement::Warn);

    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    // Track usage
    repo.track_usage(&budget.id, 25.5)
        .await
        .expect("Failed to track usage");

    repo.track_usage(&budget.id, 10.25)
        .await
        .expect("Failed to track usage");

    // Verify
    let updated = repo
        .get_by_id(&budget.id)
        .await
        .expect("Failed to retrieve")
        .expect("Not found");

    assert_eq!(updated.current_usage, 35.75);
    assert_eq!(updated.total_spent, 35.75);
}

#[tokio::test]
async fn test_budget_remaining_calculation() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = BudgetRepository::new(db);

    let budget_input = Budget::new_credit("b1".to_string(), "Calc Test".to_string(), 100.0, None, BudgetEnforcement::Warn);

    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    repo.track_usage(&budget.id, 30.0)
        .await
        .expect("Failed to track usage");

    let updated = repo
        .get_by_id(&budget.id)
        .await
        .expect("Failed to retrieve")
        .expect("Not found");

    let remaining = updated.remaining();
    assert_eq!(remaining, 70.0);
}

#[tokio::test]
async fn test_budget_reset_usage() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = BudgetRepository::new(db);

    let budget_input = Budget::new_credit("b1".to_string(), "Reset Test".to_string(), 100.0, None, BudgetEnforcement::Warn);

    let budget = repo.create(budget_input)
        .await
        .expect("Failed to create budget");

    repo.track_usage(&budget.id, 50.0)
        .await
        .expect("Failed to track usage");

    // Reset
    repo.reset_usage(&budget.id)
        .await
        .expect("Failed to reset usage");

    let reset = repo
        .get_by_id(&budget.id)
        .await
        .expect("Failed to retrieve")
        .expect("Not found");

    assert_eq!(reset.current_usage, 0.0);
}
