//! Configuration module for orchestrator
//!
//! Provides YAML configuration loading and parsing for:
//! - Pattern definitions (ReAct, Plan-Execute, Reflection, etc.)
//! - Router/supervisor configuration
//! - Workflow orchestration
//! - Environment variable expansion and file includes
//! - Server configuration (SSL/TLS, security, database, LDAP)

pub mod loader;
pub mod pattern;
pub mod router;
pub mod server;
pub mod workflow;

pub use loader::{deep_merge, load_yaml_config, load_yaml_file};
pub use pattern::{
    BasePatternSettings, CodeActConfig, CotConfig, EvaluationStrategy, GotConfig, LatsConfig,
    PatternConfig, PlanExecuteConfig, ReactConfig, ReflectionConfig, StormConfig, TotConfig,
};
pub use router::{
    ConditionCheck, GuardConfig, RegistryConfig, RoutePolicyConfig, RouteRule, RouterConfig,
    RouterSettings, RuleCondition, TerminationCondition, TerminationConfig,
};
pub use server::{
    DatabaseConfig, LdapConfig, SecurityConfig, SecurityMode, ServerConfig, ServerConfigError,
    SslConfig, SslMode, X509Config,
};
pub use server::ldap::{LdapClient, LdapError};
pub use server::security::{SecurityState, security_middleware};
pub use server::ssl::{setup_ssl_certificates, SslCertPaths, SslError};
pub use workflow::{
    StepCondition, StepTransition, WorkflowConfig, WorkflowSettings, WorkflowState,
    WorkflowStatus, WorkflowStep,
};
