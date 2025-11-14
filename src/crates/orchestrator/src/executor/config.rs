//! TaskExecutor Configuration
//!
//! This module provides configuration for LLM-based task execution,
//! including model selection, generation parameters, and retry settings.

use crate::{OrchestratorError, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

use super::retry::RetryConfig;

/// Configuration for LLM-based task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Model identifier (e.g., "gpt-4", "claude-3-opus")
    pub model: String,

    /// Temperature for LLM generation (0.0 - 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Maximum tokens for LLM response
    #[serde(default)]
    pub max_tokens: Option<usize>,

    /// Retry configuration
    #[serde(default)]
    pub retry: RetryConfig,

    /// System prompt override (if None, use default)
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Enable streaming responses
    #[serde(default = "default_streaming")]
    pub streaming: bool,

    /// Context window size for the model (in tokens)
    #[serde(default)]
    pub context_window: Option<usize>,
}

fn default_temperature() -> f32 {
    0.7
}

fn default_streaming() -> bool {
    false
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self::gpt4()
    }
}

impl ExecutorConfig {
    /// Create a new executor configuration
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            temperature: default_temperature(),
            max_tokens: None,
            retry: RetryConfig::default(),
            system_prompt: None,
            streaming: default_streaming(),
            context_window: None,
        }
    }

    /// Load configuration from a YAML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            OrchestratorError::General(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            ))
        })?;

        let config: Self = serde_yaml::from_str(&content).map_err(|e| {
            OrchestratorError::General(format!("Failed to parse YAML config: {}", e))
        })?;

        config.validate()?;
        Ok(config)
    }

    /// Load configuration with environment variable overrides
    pub fn from_file_with_env(path: impl AsRef<Path>) -> Result<Self> {
        let mut config = Self::from_file(path)?;
        config.apply_env_overrides();
        config.validate()?;
        Ok(config)
    }

    /// Create configuration from environment variables only
    pub fn from_env() -> Self {
        let mut config = Self::default();
        config.apply_env_overrides();
        config
    }

    /// Apply environment variable overrides
    ///
    /// Supported environment variables:
    /// - ACOLIB_LLM_MODEL: Model identifier
    /// - ACOLIB_LLM_TEMPERATURE: Temperature (0.0-1.0)
    /// - ACOLIB_LLM_MAX_TOKENS: Maximum tokens
    /// - ACOLIB_LLM_MAX_RETRIES: Maximum retry attempts
    /// - ACOLIB_LLM_STREAMING: Enable streaming (true/false)
    pub fn apply_env_overrides(&mut self) {
        if let Ok(model) = env::var("ACOLIB_LLM_MODEL") {
            self.model = model;
        }

        if let Ok(temp) = env::var("ACOLIB_LLM_TEMPERATURE") {
            if let Ok(value) = temp.parse::<f32>() {
                self.temperature = value.clamp(0.0, 1.0);
            }
        }

        if let Ok(tokens) = env::var("ACOLIB_LLM_MAX_TOKENS") {
            if let Ok(value) = tokens.parse::<usize>() {
                self.max_tokens = Some(value);
            }
        }

        if let Ok(retries) = env::var("ACOLIB_LLM_MAX_RETRIES") {
            if let Ok(value) = retries.parse::<u32>() {
                self.retry.max_retries = value;
            }
        }

        if let Ok(streaming) = env::var("ACOLIB_LLM_STREAMING") {
            self.streaming = streaming.eq_ignore_ascii_case("true")
                || streaming == "1"
                || streaming.eq_ignore_ascii_case("yes");
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        if self.model.is_empty() {
            return Err(OrchestratorError::General(
                "Model identifier cannot be empty".to_string(),
            ));
        }

        if !(0.0..=1.0).contains(&self.temperature) {
            return Err(OrchestratorError::General(format!(
                "Temperature must be between 0.0 and 1.0, got {}",
                self.temperature
            )));
        }

        if let Some(tokens) = self.max_tokens {
            if tokens == 0 {
                return Err(OrchestratorError::General(
                    "max_tokens must be greater than 0".to_string(),
                ));
            }
        }

        if let Some(context) = self.context_window {
            if context == 0 {
                return Err(OrchestratorError::General(
                    "context_window must be greater than 0".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Set model identifier
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 1.0);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry = config;
        self
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set streaming mode
    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.streaming = enabled;
        self
    }

    /// Set context window size
    pub fn with_context_window(mut self, size: usize) -> Self {
        self.context_window = Some(size);
        self
    }

    // ========================================================================
    // Preset configurations for common models
    // ========================================================================

    /// Configuration for GPT-4
    pub fn gpt4() -> Self {
        Self {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: Some(4096),
            retry: RetryConfig::default(),
            system_prompt: None,
            streaming: false,
            context_window: Some(8192),
        }
    }

    /// Configuration for GPT-4 Turbo
    pub fn gpt4_turbo() -> Self {
        Self {
            model: "gpt-4-turbo-preview".to_string(),
            temperature: 0.7,
            max_tokens: Some(4096),
            retry: RetryConfig::default(),
            system_prompt: None,
            streaming: false,
            context_window: Some(128000),
        }
    }

    /// Configuration for GPT-3.5 Turbo
    pub fn gpt35_turbo() -> Self {
        Self {
            model: "gpt-3.5-turbo".to_string(),
            temperature: 0.7,
            max_tokens: Some(4096),
            retry: RetryConfig::default(),
            system_prompt: None,
            streaming: false,
            context_window: Some(16385),
        }
    }

    /// Configuration for Claude 3 Opus
    pub fn claude3_opus() -> Self {
        Self {
            model: "claude-3-opus-20240229".to_string(),
            temperature: 0.7,
            max_tokens: Some(4096),
            retry: RetryConfig::default(),
            system_prompt: None,
            streaming: false,
            context_window: Some(200000),
        }
    }

    /// Configuration for Claude 3 Sonnet
    pub fn claude3_sonnet() -> Self {
        Self {
            model: "claude-3-sonnet-20240229".to_string(),
            temperature: 0.7,
            max_tokens: Some(4096),
            retry: RetryConfig::default(),
            system_prompt: None,
            streaming: false,
            context_window: Some(200000),
        }
    }

    /// Configuration for Claude 3 Haiku
    pub fn claude3_haiku() -> Self {
        Self {
            model: "claude-3-haiku-20240307".to_string(),
            temperature: 0.7,
            max_tokens: Some(4096),
            retry: RetryConfig::default(),
            system_prompt: None,
            streaming: false,
            context_window: Some(200000),
        }
    }

    /// Get model preset by name
    ///
    /// Supported presets:
    /// - "gpt-4", "gpt4"
    /// - "gpt-4-turbo", "gpt4-turbo"
    /// - "gpt-3.5-turbo", "gpt35-turbo"
    /// - "claude-3-opus", "claude3-opus"
    /// - "claude-3-sonnet", "claude3-sonnet"
    /// - "claude-3-haiku", "claude3-haiku"
    pub fn from_preset(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "gpt-4" | "gpt4" => Some(Self::gpt4()),
            "gpt-4-turbo" | "gpt4-turbo" | "gpt-4-turbo-preview" => Some(Self::gpt4_turbo()),
            "gpt-3.5-turbo" | "gpt35-turbo" => Some(Self::gpt35_turbo()),
            "claude-3-opus" | "claude3-opus" | "claude-3-opus-20240229" => {
                Some(Self::claude3_opus())
            }
            "claude-3-sonnet" | "claude3-sonnet" | "claude-3-sonnet-20240229" => {
                Some(Self::claude3_sonnet())
            }
            "claude-3-haiku" | "claude3-haiku" | "claude-3-haiku-20240307" => {
                Some(Self::claude3_haiku())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = ExecutorConfig::default();
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.7);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = ExecutorConfig::new("test-model")
            .with_temperature(0.5)
            .with_max_tokens(2000)
            .with_streaming(true);

        assert_eq!(config.model, "test-model");
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, Some(2000));
        assert!(config.streaming);
    }

    #[test]
    fn test_temperature_clamping() {
        let config = ExecutorConfig::new("test").with_temperature(1.5);
        assert_eq!(config.temperature, 1.0);

        let config = ExecutorConfig::new("test").with_temperature(-0.5);
        assert_eq!(config.temperature, 0.0);
    }

    #[test]
    fn test_validation() {
        // Valid config
        let config = ExecutorConfig::new("test-model");
        assert!(config.validate().is_ok());

        // Empty model
        let config = ExecutorConfig::new("");
        assert!(config.validate().is_err());

        // Invalid temperature (outside 0-1 range)
        let mut config = ExecutorConfig::new("test");
        config.temperature = 1.5;
        assert!(config.validate().is_err());

        // Invalid max_tokens
        let config = ExecutorConfig::new("test").with_max_tokens(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_from_yaml() {
        let yaml = r#"
model: gpt-4-turbo
temperature: 0.5
max_tokens: 2000
streaming: true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();

        let config = ExecutorConfig::from_file(temp_file.path()).unwrap();
        assert_eq!(config.model, "gpt-4-turbo");
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, Some(2000));
        assert!(config.streaming);
    }

    #[test]
    fn test_env_overrides() {
        env::set_var("ACOLIB_LLM_MODEL", "custom-model");
        env::set_var("ACOLIB_LLM_TEMPERATURE", "0.9");
        env::set_var("ACOLIB_LLM_MAX_TOKENS", "1500");
        env::set_var("ACOLIB_LLM_STREAMING", "true");

        let config = ExecutorConfig::from_env();

        assert_eq!(config.model, "custom-model");
        assert_eq!(config.temperature, 0.9);
        assert_eq!(config.max_tokens, Some(1500));
        assert!(config.streaming);

        // Cleanup
        env::remove_var("ACOLIB_LLM_MODEL");
        env::remove_var("ACOLIB_LLM_TEMPERATURE");
        env::remove_var("ACOLIB_LLM_MAX_TOKENS");
        env::remove_var("ACOLIB_LLM_STREAMING");
    }

    #[test]
    fn test_presets() {
        let gpt4 = ExecutorConfig::gpt4();
        assert_eq!(gpt4.model, "gpt-4");
        assert_eq!(gpt4.context_window, Some(8192));

        let gpt4_turbo = ExecutorConfig::gpt4_turbo();
        assert_eq!(gpt4_turbo.model, "gpt-4-turbo-preview");
        assert_eq!(gpt4_turbo.context_window, Some(128000));

        let claude_opus = ExecutorConfig::claude3_opus();
        assert_eq!(claude_opus.model, "claude-3-opus-20240229");
        assert_eq!(claude_opus.context_window, Some(200000));
    }

    #[test]
    fn test_from_preset() {
        let config = ExecutorConfig::from_preset("gpt-4").unwrap();
        assert_eq!(config.model, "gpt-4");

        let config = ExecutorConfig::from_preset("gpt4").unwrap();
        assert_eq!(config.model, "gpt-4");

        let config = ExecutorConfig::from_preset("claude3-opus").unwrap();
        assert_eq!(config.model, "claude-3-opus-20240229");

        assert!(ExecutorConfig::from_preset("unknown-model").is_none());
    }

    #[test]
    fn test_yaml_with_env_override() {
        let yaml = r#"
model: gpt-4
temperature: 0.5
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();

        // Set env override
        env::set_var("ACOLIB_LLM_TEMPERATURE", "0.8");

        let config = ExecutorConfig::from_file_with_env(temp_file.path()).unwrap();
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.8); // Overridden by env

        // Cleanup
        env::remove_var("ACOLIB_LLM_TEMPERATURE");
    }

    #[test]
    fn test_retry_config_integration() {
        let retry_config = RetryConfig::new(5).with_initial_backoff(500);

        let config = ExecutorConfig::new("test").with_retry_config(retry_config);

        assert_eq!(config.retry.max_retries, 5);
        assert_eq!(config.retry.initial_backoff_ms, 500);
    }

    #[test]
    fn test_context_window() {
        let config = ExecutorConfig::new("test").with_context_window(100000);
        assert_eq!(config.context_window, Some(100000));
        assert!(config.validate().is_ok());

        let config = ExecutorConfig::new("test").with_context_window(0);
        assert!(config.validate().is_err());
    }
}
