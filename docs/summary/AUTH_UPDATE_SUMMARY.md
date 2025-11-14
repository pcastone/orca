# Authentication Architecture Update Summary

## Changes Made

Updated tasks 002, 015, and 020 to implement a flexible multi-mode authentication system.

## New Authentication Architecture

### Server-Side (orchestrator)

**4 Authentication Modes** configured via `AUTH_MODE` environment variable:

1. **No-Auth** (`AUTH_MODE=none`)
   - No authentication required
   - All requests allowed
   - For local development only

2. **Secret** (`AUTH_MODE=secret`)
   - Shared API secret authentication
   - Client provides `x-api-secret` header
   - **ENV**: `AUTH_SECRET=<32+ char secret>`

3. **UserPass** (`AUTH_MODE=userpass`)
   - Username/password authentication
   - Argon2 password hashing
   - Issues JWT tokens
   - **ENV**: `AUTH_USERS=user1:hash1,user2:hash2`

4. **LDAP** (`AUTH_MODE=ldap`)
   - External LDAP directory authentication
   - Issues JWT tokens after validation
   - **ENV**: `LDAP_URL=ldap://...`, `LDAP_BIND_DN_TEMPLATE=...`

### Client-Side (aco)

**Single `--connect` Flag** with 4 formats:

```bash
# 1. No authentication
aco task list
aco --connect none task list

# 2. Secret authentication
aco --connect secret:my-api-secret-key task list

# 3. Username/password (obtains and caches JWT)
aco --connect admin:password123 task list

# 4. Pre-obtained JWT token
aco --connect token:eyJhbGc... task list
```

**Environment Variable Support**:
```bash
export ACO_CONNECT=admin:password123
aco task list  # Uses ACO_CONNECT
```

## Key Features

### Automatic Token Caching
- UserPass/LDAP auth calls `Authenticate` RPC once
- JWT token cached to `~/.aco/token`
- Subsequent commands reuse cached token
- No repeated authentication needed

### Token Validation
- Cached tokens validated before reuse
- Expired/invalid tokens trigger re-authentication
- Seamless user experience

### Security
- Argon2 password hashing (industry standard)
- JWT tokens with expiration
- LDAP over TLS support (configurable)
- Secrets in environment variables (not code)

## File Changes

### Task 002 - Multi-Mode Authentication System
**Before**: Simple JWT auth only
**After**: 4 auth modes with complete implementation

**New Files**:
- `src/crates/orchestrator/src/auth/mode.rs` - AuthMode enum
- `src/crates/orchestrator/src/auth/secret.rs` - Secret validation
- `src/crates/orchestrator/src/auth/userpass.rs` - UserPass with Argon2
- `src/crates/orchestrator/src/auth/ldap.rs` - LDAP integration
- `src/crates/orchestrator/src/auth/jwt.rs` - JWT manager
- `src/crates/orchestrator/src/auth/interceptor.rs` - gRPC interceptor
- `src/crates/aco/src/auth/connect.rs` - --connect parsing

**Dependencies Added**:
- `argon2 = "0.5"` - Password hashing
- `ldap3 = "0.11"` - LDAP client
- `jsonwebtoken = "9.2"` - JWT

### Task 015 - Authenticate RPC
**Before**: Login/Logout RPCs
**After**: Single Authenticate RPC

**Proto Changes**:
```protobuf
service AuthService {
  rpc Authenticate(AuthenticateRequest) returns (AuthenticateResponse);
}
```

**Features**:
- Validates userpass or LDAP credentials
- Returns JWT + expiration + username
- Only registered when `AUTH_MODE` requires JWT
- Client integration for automatic token caching

### Task 020 - Auth CLI Commands
**Before**: `aco login`, `aco logout`, `aco whoami`
**After**: `--connect` flag, `aco status`

**New Commands**:
- `aco --connect <format> <command>` - Global auth flag
- `aco status` - Show connection status

**Removed Commands**:
- `aco login` - Replaced by --connect
- `aco logout` - Not needed (just delete ~/.aco/token)
- `aco whoami` - Merged into status command

## Usage Examples

### Development (No Auth)
```bash
# Server
export AUTH_MODE=none
cargo run -p orchestrator

# Client
aco task list
```

### Production (Secret)
```bash
# Server
export AUTH_MODE=secret
export AUTH_SECRET=my-very-long-secret-key-at-least-32-chars
cargo run -p orchestrator --release

# Client
export ACO_CONNECT=secret:my-very-long-secret-key-at-least-32-chars
aco task create "Production task"
```

### Enterprise (LDAP)
```bash
# Server
export AUTH_MODE=ldap
export LDAP_URL=ldaps://ldap.company.com:636
export LDAP_BIND_DN_TEMPLATE=uid={},ou=users,dc=company,dc=com
export JWT_SECRET=production-jwt-secret-change-me
cargo run -p orchestrator --release

# Client
aco --connect jdoe:ldap-password task list
# Authenticates once, caches JWT to ~/.aco/token
# Subsequent commands use cached token automatically
```

### User/Password (File-based)
```bash
# Generate password hash
cargo run -p orchestrator --bin hash-password
# Enter password: mypass123
# Hash: $argon2id$v=19$m=19456,t=2,p=1$...

# Server
export AUTH_MODE=userpass
export AUTH_USERS=admin:$argon2id$v=19$...,user2:$argon2id$...
export JWT_SECRET=production-jwt-secret
cargo run -p orchestrator --release

# Client (first time)
aco --connect admin:mypass123 task list
# Authenticates, caches JWT

# Client (subsequent)
aco task list  # Uses cached token, no --connect needed
```

## Migration Path

### For Existing Code
1. Remove old login/logout RPC implementations
2. Implement new auth modes as per Task 002
3. Update server to use auth interceptor
4. Update client to use --connect flag
5. Test all 4 auth modes

### For Users
1. **No change** if using AUTH_MODE=none
2. **Set ENV vars** for secret/userpass/LDAP modes
3. **Use --connect** instead of login command
4. **Token caching** eliminates repeated logins

## Security Notes

### Recommendations
- Never use `AUTH_MODE=none` in production
- Use secrets of at least 32 characters
- Store secrets in environment/vault, not code
- Use LDAPS (not LDAP) for encrypted communication
- Rotate JWT_SECRET periodically
- Monitor failed authentication attempts

### Password Hashing
- Argon2id algorithm (OWASP recommended)
- Default params: m=19456, t=2, p=1
- Salted automatically
- Resistant to GPU attacks

## Complexity Estimates

- **Task 002**: 10-12 hours (up from 6-8) - Multiple auth modes
- **Task 015**: 4-5 hours (down from 5-6) - Simpler RPC
- **Task 020**: 5-6 hours (down from 3-4) - More features

**Total Auth Work**: ~20 hours

## Testing Checklist

- [ ] No-auth mode allows all requests
- [ ] Secret mode validates header
- [ ] UserPass mode validates credentials
- [ ] LDAP mode authenticates against directory
- [ ] JWT tokens generated correctly
- [ ] JWT tokens validated on requests
- [ ] Token caching works
- [ ] Expired tokens trigger re-auth
- [ ] --connect parsing for all formats
- [ ] ACO_CONNECT env var works
- [ ] Status command shows connection info

## Documentation Updates Needed

- Update README with auth modes
- Add AUTH_MODES.md with detailed setup
- Add LDAP_SETUP.md for LDAP configuration
- Add security best practices doc
- Update CLI help text

---

**Status**: Tasks updated and committed
**Date**: 2025-01-14
**Next Step**: Implement Task 002 (auth system)
