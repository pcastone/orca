# Unit Test Review - Test Coverage Plan

**Date:** 2025-11-16
**Status:** Identified gaps and prioritized implementation

## Executive Summary

The codebase has **2,332 existing test functions** across **296 source files** with **strong overall coverage**. However, critical gaps exist in:
1. **langgraph-cli** - Zero test coverage (CRITICAL)
2. **tooling/rate_limit** - No tests for production-critical functionality (CRITICAL)
3. **llm/error.rs** - No error handling tests (HIGH)
4. **langgraph-checkpoint** - Extended channels undertested (HIGH)
5. **langgraph-prebuilt/error.rs** - No error module tests (MEDIUM)

## Build Validation Results

✅ **All workspace libraries build successfully** (`cargo check --workspace --lib`)
⚠️  Some example files missing (doesn't affect core functionality)
⚠️  Minor warnings about unused variables (non-critical)

---

## Priority 1: Critical Missing Tests

### 1. langgraph-cli (0 tests → Need 20-30 tests)
**File:** `src/crates/langgraph-cli/src/main.rs`
**Status:** No tests whatsoever
**Impact:** Critical - CLI is user-facing

**Required Tests:**
- [ ] CLI argument parsing
- [ ] Command execution (init, new, dev, etc.)
- [ ] Help text generation
- [ ] Error handling for invalid commands
- [ ] Config file creation
- [ ] Project scaffolding
- [ ] Workspace initialization

**Implementation:** Create `src/crates/langgraph-cli/tests/cli_tests.rs`

---

### 2. tooling/rate_limit (0 tests → Need 15-20 tests)
**Files:**
- `src/crates/tooling/src/rate_limit/token_bucket.rs`
- `src/crates/tooling/src/rate_limit/sliding_window.rs`

**Status:** Production-critical rate limiting with zero tests
**Impact:** Critical - affects API stability and fairness

**Required Tests:**
- [ ] Token bucket refill rate
- [ ] Token bucket capacity limits
- [ ] Token bucket concurrent access
- [ ] Sliding window accuracy
- [ ] Sliding window time boundaries
- [ ] Rate limit enforcement under load
- [ ] Burst handling
- [ ] Clock skew scenarios

**Implementation:** Add tests to existing module files

---

### 3. llm/error.rs (0 tests → Need 20-25 tests)
**File:** `src/crates/llm/src/error.rs`
**Status:** Error handling module with no tests
**Impact:** High - affects error reporting and debugging

**Required Tests:**
- [ ] Error type conversion
- [ ] Error context preservation
- [ ] Error display formatting
- [ ] API error parsing
- [ ] Network error handling
- [ ] Authentication error types
- [ ] Rate limit error detection
- [ ] Timeout error scenarios
- [ ] Invalid response error handling
- [ ] Error chaining and wrapping

**Implementation:** Create `tests/error_tests.rs` in llm crate

---

## Priority 2: Important Missing Tests

### 4. langgraph-checkpoint Extended Channels (Partial → Need 30-40 tests)
**File:** `src/crates/langgraph-checkpoint/src/channels_extended.rs`
**Status:** Advanced channels undertested
**Impact:** High - affects state management reliability

**Required Tests:**
- [ ] EphemeralValueChannel lifecycle
- [ ] EphemeralValueChannel cleanup
- [ ] AnyValueChannel conflict resolution
- [ ] AnyValueChannel metadata tracking
- [ ] UntrackedValueChannel isolation
- [ ] NamedBarrierValueChannel synchronization
- [ ] DynamicBarrierValueChannel coordination
- [ ] EdgeChannel with guards
- [ ] Channel version tracking
- [ ] Concurrent channel access patterns

**Implementation:** Expand existing inline tests

---

### 5. langgraph-prebuilt/error.rs (0 tests → Need 10-15 tests)
**File:** `src/crates/langgraph-prebuilt/src/error.rs`
**Status:** No tests for error module
**Impact:** Medium - affects agent error handling

**Required Tests:**
- [ ] ToolExecutionError variants
- [ ] ValidationError formatting
- [ ] Error conversion from dependencies
- [ ] Error context building
- [ ] Error display messages
- [ ] Error debugging information

**Implementation:** Add inline tests to error.rs

---

## Priority 3: Enhanced Coverage

### 6. langgraph-core/cache.rs (Basic tests → Need 25-30 tests)
**File:** `src/crates/langgraph-core/src/cache.rs`
**Status:** Basic coverage, missing contention scenarios
**Impact:** Medium - affects performance

**Required Tests:**
- [ ] LRU eviction under concurrent writes
- [ ] LFU frequency tracking accuracy
- [ ] FIFO ordering with high throughput
- [ ] TTL expiration edge cases
- [ ] Cache contention with 100+ threads
- [ ] Memory pressure scenarios
- [ ] Cache invalidation patterns

---

### 7. llm Thinking Model Support (Partial → Need 15-20 tests)
**Files:**
- `src/crates/llm/src/remote/openai.rs` (o1 models)
- `src/crates/llm/src/remote/deepseek.rs` (R1 models)

**Status:** Basic coverage, missing thinking-specific scenarios
**Impact:** Medium - affects advanced model usage

**Required Tests:**
- [ ] Reasoning token extraction (o1)
- [ ] Thinking block parsing (R1)
- [ ] Cost calculation including reasoning tokens
- [ ] Streaming with reasoning content
- [ ] Thinking truncation handling
- [ ] Model-specific parameter validation

---

## Test Implementation Schedule

### Phase 1: Critical (Week 1)
1. **langgraph-cli** - Complete CLI test suite (Days 1-2)
2. **tooling/rate_limit** - Rate limiter tests (Days 3-4)
3. **llm/error.rs** - Error handling tests (Day 5)

### Phase 2: Important (Week 2)
4. **langgraph-checkpoint** - Extended channel tests (Days 1-3)
5. **langgraph-prebuilt/error.rs** - Error module tests (Day 4)
6. **Verification** - Run all tests, fix failures (Day 5)

### Phase 3: Enhanced (Week 3)
7. **langgraph-core/cache.rs** - Advanced cache tests (Days 1-2)
8. **llm thinking models** - Model-specific tests (Days 3-4)
9. **Documentation** - Update test coverage docs (Day 5)

---

## Test Coverage Metrics (Current State)

| Crate | Test Functions | Coverage Status | Priority |
|-------|---------------|-----------------|----------|
| orchestrator | 703 | ✅ EXCELLENT | ✓ Complete |
| langgraph-core | 428 | ✅ GOOD | Low |
| orca | 401 | ✅ GOOD | Low |
| aco | 331 | ✅ GOOD | Low |
| langgraph-prebuilt | 137 | ⚠️ GOOD* | Medium |
| tooling | 99 | ⚠️ PARTIAL | **HIGH** |
| llm | 90 | ⚠️ PARTIAL | **HIGH** |
| utils | 80 | ✅ GOOD | Low |
| langgraph-checkpoint | 63 | ⚠️ PARTIAL | **HIGH** |
| langgraph-cli | 0 | ❌ MISSING | **CRITICAL** |

*langgraph-prebuilt marked GOOD but has untested error.rs module

---

## Expected Outcomes

### After Phase 1 (Critical):
- langgraph-cli: 0 → 25 tests
- tooling/rate_limit: 0 → 18 tests
- llm/error.rs: 0 → 22 tests
- **New Tests Added:** ~65

### After Phase 2 (Important):
- langgraph-checkpoint: +35 tests
- langgraph-prebuilt: +12 tests
- **Total New Tests:** ~112

### After Phase 3 (Enhanced):
- langgraph-core/cache: +28 tests
- llm thinking models: +17 tests
- **Total New Tests:** ~157

### Final Coverage:
- **Current:** 2,332 test functions
- **Target:** 2,489 test functions (+157, +6.7%)
- **Critical Gaps:** All addressed
- **Quality:** Production-ready test coverage

---

## Files to Create/Modify

### New Files:
1. `src/crates/langgraph-cli/tests/cli_tests.rs`
2. `src/crates/llm/tests/error_tests.rs`

### Files to Modify:
1. `src/crates/tooling/src/rate_limit/token_bucket.rs` (add inline tests)
2. `src/crates/tooling/src/rate_limit/sliding_window.rs` (add inline tests)
3. `src/crates/langgraph-checkpoint/src/channels_extended.rs` (expand tests)
4. `src/crates/langgraph-prebuilt/src/error.rs` (add inline tests)
5. `src/crates/langgraph-core/src/cache.rs` (add contention tests)
6. `src/crates/llm/src/remote/openai.rs` (add thinking model tests)
7. `src/crates/llm/src/remote/deepseek.rs` (add R1 tests)

---

## Success Criteria

- [ ] All new tests pass
- [ ] No regressions in existing tests
- [ ] Code coverage increases in critical modules
- [ ] All CRITICAL and HIGH priority gaps addressed
- [ ] Documentation updated with test patterns
- [ ] CI/CD pipeline passes

---

## Notes

- All crates build successfully (`cargo check --workspace --lib` passes)
- Existing 2,332 tests provide strong foundation
- Focus on high-impact, production-critical areas first
- Test implementation should follow existing patterns in each crate
- Use `#[cfg(test)]` modules for inline tests, `tests/` directory for integration tests
