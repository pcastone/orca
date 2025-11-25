//! Orchestrator server binary
//!
//! Standalone server for the orchestrator service, providing REST API
//! for task management, workflows, and orchestration.

use std::sync::Arc;
use std::net::SocketAddr;
use tracing_subscriber;
use orchestrator::api::ws::BroadcastState;
use orchestrator::api::routes::create_router;
use orchestrator::config::{
    LdapClient, SecurityState, ServerConfig, setup_ssl_certificates,
};
use orchestrator::db::{DatabaseConnection, repositories::ConfigurationRepository};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(rust_log)
        .init();

    // Load configuration from orchestrator-server.toml
    tracing::info!("Loading server configuration...");
    let config = match ServerConfig::load() {
        Ok(cfg) => {
            tracing::info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            tracing::warn!("Failed to load configuration file: {}. Using defaults.", e);
            // Fall back to environment variables or defaults
            return Err(format!("Configuration required: {}. Set CONFIG_PATH or place config/orchestrator-server.toml", e).into());
        }
    };

    // Log configuration summary
    tracing::info!("SSL Mode: {:?}", config.ssl.mode);
    tracing::info!("Security Mode: {:?}", config.security.mode);
    tracing::info!("LDAP Enabled: {}", config.ldap.enabled);
    tracing::info!("Database Path: {}", config.database.path);

    // Get server address from environment (can be overridden)
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");
    let host = std::env::var("HOST")
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    // Use database path from configuration
    let database_url = config.database_url();

    // Initialize database connection
    tracing::info!("Connecting to database: {}", database_url);
    let db = DatabaseConnection::new(&database_url).await?;

    // Setup SSL/TLS certificates if configured
    if config.ssl.mode == orchestrator::config::SslMode::Auto || config.ssl.mode == orchestrator::config::SslMode::Pem {
        match setup_ssl_certificates(&config.ssl, None) {
            Ok(paths) => {
                tracing::info!("SSL certificates configured: {:?}", paths.cert);
            }
            Err(e) => {
                tracing::warn!("SSL certificate setup failed: {}. Server will run without SSL.", e);
            }
        }
    }

    // Initialize security middleware
    let security_state = Arc::new(SecurityState::new(config.security.clone()));
    tracing::info!("Security mode: {:?}", security_state.mode());

    // Initialize LDAP if enabled
    let _ldap_client = if config.ldap.enabled {
        let mut client = LdapClient::new(config.ldap.clone());
        if let Err(e) = client.connect().await {
            tracing::warn!("LDAP connection failed: {}. LDAP authentication disabled.", e);
            None
        } else {
            tracing::info!("LDAP authentication enabled");
            Some(client)
        }
    } else {
        None
    };

    // Run migrations
    tracing::info!("Running database migrations");
    db.run_migrations().await?;

    // Health check the database
    tracing::info!("Performing database health check");
    db.health_check().await?;

    // Store server name and generate UUID if not exists
    let pool = db.pool();
    
    // Store server name from config
    ConfigurationRepository::set(
        pool,
        "server.name".to_string(),
        config.server.name.clone(),
        "string".to_string(),
    ).await?;
    tracing::info!("Server name: {}", config.server.name);
    
    // Generate and store UUID if not exists
    let uuid = if let Some(existing) = ConfigurationRepository::get(pool, "server.uuid").await? {
        existing.value
    } else {
        let new_uuid = uuid::Uuid::new_v4().to_string();
        ConfigurationRepository::set(
            pool,
            "server.uuid".to_string(),
            new_uuid.clone(),
            "string".to_string(),
        ).await?;
        tracing::info!("Generated new server UUID: {}", new_uuid);
        new_uuid
    };
    tracing::info!("Server UUID: {}", uuid);

    // Create WebSocket broadcast state
    let broadcast = Arc::new(BroadcastState::new());

    // Connect to user database for LLM provider config
    let user_db_path = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".orca")
        .join("user.db");

    let user_db = match orca::db::Database::new(&user_db_path).await {
        Ok(db) => {
            tracing::info!("Connected to user database: {}", user_db_path.display());
            Some(std::sync::Arc::new(db))
        }
        Err(e) => {
            tracing::warn!("Failed to connect to user database: {}. Using server config for LLM.", e);
            None
        }
    };

    // Initialize LLM prompt service from user database or server config
    let prompt_service = if let Some(ref udb) = user_db {
        // Try to load from user database
        let repo = orca::repositories::LlmProviderRepository::new(std::sync::Arc::clone(udb));
        match repo.get_default().await {
            Ok(provider) => {
                // Convert database config to LlmConfig
                let llm_config = orchestrator::config::LlmConfig {
                    enabled: true,
                    provider: provider.provider_type.clone(),
                    model: provider.model.clone(),
                    api_key: provider.api_key.clone(),
                    api_base: provider.api_base.clone(),
                    temperature: provider.temperature as f32,
                    max_tokens: provider.max_tokens as u32,
                };
                match orchestrator::services::PromptService::new(&llm_config) {
                    Ok(service) => {
                        tracing::info!("LLM prompt service enabled from database: {}/{}", provider.provider_type, provider.model);
                        Some(service)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize LLM from database config: {}", e);
                        None
                    }
                }
            }
            Err(_) => {
                tracing::info!("No default LLM provider in database, falling back to server config");
                // Fall back to server config
                if config.llm.enabled {
                    match orchestrator::services::PromptService::new(&config.llm) {
                        Ok(service) => {
                            tracing::info!("LLM prompt service enabled with provider: {}", config.llm.provider);
                            Some(service)
                        }
                        Err(e) => {
                            tracing::warn!("Failed to initialize LLM prompt service: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            }
        }
    } else if config.llm.enabled {
        match orchestrator::services::PromptService::new(&config.llm) {
            Ok(service) => {
                tracing::info!("LLM prompt service enabled with provider: {}", config.llm.provider);
                Some(service)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize LLM prompt service: {}. Prompt endpoint disabled.", e);
                None
            }
        }
    } else {
        tracing::info!("LLM prompt service not enabled");
        None
    };

    // Build the router
    tracing::info!("Building API router");
    let app = create_router(db, broadcast, prompt_service, user_db);

    // Create server
    tracing::info!("Starting orchestrator server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Run server with graceful shutdown
    axum::serve(
        listener,
        app.into_make_service(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    tracing::info!("Orchestrator server shut down gracefully");
    Ok(())
}

/// Signal for graceful shutdown (Ctrl-C or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL-C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received CTRL-C signal, shutting down");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM signal, shutting down");
        }
    }
}
