//! Prompt history endpoint handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::api::{
    error::{ApiError, ApiResult},
    models::prompt_history::{CreatePromptHistoryRequest, PromptHistoryResponse, PromptHistoryListQuery, PromptHistoryStatsResponse},
    response,
};
use crate::db::repositories::PromptHistoryRepository;

/// Create a new prompt history entry
///
/// POST /api/v1/prompts
pub async fn create_prompt_history(
    State(app_state): State<crate::api::routes::AppState>,
    Json(req): Json<CreatePromptHistoryRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    req.validate()?;

    let pool = app_state.db.pool();
    let prompt_id = Uuid::new_v4().to_string();

    let created = PromptHistoryRepository::create(
        pool,
        prompt_id,
        req.provider,
        req.model,
        req.user_prompt,
        req.system_prompt,
        req.assistant_response,
        req.task_id,
        req.workflow_id,
        req.execution_id,
        req.session_id,
        req.input_tokens,
        req.output_tokens,
        req.cost_usd,
        req.latency_ms,
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Created prompt history: {}", created.id);
    Ok((StatusCode::CREATED, Json(PromptHistoryResponse::from_db_prompt(created))))
}

/// List all prompt history with filtering and pagination
///
/// GET /api/v1/prompts
pub async fn list_prompt_history(
    State(app_state): State<crate::api::routes::AppState>,
    Query(query): Query<PromptHistoryListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(0);
    let per_page = query.per_page.unwrap_or(20);

    crate::api::middleware::validation::validate_pagination(page, per_page, 100)?;

    let pool = app_state.db.pool();
    let prompts = PromptHistoryRepository::list(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Apply filtering
    let filtered: Vec<_> = prompts
        .into_iter()
        .filter(|p| {
            if let Some(ref task_id) = query.task_id {
                if p.task_id.as_ref() != Some(task_id) {
                    return false;
                }
            }
            if let Some(ref workflow_id) = query.workflow_id {
                if p.workflow_id.as_ref() != Some(workflow_id) {
                    return false;
                }
            }
            if let Some(ref execution_id) = query.execution_id {
                if p.execution_id.as_ref() != Some(execution_id) {
                    return false;
                }
            }
            if let Some(ref session_id) = query.session_id {
                if p.session_id.as_ref() != Some(session_id) {
                    return false;
                }
            }
            if let Some(ref provider) = query.provider {
                if p.provider != *provider {
                    return false;
                }
            }
            if let Some(ref model) = query.model {
                if p.model != *model {
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

    let responses: Vec<PromptHistoryResponse> = paginated
        .into_iter()
        .map(PromptHistoryResponse::from_db_prompt)
        .collect();
    Ok(response::paginated(responses, page, per_page, total))
}

/// Get a single prompt history entry by ID
///
/// GET /api/v1/prompts/:id
pub async fn get_prompt_history(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    let prompt = PromptHistoryRepository::get_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Prompt history not found: {}", id)))?;

    Ok(response::ok(PromptHistoryResponse::from_db_prompt(prompt)))
}

/// Delete a prompt history entry
///
/// DELETE /api/v1/prompts/:id
pub async fn delete_prompt_history(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    PromptHistoryRepository::delete(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Deleted prompt history: {}", id);
    Ok(StatusCode::NO_CONTENT)
}

/// Get prompt history statistics
///
/// GET /api/v1/prompts/stats
pub async fn get_prompt_stats(
    State(app_state): State<crate::api::routes::AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let pool = app_state.db.pool();

    let total_prompts = PromptHistoryRepository::count(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total_tokens = PromptHistoryRepository::get_total_tokens(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total_cost_usd = PromptHistoryRepository::get_total_cost(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(response::ok(PromptHistoryStatsResponse {
        total_prompts,
        total_tokens,
        total_cost_usd,
    }))
}

/// Get prompt history for a specific task
///
/// GET /api/v1/tasks/:task_id/prompts
pub async fn list_task_prompts(
    State(app_state): State<crate::api::routes::AppState>,
    Path(task_id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&task_id)?;

    let pool = app_state.db.pool();
    let prompts = PromptHistoryRepository::list_by_task(pool, &task_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let responses: Vec<PromptHistoryResponse> = prompts
        .into_iter()
        .map(PromptHistoryResponse::from_db_prompt)
        .collect();

    Ok(response::ok(responses))
}

/// Get prompt history for a specific session
///
/// GET /api/v1/sessions/:session_id/prompts
pub async fn list_session_prompts(
    State(app_state): State<crate::api::routes::AppState>,
    Path(session_id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let pool = app_state.db.pool();
    let prompts = PromptHistoryRepository::list_by_session(pool, &session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let responses: Vec<PromptHistoryResponse> = prompts
        .into_iter()
        .map(PromptHistoryResponse::from_db_prompt)
        .collect();

    Ok(response::ok(responses))
}
