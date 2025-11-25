//! Integration tests for the full pattern selection flow
//!
//! Tests the complete flow: task → classify → route → build → execute

use orca::services::{TaskCategory, TaskClassifier};

mod common;

/// Test that the classification → routing flow works correctly for simple queries
#[test]
fn test_flow_simple_query_classification() {
    let classifier = TaskClassifier::new();

    // Test various simple query patterns
    let simple_queries = vec![
        "What is 2+2?",
        "How many days in a week?",
        "Who is the president?",
    ];

    for query in simple_queries {
        let category = classifier.classify(query);
        assert_eq!(
            category,
            TaskCategory::SimpleQuery,
            "Expected SimpleQuery for: {}",
            query
        );

        // Verify pattern config mapping
        let config_id = category.default_pattern_config_id();
        assert_eq!(
            config_id, "default_react_simple",
            "SimpleQuery should map to react_simple pattern"
        );
    }
}

/// Test that code generation tasks are routed to reflection pattern
#[test]
fn test_flow_code_generation_classification() {
    let classifier = TaskClassifier::new();

    let code_tasks = vec![
        "Write a function to sort an array",
        "Implement unit tests for authentication",
        "Refactor the database module",
        "Debug the login issue",
    ];

    for task in code_tasks {
        let category = classifier.classify(task);
        assert_eq!(
            category,
            TaskCategory::CodeGeneration,
            "Expected CodeGeneration for: {}",
            task
        );

        let config_id = category.default_pattern_config_id();
        assert_eq!(
            config_id, "default_reflection_code",
            "CodeGeneration should map to reflection pattern"
        );
    }
}

/// Test that research tasks are routed to plan-execute pattern
#[test]
fn test_flow_research_classification() {
    let classifier = TaskClassifier::new();

    let research_tasks = vec![
        "Research how async/await works in Rust",
        "Investigate how the caching system works",
        "Compare different approaches for state management",
    ];

    for task in research_tasks {
        let category = classifier.classify(task);
        assert_eq!(
            category,
            TaskCategory::Research,
            "Expected Research for: {}",
            task
        );

        let config_id = category.default_pattern_config_id();
        assert_eq!(
            config_id, "default_plan_execute",
            "Research should map to plan_execute pattern"
        );
    }
}

/// Test that data analysis tasks are routed correctly
#[test]
fn test_flow_data_analysis_classification() {
    let classifier = TaskClassifier::new();

    let data_tasks = vec!["Analyze the data", "Generate a report", "Calculate statistics"];

    for task in data_tasks {
        let category = classifier.classify(task);
        assert_eq!(
            category,
            TaskCategory::DataAnalysis,
            "Expected DataAnalysis for: {}",
            task
        );

        let config_id = category.default_pattern_config_id();
        assert_eq!(
            config_id, "default_plan_execute",
            "DataAnalysis should map to plan_execute pattern"
        );
    }
}

/// Test that file operation tasks are routed correctly
#[test]
fn test_flow_file_operation_classification() {
    let classifier = TaskClassifier::new();

    let file_tasks = vec!["Read file config.toml", "Write file output.txt", "List all files"];

    for task in file_tasks {
        let category = classifier.classify(task);
        assert_eq!(
            category,
            TaskCategory::FileOperation,
            "Expected FileOperation for: {}",
            task
        );

        let config_id = category.default_pattern_config_id();
        assert_eq!(
            config_id, "default_react",
            "FileOperation should map to default react pattern"
        );
    }
}

/// Test that system command tasks are routed correctly
#[test]
fn test_flow_system_command_classification() {
    let classifier = TaskClassifier::new();

    let system_tasks = vec!["Run the command now", "Git status", "Build the project"];

    for task in system_tasks {
        let category = classifier.classify(task);
        assert_eq!(
            category,
            TaskCategory::SystemCommand,
            "Expected SystemCommand for: {}",
            task
        );

        let config_id = category.default_pattern_config_id();
        assert_eq!(
            config_id, "default_react",
            "SystemCommand should map to default react pattern"
        );
    }
}

/// Test confidence scoring works correctly
#[test]
fn test_flow_confidence_scoring() {
    let classifier = TaskClassifier::new();

    // High-confidence classification (multiple pattern matches)
    let (category, confidence) =
        classifier.classify_with_confidence("Write unit tests for the auth module");
    assert_eq!(category, TaskCategory::CodeGeneration);
    assert!(confidence > 0.7, "Should have high confidence for clear code task");

    // Low-confidence classification (no pattern matches)
    let (category, confidence) = classifier.classify_with_confidence("xyz abc 123 random");
    assert_eq!(category, TaskCategory::General);
    assert!(confidence < 0.5, "Should have low confidence for unclear task");
}

/// Test that general tasks fall back correctly
#[test]
fn test_flow_general_fallback() {
    let classifier = TaskClassifier::new();

    // Use truly unrecognizable inputs that don't match any patterns
    let vague_tasks = vec!["xyz abc 123", "blah blah blah", "random nonsense here"];

    for task in vague_tasks {
        let category = classifier.classify(task);
        assert_eq!(
            category,
            TaskCategory::General,
            "Expected General fallback for: {}",
            task
        );

        let config_id = category.default_pattern_config_id();
        assert_eq!(
            config_id, "default_react",
            "General should map to default react pattern"
        );
    }
}

/// Test that custom categories work correctly
#[test]
fn test_flow_custom_category() {
    let custom = TaskCategory::Custom("my_special_pattern".to_string());

    assert_eq!(custom.as_str(), "my_special_pattern");
    assert_eq!(custom.display_name(), "Custom");
    // Custom categories fall back to default react
    assert_eq!(custom.default_pattern_config_id(), "default_react");
}

/// Test pattern config ID consistency
#[test]
fn test_pattern_config_id_consistency() {
    // Verify all categories have consistent config IDs
    let categories = vec![
        (TaskCategory::SimpleQuery, "default_react_simple"),
        (TaskCategory::FileOperation, "default_react"),
        (TaskCategory::CodeGeneration, "default_reflection_code"),
        (TaskCategory::Research, "default_plan_execute"),
        (TaskCategory::DataAnalysis, "default_plan_execute"),
        (TaskCategory::SystemCommand, "default_react"),
        (TaskCategory::General, "default_react"),
    ];

    for (category, expected_config) in categories {
        assert_eq!(
            category.default_pattern_config_id(),
            expected_config,
            "{:?} should map to {}",
            category,
            expected_config
        );
    }
}
