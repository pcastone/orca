/// gRPC Service implementations for orchestrator

pub mod agent;
pub mod auth;
pub mod prompt;
pub mod task;
pub mod workflow;
pub mod execution_metrics;

pub use agent::{AgentService, AgentError, AgentResult};
pub use auth::{AuthServiceImpl, AuthMode, JwtManager, UserPassAuth, LdapAuth};
pub use prompt::{PromptService, PromptError};
pub use task::TaskServiceImpl;
pub use workflow::WorkflowServiceImpl;
pub use execution_metrics::{ExecutionMetricsService, ExecutionTracker, MetricsError, MetricsResult};
