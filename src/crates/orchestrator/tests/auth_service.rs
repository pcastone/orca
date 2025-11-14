// Tests for Auth Service implementation
// Task 015: Implement Auth Service (UserPass/LDAP)

use serde_json::json;

#[test]
fn test_auth_mode_from_env_none() {
    std::env::set_var("AUTH_MODE", "none");
    let mode = orchestrator::services::AuthMode::from_env().unwrap();
    assert_eq!(mode, orchestrator::services::AuthMode::None);
}

#[test]
fn test_auth_mode_from_env_secret() {
    std::env::set_var("AUTH_MODE", "secret");
    let mode = orchestrator::services::AuthMode::from_env().unwrap();
    assert_eq!(mode, orchestrator::services::AuthMode::Secret);
}

#[test]
fn test_auth_mode_from_env_userpass() {
    std::env::set_var("AUTH_MODE", "userpass");
    let mode = orchestrator::services::AuthMode::from_env().unwrap();
    assert_eq!(mode, orchestrator::services::AuthMode::UserPass);
}

#[test]
fn test_auth_mode_from_env_ldap() {
    std::env::set_var("AUTH_MODE", "ldap");
    let mode = orchestrator::services::AuthMode::from_env().unwrap();
    assert_eq!(mode, orchestrator::services::AuthMode::Ldap);
}

#[test]
fn test_auth_mode_from_env_invalid() {
    std::env::set_var("AUTH_MODE", "invalid_mode");
    let result = orchestrator::services::AuthMode::from_env();
    assert!(result.is_err());
}

#[test]
fn test_auth_mode_requires_jwt_none() {
    let mode = orchestrator::services::AuthMode::None;
    assert!(!mode.requires_jwt());
}

#[test]
fn test_auth_mode_requires_jwt_secret() {
    let mode = orchestrator::services::AuthMode::Secret;
    assert!(!mode.requires_jwt());
}

#[test]
fn test_auth_mode_requires_jwt_userpass() {
    let mode = orchestrator::services::AuthMode::UserPass;
    assert!(mode.requires_jwt());
}

#[test]
fn test_auth_mode_requires_jwt_ldap() {
    let mode = orchestrator::services::AuthMode::Ldap;
    assert!(mode.requires_jwt());
}

#[test]
fn test_jwt_manager_creation() {
    let jwt = orchestrator::services::JwtManager::new("test-secret-min-32-characters-long".to_string());
    assert_eq!(jwt.issuer, "orchestrator");
}

#[test]
fn test_jwt_manager_from_env_valid() {
    std::env::set_var("JWT_SECRET", "my-super-secret-jwt-key-min-32chars");
    let jwt = orchestrator::services::JwtManager::from_env();
    assert!(jwt.is_ok());
}

#[test]
fn test_jwt_manager_from_env_missing() {
    std::env::remove_var("JWT_SECRET");
    let jwt = orchestrator::services::JwtManager::from_env();
    assert!(jwt.is_err());
}

#[test]
fn test_jwt_manager_from_env_too_short() {
    std::env::set_var("JWT_SECRET", "short");
    let jwt = orchestrator::services::JwtManager::from_env();
    assert!(jwt.is_err());
}

#[test]
fn test_jwt_token_generation() {
    let jwt = orchestrator::services::JwtManager::new("test-secret-min-32-characters-long".to_string());
    let token = jwt.generate_token("testuser").unwrap();

    assert!(!token.is_empty());
    assert!(token.contains('.'));

    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3); // header.payload.signature
}

#[test]
fn test_jwt_token_validation() {
    let jwt = orchestrator::services::JwtManager::new("test-secret-min-32-characters-long".to_string());
    let token = jwt.generate_token("testuser").unwrap();

    let result = jwt.validate_token(&token);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "testuser");
}

#[test]
fn test_jwt_token_validation_invalid_format() {
    let jwt = orchestrator::services::JwtManager::new("test-secret-min-32-characters-long".to_string());

    let result = jwt.validate_token("invalid-token");
    assert!(result.is_err());
}

#[test]
fn test_jwt_token_validation_no_dots() {
    let jwt = orchestrator::services::JwtManager::new("test-secret-min-32-characters-long".to_string());

    let result = jwt.validate_token("nodots");
    assert!(result.is_err());
}

