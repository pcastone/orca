//! Generic validation utilities
//!
//! Provides a fluent API for validating values with chainable rules.
//!
//! # Example
//!
//! ```rust
//! use tooling::validation::{Validator, ValidationRule};
//!
//! // Validate a number
//! let age = 25;
//! Validator::new(age, "age")
//!     .min(0)
//!     .max(120)
//!     .validate()
//!     .unwrap();
//!
//! // Validate a string
//! let email = "user@example.com";
//! Validator::new(email, "email")
//!     .not_empty()
//!     .min_length(3)
//!     .max_length(100)
//!     .matches(r"^[^@]+@[^@]+\.[^@]+$")
//!     .validate()
//!     .unwrap();
//!
//! // Custom validation
//! let value = 42;
//! Validator::new(value, "value")
//!     .custom(|v| {
//!         if v % 2 == 0 {
//!             Ok(())
//!         } else {
//!             Err("Value must be even".to_string())
//!         }
//!     })
//!     .validate()
//!     .unwrap();
//! ```

use crate::{Result, ToolingError};
use regex::Regex;
use std::fmt::Display;

/// Validation rule for a value
pub trait ValidationRule<T> {
    /// Validate the value
    ///
    /// # Returns
    ///
    /// Ok(()) if valid, Err with message if invalid
    fn validate(&self, value: &T, field_name: &str) -> std::result::Result<(), String>;
}

/// Fluent validator for values
///
/// Allows chaining multiple validation rules and collecting errors.
pub struct Validator<T> {
    value: T,
    field_name: String,
    rules: Vec<Box<dyn ValidationRule<T>>>,
}

impl<T: 'static> Validator<T> {
    /// Create a new validator for a value
    ///
    /// # Arguments
    ///
    /// * `value` - Value to validate
    /// * `field_name` - Name of the field (for error messages)
    pub fn new(value: T, field_name: impl Into<String>) -> Self {
        Self {
            value,
            field_name: field_name.into(),
            rules: Vec::new(),
        }
    }

    /// Add a custom validation rule
    pub fn custom<F>(mut self, validator: F) -> Self
    where
        F: Fn(&T) -> std::result::Result<(), String> + 'static,
    {
        struct CustomRule<T, F> {
            validator: F,
            _phantom: std::marker::PhantomData<T>,
        }

        impl<T, F> ValidationRule<T> for CustomRule<T, F>
        where
            F: Fn(&T) -> std::result::Result<(), String>,
        {
            fn validate(&self, value: &T, _field_name: &str) -> std::result::Result<(), String> {
                (self.validator)(value)
            }
        }

        self.rules.push(Box::new(CustomRule {
            validator,
            _phantom: std::marker::PhantomData,
        }));
        self
    }

    /// Validate all rules
    ///
    /// # Returns
    ///
    /// Ok(value) if all rules pass, Err with first error message
    pub fn validate(self) -> Result<T> {
        for rule in &self.rules {
            rule.validate(&self.value, &self.field_name)
                .map_err(|msg| ToolingError::General(msg))?;
        }
        Ok(self.value)
    }

    /// Validate all rules and collect all errors
    ///
    /// # Returns
    ///
    /// Ok(value) if all rules pass, Err with all error messages
    pub fn validate_all(self) -> std::result::Result<T, Vec<String>> {
        let errors: Vec<String> = self
            .rules
            .iter()
            .filter_map(|rule| rule.validate(&self.value, &self.field_name).err())
            .collect();

        if errors.is_empty() {
            Ok(self.value)
        } else {
            Err(errors)
        }
    }
}

// Numeric validators
impl<T> Validator<T>
where
    T: PartialOrd + Copy + Display + 'static,
{
    /// Ensure value is greater than or equal to minimum
    pub fn min(mut self, min: T) -> Self {
        struct MinRule<T> {
            min: T,
        }

        impl<T: PartialOrd + Display> ValidationRule<T> for MinRule<T> {
            fn validate(&self, value: &T, field_name: &str) -> std::result::Result<(), String> {
                if value >= &self.min {
                    Ok(())
                } else {
                    Err(format!(
                        "{} must be at least {} (got {})",
                        field_name, self.min, value
                    ))
                }
            }
        }

        self.rules.push(Box::new(MinRule { min }));
        self
    }

    /// Ensure value is less than or equal to maximum
    pub fn max(mut self, max: T) -> Self {
        struct MaxRule<T> {
            max: T,
        }

        impl<T: PartialOrd + Display> ValidationRule<T> for MaxRule<T> {
            fn validate(&self, value: &T, field_name: &str) -> std::result::Result<(), String> {
                if value <= &self.max {
                    Ok(())
                } else {
                    Err(format!(
                        "{} must be at most {} (got {})",
                        field_name, self.max, value
                    ))
                }
            }
        }

        self.rules.push(Box::new(MaxRule { max }));
        self
    }

    /// Ensure value is within range (inclusive)
    pub fn range(self, min: T, max: T) -> Self {
        self.min(min).max(max)
    }
}

