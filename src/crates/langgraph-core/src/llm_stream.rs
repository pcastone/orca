//! LLM streaming utilities for token-level output
//!
//! This module provides utilities for streaming LLM outputs token-by-token,
//! which is essential for modern chat applications and agent workflows.

use crate::stream::StreamEvent;
use futures::stream::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Type alias for a stream of string chunks (tokens)
pub type TokenStream = Pin<Box<dyn Stream<Item = String> + Send>>;

/// Type alias for a stream of message chunks with metadata
pub type MessageChunkStream = Pin<Box<dyn Stream<Item = MessageChunk> + Send>>;

/// A chunk of a streaming message
#[derive(Debug, Clone)]
pub struct MessageChunk {
    /// The content chunk (token or partial message)
    pub content: String,

    /// Optional message ID this chunk belongs to
    pub message_id: Option<String>,

    /// Whether this is the final chunk
    pub is_final: bool,

    /// Optional metadata (model name, finish_reason, etc.)
    pub metadata: Option<Value>,
}

impl MessageChunk {
    /// Create a new message chunk
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            message_id: None,
            is_final: false,
            metadata: None,
        }
    }

    /// Create a message chunk with metadata
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Mark this chunk as the final chunk
    pub fn final_chunk(mut self) -> Self {
        self.is_final = true;
        self
    }

    /// Set the message ID
    pub fn with_message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    /// Convert to a StreamEvent for emission
    pub fn to_stream_event(&self, node: impl Into<String>) -> StreamEvent {
        StreamEvent::message_chunk_with_metadata(
            node.into(),
            self.content.clone(),
            self.message_id.clone(),
            self.metadata.clone(),
        )
    }
}

/// Token buffer for accumulating streaming tokens
///
/// This helps manage token streaming by buffering chunks and providing
/// utilities for reconstructing complete messages.
#[derive(Debug, Default)]
pub struct TokenBuffer {
    /// Accumulated content
    buffer: String,

    /// Number of chunks received
    chunk_count: usize,

    /// Whether the stream has finished
    finished: bool,
}

impl TokenBuffer {
    /// Create a new token buffer
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a chunk to the buffer
    pub fn add_chunk(&mut self, chunk: &str) {
        self.buffer.push_str(chunk);
        self.chunk_count += 1;
    }

    /// Mark the stream as finished
    pub fn finish(&mut self) {
        self.finished = true;
    }

    /// Get the current buffer content
    pub fn content(&self) -> &str {
        &self.buffer
    }

    /// Get the number of chunks received
    pub fn chunk_count(&self) -> usize {
        self.chunk_count
    }

    /// Check if the stream is finished
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.chunk_count = 0;
        self.finished = false;
    }

    /// Consume the buffer and return the complete content
    pub fn into_string(self) -> String {
        self.buffer
    }
}

/// Stream adapter for converting token streams to message chunk streams
pub struct TokenStreamAdapter {
    node_id: String,
    message_id: Option<String>,
}

impl TokenStreamAdapter {
    /// Create a new token stream adapter
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            message_id: None,
        }
    }

    /// Set the message ID for all chunks
    pub fn with_message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    /// Adapt a token stream to a stream of StreamEvents
    pub fn adapt(
        self,
        token_stream: TokenStream,
    ) -> Pin<Box<dyn Stream<Item = StreamEvent> + Send>> {
        let node_id = self.node_id.clone();
        let message_id = self.message_id.clone();

        Box::pin(token_stream.map(move |chunk| {
            StreamEvent::message_chunk_with_metadata(
                node_id.clone(),
                chunk,
                message_id.clone(),
                None,
            )
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[test]
    fn test_message_chunk_creation() {
        let chunk = MessageChunk::new("Hello")
            .with_message_id("msg_123")
            .final_chunk();

        assert_eq!(chunk.content, "Hello");
        assert_eq!(chunk.message_id, Some("msg_123".to_string()));
        assert!(chunk.is_final);
    }

    #[test]
    fn test_token_buffer() {
        let mut buffer = TokenBuffer::new();

        buffer.add_chunk("Hello");
        buffer.add_chunk(" ");
        buffer.add_chunk("world");

        assert_eq!(buffer.content(), "Hello world");
        assert_eq!(buffer.chunk_count(), 3);
        assert!(!buffer.is_finished());

        buffer.finish();
        assert!(buffer.is_finished());

        let content = buffer.into_string();
        assert_eq!(content, "Hello world");
    }

    #[tokio::test]
    async fn test_token_stream_adapter() {
        let tokens = vec!["Hello".to_string(), " ".to_string(), "world".to_string()];
        let token_stream: TokenStream = Box::pin(stream::iter(tokens));

        let adapter = TokenStreamAdapter::new("llm_node")
            .with_message_id("msg_456");

        let mut event_stream = adapter.adapt(token_stream);

        let mut chunks = Vec::new();
        while let Some(event) = event_stream.next().await {
            if let StreamEvent::MessageChunk { chunk, message_id, node, .. } = event {
                assert_eq!(node, "llm_node");
                assert_eq!(message_id, Some("msg_456".to_string()));
                chunks.push(chunk);
            }
        }

        assert_eq!(chunks, vec!["Hello", " ", "world"]);
    }
}
