//! Keep `Backtrace::capture` off during allocation-floor tests.
//!
//! `wgpu-core`'s debug snatch lock calls `std::backtrace::Backtrace::capture`
//! on every acquire (`snatch.rs` `LockTrace::enter`). Capture allocates only
//! when library backtraces are enabled. The PBR workflow sets `RUST_BACKTRACE=1`,
//! which enables them, and that adds 48 allocations to every measured
//! `draw_into`. The stacks are `LockTrace::enter` → `Backtrace::capture` →
//! the unwind walker. Panic backtraces still follow `RUST_BACKTRACE`.
//!
//! The standard library caches the env var on the first capture, so this has
//! to run before any device exists.

/// Disable library backtraces for the rest of this process.
pub fn disable_library_backtraces() {
    // SAFETY: this test binary calls this on the test thread before it spawns
    // work or creates a wgpu device, so nothing else is reading the backtrace
    // environment cache yet. `set_var` is unsafe because a concurrent `getenv`
    // would race; there is no such caller here.
    unsafe {
        std::env::set_var("RUST_LIB_BACKTRACE", "0");
    }
}
