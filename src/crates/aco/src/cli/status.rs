//! Status command handler for connection and authentication info

use crate::auth::ConnectAuth;
use crate::error::Result;
use serde_json::json;
use tracing::info;

/// Display connection status information
pub async fn handle_status(auth: &ConnectAuth, server_url: &str) -> Result<()> {
    info!("Getting connection status");

    let status_json = json!({
        "status": "connected",
        "server": server_url,
        "auth_mode": auth.to_string(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    println!("{}", serde_json::to_string_pretty(&status_json)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_status_none_auth() {
        let result = handle_status(&ConnectAuth::None, "http://localhost:50051").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_status_secret_auth() {
        let result = handle_status(
            &ConnectAuth::Secret("test-secret".to_string()),
            "http://localhost:50051",
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_status_token_auth() {
        let result = handle_status(
            &ConnectAuth::Token("test-token".to_string()),
            "http://localhost:50051",
        )
        .await;
        assert!(result.is_ok());
    }
}
