//! OpenAI-compatible streaming helper
//!
//! This module provides streaming support for providers that use the OpenAI API format:
//! - OpenAI
//! - Deepseek
//! - Grok
//! - OpenRouter
//! - LM Studio
//! - llama.cpp
//!
//! All these providers use Server-Sent Events (SSE) with the same JSON structure.

use crate::error::{LlmError, Result};
use futures::stream::StreamExt;
use langgraph_core::llm_stream::{MessageChunk, MessageChunkStream};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// OpenAI streaming chunk format
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiStreamChunk {
    pub id: String,
    #[serde(default)]
    pub choices: Vec<StreamChoice>,
    pub model: String,
    #[serde(default)]
    pub usage: Option<StreamUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamChoice {
    pub delta: StreamDelta,
    pub index: usize,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct StreamDelta {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Configuration for OpenAI-compatible streaming
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Base URL for the API
    pub base_url: String,
    /// Model name
    pub model: String,
    /// API key (if required)
    pub api_key: Option<String>,
    /// Additional headers
    pub extra_headers: Vec<(String, String)>,
}

/// Stream from an OpenAI-compatible API endpoint
///
/// This function handles:
/// - Building the HTTP request with proper headers
/// - Parsing Server-Sent Events (SSE) from the response
/// - Converting SSE data to MessageChunk stream
/// - Handling reasoning content for thinking models (Deepseek R1, etc.)
///
/// # Arguments
/// * `client` - The reqwest HTTP client
/// * `url` - Full URL to the chat completions endpoint
/// * `request_body` - JSON request body (must have "stream": true)
/// * `headers` - Additional headers to include
///
/// # Returns
/// A tuple of (content_stream, optional_reasoning_stream)
pub async fn stream_openai_compatible(
    client: &reqwest::Client,
    url: &str,
    request_body: Value,
    headers: Vec<(&str, &str)>,
) -> Result<(MessageChunkStream, Option<MessageChunkStream>)> {
    // Build headers
    let mut header_map = HeaderMap::new();
    header_map.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    for (key, value) in headers {
        let header_name = HeaderName::try_from(key)
            .map_err(|e| LlmError::Other(format!("Invalid header name: {}", e)))?;
        let header_value = HeaderValue::try_from(value)
            .map_err(|e| LlmError::Other(format!("Invalid header value: {}", e)))?;
        header_map.insert(header_name, header_value);
    }

    debug!("Starting OpenAI-compatible stream to {}", url);

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
            LlmError::ProviderError(format!("API error {}: {}", status, error_text))
        });
    }

    // Create channels for content and reasoning streams
    let (content_tx, content_rx) = mpsc::channel::<MessageChunk>(100);
    let (reasoning_tx, reasoning_rx) = mpsc::channel::<MessageChunk>(100);

    // Track if we've seen any reasoning content
    let reasoning_tx_clone = reasoning_tx.clone();
    let has_reasoning = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let has_reasoning_clone = has_reasoning.clone();

    // Spawn task to process the SSE stream
    let byte_stream = response.bytes_stream();
    tokio::spawn(async move {
        let mut buffer = String::new();
        let mut stream = byte_stream;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    // Append to buffer
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
                            // Check for stream end
                            if data.trim() == "[DONE]" {
                                debug!("Stream completed with [DONE]");
                                break;
                            }

                            // Parse JSON chunk
                            match serde_json::from_str::<OpenAiStreamChunk>(data) {
                                Ok(chunk) => {
                                    for choice in &chunk.choices {
                                        // Handle content
                                        if let Some(ref content) = choice.delta.content {
                                            if !content.is_empty() {
                                                let is_final =
                                                    choice.finish_reason.is_some();
                                                let msg_chunk = if is_final {
                                                    MessageChunk::new(content.clone())
                                                        .with_message_id(&chunk.id)
                                                        .final_chunk()
                                                } else {
                                                    MessageChunk::new(content.clone())
                                                        .with_message_id(&chunk.id)
                                                };

                                                if content_tx.send(msg_chunk).await.is_err() {
                                                    return; // Receiver dropped
                                                }
                                            }
                                        }

                                        // Handle reasoning content (Deepseek R1, etc.)
                                        if let Some(ref reasoning) = choice.delta.reasoning_content
                                        {
                                            if !reasoning.is_empty() {
                                                has_reasoning_clone.store(
                                                    true,
                                                    std::sync::atomic::Ordering::SeqCst,
                                                );
                                                let msg_chunk =
                                                    MessageChunk::new(reasoning.clone())
                                                        .with_message_id(format!(
                                                            "{}_reasoning",
                                                            chunk.id
                                                        ));

                                                if reasoning_tx_clone.send(msg_chunk).await.is_err()
                                                {
                                                    return; // Receiver dropped
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to parse SSE chunk: {} - data: {}", e, data);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Error reading stream: {}", e);
                    break;
                }
            }
        }

        debug!("Stream processing complete");
    });

    // Convert channels to streams
    let content_stream: MessageChunkStream =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(content_rx));

    // Only return reasoning stream if we expect it might have content
    // For now, always return it - the consumer can check if it's empty
    let reasoning_stream: MessageChunkStream =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(reasoning_rx));

    Ok((content_stream, Some(reasoning_stream)))
}

/// Build a standard OpenAI-compatible request body
pub fn build_openai_request(
    model: &str,
    messages: &[Value],
    temperature: Option<f32>,
    max_tokens: Option<usize>,
    stream: bool,
) -> Value {
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": stream,
    });

    if let Some(temp) = temperature {
        body["temperature"] = serde_json::json!(temp);
    }

    if let Some(max) = max_tokens {
        body["max_tokens"] = serde_json::json!(max);
    }

    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stream_chunk() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [{
                "delta": {"content": "Hello"},
                "index": 0,
                "finish_reason": null
            }]
        }"#;

        let chunk: OpenAiStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.id, "chatcmpl-123");
        assert_eq!(chunk.model, "gpt-4");
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_parse_stream_chunk_with_reasoning() {
        let json = r#"{
            "id": "chatcmpl-456",
            "model": "deepseek-reasoner",
            "choices": [{
                "delta": {
                    "content": null,
                    "reasoning_content": "Let me think..."
                },
                "index": 0,
                "finish_reason": null
            }]
        }"#;

        let chunk: OpenAiStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(
            chunk.choices[0].delta.reasoning_content,
            Some("Let me think...".to_string())
        );
    }

    #[test]
    fn test_parse_final_chunk() {
        let json = r#"{
            "id": "chatcmpl-789",
            "model": "gpt-4",
            "choices": [{
                "delta": {"content": ""},
                "index": 0,
                "finish_reason": "stop"
            }]
        }"#;

        let chunk: OpenAiStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices[0].finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_build_openai_request() {
        let messages = vec![serde_json::json!({"role": "user", "content": "Hello"})];
        let body = build_openai_request("gpt-4", &messages, Some(0.7), Some(100), true);

        assert_eq!(body["model"], "gpt-4");
        assert_eq!(body["stream"], true);
        assert_eq!(body["temperature"], 0.7);
        assert_eq!(body["max_tokens"], 100);
    }
}
