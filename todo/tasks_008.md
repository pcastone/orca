# Task 008: Implement Database Layer with SQLx

## Objective
Create the database abstraction layer using SQLx for the orchestrator, including connection pooling, repository pattern for tasks/workflows, and integration with existing migrations.

## Priority
**CRITICAL** - Required for persistent storage

## Dependencies
- Task 007 (Server infrastructure)
- Existing database migrations in `src/crates/orchestrator/migrations/`

## Implementation Details

### Files to Create

1. **`src/crates/orchestrator/src/database/mod.rs`**:
```rust
pub mod connection;
pub mod repository;

pub use connection::Database;
pub use repository::{TaskRepository, WorkflowRepository};
```

2. **`src/crates/orchestrator/src/database/connection.rs`**:
```rust
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use anyhow::{Result, Context};
use std::time::Duration;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        tracing::info!("Connecting to database: {}", database_url);

        let pool = SqlitePoolOptions::new()
            .max_connections(20)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
            .context("Failed to connect to database")?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        tracing::info!("Running database migrations");

        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .context("Failed to run migrations")?;

        tracing::info!("Migrations completed successfully");
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .context("Database health check failed")?;

        Ok(())
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_connection() {
        let db = Database::new("sqlite::memory:").await.unwrap();
        assert!(db.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_migrations() {
        let db = Database::new("sqlite::memory:").await.unwrap();
        let result = db.run_migrations().await;
        assert!(result.is_ok());
    }
}
```

3. **`src/crates/orchestrator/src/database/repository/mod.rs`**:
```rust
pub mod task;
pub mod workflow;

pub use task::TaskRepository;
pub use workflow::WorkflowRepository;
```

