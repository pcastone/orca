//! Error context utilities
//!
//! Provides helpers for adding context to errors and formatting error chains.

use std::error::Error as StdError;
use std::fmt;

/// Trait for adding context to errors
///
/// This trait provides convenient methods for adding contextual information
/// to errors, making debugging and error reporting more informative.
///
/// # Example
///
/// ```rust,ignore
/// use tooling::error::ErrorContext;
///
/// fn read_config(path: &str) -> tooling::Result<String> {
///     std::fs::read_to_string(path)
///         .map_err(|e| e.into())
///         .context(format!("Failed to read config file: {}", path))?;
///     Ok(contents)
/// }
/// ```
pub trait ErrorContext<T> {
    /// Add context to an error with a static string message
    ///
    /// # Arguments
    ///
    /// * `msg` - Context message to add to the error
    ///
    /// # Returns
    ///
    /// The result with error wrapped with context
    fn context(self, msg: impl Into<String>) -> Result<T, Box<dyn StdError + Send + Sync>>;

    /// Add context to an error using a closure (lazily evaluated)
    ///
    /// Useful when the context message is expensive to compute
    /// and should only be created if an error actually occurs.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that returns the context message
    ///
    /// # Returns
    ///
    /// The result with error wrapped with context
    fn with_context<F>(self, f: F) -> Result<T, Box<dyn StdError + Send + Sync>>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn context(self, msg: impl Into<String>) -> Result<T, Box<dyn StdError + Send + Sync>> {
        self.map_err(|e| {
            let context = ContextError {
                message: msg.into(),
                source: Box::new(e),
            };
            Box::new(context) as Box<dyn StdError + Send + Sync>
        })
    }

    fn with_context<F>(self, f: F) -> Result<T, Box<dyn StdError + Send + Sync>>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let context = ContextError {
                message: f(),
                source: Box::new(e),
            };
            Box::new(context) as Box<dyn StdError + Send + Sync>
        })
    }
}

/// Error with contextual information
#[derive(Debug)]
struct ContextError {
    message: String,
    source: Box<dyn StdError + Send + Sync>,
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl StdError for ContextError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&*self.source as &(dyn StdError + 'static))
    }
}

/// Format an error chain as a multi-line string
///
/// Walks the error chain via `source()` and formats each error
/// on a separate line with indentation.
///
/// # Arguments
///
/// * `error` - The error to format
///
/// # Returns
///
/// A formatted string showing the complete error chain
///
/// # Example
///
/// ```rust,ignore
/// use tooling::error::format_error_chain;
///
/// match some_operation() {
///     Err(e) => {
///         eprintln!("Error occurred:\n{}", format_error_chain(&e));
///     }
///     Ok(_) => {}
/// }
/// ```
pub fn format_error_chain(error: &dyn StdError) -> String {
    let mut result = format!("Error: {}", error);
    let mut current = error.source();
    let mut level = 1;

    while let Some(source) = current {
        result.push_str(&format!("\n{:indent$}Caused by: {}", "", source, indent = level * 2));
        current = source.source();
        level += 1;
    }

    result
}

/// Get the root cause of an error chain
///
/// Walks the error chain via `source()` until reaching the bottom.
///
/// # Arguments
///
/// * `error` - The error to analyze
///
/// # Returns
///
/// The root cause (last error in the chain)
///
/// # Example
///
/// ```rust,ignore
/// use tooling::error::root_cause;
///
/// match some_operation() {
///     Err(e) => {
///         let root = root_cause(&e);
///         eprintln!("Root cause: {}", root);
///     }
///     Ok(_) => {}
/// }
/// ```
pub fn root_cause(error: &dyn StdError) -> &dyn StdError {
    let mut current = error;
    while let Some(source) = current.source() {
        current = source;
    }
    current
}

/// Count the number of errors in an error chain
///
/// # Arguments
///
/// * `error` - The error to analyze
///
/// # Returns
///
/// The number of errors in the chain (minimum 1)
pub fn error_chain_length(error: &dyn StdError) -> usize {
    let mut count = 1;
    let mut current = error.source();

    while let Some(source) = current {
        count += 1;
        current = source.source();
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolingError;

    fn inner_operation() -> Result<(), std::io::Error> {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ))
    }

    fn middle_operation() -> Result<(), Box<dyn StdError + Send + Sync>> {
        inner_operation()
            .context("Failed to read configuration")
    }

    fn outer_operation() -> Result<(), Box<dyn StdError + Send + Sync>> {
        match middle_operation() {
            Ok(v) => Ok(v),
            Err(e) => {
                let ctx = ContextError {
                    message: "Application initialization failed".to_string(),
                    source: e,
                };
                Err(Box::new(ctx))
            }
        }
    }

    #[test]
    fn test_error_context() {
        let result: Result<(), ToolingError> = Err(ToolingError::General("test error".to_string()));
        let with_context = result.context("Operation failed");

        assert!(with_context.is_err());
        let err = with_context.unwrap_err();
        assert_eq!(err.to_string(), "Operation failed");
    }

    #[test]
    fn test_error_with_context() {
        let result: Result<(), ToolingError> = Err(ToolingError::General("test error".to_string()));
        let with_context = result.with_context(|| format!("Failed at {}", "location"));

        assert!(with_context.is_err());
        let err = with_context.unwrap_err();
        assert_eq!(err.to_string(), "Failed at location");
    }

    #[test]
    fn test_format_error_chain() {
        let result = outer_operation();
        assert!(result.is_err());

        let error = result.unwrap_err();
        let formatted = format_error_chain(&*error);

        assert!(formatted.contains("Application initialization failed"));
        assert!(formatted.contains("Failed to read configuration"));
        assert!(formatted.contains("File not found"));
        assert!(formatted.contains("Caused by:"));
    }

    #[test]
    fn test_root_cause() {
        let result = outer_operation();
        assert!(result.is_err());

        let error = result.unwrap_err();
        let root = root_cause(&*error);

        assert_eq!(root.to_string(), "File not found");
    }

    #[test]
    fn test_error_chain_length() {
        let result = outer_operation();
        assert!(result.is_err());

        let error = result.unwrap_err();
        let length = error_chain_length(&*error);

        // Should be: Application init -> read config -> file not found
        assert_eq!(length, 3);
    }

    #[test]
    fn test_single_error_chain() {
        let error = ToolingError::General("single error".to_string());
        let formatted = format_error_chain(&error);

        assert_eq!(formatted, "Error: Tooling error: single error");
        assert_eq!(error_chain_length(&error), 1);
        assert_eq!(root_cause(&error).to_string(), "Tooling error: single error");
    }
}
