// Tests for aco client infrastructure
// Task 016: Set Up aco Client Infrastructure

use aco::{AcoClient, ClientConfig, AuthToken};
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn test_client_config_default_values() {
    let config = ClientConfig::default();

    assert_eq!(config.server_url, "http://localhost:50051");
    assert_eq!(config.timeout_seconds, 30);
    assert!(!config.use_tls);
    assert!(config.cert_path.is_none());
}

#[test]
fn test_client_config_custom_server_url() {
    let config = ClientConfig::new("http://example.com:8080".to_string());

    assert_eq!(config.server_url, "http://example.com:8080");
}

#[test]
fn test_client_config_with_tls() {
    let config = ClientConfig::default().with_tls(None);

    assert!(config.use_tls);
    assert!(config.cert_path.is_none());
}

#[test]
fn test_client_config_with_tls_and_cert() {
    let cert_path = PathBuf::from("/path/to/cert.pem");
    let config = ClientConfig::default().with_tls(Some(cert_path.clone()));

    assert!(config.use_tls);
    assert_eq!(config.cert_path, Some(cert_path));
}

#[test]
fn test_client_config_with_timeout() {
    let config = ClientConfig::default().with_timeout(60);

    assert_eq!(config.timeout_seconds, 60);
}

#[test]
fn test_client_config_with_token_path() {
    let token_path = PathBuf::from("/custom/token/path");
    let config = ClientConfig::default().with_token_path(token_path.clone());

    assert_eq!(config.token_path, token_path);
}

#[test]
fn test_client_config_chaining() {
    let config = ClientConfig::new("http://localhost:50051".to_string())
        .with_timeout(90)
        .with_tls(None);

    assert_eq!(config.server_url, "http://localhost:50051");
    assert_eq!(config.timeout_seconds, 90);
    assert!(config.use_tls);
}

#[test]
fn test_retry_config_defaults() {
    let config = ClientConfig::default();

    assert_eq!(config.retry_config.max_retries, 3);
    assert_eq!(config.retry_config.initial_backoff_ms, 100);
    assert_eq!(config.retry_config.max_backoff_ms, 5000);
    assert_eq!(config.retry_config.backoff_multiplier, 2.0);
}

#[test]
fn test_auth_token_creation_with_username() {
    let token = AuthToken::new(
        "eyJhbGc...".to_string(),
        3600,
        Some("alice".to_string()),
    );

    assert_eq!(token.access_token, "eyJhbGc...");
    assert_eq!(token.username, Some("alice".to_string()));
}

#[test]
fn test_auth_token_creation_without_username() {
    let token = AuthToken::new(
        "eyJhbGc...".to_string(),
        3600,
        None,
    );

    assert_eq!(token.access_token, "eyJhbGc...");
    assert!(token.username.is_none());
}

#[test]
fn test_auth_token_not_expired() {
    let token = AuthToken::new(
        "token".to_string(),
        3600,
        None,
    );

    assert!(!token.is_expired());
}

#[test]
fn test_auth_token_expiration_check() {
    let token = AuthToken::new(
        "token".to_string(),
        0,
        None,
    );

    // Wait a bit to ensure time has passed
    std::thread::sleep(Duration::from_millis(10));
    assert!(token.is_expired());
}

#[test]
fn test_auth_token_time_to_expiration() {
    let token = AuthToken::new(
        "token".to_string(),
        100,
        None,
    );

    let time_left = token.time_to_expiration();
    assert!(time_left > 0);
    assert!(time_left <= 100);
}

#[test]
fn test_auth_token_serialization() {
    let token = AuthToken::new(
        "test-token".to_string(),
        3600,
        Some("testuser".to_string()),
    );

    let json = serde_json::to_string(&token).unwrap();
    assert!(!json.is_empty());

    let deserialized: AuthToken = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.access_token, token.access_token);
    assert_eq!(deserialized.username, token.username);
}

#[tokio::test]
async fn test_aco_client_creation() {
    let config = ClientConfig::default();
    let client = AcoClient::new(config);

    assert!(!client.is_connected());
}

#[tokio::test]
async fn test_aco_client_from_url() {
    let url = "http://orchestrator.example.com:50051".to_string();
    let client = AcoClient::from_url(url.clone());

    assert_eq!(client.config().server_url, url);
}

#[tokio::test]
async fn test_aco_client_config_access() {
    let config = ClientConfig::new("http://localhost:8080".to_string());
    let client = AcoClient::new(config);

    assert_eq!(client.config().server_url, "http://localhost:8080");
}

#[tokio::test]
async fn test_aco_client_connection_status() {
    let client = AcoClient::new(ClientConfig::default());

    assert!(!client.is_connected());
}

#[tokio::test]
async fn test_aco_client_authentication_status_unauthenticated() {
    let client = AcoClient::new(ClientConfig::default());

    assert!(!client.is_authenticated().await);
}

