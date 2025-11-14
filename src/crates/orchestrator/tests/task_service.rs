use sqlx::SqlitePool;
use orchestrator::services::TaskServiceImpl;
use orchestrator::proto::tasks::{
    task_service_server::TaskService,
    CreateTaskRequest, GetTaskRequest, ListTasksRequest, UpdateTaskRequest,
    DeleteTaskRequest,
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
async fn test_create_task() {
    let pool = setup_test_db().await;
    let service = TaskServiceImpl::new(pool, 100);

    let request = Request::new(CreateTaskRequest {
        title: "Test Task".to_string(),
        description: "A test task".to_string(),
        task_type: "code".to_string(),
        config: None,
        metadata: None,
        workspace_path: String::new(),
    });

    let response = service.create_task(request).await;
    assert!(response.is_ok());

    let task = response.unwrap().into_inner();
    assert_eq!(task.title, "Test Task");
    assert_eq!(task.description, "A test task");
}

#[tokio::test]
async fn test_create_task_missing_title() {
    let pool = setup_test_db().await;
    let service = TaskServiceImpl::new(pool, 100);

    let request = Request::new(CreateTaskRequest {
        title: String::new(), // Empty title should fail
        description: "A test task".to_string(),
        task_type: "code".to_string(),
        config: None,
        metadata: None,
        workspace_path: String::new(),
    });

    let response = service.create_task(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_create_task_with_config() {
    let pool = setup_test_db().await;
    let service = TaskServiceImpl::new(pool, 100);

    let config = r#"{"timeout": 3600, "retries": 3}"#;
    let request = Request::new(CreateTaskRequest {
        title: "Task with Config".to_string(),
        description: "Task with JSON config".to_string(),
        task_type: "research".to_string(),
        config: Some(config.to_string()),
        metadata: None,
        workspace_path: String::new(),
    });

    let response = service.create_task(request).await;
    assert!(response.is_ok());

    let task = response.unwrap().into_inner();
    assert_eq!(task.title, "Task with Config");
}

#[tokio::test]
async fn test_create_task_invalid_config_json() {
    let pool = setup_test_db().await;
    let service = TaskServiceImpl::new(pool, 100);

    let invalid_config = r#"{ invalid json"#;
    let request = Request::new(CreateTaskRequest {
        title: "Task with Bad Config".to_string(),
        description: "Task with invalid JSON config".to_string(),
        task_type: "code".to_string(),
        config: Some(invalid_config.to_string()),
        metadata: None,
        workspace_path: String::new(),
    });

    let response = service.create_task(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_get_nonexistent_task() {
    let pool = setup_test_db().await;
    let service = TaskServiceImpl::new(pool, 100);

    let request = Request::new(GetTaskRequest {
        id: "nonexistent-id".to_string(),
    });

    let response = service.get_task(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_list_tasks_empty() {
    let pool = setup_test_db().await;
    let service = TaskServiceImpl::new(pool, 100);

    let request = Request::new(ListTasksRequest {
        limit: 0,
        offset: 0,
        status: 0,
    });

    let response = service.list_tasks(request).await;
    assert!(response.is_ok());

    let result = response.unwrap().into_inner();
    assert_eq!(result.tasks.len(), 0);
    assert_eq!(result.total, 0);
}

#[tokio::test]
async fn test_update_task_nonexistent() {
    let pool = setup_test_db().await;
    let service = TaskServiceImpl::new(pool, 100);

    let request = Request::new(UpdateTaskRequest {
        id: "nonexistent-id".to_string(),
        title: "Updated Title".to_string(),
        description: "Updated Description".to_string(),
        status: 0,
    });

    let response = service.update_task(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_delete_task_nonexistent() {
    let pool = setup_test_db().await;
    let service = TaskServiceImpl::new(pool, 100);

    let request = Request::new(DeleteTaskRequest {
        id: "nonexistent-id".to_string(),
    });

    let response = service.delete_task(request).await;
    assert!(response.is_ok());

    let result = response.unwrap().into_inner();
    assert!(!result.success); // Deletion of non-existent should return false
}

#[tokio::test]
async fn test_execute_task_placeholder() {
    let pool = setup_test_db().await;
    let service = TaskServiceImpl::new(pool, 100);

    let request = Request::new(orchestrator::proto::tasks::ExecuteTaskRequest {
        id: "some-task-id".to_string(),
        parameters: None,
    });

    let response = service.execute_task(request).await;
    // Should return stream with unimplemented error (Task 012)
    assert!(response.is_ok());
}
