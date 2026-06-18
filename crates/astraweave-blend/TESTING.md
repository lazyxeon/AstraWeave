# astraweave-blend Testing Infrastructure

## Overview

This crate has a comprehensive testing and benchmarking infrastructure designed for **coverage** with no happy-path-only tests. The test suite focuses on edge cases, error handling, security vulnerabilities, adversarial inputs, and property-based testing.

## Test Summary

| Test Suite | Test Count | Focus Area |
|------------|------------|------------|
| Unit Tests | 55 | Core functionality |
| Adversarial Tests | 30 | Malicious inputs, concurrency, edge cases |
| Edge Case Tests | 53 | Boundary conditions, special values |
| Error Handling Tests | 47 | All error variants, propagation |
| Property Tests | 39 | Invariant verification |
| Security Tests | 35 | Path traversal, injection, resource limits |
| Doc Tests | 1 | Example validation |
| **Total** | **260** | |

## Running Tests

```bash
# Run all tests
cd crates/astraweave-blend
cargo test

# Run specific test suite
cargo test --test adversarial
cargo test --test edge_cases
cargo test --test error_handling
cargo test --test property_tests
cargo test --test security_tests

# Run with features
cargo test --features test-utils
```

## Test Suites

### 1. Adversarial Tests (`tests/adversarial.rs`)
Tests designed to break the system through:
- Concurrent access patterns
- Malformed/corrupted data
- Resource exhaustion attempts
- Extreme values (u32::MAX, very long strings)
- Hash collision attempts
- Type safety verification (Send + Sync)

### 2. Edge Case Tests (`tests/edge_cases.rs`)
Boundary condition testing:
- Zero values, maximum values
- Empty strings, very long strings
- Unicode characters, special characters
- Timeout extremes (zero, maximum Duration)
- Path edge cases (dots, extensions, deep nesting)
- Version comparison edge cases

### 3. Error Handling Tests (`tests/error_handling.rs`)
Complete error coverage:
- All `BlendError` variants tested
- Error propagation with `?` operator
- Thread safety of errors
- Error message quality (actionable, includes context)
- `std::error::Error` trait implementation

### 4. Property Tests (`tests/property_tests.rs`)
Invariant-based testing:
- Version comparison properties (reflexive, transitive, symmetric)
- Serialization roundtrips preserve data
- Builder patterns work correctly
- Preset configurations are valid
- Format extensions are consistent

### 5. Security Tests (`tests/security_tests.rs`)
Security-focused testing:
- Path traversal prevention (`../`, `..\\`)
- Command injection character detection
- Null byte handling
- Resource limit enforcement
- Symlink attack detection
- Unicode normalization attacks (homoglyphs, RTL override)
- Hash collision resistance
- Oversized input rejection

## Benchmarks

Located in `benches/`:

### `blend_benchmarks.rs`
- Version parsing and comparison
- Options construction
- Preset creation
- Error creation

### `cache_benchmarks.rs`
- Cache entry creation
- Entry serialization
- Access time tracking
- Age calculation

### `hash_benchmarks.rs`
- SHA-256 hashing (small to large inputs)
- Empty content handling
- Hash comparison

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench blend_benchmarks
cargo bench --bench cache_benchmarks
cargo bench --bench hash_benchmarks
```

## Fuzz Testing

Located in `fuzz/`:

### Fuzz Targets
- `fuzz_version` - BlenderVersion parsing from arbitrary inputs
- `fuzz_options` - ConversionOptions with arbitrary configurations
- `fuzz_cache_entry` - CacheEntry serialization and operations
- `fuzz_serialization` - RON/JSON/bincode roundtrip testing
- `fuzz_path` - Path handling security

### Running Fuzz Tests

Requires nightly Rust and `cargo-fuzz`:

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# List targets
cd crates/astraweave-blend/fuzz
cargo fuzz list

# Run a target
cargo +nightly fuzz run fuzz_version

# Run with time limit
cargo +nightly fuzz run fuzz_version -- -max_total_time=300

# Run with coverage
cargo +nightly fuzz coverage fuzz_version
```

## Test Utilities

The `test-utils` feature provides helpers for testing:

```rust
use astraweave_blend::test_utils::*;

// Create test fixtures
let fixture = TestFixture::new();

// Generate adversarial inputs
let inputs = AdversarialInputs::all();

// Create mock installation
let mock = MockBlenderInstallation::new();
```

### Available Generators

When the `test-utils` feature is enabled, the following generators are available:
- `arbitrary_version()` - Random BlenderVersion
- `arbitrary_options()` - Random ConversionOptions
- `arbitrary_cache_entry()` - Random CacheEntry
- `arbitrary_conversion_result()` - Random ConversionResult

## Coverage Goals

This test suite targets:
- **Statement coverage**: >90%
- **Branch coverage**: >85%
- **Error path coverage**: 100% (all error variants tested)
- **Security coverage**: All known attack vectors

## Test Philosophy

1. **No Happy Path Only**: Every test considers what can go wrong
2. **Adversarial Mindset**: Test like an attacker would use the system
3. **Property-Based**: Verify invariants hold across random inputs
4. **Security First**: Path traversal, injection, and DoS prevention tested
5. **Deterministic**: Tests are reproducible and non-flaky
6. **Fast**: Unit tests complete in seconds, full suite in ~2 seconds

## Adding New Tests

When adding new functionality:
1. Add unit tests in the module's `tests` submodule
2. Add edge cases to `tests/edge_cases.rs`
3. Add error handling tests to `tests/error_handling.rs`
4. Consider security implications → add to `tests/security_tests.rs`
5. Add property tests if applicable → `tests/property_tests.rs`
6. Consider adding fuzz target for parsing/deserialization

## Test Maintenance

- Run `cargo test` before every commit
- Run `cargo bench` periodically to catch performance regressions
- Run fuzz tests before releases
- Review and update tests when API changes
