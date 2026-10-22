//! Keep `Backtrace::capture` off during allocation-floor tests.
//!
//! `wgpu-core`'s debug snatch lock calls `std::backtrace::Backtrace::capture`
//! on every acquire (`snatch.rs` `LockTrace::enter`). Capture allocates only
//! when library backtraces are enabled. The PBR workflow sets `RUST_BACKTRACE=1`,
//! which enables them, and that adds 48 allocations to every measured
//! `draw_into`. The stacks are `LockTrace::enter` → `Backtrace::capture` →
//! the unwind walker. Panic backtraces still follow `RUST_BACKTRACE`.
//!
//! `Backtrace::enabled` caches the first answer in a process-global
//! `AtomicU8`. A later `set_var` does nothing once anything has captured.
//! The CI job sets `RUST_LIB_BACKTRACE=0` in the environment before `main`.
//! This call is only the local backstop, and it has to be the first thing
//! the test does, before any other test in the binary constructs a device.

/// Disable library backtraces for the rest of this process.
pub fn disable_library_backtraces() {
    // SAFETY: this test calls this before it spawns work or creates a wgpu
    // device. `set_var` is unsafe because a concurrent `getenv` would race.
    // It cannot undo a capture that already cached `enabled == true`.
    unsafe {
        std::env::set_var("RUST_LIB_BACKTRACE", "0");
    }
}
