# Testing Guide

This guide covers testing practices for AstraWeave, including unit tests, integration tests, benchmarks, and coverage analysis.

## Table of Contents

- [Running Tests](#running-tests)
- [Writing Unit Tests](#writing-unit-tests)
- [Integration Tests](#integration-tests)
- [Benchmarks](#benchmarks)
- [Code Coverage](#code-coverage)
- [Testing Best Practices](#testing-best-practices)

## Running Tests

### Quick Start

```bash
# Run all tests
cargo test

# Run tests in release mode (faster for compute-heavy tests)
cargo test --release

# Run tests for a specific package
cargo test -p astraweave-ai

# Run tests with all features enabled
cargo test --all-features
```

### Verbose Output

```bash
# Show println! output from tests
cargo test -- --nocapture

# Show one test per line
cargo test -- --test-threads=1 --nocapture

# Run specific test
cargo test test_emotion_processing -- --exact
```

### Filtering Tests

```bash
# Run tests matching a pattern
cargo test emotion

# Run only unit tests (exclude integration tests)
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run only documentation tests
cargo test --doc
```

```admonish tip
Use `cargo test --help` and `cargo test -- --help` to see all available options.
```

## Writing Unit Tests

### Basic Unit Tests

Place unit tests in the same file as the code they test:

```rust
pub fn calculate_emotion_decay(initial: f32, decay_rate: f32, dt: f32) -> f32 {
    initial * (1.0 - decay_rate * dt).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_decay_basic() {
        let result = calculate_emotion_decay(1.0, 0.5, 0.1);
        assert_eq!(result, 0.95);
    }

    #[test]
    fn test_emotion_decay_never_negative() {
        let result = calculate_emotion_decay(1.0, 2.0, 1.0);
        assert_eq!(result, 0.0);
    }
}
```

### Testing with Floating Point

Use approximate comparisons for floating-point values:

```rust
#[test]
fn test_emotion_blending() {
    let result = blend_emotions(0.8, 0.6, 0.5);
    assert!((result - 0.7).abs() < 1e-6, "Expected ~0.7, got {}", result);
}

// Or use the approx crate
use approx::assert_relative_eq;

#[test]
fn test_emotion_blending_approx() {
    let result = blend_emotions(0.8, 0.6, 0.5);
    assert_relative_eq!(result, 0.7, epsilon = 1e-6);
}
```

### Testing Errors

Test error conditions explicitly:

```rust
#[test]
fn test_invalid_companion_id() {
    let world = CompanionWorld::new();
    let result = world.get_companion(CompanionId(999));
    assert!(result.is_err());
    assert!(matches!(result, Err(CompanionError::NotFound(_))));
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn test_panic_on_invalid_index() {
    let emotions = vec![Emotion::new("joy", 0.8)];
    let _ = emotions[10]; // Should panic
}
```

### Testing with Resources

Use setup and teardown for resource management:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_config() -> (CompanionConfig, PathBuf) {
        let temp_dir = std::env::temp_dir().join("astraweave_test");
        fs::create_dir_all(&temp_dir).unwrap();
        
        let config_path = temp_dir.join("config.json");
        let config = CompanionConfig::default();
        
        (config, config_path)
    }

    fn teardown_test_config(path: &Path) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_config_save_load() {
        let (config, path) = setup_test_config();
        
        config.save(&path).unwrap();
        let loaded = CompanionConfig::load(&path).unwrap();
        
        assert_eq!(config, loaded);
        
        teardown_test_config(&path);
    }
}
```

### Parameterized Tests

Test multiple cases efficiently:

```rust
#[test]
fn test_emotion_clamping() {
    let test_cases = vec![
        (0.5, 0.5),   // Normal case
        (1.5, 1.0),   // Above max
        (-0.5, 0.0),  // Below min
        (0.0, 0.0),   // Edge case: zero
        (1.0, 1.0),   // Edge case: max
    ];

    for (input, expected) in test_cases {
        let result = clamp_emotion(input);
        assert_eq!(result, expected, "Failed for input {}", input);
    }
}
```

### Testing Async Code

For async functions, use `tokio::test` or similar:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[tokio::test]
    async fn test_async_companion_update() {
        let mut companion = Companion::new("TestBot");
        companion.update_async(0.016).await;
        assert!(companion.is_initialized());
    }

    #[tokio::test]
    async fn test_concurrent_updates() {
        let companion = Arc::new(Mutex::new(Companion::new("Concurrent")));
        
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let c = Arc::clone(&companion);
                tokio::spawn(async move {
                    c.lock().await.update_async(0.016).await;
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }
    }
}
```

## Integration Tests

### Integration Test Structure

Integration tests live in the `tests/` directory:

```
astraweave-ai/
├── src/
│   └── lib.rs
├── tests/
│   ├── companion_lifecycle.rs
│   ├── emotion_integration.rs
│   └── perception_behavior.rs
└── Cargo.toml
```

### Example Integration Test

```rust
// tests/companion_lifecycle.rs
use astraweave_ai::*;

#[test]
fn test_complete_companion_lifecycle() {
    // Creation
    let mut companion = CompanionBuilder::new()
        .with_name("IntegrationBot")
        .with_perception()
        .with_emotion()
        .with_behavior()
        .build();

    assert!(!companion.is_initialized());

    // Initialization
    companion.initialize();
    assert!(companion.is_initialized());

    // Update cycle
    for _ in 0..100 {
        companion.update(0.016);
    }

    // Verify state
    assert!(companion.get_emotion("joy").is_some());
    assert!(companion.perception_active());

    // Shutdown
    companion.shutdown();
    assert!(!companion.is_active());
}

#[test]
fn test_companion_persistence() {
    let temp_path = std::env::temp_dir().join("companion_save.json");

    // Create and save
    let original = CompanionBuilder::new()
        .with_name("Persistent")
        .build();
    original.save(&temp_path).unwrap();

    // Load and verify
    let loaded = Companion::load(&temp_path).unwrap();
    assert_eq!(original.name(), loaded.name());

    std::fs::remove_file(&temp_path).unwrap();
}
```

### Integration Test Helpers

Create test utilities in `tests/common/mod.rs`:

```rust
// tests/common/mod.rs
use astraweave_ai::*;

pub fn create_test_companion() -> Companion {
    CompanionBuilder::new()
        .with_name("TestCompanion")
        .with_perception()
        .with_emotion()
        .build()
}

pub fn create_test_world() -> CompanionWorld {
    let mut world = CompanionWorld::new();
    for i in 0..5 {
        world.add_companion(create_test_companion());
    }
    world
}

pub fn assert_emotion_in_range(emotion: &Emotion, min: f32, max: f32) {
    assert!(
        emotion.intensity >= min && emotion.intensity <= max,
        "Emotion {} out of range [{}, {}]",
        emotion.intensity,
        min,
        max
    );
}
```

```rust
// tests/world_simulation.rs
mod common;

use common::*;

#[test]
fn test_world_simulation() {
    let mut world = create_test_world();
    
    for _ in 0..100 {
        world.update(0.016);
    }

    assert_eq!(world.companion_count(), 5);
}
```

## Benchmarks

### Setting Up Benchmarks

Benchmarks use the `criterion` crate. Add to `Cargo.toml`:

```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "emotion_benchmarks"
harness = false
```

### Writing Benchmarks

```rust
// benches/emotion_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use astraweave_ai::*;

fn benchmark_emotion_update(c: &mut Criterion) {
    let mut system = EmotionSystem::new();
    
    c.bench_function("emotion_update_single", |b| {
        b.iter(|| {
            system.update(black_box(0.016));
        });
    });
}

fn benchmark_emotion_blending(c: &mut Criterion) {
    let mut group = c.benchmark_group("emotion_blending");
    
    for count in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                let emotions: Vec<_> = (0..count)
                    .map(|i| Emotion::new(&format!("emotion_{}", i), 0.5))
                    .collect();
                
                b.iter(|| {
                    blend_emotion_array(black_box(&emotions))
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_companion_update(c: &mut Criterion) {
    let mut companion = CompanionBuilder::new()
        .with_perception()
        .with_emotion()
        .with_behavior()
        .build();
    
    c.bench_function("companion_full_update", |b| {
        b.iter(|| {
            companion.update(black_box(0.016));
        });
    });
}

criterion_group!(
    benches,
    benchmark_emotion_update,
    benchmark_emotion_blending,
    benchmark_companion_update
);
criterion_main!(benches);
```

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench emotion_update

# Save baseline for comparison
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main

# Generate detailed reports
cargo bench -- --verbose
```

```admonish note
Benchmark results are saved in `target/criterion/`. You can view HTML reports by opening `target/criterion/report/index.html`.
```

### Benchmark Best Practices

```rust
// Good: Use black_box to prevent optimization
use std::hint::black_box;

c.bench_function("optimized_safe", |b| {
    b.iter(|| {
        expensive_function(black_box(input))
    });
});

// Good: Separate setup from measurement
c.bench_function("with_setup", |b| {
    let data = create_large_dataset();
    b.iter(|| {
        process_dataset(black_box(&data))
    });
});

// Good: Use iter_batched for setup per iteration
use criterion::BatchSize;

c.bench_function("setup_per_iter", |b| {
    b.iter_batched(
        || create_test_data(),
        |data| process_data(data),
        BatchSize::SmallInput,
    );
});
```

## Code Coverage

### Installing Coverage Tools

```bash
# Install tarpaulin (Linux only)
cargo install cargo-tarpaulin

# Or use llvm-cov (cross-platform)
cargo install cargo-llvm-cov
```

### Running Coverage Analysis

**Using tarpaulin** (Linux):

```bash
# Generate coverage report
cargo tarpaulin --all-features --workspace --timeout 300 --out Html

# View report
firefox tarpaulin-report.html
```

**Using llvm-cov** (cross-platform):

```bash
# Install llvm-tools
rustup component add llvm-tools-preview

# Generate coverage
cargo llvm-cov --all-features --workspace --html

# View report
open target/llvm-cov/html/index.html
```

### Coverage in CI/CD

```yaml
# .github/workflows/coverage.yml
name: Coverage

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      
      - name: Generate coverage
        run: cargo tarpaulin --all-features --workspace --timeout 300 --out Xml
      
      - name: Upload to codecov
        uses: codecov/codecov-action@v3
        with:
          files: cobertura.xml
```

### Coverage Goals

- **Overall**: Aim for >80% code coverage
- **Critical paths**: AI logic, physics should be >90%
- **Edge cases**: All error paths should be tested
- **Public APIs**: 100% coverage of public interfaces

```admonish warning
High coverage doesn't guarantee quality tests. Focus on meaningful test cases, not just hitting lines.
```

## Testing Best Practices

### Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Group related tests
    mod emotion_tests {
        use super::*;

        #[test]
        fn test_creation() { /* ... */ }

        #[test]
        fn test_update() { /* ... */ }
    }

    mod perception_tests {
        use super::*;

        #[test]
        fn test_stimulus_detection() { /* ... */ }
    }
}
```

### Clear Test Names

```rust
// Good: Descriptive names
#[test]
fn test_emotion_decay_clamps_to_zero() { }

#[test]
fn test_companion_initialization_fails_without_required_components() { }

// Bad: Unclear names
#[test]
fn test1() { }

#[test]
fn test_emotion() { }
```

### Arrange-Act-Assert Pattern

```rust
#[test]
fn test_companion_emotion_response() {
    // Arrange
    let mut companion = create_test_companion();
    let stimulus = Stimulus::new(StimulusType::Positive, 0.8);

    // Act
    companion.process_stimulus(stimulus);
    companion.update(0.016);

    // Assert
    let joy = companion.get_emotion("joy").unwrap();
    assert!(joy.intensity > 0.5);
}
```

### Test Independence

```rust
// Good: Each test is independent
#[test]
fn test_a() {
    let mut system = EmotionSystem::new();
    system.add_emotion("joy", 0.5);
    assert_eq!(system.emotion_count(), 1);
}

#[test]
fn test_b() {
    let mut system = EmotionSystem::new(); // Fresh instance
    system.add_emotion("calm", 0.8);
    assert_eq!(system.emotion_count(), 1);
}
```

### Documentation in Tests

```rust
/// Tests that emotion decay never produces negative values,
/// even with extreme decay rates or large time deltas.
#[test]
fn test_emotion_decay_bounds() {
    let test_cases = vec![
        (1.0, 10.0, 1.0),  // Extreme decay
        (0.5, 1.0, 100.0), // Large time delta
    ];

    for (initial, rate, dt) in test_cases {
        let result = calculate_emotion_decay(initial, rate, dt);
        assert!(result >= 0.0, "Decay produced negative value: {}", result);
    }
}
```

```admonish success
Well-tested code is reliable code. Invest time in comprehensive tests to catch bugs early and enable confident refactoring.
```
