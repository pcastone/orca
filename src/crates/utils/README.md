# Utils Crate

Utility functions and helpers for acolib.

## Overview

This crate provides common utilities for HTTP servers, clients, and configuration management. It's designed to complement the acolib ecosystem with reusable components.

## Modules

### Server (`server`)

HTTP server utilities including configuration builders and helpers.

**Features:**
- Server configuration with builder pattern
- Timeout and connection limit settings
- CORS and logging support
- Socket address management
- Environment variable loading

**Example:**

```rust
use utils::server::{ServerConfig, ServerBuilder};
use std::time::Duration;

let config = ServerBuilder::new()
    .bind("0.0.0.0", 8080)
    .timeout(Duration::from_secs(30))
    .max_connections(1000)
    .with_logging()
    .with_cors()
    .build();

println!("Server will bind to: {:?}", config.socket_addr()?);
```

### Client (`client`)

HTTP client utilities with retry logic and authentication helpers.

**Features:**
- HTTP client with configurable retry logic
- Exponential backoff for retries
- User agent and custom header support
- Authentication helpers (Bearer token, Basic auth, API key)
- Request builders

**Example:**

```rust
use utils::client::{ClientConfig, HttpClient, AuthHelper};
use std::time::Duration;

let config = ClientConfig::new()
    .with_timeout(Duration::from_secs(30))
    .with_max_retries(3)
    .with_user_agent("my-app")
    .with_header("X-Custom", "value");

let client = HttpClient::new(config)?;

// Make a request
let response = client.get("https://api.example.com/data").await?;

// POST with JSON
let body = serde_json::json!({"key": "value"});
let response = client.post_json("https://api.example.com/data", &body).await?;

// Authentication helpers
let auth = AuthHelper::bearer_token("my-token");
```

### Config (`config`)

Configuration management utilities for environment variables and file loading.

**Features:**
- Environment variable loading with type parsing
- Boolean environment variable parsing
- YAML and JSON config file loading
- Default value support
- Configuration validation traits

**Example:**

```rust
use utils::config::{get_env, get_env_parse, get_env_bool, load_config_file};
use serde::Deserialize;

#[derive(Deserialize)]
struct AppConfig {
    api_key: String,
    port: u16,
    debug: bool,
}

// Load from environment
let api_key = get_env("API_KEY")?;
let port = get_env_parse::<u16>("PORT")?;
let debug = get_env_bool("DEBUG")?;

// With defaults
let host = get_env_or("HOST", "127.0.0.1");
let timeout = get_env_parse_or("TIMEOUT", 30);

// Load from file (auto-detects YAML/JSON from extension)
let config: AppConfig = load_config_file("config.yaml")?;
```

## Features

- `default` - Enables all modules (server, client, config)
- `server` - Server utilities only
- `client` - Client utilities only
- `config` - Configuration utilities only

## Re-exports

Common types are re-exported at the crate root for convenience:

```rust
use utils::{
    // Server
    ServerConfig, ServerBuilder,
    
    // Client
    ClientConfig, HttpClient, AuthHelper,
    
    // Config
    get_env, get_env_parse, get_env_bool,
    load_config_file,
    
    // Error types
    Result, UtilsError,
};
```

## Error Handling

All modules use a common `UtilsError` type with the following variants:
- `HttpError` - HTTP request errors
- `SerializationError` - JSON/YAML serialization errors
- `ConfigError` - Configuration errors
- `IoError` - File I/O errors
- `EnvError` - Environment variable errors
- `InvalidInput` - Invalid input data
- `ServerError` - Server-specific errors
- `ClientError` - Client-specific errors

## Dependencies

- `reqwest` - HTTP client
- `tokio` - Async runtime
- `serde` / `serde_json` / `serde_yaml` - Serialization
- `base64` - Base64 encoding for authentication
- `thiserror` - Error handling

