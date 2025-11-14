# Task 015: Implement Auth Service

## Objective
Implement AuthService for login, logout, and token management.

## Dependencies
- Task 002 (JWT implementation)

## Key Files
- `src/crates/orchestrator/src/services/auth.rs`

## Implementation
- Login RPC: Validate credentials, generate JWT
- Logout RPC: Invalidate token (add to blacklist)
- RefreshToken RPC: Issue new access token
- User lookup from database or config
- Password hashing (bcrypt/argon2)

## For MVP
Can use simple config-based users or skip password verification (dev mode)

## Complexity: Moderate | Effort: 5-6 hours
