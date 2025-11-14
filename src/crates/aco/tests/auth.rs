//! Tests for authentication module
//! Task 020: Implement --connect Flag

use aco::{ConnectAuth, CachedToken};

#[test]
fn test_parse_auth_none() {
    let auth = ConnectAuth::from_connect_string("none").unwrap();
    assert!(matches!(auth, ConnectAuth::None));
}

#[test]
fn test_parse_auth_secret() {
    let auth = ConnectAuth::from_connect_string("secret:my-api-key").unwrap();
    match auth {
        ConnectAuth::Secret(key) => {
            assert_eq!(key, "my-api-key");
        }
        _ => panic!("Expected Secret variant"),
    }
}

#[test]
fn test_parse_auth_secret_with_special_chars() {
    let auth = ConnectAuth::from_connect_string("secret:my-api-key-with-special-$pecial").unwrap();
    match auth {
        ConnectAuth::Secret(key) => {
            assert_eq!(key, "my-api-key-with-special-$pecial");
        }
        _ => panic!("Expected Secret variant"),
    }
}

#[test]
fn test_parse_auth_userpass() {
    let auth = ConnectAuth::from_connect_string("alice:password123").unwrap();
    match auth {
        ConnectAuth::UserPass { username, password } => {
            assert_eq!(username, "alice");
            assert_eq!(password, "password123");
        }
        _ => panic!("Expected UserPass variant"),
    }
}

#[test]
fn test_parse_auth_userpass_complex_password() {
    let auth = ConnectAuth::from_connect_string("bob:p@ssw0rd!:with:colons").unwrap();
    match auth {
        ConnectAuth::UserPass { username, password } => {
            assert_eq!(username, "bob");
            assert_eq!(password, "p@ssw0rd!:with:colons");
        }
        _ => panic!("Expected UserPass variant"),
    }
}

#[test]
fn test_parse_auth_token() {
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let auth = ConnectAuth::from_connect_string(&format!("token:{}", jwt)).unwrap();
    match auth {
        ConnectAuth::Token(t) => {
            assert_eq!(t, jwt);
        }
        _ => panic!("Expected Token variant"),
    }
}

#[test]
fn test_parse_auth_invalid_empty() {
    let result = ConnectAuth::from_connect_string("");
    assert!(result.is_err());
}

#[test]
fn test_parse_auth_invalid_colon_only() {
    let result = ConnectAuth::from_connect_string(":");
    assert!(result.is_err());
}

#[test]
fn test_parse_auth_invalid_format() {
    let result = ConnectAuth::from_connect_string("invalid-format");
    assert!(result.is_err());
}

#[test]
fn test_auth_display_none() {
    assert_eq!(ConnectAuth::None.to_string(), "none");
}

#[test]
fn test_auth_display_secret() {
    assert_eq!(ConnectAuth::Secret("key".to_string()).to_string(), "secret");
}

#[test]
fn test_auth_display_userpass() {
    let auth = ConnectAuth::UserPass {
        username: "alice".to_string(),
        password: "pass".to_string(),
    };
    assert!(auth.to_string().contains("alice"));
    assert!(auth.to_string().contains("userpass"));
}

#[test]
fn test_auth_display_token() {
    assert_eq!(ConnectAuth::Token("jwt".to_string()).to_string(), "jwt token");
}

#[test]
fn test_auth_requires_token_exchange() {
    assert!(!ConnectAuth::None.requires_token_exchange());
    assert!(!ConnectAuth::Secret("key".to_string()).requires_token_exchange());
    assert!(ConnectAuth::UserPass {
        username: "user".to_string(),
        password: "pass".to_string()
    }
    .requires_token_exchange());
    assert!(!ConnectAuth::Token("jwt".to_string()).requires_token_exchange());
}

#[test]
fn test_auth_with_token() {
    let userpass_auth = ConnectAuth::UserPass {
        username: "alice".to_string(),
        password: "password".to_string(),
    };
    let token_auth = userpass_auth.with_token("new-jwt-token".to_string());

    match token_auth {
        ConnectAuth::Token(t) => {
            assert_eq!(t, "new-jwt-token");
        }
        _ => panic!("Expected Token variant after with_token"),
    }
}

#[test]
fn test_cached_token_new() {
    let token = CachedToken::new("test-token".to_string());
    assert_eq!(token.token, "test-token");
    assert!(token.expires_at.is_none());
    assert!(token.is_valid());
}

#[test]
fn test_cached_token_with_expiration() {
    let future = chrono::Utc::now() + chrono::Duration::hours(1);
    let token = CachedToken::with_expiration("test-token".to_string(), future);
    assert_eq!(token.token, "test-token");
    assert!(token.expires_at.is_some());
    assert!(token.is_valid());
}

#[test]
fn test_cached_token_expired() {
    let past = chrono::Utc::now() - chrono::Duration::hours(1);
    let token = CachedToken::with_expiration("test-token".to_string(), past);
    assert!(!token.is_valid());
}

#[test]
fn test_cached_token_is_stale() {
    let token = CachedToken::new("test-token".to_string());
    assert!(!token.is_stale(24)); // Not stale within 24 hours
}

#[test]
fn test_cached_token_mode_description() {
    let auth = ConnectAuth::Secret("my-secret".to_string());
    let description = auth.mode_description();
    assert!(description.contains("Auth mode"));
    assert!(description.contains("secret"));
}

#[test]
fn test_multiple_auth_formats() {
    // Test that different formats work independently
    let auths = vec![
        ("none", ConnectAuth::None),
        ("secret:key", ConnectAuth::Secret("key".to_string())),
        ("token:jwt", ConnectAuth::Token("jwt".to_string())),
    ];

    for (input, expected) in auths {
        let parsed = ConnectAuth::from_connect_string(input).unwrap();
        match (parsed, expected) {
            (ConnectAuth::None, ConnectAuth::None) => {}
            (ConnectAuth::Secret(a), ConnectAuth::Secret(b)) => assert_eq!(a, b),
            (ConnectAuth::Token(a), ConnectAuth::Token(b)) => assert_eq!(a, b),
            _ => panic!("Format mismatch for {}", input),
        }
    }
}

#[test]
fn test_auth_parsing_preserves_content() {
    // Ensure parsing doesn't modify the content
    let original_key = "secret-with-special-chars-!@#$%^&*()";
    let auth = ConnectAuth::from_connect_string(&format!("secret:{}", original_key)).unwrap();

    match auth {
        ConnectAuth::Secret(key) => {
            assert_eq!(key, original_key);
        }
        _ => panic!("Expected Secret variant"),
    }
}

#[test]
fn test_token_cache_serialization() {
    let token = CachedToken::new("test-token".to_string());
    let json = serde_json::to_string(&token).unwrap();
    let deserialized: CachedToken = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.token, token.token);
}

#[test]
fn test_userpass_with_email() {
    // Test userpass with email-like username
    let auth = ConnectAuth::from_connect_string("alice@example.com:password123").unwrap();
    match auth {
        ConnectAuth::UserPass { username, password } => {
            assert_eq!(username, "alice@example.com");
            assert_eq!(password, "password123");
        }
        _ => panic!("Expected UserPass variant"),
    }
}
