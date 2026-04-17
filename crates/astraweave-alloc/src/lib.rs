#![forbid(unsafe_code)]
//! AstraWeave global allocator selection — opt-in mimalloc replacement.
//!
//! This crate is intentionally tiny. It exists for one reason: to let any
//! AstraWeave binary opt into mimalloc as its global allocator via a single
//! feature flag, without touching allocation patterns in library code.
//!
//! # Usage
//!
//! 1. Add `astraweave-alloc = { path = "..." }` as a dep of your binary crate.
//! 2. Add a `fast-alloc` feature that cascades to `astraweave-alloc/fast-alloc`.
//! 3. At the top of `main.rs`:
//!
//! ```rust,ignore
//! // Unconditional — expands to nothing when `fast-alloc` is off.
//! astraweave_alloc::setup_global_allocator!();
//! ```
//!
//! When `fast-alloc` is on in this crate, the macro installs `MiMalloc` as
//! `#[global_allocator]`. When off, the macro expands to no code at all.
//!
//! # Interaction with `CountingAlloc`
//!
//! `astraweave-ecs::counting_alloc::CountingAlloc` also installs itself as
//! `#[global_allocator]` under the `alloc-counter` feature. Rust allows only
//! one `#[global_allocator]` per binary, so callers must not install both.
//!
//! The convention used across AstraWeave binaries is:
//!
//! - If `alloc-counter` is enabled, install `CountingAlloc`. `CountingAlloc`
//!   internally forwards to `MiMalloc` (when `astraweave-ecs/fast-alloc` is
//!   also on) or `System` (otherwise). This lets measurement runs exercise
//!   mimalloc without changing which type is installed as the global allocator.
//! - If `alloc-counter` is off, call `setup_global_allocator!()`. With
//!   `fast-alloc` on this installs `MiMalloc`; with `fast-alloc` off it is a
//!   no-op (the platform default allocator stays in place).
//!
//! Both paths are covered in the `profiling_demo`, `hello_companion`, and
//! `aw_editor` binaries.

/// Re-export of `mimalloc::MiMalloc`. Only available when `fast-alloc` is on.
#[cfg(feature = "fast-alloc")]
pub use mimalloc::MiMalloc;

/// Install `MiMalloc` as `#[global_allocator]` at the call site when this
/// crate's `fast-alloc` feature is on. No-op otherwise.
///
/// Prefer calling this unconditionally from `main.rs`. It is a macro (rather
/// than a function) because `#[global_allocator]` can only be attached to a
/// static item at module scope.
#[cfg(feature = "fast-alloc")]
#[macro_export]
macro_rules! setup_global_allocator {
    () => {
        #[global_allocator]
        static __AW_FAST_ALLOC: $crate::MiMalloc = $crate::MiMalloc;
    };
}

/// No-op form of `setup_global_allocator!` used when `fast-alloc` is off.
#[cfg(not(feature = "fast-alloc"))]
#[macro_export]
macro_rules! setup_global_allocator {
    () => {};
}
