//! Counting allocator for zero-alloc hot path validation.
//!
//! This module provides a global allocator wrapper that counts heap allocations,
//! allowing tests and benchmarks to assert zero allocations in hot paths.
//!
//! # Architecture
//!
//! As of the allocation-measurement instrumentation work, this module keeps
//! **two sets of counters in lockstep**:
//!
//! 1. **Local atomics** (`ALLOCS`, `DEALLOCS`, `BYTES_ALLOCATED`, `BYTES_DEALLOCATED`,
//!    `REALLOCS`) — preserve the original `allocs()` / `deallocs()` / `net_allocs()`
//!    / `reset_allocs()` API used by existing `zero_alloc_tests` and other ECS tests.
//!
//! 2. **`astraweave_profiling::counters`** — process-wide counters consumed by
//!    `FrameAllocStats` and the `alloc_plot!` / `measured_span!` macros used across
//!    the render, physics, and AI crates. Forwarding into this module lets a single
//!    `#[global_allocator]` installation (this one) feed both the test-assertion
//!    machinery here and the Tracy plots emitted from `astraweave-profiling`.
//!
//! Both sets are updated with `Relaxed` ordering (for the counters themselves) and
//! `SeqCst` ordering for `reset` — identical to the pre-existing semantics.
//!
//! # Allocator precedence (mimalloc experiment)
//!
//! Rust allows only one `#[global_allocator]` per binary. `CountingAlloc` is a
//! wrapper that delegates to an inner allocator chosen at compile time:
//!
//! - Default (no `fast-alloc` feature): inner = `std::alloc::System`.
//! - With `fast-alloc`: inner = `astraweave_alloc::MiMalloc`.
//!
//! This lets the same `#[global_allocator] CountingAlloc` installation be used in
//! both baseline and mimalloc-paired measurement runs. The binary-side rule is:
//!
//! - If `alloc-counter` is on → install `CountingAlloc` (via a `#[global_allocator]`
//!   static in the binary's `main.rs`). Inner allocator follows the `fast-alloc`
//!   feature.
//! - If `alloc-counter` is off and `fast-alloc` is on → call
//!   `astraweave_alloc::setup_global_allocator!()` instead, which installs
//!   `MiMalloc` directly with no counting overhead.
//! - If neither feature is on → no explicit global allocator; the platform
//!   default is used.
//!
//! # Usage
//!
//! In test files, enable the counting allocator with `--features alloc-counter`:
//!
//! ```rust,ignore
//! use astraweave_ecs::counting_alloc::{reset_allocs, allocs};
//!
//! reset_allocs();
//! // ... run hot path code ...
//! assert_eq!(allocs(), 0, "Hot path should not allocate");
//! ```
//!
//! # Note
//!
//! This is only enabled with the `alloc-counter` feature to avoid overhead in
//! production builds.

use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};

// Compile-time selection of the inner allocator. Both `System` and `MiMalloc`
// are unit structs implementing `GlobalAlloc`, so using them through a static
// keeps the call sites identical.
#[cfg(not(feature = "fast-alloc"))]
static INNER: std::alloc::System = std::alloc::System;
#[cfg(feature = "fast-alloc")]
static INNER: astraweave_alloc::MiMalloc = astraweave_alloc::MiMalloc;

/// Local allocation counter (preserved for backward-compatible API).
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

/// Local deallocation counter.
static DEALLOCS: AtomicUsize = AtomicUsize::new(0);

/// Local reallocation counter.
static REALLOCS: AtomicUsize = AtomicUsize::new(0);

/// Local bytes-allocated counter.
static BYTES_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

/// Local bytes-deallocated counter.
static BYTES_DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

