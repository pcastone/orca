//! Integration tests for pricing system

mod common;

use orca::services::PricingService;

#[tokio::test]
async fn test_pricing_initialization() {
    let (_temp, db) = common::setup_test_db().await;
    let service = PricingService::new(db);

    let result = service.initialize_pricing().await;

    assert!(result.is_ok(), "Failed to initialize pricing");
}

#[tokio::test]
async fn test_pricing_get_anthropic() {
    let (_temp, db) = common::setup_test_db().await;
    let service = PricingService::new(db);

    service
        .initialize_pricing()
        .await
        .expect("Failed to initialize");

    let pricing = service
        .get_pricing("anthropic", "claude-3-sonnet")
        .await
        .expect("Failed to get pricing");

    assert_eq!(pricing.provider, "anthropic");
    assert_eq!(pricing.model, "claude-3-sonnet");
    assert!(pricing.cost_per_input_token > 0.0);
    assert!(pricing.cost_per_output_token > 0.0);
}

#[tokio::test]
async fn test_pricing_get_openai() {
    let (_temp, db) = common::setup_test_db().await;
    let service = PricingService::new(db);

    service
        .initialize_pricing()
        .await
        .expect("Failed to initialize");

    let pricing = service
        .get_pricing("openai", "gpt-4")
        .await
        .expect("Failed to get pricing");

    assert_eq!(pricing.provider, "openai");
    assert_eq!(pricing.model, "gpt-4");
}

#[tokio::test]
async fn test_pricing_unknown_model_defaults_to_free() {
    let (_temp, db) = common::setup_test_db().await;
    let service = PricingService::new(db);

    service
        .initialize_pricing()
        .await
        .expect("Failed to initialize");

    let pricing = service
        .get_pricing("unknown_provider", "unknown_model")
        .await
        .expect("Failed to get pricing");

    // Unknown models should return free pricing
    assert_eq!(pricing.cost_per_input_token, 0.0);
    assert_eq!(pricing.cost_per_output_token, 0.0);
}

#[tokio::test]
async fn test_pricing_calculate_cost_basic() {
    let (_temp, db) = common::setup_test_db().await;
    let service = PricingService::new(db);

    service
        .initialize_pricing()
        .await
        .expect("Failed to initialize");

    // Calculate cost for 1000 input tokens, 500 output tokens
    let cost = service
        .calculate_cost("anthropic", "claude-3-sonnet", 1000, 500, None)
        .await
        .expect("Failed to calculate cost");

    // Cost should be positive
    assert!(cost > 0.0);
}

#[tokio::test]
async fn test_pricing_calculate_cost_with_reasoning() {
    let (_temp, db) = common::setup_test_db().await;
    let service = PricingService::new(db);

    service
        .initialize_pricing()
        .await
        .expect("Failed to initialize");

    // Calculate cost with reasoning tokens (for o1, R1, etc.)
    let cost = service
        .calculate_cost("openai", "gpt-4", 1000, 500, Some(100))
        .await
        .expect("Failed to calculate cost");

    // Cost should be positive
    assert!(cost > 0.0);
}

#[tokio::test]
async fn test_pricing_list_all() {
    let (_temp, db) = common::setup_test_db().await;
    let service = PricingService::new(db);

    service
        .initialize_pricing()
        .await
        .expect("Failed to initialize");

    let all_pricing = service
        .list_all_pricing()
        .await
        .expect("Failed to list pricing");

    assert!(!all_pricing.is_empty());
    assert!(all_pricing.iter().any(|p| p.provider == "anthropic"));
    assert!(all_pricing.iter().any(|p| p.provider == "openai"));
}

#[tokio::test]
async fn test_pricing_update() {
    let (_temp, db) = common::setup_test_db().await;
    let service = PricingService::new(db);

    service
        .initialize_pricing()
        .await
        .expect("Failed to initialize");

    // Update pricing for a model
    service
        .update_pricing("anthropic", "claude-3-sonnet", 0.001, 0.002, None)
        .await
        .expect("Failed to update pricing");

    // Verify update
    let updated = service
        .get_pricing("anthropic", "claude-3-sonnet")
        .await
        .expect("Failed to get pricing");

    assert_eq!(updated.cost_per_input_token, 0.001);
    assert_eq!(updated.cost_per_output_token, 0.002);
}

#[tokio::test]
async fn test_pricing_zero_tokens_zero_cost() {
    let (_temp, db) = common::setup_test_db().await;
    let service = PricingService::new(db);

    service
        .initialize_pricing()
        .await
        .expect("Failed to initialize");

    let cost = service
        .calculate_cost("anthropic", "claude-3-sonnet", 0, 0, None)
        .await
        .expect("Failed to calculate cost");

    assert_eq!(cost, 0.0);
}
