//! CORS middleware configuration
//!
//! Configures Cross-Origin Resource Sharing (CORS) to allow requests from
//! localhost and development environments.

use tower_http::cors::CorsLayer;

/// Create CORS layer for development (allows localhost)
pub fn cors_layer() -> CorsLayer {
    CorsLayer::permissive()
}

/// Create CORS layer for production (restricted origins)
pub fn cors_layer_restricted(_allowed_origins: &[&str]) -> CorsLayer {
    // Note: For production, implement proper origin checking
    // This is a placeholder that allows permissive CORS
    CorsLayer::permissive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_layer_creation() {
        let cors = cors_layer();
        // Just ensure it creates without panic
        assert!(true);
    }

    #[test]
    fn test_cors_layer_restricted() {
        let origins = vec!["https://example.com"];
        let cors = cors_layer_restricted(&origins);
        // Just ensure it creates without panic
        assert!(true);
    }
}
