//! Tool execution endpoint handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::api::{
    error::{ApiError, ApiResult},
    models::{ExecuteToolRequest, ToolExecutionResponse, ExecutionListQuery},
    response,
};
use crate::db::{DatabaseConnection, repositories::ToolExecutionRepository};

/// Execute a tool for a given task
///
/// POST /api/v1/tasks/:task_id/execute
pub async fn execute_tool(
    State(app_state): State<crate::api::routes::AppState>,
    Path(task_id): Path<String>,
    Json(req): Json<ExecuteToolRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&task_id)?;
    req.validate()?;

    let pool = app_state.db.pool();
    let execution_id = Uuid::new_v4().to_string();

    let created = ToolExecutionRepository::create(
        pool,
        execution_id,
        task_id,
        req.tool_name,
        req.arguments,
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    tracing::info!("Created tool execution: {}", created.id);
    Ok((StatusCode::CREATED, Json(ToolExecutionResponse::from_db_execution(created))))
}

/// List executions for a task
///
/// GET /api/v1/tasks/:task_id/executions
pub async fn list_task_executions(
    State(app_state): State<crate::api::routes::AppState>,
    Path(task_id): Path<String>,
    Query(query): Query<ExecutionListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&task_id)?;

    let page = query.page.unwrap_or(0);
    let per_page = query.per_page.unwrap_or(20);

    crate::api::middleware::validation::validate_pagination(page, per_page, 100)?;

    let pool = app_state.db.pool();
    let executions = ToolExecutionRepository::list_by_task(pool, &task_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = executions.len() as u32;
    let offset = (page as usize) * (per_page as usize);
    let paginated: Vec<_> = executions
        .into_iter()
        .skip(offset)
        .take(per_page as usize)
        .collect();

    let responses: Vec<ToolExecutionResponse> = paginated
        .into_iter()
        .map(ToolExecutionResponse::from_db_execution)
        .collect();

    Ok(response::paginated(responses, page, per_page, total))
}

/// Get all executions with filtering
///
/// GET /api/v1/executions
pub async fn list_executions(
    State(app_state): State<crate::api::routes::AppState>,
    Query(query): Query<ExecutionListQuery>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = query.page.unwrap_or(0);
    let per_page = query.per_page.unwrap_or(20);

    crate::api::middleware::validation::validate_pagination(page, per_page, 100)?;

    let pool = app_state.db.pool();

    // Get executions - use list_by_status if status is provided, otherwise get all by other means
    let mut executions = if let Some(ref status) = query.status {
        ToolExecutionRepository::list_by_status(pool, status)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else if let Some(ref tool_name) = query.tool_name {
        ToolExecutionRepository::list_by_tool(pool, tool_name)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        // Fallback: try to get a reasonable default list
        // For now, return empty list since we don't have a generic list method
        vec![]
    };

    // Apply additional filtering
    if query.status.is_none() && query.tool_name.is_none() {
        // No filters, so we got empty vec - this is okay for now
    }

    let total = executions.len() as u32;
    let offset = (page as usize) * (per_page as usize);
    let paginated: Vec<_> = executions
        .drain(offset..)
        .take(per_page as usize)
        .collect();

    let responses: Vec<ToolExecutionResponse> = paginated
        .into_iter()
        .map(ToolExecutionResponse::from_db_execution)
        .collect();

    Ok(response::paginated(responses, page, per_page, total))
}

/// Get a single execution by ID
///
/// GET /api/v1/executions/:id
pub async fn get_execution(
    State(app_state): State<crate::api::routes::AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl axum::response::IntoResponse> {
    crate::api::middleware::validation::validate_uuid(&id)?;

    let pool = app_state.db.pool();
    let execution = ToolExecutionRepository::get_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Execution not found: {}", id)))?;

    Ok(response::ok(ToolExecutionResponse::from_db_execution(execution)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_tool_request_validation() {
        let req = ExecuteToolRequest {
            tool_name: "shell_exec".to_string(),
            arguments: "{}".to_string(),
        };
        assert!(req.validate().is_ok());
    }
}
