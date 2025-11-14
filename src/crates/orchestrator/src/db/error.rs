//! Database error types and handling
//!
//! Provides custom error types for database operations with proper error
//! propagation and conversion from sqlx errors.

use std::fmt;
use thiserror::Error;

/// Custom database error type
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// Connection error
    #[error("Database connection failed: {0}")]
    ConnectionError(String),

    /// Record not found
    #[error("Record not found: {0}")]
    NotFound(String),

    /// Constraint violation (unique, foreign key, etc.)
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    /// Data type mismatch or conversion error
    #[error("Data type error: {0}")]
    TypeError(String),

    /// Migration error
    #[error("Migration failed: {0}")]
    MigrationError(String),

    /// Transaction error
    #[error("Transaction failed: {0}")]
    TransactionError(String),

    /// Query execution error
    #[error("Query error: {0}")]
    QueryError(String),

    /// Row mapping error
    #[error("Row mapping error: {0}")]
    RowMappingError(String),

    /// Pool error
    #[error("Connection pool error: {0}")]
    PoolError(String),

    /// Generic database error
    #[error("Database error: {0}")]
    Other(String),
}

impl DatabaseError {
    /// Create a new NotFound error with context
    pub fn not_found(context: impl Into<String>) -> Self {
        DatabaseError::NotFound(context.into())
    }

    /// Create a new ConstraintViolation error
    pub fn constraint(msg: impl Into<String>) -> Self {
        DatabaseError::ConstraintViolation(msg.into())
    }

    /// Create a new TypeError error
    pub fn type_error(msg: impl Into<String>) -> Self {
        DatabaseError::TypeError(msg.into())
    }

    /// Create a new QueryError error
    pub fn query_error(msg: impl Into<String>) -> Self {
        DatabaseError::QueryError(msg.into())
    }

    /// Check if this is a not found error
    pub fn is_not_found(&self) -> bool {
        matches!(self, DatabaseError::NotFound(_))
    }

    /// Check if this is a constraint violation
    pub fn is_constraint_violation(&self) -> bool {
        matches!(self, DatabaseError::ConstraintViolation(_))
    }
}

/// Result type for database operations
pub type DbResult<T> = std::result::Result<T, DatabaseError>;

/// Convert sqlx::Error to DatabaseError
impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => {
                DatabaseError::NotFound("No matching row found in database".to_string())
            }
            sqlx::Error::ColumnNotFound(col) => {
                DatabaseError::RowMappingError(format!("Column not found: {}", col))
            }
            sqlx::Error::ColumnIndexOutOfBounds { index, len } => {
                DatabaseError::RowMappingError(format!(
                    "Column index out of bounds: {} >= {}",
                    index, len
                ))
            }
            sqlx::Error::ColumnDecode { index, source } => {
                DatabaseError::TypeError(format!(
                    "Error decoding column {}: {}",
                    index, source
                ))
            }
            sqlx::Error::Decode(source) => {
                DatabaseError::TypeError(format!("Decode error: {}", source))
            }
            sqlx::Error::Configuration(msg) => {
                DatabaseError::ConnectionError(format!("Configuration error: {}", msg))
            }
            sqlx::Error::Io(err) => {
                DatabaseError::ConnectionError(format!("IO error: {}", err))
            }
            sqlx::Error::Tls(err) => {
                DatabaseError::ConnectionError(format!("TLS error: {}", err))
            }
            sqlx::Error::PoolTimedOut => {
                DatabaseError::PoolError("Connection pool timed out".to_string())
            }
            sqlx::Error::PoolClosed => {
                DatabaseError::PoolError("Connection pool is closed".to_string())
            }
            sqlx::Error::Migrate(err) => {
                DatabaseError::MigrationError(format!("Migration error: {}", err))
            }
            err => {
                DatabaseError::Other(format!("Database error: {}", err))
            }
        }
    }
}

/// Convert DatabaseError to sqlx::Error (for compatibility)
impl From<DatabaseError> for sqlx::Error {
    fn from(err: DatabaseError) -> Self {
        sqlx::Error::Configuration(Box::new(DatabaseErrorWrapper(err)))
    }
}

/// Wrapper for DatabaseError to implement std::error::Error
#[derive(Debug)]
struct DatabaseErrorWrapper(DatabaseError);

impl fmt::Display for DatabaseErrorWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DatabaseErrorWrapper {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = DatabaseError::not_found("user_id=123");
        assert!(err.is_not_found());
        assert!(!err.is_constraint_violation());
    }

    #[test]
    fn test_constraint_error() {
        let err = DatabaseError::constraint("UNIQUE constraint failed");
        assert!(err.is_constraint_violation());
        assert!(!err.is_not_found());
    }

    #[test]
    fn test_type_error() {
        let err = DatabaseError::type_error("Expected integer, got string");
        match err {
            DatabaseError::TypeError(_) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_error_display() {
        let err = DatabaseError::not_found("record");
        let msg = format!("{}", err);
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_sqlx_row_not_found_conversion() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let db_err: DatabaseError = sqlx_err.into();
        assert!(db_err.is_not_found());
    }

    #[test]
    fn test_database_error_debug() {
        let err = DatabaseError::not_found("test");
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
    }
}
