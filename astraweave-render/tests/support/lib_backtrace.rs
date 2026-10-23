//! The allocation-floor tests require `RUST_LIB_BACKTRACE=0` in the process
//! environment.
//!
//! `wgpu-core`'s debug snatch lock calls `std::backtrace::Backtrace::capture`
//! on every acquire (`snatch.rs` `LockTrace::enter`). Capture allocates only
//! when library backtraces are enabled. The PBR workflow sets `RUST_BACKTRACE=1`,
//! which enables them, and that adds 48 allocations to every measured
//! `draw_into`. Panic backtraces still follow `RUST_BACKTRACE`.
//!
//! `Backtrace::enabled` caches the first read. `std::env::set_var` from a
//! libtest worker is a data race with glibc `getenv`, and it cannot turn a
//! cached capture back off. The workflow exports `RUST_LIB_BACKTRACE=0`
//! before `main`.

/// Fail unless this process was started with library backtraces off.
pub fn require_library_backtraces_off() {
    let value = std::env::var_os("RUST_LIB_BACKTRACE");
    assert!(
        value.as_deref() == Some(std::ffi::OsStr::new("0")),
        "export RUST_LIB_BACKTRACE=0 before starting this process"
    );
}
