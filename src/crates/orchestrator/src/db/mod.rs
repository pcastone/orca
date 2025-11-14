//! Database module for orchestrator
//!
//! Provides database connectivity, models, repositories, and error handling
//! for persistent storage of orchestrator entities.

pub mod connection;
pub mod error;
pub mod models;
pub mod repositories;

pub use connection::{DatabaseConnection, DatabasePool, PoolStatistics};
pub use error::{DatabaseError, DbResult};
