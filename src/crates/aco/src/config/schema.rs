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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
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

impl ServerConfig {
    /// Merge another config into this one, only replacing non-default fields
    pub fn merge(&mut self, other: ServerConfig) {
        let defaults = ServerConfig::default();
        if other.host != defaults.host {
            self.host = other.host;
        }
        if other.port != defaults.port {
            self.port = other.port;
        }
        if other.ws_path != defaults.ws_path {
            self.ws_path = other.ws_path;
        }
        if other.enable_tls != defaults.enable_tls {
            self.enable_tls = other.enable_tls;
        }
    }
}

impl ClientConfig {
    /// Merge another config into this one, only replacing non-default fields
    pub fn merge(&mut self, other: ClientConfig) {
        let defaults = ClientConfig::default();
        if other.orchestrator_url != defaults.orchestrator_url {
            self.orchestrator_url = other.orchestrator_url;
        }
        if other.session_timeout != defaults.session_timeout {
            self.session_timeout = other.session_timeout;
        }
        if other.reconnect_attempts != defaults.reconnect_attempts {
            self.reconnect_attempts = other.reconnect_attempts;
        }
        if other.reconnect_delay_ms != defaults.reconnect_delay_ms {
            self.reconnect_delay_ms = other.reconnect_delay_ms;
        }
    }
}

impl ToolsConfig {
    /// Merge another config into this one, only replacing non-default fields
    pub fn merge(&mut self, other: ToolsConfig) {
        let defaults = ToolsConfig::default();

        // Merge enabled_tools (union of both lists, removing duplicates)
        if other.enabled_tools != defaults.enabled_tools {
            for tool in other.enabled_tools {
                if !self.enabled_tools.contains(&tool) {
                    self.enabled_tools.push(tool);
                }
            }
        }

        // Merge tool_settings (extend with other's settings)
        self.tool_settings.extend(other.tool_settings);

        if other.execution_timeout != defaults.execution_timeout {
            self.execution_timeout = other.execution_timeout;
        }
    }
}

impl UiConfig {
    /// Merge another config into this one, only replacing non-default fields
    pub fn merge(&mut self, other: UiConfig) {
        let defaults = UiConfig::default();
        if other.enable_tui != defaults.enable_tui {
            self.enable_tui = other.enable_tui;
        }
        if other.log_level != defaults.log_level {
            self.log_level = other.log_level;
        }
        if other.colored_output != defaults.colored_output {
            self.colored_output = other.colored_output;
        }
        if other.show_timestamps != defaults.show_timestamps {
            self.show_timestamps = other.show_timestamps;
        }
    }
}

impl AcoConfig {
    /// Merge another config into this one (other takes precedence)
    ///
    /// Field-level merging: only non-default fields from other override this config
    /// The loader handles priority: defaults → user → project
    pub fn merge(&mut self, other: AcoConfig) {
        self.server.merge(other.server);
        self.client.merge(other.client);
        self.tools.merge(other.tools);
        self.ui.merge(other.ui);
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        // Validate server port is in valid range
        if self.server.port == 0 {
            return Err("Server port must be greater than 0".to_string());
        }

        // Validate session timeout is reasonable
        if self.client.session_timeout == 0 {
            return Err("Session timeout must be greater than 0".to_string());
        }

        // Validate reconnect attempts
        if self.client.reconnect_attempts == 0 {
            return Err("Reconnect attempts must be at least 1".to_string());
        }

        // Validate log level
        let valid_log_levels = vec!["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&self.ui.log_level.as_str()) {
            return Err(format!(
                "Invalid log level '{}'. Must be one of: trace, debug, info, warn, error",
                self.ui.log_level
            ));
        }

        Ok(())
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
