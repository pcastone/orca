//! Claude (Anthropic) streaming helper
//!
//! Claude uses Server-Sent Events (SSE) with custom event types:
//! - message_start: Initial message metadata
//! - content_block_start: Start of a content block (text or thinking)
//! - content_block_delta: Content chunk
//! - content_block_stop: End of content block
//! - message_delta: Final message metadata
//! - message_stop: Stream complete

use crate::error::{LlmError, Result};
use futures::stream::StreamExt;
use langgraph_core::llm_stream::{MessageChunk, MessageChunkStream};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Claude SSE event types
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: ClaudeMessageStart },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ClaudeContentBlockStart,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: usize,
        delta: ClaudeContentDelta,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: ClaudeMessageDeltaContent },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: ClaudeStreamError },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeMessageStart {
    pub id: String,
    pub model: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeContentBlockStart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeMessageDeltaContent {
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeStreamError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

/// Stream from Claude API endpoint
pub async fn stream_claude(
    client: &reqwest::Client,
    url: &str,
    request_body: Value,
    api_key: &str,
    anthropic_version: &str,
) -> Result<(MessageChunkStream, Option<MessageChunkStream>)> {
    // Build headers
    let mut header_map = HeaderMap::new();
    header_map.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    header_map.insert(
        HeaderName::try_from("x-api-key").unwrap(),
        HeaderValue::try_from(api_key).map_err(|e| LlmError::Other(format!("Invalid API key: {}", e)))?,
    );
    header_map.insert(
        HeaderName::try_from("anthropic-version").unwrap(),
        HeaderValue::try_from(anthropic_version).map_err(|e| LlmError::Other(format!("Invalid version: {}", e)))?,
    );

    debug!("Starting Claude stream to {}", url);

    // Send the request
    let response = client
        .post(url)
        .headers(header_map)
        .json(&request_body)
        .send()
        .await
        .map_err(LlmError::HttpError)?;

    // Check for errors
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        return Err(if status.as_u16() == 401 {
            LlmError::AuthenticationError(error_text)
        } else if status.as_u16() == 429 {
            LlmError::RateLimitExceeded(error_text)
        } else {
            LlmError::ProviderError(format!("Claude API error {}: {}", status, error_text))
        });
    }

    // Create channels for content and reasoning streams
    let (content_tx, content_rx) = mpsc::channel::<MessageChunk>(100);
    let (reasoning_tx, reasoning_rx) = mpsc::channel::<MessageChunk>(100);

    // Track current content block type
    let reasoning_tx_clone = reasoning_tx.clone();

    // Spawn task to process the SSE stream
    let byte_stream = response.bytes_stream();
    tokio::spawn(async move {
        let mut buffer = String::new();
        let mut stream = byte_stream;
        let mut message_id = String::new();
        let mut current_block_is_thinking = false;

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
                            match serde_json::from_str::<ClaudeStreamEvent>(data) {
                                Ok(event) => match event {
                                    ClaudeStreamEvent::MessageStart { message } => {
                                        message_id = message.id;
                                        debug!("Claude stream started: {}", message_id);
                                    }
                                    ClaudeStreamEvent::ContentBlockStart { content_block, .. } => {
                                        current_block_is_thinking = matches!(
                                            content_block,
                                            ClaudeContentBlockStart::Thinking { .. }
                                        );
                                    }
                                    ClaudeStreamEvent::ContentBlockDelta { delta, .. } => {
                                        match delta {
                                            ClaudeContentDelta::TextDelta { text } => {
                                                if !text.is_empty() {
                                                    let chunk = MessageChunk::new(text)
                                                        .with_message_id(&message_id);
                                                    if content_tx.send(chunk).await.is_err() {
                                                        return;
                                                    }
                                                }
                                            }
                                            ClaudeContentDelta::ThinkingDelta { thinking } => {
                                                if !thinking.is_empty() {
                                                    let chunk = MessageChunk::new(thinking)
                                                        .with_message_id(format!(
                                                            "{}_reasoning",
                                                            message_id
                                                        ));
                                                    if reasoning_tx_clone.send(chunk).await.is_err()
                                                    {
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    ClaudeStreamEvent::ContentBlockStop { .. } => {
                                        current_block_is_thinking = false;
                                    }
                                    ClaudeStreamEvent::MessageDelta { delta } => {
                                        if delta.stop_reason.is_some() {
                                            debug!("Claude stream ending: {:?}", delta.stop_reason);
                                        }
                                    }
                                    ClaudeStreamEvent::MessageStop => {
                                        debug!("Claude stream complete");
                                        break;
                                    }
                                    ClaudeStreamEvent::Ping => {
                                        // Keep-alive, ignore
                                    }
                                    ClaudeStreamEvent::Error { error } => {
                                        warn!(
                                            "Claude stream error: {} - {}",
                                            error.error_type, error.message
                                        );
                                        break;
                                    }
                                },
                                Err(e) => {
                                    warn!("Failed to parse Claude SSE event: {} - data: {}", e, data);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Error reading Claude stream: {}", e);
                    break;
                }
            }
        }

        debug!("Claude stream processing complete");
    });

    // Convert channels to streams
    let content_stream: MessageChunkStream =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(content_rx));
    let reasoning_stream: MessageChunkStream =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(reasoning_rx));

    Ok((content_stream, Some(reasoning_stream)))
}
