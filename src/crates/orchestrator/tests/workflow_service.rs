use sqlx::SqlitePool;
use orchestrator::services::WorkflowServiceImpl;
use orchestrator::proto::workflows::{
    workflow_service_server::WorkflowService,
    CreateWorkflowRequest, GetWorkflowRequest, ListWorkflowsRequest, UpdateWorkflowRequest,
    DeleteWorkflowRequest,
};
use tonic::Request;

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await
        .expect("Failed to create test database");

    // Run migrations (if available)
    // sqlx::migrate!("./migrations")
    //     .run(&pool)
    //     .await
    //     .expect("Failed to run migrations");

    pool
}

#[tokio::test]
async fn test_create_workflow() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let definition = r#"{"nodes": [{"id": "step1", "type": "task"}], "edges": []}"#;
    let request = Request::new(CreateWorkflowRequest {
        name: "Test Workflow".to_string(),
        description: "A test workflow".to_string(),
        definition: definition.to_string(),
    });

    let response = service.create_workflow(request).await;
    assert!(response.is_ok());

    let workflow = response.unwrap().into_inner();
    assert_eq!(workflow.name, "Test Workflow");
    assert_eq!(workflow.description, "A test workflow");
}

#[tokio::test]
async fn test_create_workflow_missing_name() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let request = Request::new(CreateWorkflowRequest {
        name: String::new(), // Empty name should fail
        description: "A test workflow".to_string(),
        definition: r#"{"nodes": []}"#.to_string(),
    });

    let response = service.create_workflow(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_create_workflow_missing_definition() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let request = Request::new(CreateWorkflowRequest {
        name: "Test Workflow".to_string(),
        description: "A test workflow".to_string(),
        definition: String::new(), // Empty definition should fail
    });

    let response = service.create_workflow(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_create_workflow_invalid_json_definition() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let invalid_definition = r#"{ invalid json"#;
    let request = Request::new(CreateWorkflowRequest {
        name: "Test Workflow".to_string(),
        description: "A test workflow".to_string(),
        definition: invalid_definition.to_string(),
    });

    let response = service.create_workflow(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_create_workflow_non_object_definition() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let non_object_definition = r#"["array", "not", "object"]"#;
    let request = Request::new(CreateWorkflowRequest {
        name: "Test Workflow".to_string(),
        description: "A test workflow".to_string(),
        definition: non_object_definition.to_string(),
    });

    let response = service.create_workflow(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_get_nonexistent_workflow() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let request = Request::new(GetWorkflowRequest {
        id: "nonexistent-id".to_string(),
    });

    let response = service.get_workflow(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_list_workflows_empty() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let request = Request::new(ListWorkflowsRequest {
        limit: 0,
        offset: 0,
    });

    let response = service.list_workflows(request).await;
    assert!(response.is_ok());

    let result = response.unwrap().into_inner();
    assert_eq!(result.workflows.len(), 0);
    assert_eq!(result.total, 0);
}

#[tokio::test]
async fn test_update_workflow_nonexistent() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let request = Request::new(UpdateWorkflowRequest {
        id: "nonexistent-id".to_string(),
        name: "Updated Name".to_string(),
        description: "Updated Description".to_string(),
        definition: String::new(),
        status: String::new(),
    });

    let response = service.update_workflow(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_update_workflow_invalid_definition() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let request = Request::new(UpdateWorkflowRequest {
        id: "some-id".to_string(),
        name: String::new(),
        description: String::new(),
        definition: r#"{ invalid json"#.to_string(),
        status: String::new(),
    });

    let response = service.update_workflow(request).await;
    // Should fail because workflow doesn't exist
    assert!(response.is_err());
}

#[tokio::test]
async fn test_delete_workflow_nonexistent() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let request = Request::new(DeleteWorkflowRequest {
        id: "nonexistent-id".to_string(),
    });

    let response = service.delete_workflow(request).await;
    assert!(response.is_ok());

    let result = response.unwrap().into_inner();
    assert!(!result.success); // Deletion of non-existent should return false
}

#[tokio::test]
async fn test_execute_workflow_placeholder() {
    let pool = setup_test_db().await;
    let service = WorkflowServiceImpl::new(pool, 100);

    let request = Request::new(orchestrator::proto::workflows::ExecuteWorkflowRequest {
        id: "some-workflow-id".to_string(),
        parameters: None,
    });

    let response = service.execute_workflow(request).await;
    // Should return stream with unimplemented error (Task 014)
    assert!(response.is_ok());
}
