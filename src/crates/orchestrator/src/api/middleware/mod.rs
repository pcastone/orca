//! API middleware layer
//!
//! Provides middleware for request processing including CORS, logging, and validation.

pub mod cors;
pub mod logging;
pub mod validation;

pub use cors::cors_layer;
pub use logging::logging_layer;
pub use validation::{
    validate_not_empty, validate_string_length, validate_pagination, validate_uuid,
};
