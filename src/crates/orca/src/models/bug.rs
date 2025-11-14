//! Bug tracking model

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Bug status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BugStatus {
    /// Bug is open and needs attention
    Open,
    /// Bug is being worked on
    InProgress,
    /// Bug has been fixed
    Fixed,
    /// Bug will not be fixed
    Wontfix,
    /// Bug is a duplicate of another bug
    Duplicate,
}

impl BugStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Fixed => "fixed",
            Self::Wontfix => "wontfix",
            Self::Duplicate => "duplicate",
        }
    }
}

impl std::fmt::Display for BugStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for BugStatus {
    fn from(s: &str) -> Self {
        match s {
            "open" => Self::Open,
            "in_progress" => Self::InProgress,
            "fixed" => Self::Fixed,
            "wontfix" => Self::Wontfix,
            "duplicate" => Self::Duplicate,
            _ => Self::Open,
        }
    }
}

/// Bug priority enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BugPriority {
    /// Critical bug blocking progress
    Critical = 1,
    /// High priority bug
    High = 2,
    /// Medium priority bug
    Medium = 3,
    /// Low priority bug
    Low = 4,
    /// Trivial bug
    Trivial = 5,
}

impl From<i64> for BugPriority {
    fn from(n: i64) -> Self {
        match n {
            1 => Self::Critical,
            2 => Self::High,
            3 => Self::Medium,
            4 => Self::Low,
            5 => Self::Trivial,
            _ => Self::Medium,
        }
    }
}

/// Bug tracking record
///
/// Stored in project database (<project>/.orca/project.db)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bug {
    /// Unique bug identifier (UUID string)
    pub id: String,

    /// Bug title
    pub title: String,

    /// Detailed bug description
    pub description: Option<String>,

    /// Bug status
    pub status: String,

    /// Bug priority (1=critical, 2=high, 3=medium, 4=low, 5=trivial)
    pub priority: i64,

    /// Severity label
    pub severity: Option<String>,

    /// Assigned developer/user
    pub assignee: Option<String>,

    /// Bug reporter
    pub reporter: Option<String>,

    /// Labels (JSON array)
    pub labels: String,

    /// Related file paths (JSON array)
    pub related_files: String,

    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,

    /// Last update timestamp (Unix timestamp)
    pub updated_at: i64,

    /// Resolution timestamp (Unix timestamp, optional)
    pub resolved_at: Option<i64>,

    /// Additional metadata (JSON)
    pub metadata: String,
}

impl Bug {
    /// Create a new bug
    pub fn new(title: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            description: None,
            status: BugStatus::Open.as_str().to_string(),
            priority: 3, // Medium by default
            severity: None,
            assignee: None,
            reporter: None,
            labels: "[]".to_string(),
            related_files: "[]".to_string(),
            created_at: now,
            updated_at: now,
            resolved_at: None,
            metadata: "{}".to_string(),
        }
    }

    /// Builder: Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Builder: Set priority
    pub fn with_priority(mut self, priority: BugPriority) -> Self {
        self.priority = priority as i64;
        self
    }

    /// Builder: Set assignee
    pub fn with_assignee(mut self, assignee: String) -> Self {
        self.assignee = Some(assignee);
        self
    }

    /// Builder: Set reporter
    pub fn with_reporter(mut self, reporter: String) -> Self {
        self.reporter = Some(reporter);
        self
    }

    /// Mark bug as fixed
    pub fn mark_fixed(&mut self) {
        self.status = BugStatus::Fixed.as_str().to_string();
        self.resolved_at = Some(Utc::now().timestamp());
        self.updated_at = Utc::now().timestamp();
    }

    /// Mark bug as in progress
    pub fn start_work(&mut self) {
        self.status = BugStatus::InProgress.as_str().to_string();
        self.updated_at = Utc::now().timestamp();
    }
}
