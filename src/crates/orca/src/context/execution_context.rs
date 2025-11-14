//! Execution context for workflows and tasks
//!
//! Provides unified access to all resources needed during execution.

use crate::config::OrcaConfig;
use crate::context::SessionInfo;
use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::events::EventLogger;
use crate::executor::{LlmProvider, TaskExecutor};
use crate::repositories::{TaskRepository, WorkflowRepository};
use crate::shutdown::ShutdownCoordinator;
use crate::tools::DirectToolBridge;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Execution context that provides access to all resources
#[derive(Clone, Debug)]
pub struct ExecutionContext {
    /// Session information
    session: SessionInfo,

    /// Database connection
    database: Arc<Database>,

    /// Direct tool bridge for tool execution
    tool_bridge: Arc<DirectToolBridge>,

    /// LLM provider for agent reasoning
    llm_provider: Arc<LlmProvider>,

    /// Task executor
    task_executor: Arc<TaskExecutor>,

    /// Task repository
    task_repository: TaskRepository,

    /// Workflow repository
    workflow_repository: WorkflowRepository,

    /// Configuration
    config: OrcaConfig,

    /// Shutdown coordinator for graceful termination
    shutdown_coordinator: Arc<ShutdownCoordinator>,

    /// Event logger for execution observability
    event_logger: EventLogger,
}

impl ExecutionContext {
    /// Get session information
    pub fn session(&self) -> &SessionInfo {
        &self.session
    }

    /// Get database connection
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Get tool bridge
    pub fn tool_bridge(&self) -> &Arc<DirectToolBridge> {
        &self.tool_bridge
    }

    /// Get LLM provider
    pub fn llm_provider(&self) -> &Arc<LlmProvider> {
        &self.llm_provider
    }

    /// Get task executor
    pub fn task_executor(&self) -> &Arc<TaskExecutor> {
        &self.task_executor
    }

    /// Get configuration
    pub fn config(&self) -> &OrcaConfig {
        &self.config
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session.session_id
    }

    /// Get workspace root
    pub fn workspace_root(&self) -> &PathBuf {
        self.tool_bridge.workspace_root()
    }

    /// Get task repository
    pub fn task_repository(&self) -> &TaskRepository {
        &self.task_repository
    }

    /// Get workflow repository
    pub fn workflow_repository(&self) -> &WorkflowRepository {
        &self.workflow_repository
    }

    /// Get shutdown coordinator
    pub fn shutdown_coordinator(&self) -> &Arc<ShutdownCoordinator> {
        &self.shutdown_coordinator
    }

    /// Check if shutdown has been requested
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_coordinator.is_shutdown_requested()
    }

    /// Get event logger
    pub fn event_logger(&self) -> &EventLogger {
        &self.event_logger
    }
}

/// Builder for creating execution contexts
pub struct ContextBuilder {
    session: Option<SessionInfo>,
    database: Option<Arc<Database>>,
    config: Option<OrcaConfig>,
    workspace_root: Option<PathBuf>,
}

impl ContextBuilder {
    /// Create a new context builder
    pub fn new() -> Self {
        Self {
            session: None,
            database: None,
            config: None,
            workspace_root: None,
        }
    }

    /// Set session information
    pub fn with_session(mut self, session: SessionInfo) -> Self {
        self.session = Some(session);
        self
    }

    /// Set database
    pub fn with_database(mut self, database: Arc<Database>) -> Self {
        self.database = Some(database);
        self
    }

