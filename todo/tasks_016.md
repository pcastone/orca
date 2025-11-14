# Task 016: Set Up aco Client Infrastructure

## Objective
Create aco client infrastructure with gRPC connection, configuration, and auth token management.

## Dependencies
- Task 001 (Proto definitions)
- Task 002 (Auth client)

## Key Files
- `src/crates/aco/src/main.rs` - Entry point
- `src/crates/aco/src/client.rs` - AcoClient struct
- `src/crates/aco/src/config.rs` - ClientConfig

## Implementation
- ClientConfig: server_url, timeout, auth_token_path
- AcoClient: gRPC channel management
- Connection with retry logic
- Auth token loading from ~/.aco/token
- TLS/insecure configuration

## Complexity: Moderate | Effort: 5-6 hours