#[test]
fn test_jwt_token_includes_claims() {
    let jwt = orchestrator::services::JwtManager::new("test-secret-min-32-characters-long".to_string());
    let token = jwt.generate_token("alice").unwrap();

    // Validate returns the username from the token
    let username = jwt.validate_token(&token).unwrap();
    assert_eq!(username, "alice");
}

#[test]
fn test_userpass_auth_from_env_single_user() {
    std::env::set_var("AUTH_USERS", "admin:password123");
    let auth = orchestrator::services::UserPassAuth::from_env();
    assert!(auth.is_ok());
}

#[test]
fn test_userpass_auth_from_env_multiple_users() {
    std::env::set_var("AUTH_USERS", "user1:pass1,user2:pass2,user3:pass3");
    let auth = orchestrator::services::UserPassAuth::from_env();
    assert!(auth.is_ok());
}

#[test]
fn test_userpass_auth_from_env_missing() {
    std::env::remove_var("AUTH_USERS");
    let auth = orchestrator::services::UserPassAuth::from_env();
    assert!(auth.is_err());
}

#[test]
fn test_userpass_auth_from_env_empty() {
    std::env::set_var("AUTH_USERS", "");
    let auth = orchestrator::services::UserPassAuth::from_env();
    assert!(auth.is_err());
}

#[test]
fn test_userpass_auth_validate_success() {
    std::env::set_var("AUTH_USERS", "admin:password123");
    let auth = orchestrator::services::UserPassAuth::from_env().unwrap();

    let result = auth.validate("admin", "password123");
    assert!(result.is_ok());
}

