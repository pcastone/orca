//! Checkpoint endpoint handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::api::{
    error::{ApiError, ApiResult},
    models::checkpoint::{CreateCheckpointRequest, CheckpointResponse, CheckpointListQuery},
    response,
};
use crate::db::repositories::CheckpointRepository;

/// Create a new checkpoint
///
/// POST /api/v1/checkpoints
pub async fn create_checkpoint(
    State(app_state): State<crate::api::routes::AppState>,
    Json(req): Json<CreateCheckpointRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    req.validate()?;

    let pool = app_state.db.pool();
    let checkpoint_id = Uuid::new_v4().to_string();
    let superstep = req.superstep.unwrap_or(0);

    let created = CheckpointRepository::create(
        pool,
        checkpoint_id,
        req.execution_id,
        req.workflow_id,
        req.state,
        req.node_id,
        superstep,
        req.parent_checkpoint_id,
        req.metadata,
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Created checkpoint: {}", created.id);
    Ok((StatusCode::CREATED, Json(CheckpointResponse::from_db_checkpoint(created))))
}

/// List all checkpoints with filtering and pagination
///
/// GET /api/v1/checkpoints
pub async fn list_checkpoints(
    State(app_state): State<crate::api::routes::AppState>,
    Query(query): Query<CheckpointListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(0);
    let per_page = query.per_page.unwrap_or(20);

    crate::api::middleware::validation::validate_pagination(page, per_page, 100)?;

    let pool = app_state.db.pool();
    let checkpoints = CheckpointRepository::list(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Apply filtering
    let filtered: Vec<_> = checkpoints
        .into_iter()
        .filter(|c| {
            if let Some(ref execution_id) = query.execution_id {
                if c.execution_id != *execution_id {
                    return false;
                }
            }
            if let Some(ref workflow_id) = query.workflow_id {
                if c.workflow_id != *workflow_id {
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

    let responses: Vec<CheckpointResponse> = paginated
        .into_iter()
        .map(CheckpointResponse::from_db_checkpoint)
        .collect();
    Ok(response::paginated(responses, page, per_page, total))
}

/// Get a single checkpoint by ID
///
/// GET /api/v1/checkpoints/:id
pub async fn get_checkpoint(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    let checkpoint = CheckpointRepository::get_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Checkpoint not found: {}", id)))?;

    Ok(response::ok(CheckpointResponse::from_db_checkpoint(checkpoint)))
}

/// Delete a checkpoint
///
/// DELETE /api/v1/checkpoints/:id
pub async fn delete_checkpoint(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    CheckpointRepository::delete(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Deleted checkpoint: {}", id);
    Ok(StatusCode::NO_CONTENT)
}

/// Get checkpoints for a specific execution
///
/// GET /api/v1/executions/:execution_id/checkpoints
pub async fn list_execution_checkpoints(
    State(app_state): State<crate::api::routes::AppState>,
    Path(execution_id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let pool = app_state.db.pool();
    let checkpoints = CheckpointRepository::list_by_execution(pool, &execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let responses: Vec<CheckpointResponse> = checkpoints
        .into_iter()
        .map(CheckpointResponse::from_db_checkpoint)
        .collect();

    Ok(response::ok(responses))
}

/// Get the latest checkpoint for an execution
///
/// GET /api/v1/executions/:execution_id/checkpoints/latest
pub async fn get_latest_checkpoint(
    State(app_state): State<crate::api::routes::AppState>,
    Path(execution_id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let pool = app_state.db.pool();
    let checkpoint = CheckpointRepository::get_latest_for_execution(pool, &execution_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("No checkpoints found for execution: {}", execution_id)))?;

    Ok(response::ok(CheckpointResponse::from_db_checkpoint(checkpoint)))
}
