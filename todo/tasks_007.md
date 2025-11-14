# Task 007: Set Up orchestrator gRPC Server Infrastructure

## Objective
Create the foundational gRPC server infrastructure for orchestrator including server setup, configuration, health checks, and graceful shutdown with proper async runtime management.

## Priority
**CRITICAL** - Foundation for all server functionality

## Dependencies
- Task 001 (Protocol Buffer definitions)
- Task 002 (Authentication)
- Task 005 (Error handling)

## Implementation Details

### Files to Create

1. **`src/crates/orchestrator/src/main.rs`**:
```rust
use orchestrator::server::OrchestratorServer;
use orchestrator::config::ServerConfig;
use tracing_subscriber;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    // Load configuration
    let config = ServerConfig::from_env()?;

    tracing::info!("Starting orchestrator server on {}", config.address());

    // Create and run server
    let server = OrchestratorServer::new(config).await?;
    server.serve().await?;

    Ok(())
}
```

2. **`src/crates/orchestrator/src/config.rs`**:
```rust
use anyhow::{Result, Context};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_seconds: u64,
    pub max_connections: u32,
    pub stream_buffer_size: usize,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: std::env::var("ORCHESTRATOR_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("ORCHESTRATOR_PORT")
                .unwrap_or_else(|_| "50051".to_string())
                .parse()
                .context("Invalid port number")?,
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./orchestrator.db".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| {
                    tracing::warn!("Using default JWT secret - CHANGE IN PRODUCTION");
                    "dev-secret-change-in-production".to_string()
                }),
            jwt_expiration_seconds: std::env::var("JWT_EXPIRATION_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .context("Invalid JWT expiration")?,
            max_connections: std::env::var("MAX_CONNECTIONS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .context("Invalid max connections")?,
            stream_buffer_size: std::env::var("STREAM_BUFFER_SIZE")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .context("Invalid stream buffer size")?,
        })
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn socket_addr(&self) -> Result<SocketAddr> {
        self.address().parse().context("Invalid socket address")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        std::env::remove_var("ORCHESTRATOR_HOST");
        std::env::remove_var("ORCHESTRATOR_PORT");

        let config = ServerConfig::from_env().unwrap();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 50051);
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("ORCHESTRATOR_HOST", "127.0.0.1");
        std::env::set_var("ORCHESTRATOR_PORT", "8080");

        let config = ServerConfig::from_env().unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);

        std::env::remove_var("ORCHESTRATOR_HOST");
        std::env::remove_var("ORCHESTRATOR_PORT");
    }

    #[test]
    fn test_address_formatting() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 3000,
            database_url: "".to_string(),
            jwt_secret: "test".to_string(),
            jwt_expiration_seconds: 3600,
            max_connections: 100,
            stream_buffer_size: 100,
        };

        assert_eq!(config.address(), "localhost:3000");
    }
}
```

3. **`src/crates/orchestrator/src/server.rs`**:
```rust
use crate::config::ServerConfig;
use crate::services::{TaskServiceImpl, WorkflowServiceImpl, HealthServiceImpl, AuthServiceImpl};
use crate::auth::{JwtManager, AuthInterceptor};
use crate::database::Database;
use crate::proto::tasks::task_service_server::TaskServiceServer;
use crate::proto::workflows::workflow_service_server::WorkflowServiceServer;
use crate::proto::health::health_service_server::HealthServiceServer;
use crate::proto::auth::auth_service_server::AuthServiceServer;
use tonic::transport::Server;
use anyhow::Result;
use std::sync::Arc;
use tokio::signal;

pub struct OrchestratorServer {
    config: ServerConfig,
    database: Arc<Database>,
    jwt_manager: Arc<JwtManager>,
}

impl OrchestratorServer {
    pub async fn new(config: ServerConfig) -> Result<Self> {
        // Initialize database
        let database = Database::new(&config.database_url).await?;
        database.run_migrations().await?;

        // Initialize JWT manager
        let jwt_manager = JwtManager::new(
            &config.jwt_secret,
            config.jwt_expiration_seconds,
        );

        Ok(Self {
            config,
            database: Arc::new(database),
            jwt_manager: Arc::new(jwt_manager),
        })
    }

    pub async fn serve(self) -> Result<()> {
        let addr = self.config.socket_addr()?;

        // Create services
        let task_service = TaskServiceImpl::new(
            self.database.clone(),
            self.config.stream_buffer_size,
        );

        let workflow_service = WorkflowServiceImpl::new(
            self.database.clone(),
            self.config.stream_buffer_size,
        );

        let health_service = HealthServiceImpl::new();

        let auth_service = AuthServiceImpl::new(
            self.database.clone(),
            self.jwt_manager.clone(),
        );

        // Create auth interceptor for protected services
        let auth_interceptor = AuthInterceptor::new((*self.jwt_manager).clone());

        tracing::info!("gRPC server listening on {}", addr);

        // Build and serve
        Server::builder()
            .add_service(AuthServiceServer::new(auth_service))
            .add_service(HealthServiceServer::new(health_service))
            .add_service(TaskServiceServer::with_interceptor(
                task_service,
                move |req| auth_interceptor.intercept(req),
            ))
            .add_service(WorkflowServiceServer::with_interceptor(
                workflow_service,
                move |req| auth_interceptor.intercept(req),
            ))
            .serve_with_shutdown(addr, Self::shutdown_signal())
            .await?;

        Ok(())
    }

    async fn shutdown_signal() {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("Received Ctrl+C, shutting down");
            },
            _ = terminate => {
                tracing::info!("Received terminate signal, shutting down");
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0, // Random port
            database_url: "sqlite::memory:".to_string(),
            jwt_secret: "test-secret".to_string(),
            jwt_expiration_seconds: 3600,
            max_connections: 10,
            stream_buffer_size: 10,
        };

        let server = OrchestratorServer::new(config).await;
        assert!(server.is_ok());
    }
}
```

