//! Tool permission model

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Permission level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    /// Tool is fully allowed
    Allowed,
    /// Tool is restricted (path/arg restrictions apply)
    Restricted,
    /// Tool requires user approval before execution
    RequiresApproval,
    /// Tool execution is denied
    Denied,
}

impl PermissionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Restricted => "restricted",
            Self::RequiresApproval => "requires_approval",
            Self::Denied => "denied",
        }
    }
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for PermissionLevel {
    fn from(s: &str) -> Self {
        match s {
            "allowed" => Self::Allowed,
            "restricted" => Self::Restricted,
            "requires_approval" => Self::RequiresApproval,
            "denied" => Self::Denied,
            _ => Self::RequiresApproval, // Default to safe option
        }
    }
}

/// Tool permission configuration
///
/// Defines security controls for tool execution
/// Stored in project database (<project>/.orca/project.db)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ToolPermission {
    /// Unique permission identifier (UUID string)
    pub id: String,

    /// Tool name (unique)
    pub tool_name: String,

    /// Permission level
    pub permission_level: String,

    /// Path restrictions (JSON array of allowed paths, e.g., ["/project/*"])
    pub path_restrictions: Option<String>,

    /// Argument whitelist (JSON array of allowed argument patterns)
    pub arg_whitelist: Option<String>,

    /// Argument blacklist (JSON array of forbidden argument patterns)
    pub arg_blacklist: Option<String>,

    /// Description of permission rules
    pub description: Option<String>,

    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,
}

impl ToolPermission {
    /// Create a new tool permission
    pub fn new(tool_name: String, permission_level: PermissionLevel) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            tool_name,
            permission_level: permission_level.as_str().to_string(),
            path_restrictions: None,
            arg_whitelist: None,
            arg_blacklist: None,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Builder: Set path restrictions
    pub fn with_path_restrictions(mut self, restrictions: String) -> Self {
        self.path_restrictions = Some(restrictions);
        self
    }

    /// Builder: Set argument whitelist
    pub fn with_arg_whitelist(mut self, whitelist: String) -> Self {
        self.arg_whitelist = Some(whitelist);
        self
    }

    /// Builder: Set argument blacklist
    pub fn with_arg_blacklist(mut self, blacklist: String) -> Self {
        self.arg_blacklist = Some(blacklist);
        self
    }

    /// Builder: Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Check if tool is allowed to execute
    pub fn is_allowed(&self) -> bool {
        self.permission_level == PermissionLevel::Allowed.as_str()
    }

    /// Check if tool is denied
    pub fn is_denied(&self) -> bool {
        self.permission_level == PermissionLevel::Denied.as_str()
    }

    /// Check if tool requires approval
    pub fn requires_approval(&self) -> bool {
        self.permission_level == PermissionLevel::RequiresApproval.as_str()
    }
}
