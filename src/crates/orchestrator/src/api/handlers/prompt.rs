//! LLM prompt endpoint handler
//!
//! Provides the endpoint for sending prompts to LLM providers.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::api::{models::{PromptRequest, PromptResponse}, response, routes::AppState};
use crate::services::PromptError;

/// Handler for POST /api/v1/prompt
///
/// Sends a prompt to the configured LLM and returns the response.
pub async fn send_prompt(
    State(app_state): State<AppState>,
    Json(request): Json<PromptRequest>,
) -> Response {
    // Check if prompt service is available
    let prompt_service = match &app_state.prompt_service {
        Some(service) => service,
        None => {
            return response::bad_request("LLM prompt service not configured").into_response();
        }
    };

    // Validate prompt
    if request.prompt.trim().is_empty() {
        return response::bad_request("Prompt cannot be empty").into_response();
    }

    // Send prompt to LLM
    match prompt_service.send_prompt(&request.prompt).await {
        Ok(response_text) => {
            let resp = PromptResponse {
                response: response_text,
            };
            response::ok(resp).into_response()
        }
        Err(e) => match e {
            PromptError::EmptyPrompt => response::bad_request("Prompt cannot be empty").into_response(),
            PromptError::NotConfigured => response::bad_request("LLM not configured").into_response(),
            PromptError::UnsupportedProvider(p) => {
                response::bad_request(format!("Unsupported LLM provider: {}", p)).into_response()
            }
            PromptError::MissingApiKey(p) => {
                response::bad_request(format!("Missing API key for provider: {}", p)).into_response()
            }
            PromptError::LlmError(msg) => response::internal_error(format!("LLM error: {}", msg)).into_response(),
        },
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_prompt_handler_exists() {
        // Basic test to ensure handler compiles
        assert!(true);
    }
}
