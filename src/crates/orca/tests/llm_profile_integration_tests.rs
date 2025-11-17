//! Integration tests for LLM profile system

mod common;

use orca::models::LlmProfile;
use orca::repositories::LlmProfileRepository;

#[tokio::test]
async fn test_llm_profile_create_and_retrieve() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = LlmProfileRepository::new(db);

    let profile = LlmProfile {
        id: "profile-1".to_string(),
        name: "Multi LLM".to_string(),
        planner_provider: "anthropic".to_string(),
        planner_model: "claude-3-sonnet".to_string(),
        worker_provider: "openai".to_string(),
        worker_model: "gpt-4".to_string(),
        description: Some("Planner for analysis, worker for execution".to_string()),
        active: false,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    repo.create(profile.clone())
        .await
        .expect("Failed to create profile");

    let retrieved = repo
        .get_by_id(&profile.id)
        .await
        .expect("Failed to retrieve")
        .expect("Profile not found");

    assert_eq!(retrieved.name, "Multi LLM");
    assert_eq!(retrieved.planner_provider, "anthropic");
    assert_eq!(retrieved.worker_model, "gpt-4");
    assert_eq!(retrieved.description, Some("Planner for analysis, worker for execution".to_string()));
}

#[tokio::test]
async fn test_llm_profile_get_by_name() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = LlmProfileRepository::new(db);

    let profile = LlmProfile {
        id: "profile-1".to_string(),
        name: "FastExecution".to_string(),
        planner_provider: "openai".to_string(),
        planner_model: "gpt-4-turbo".to_string(),
        worker_provider: "openai".to_string(),
        worker_model: "gpt-4".to_string(),
        description: None,
        active: false,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    repo.create(profile.clone())
        .await
        .expect("Failed to create profile");

    let found = repo
        .get_by_name("FastExecution")
        .await
        .expect("Failed to get by name")
        .expect("Profile not found");

    assert_eq!(found.id, profile.id);
    assert_eq!(found.planner_provider, "openai");
}

#[tokio::test]
async fn test_llm_profile_list_all() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = LlmProfileRepository::new(db);

    let profile1 = LlmProfile {
        id: "p1".to_string(),
        name: "Profile 1".to_string(),
        planner_provider: "anthropic".to_string(),
        planner_model: "claude-3-sonnet".to_string(),
        worker_provider: "openai".to_string(),
        worker_model: "gpt-4".to_string(),
        description: None,
        active: false,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    let profile2 = LlmProfile {
        id: "p2".to_string(),
        name: "Profile 2".to_string(),
        planner_provider: "openai".to_string(),
        planner_model: "gpt-4".to_string(),
        worker_provider: "anthropic".to_string(),
        worker_model: "claude-3-opus".to_string(),
        description: None,
        active: false,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    repo.create(profile1).await.expect("Failed to create p1");
    repo.create(profile2).await.expect("Failed to create p2");

    let profiles = repo.list_all().await.expect("Failed to list");

    assert_eq!(profiles.len(), 2);
    assert!(profiles.iter().any(|p| p.name == "Profile 1"));
    assert!(profiles.iter().any(|p| p.name == "Profile 2"));
}

#[tokio::test]
async fn test_llm_profile_update() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = LlmProfileRepository::new(db);

    let mut profile = LlmProfile {
        id: "p1".to_string(),
        name: "Original".to_string(),
        planner_provider: "anthropic".to_string(),
        planner_model: "claude-3-sonnet".to_string(),
        worker_provider: "openai".to_string(),
        worker_model: "gpt-4".to_string(),
        description: None,
        active: false,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    repo.create(profile.clone())
        .await
        .expect("Failed to create");

    profile.planner_model = "claude-3-opus".to_string();
    profile.description = Some("Updated description".to_string());
    profile.updated_at = chrono::Utc::now().timestamp();

    repo.update(&profile)
        .await
        .expect("Failed to update");

    let updated = repo
        .get_by_id(&profile.id)
        .await
        .expect("Failed to retrieve")
        .expect("Not found");

    assert_eq!(updated.planner_model, "claude-3-opus");
    assert_eq!(updated.description, Some("Updated description".to_string()));
}

#[tokio::test]
async fn test_llm_profile_delete() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = LlmProfileRepository::new(db);

    let profile = LlmProfile {
        id: "p1".to_string(),
        name: "ToDelete".to_string(),
        planner_provider: "anthropic".to_string(),
        planner_model: "claude-3-sonnet".to_string(),
        worker_provider: "openai".to_string(),
        worker_model: "gpt-4".to_string(),
        description: None,
        active: false,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    repo.create(profile.clone())
        .await
        .expect("Failed to create");

    repo.delete(&profile.id)
        .await
        .expect("Failed to delete");

    let found = repo
        .get_by_id(&profile.id)
        .await
        .expect("Failed to check");

    assert!(found.is_none());
}

#[tokio::test]
async fn test_llm_profile_activate() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = LlmProfileRepository::new(db);

    let profile = LlmProfile {
        id: "p1".to_string(),
        name: "ToActivate".to_string(),
        planner_provider: "anthropic".to_string(),
        planner_model: "claude-3-sonnet".to_string(),
        worker_provider: "openai".to_string(),
        worker_model: "gpt-4".to_string(),
        description: None,
        active: false,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    repo.create(profile.clone())
        .await
        .expect("Failed to create");

    repo.activate(&profile.id)
        .await
        .expect("Failed to activate");

    let active = repo
        .get_active()
        .await
        .expect("Failed to get active")
        .expect("No active profile");

    assert_eq!(active.id, profile.id);
    assert!(active.active);
}

#[tokio::test]
async fn test_llm_profile_with_description() {
    let (_temp, db) = common::setup_test_db().await;
    let repo = LlmProfileRepository::new(db);

    let profile = LlmProfile {
        id: "p1".to_string(),
        name: "Documented".to_string(),
        planner_provider: "anthropic".to_string(),
        planner_model: "claude-3-sonnet".to_string(),
        worker_provider: "openai".to_string(),
        worker_model: "gpt-4".to_string(),
        description: Some("Fast planner with powerful worker for complex tasks".to_string()),
        active: false,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    repo.create(profile.clone())
        .await
        .expect("Failed to create");

    let retrieved = repo
        .get_by_id(&profile.id)
        .await
        .expect("Failed to retrieve")
        .expect("Not found");

    assert_eq!(
        retrieved.description,
        Some("Fast planner with powerful worker for complex tasks".to_string())
    );
}
