//! Ollama streaming helper
//!
//! Ollama uses NDJSON (newline-delimited JSON) streaming format.
//! Each line is a complete JSON object with the response chunk.

use crate::error::{LlmError, Result};
use futures::stream::StreamExt;
use langgraph_core::llm_stream::{MessageChunk, MessageChunkStream};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Ollama streaming chunk format
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaStreamChunk {
    pub model: String,
    pub message: OllamaStreamMessage,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<usize>,
    #[serde(default)]
    pub eval_count: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaStreamMessage {
    pub role: String,
    pub content: String,
}

/// Stream from Ollama API endpoint
pub async fn stream_ollama(
    client: &reqwest::Client,
    url: &str,
    request_body: Value,
) -> Result<(MessageChunkStream, Option<MessageChunkStream>)> {
    debug!("Starting Ollama stream to {}", url);

    // Send the request
    let response = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(LlmError::HttpError)?;

    // Check for errors
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        return Err(LlmError::ProviderError(format!(
            "Ollama API error {}: {}",
            status, error_text
        )));
    }

    // Create channel for content stream (Ollama doesn't support reasoning streams)
    let (content_tx, content_rx) = mpsc::channel::<MessageChunk>(100);

    // Spawn task to process the NDJSON stream
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

                    // Process complete NDJSON lines
                    while let Some(line_end) = buffer.find('\n') {
                        let line = buffer[..line_end].trim().to_string();
                        buffer = buffer[line_end + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        // Parse JSON chunk
                        match serde_json::from_str::<OllamaStreamChunk>(&line) {
                            Ok(chunk) => {
                                chunk_count += 1;

                                // Send content if not empty
                                if !chunk.message.content.is_empty() {
                                    let msg_chunk = if chunk.done {
                                        MessageChunk::new(chunk.message.content)
                                            .with_message_id(format!("ollama_{}", chunk_count))
                                            .final_chunk()
                                    } else {
                                        MessageChunk::new(chunk.message.content)
                                            .with_message_id(format!("ollama_{}", chunk_count))
                                    };

                                    if content_tx.send(msg_chunk).await.is_err() {
                                        return; // Receiver dropped
                                    }
                                }

                                // Check if stream is done
                                if chunk.done {
                                    debug!("Ollama stream complete after {} chunks", chunk_count);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse Ollama NDJSON chunk: {} - line: {}", e, line);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Error reading Ollama stream: {}", e);
                    break;
                }
            }
        }

        debug!("Ollama stream processing complete");
    });

    // Convert channel to stream
    let content_stream: MessageChunkStream =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(content_rx));

    // Ollama doesn't have a separate reasoning stream
    Ok((content_stream, None))
}
