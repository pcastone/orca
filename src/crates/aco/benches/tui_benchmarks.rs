// Performance benchmarks for TUI
// Task 029: Create performance/benchmark tests

use aco::tui::{App, TaskItem, WorkflowItem, TuiConfig};
use std::path::PathBuf;
use std::time::Instant;

/// Create a test config
fn test_config() -> TuiConfig {
    TuiConfig {
        server_url: "http://localhost:50051".to_string(),
        workspace: PathBuf::from("/tmp/test-workspace"),
        verbose: false,
    }
}

/// Create a sample task for benchmarking
fn create_task(id: usize) -> TaskItem {
    TaskItem {
        id: format!("task-{}", id),
        title: format!("Test Task {}", id),
        description: format!("Description for task {}", id),
        status: "pending".to_string(),
        task_type: "execution".to_string(),
        config: r#"{"timeout": 300}"#.to_string(),
        metadata: r#"{"priority": "high"}"#.to_string(),
        workspace_path: format!("/tmp/workspace/task-{}", id),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

/// Create a sample workflow for benchmarking
fn create_workflow(id: usize) -> WorkflowItem {
    WorkflowItem {
        id: format!("wf-{}", id),
        name: format!("Test Workflow {}", id),
        status: "active".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

/// Benchmark: Create and initialize app
fn bench_app_initialization() {
    let start = Instant::now();

    for _ in 0..1000 {
        let _app = App::new(test_config());
    }

    let duration = start.elapsed();
    println!("App initialization (1000x): {:?} ({:.2?} avg)",
        duration, duration / 1000);
}

/// Benchmark: Add tasks to app
fn bench_add_tasks() {
    let mut app = App::new(test_config());
    let start = Instant::now();

    for i in 0..10000 {
        app.add_task(create_task(i));
    }

    let duration = start.elapsed();
    println!("Add 10,000 tasks: {:?} ({:.2?} avg)",
        duration, duration / 10000);
}

/// Benchmark: Navigation performance with many tasks
fn bench_navigation_many_tasks() {
    let mut app = App::new(test_config());

    // Add 10,000 tasks
    for i in 0..10000 {
        app.add_task(create_task(i));
    }

    let start = Instant::now();

    // Perform 10,000 navigation operations
    for _ in 0..5000 {
        app.next_item();
    }
    for _ in 0..5000 {
        app.previous_item();
    }

    let duration = start.elapsed();
    println!("Navigation (10,000 ops with 10,000 tasks): {:?} ({:.2?} avg)",
        duration, duration / 10000);
}

/// Benchmark: Page navigation performance
fn bench_page_navigation() {
    let mut app = App::new(test_config());

    // Add 10,000 tasks
    for i in 0..10000 {
        app.add_task(create_task(i));
    }

    let start = Instant::now();

    // Perform 1,000 page jumps
    for _ in 0..500 {
        app.page_down();
    }
    for _ in 0..500 {
        app.page_up();
    }

    let duration = start.elapsed();
    println!("Page navigation (1,000 ops with 10,000 tasks): {:?} ({:.2?} avg)",
        duration, duration / 1000);
}

/// Benchmark: First/last navigation
fn bench_jump_navigation() {
    let mut app = App::new(test_config());

    // Add 10,000 tasks
    for i in 0..10000 {
        app.add_task(create_task(i));
    }

    let start = Instant::now();

    // Jump between first and last 1,000 times
    for _ in 0..500 {
        app.first_item();
        app.last_item();
    }

    let duration = start.elapsed();
    println!("Jump navigation (1,000 first/last with 10,000 tasks): {:?} ({:.2?} avg)",
        duration, duration / 1000);
}

/// Benchmark: View switching
fn bench_view_switching() {
    let mut app = App::new(test_config());

    let start = Instant::now();

    // Switch views 10,000 times
    for _ in 0..10000 {
        app.next_view();
    }

    let duration = start.elapsed();
    println!("View switching (10,000 ops): {:?} ({:.2?} avg)",
        duration, duration / 10000);
}

/// Benchmark: Task selection and deselection
fn bench_task_selection() {
    let mut app = App::new(test_config());

    // Add 100 tasks
    for i in 0..100 {
        app.add_task(create_task(i));
    }

    let start = Instant::now();

    // Select and deselect 1,000 times
    for _ in 0..1000 {
        app.select_item();
        app.deselect_item();
    }

    let duration = start.elapsed();
    println!("Task selection (1,000 select/deselect): {:?} ({:.2?} avg)",
        duration, duration / 1000);
}

/// Benchmark: Clear and add operations
fn bench_clear_and_add() {
    let mut app = App::new(test_config());

    let start = Instant::now();

    // Clear and add 100 times
    for _ in 0..100 {
        for i in 0..100 {
            app.add_task(create_task(i));
        }
        app.clear_tasks();
    }

    let duration = start.elapsed();
    println!("Clear and add (100x with 100 tasks): {:?} ({:.2?} avg per iteration)",
        duration, duration / 100);
}

/// Benchmark: Workflow operations
fn bench_workflow_operations() {
    let mut app = App::new(test_config());

    let start = Instant::now();

    // Add 1,000 workflows
    for i in 0..1000 {
        app.add_workflow(create_workflow(i));
    }

    // Clear
    app.clear_workflows();

    let duration = start.elapsed();
    println!("Workflow operations (1,000 add + clear): {:?}",
        duration);
}

/// Benchmark: Status filtering
fn bench_status_filtering() {
    let mut app = App::new(test_config());

    // Add 10,000 tasks with various statuses
    for i in 0..10000 {
        let mut task = create_task(i);
        task.status = match i % 5 {
            0 => "pending",
            1 => "running",
            2 => "completed",
            3 => "failed",
            _ => "cancelled",
        }.to_string();
        app.add_task(task);
    }

    let start = Instant::now();

    // Filter by each status 100 times
    for _ in 0..100 {
        let _pending: Vec<_> = app.tasks.iter().filter(|t| t.status == "pending").collect();
        let _running: Vec<_> = app.tasks.iter().filter(|t| t.status == "running").collect();
        let _completed: Vec<_> = app.tasks.iter().filter(|t| t.status == "completed").collect();
        let _failed: Vec<_> = app.tasks.iter().filter(|t| t.status == "failed").collect();
    }

    let duration = start.elapsed();
    println!("Status filtering (400 filters on 10,000 tasks): {:?} ({:.2?} avg)",
        duration, duration / 400);
}

/// Benchmark: Memory usage with large datasets
fn bench_memory_large_dataset() {
    let mut app = App::new(test_config());

    println!("Adding 50,000 tasks...");
    let start = Instant::now();

    for i in 0..50000 {
        app.add_task(create_task(i));
    }

    let duration = start.elapsed();
    println!("Large dataset (50,000 tasks): {:?} total ({:.2?} avg)",
        duration, duration / 50000);

    // Verify data integrity
    assert_eq!(app.tasks.len(), 50000);
    println!("Data integrity verified: {} tasks", app.tasks.len());
}

fn main() {
    println!("=== TUI Performance Benchmarks ===\n");

    println!("1. App Initialization:");
    bench_app_initialization();
    println!();

    println!("2. Add Tasks:");
    bench_add_tasks();
    println!();

    println!("3. Navigation with Many Tasks:");
    bench_navigation_many_tasks();
    println!();

    println!("4. Page Navigation:");
    bench_page_navigation();
    println!();

    println!("5. Jump Navigation:");
    bench_jump_navigation();
    println!();

    println!("6. View Switching:");
    bench_view_switching();
    println!();

    println!("7. Task Selection:");
    bench_task_selection();
    println!();

    println!("8. Clear and Add:");
    bench_clear_and_add();
    println!();

    println!("9. Workflow Operations:");
    bench_workflow_operations();
    println!();

    println!("10. Status Filtering:");
    bench_status_filtering();
    println!();

    println!("11. Large Dataset Memory Test:");
    bench_memory_large_dataset();
    println!();

    println!("=== Benchmarks Complete ===");
}
