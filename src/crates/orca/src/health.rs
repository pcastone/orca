//! Health check functionality
//!
//! Provides system health verification for database, configuration, and dependencies.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All checks passed
    Healthy,
    /// Some checks failed but system is partially operational
    Degraded,
    /// Critical checks failed, system is not operational
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Individual component check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Check status
    pub status: HealthStatus,
    /// Human-readable message
    pub message: Option<String>,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

impl ComponentHealth {
    /// Create a healthy component check
    pub fn healthy(name: impl Into<String>, response_time_ms: u64) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Healthy,
            message: Some("OK".to_string()),
            response_time_ms,
        }
    }

    /// Create a degraded component check
    pub fn degraded(name: impl Into<String>, message: impl Into<String>, response_time_ms: u64) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Degraded,
            message: Some(message.into()),
            response_time_ms,
        }
    }

    /// Create an unhealthy component check
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>, response_time_ms: u64) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            response_time_ms,
        }
    }
}

/// Overall system health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Overall status
    pub status: HealthStatus,
    /// Individual component checks
    pub checks: Vec<ComponentHealth>,
    /// Total response time in milliseconds
    pub total_response_time_ms: u64,
    /// Timestamp of the check
    pub timestamp: i64,
}

impl HealthReport {
    /// Create a new health report from component checks
    pub fn new(checks: Vec<ComponentHealth>) -> Self {
        // Determine overall status
        let status = if checks.iter().any(|c| c.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if checks.iter().any(|c| c.status == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        let total_response_time_ms = checks.iter().map(|c| c.response_time_ms).sum();
        let timestamp = chrono::Utc::now().timestamp();

        Self {
            status,
            checks,
            total_response_time_ms,
            timestamp,
        }
    }
}

/// Health checker for system components
pub struct HealthChecker;

impl HealthChecker {
    /// Check database health
    pub async fn check_database(db: &crate::db::Database) -> ComponentHealth {
        let start = Instant::now();

        match db.health_check().await {
            Ok(_) => ComponentHealth::healthy("database", start.elapsed().as_millis() as u64),
            Err(e) => ComponentHealth::unhealthy("database", format!("Database error: {}", e), start.elapsed().as_millis() as u64),
        }
    }

    /// Check configuration validity
    pub fn check_config(config: &crate::config::OrcaConfig) -> ComponentHealth {
        let start = Instant::now();

        // Basic validation checks
        let mut issues = Vec::new();

        if config.llm.model.is_empty() {
            issues.push("LLM model not configured");
        }

        if config.llm.api_key.is_none() && config.llm.provider != "ollama" {
            issues.push("API key not set for non-local provider");
        }

        if config.execution.max_concurrent_tasks == 0 {
            issues.push("max_concurrent_tasks is 0");
        }

        let response_time_ms = start.elapsed().as_millis() as u64;

        if issues.is_empty() {
            ComponentHealth::healthy("configuration", response_time_ms)
        } else {
            ComponentHealth::degraded("configuration", issues.join("; "), response_time_ms)
        }
    }

    /// Check tool bridge initialization
    pub fn check_tool_bridge(bridge: &crate::tools::DirectToolBridge) -> ComponentHealth {
        let start = Instant::now();

        let tool_count = bridge.list_tools().len();

        if tool_count > 0 {
            ComponentHealth::healthy(
                "tool_bridge",
                start.elapsed().as_millis() as u64,
            )
        } else {
            ComponentHealth::degraded(
                "tool_bridge",
                "No tools registered",
                start.elapsed().as_millis() as u64,
            )
        }
    }

    /// Check workspace accessibility
    pub async fn check_workspace(workspace: &std::path::Path) -> ComponentHealth {
        let start = Instant::now();

        match tokio::fs::metadata(workspace).await {
            Ok(metadata) => {
                if metadata.is_dir() {
                    ComponentHealth::healthy("workspace", start.elapsed().as_millis() as u64)
                } else {
                    ComponentHealth::unhealthy("workspace", "Workspace path is not a directory", start.elapsed().as_millis() as u64)
                }
            }
            Err(e) => {
                ComponentHealth::unhealthy("workspace", format!("Cannot access workspace: {}", e), start.elapsed().as_millis() as u64)
            }
        }
    }

    /// Perform comprehensive health check on execution context
    pub async fn check_context(context: &crate::context::ExecutionContext) -> Result<HealthReport> {
        let mut checks = Vec::new();

        // Check database
        checks.push(Self::check_database(context.database()).await);

        // Check configuration
        checks.push(Self::check_config(context.config()));

        // Check tool bridge
        checks.push(Self::check_tool_bridge(context.tool_bridge()));

        // Check workspace
        checks.push(Self::check_workspace(context.workspace_root()).await);

        Ok(HealthReport::new(checks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
    }

    #[test]
    fn test_component_health_constructors() {
        let healthy = ComponentHealth::healthy("test", 10);
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.response_time_ms, 10);

        let degraded = ComponentHealth::degraded("test", "warning", 20);
        assert_eq!(degraded.status, HealthStatus::Degraded);
        assert_eq!(degraded.response_time_ms, 20);

        let unhealthy = ComponentHealth::unhealthy("test", "error", 30);
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert_eq!(unhealthy.response_time_ms, 30);
    }

    #[test]
    fn test_health_report_all_healthy() {
        let checks = vec![
            ComponentHealth::healthy("db", 10),
            ComponentHealth::healthy("config", 5),
        ];

        let report = HealthReport::new(checks);
        assert_eq!(report.status, HealthStatus::Healthy);
        assert_eq!(report.total_response_time_ms, 15);
        assert_eq!(report.checks.len(), 2);
    }

    #[test]
    fn test_health_report_with_degraded() {
        let checks = vec![
            ComponentHealth::healthy("db", 10),
            ComponentHealth::degraded("config", "missing key", 5),
        ];

        let report = HealthReport::new(checks);
        assert_eq!(report.status, HealthStatus::Degraded);
    }

    #[test]
    fn test_health_report_with_unhealthy() {
        let checks = vec![
            ComponentHealth::healthy("config", 5),
            ComponentHealth::unhealthy("db", "connection failed", 100),
        ];

        let report = HealthReport::new(checks);
        assert_eq!(report.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_report_unhealthy_overrides_degraded() {
        let checks = vec![
            ComponentHealth::degraded("config", "warning", 5),
            ComponentHealth::unhealthy("db", "error", 10),
            ComponentHealth::healthy("tools", 3),
        ];

        let report = HealthReport::new(checks);
        assert_eq!(report.status, HealthStatus::Unhealthy);
    }
}
