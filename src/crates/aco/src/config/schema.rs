//! Configuration schema for aco client

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main aco configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcoConfig {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Client configuration
    #[serde(default)]
    pub client: ClientConfig,

    /// Tools configuration
    #[serde(default)]
    pub tools: ToolsConfig,

    /// UI configuration
    #[serde(default)]
    pub ui: UiConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host
    pub host: String,

    /// Server port
    pub port: u16,

    /// WebSocket path
    pub ws_path: String,

    /// Enable TLS
    pub enable_tls: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            ws_path: "/ws".to_string(),
            enable_tls: false,
        }
    }
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Orchestrator URL
    pub orchestrator_url: String,

    /// Session timeout in seconds
    pub session_timeout: u64,

    /// Reconnect attempts
    pub reconnect_attempts: u32,

    /// Reconnect delay in milliseconds
    pub reconnect_delay_ms: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            orchestrator_url: "ws://127.0.0.1:8080/ws".to_string(),
            session_timeout: 3600,
            reconnect_attempts: 5,
            reconnect_delay_ms: 1000,
        }
    }
}

/// Tools configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    /// Enable specific tools
    pub enabled_tools: Vec<String>,

    /// Tool-specific settings
    pub tool_settings: HashMap<String, toml::Value>,

    /// Tool execution timeout in seconds
    pub execution_timeout: u64,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            enabled_tools: vec![],
            tool_settings: HashMap::new(),
            execution_timeout: 300,
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Enable TUI mode
    pub enable_tui: bool,

    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,

    /// Enable colored output
    pub colored_output: bool,

    /// Show timestamps in logs
    pub show_timestamps: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            enable_tui: false,
            log_level: "info".to_string(),
            colored_output: true,
            show_timestamps: true,
        }
    }
}

impl AcoConfig {
    /// Merge another config into this one (other takes precedence)
    ///
    /// Simply replaces all fields from other, since serde already fills in defaults
    /// for missing fields. The loader handles priority: defaults → user → project
    pub fn merge(&mut self, other: AcoConfig) {
        // Replace all sections from other config
        self.server = other.server;
        self.client = other.client;

        // For tools, merge the settings map
        self.tools.enabled_tools = other.tools.enabled_tools;
        self.tools.tool_settings.extend(other.tools.tool_settings);
        self.tools.execution_timeout = other.tools.execution_timeout;

        self.ui = other.ui;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AcoConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.client.orchestrator_url, "ws://127.0.0.1:8080/ws");
    }

    #[test]
    fn test_merge_config() {
        let mut base = AcoConfig::default();
        let mut override_config = AcoConfig::default();
        override_config.server.port = 9090;
        override_config.client.session_timeout = 7200;

        base.merge(override_config);

        assert_eq!(base.server.port, 9090);
        assert_eq!(base.client.session_timeout, 7200);
        assert_eq!(base.server.host, "127.0.0.1"); // Unchanged
    }
}
