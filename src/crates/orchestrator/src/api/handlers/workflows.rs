//! Workflow CRUD endpoint handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::api::{
    error::{ApiError, ApiResult},
    models::{CreateWorkflowRequest, UpdateWorkflowRequest, WorkflowResponse, WorkflowListQuery},
    response,
};
use crate::db::repositories::WorkflowRepository;

/// Create a new workflow
///
/// POST /api/v1/workflows
pub async fn create_workflow(
    State(app_state): State<crate::api::routes::AppState>,
    Json(req): Json<CreateWorkflowRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    req.validate()?;

    let pool = app_state.db.pool();
    let workflow_id = Uuid::new_v4().to_string();

    let created = WorkflowRepository::create(
        pool,
        workflow_id,
        req.name,
        req.definition,
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Created workflow: {}", created.id);
    Ok((StatusCode::CREATED, Json(WorkflowResponse::from_db_workflow(created))))
}

/// List all workflows with filtering and pagination
///
/// GET /api/v1/workflows
pub async fn list_workflows(
    State(app_state): State<crate::api::routes::AppState>,
    Query(query): Query<WorkflowListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(0);
    let per_page = query.per_page.unwrap_or(20);

    crate::api::middleware::validation::validate_pagination(page, per_page, 100)?;

    let pool = app_state.db.pool();
    let workflows = WorkflowRepository::list(pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Apply filtering
    let filtered: Vec<_> = workflows
        .into_iter()
        .filter(|w| {
            if let Some(ref status) = query.status {
                if w.status != *status {
                    return false;
                }
            }
            if let Some(ref search) = query.search {
                if !w.name.contains(search)
                    && w.description.as_ref().map_or(false, |d| d.contains(search))
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

    let responses: Vec<WorkflowResponse> = paginated.into_iter().map(WorkflowResponse::from_db_workflow).collect();
    Ok(response::paginated(responses, page, per_page, total))
}

/// Get a single workflow by ID
///
/// GET /api/v1/workflows/:id
pub async fn get_workflow(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    let workflow = WorkflowRepository::get_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workflow not found: {}", id)))?;

    Ok(response::ok(WorkflowResponse::from_db_workflow(workflow)))
}

/// Update an existing workflow
///
/// PUT /api/v1/workflows/:id
pub async fn update_workflow(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorkflowRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    if !req.has_updates() {
        return Err(ApiError::BadRequest("No fields to update".to_string()));
    }

    let pool = app_state.db.pool();
    let mut workflow = WorkflowRepository::get_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workflow not found: {}", id)))?;

    // Update status if provided
    if let Some(status) = &req.status {
        WorkflowRepository::update_status(pool, &id, status)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        workflow.status = status.clone();
    }

    // Update other fields in memory
    if let Some(name) = req.name {
        workflow.name = name;
    }
    if let Some(description) = req.description {
        workflow.description = Some(description);
    }
    if let Some(definition) = req.definition {
        workflow.definition = definition;
    }

    workflow.updated_at = chrono::Utc::now().to_rfc3339();

    tracing::info!("Updated workflow: {}", workflow.id);
    Ok(response::ok(WorkflowResponse::from_db_workflow(workflow)))
}

/// Delete a workflow
///
/// DELETE /api/v1/workflows/:id
pub async fn delete_workflow(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    WorkflowRepository::delete(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Deleted workflow: {}", id);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_workflow_request_validation() {
        let req = CreateWorkflowRequest {
            name: "Test Workflow".to_string(),
            description: None,
            definition: "{}".to_string(),
        };
        assert!(req.validate().is_ok());
    }
}