// String validators
impl Validator<&str> {
    /// Ensure string is not empty
    pub fn not_empty(mut self) -> Self {
        struct NotEmptyRule;

        impl ValidationRule<&str> for NotEmptyRule {
            fn validate(&self, value: &&str, field_name: &str) -> std::result::Result<(), String> {
                if !value.is_empty() {
                    Ok(())
                } else {
                    Err(format!("{} must not be empty", field_name))
                }
            }
        }

        self.rules.push(Box::new(NotEmptyRule));
        self
    }

    /// Ensure string has minimum length
    pub fn min_length(mut self, min: usize) -> Self {
        struct MinLengthRule {
            min: usize,
        }

        impl ValidationRule<&str> for MinLengthRule {
            fn validate(&self, value: &&str, field_name: &str) -> std::result::Result<(), String> {
                if value.len() >= self.min {
                    Ok(())
                } else {
                    Err(format!(
                        "{} must be at least {} characters (got {})",
                        field_name,
                        self.min,
                        value.len()
                    ))
                }
            }
        }

        self.rules.push(Box::new(MinLengthRule { min }));
        self
    }

    /// Ensure string has maximum length
    pub fn max_length(mut self, max: usize) -> Self {
        struct MaxLengthRule {
            max: usize,
        }

        impl ValidationRule<&str> for MaxLengthRule {
            fn validate(&self, value: &&str, field_name: &str) -> std::result::Result<(), String> {
                if value.len() <= self.max {
                    Ok(())
                } else {
                    Err(format!(
                        "{} must be at most {} characters (got {})",
                        field_name,
                        self.max,
                        value.len()
                    ))
                }
            }
        }

        self.rules.push(Box::new(MaxLengthRule { max }));
        self
    }

    /// Ensure string matches regex pattern
    pub fn matches(mut self, pattern: &str) -> Self {
        let regex = Regex::new(pattern).expect("Invalid regex pattern");

        struct MatchesRule {
            regex: Regex,
            pattern: String,
        }

        impl ValidationRule<&str> for MatchesRule {
            fn validate(&self, value: &&str, field_name: &str) -> std::result::Result<(), String> {
                if self.regex.is_match(value) {
                    Ok(())
                } else {
                    Err(format!(
                        "{} must match pattern: {}",
                        field_name, self.pattern
                    ))
                }
            }
        }

        self.rules.push(Box::new(MatchesRule {
            regex,
            pattern: pattern.to_string(),
        }));
        self
    }
}

impl Validator<String> {
    /// Ensure string is not empty
    pub fn not_empty(mut self) -> Self {
        struct NotEmptyRule;

        impl ValidationRule<String> for NotEmptyRule {
            fn validate(
                &self,
                value: &String,
                field_name: &str,
            ) -> std::result::Result<(), String> {
                if !value.is_empty() {
                    Ok(())
                } else {
                    Err(format!("{} must not be empty", field_name))
                }
            }
        }

        self.rules.push(Box::new(NotEmptyRule));
        self
    }

    /// Ensure string has minimum length
    pub fn min_length(mut self, min: usize) -> Self {
        struct MinLengthRule {
            min: usize,
        }

        impl ValidationRule<String> for MinLengthRule {
            fn validate(
                &self,
                value: &String,
                field_name: &str,
            ) -> std::result::Result<(), String> {
                if value.len() >= self.min {
                    Ok(())
                } else {
                    Err(format!(
                        "{} must be at least {} characters (got {})",
                        field_name,
                        self.min,
                        value.len()
                    ))
                }
            }
        }

        self.rules.push(Box::new(MinLengthRule { min }));
        self
    }

    /// Ensure string has maximum length
    pub fn max_length(mut self, max: usize) -> Self {
        struct MaxLengthRule {
            max: usize,
        }

        impl ValidationRule<String> for MaxLengthRule {
            fn validate(
                &self,
                value: &String,
                field_name: &str,
            ) -> std::result::Result<(), String> {
                if value.len() <= self.max {
                    Ok(())
                } else {
                    Err(format!(
                        "{} must be at most {} characters (got {})",
                        field_name,
                        self.max,
                        value.len()
                    ))
                }
            }
        }

        self.rules.push(Box::new(MaxLengthRule { max }));
        self
    }

    /// Ensure string matches regex pattern
    pub fn matches(mut self, pattern: &str) -> Self {
        let regex = Regex::new(pattern).expect("Invalid regex pattern");

        struct MatchesRule {
            regex: Regex,
            pattern: String,
        }

        impl ValidationRule<String> for MatchesRule {
            fn validate(
                &self,
                value: &String,
                field_name: &str,
            ) -> std::result::Result<(), String> {
                if self.regex.is_match(value) {
                    Ok(())
                } else {
                    Err(format!(
                        "{} must match pattern: {}",
                        field_name, self.pattern
                    ))
                }
            }
        }

        self.rules.push(Box::new(MatchesRule {
            regex,
            pattern: pattern.to_string(),
        }));
        self
    }
}

