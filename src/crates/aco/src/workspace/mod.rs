//! Workspace Management
//!
//! This module handles workspace initialization, validation, and management.
//! A workspace is a directory containing configuration, logs, and state for acolib.

pub mod initializer;
pub mod security;

pub use initializer::{
    WorkspaceInitConfig, WorkspaceInitializer, WorkspaceMetadata, WorkspaceValidator,
};
pub use security::{PathValidator, SecurityConfig};
