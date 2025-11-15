// Integration tests for TUI functionality
// Task 028: Create integration tests

use aco::tui::{App, View, TaskItem, WorkflowItem, ExecutionEvent, TuiConfig};
use std::path::PathBuf;

/// Create a test config
fn test_config() -> TuiConfig {
    TuiConfig {
        server_url: "http://localhost:50051".to_string(),
        workspace: PathBuf::from("/tmp/test-workspace"),
        verbose: false,
    }
}

/// Create a sample task for testing
fn create_test_task(id: &str, status: &str) -> TaskItem {
    TaskItem {
        id: id.to_string(),
        title: format!("Test Task {}", id),
        description: format!("Description for task {}", id),
        status: status.to_string(),
        task_type: "execution".to_string(),
        config: r#"{"timeout": 300}"#.to_string(),
        metadata: r#"{"priority": "high"}"#.to_string(),
        workspace_path: format!("/tmp/workspace/{}", id),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

/// Create a sample workflow for testing
fn create_test_workflow(id: &str, status: &str) -> WorkflowItem {
    WorkflowItem {
        id: id.to_string(),
        name: format!("Test Workflow {}", id),
        status: status.to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

#[test]
fn test_app_initialization() {
    let app = App::new(test_config());
    assert_eq!(app.view(), View::TaskList);
    assert_eq!(app.tasks.len(), 0);
    assert_eq!(app.workflows.len(), 0);
    assert_eq!(app.selected, 0);
    assert_eq!(app.scroll, 0);
}

#[test]
fn test_task_list_navigation_flow() {
    let mut app = App::new(test_config());

    // Add multiple tasks
    for i in 0..5 {
        app.add_task(create_test_task(&format!("task-{}", i), "pending"));
    }

    assert_eq!(app.tasks.len(), 5);
    assert_eq!(app.selected, 0);

    // Navigate down
    app.next_item();
    assert_eq!(app.selected, 1);

    app.next_item();
    assert_eq!(app.selected, 2);

    // Navigate up
    app.previous_item();
    assert_eq!(app.selected, 1);

    // Jump to first
    app.first_item();
    assert_eq!(app.selected, 0);

    // Jump to last
    app.last_item();
    assert_eq!(app.selected, 4);
}

#[test]
fn test_task_selection_and_detail_view() {
    let mut app = App::new(test_config());
    app.add_task(create_test_task("task-123", "running"));

    // Start in task list
    assert_eq!(app.view(), View::TaskList);
    assert!(app.selected_task_id().is_none());

    // Select task to view details
    app.select_item();
    assert_eq!(app.view(), View::TaskDetail);
    assert_eq!(app.selected_task_id(), Some("task-123"));

    // Deselect to return to list
    app.deselect_item();
    assert_eq!(app.view(), View::TaskList);
    assert!(app.selected_task_id().is_none());
}

#[test]
fn test_view_cycling() {
    let mut app = App::new(test_config());

    // Start at TaskList
    assert_eq!(app.view(), View::TaskList);

    // Cycle forward: TaskList -> WorkflowList -> Help -> TaskList
    app.next_view();
    assert_eq!(app.view(), View::WorkflowList);

    app.next_view();
    assert_eq!(app.view(), View::Help);

    app.next_view();
    assert_eq!(app.view(), View::TaskList);

    // Cycle backward: TaskList -> Help -> WorkflowList -> TaskList
    app.previous_view();
    assert_eq!(app.view(), View::Help);

    app.previous_view();
    assert_eq!(app.view(), View::WorkflowList);

    app.previous_view();
    assert_eq!(app.view(), View::TaskList);
}

#[test]
fn test_direct_view_navigation() {
    let mut app = App::new(test_config());

    // Direct navigation to each view
    app.go_to_view(View::WorkflowList);
    assert_eq!(app.view(), View::WorkflowList);

    app.go_to_view(View::ExecutionStream);
    assert_eq!(app.view(), View::ExecutionStream);

    app.go_to_view(View::Help);
    assert_eq!(app.view(), View::Help);

    app.go_to_view(View::TaskList);
    assert_eq!(app.view(), View::TaskList);
}

#[test]
fn test_page_navigation_with_many_tasks() {
    let mut app = App::new(test_config());

    // Add 30 tasks
    for i in 0..30 {
        app.add_task(create_test_task(&format!("task-{}", i), "pending"));
    }

    assert_eq!(app.selected, 0);

    // Page down (10 items)
    app.page_down();
    assert_eq!(app.selected, 10);

    app.page_down();
    assert_eq!(app.selected, 20);

    // Page down again (should stop at last item)
    app.page_down();
    assert_eq!(app.selected, 29);

    // Page up
    app.page_up();
    assert_eq!(app.selected, 19);

    app.page_up();
    assert_eq!(app.selected, 9);
}

#[test]
fn test_workflow_list_operations() {
    let mut app = App::new(test_config());

    // Add workflows
    for i in 0..3 {
        app.add_workflow(create_test_workflow(&format!("wf-{}", i), "draft"));
    }

    assert_eq!(app.workflows.len(), 3);

    // Switch to workflow view
    app.go_to_view(View::WorkflowList);
    assert_eq!(app.view(), View::WorkflowList);

    // Navigate workflows
    app.next_item();
    assert_eq!(app.selected, 1);

    // Select workflow
    app.select_item();
    assert_eq!(app.view(), View::WorkflowDetail);
    assert_eq!(app.selected_workflow_id(), Some("wf-1"));
}

#[test]
fn test_execution_events() {
    let mut app = App::new(test_config());

    // Initially no events
    assert_eq!(app.execution_events.len(), 0);
    assert!(app.executing_id().is_none());

    // Add execution events
    app.add_execution_event(ExecutionEvent {
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        event_type: "started".to_string(),
        message: "Task execution started".to_string(),
        status: "started".to_string(),
    });

    app.add_execution_event(ExecutionEvent {
        timestamp: "2024-01-01T00:00:01Z".to_string(),
        event_type: "progress".to_string(),
        message: "Processing...".to_string(),
        status: "in_progress".to_string(),
    });

    app.add_execution_event(ExecutionEvent {
        timestamp: "2024-01-01T00:00:02Z".to_string(),
        event_type: "completed".to_string(),
        message: "Task completed successfully".to_string(),
        status: "completed".to_string(),
    });

    assert_eq!(app.execution_events.len(), 3);

    // Clear execution
    app.clear_execution();
    assert_eq!(app.execution_events.len(), 0);
    assert!(app.executing_id().is_none());
}

#[test]
fn test_task_filtering_by_status() {
    let mut app = App::new(test_config());

    // Add tasks with different statuses
    app.add_task(create_test_task("task-1", "pending"));
    app.add_task(create_test_task("task-2", "running"));
    app.add_task(create_test_task("task-3", "completed"));
    app.add_task(create_test_task("task-4", "failed"));
    app.add_task(create_test_task("task-5", "pending"));

    let total = app.tasks.len();
    assert_eq!(total, 5);

    // Count by status
    let pending = app.tasks.iter().filter(|t| t.status == "pending").count();
    let running = app.tasks.iter().filter(|t| t.status == "running").count();
    let completed = app.tasks.iter().filter(|t| t.status == "completed").count();
    let failed = app.tasks.iter().filter(|t| t.status == "failed").count();

    assert_eq!(pending, 2);
    assert_eq!(running, 1);
    assert_eq!(completed, 1);
    assert_eq!(failed, 1);
}

#[test]
fn test_empty_list_edge_cases() {
    let mut app = App::new(test_config());

    // No tasks initially
    assert_eq!(app.tasks.len(), 0);

    // Navigation should not crash
    app.next_item();
    app.previous_item();
    app.first_item();
    app.last_item();
    app.page_up();
    app.page_down();

    // Selection should be 0
    assert_eq!(app.selected, 0);

    // Select item with no tasks should not crash
    app.select_item();
    assert!(app.selected_task_id().is_none());
}

#[test]
fn test_task_clear_and_add() {
    let mut app = App::new(test_config());

    // Add tasks
    for i in 0..5 {
        app.add_task(create_test_task(&format!("task-{}", i), "pending"));
    }
    assert_eq!(app.tasks.len(), 5);

    // Clear all tasks
    app.clear_tasks();
    assert_eq!(app.tasks.len(), 0);
    assert_eq!(app.selected, 0);
    assert_eq!(app.scroll, 0);

    // Add new tasks after clearing
    for i in 10..13 {
        app.add_task(create_test_task(&format!("task-{}", i), "running"));
    }
    assert_eq!(app.tasks.len(), 3);
}

#[test]
fn test_workflow_clear_and_add() {
    let mut app = App::new(test_config());

    // Add workflows
    for i in 0..3 {
        app.add_workflow(create_test_workflow(&format!("wf-{}", i), "active"));
    }
    assert_eq!(app.workflows.len(), 3);

    // Clear workflows
    app.clear_workflows();
    assert_eq!(app.workflows.len(), 0);
    assert_eq!(app.selected, 0);

    // Add new workflows
    app.add_workflow(create_test_workflow("wf-new", "draft"));
    assert_eq!(app.workflows.len(), 1);
}

#[test]
fn test_status_and_error_management() {
    let mut app = App::new(test_config());

    // Initially no error
    assert!(app.error().is_none());

    // Set status
    app.set_status("Loading tasks...".to_string());
    assert_eq!(app.status(), "Loading tasks...");

    // Set error
    app.set_error("Connection failed".to_string());
    assert_eq!(app.error(), Some("Connection failed"));

    // Clear error
    app.clear_error();
    assert!(app.error().is_none());
}

#[test]
fn test_quit_functionality() {
    let mut app = App::new(test_config());

    // App should be running initially
    assert!(app.is_running());
    assert!(!app.should_quit());

    // Quit the app
    app.quit();
    assert!(!app.is_running());
    assert!(app.should_quit());
}

#[test]
fn test_full_user_workflow() {
    let mut app = App::new(test_config());

    // 1. User starts the app
    assert_eq!(app.view(), View::TaskList);

    // 2. Add some tasks (simulating data load)
    for i in 0..10 {
        app.add_task(create_test_task(&format!("task-{}", i), "pending"));
    }

    // 3. User navigates to a specific task
    app.next_item();
    app.next_item();
    app.next_item(); // Selected index 3
    assert_eq!(app.selected, 3);

    // 4. User views task details
    app.select_item();
    assert_eq!(app.view(), View::TaskDetail);
    assert_eq!(app.selected_task_id(), Some("task-3"));

    // 5. User goes back to list
    app.deselect_item();
    assert_eq!(app.view(), View::TaskList);

    // 6. User switches to workflow view
    app.go_to_view(View::WorkflowList);
    assert_eq!(app.view(), View::WorkflowList);

    // 7. Add workflows
    app.add_workflow(create_test_workflow("wf-1", "active"));
    app.add_workflow(create_test_workflow("wf-2", "draft"));

    // 8. User navigates workflows
    app.next_item();
    assert_eq!(app.selected, 1);

    // 9. User views help
    app.go_to_view(View::Help);
    assert_eq!(app.view(), View::Help);

    // 10. User returns to tasks
    app.go_to_view(View::TaskList);
    assert_eq!(app.view(), View::TaskList);

    // 11. User quits
    app.quit();
    assert!(app.should_quit());
}

#[test]
fn test_refresh_flag() {
    let app = App::new(test_config());

    // Should not need refresh immediately
    assert!(!app.should_refresh());
}
