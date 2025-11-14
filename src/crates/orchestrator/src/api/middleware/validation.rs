//! Request validation middleware and utilities
//!
//! Provides validation helpers for ensuring request data meets requirements.

use crate::api::error::{ApiError, ApiResult};

/// Validate that a required string field is not empty
pub fn validate_not_empty(value: &str, field_name: &str) -> ApiResult<()> {
    if value.is_empty() {
        return Err(ApiError::ValidationError(format!("{} cannot be empty", field_name)));
    }
    Ok(())
}

/// Validate string length constraints
pub fn validate_string_length(value: &str, field_name: &str, min: usize, max: usize) -> ApiResult<()> {
    if value.len() < min || value.len() > max {
        return Err(ApiError::ValidationError(
            format!("{} must be between {} and {} characters", field_name, min, max)
        ));
    }
    Ok(())
}

/// Validate pagination parameters
pub fn validate_pagination(_page: u32, per_page: u32, max_per_page: u32) -> ApiResult<()> {
    if per_page == 0 {
        return Err(ApiError::ValidationError("per_page must be greater than 0".to_string()));
    }
    if per_page > max_per_page {
        return Err(ApiError::ValidationError(
            format!("per_page cannot exceed {}", max_per_page)
        ));
    }
    Ok(())
}

/// Validate UUID format
pub fn validate_uuid(value: &str) -> ApiResult<uuid::Uuid> {
    uuid::Uuid::parse_str(value)
        .map_err(|_| ApiError::ValidationError(format!("Invalid UUID: {}", value)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_not_empty_valid() {
        assert!(validate_not_empty("hello", "name").is_ok());
    }

    #[test]
    fn test_validate_not_empty_empty() {
        assert!(validate_not_empty("", "name").is_err());
    }

    #[test]
    fn test_validate_string_length_valid() {
        assert!(validate_string_length("hello", "name", 1, 10).is_ok());
    }

    #[test]
    fn test_validate_string_length_too_short() {
        assert!(validate_string_length("hi", "name", 5, 10).is_err());
    }

    #[test]
    fn test_validate_string_length_too_long() {
        assert!(validate_string_length("very long string", "name", 1, 5).is_err());
    }

    #[test]
    fn test_validate_pagination_valid() {
        assert!(validate_pagination(0, 10, 100).is_ok());
    }

    #[test]
    fn test_validate_pagination_zero_per_page() {
        assert!(validate_pagination(0, 0, 100).is_err());
    }

    #[test]
    fn test_validate_pagination_exceeds_max() {
        assert!(validate_pagination(0, 150, 100).is_err());
    }

    #[test]
    fn test_validate_uuid_valid() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        assert!(validate_uuid(uuid_str).is_ok());
    }

    #[test]
    fn test_validate_uuid_invalid() {
        assert!(validate_uuid("invalid-uuid").is_err());
    }
}
