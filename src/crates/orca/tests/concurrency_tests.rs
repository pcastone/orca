//! Integration tests for concurrent workflow execution (ORCA-034)

use orca::{OrcaConfig};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use std::sync::Arc;
use tokio::time::sleep;

fn create_test_config_with_concurrency(max_concurrent: usize) -> OrcaConfig {
    let mut config = OrcaConfig::default();
    config.execution.max_concurrent_tasks = max_concurrent;
    config.llm.provider = "ollama".to_string();
    config.llm.model = "llama2".to_string();
    config.llm.api_base = Some("http://localhost:11434".to_string());
    config
}

#[test]
fn test_concurrency_configuration() {
    let config = create_test_config_with_concurrency(1);
    assert_eq!(config.execution.max_concurrent_tasks, 1);

    let config = create_test_config_with_concurrency(3);
    assert_eq!(config.execution.max_concurrent_tasks, 3);

    let config = create_test_config_with_concurrency(10);
    assert_eq!(config.execution.max_concurrent_tasks, 10);
}

#[tokio::test]
async fn test_semaphore_limits_concurrent_tasks() {
    // Test that semaphore correctly limits concurrency
    let max_concurrent = 3;
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let start_time = Instant::now();
    let mut handles = vec![];

    // Launch 9 tasks (3 batches of 3)
    for i in 0..9 {
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            // Simulate task execution
            sleep(Duration::from_millis(100)).await;
            i
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let results: Vec<_> = futures::future::join_all(handles).await;

    let duration = start_time.elapsed();

    // With max_concurrent=3 and 9 tasks taking 100ms each:
    // Expected: ~300ms (3 batches × 100ms)
    // Allow some overhead, should be between 250ms and 500ms
    assert!(duration >= Duration::from_millis(250), "Too fast: {:?}", duration);
    assert!(duration < Duration::from_millis(500), "Too slow: {:?}", duration);

    // Verify all tasks completed
    assert_eq!(results.len(), 9);
    for (idx, result) in results.iter().enumerate() {
        assert!(result.is_ok());
        assert_eq!(result.as_ref().unwrap(), &idx);
    }
}

#[tokio::test]
async fn test_sequential_execution_with_concurrency_1() {
    // Test that concurrency=1 means sequential execution
    let semaphore = Arc::new(Semaphore::new(1));

    let start_time = Instant::now();
    let mut handles = vec![];

    // Launch 3 tasks
    for i in 0..3 {
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            sleep(Duration::from_millis(100)).await;
            i
        });
        handles.push(handle);
    }

    futures::future::join_all(handles).await;

    let duration = start_time.elapsed();

    // With concurrency=1, tasks run sequentially
    // Expected: ~300ms (3 tasks × 100ms)
    assert!(duration >= Duration::from_millis(250));
    assert!(duration < Duration::from_millis(400));
}

#[tokio::test]
async fn test_parallel_execution_with_high_concurrency() {
    // Test that high concurrency allows parallel execution
    let semaphore = Arc::new(Semaphore::new(10));

    let start_time = Instant::now();
    let mut handles = vec![];

    // Launch 5 tasks
    for i in 0..5 {
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            sleep(Duration::from_millis(100)).await;
            i
        });
        handles.push(handle);
    }

    futures::future::join_all(handles).await;

    let duration = start_time.elapsed();

    // With concurrency=10 and 5 tasks, all should run in parallel
    // Expected: ~100ms (single batch)
    assert!(duration >= Duration::from_millis(80));
    assert!(duration < Duration::from_millis(200));
}

#[test]
fn test_concurrency_values() {
    // Test various concurrency values
    let concurrency_limits = vec![1, 2, 3, 4, 5, 10, 20];

    for limit in concurrency_limits {
        let config = create_test_config_with_concurrency(limit);
        assert_eq!(config.execution.max_concurrent_tasks, limit);
    }
}

#[tokio::test]
async fn test_concurrent_batch_execution() {
    // Simulate batch execution like the router does
    let max_concurrent = 2;
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let mut total_completed = 0;
    let task_ids = vec!["task1", "task2", "task3", "task4"];

    for batch_start in (0..task_ids.len()).step_by(max_concurrent) {
        let batch_end = (batch_start + max_concurrent).min(task_ids.len());
        let batch = &task_ids[batch_start..batch_end];

        let mut handles = vec![];

        for &task_id in batch {
            let sem = semaphore.clone();
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                sleep(Duration::from_millis(50)).await;
                task_id
            });
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;
        total_completed += results.len();
    }

    assert_eq!(total_completed, 4);
}

#[tokio::test]
async fn test_concurrent_task_failure_doesnt_block_others() {
    // Test that one failing task doesn't prevent others from completing
    let semaphore = Arc::new(Semaphore::new(3));

    let mut handles = vec![];

    for i in 0..5 {
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            sleep(Duration::from_millis(50)).await;

            // Simulate one failing task
            if i == 2 {
                Err::<i32, &str>("Task failed")
            } else {
                Ok(i)
            }
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    // Check that 4 succeeded and 1 failed
    let successful = results.iter().filter(|r| r.is_ok() && r.as_ref().unwrap().is_ok()).count();
    let failed = results.iter().filter(|r| r.is_ok() && r.as_ref().unwrap().is_err()).count();

    assert_eq!(successful, 4);
    assert_eq!(failed, 1);
}
