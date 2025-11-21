//! Bug CRUD endpoint handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::api::{
    error::{ApiError, ApiResult},
    models::bug::{CreateBugRequest, UpdateBugRequest, BugResponse, BugListQuery},
    response,
};
use crate::db::repositories::BugRepository;

/// Create a new bug
///
/// POST /api/v1/bugs
pub async fn create_bug(
    State(app_state): State<crate::api::routes::AppState>,
    Json(req): Json<CreateBugRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    req.validate()?;

    let pool = app_state.db.pool();
    let bug_id = Uuid::new_v4().to_string();
    let severity = req.severity.unwrap_or_else(|| "medium".to_string());

    let created = BugRepository::create(
        pool,
        bug_id,
        req.title,
        severity,
        req.description,
        req.task_id,
        req.workflow_id,
        req.reporter,
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Created bug: {}", created.id);
    Ok((StatusCode::CREATED, Json(BugResponse::from_db_bug(created))))
}

/// List all bugs with filtering and pagination
///
/// GET /api/v1/bugs
pub async fn list_bugs(
    State(app_state): State<crate::api::routes::AppState>,
    Query(query): Query<BugListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(0);
    let per_page = query.per_page.unwrap_or(20);

    crate::api::middleware::validation::validate_pagination(page, per_page, 100)?;

    let pool = app_state.db.pool();
    let bugs = BugRepository::list(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Apply filtering
    let filtered: Vec<_> = bugs
        .into_iter()
        .filter(|b| {
            if let Some(ref status) = query.status {
                if b.status != *status {
                    return false;
                }
            }
            if let Some(ref severity) = query.severity {
                if b.severity != *severity {
                    return false;
                }
            }
            if let Some(ref task_id) = query.task_id {
                if b.task_id.as_ref() != Some(task_id) {
                    return false;
                }
            }
            if let Some(ref assignee) = query.assignee {
                if b.assignee.as_ref() != Some(assignee) {
                    return false;
                }
            }
            if let Some(ref search) = query.search {
                let search_lower = search.to_lowercase();
                if !b.title.to_lowercase().contains(&search_lower)
                    && !b.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&search_lower))
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

    let responses: Vec<BugResponse> = paginated.into_iter().map(BugResponse::from_db_bug).collect();
    Ok(response::paginated(responses, page, per_page, total))
}

/// Get a single bug by ID
///
/// GET /api/v1/bugs/:id
pub async fn get_bug(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    let bug = BugRepository::get_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Bug not found: {}", id)))?;

    Ok(response::ok(BugResponse::from_db_bug(bug)))
}

/// Update an existing bug
///
/// PUT /api/v1/bugs/:id
pub async fn update_bug(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateBugRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    if !req.has_updates() {
        return Err(ApiError::BadRequest("No fields to update".to_string()));
    }

    let pool = app_state.db.pool();
    let mut bug = BugRepository::get_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Bug not found: {}", id)))?;

    // Update status if provided
    if let Some(status) = &req.status {
        BugRepository::update_status(pool, &id, status)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        bug.status = status.clone();
    }

    // Update assignee if provided
    if let Some(assignee) = &req.assignee {
        BugRepository::update_assignee(pool, &id, assignee)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        bug.assignee = Some(assignee.clone());
    }

    // Update other fields in memory
    if let Some(title) = req.title {
        bug.title = title;
    }
    if let Some(description) = req.description {
        bug.description = Some(description);
    }
    if let Some(severity) = req.severity {
        bug.severity = severity;
    }
    if let Some(labels) = req.labels {
        bug.labels = Some(labels);
    }

    bug.updated_at = chrono::Utc::now().to_rfc3339();

    tracing::info!("Updated bug: {}", bug.id);
    Ok(response::ok(BugResponse::from_db_bug(bug)))
}

/// Delete a bug
///
/// DELETE /api/v1/bugs/:id
pub async fn delete_bug(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    BugRepository::delete(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Deleted bug: {}", id);
    Ok(StatusCode::NO_CONTENT)
}

/// Get bug statistics
///
/// GET /api/v1/bugs/stats
pub async fn get_bug_stats(
    State(app_state): State<crate::api::routes::AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let pool = app_state.db.pool();

    let total = BugRepository::count(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let open = BugRepository::count_by_status(pool, "open")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let in_progress = BugRepository::count_by_status(pool, "in_progress")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let resolved = BugRepository::count_by_status(pool, "resolved")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(response::ok(serde_json::json!({
        "total": total,
        "open": open,
        "in_progress": in_progress,
        "resolved": resolved
    })))
}