4. **`src/crates/orchestrator/src/database/repository/task.rs`**:
```rust
use sqlx::SqlitePool;
use domain::{Task, TaskStatus, TaskType};
use anyhow::{Result, Context};
use chrono::Utc;

pub struct TaskRepository {
    pool: SqlitePool,
}

impl TaskRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, task: &Task) -> Result<()> {
        let status_str = format!("{:?}", task.status).to_lowercase();
        let task_type_str = match &task.task_type {
            TaskType::Code => "code",
            TaskType::Research => "research",
            TaskType::Review => "review",
            TaskType::Custom(s) => s.as_str(),
        };

        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, title, description, task_type, status,
                config, metadata, workspace_path,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&task.id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(task_type_str)
        .bind(&status_str)
        .bind(task.config.as_ref().map(|v| serde_json::to_string(v).ok()).flatten())
        .bind(task.metadata.as_ref().map(|v| serde_json::to_string(v).ok()).flatten())
        .bind(&task.workspace_path)
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to create task")?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<Task>> {
        let row = sqlx::query_as::<_, TaskRow>(
            "SELECT * FROM tasks WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch task")?;

        row.map(|r| r.into_task()).transpose()
    }

    pub async fn list(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        status: Option<TaskStatus>,
    ) -> Result<Vec<Task>> {
        let mut query = String::from("SELECT * FROM tasks WHERE 1=1");

        if let Some(status) = status {
            let status_str = format!("{:?}", status).to_lowercase();
            query.push_str(&format!(" AND status = '{}'", status_str));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = sqlx::query_as::<_, TaskRow>(&query)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list tasks")?;

        rows.into_iter()
            .map(|r| r.into_task())
            .collect()
    }

    pub async fn update(&self, task: &Task) -> Result<()> {
        let status_str = format!("{:?}", task.status).to_lowercase();

        sqlx::query(
            r#"
            UPDATE tasks
            SET title = ?, description = ?, status = ?,
                config = ?, metadata = ?, updated_at = ?,
                started_at = ?, completed_at = ?, error = ?
            WHERE id = ?
            "#
        )
        .bind(&task.title)
        .bind(&task.description)
        .bind(&status_str)
        .bind(task.config.as_ref().map(|v| serde_json::to_string(v).ok()).flatten())
        .bind(task.metadata.as_ref().map(|v| serde_json::to_string(v).ok()).flatten())
        .bind(task.updated_at.to_rfc3339())
        .bind(task.started_at.map(|t| t.to_rfc3339()))
        .bind(task.completed_at.map(|t| t.to_rfc3339()))
        .bind(&task.error)
        .bind(&task.id)
        .execute(&self.pool)
        .await
        .context("Failed to update task")?;

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete task")?;

        Ok(result.rows_affected() > 0)
    }
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: String,
    title: String,
    description: String,
    task_type: String,
    status: String,
    config: Option<String>,
    metadata: Option<String>,
    workspace_path: Option<String>,
    created_at: String,
    updated_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    error: Option<String>,
}

impl TaskRow {
    fn into_task(self) -> Result<Task> {
        let task_type = match self.task_type.as_str() {
            "code" => TaskType::Code,
            "research" => TaskType::Research,
            "review" => TaskType::Review,
            other => TaskType::Custom(other.to_string()),
        };

        let status = TaskStatus::from_str(&self.status)
            .context("Invalid task status")?;

        Ok(Task {
            id: self.id,
            title: self.title,
            description: self.description,
            task_type,
            status,
            config: self.config.as_ref()
                .map(|s| serde_json::from_str(s))
                .transpose()?,
            metadata: self.metadata.as_ref()
                .map(|s| serde_json::from_str(s))
                .transpose()?,
            workspace_path: self.workspace_path,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)?
                .with_timezone(&Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)?
                .with_timezone(&Utc),
            started_at: self.started_at.as_ref()
                .map(|s| chrono::DateTime::parse_from_rfc3339(s))
                .transpose()?
                .map(|dt| dt.with_timezone(&Utc)),
            completed_at: self.completed_at.as_ref()
                .map(|s| chrono::DateTime::parse_from_rfc3339(s))
                .transpose()?
                .map(|dt| dt.with_timezone(&Utc)),
            error: self.error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    async fn setup_test_db() -> Database {
        let db = Database::new("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_create_and_get_task() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db.pool().clone());

        let task = Task::new(
            "Test Task".to_string(),
            "Description".to_string(),
            TaskType::Code,
        );

        repo.create(&task).await.unwrap();

        let fetched = repo.get_by_id(&task.id).await.unwrap();
        assert!(fetched.is_some());

        let fetched_task = fetched.unwrap();
        assert_eq!(fetched_task.title, task.title);
        assert_eq!(fetched_task.status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn test_update_task() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db.pool().clone());

        let mut task = Task::new("T".to_string(), "D".to_string(), TaskType::Code);
        repo.create(&task).await.unwrap();

        task.status = TaskStatus::Completed;
        task.completed_at = Some(Utc::now());
        repo.update(&task).await.unwrap();

        let fetched = repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(fetched.status, TaskStatus::Completed);
        assert!(fetched.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_delete_task() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db.pool().clone());

        let task = Task::new("T".to_string(), "D".to_string(), TaskType::Code);
        repo.create(&task).await.unwrap();

        let deleted = repo.delete(&task.id).await.unwrap();
        assert!(deleted);

        let fetched = repo.get_by_id(&task.id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let db = setup_test_db().await;
        let repo = TaskRepository::new(db.pool().clone());

        // Create multiple tasks
        for i in 0..5 {
            let task = Task::new(
                format!("Task {}", i),
                "Desc".to_string(),
                TaskType::Code,
            );
            repo.create(&task).await.unwrap();
        }

        let tasks = repo.list(Some(3), None, None).await.unwrap();
        assert_eq!(tasks.len(), 3);

        let tasks = repo.list(None, None, Some(TaskStatus::Pending)).await.unwrap();
        assert_eq!(tasks.len(), 5);
    }
}
```

## Update Cargo.toml

**`src/crates/orchestrator/Cargo.toml`**:
```toml
[dependencies]
sqlx = { version = "0.7", features = [
    "runtime-tokio-rustls",
    "sqlite",
    "macros",
    "migrate"
] }
chrono = { workspace = true }
serde_json = { workspace = true }
```

## Unit Tests

All tests embedded in implementation files.

## Acceptance Criteria

- [ ] Database connection pool with SQLite
- [ ] Connection timeout and max connections configured
- [ ] Migrations run automatically on startup
- [ ] TaskRepository CRUD operations
- [ ] WorkflowRepository CRUD operations
- [ ] Proper error handling with context
- [ ] Health check query
- [ ] List with pagination (limit/offset)
- [ ] Filter by status
- [ ] JSON serialization for config/metadata
- [ ] RFC3339 timestamp storage
- [ ] All tests pass with in-memory database

## Complexity
**Moderate** - Standard SQLx repository pattern

## Estimated Effort
**8-10 hours**

## Notes
- SQLite for simplicity (can migrate to PostgreSQL later)
- Use parameterized queries to prevent SQL injection
- Store timestamps as RFC3339 strings
- Store JSON as TEXT
- Connection pool shared via Arc
- Run migrations on startup for zero-config deployment