/// Counting allocator that wraps the system allocator.
///
/// When registered as `#[global_allocator]`, this tracks all heap allocations
/// and deallocations, enabling zero-alloc assertions in tests. Every call is
/// also forwarded to `astraweave_profiling::counters` so that higher-level
/// instrumentation (`FrameAllocStats`, `alloc_plot!`, `measured_span!`) sees
/// the same activity without requiring a second `#[global_allocator]`.
pub struct CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_add(size, Ordering::Relaxed);
        astraweave_profiling::counters::record_alloc(size);
        INNER.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();
        DEALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES_DEALLOCATED.fetch_add(size, Ordering::Relaxed);
        astraweave_profiling::counters::record_dealloc(size);
        INNER.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_add(size, Ordering::Relaxed);
        astraweave_profiling::counters::record_alloc(size);
        INNER.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // A realloc counts as both a dealloc (old size) and an alloc (new size),
        // consistent with the original semantics. REALLOCS records the resize event
        // itself so callers can distinguish "growth from existing buffer" from
        // "fresh allocation".
        let old_size = layout.size();
        DEALLOCS.fetch_add(1, Ordering::Relaxed);
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        REALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES_DEALLOCATED.fetch_add(old_size, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_add(new_size, Ordering::Relaxed);
        astraweave_profiling::counters::record_realloc(old_size, new_size);
        INNER.realloc(ptr, layout, new_size)
    }
}

// ============================================================================
// Public counter API — preserved for backward compatibility and extended for
// the measurement instrumentation work (reallocs, bytes_allocated, reset).
// ============================================================================

/// Get the current allocation count since last reset.
///
/// Includes fresh `alloc`/`alloc_zeroed` events AND the alloc-half of `realloc`
/// calls (because realloc conceptually splits into dealloc + alloc).
#[inline]
pub fn allocs() -> usize {
    ALLOCS.load(Ordering::Relaxed)
}

/// Get the current deallocation count since last reset.
///
/// Includes fresh `dealloc` events AND the dealloc-half of `realloc` calls.
#[inline]
pub fn deallocs() -> usize {
    DEALLOCS.load(Ordering::Relaxed)
}

/// Get the current reallocation count since last reset.
///
/// This is the count of `realloc` events only — useful for detecting Vec/HashMap
/// growth patterns vs. fresh-allocation patterns. Note that each realloc also
/// increments `allocs()` and `deallocs()` by one each, matching the system
/// allocator's view.
#[inline]
pub fn reallocs() -> usize {
    REALLOCS.load(Ordering::Relaxed)
}

/// Total bytes allocated since last reset.
#[inline]
pub fn bytes_allocated() -> usize {
    BYTES_ALLOCATED.load(Ordering::Relaxed)
}

/// Total bytes deallocated since last reset.
#[inline]
pub fn bytes_deallocated() -> usize {
    BYTES_DEALLOCATED.load(Ordering::Relaxed)
}

/// Reset both allocation and deallocation counters to zero.
///
/// Also resets `reallocs`, `bytes_allocated`, `bytes_deallocated`, and forwards
/// the reset to `astraweave_profiling::counters` so the two counter sets stay
/// in sync.
#[inline]
pub fn reset_allocs() {
    ALLOCS.store(0, Ordering::SeqCst);
    DEALLOCS.store(0, Ordering::SeqCst);
    REALLOCS.store(0, Ordering::SeqCst);
    BYTES_ALLOCATED.store(0, Ordering::SeqCst);
    BYTES_DEALLOCATED.store(0, Ordering::SeqCst);
    astraweave_profiling::counters::reset();
}

/// Alias for `reset_allocs`, matching the API listed in the measurement plan
/// (`allocs()`, `deallocs()`, `reallocs()`, `bytes_allocated()`, `reset()`).
#[inline]
pub fn reset() {
    reset_allocs();
}

/// Get net allocations (allocs - deallocs). Useful for leak detection.
#[inline]
pub fn net_allocs() -> isize {
    allocs() as isize - deallocs() as isize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counting_functions() {
        // Just verify the functions work (actual counting requires global allocator)
        let _ = allocs();
        let _ = deallocs();
        let _ = reallocs();
        let _ = bytes_allocated();
        let _ = bytes_deallocated();
        let _ = net_allocs();
        reset_allocs();
        reset();
    }
}
