# Code Style Guide

This guide defines the coding standards and style conventions for AstraWeave. Consistent code style improves readability and maintainability.

## Table of Contents

- [Rust Conventions](#rust-conventions)
- [Naming Conventions](#naming-conventions)
- [Code Organization](#code-organization)
- [Documentation Style](#documentation-style)
- [Error Handling](#error-handling)
- [Performance Guidelines](#performance-guidelines)
- [Clippy and Formatting](#clippy-and-formatting)

## Rust Conventions

### General Principles

1. **Idiomatic Rust**: Write code that follows Rust idioms and patterns
2. **Safety First**: Prefer safe abstractions; document unsafe code thoroughly
3. **Zero-Cost Abstractions**: Don't sacrifice performance for convenience
4. **Explicit over Implicit**: Make behavior clear and predictable

### Code Layout

```rust
// Good: Clear structure with proper spacing
pub struct Companion {
    id: CompanionId,
    name: String,
    emotion_system: EmotionSystem,
    perception: PerceptionSystem,
}

impl Companion {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: CompanionId::generate(),
            name: name.into(),
            emotion_system: EmotionSystem::default(),
            perception: PerceptionSystem::default(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.perception.update(dt);
        self.emotion_system.update(dt);
    }
}

// Bad: Cramped, hard to read
pub struct Companion{id:CompanionId,name:String}
impl Companion{pub fn new(n:String)->Self{Self{id:CompanionId::generate(),name:n}}}
```

### Line Length

- Maximum line length: **100 characters**
- Break long lines at logical points

```rust
// Good: Broken at logical points
let companion = CompanionBuilder::new()
    .with_name("Alice")
    .with_perception()
    .with_emotion()
    .with_behavior()
    .build();

// Bad: Too long
let companion = CompanionBuilder::new().with_name("Alice").with_perception().with_emotion().with_behavior().build();
```

### Imports

Group and sort imports:

```rust
// 1. Standard library
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crates (alphabetically)
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// 3. Internal crates
use astraweave_ai::*;
use astraweave_physics::*;

// 4. Local modules
use crate::emotion::*;
use crate::perception::Stimulus;

// 5. Type aliases and trait imports
use super::CompanionId;
```

```admonish tip
Use `cargo fmt` to automatically organize imports.
```

## Naming Conventions

### Types

**PascalCase** for types, structs, enums, and traits:

```rust
// Good
pub struct CompanionState { }
pub enum EmotionType { }
pub trait BehaviorController { }

// Bad
pub struct companion_state { }
pub enum emotion_type { }
pub trait behavior_controller { }
```

### Functions and Methods

**snake_case** for functions and methods:

```rust
// Good
pub fn calculate_emotion_intensity(stimulus: f32) -> f32 { }
pub fn process_visual_stimulus(&mut self, data: &[u8]) { }

// Bad
pub fn CalculateEmotionIntensity(stimulus: f32) -> f32 { }
pub fn ProcessVisualStimulus(&mut self, data: &[u8]) { }
```

### Variables

**snake_case** for variables and parameters:

```rust
// Good
let companion_id = CompanionId::new();
let emotion_decay_rate = 0.95;

// Bad
let CompanionID = CompanionId::new();
let EmotionDecayRate = 0.95;
```

### Constants

**SCREAMING_SNAKE_CASE** for constants:

```rust
// Good
pub const MAX_COMPANIONS: usize = 1000;
pub const DEFAULT_TICK_RATE: f32 = 60.0;
const PI: f32 = std::f32::consts::PI;

// Bad
pub const max_companions: usize = 1000;
pub const DefaultTickRate: f32 = 60.0;
```

### Modules

**snake_case** for module names:

```rust
// Good
mod emotion_system;
mod companion_ai;
mod perception_processing;

// Bad
mod EmotionSystem;
mod CompanionAI;
mod PerceptionProcessing;
```

### Acronyms

Treat acronyms as words in PascalCase:

```rust
// Good
struct HttpClient;
struct XmlParser;
struct AiCompanion;

// Bad (but acceptable for well-known 2-letter acronyms)
struct AICompanion; // Acceptable
struct XMLParser;   // Avoid

// Definitely bad
struct HTTPClient;
```

## Code Organization

### Module Structure

Organize code logically by functionality:

```
astraweave-ai/
├── src/
│   ├── lib.rs              // Public API exports
│   ├── companion/
│   │   ├── mod.rs          // Module root
│   │   ├── builder.rs      // CompanionBuilder
│   │   ├── state.rs        // CompanionState
│   │   └── lifecycle.rs    // Lifecycle management
│   ├── emotion/
│   │   ├── mod.rs
│   │   ├── system.rs       // EmotionSystem
│   │   ├── types.rs        // Emotion types
│   │   └── blending.rs     // Emotion blending
│   └── perception/
│       ├── mod.rs
│       ├── visual.rs       // Visual perception
│       └── auditory.rs     // Auditory perception
```

### File Organization

Structure within a file:

```rust
// 1. Module documentation
//! # Emotion System
//!
//! This module implements the emotion processing system.

// 2. Imports (grouped as shown earlier)
use std::collections::HashMap;
use bevy::prelude::*;

// 3. Type definitions
pub struct EmotionSystem {
    emotions: HashMap<String, Emotion>,
}

// 4. Implementation blocks
impl EmotionSystem {
    pub fn new() -> Self {
        Self {
            emotions: HashMap::new(),
        }
    }
}

// 5. Trait implementations
impl Default for EmotionSystem {
    fn default() -> Self {
        Self::new()
    }
}

// 6. Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_system_creation() {
        let system = EmotionSystem::new();
        assert!(system.emotions.is_empty());
    }
}
```

### Public vs Private

Be intentional about visibility:

```rust
// Good: Clear public API
pub struct Companion {
    // Public fields only when necessary
    pub id: CompanionId,
    
    // Private implementation details
    emotion_system: EmotionSystem,
    internal_state: State,
}

impl Companion {
    // Public constructor
    pub fn new(name: String) -> Self { }
    
    // Public methods for API
    pub fn update(&mut self, dt: f32) { }
    
    // Private helper methods
    fn process_internal_state(&mut self) { }
}

// Bad: Everything public
pub struct Companion {
    pub id: CompanionId,
    pub emotion_system: EmotionSystem,
    pub internal_state: State,
}
```

## Documentation Style

### Module Documentation

```rust
//! # Emotion System
//!
//! This module provides the core emotion processing system for AI companions.
//! Emotions are represented as continuous values that decay over time and
//! blend based on stimuli.
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

### Type Documentation

```rust
/// Represents an AI companion with emotion and behavior systems.
///
/// A companion processes stimuli through its perception system,
/// updates its emotional state, and exhibits behaviors based on
/// its internal state and environment.
///
/// # Examples
///
/// ```
/// use astraweave_ai::Companion;
///
/// let mut companion = Companion::new("Buddy");
/// companion.update(0.016);
///
/// if let Some(emotion) = companion.get_emotion("joy") {
///     println!("Joy level: {}", emotion.intensity);
/// }
/// ```
///
/// # Performance
///
/// Companion updates are O(n) where n is the number of active
/// emotions and behaviors.
pub struct Companion {
    // ...
}
```

### Function Documentation

```rust
/// Calculates the blended emotion intensity from two emotions.
///
/// Uses linear interpolation to blend two emotion intensities
/// based on a blend factor.
///
/// # Arguments
///
/// * `emotion_a` - First emotion intensity (0.0 to 1.0)
/// * `emotion_b` - Second emotion intensity (0.0 to 1.0)
/// * `blend_factor` - Blend weight (0.0 = all A, 1.0 = all B)
///
/// # Returns
///
/// Blended emotion intensity clamped to [0.0, 1.0]
///
/// # Examples
///
/// ```
/// use astraweave_ai::emotion::blend_emotions;
///
/// let joy = 0.8;
/// let calm = 0.6;
/// let blended = blend_emotions(joy, calm, 0.5);
/// assert_eq!(blended, 0.7);
/// ```
///
/// # Panics
///
/// Panics if `blend_factor` is not in range [0.0, 1.0] in debug builds.
pub fn blend_emotions(emotion_a: f32, emotion_b: f32, blend_factor: f32) -> f32 {
    debug_assert!(
        (0.0..=1.0).contains(&blend_factor),
        "blend_factor must be in [0.0, 1.0]"
    );
    
    (emotion_a * (1.0 - blend_factor) + emotion_b * blend_factor).clamp(0.0, 1.0)
}
```

### Documentation Sections

Use these sections in order:

1. **Summary**: One-line description
2. **Detailed description**: Multi-paragraph explanation
3. **Arguments**: Parameter descriptions
4. **Returns**: Return value description
5. **Examples**: Code examples
6. **Errors**: Possible errors (for `Result`)
7. **Panics**: Panic conditions
8. **Safety**: Safety invariants (for `unsafe`)
9. **Performance**: Performance characteristics

## Error Handling

### Use Result for Recoverable Errors

```rust
// Good: Use Result for expected errors
pub fn load_companion(path: &Path) -> Result<Companion, LoadError> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| LoadError::FileRead(path.to_path_buf(), e))?;
    
    serde_json::from_str(&data)
        .map_err(LoadError::Parse)
}

// Bad: Panic on expected errors
pub fn load_companion(path: &Path) -> Companion {
    let data = std::fs::read_to_string(path).unwrap(); // Don't do this
    serde_json::from_str(&data).unwrap()
}
```

### Define Custom Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompanionError {
    #[error("Companion not found: {0}")]
    NotFound(CompanionId),
    
    #[error("Invalid companion state: {0}")]
    InvalidState(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),
}
```

### Provide Context

```rust
use anyhow::{Context, Result};

pub fn save_companion(&self, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(self)
        .context("Failed to serialize companion")?;
    
    std::fs::write(path, json)
        .with_context(|| format!("Failed to write to {}", path.display()))?;
    
    Ok(())
}
```

## Performance Guidelines

### Prefer Iteration over Indexing

```rust
// Good: Iterator-based
fn sum_emotions(emotions: &[Emotion]) -> f32 {
    emotions.iter().map(|e| e.intensity).sum()
}

// Less efficient: Index-based
fn sum_emotions_indexed(emotions: &[Emotion]) -> f32 {
    let mut sum = 0.0;
    for i in 0..emotions.len() {
        sum += emotions[i].intensity;
    }
    sum
}
```

### Avoid Unnecessary Allocations

```rust
// Good: Reuse buffer
pub struct EmotionProcessor {
    buffer: Vec<f32>,
}

impl EmotionProcessor {
    pub fn process(&mut self, emotions: &[Emotion]) {
        self.buffer.clear();
        self.buffer.extend(emotions.iter().map(|e| e.intensity));
        // Process buffer...
    }
}

// Bad: Allocate every time
pub fn process(emotions: &[Emotion]) -> Vec<f32> {
    emotions.iter().map(|e| e.intensity).collect() // New allocation
}
```

### Use References When Possible

```rust
// Good: Borrow instead of clone
pub fn analyze_emotion(&self, emotion: &Emotion) -> Analysis {
    Analysis {
        intensity: emotion.intensity,
        category: emotion.category.clone(), // Only clone when necessary
    }
}

// Bad: Unnecessary clone
pub fn analyze_emotion(&self, emotion: Emotion) -> Analysis {
    // Took ownership, forcing caller to clone
    Analysis {
        intensity: emotion.intensity,
        category: emotion.category,
    }
}
```

### Document Performance Characteristics

```rust
/// Finds a companion by ID.
///
/// # Performance
///
/// O(1) average case using HashMap lookup.
pub fn find_companion(&self, id: CompanionId) -> Option<&Companion> {
    self.companions.get(&id)
}

/// Sorts companions by emotion intensity.
///
/// # Performance
///
/// O(n log n) where n is the number of companions.
/// Consider using `find_max_emotion` for single maximum lookup.
pub fn sort_by_emotion(&mut self, emotion_name: &str) {
    self.companions.sort_by(|a, b| {
        a.get_emotion(emotion_name)
            .cmp(&b.get_emotion(emotion_name))
    });
}
```

## Clippy and Formatting

### Running Clippy

```bash
# Run clippy with all features
cargo clippy --all-targets --all-features

# Fix clippy warnings automatically (where possible)
cargo clippy --fix --all-targets --all-features

# Deny all warnings (for CI)
cargo clippy --all-targets --all-features -- -D warnings
```

### Common Clippy Lints

Enable project-wide lints in `lib.rs`:

```rust
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::cargo,
    missing_docs,
    rust_2018_idioms,
)]

// Allow specific lints when justified
#![allow(
    clippy::module_name_repetitions, // CompanionBuilder in companion module is fine
    clippy::must_use_candidate,      // Not all pure functions need #[must_use]
)]
```

### Code Formatting

```bash
# Format all code
cargo fmt

# Check formatting without applying
cargo fmt -- --check

# Format with custom config
cargo fmt -- --config max_width=100
```

### rustfmt Configuration

Create `rustfmt.toml`:

```toml
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
edition = "2021"
```

### Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/sh

# Run formatter
cargo fmt -- --check
if [ $? -ne 0 ]; then
    echo "Code is not formatted. Run 'cargo fmt' first."
    exit 1
fi

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
if [ $? -ne 0 ]; then
    echo "Clippy found issues. Fix them before committing."
    exit 1
fi

exit 0
```

```admonish success
Consistent style makes code easier to read, review, and maintain. Use automated tools to enforce standards.
```