4. **`src/crates/orchestrator/src/lib.rs`**:
```rust
pub mod config;
pub mod server;
pub mod database;
pub mod services;
pub mod auth;
pub mod error;
pub mod proto_conv;
pub mod streaming;

pub use server::OrchestratorServer;
pub use config::ServerConfig;

pub mod proto {
    pub mod tasks {
        tonic::include_proto!("orchestrator.tasks");
    }
    pub mod workflows {
        tonic::include_proto!("orchestrator.workflows");
    }
    pub mod health {
        tonic::include_proto!("orchestrator.health");
    }
    pub mod auth {
        tonic::include_proto!("orchestrator.auth");
    }
}
```

5. **`src/crates/orchestrator/src/services/mod.rs`**:
```rust
pub mod task;
pub mod workflow;
pub mod health;
pub mod auth;

pub use task::TaskServiceImpl;
pub use workflow::WorkflowServiceImpl;
pub use health::HealthServiceImpl;
pub use auth::AuthServiceImpl;
```

6. **`src/crates/orchestrator/src/services/health.rs`**:
```rust
use crate::proto::health::{
    health_service_server::HealthService,
    HealthCheckRequest, HealthCheckResponse, HealthStatus,
};
use tonic::{Request, Response, Status};
use std::time::SystemTime;

pub struct HealthServiceImpl {
    start_time: SystemTime,
}

impl HealthServiceImpl {
    pub fn new() -> Self {
        Self {
            start_time: SystemTime::now(),
        }
    }
}

#[tonic::async_trait]
impl HealthService for HealthServiceImpl {
    async fn check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let uptime = self.start_time
            .elapsed()
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let response = HealthCheckResponse {
            status: HealthStatus::Serving as i32,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            metadata: std::collections::HashMap::new(),
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let service = HealthServiceImpl::new();
        let request = Request::new(HealthCheckRequest {
            service: "test".to_string(),
        });

        let response = service.check(request).await.unwrap();
        let health = response.into_inner();

        assert_eq!(health.status, HealthStatus::Serving as i32);
        assert!(!health.version.is_empty());
        assert!(health.uptime_seconds >= 0);
    }
}
```

## Update Cargo.toml

**`src/crates/orchestrator/Cargo.toml`**:
```toml
[dependencies]
tonic = "0.10"
prost = "0.12"
tokio = { workspace = true, features = ["full"] }
tokio-stream = "0.1"
tracing = { workspace = true }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = { workspace = true }
domain = { path = "../domain" }
```

## Unit Tests

All tests embedded in implementation files.

## Acceptance Criteria

- [ ] Server starts and listens on configured port
- [ ] Configuration loaded from environment variables
- [ ] Database initialized on startup
- [ ] Migrations run automatically
- [ ] JWT manager configured
- [ ] All services registered (Task, Workflow, Health, Auth)
- [ ] Auth interceptor applied to protected services
- [ ] Graceful shutdown on Ctrl+C or SIGTERM
- [ ] Health check endpoint works
- [ ] All tests pass
- [ ] Logging initialized properly

## Complexity
**Moderate** - Standard gRPC server setup with tonic

## Estimated Effort
**6-8 hours**

## Notes
- Use tokio runtime with all features for server
- Apply auth interceptor only to protected services (not Health/Auth)
- Graceful shutdown ensures in-flight requests complete
- Log all important lifecycle events
- Use Arc for shared state (Database, JwtManager)
- Default to SQLite for simple deployment
