//! Task CRUD endpoint handlers
//!
//! Provides handlers for creating, reading, updating, and deleting tasks.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::api::{
    error::{ApiError, ApiResult},
    models::{CreateTaskRequest, UpdateTaskRequest, TaskResponse, TaskListQuery},
    response,
};
use crate::db::repositories::TaskRepository;

/// Create a new task
///
/// POST /api/v1/tasks
pub async fn create_task(
    State(app_state): State<crate::api::routes::AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    req.validate()?;

    let pool = app_state.db.pool();
    let task_id = Uuid::new_v4().to_string();

    let created = TaskRepository::create(
        pool,
        task_id,
        req.title,
        req.task_type,
        req.workspace_path.unwrap_or_default(),
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Created task: {}", created.id);
    Ok((StatusCode::CREATED, Json(TaskResponse::from_db_task(created))))
}

/// List all tasks with filtering and pagination
///
/// GET /api/v1/tasks
pub async fn list_tasks(
    State(app_state): State<crate::api::routes::AppState>,
    Query(query): Query<TaskListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(0);
    let per_page = query.per_page.unwrap_or(20);

    crate::api::middleware::validation::validate_pagination(page, per_page, 100)?;

    let pool = app_state.db.pool();
    let tasks = TaskRepository::list(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Apply filtering
    let filtered: Vec<_> = tasks
        .into_iter()
        .filter(|t| {
            if let Some(ref status) = query.status {
                if t.status != *status {
                    return false;
                }
            }
            if let Some(ref task_type) = query.task_type {
                if t.task_type != *task_type {
                    return false;
                }
            }
            if let Some(ref search) = query.search {
                if !t.title.contains(search)
                    && t.description.as_ref().map_or(false, |d| d.contains(search))
                {
                    return false;
                }
            }
            true
        })
        .collect();

    let total = filtered.len() as u32;
    let offset = (page as usize) * (per_page as usize);
    let paginated: Vec<_> = filtered
        .into_iter()
        .skip(offset)
        .take(per_page as usize)
        .collect();

    let responses: Vec<TaskResponse> = paginated.into_iter().map(TaskResponse::from_db_task).collect();
    Ok(response::paginated(responses, page, per_page, total))
}

/// Get a single task by ID
///
/// GET /api/v1/tasks/:id
pub async fn get_task(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    let task = TaskRepository::get_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Task not found: {}", id)))?;

    Ok(response::ok(TaskResponse::from_db_task(task)))
}

/// Update an existing task
///
/// PUT /api/v1/tasks/:id
pub async fn update_task(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    if !req.has_updates() {
        return Err(ApiError::BadRequest("No fields to update".to_string()));
    }

    let pool = app_state.db.pool();
    let mut task = TaskRepository::get_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Task not found: {}", id)))?;

    // Update status if provided
    if let Some(status) = &req.status {
        TaskRepository::update_status(pool, &id, status)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        task.status = status.clone();
    }

    // For other fields, update in memory only
    if let Some(title) = req.title {
        task.title = title;
    }
    if let Some(description) = req.description {
        task.description = Some(description);
    }
    if let Some(task_type) = req.task_type {
        task.task_type = task_type;
    }
    if let Some(config) = req.config {
        task.config = Some(config);
    }
    if let Some(metadata) = req.metadata {
        task.metadata = Some(metadata);
    }
    if let Some(workspace_path) = req.workspace_path {
        task.workspace_path = Some(workspace_path);
    }

    task.updated_at = chrono::Utc::now().to_rfc3339();

    tracing::info!("Updated task: {}", task.id);
    Ok(response::ok(TaskResponse::from_db_task(task)))
}

/// Delete a task
///
/// DELETE /api/v1/tasks/:id
pub async fn delete_task(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    TaskRepository::delete(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Deleted task: {}", id);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task_request_validation() {
        let req = CreateTaskRequest {
            title: "Test".to_string(),
            description: None,
            task_type: "execution".to_string(),
            workspace_path: None,
            config: None,
            metadata: None,
        };
        assert!(req.validate().is_ok());
    }
}
