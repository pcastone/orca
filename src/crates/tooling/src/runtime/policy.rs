//! Policy enforcement for Tool Runtime SDK
//!
//! Implements policy registry and validation rules for tool execution.

use serde::{Deserialize, Serialize};

/// Policy registry for tool runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRegistry {
    /// Registry version
    pub version: u32,

    /// Network policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkPolicy>,

    /// Shell command allowlist
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_allow: Option<Vec<String>>,

    /// Git policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitPolicy>,

    /// AST policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast: Option<AstPolicy>,

    /// Validation rules
    #[serde(default)]
    pub validators: Vec<ValidatorRule>,
}

impl PolicyRegistry {
    /// Create a new empty policy registry
    pub fn new() -> Self {
        Self {
            version: 1,
            network: None,
            shell_allow: None,
            git: None,
            ast: None,
            validators: Vec::new(),
        }
    }

    /// Create a default policy registry with common rules
    pub fn default_policy() -> Self {
        Self {
            version: 1,
            network: Some(NetworkPolicy::default()),
            shell_allow: Some(vec![
                "^cargo(\\s|$)".to_string(),
                "^npm(\\s|$)".to_string(),
                "^rg(\\s|$)".to_string(),
            ]),
            git: Some(GitPolicy::default()),
            ast: Some(AstPolicy::default()),
            validators: vec![
                ValidatorRule::blocking("sec.network.allowlist"),
                ValidatorRule::blocking("sec.shell.allowlist"),
                ValidatorRule::blocking("sec.paths.sandbox"),
                ValidatorRule::blocking("code.ast.parsed"),
            ],
        }
    }

    /// Validate a tool request against policies
    pub fn validate_tool_request(
        &self,
        tool: &str,
        _args: &serde_json::Value,
    ) -> Result<(), PolicyViolation> {
        // Apply validators based on tool type
        for validator in &self.validators {
            if validator.enforcement == EnforcementLevel::Blocking {
                // Check if validator applies to this tool
                if self.validator_applies_to_tool(&validator.rule, tool) {
                    // Perform validation (simplified for now)
                    // In full implementation, this would call specific validation logic
                }
            }
        }

        Ok(())
    }

    fn validator_applies_to_tool(&self, rule: &str, tool: &str) -> bool {
        match rule {
            "sec.network.allowlist" => tool == "curl",
            "sec.shell.allowlist" => tool == "shell_exec",
            "sec.paths.sandbox" => tool.starts_with("file_") || tool.starts_with("fs_"),
            "sec.git.clean_tree" => tool.starts_with("git_"),
            _ => false,
        }
    }
}

impl Default for PolicyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Network policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// Allowed domains for network access
    pub allowed_domains: Vec<String>,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            allowed_domains: vec![
                "doc.rust-lang.org".to_string(),
                "crates.io".to_string(),
            ],
        }
    }
}

/// Git policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitPolicy {
    /// Allow git push operations
    pub allow_push: bool,

    /// Require clean tree for commit operations
    pub require_clean_tree_for_commit: bool,
}

impl Default for GitPolicy {
    fn default() -> Self {
        Self {
            allow_push: true,
            require_clean_tree_for_commit: true,
        }
    }
}

/// AST policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstPolicy {
    /// Programming language for AST operations
    pub language: String,

    /// Format code on save
    pub format_on_save: bool,

    /// Validate AST on edit
    pub validate_on_edit: bool,
}

impl Default for AstPolicy {
    fn default() -> Self {
        Self {
            language: "rust".to_string(),
            format_on_save: true,
            validate_on_edit: true,
        }
    }
}

/// Validator rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorRule {
    /// Rule identifier
    pub rule: String,

    /// Enforcement level
    pub enforcement: EnforcementLevel,
}

impl ValidatorRule {
    /// Create a blocking validator rule
    pub fn blocking(rule: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            enforcement: EnforcementLevel::Blocking,
        }
    }

    /// Create a warning validator rule
    pub fn warning(rule: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            enforcement: EnforcementLevel::Warning,
        }
    }

    /// Create a suggestion validator rule
    pub fn suggestion(rule: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            enforcement: EnforcementLevel::Suggestion,
        }
    }
}

/// Enforcement level for validation rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnforcementLevel {
    /// Blocking - prevents operation
    Blocking,

    /// Warning - logs warning but allows operation
    Warning,

    /// Suggestion - informational only
    Suggestion,
}

/// Policy violation error
#[derive(Debug, Clone)]
pub struct PolicyViolation {
    /// Rule that was violated
    pub rule: String,

    /// Human-readable violation message
    pub message: String,

    /// Enforcement level of the violated rule
    pub enforcement: EnforcementLevel,
}

impl PolicyViolation {
    /// Create a new policy violation
    pub fn new(
        rule: impl Into<String>,
        message: impl Into<String>,
        enforcement: EnforcementLevel,
    ) -> Self {
        Self {
            rule: rule.into(),
            message: message.into(),
            enforcement,
        }
    }
}

impl std::fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Policy violation ({:?}): {} - {}",
            self.enforcement, self.rule, self.message
        )
    }
}

impl std::error::Error for PolicyViolation {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_registry_new() {
        let registry = PolicyRegistry::new();
        assert_eq!(registry.version, 1);
        assert!(registry.validators.is_empty());
    }

    #[test]
    fn test_policy_registry_default() {
        let registry = PolicyRegistry::default_policy();
        assert_eq!(registry.version, 1);
        assert!(registry.network.is_some());
        assert!(registry.shell_allow.is_some());
        assert!(registry.git.is_some());
        assert!(registry.ast.is_some());
        assert!(!registry.validators.is_empty());
    }

    #[test]
    fn test_network_policy_default() {
        let policy = NetworkPolicy::default();
        assert!(policy.allowed_domains.contains(&"crates.io".to_string()));
    }

    #[test]
    fn test_git_policy_default() {
        let policy = GitPolicy::default();
        assert!(policy.allow_push);
        assert!(policy.require_clean_tree_for_commit);
    }

    #[test]
    fn test_ast_policy_default() {
        let policy = AstPolicy::default();
        assert_eq!(policy.language, "rust");
        assert!(policy.format_on_save);
        assert!(policy.validate_on_edit);
    }

    #[test]
    fn test_validator_rule_creation() {
        let blocking = ValidatorRule::blocking("test.rule");
        assert_eq!(blocking.enforcement, EnforcementLevel::Blocking);

        let warning = ValidatorRule::warning("test.rule");
        assert_eq!(warning.enforcement, EnforcementLevel::Warning);

        let suggestion = ValidatorRule::suggestion("test.rule");
        assert_eq!(suggestion.enforcement, EnforcementLevel::Suggestion);
    }

    #[test]
    fn test_policy_violation() {
        let violation =
            PolicyViolation::new("sec.test", "Test violation", EnforcementLevel::Blocking);

        assert_eq!(violation.rule, "sec.test");
        assert_eq!(violation.enforcement, EnforcementLevel::Blocking);

        let message = violation.to_string();
        assert!(message.contains("sec.test"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let registry = PolicyRegistry::default_policy();
        let yaml = serde_yaml::to_string(&registry).unwrap();
        let deserialized: PolicyRegistry = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(registry.version, deserialized.version);
    }
}
