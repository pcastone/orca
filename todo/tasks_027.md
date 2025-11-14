# Task 027: Fix Existing Test Compilation Errors

## Objective
Fix all failing tests across the workspace to establish clean baseline.

## Dependencies
- All implementation tasks

## Scope
Review and fix:
- All crates in src/crates/
- Update import paths for new architecture
- Fix type mismatches from refactoring
- Update mock/fixture data
- Ensure all unit tests compile and pass

## Steps
1. Run `cargo test --all` and collect errors
2. Fix compilation errors crate by crate
3. Update tests for new gRPC architecture
4. Remove obsolete tests
5. Verify all tests pass

## Complexity: Moderate | Effort: 8-10 hours