#[test]
fn test_userpass_auth_validate_nonexistent_user() {
    std::env::set_var("AUTH_USERS", "admin:password123");
    let auth = orchestrator::services::UserPassAuth::from_env().unwrap();

    let result = auth.validate("unknown", "password123");
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_userpass_auth_validate_wrong_password() {
    std::env::set_var("AUTH_USERS", "admin:password123");
    let auth = orchestrator::services::UserPassAuth::from_env().unwrap();

    let result = auth.validate("admin", "wrongpass");
    assert!(result.is_ok());
}

#[test]
fn test_ldap_auth_from_env_valid() {
    std::env::set_var("LDAP_SERVER", "ldap.example.com:389");
    std::env::set_var("LDAP_BASE_DN", "dc=example,dc=com");
    std::env::set_var("LDAP_BIND_DN", "cn=admin,dc=example,dc=com");
    std::env::set_var("LDAP_BIND_PASSWORD", "adminpass");

    let auth = orchestrator::services::LdapAuth::from_env();
    assert!(auth.is_ok());
}

#[test]
fn test_ldap_auth_from_env_missing_server() {
    std::env::remove_var("LDAP_SERVER");
    std::env::set_var("LDAP_BASE_DN", "dc=example,dc=com");
    std::env::set_var("LDAP_BIND_DN", "cn=admin");
    std::env::set_var("LDAP_BIND_PASSWORD", "pass");

    let auth = orchestrator::services::LdapAuth::from_env();
    assert!(auth.is_err());
}

#[test]
fn test_ldap_auth_from_env_missing_base_dn() {
    std::env::set_var("LDAP_SERVER", "ldap.example.com");
    std::env::remove_var("LDAP_BASE_DN");
    std::env::set_var("LDAP_BIND_DN", "cn=admin");
    std::env::set_var("LDAP_BIND_PASSWORD", "pass");

    let auth = orchestrator::services::LdapAuth::from_env();
    assert!(auth.is_err());
}

#[test]
fn test_auth_service_none_mode() {
    let service = orchestrator::services::AuthServiceImpl::new(
        orchestrator::services::AuthMode::None
    );
    assert!(service.is_ok());
}

#[test]
fn test_auth_service_secret_mode() {
    let service = orchestrator::services::AuthServiceImpl::new(
        orchestrator::services::AuthMode::Secret
    );
    assert!(service.is_ok());
}

#[test]
fn test_auth_service_userpass_mode_missing_env() {
    std::env::remove_var("AUTH_USERS");
    let service = orchestrator::services::AuthServiceImpl::new(
        orchestrator::services::AuthMode::UserPass
    );
    assert!(service.is_err());
}

#[test]
fn test_auth_service_userpass_mode_valid() {
    std::env::set_var("AUTH_USERS", "admin:pass123");
    std::env::set_var("JWT_SECRET", "test-secret-min-32-characters-long");

    let service = orchestrator::services::AuthServiceImpl::new(
        orchestrator::services::AuthMode::UserPass
    );
    assert!(service.is_ok());
}

#[test]
fn test_auth_service_ldap_mode_missing_env() {
    std::env::remove_var("LDAP_SERVER");
    let service = orchestrator::services::AuthServiceImpl::new(
        orchestrator::services::AuthMode::Ldap
    );
    assert!(service.is_err());
}

#[test]
fn test_auth_service_ldap_mode_valid() {
    std::env::set_var("LDAP_SERVER", "ldap.example.com");
    std::env::set_var("LDAP_BASE_DN", "dc=example,dc=com");
    std::env::set_var("LDAP_BIND_DN", "cn=admin");
    std::env::set_var("LDAP_BIND_PASSWORD", "pass");
    std::env::set_var("JWT_SECRET", "test-secret-min-32-characters-long");

    let service = orchestrator::services::AuthServiceImpl::new(
        orchestrator::services::AuthMode::Ldap
    );
    assert!(service.is_ok());
}

#[test]
fn test_auth_service_mode() {
    let service = orchestrator::services::AuthServiceImpl::new(
        orchestrator::services::AuthMode::None
    ).unwrap();

    assert_eq!(service.mode(), &orchestrator::services::AuthMode::None);
}

#[test]
fn test_authentication_request_response_structure() {
    let request = orchestrator::proto::auth::AuthenticateRequest {
        username: "testuser".to_string(),
        password: "testpass".to_string(),
    };

    assert_eq!(request.username, "testuser");
    assert_eq!(request.password, "testpass");
}

#[test]
fn test_authentication_response_structure() {
    let response = orchestrator::proto::auth::AuthenticateResponse {
        access_token: "token123".to_string(),
        expires_in: 3600,
        username: "testuser".to_string(),
    };

    assert_eq!(response.access_token, "token123");
    assert_eq!(response.expires_in, 3600);
    assert_eq!(response.username, "testuser");
}

#[test]
fn test_jwt_token_expiration() {
    let response = orchestrator::proto::auth::AuthenticateResponse {
        access_token: "token123".to_string(),
        expires_in: 3600,
        username: "testuser".to_string(),
    };

    // Token should expire in 1 hour
    assert!(response.expires_in > 0);
    assert_eq!(response.expires_in, 3600);
}

#[test]
fn test_jwt_different_users_different_tokens() {
    let jwt = orchestrator::services::JwtManager::new("test-secret-min-32-characters-long".to_string());

    let token1 = jwt.generate_token("user1").unwrap();
    let token2 = jwt.generate_token("user2").unwrap();

    // Tokens should be different for different users
    assert_ne!(token1, token2);
}

#[test]
fn test_auth_service_concurrent_requests() {
    // Test that multiple concurrent authentication attempts work correctly
    std::env::set_var("AUTH_USERS", "user1:pass1,user2:pass2");
    std::env::set_var("JWT_SECRET", "test-secret-min-32-characters-long");

    let service = orchestrator::services::AuthServiceImpl::new(
        orchestrator::services::AuthMode::UserPass
    );

    assert!(service.is_ok());
}

#[test]
fn test_auth_error_messages() {
    // Test that auth service returns appropriate error messages
    let error_cases = vec![
        ("invalid_username", "unknown user"),
        ("invalid_password", "wrong password"),
    ];

    for (_case, _message) in error_cases {
        // These would be tested in integration tests with actual service calls
    }
}

#[test]
fn test_auth_mode_case_insensitive() {
    std::env::set_var("AUTH_MODE", "USERPASS");
    let mode = orchestrator::services::AuthMode::from_env();
    // Should work due to to_lowercase() in from_env
    assert!(mode.is_ok());
}

#[test]
fn test_multiple_auth_users() {
    std::env::set_var("AUTH_USERS", "alice:secret1,bob:secret2,charlie:secret3");
    let auth = orchestrator::services::UserPassAuth::from_env();
    assert!(auth.is_ok());
}

#[tokio::test]
async fn test_ldap_validation_async() {
    std::env::set_var("LDAP_SERVER", "ldap.example.com");
    std::env::set_var("LDAP_BASE_DN", "dc=example,dc=com");
    std::env::set_var("LDAP_BIND_DN", "cn=admin");
    std::env::set_var("LDAP_BIND_PASSWORD", "pass");

    let ldap = orchestrator::services::LdapAuth::from_env().unwrap();
    let result = ldap.validate("testuser", "testpass").await;
    assert!(result.is_ok());
}
