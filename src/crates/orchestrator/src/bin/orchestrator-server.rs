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

    // Build the router
    tracing::info!("Building API router");
    let app = create_router(db, broadcast);

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
