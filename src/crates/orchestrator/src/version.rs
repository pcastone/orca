// Version information module for orchestrator
//
// Provides version constants for the orchestrator crate

/// Version string for the orchestrator crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Package name
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");

/// Build timestamp (if available)
pub const BUILD_TIMESTAMP: &str = env!("CARGO_PKG_VERSION");
