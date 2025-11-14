# Orca Crate Testing Plan

## Test Infrastructure Requirements

### Current State
- Basic unit tests exist for enum types and simple logic
- No test database infrastructure for integration tests
- Tests that require `DatabaseManager` currently fail

### Required Infrastructure

#### 1. Test Database Helper
Create a test helper module that provides:
- Temporary database creation for tests
- Automatic cleanup after tests
- Fixture data loading
- Mock/stub database for unit tests

```rust
// Proposed: src/testing/mod.rs
pub async fn create_test_db() -> Result<DatabaseManager> {
    let temp_dir = tempfile::tempdir()?;
    DatabaseManager::new(temp_dir.path()).await
}
```

#### 2. Test Fixtures
- Sample permission configurations
- Test workflow and task data
- Mock LLM responses

## Critical Security Tests Needed

### Phase 1: Permission Enforcer (CRITICAL)

File: `src/tools/permission_enforcer.rs`

#### Path Traversal Prevention Tests
- [ ] Test `../` escape attempts
- [ ] Test multiple `../../` traversals
- [ ] Test absolute path bypasses (`/etc/passwd`)
- [ ] Test encoded traversal attempts (`%2e%2e%2f`)
- [ ] Test null byte injection
- [ ] Test symlink following
- [ ] Test Windows path separators (`\\`)
- [ ] Test mixed separators (`/..\\`)

#### Whitelist Validation Tests
- [ ] Empty whitelist behavior
- [ ] Pattern matching accuracy
- [ ] Case sensitivity
- [ ] Regex injection attempts
- [ ] Unicode/UTF-8 handling
- [ ] Partial match behavior

#### Blacklist Validation Tests
- [ ] Dangerous command patterns (rm -rf, dd, fork bombs)
- [ ] Case sensitivity bypass attempts
- [ ] Encoding bypass attempts
- [ ] Command injection patterns
- [ ] Shell metacharacter handling

#### Permission Level Tests
- [ ] Default behavior for each level (Allowed, Denied, RequiresApproval, Restricted)
- [ ] Permission escalation attempts
- [ ] Missing permission handling
- [ ] Conflicting permissions

#### Audit Logging Tests
- [ ] All execution attempts logged
- [ ] Denied operations logged with reason
- [ ] Log tampering prevention
- [ ] Log retention
- [ ] Sensitive data redaction in logs

### Phase 2: Direct Bridge (CRITICAL)

File: `src/tools/direct_bridge.rs`

#### Sandboxing Tests
- [ ] Resource isolation
- [ ] Filesystem access restrictions
- [ ] Network access controls
- [ ] Process limits
- [ ] Memory limits
- [ ] Timeout enforcement
- [ ] Escape attempt detection

#### Error Propagation Tests
- [ ] Tool execution failures
- [ ] Timeout errors
- [ ] Permission denied errors
- [ ] Resource exhaustion

### Phase 3: Database Operations (HIGH)

Files: `src/db/*.rs`, `src/repositories/*.rs`

#### Concurrent Access Tests
- [ ] Multiple simultaneous reads
- [ ] Concurrent writes
- [ ] Transaction isolation
- [ ] Deadlock prevention
- [ ] Connection pool exhaustion

#### Migration Tests
- [ ] Forward migrations
- [ ] Rollback scenarios
- [ ] Data integrity during migration
- [ ] Migration failure recovery

#### SQL Injection Tests
- [ ] Parameterized query validation
- [ ] User input sanitization
- [ ] Dynamic query construction safety

## Test Organization

### Unit Tests
Location: Inline `#[cfg(test)]` modules or `tests/unit/`
- Pure function logic
- No database dependencies
- Fast execution
- High coverage of edge cases

### Integration Tests
Location: `tests/integration/`
- Database interactions
- Multi-component workflows
- End-to-end scenarios
- Slower but comprehensive

### Security Tests
Location: `tests/security/`
- Penetration testing scenarios
- Fuzzing inputs
- Known vulnerability patterns
- Compliance validation

## Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# Security tests
cargo test --test security_*

# Ignored tests (require setup)
cargo test -- --ignored

# With coverage
cargo tarpaulin --out Html
```

## Test Coverage Goals

- **Security-critical code**: 100% coverage
  - `permission_enforcer.rs`
  - `direct_bridge.rs`
  - Authentication/authorization logic

- **Core functionality**: 80%+ coverage
  - Database operations
  - Workflow execution
  - LLM provider integration

- **Utilities**: 70%+ coverage
  - Configuration loading
  - Error handling
  - Logging

## Next Steps

1. **Set up test infrastructure** (current task)
   - Create `src/testing/mod.rs` helper module
   - Add `tempfile` dependency for temp databases
   - Create test database initialization helpers

2. **Implement security tests**
   - Start with permission enforcer tests
   - Add direct bridge sandboxing tests
   - Create database security tests

3. **Add integration tests**
   - End-to-end workflow execution
   - Multi-component interactions
   - Error recovery scenarios

4. **Set up CI/CD**
   - Automated test running
   - Coverage reporting
   - Security scan integration

## Dependencies Needed

Add to `Cargo.toml`:
```toml
[dev-dependencies]
tempfile = "3.10"
mockall = "0.12" # For mocking
proptest = "1.4" # For property-based testing
criterion = "0.5" # For benchmarking
```

## Security Testing Tools

- `cargo-audit` - Dependency vulnerability scanning
- `cargo-deny` - License and security policy enforcement
- `cargo-fuzz` - Fuzzing framework integration
- `tarpaulin` - Code coverage measurement
