# Contributing to AstraWeave

Thank you for your interest in contributing to AstraWeave! This guide will help you get started with contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Documentation Requirements](#documentation-requirements)
- [Code Review Process](#code-review-process)
- [Community](#community)

## Code of Conduct

We are committed to providing a welcoming and inclusive environment for all contributors. Please be respectful and constructive in all interactions.

## Getting Started

Before you start contributing, please:

1. Read the [Building from Source](building.md) guide
2. Familiarize yourself with the [Code Style Guide](code-style.md)
3. Review the [Testing Guide](testing.md)
4. Check existing [Issues](https://github.com/verdentlabs/astraweave/issues) and [Pull Requests](https://github.com/verdentlabs/astraweave/pulls)

## How to Contribute

### Reporting Bugs

When reporting bugs, please include:

- **Clear title**: Describe the issue concisely
- **Environment**: OS, Rust version, GPU model
- **Steps to reproduce**: Detailed steps to trigger the bug
- **Expected behavior**: What should happen
- **Actual behavior**: What actually happens
- **Logs**: Relevant error messages or stack traces

```admonish example title="Bug Report Template"
**Environment:**
- OS: Windows 11
- Rust: 1.75.0
- GPU: NVIDIA RTX 4080

**Steps to Reproduce:**
1. Create a new companion with perception
2. Add emotion system
3. Run the simulation

**Expected:** Companion should respond to stimuli
**Actual:** Panic in emotion processing thread

**Logs:**
\`\`\`
thread 'emotion' panicked at 'index out of bounds'
\`\`\`
```

### Suggesting Features

For feature requests, please:

- Check if the feature already exists or is planned
- Describe the use case and problem it solves
- Provide examples of how it would work
- Consider implementation complexity and alternatives

### Submitting Pull Requests

1. **Fork the repository** and create a new branch:
   ```bash
   git checkout -b feature/my-awesome-feature
   ```

2. **Make your changes** following our coding standards

3. **Write or update tests** to cover your changes

4. **Run the full test suite**:
   ```bash
   cargo test --all-features
   cargo clippy --all-targets --all-features
   cargo fmt --check
   ```

5. **Update documentation** if needed

6. **Commit with clear messages**:
   ```bash
   git commit -m "feat(ai): Add emotion blending system"
   ```

7. **Push and create a Pull Request**:
   ```bash
   git push origin feature/my-awesome-feature
   ```

```admonish tip
Keep PRs focused on a single feature or fix. Large PRs are harder to review and merge.
```

## Development Setup

### Prerequisites

- Rust 1.75.0 or later
- Git
- A supported GPU with Vulkan/DirectX 12/Metal

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/verdentlabs/astraweave.git
cd astraweave

# Build the project
cargo build

# Run tests
cargo test

# Run examples
cargo run --example basic_companion
```

### Development Workflow

```bash
# Create a new branch
git checkout -b feature/my-feature

# Make changes and test frequently
cargo test

# Run clippy for linting
cargo clippy --all-targets --all-features

# Format code
cargo fmt

# Commit changes
git commit -m "feat: Add feature description"
```

## Coding Standards

### General Principles

- **Clarity over cleverness**: Write code that is easy to understand
- **Performance matters**: AstraWeave is a high-performance engine
- **Safety first**: Prefer safe abstractions over unsafe code
- **Documentation**: Document all public APIs

### Rust Conventions

```rust
// Good: Clear naming and documentation
/// Calculates emotion intensity based on stimuli strength.
///
/// # Arguments
/// * `stimulus` - The input stimulus value (0.0 to 1.0)
/// * `sensitivity` - Sensitivity multiplier (default: 1.0)
///
/// # Returns
/// Emotion intensity clamped to [0.0, 1.0]
pub fn calculate_emotion_intensity(stimulus: f32, sensitivity: f32) -> f32 {
    (stimulus * sensitivity).clamp(0.0, 1.0)
}

// Bad: Unclear naming and no documentation
pub fn calc(s: f32, m: f32) -> f32 {
    (s * m).min(1.0).max(0.0)
}
```

### Naming Conventions

- **Types**: `PascalCase` (e.g., `EmotionSystem`, `CompanionState`)
- **Functions**: `snake_case` (e.g., `process_emotion`, `update_behavior`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_COMPANIONS`, `DEFAULT_TICK_RATE`)
- **Modules**: `snake_case` (e.g., `emotion_system`, `companion_ai`)

### Error Handling

```rust
// Good: Use Result for recoverable errors
pub fn load_companion_config(path: &Path) -> Result<CompanionConfig, ConfigError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::FileRead(path.to_path_buf(), e))?;
    
    serde_json::from_str(&contents)
        .map_err(|e| ConfigError::Parse(e))
}

// Good: Use panic only for programming errors
pub fn get_emotion(&self, index: usize) -> &Emotion {
    assert!(index < self.emotions.len(), "Emotion index out of bounds");
    &self.emotions[index]
}
```

## Testing Requirements

All contributions must include appropriate tests:

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_intensity_clamping() {
        assert_eq!(calculate_emotion_intensity(0.5, 1.0), 0.5);
        assert_eq!(calculate_emotion_intensity(1.5, 1.0), 1.0);
        assert_eq!(calculate_emotion_intensity(-0.5, 1.0), 0.0);
    }

    #[test]
    fn test_emotion_blending() {
        let joy = Emotion::new("joy", 0.8);
        let calm = Emotion::new("calm", 0.6);
        let blended = blend_emotions(&joy, &calm, 0.5);
        assert_eq!(blended.intensity, 0.7);
    }
}
```

### Integration Tests

Place integration tests in `tests/` directory:

```rust
// tests/companion_integration.rs
use astraweave_ai::*;

#[test]
fn test_companion_lifecycle() {
    let mut companion = CompanionBuilder::new()
        .with_perception()
        .with_emotion()
        .build();
    
    companion.update(0.016);
    assert!(companion.is_active());
}
```

### Benchmarks

For performance-critical code, add benchmarks:

```rust
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn benchmark_emotion_processing(c: &mut Criterion) {
    c.bench_function("process_100_emotions", |b| {
        let mut system = EmotionSystem::new();
        b.iter(|| {
            for _ in 0..100 {
                system.process(black_box(0.016));
            }
        });
    });
}

criterion_group!(benches, benchmark_emotion_processing);
criterion_main!(benches);
```

### Coverage Requirements

- New features should have **>80% code coverage**
- Bug fixes must include a regression test
- Run coverage locally:
  ```bash
  cargo tarpaulin --all-features --workspace --timeout 300
  ```

```admonish warning
PRs with insufficient test coverage will not be merged.
```

## Documentation Requirements

### Public API Documentation

All public items must have documentation:

```rust
/// Represents an AI companion with emotion and behavior systems.
///
/// A companion can perceive its environment, process emotions, and
/// exhibit behaviors based on its internal state.
///
/// # Examples
///
/// ```
/// use astraweave_ai::Companion;
///
/// let companion = Companion::new("Buddy");
/// companion.update(0.016);
/// ```
pub struct Companion {
    // ...
}
```

### Module Documentation

Add module-level documentation in `lib.rs` or `mod.rs`:

```rust
//! # Emotion System
//!
//! This module provides the emotion processing system for AI companions.
//! Emotions are processed based on stimuli and decay over time.
//!
//! ## Example
//!
//! ```
//! use astraweave_ai::emotion::EmotionSystem;
//!
//! let mut system = EmotionSystem::new();
//! system.add_emotion("joy", 0.8);
//! system.update(0.016);
//! ```
```

### Changelog Updates

Update `CHANGELOG.md` for significant changes:

```markdown
## [Unreleased]

### Added
- Emotion blending system for smoother transitions
- New `CompanionBuilder` for easier companion creation

### Changed
- Improved perception system performance by 30%

### Fixed
- Fixed emotion decay rate calculation
```

## Code Review Process

### What Reviewers Look For

1. **Correctness**: Does the code work as intended?
2. **Tests**: Are there adequate tests?
3. **Documentation**: Is the code well-documented?
4. **Style**: Does it follow our coding standards?
5. **Performance**: Are there any performance concerns?
6. **Safety**: Are there any unsafe patterns?

### Review Timeline

- Initial review: Within 48 hours
- Follow-up reviews: Within 24 hours
- Merging: After approval from at least one maintainer

### Addressing Feedback

```admonish tip
Respond to all review comments, even if just to acknowledge them.
```

- Make requested changes in new commits
- Don't force-push during review (makes tracking changes hard)
- Mark conversations as resolved when addressed
- Ask questions if feedback is unclear

### Approval Process

1. All CI checks must pass
2. At least one maintainer approval required
3. No unresolved review comments
4. Up-to-date with the main branch

## Community

### Getting Help

- **Discord**: Join our [Discord server](https://discord.gg/astraweave)
- **Discussions**: Use [GitHub Discussions](https://github.com/verdentlabs/astraweave/discussions)
- **Issues**: Search existing issues before creating new ones

### Stay Updated

- Watch the repository for updates
- Follow our [blog](https://verdentlabs.com/blog)
- Subscribe to release notifications

Thank you for contributing to AstraWeave! Your efforts help make AI-native gaming better for everyone.
