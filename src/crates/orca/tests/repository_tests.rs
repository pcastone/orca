//! Integration tests for repository operations
//!
//! Tests the new dual-database repository implementations:
//! - BugRepository
//! - ProjectRuleRepository
//! - ToolPermissionRepository
//! - AstCacheRepository

use orca::db::Database;
use orca::models::{Bug, BugPriority, BugStatus, ProjectRule};
use orca::repositories::{BugRepository, ProjectRuleRepository};
use std::sync::Arc;
use tempfile::NamedTempFile;

/// Helper to create a temporary test database
async fn create_test_db() -> (Arc<Database>, NamedTempFile) {
    // Create a named temporary file for the database
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    // Create database connection
    let db = Database::new(db_path).await.unwrap();

    (Arc::new(db), temp_file)
}

/// Helper to run project database migrations
async fn run_project_migrations(db: &Arc<Database>) {
    // Read and execute project schema
    let schema_sql = include_str!("../migrations/project/20250115000001_project_schema.sql");

    // Parse and execute SQL statements
    // Split by newlines and reconstruct statements
    let mut current_statement = String::new();
    for line in schema_sql.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }

        current_statement.push_str(line);
        current_statement.push('\n');

        // If the line ends with semicolon, execute the statement
        if trimmed.ends_with(';') {
            let stmt = current_statement.trim();
            if !stmt.is_empty() {
                sqlx::query(stmt)
                    .execute(db.pool())
                    .await
                    .unwrap();
            }
            current_statement.clear();
        }
    }

    // Execute any remaining statement
    if !current_statement.trim().is_empty() {
        sqlx::query(current_statement.trim())
            .execute(db.pool())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn test_bug_repository_crud() {
    let (db, _temp) = create_test_db().await;
    run_project_migrations(&db).await;

    let repo = BugRepository::new(db.clone());

    // Create
    let bug = Bug::new("Test bug".to_string())
        .with_description("This is a test bug".to_string())
        .with_priority(BugPriority::High)
        .with_assignee("test-user".to_string());

    let bug_id = bug.id.clone();
    repo.save(&bug).await.unwrap();

    // Read
    let loaded_bug = repo.find_by_id(&bug_id).await.unwrap();
    assert_eq!(loaded_bug.title, "Test bug");
    assert_eq!(loaded_bug.description, Some("This is a test bug".to_string()));
    assert_eq!(loaded_bug.priority, 2); // High = 2
    assert_eq!(loaded_bug.assignee, Some("test-user".to_string()));
    assert_eq!(loaded_bug.status, BugStatus::Open.as_str());

    // Update
    let mut updated_bug = loaded_bug;
    updated_bug.start_work();
    repo.update(&updated_bug).await.unwrap();

    let loaded_bug = repo.find_by_id(&bug_id).await.unwrap();
    assert_eq!(loaded_bug.status, BugStatus::InProgress.as_str());

    // List
    let all_bugs = repo.list().await.unwrap();
    assert_eq!(all_bugs.len(), 1);

    // Delete
    repo.delete(&bug_id).await.unwrap();

    // Verify deletion
    let result = repo.find_by_id(&bug_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_bug_repository_list_by_status() {
    let (db, _temp) = create_test_db().await;
    run_project_migrations(&db).await;

    let repo = BugRepository::new(db.clone());

    // Create bugs with different statuses
    let mut bug1 = Bug::new("Bug 1".to_string());
    bug1.start_work();
    repo.save(&bug1).await.unwrap();

    let bug2 = Bug::new("Bug 2".to_string());
    repo.save(&bug2).await.unwrap();

    let mut bug3 = Bug::new("Bug 3".to_string());
    bug3.mark_fixed();
    repo.save(&bug3).await.unwrap();

    // List by status
    let open_bugs = repo.list_by_status(BugStatus::Open.as_str()).await.unwrap();
    assert_eq!(open_bugs.len(), 1);
    assert_eq!(open_bugs[0].title, "Bug 2");

    let in_progress_bugs = repo.list_by_status(BugStatus::InProgress.as_str()).await.unwrap();
    assert_eq!(in_progress_bugs.len(), 1);
    assert_eq!(in_progress_bugs[0].title, "Bug 1");

    let fixed_bugs = repo.list_by_status(BugStatus::Fixed.as_str()).await.unwrap();
    assert_eq!(fixed_bugs.len(), 1);
    assert_eq!(fixed_bugs[0].title, "Bug 3");
}

#[tokio::test]
async fn test_bug_repository_list_by_assignee() {
    let (db, _temp) = create_test_db().await;
    run_project_migrations(&db).await;

    let repo = BugRepository::new(db.clone());

    // Create bugs with different assignees
    let bug1 = Bug::new("Bug 1".to_string())
        .with_assignee("alice".to_string());
    repo.save(&bug1).await.unwrap();

    let bug2 = Bug::new("Bug 2".to_string())
        .with_assignee("bob".to_string());
    repo.save(&bug2).await.unwrap();

    let bug3 = Bug::new("Bug 3".to_string())
        .with_assignee("alice".to_string());
    repo.save(&bug3).await.unwrap();

    // List by assignee
    let alice_bugs = repo.list_by_assignee("alice").await.unwrap();
    assert_eq!(alice_bugs.len(), 2);

    let bob_bugs = repo.list_by_assignee("bob").await.unwrap();
    assert_eq!(bob_bugs.len(), 1);
}

#[tokio::test]
async fn test_project_rule_repository_crud() {
    let (db, _temp) = create_test_db().await;
    run_project_migrations(&db).await;

    let repo = ProjectRuleRepository::new(db.clone());

    // Create
    let rule = ProjectRule::new(
        "No console.log".to_string(),
        "style".to_string(),
        r#"{"pattern": "console\\.log", "message": "Use logger instead"}"#.to_string(),
    )
    .with_description("Disallow console.log statements".to_string())
    .with_severity("error");

    let rule_id = rule.id.clone();
    repo.save(&rule).await.unwrap();

    // Read
    let loaded_rule = repo.find_by_id(&rule_id).await.unwrap();
    assert_eq!(loaded_rule.name, "No console.log");
    assert_eq!(loaded_rule.rule_type, "style");
    assert_eq!(loaded_rule.severity, "error");
    assert!(loaded_rule.enabled);
    assert_eq!(
        loaded_rule.description,
        Some("Disallow console.log statements".to_string())
    );

    // Update
    let mut updated_rule = loaded_rule;
    updated_rule.severity = "warning".to_string();
    repo.update(&updated_rule).await.unwrap();

    let loaded_rule = repo.find_by_id(&rule_id).await.unwrap();
    assert_eq!(loaded_rule.severity, "warning");

    // List
    let all_rules = repo.list().await.unwrap();
    assert_eq!(all_rules.len(), 1);

    // Delete
    repo.delete(&rule_id).await.unwrap();

    // Verify deletion
    let result = repo.find_by_id(&rule_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_project_rule_repository_list_by_type() {
    let (db, _temp) = create_test_db().await;
    run_project_migrations(&db).await;

    let repo = ProjectRuleRepository::new(db.clone());

    // Create rules with different types
    let rule1 = ProjectRule::new(
        "Style rule".to_string(),
        "style".to_string(),
        "{}".to_string(),
    );
    repo.save(&rule1).await.unwrap();

    let rule2 = ProjectRule::new(
        "Security rule 1".to_string(),
        "security".to_string(),
        "{}".to_string(),
    );
    repo.save(&rule2).await.unwrap();

    let rule3 = ProjectRule::new(
        "Security rule 2".to_string(),
        "security".to_string(),
        "{}".to_string(),
    );
    repo.save(&rule3).await.unwrap();

    let rule4 = ProjectRule::new(
        "Workflow rule".to_string(),
        "workflow".to_string(),
        "{}".to_string(),
    );
    repo.save(&rule4).await.unwrap();

    // List by type
    let style_rules = repo.list_by_type("style").await.unwrap();
    assert_eq!(style_rules.len(), 1);

    let security_rules = repo.list_by_type("security").await.unwrap();
    assert_eq!(security_rules.len(), 2);

    let workflow_rules = repo.list_by_type("workflow").await.unwrap();
    assert_eq!(workflow_rules.len(), 1);
}

#[tokio::test]
async fn test_project_rule_repository_enable_disable() {
    let (db, _temp) = create_test_db().await;
    run_project_migrations(&db).await;

    let repo = ProjectRuleRepository::new(db.clone());

    // Create rule
    let rule = ProjectRule::new(
        "Test rule".to_string(),
        "custom".to_string(),
        "{}".to_string(),
    );
    let rule_id = rule.id.clone();
    repo.save(&rule).await.unwrap();

    // Verify initially enabled
    let loaded_rule = repo.find_by_id(&rule_id).await.unwrap();
    assert!(loaded_rule.enabled);

    // Disable
    repo.disable(&rule_id).await.unwrap();
    let loaded_rule = repo.find_by_id(&rule_id).await.unwrap();
    assert!(!loaded_rule.enabled);

    // Enable
    repo.enable(&rule_id).await.unwrap();
    let loaded_rule = repo.find_by_id(&rule_id).await.unwrap();
    assert!(loaded_rule.enabled);
}

#[tokio::test]
async fn test_project_rule_repository_list_enabled() {
    let (db, _temp) = create_test_db().await;
    run_project_migrations(&db).await;

    let repo = ProjectRuleRepository::new(db.clone());

    // Create enabled rule
    let rule1 = ProjectRule::new(
        "Enabled rule".to_string(),
        "style".to_string(),
        "{}".to_string(),
    );
    repo.save(&rule1).await.unwrap();

    // Create disabled rule
    let rule2 = ProjectRule::new(
        "Disabled rule".to_string(),
        "style".to_string(),
        "{}".to_string(),
    ).disabled();
    repo.save(&rule2).await.unwrap();

    // List enabled
    let enabled_rules = repo.list_enabled().await.unwrap();
    assert_eq!(enabled_rules.len(), 1);
    assert_eq!(enabled_rules[0].name, "Enabled rule");
}

#[tokio::test]
async fn test_bug_model_builder_pattern() {
    let mut bug = Bug::new("Test bug".to_string())
        .with_description("Description".to_string())
        .with_priority(BugPriority::Critical)
        .with_assignee("john".to_string())
        .with_reporter("jane".to_string());

    // Severity is set directly, not via builder
    bug.severity = Some("major".to_string());

    assert_eq!(bug.title, "Test bug");
    assert_eq!(bug.description, Some("Description".to_string()));
    assert_eq!(bug.priority, 1); // Critical = 1
    assert_eq!(bug.assignee, Some("john".to_string()));
    assert_eq!(bug.reporter, Some("jane".to_string()));
    assert_eq!(bug.severity, Some("major".to_string()));
}

#[tokio::test]
async fn test_bug_model_status_transitions() {
    let mut bug = Bug::new("Test".to_string());
    assert_eq!(bug.status, BugStatus::Open.as_str());

    bug.start_work();
    assert_eq!(bug.status, BugStatus::InProgress.as_str());

    bug.mark_fixed();
    assert_eq!(bug.status, BugStatus::Fixed.as_str());
    assert!(bug.resolved_at.is_some());
}

#[tokio::test]
async fn test_project_rule_model_builder_pattern() {
    let rule = ProjectRule::new(
        "Test rule".to_string(),
        "security".to_string(),
        r#"{"check": "sql_injection"}"#.to_string(),
    )
    .with_description("Prevent SQL injection".to_string())
    .with_severity("error");

    assert_eq!(rule.name, "Test rule");
    assert_eq!(rule.rule_type, "security");
    assert_eq!(rule.description, Some("Prevent SQL injection".to_string()));
    assert_eq!(rule.severity, "error");
    assert!(rule.enabled);
    assert!(rule.is_error());
}

#[tokio::test]
async fn test_project_rule_model_enable_disable() {
    let mut rule = ProjectRule::new(
        "Test".to_string(),
        "style".to_string(),
        "{}".to_string(),
    );

    assert!(rule.is_enabled());

    rule.disable();
    assert!(!rule.is_enabled());

    rule.enable();
    assert!(rule.is_enabled());
}
