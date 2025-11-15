//! Tool Runtime SDK - Core implementation
//!
//! Provides neutral, implementation-agnostic SDK for discovering, invoking,
//! and auditing developer tools (filesystem, git, AST, shell, HTTP, etc.)
//! under consistent contracts and rules.
//!
//! # Architecture
//!
//! - **Client** — Issues tool calls (e.g., UI, agent)
//! - **Runtime Host** — Loads the SDK and executes tools
//! - **Coordinator** — Optional planner/enforcer for budgets and policy
//! - **Session** — Authenticated context for a set of tool calls
//! - **Registry** — Declarative config for tools, rules, and policies
//!
//! # Example
//!
//! ```rust
//! use tooling::runtime::{ToolRequest, ToolResponse, PolicyRegistry, ToolRuntimeContext};
//! use std::path::PathBuf;
//!
//! // Create a policy registry
//! let policy = PolicyRegistry::default_policy();
//!
//! // Create execution context
//! let context = ToolRuntimeContext::new("sess-123", PathBuf::from("/workspace"));
//!
//! // Create a tool request
//! let request = ToolRequest::new(
//!     "file_read",
//!     serde_json::json!({"path": "src/main.rs"}),
//!     "req-1",
//!     "sess-123",
//! );
//!
//! // Tool would be executed here...
//! ```

pub mod context;
pub mod error;
pub mod messages;
pub mod policy;

// Re-export commonly used types
pub use context::ToolRuntimeContext;
pub use error::{Result, RuntimeError};
pub use messages::{
    error_codes, ErrorMessage, EventMessage, Heartbeat, ProgressInfo, SessionAck, ToolRequest,
    ToolResponse,
};
pub use policy::{
    AstPolicy, EnforcementLevel, GitPolicy, NetworkPolicy, PolicyRegistry, PolicyViolation,
    ValidatorRule,
};

/// Tool Runtime SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default timeouts for different tool categories (in milliseconds)
pub mod timeouts {
    /// Filesystem operations timeout
    pub const FILESYSTEM: u64 = 10_000; // 10s

    /// Git operations timeout
    pub const GIT: u64 = 30_000; // 30s

    /// Shell command timeout
    pub const SHELL: u64 = 600_000; // 10m

    /// HTTP request timeout
    pub const HTTP: u64 = 15_000; // 15s

    /// AST generation timeout
    pub const AST_GENERATE: u64 = 300_000; // 5m

    /// AST edit timeout
    pub const AST_EDIT: u64 = 30_000; // 30s

    /// Grep operation timeout
    pub const GREP: u64 = 10_000; // 10s
}

/// Default limits for tool operations
pub mod limits {
    /// Maximum stdout/stderr capture size (bytes)
    pub const MAX_OUTPUT_SIZE: usize = 5 * 1024 * 1024; // 5 MiB

    /// Maximum HTTP response body size (bytes)
    pub const MAX_HTTP_BODY: usize = 5 * 1024 * 1024; // 5 MiB

    /// Maximum number of processes returned by proc_list
    pub const MAX_PROCESSES: usize = 50;

    /// Maximum number of grep matches
    pub const MAX_GREP_MATCHES: usize = 1000;

    /// Maximum number of files for fs_list
    pub const MAX_FILES_LIST: usize = 5000;

    /// Maximum file read size (bytes)
    pub const MAX_FILE_READ: usize = 1024 * 1024; // 1 MiB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_timeouts() {
        assert_eq!(timeouts::FILESYSTEM, 10_000);
        assert_eq!(timeouts::GIT, 30_000);
        assert_eq!(timeouts::SHELL, 600_000);
    }

    #[test]
    fn test_limits() {
        assert_eq!(limits::MAX_OUTPUT_SIZE, 5 * 1024 * 1024);
        assert_eq!(limits::MAX_FILE_READ, 1024 * 1024);
    }
}
