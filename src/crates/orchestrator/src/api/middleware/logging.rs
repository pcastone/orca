//! Request logging middleware
//!
//! Logs all incoming requests with method, path, and response status using tracing.

use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse};
use tracing::Level;

/// Create request logging middleware
pub fn logging_layer() -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
        )
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .include_headers(false)
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_layer_creation() {
        let _layer = logging_layer();
        // Just ensure it creates without panic
        assert!(true);
    }
}