    /// Set configuration
    pub fn with_config(mut self, config: OrcaConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set workspace root
    pub fn with_workspace_root(mut self, workspace_root: PathBuf) -> Self {
        self.workspace_root = Some(workspace_root);
        self
    }

    /// Build the execution context
    ///
    /// # Returns
    /// A fully initialized ExecutionContext
    ///
    /// # Errors
    /// Returns error if required components are missing or initialization fails
    pub async fn build(self) -> Result<ExecutionContext> {
        // Get or create session
        let session = self.session.unwrap_or_else(SessionInfo::new);

        // Get database (required)
        let database = self.database
            .ok_or_else(|| OrcaError::Config("Database is required for execution context".to_string()))?;

        // Get config (required)
        let config = self.config
            .ok_or_else(|| OrcaError::Config("Configuration is required for execution context".to_string()))?;

        // Get workspace root
        let workspace_root = self.workspace_root
            .or_else(|| config.execution.workspace_root.clone())
            .or_else(|| std::env::current_dir().ok())
            .ok_or_else(|| OrcaError::Config("Unable to determine workspace root".to_string()))?;

        info!(
            session_id = %session.session_id,
            workspace = %workspace_root.display(),
            "Initializing execution context"
        );

        // Create tool bridge
        let tool_bridge = Arc::new(
            DirectToolBridge::new(workspace_root, session.session_id.clone())
                .map_err(|e| OrcaError::ToolExecution(format!("Failed to create tool bridge: {}", e)))?
        );

        // Create LLM provider
        let llm_provider = Arc::new(LlmProvider::from_config(&config)?);

        // Create task executor
        let task_executor = Arc::new(TaskExecutor::new(tool_bridge.clone(), config.clone())?);

        // Create repositories
        let task_repository = TaskRepository::new(database.clone());
        let workflow_repository = WorkflowRepository::new(database.clone());

        // Create shutdown coordinator
        let shutdown_coordinator = Arc::new(ShutdownCoordinator::new());

        // Create event logger (enabled by default)
        let event_logger = EventLogger::new(true);

        info!(session_id = %session.session_id, "Execution context initialized");

        Ok(ExecutionContext {
            session,
            database,
            tool_bridge,
            llm_provider,
            task_executor,
            task_repository,
            workflow_repository,
            config,
            shutdown_coordinator,
            event_logger,
        })
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DatabaseConfig, ExecutionConfig, LlmConfig, LoggingConfig};
    use tempfile::TempDir;

    fn create_test_config() -> OrcaConfig {
        OrcaConfig {
            database: DatabaseConfig {
                path: "orca.db".to_string(),
            },
            llm: LlmConfig {
                provider: "ollama".to_string(),
                model: "llama2".to_string(),
                api_key: None,
                api_base: Some("http://localhost:11434".to_string()),
                temperature: 0.7,
                max_tokens: 1000,
            },
            execution: ExecutionConfig {
                max_concurrent_tasks: 3,
                task_timeout: 300,
                streaming: false,
                workspace_root: None,
                max_iterations: 5,
                ..Default::default()
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                colored: false,
                timestamps: true,
            },
        }
    }

    #[tokio::test]
    async fn test_context_builder_missing_database() {
        let config = create_test_config();

        let result = ContextBuilder::new()
            .with_config(config)
            .build()
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Database is required"));
    }

    #[tokio::test]
    async fn test_context_builder_missing_config() {
        use sqlx::sqlite::SqlitePoolOptions;

        // Use in-memory database for testing
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        let result = ContextBuilder::new()
            .with_database(db)
            .build()
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Configuration is required"));
    }

    #[tokio::test]
    async fn test_context_builder_success() {
        use sqlx::sqlite::SqlitePoolOptions;

        let temp_dir = TempDir::new().unwrap();

        // Use in-memory database for testing
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        let config = create_test_config();
        let session = SessionInfo::new().with_description("Test session");

        let context = ContextBuilder::new()
            .with_database(db)
            .with_config(config)
            .with_session(session.clone())
            .with_workspace_root(temp_dir.path().to_path_buf())
            .build()
            .await
            .unwrap();

        assert_eq!(context.session_id(), session.session_id);
        assert_eq!(context.workspace_root(), temp_dir.path());
        assert!(!context.tool_bridge().list_tools().is_empty());
    }

    #[tokio::test]
    async fn test_context_accessors() {
        use sqlx::sqlite::SqlitePoolOptions;

        let temp_dir = TempDir::new().unwrap();

        // Use in-memory database for testing
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db = Arc::new(Database {
            pool: Arc::new(pool),
        });

        let config = create_test_config();

        let context = ContextBuilder::new()
            .with_database(db.clone())
            .with_config(config.clone())
            .with_workspace_root(temp_dir.path().to_path_buf())
            .build()
            .await
            .unwrap();

        // Test accessors
        assert_eq!(context.config().llm.provider, "ollama");
        assert!(context.session().session_id.len() > 0);

        // Verify database is accessible
        let _ = context.database().health_check().await.unwrap();
    }
}
