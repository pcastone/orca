//! Gemini (Google) streaming helper
//!
//! Gemini uses server-side streaming with JSON responses.
//! The streaming endpoint is `streamGenerateContent` and returns
//! JSON arrays with partial responses.

use crate::error::{LlmError, Result};
use futures::stream::StreamExt;
use langgraph_core::llm_stream::{MessageChunk, MessageChunkStream};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Gemini streaming response format
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiStreamChunk {
    pub candidates: Vec<GeminiStreamCandidate>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeminiStreamCandidate {
    pub content: GeminiStreamContent,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeminiStreamContent {
    pub parts: Vec<GeminiStreamPart>,
    pub role: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeminiStreamPart {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: usize,
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: usize,
}

/// Stream from Gemini API endpoint
pub async fn stream_gemini(
    client: &reqwest::Client,
    url: &str,
    request_body: Value,
    api_key: &str,
) -> Result<(MessageChunkStream, Option<MessageChunkStream>)> {
    debug!("Starting Gemini stream to {}", url);

    // Send the request with API key as query parameter
    let response = client
        .post(url)
        .query(&[("key", api_key), ("alt", "sse")])
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(LlmError::HttpError)?;

    // Check for errors
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        return Err(if status.as_u16() == 401 || status.as_u16() == 403 {
            LlmError::AuthenticationError(error_text)
        } else if status.as_u16() == 429 {
            LlmError::RateLimitExceeded(error_text)
        } else {
            LlmError::ProviderError(format!("Gemini API error {}: {}", status, error_text))
        });
    }

    // Create channel for content stream (Gemini doesn't support reasoning streams)
    let (content_tx, content_rx) = mpsc::channel::<MessageChunk>(100);

    // Spawn task to process the stream
    let byte_stream = response.bytes_stream();
    tokio::spawn(async move {
        let mut buffer = String::new();
        let mut stream = byte_stream;
        let mut chunk_count = 0u64;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        buffer.push_str(&text);
                    }

                    // Process complete SSE lines
                    while let Some(line_end) = buffer.find('\n') {
                        let line = buffer[..line_end].trim().to_string();
                        buffer = buffer[line_end + 1..].to_string();

                        // Skip empty lines and event type lines
                        if line.is_empty() || line.starts_with("event:") {
                            continue;
                        }

                        // Handle data lines
                        if let Some(data) = line.strip_prefix("data: ") {
                            // Skip [DONE] marker if present
                            if data.trim() == "[DONE]" {
                                debug!("Gemini stream completed with [DONE]");
                                break;
                            }

                            // Parse JSON chunk
                            match serde_json::from_str::<GeminiStreamChunk>(data) {
                                Ok(chunk) => {
                                    chunk_count += 1;

                                    for candidate in &chunk.candidates {
                                        // Extract text from parts
                                        let text: String = candidate
                                            .content
                                            .parts
                                            .iter()
                                            .map(|p| p.text.as_str())
                                            .collect();

                                        if !text.is_empty() {
                                            let is_final = candidate.finish_reason.is_some();
                                            let msg_chunk = if is_final {
                                                MessageChunk::new(text)
                                                    .with_message_id(format!(
                                                        "gemini_{}",
                                                        chunk_count
                                                    ))
                                                    .final_chunk()
                                            } else {
                                                MessageChunk::new(text)
                                                    .with_message_id(format!(
                                                        "gemini_{}",
                                                        chunk_count
                                                    ))
                                            };

                                            if content_tx.send(msg_chunk).await.is_err() {
                                                return; // Receiver dropped
                                            }
                                        }

                                        if candidate.finish_reason.is_some() {
                                            debug!(
                                                "Gemini stream ending: {:?}",
                                                candidate.finish_reason
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to parse Gemini SSE chunk: {} - data: {}",
                                        e, data
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Error reading Gemini stream: {}", e);
                    break;
                }
            }
        }

        debug!("Gemini stream processing complete after {} chunks", chunk_count);
    });

    // Convert channel to stream
    let content_stream: MessageChunkStream =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(content_rx));

    // Gemini doesn't have a separate reasoning stream
    Ok((content_stream, None))
}