#[tokio::test]
async fn test_aco_client_set_token() {
    let client = AcoClient::new(ClientConfig::default());
    let token = AuthToken::new("test-token".to_string(), 3600, Some("testuser".to_string()));

    client.set_token(token.clone()).await;

    let retrieved = client.get_token().await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().access_token, "test-token");
}

#[tokio::test]
async fn test_aco_client_get_token() {
    let client = AcoClient::new(ClientConfig::default());

    let token = client.get_token().await;
    assert!(token.is_none());
}

#[tokio::test]
async fn test_aco_client_clear_token() {
    let client = AcoClient::new(ClientConfig::default());
    let token = AuthToken::new("test-token".to_string(), 3600, None);

    client.set_token(token).await;
    assert!(client.get_token().await.is_some());

    let _ = client.clear_token().await;
    assert!(client.get_token().await.is_none());
}

#[tokio::test]
async fn test_token_manager_basic_operations() {
    let temp_dir = std::env::temp_dir().join("aco_test_token_manager");
    let token_path = temp_dir.join("token.json");

    let manager = aco::TokenManager::new(token_path.clone());

    // Should be None initially
    assert!(manager.get_token().await.is_none());
}

#[tokio::test]
async fn test_token_manager_save_and_load() {
    let temp_dir = std::env::temp_dir().join("aco_test_token_save_load");
    let token_path = temp_dir.join("token.json");

    let manager = aco::TokenManager::new(token_path.clone());
    let token = AuthToken::new("test-token".to_string(), 3600, Some("user".to_string()));

    // Save token
    if let Err(e) = manager.save_token(token.clone()).await {
        eprintln!("Failed to save token: {}", e);
    }

    // Token should be available after saving
    let retrieved = manager.get_token().await;
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_token_manager_clear() {
    let temp_dir = std::env::temp_dir().join("aco_test_token_clear");
    let token_path = temp_dir.join("token.json");

    let manager = aco::TokenManager::new(token_path.clone());
    let token = AuthToken::new("test-token".to_string(), 3600, None);

    let _ = manager.save_token(token).await;
    assert!(manager.get_token().await.is_some());

    let _ = manager.clear_token().await;
    assert!(manager.get_token().await.is_none());
}

#[test]
fn test_client_config_server_url_variations() {
    let urls = vec![
        "http://localhost:50051",
        "https://orchestrator.example.com:50051",
        "http://192.168.1.1:50051",
        "http://[::1]:50051", // IPv6
    ];

    for url in urls {
        let config = ClientConfig::new(url.to_string());
        assert_eq!(config.server_url, url);
    }
}

#[test]
fn test_client_config_timeout_variations() {
    let timeouts = vec![1, 10, 30, 60, 300];

    for timeout in timeouts {
        let config = ClientConfig::default().with_timeout(timeout);
        assert_eq!(config.timeout_seconds, timeout);
    }
}

#[tokio::test]
async fn test_client_authentication_workflow() {
    let client = AcoClient::new(ClientConfig::default());

    // Initially not authenticated
    assert!(!client.is_authenticated().await);

    // Set a token
    let token = AuthToken::new("token123".to_string(), 3600, Some("alice".to_string()));
    client.set_token(token).await;

    // Now authenticated
    assert!(client.is_authenticated().await);
}

#[test]
fn test_auth_token_expiration_calculation() {
    let token = AuthToken::new("token".to_string(), 3600, None);

    let time_left = token.time_to_expiration();
    assert!(time_left > 3500 && time_left <= 3600);
}

#[tokio::test]
async fn test_client_load_token_no_file() {
    let temp_dir = std::env::temp_dir().join("aco_test_no_token_file");
    let token_path = temp_dir.join("nonexistent_token.json");

    let config = ClientConfig::default().with_token_path(token_path);
    let client = AcoClient::new(config);

    // Should not error even if token file doesn't exist
    let result = client.load_token().await;
    assert!(result.is_ok());
}

#[test]
fn test_client_retry_config_exponential_backoff() {
    let config = ClientConfig::default();
    let mut backoff = config.retry_config.initial_backoff_ms as f64;

    for _ in 0..3 {
        backoff *= config.retry_config.backoff_multiplier;
        assert!(backoff <= config.retry_config.max_backoff_ms as f64);
    }
}

#[tokio::test]
async fn test_multiple_clients_independent_tokens() {
    let client1 = AcoClient::new(ClientConfig::default());
    let client2 = AcoClient::new(ClientConfig::default());

    let token1 = AuthToken::new("token1".to_string(), 3600, Some("alice".to_string()));
    let token2 = AuthToken::new("token2".to_string(), 3600, Some("bob".to_string()));

    client1.set_token(token1).await;
    client2.set_token(token2).await;

    // Each client has its own token
    let c1_token = client1.get_token().await.unwrap();
    let c2_token = client2.get_token().await.unwrap();

    assert_eq!(c1_token.access_token, "token1");
    assert_eq!(c2_token.access_token, "token2");
}
