# Tooling Crate

Common utilities and helpers for the acolib workspace. This crate provides reusable patterns and utilities for configuration management, error handling, async operations, validation, serialization, rate limiting, and logging.

## Features

### Configuration Management (`config`)

Provides consistent patterns for loading and managing configuration:

```rust
use tooling::config::{ConfigBuilder, get_env_parse, get_env_bool};

#[derive(Clone, Default)]
struct AppConfig {
    pub port: u16,
    pub debug: bool,
}

impl ConfigBuilder for AppConfig {
    fn from_env(prefix: &str) -> tooling::Result<Self> {
        use tooling::config::{get_env_parse_or, get_env_bool};

        Ok(Self {
            port: get_env_parse_or(&format!("{}PORT", prefix), 8080)?,
            debug: get_env_bool(&format!("{}DEBUG", prefix))?.unwrap_or(false),
        })
    }
}

// Load config from environment with defaults
let config = AppConfig::from_env_with_defaults("APP_")?;
```

### Error Handling (`error`)

Add context to errors and format error chains:

```rust
use tooling::error::{ErrorContext, format_error_chain};

fn read_config() -> Result<String, Box<dyn std::error::Error>> {
    std::fs::read_to_string("config.json")
        .context("Failed to read configuration file")?;
    Ok(contents)
}

// Format error chains for display
match operation() {
    Err(e) => eprintln!("{}", format_error_chain(&*e)),
    Ok(_) => {}
}
```

### Async Utilities (`async_utils`)

Retry policies with exponential backoff and timeout utilities:

```rust
use tooling::async_utils::retry::{RetryPolicy, with_retry};
use tooling::async_utils::timeout::{with_timeout, TimeoutGuard};
use std::time::Duration;

// Retry with exponential backoff
let policy = RetryPolicy::new(3)
    .with_initial_interval(1.0)
    .with_backoff_factor(2.0);

let result = with_retry(&policy, || async {
    call_external_api().await
}).await?;

// Timeout for operations
let result = with_timeout(
    Duration::from_secs(30),
    slow_operation()
).await?;

// RAII timeout guard
let guard = TimeoutGuard::new(Duration::from_secs(60));
guard.execute(async {
    // Operation with timeout
    Ok("done")
}).await?;
```

### Validation (`validation`)

Fluent validation API with chainable rules:

```rust
use tooling::validation::Validator;

// Validate numbers
let age = 25;
Validator::new(age, "age")
    .min(0)
    .max(120)
    .validate()?;

// Validate strings
let email = "user@example.com";
Validator::new(email, "email")
    .not_empty()
    .min_length(3)
    .max_length(100)
    .matches(r"^[^@]+@[^@]+\.[^@]+$")
    .validate()?;

// Custom validation
Validator::new(value, "value")
    .custom(|v| {
        if v % 2 == 0 {
            Ok(())
        } else {
            Err("Value must be even".to_string())
        }
    })
    .validate()?;

// Collect all errors
match Validator::new(value, "value")
    .min(0)
    .max(100)
    .validate_all() {
    Ok(v) => println!("Valid: {}", v),
    Err(errors) => {
        for error in errors {
            eprintln!("Error: {}", error);
        }
    }
}
```

### Serialization (`serialization`)

Stable JSON serialization and hashing for cache keys:

```rust
use tooling::serialization::{
    generate_hash, generate_json_hash, stable_json_string,
    truncate_json, pretty_json
};
use serde_json::json;

// Generate cache keys
let hash = generate_json_hash(&json_value);

// Stable serialization (sorted keys)
let val = json!({"b": 2, "a": 1});
let stable = stable_json_string(&val)?;
// Output: {"a":1,"b":2}

// Truncate for logging
let large_json = "...";
let truncated = truncate_json(large_json, 100);
```

### Rate Limiting (`rate_limit`)

Token bucket and sliding window rate limiters:

```rust
use tooling::rate_limit::{RateLimiter, SlidingWindowLimiter};
use std::time::Duration;

// Token bucket: 100 requests per minute
let limiter = RateLimiter::new(100, Duration::from_secs(60));

if limiter.check().await {
    // Perform operation
} else {
    // Rate limited
}

// Wait until allowed (blocking)
limiter.acquire().await;

// Sliding window limiter
let limiter = SlidingWindowLimiter::new(1000, Duration::from_secs(60));
if limiter.check().await {
    // Operation allowed
}
```

### Logging (`logging`)

Structured logging helpers and formatters:

```rust
use tooling::logging::{
    LogGuard, timed, format_duration, format_bytes, sanitize_for_logging
};

// RAII logging guard
fn process() {
    let _guard = LogGuard::new("process");
    // Logs entry and exit automatically
}

// Time async operations
let result = timed("api_call", async {
    call_api().await
}).await;

// Format helpers
let duration_str = format_duration(Duration::from_millis(1500));
// Output: "1.50s"

let size_str = format_bytes(1024 * 1024);
// Output: "1.00 MB"

// Sanitize sensitive data for logging
let log = "API key: sk-abc123";
let safe = sanitize_for_logging(&log);
// Output: "API key: [REDACTED]"
```

## Testing

All modules have comprehensive test coverage:

```bash
# Run all tests
cargo test -p tooling

# Run specific module tests
cargo test -p tooling config::
cargo test -p tooling validation::
```

## Dependencies

- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `thiserror` / `anyhow` - Error handling
- `tracing` - Structured logging
- `regex` - Pattern matching
- `rand` - Random number generation (for jitter)

## Integration

The tooling crate is designed to be used across the entire acolib workspace:

```toml
[dependencies]
tooling = { path = "../tooling" }
```

## Module Overview

| Module | Purpose | Key Features |
|--------|---------|-------------|
| `config` | Configuration management | Env var loading, builder pattern, validation |
| `error` | Error handling | Context chains, formatting, root cause |
| `async_utils` | Async operations | Retry policies, timeouts, exponential backoff |
| `validation` | Data validation | Fluent API, chainable rules, custom validators |
| `serialization` | JSON utilities | Stable hashing, truncation, pretty printing |
| `rate_limit` | Rate limiting | Token bucket, sliding window |
| `logging` | Structured logging | Timing, formatting, sanitization |

## Design Principles

1. **Ergonomic APIs** - Fluent, chainable interfaces
2. **Type Safety** - Leverage Rust's type system
3. **Composability** - Mix and match utilities
4. **Performance** - Zero-cost abstractions where possible
5. **Testing** - Comprehensive test coverage
6. **Documentation** - Clear examples and use cases

## Examples

See the `examples/` directory for complete usage examples:

```bash
cargo run --example config
cargo run --example validation
cargo run --example retry
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