// Collection validators
impl<T> Validator<Vec<T>> {
    /// Ensure collection is not empty
    pub fn not_empty(mut self) -> Self {
        struct NotEmptyRule;

        impl<T> ValidationRule<Vec<T>> for NotEmptyRule {
            fn validate(
                &self,
                value: &Vec<T>,
                field_name: &str,
            ) -> std::result::Result<(), String> {
                if !value.is_empty() {
                    Ok(())
                } else {
                    Err(format!("{} must not be empty", field_name))
                }
            }
        }

        self.rules.push(Box::new(NotEmptyRule));
        self
    }

    /// Ensure collection has minimum length
    pub fn min_length(mut self, min: usize) -> Self {
        struct MinLengthRule {
            min: usize,
        }

        impl<T> ValidationRule<Vec<T>> for MinLengthRule {
            fn validate(
                &self,
                value: &Vec<T>,
                field_name: &str,
            ) -> std::result::Result<(), String> {
                if value.len() >= self.min {
                    Ok(())
                } else {
                    Err(format!(
                        "{} must have at least {} items (got {})",
                        field_name,
                        self.min,
                        value.len()
                    ))
                }
            }
        }

        self.rules.push(Box::new(MinLengthRule { min }));
        self
    }

    /// Ensure collection has maximum length
    pub fn max_length(mut self, max: usize) -> Self {
        struct MaxLengthRule {
            max: usize,
        }

        impl<T> ValidationRule<Vec<T>> for MaxLengthRule {
            fn validate(
                &self,
                value: &Vec<T>,
                field_name: &str,
            ) -> std::result::Result<(), String> {
                if value.len() <= self.max {
                    Ok(())
                } else {
                    Err(format!(
                        "{} must have at most {} items (got {})",
                        field_name,
                        self.max,
                        value.len()
                    ))
                }
            }
        }

        self.rules.push(Box::new(MaxLengthRule { max }));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_min() {
        let result = Validator::new(5, "value").min(3).validate();
        assert!(result.is_ok());

        let result = Validator::new(2, "value").min(3).validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_number_max() {
        let result = Validator::new(5, "value").max(10).validate();
        assert!(result.is_ok());

        let result = Validator::new(15, "value").max(10).validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_number_range() {
        let result = Validator::new(5, "value").range(1, 10).validate();
        assert!(result.is_ok());

        let result = Validator::new(15, "value").range(1, 10).validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_string_not_empty() {
        let result = Validator::new("hello", "value").not_empty().validate();
        assert!(result.is_ok());

        let result = Validator::new("", "value").not_empty().validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_string_length() {
        let result = Validator::new("hello", "value")
            .min_length(3)
            .max_length(10)
            .validate();
        assert!(result.is_ok());

        let result = Validator::new("hi", "value").min_length(3).validate();
        assert!(result.is_err());

        let result = Validator::new("hello world!", "value")
            .max_length(5)
            .validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_string_matches() {
        let result = Validator::new("test@example.com", "email")
            .matches(r"^[^@]+@[^@]+\.[^@]+$")
            .validate();
        assert!(result.is_ok());

        let result = Validator::new("invalid-email", "email")
            .matches(r"^[^@]+@[^@]+\.[^@]+$")
            .validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_string_owned() {
        let value = String::from("hello");
        let result = Validator::new(value, "value")
            .not_empty()
            .min_length(3)
            .validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_validator() {
        let result = Validator::new(10, "value")
            .custom(|v| {
                if v % 2 == 0 {
                    Ok(())
                } else {
                    Err("Value must be even".to_string())
                }
            })
            .validate();
        assert!(result.is_ok());

        let result = Validator::new(9, "value")
            .custom(|v| {
                if v % 2 == 0 {
                    Ok(())
                } else {
                    Err("Value must be even".to_string())
                }
            })
            .validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_vec_not_empty() {
        let result = Validator::new(vec![1, 2, 3], "items")
            .not_empty()
            .validate();
        assert!(result.is_ok());

        let result = Validator::new(Vec::<i32>::new(), "items")
            .not_empty()
            .validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_vec_length() {
        let result = Validator::new(vec![1, 2, 3], "items")
            .min_length(2)
            .max_length(5)
            .validate();
        assert!(result.is_ok());

        let result = Validator::new(vec![1], "items").min_length(2).validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_all() {
        let result = Validator::new("x", "value")
            .not_empty()
            .min_length(3)
            .max_length(10)
            .validate_all();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1); // Only min_length fails
    }

    #[test]
    fn test_chained_validations() {
        let age = 25;
        let result = Validator::new(age, "age")
            .min(0)
            .max(120)
            .custom(|v| {
                if v >= &18 {
                    Ok(())
                } else {
                    Err("Must be 18 or older".to_string())
                }
            })
            .validate();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 25);
    }
}
