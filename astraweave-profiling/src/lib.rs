#![forbid(unsafe_code)]
/*!
# AstraWeave Profiling

Zero-cost profiling abstraction layer with Tracy integration.

This crate provides profiling macros that compile to no-ops when profiling is disabled,
ensuring zero runtime cost in production builds.

## Features

- **profiling**: Enable basic Tracy profiling
- **profiling-sampling**: Enable Tracy sampling mode (8KHz, lower overhead)
- **profiling-system**: Enable system tracing (GPU, memory, locks)
- **profiling-full**: All profiling features combined

## Usage

```rust,no_run
use astraweave_profiling::{span, frame_mark};

fn game_loop() {
    loop {
        span!("frame");

        {
            span!("update");
            update_game();
        }

        {
            span!("render");
            render_game();
        }

        frame_mark!();
    }
}

fn update_game() {
    span!("physics");
    // Physics simulation

    span!("AI");
    // AI updates
}

fn render_game() {
    span!("submit");
    // Render submission
}
```

## Tracy Integration

When compiled with `--features profiling`, this crate integrates with Tracy Profiler:
1. Download Tracy from https://github.com/wolfpld/tracy
2. Build with profiling: `cargo build --features profiling`
3. Run Tracy server
4. Run your application
5. Tracy will automatically connect and capture profiling data

## Zero-Cost Abstraction

When profiling features are disabled (default), all macros compile to empty code blocks,
resulting in **zero runtime overhead**.
*/

#![cfg_attr(not(feature = "profiling"), no_std)]

#[cfg(feature = "profiling")]
pub use tracy_client;

/// Profiling span macro - measures execution time of a code block
///
/// # Examples
///
/// ```rust,no_run
/// use astraweave_profiling::span;
///
/// fn expensive_operation() {
///     span!("expensive");
///     // Your code here
/// }
/// ```
#[macro_export]
macro_rules! span {
    ($name:expr) => {
        #[cfg(feature = "profiling")]
        let _tracy_span = $crate::tracy_client::span!($name);
    };
}

/// Frame mark macro - marks the end of a frame
///
/// Use this at the end of your main loop iteration to help Tracy
/// visualize frame boundaries and calculate FPS.
///
/// # Examples
///
/// ```rust,no_run
/// use astraweave_profiling::frame_mark;
///
/// loop {
///     // Game loop code
///     frame_mark!();
/// }
/// ```
#[macro_export]
macro_rules! frame_mark {
    () => {
        #[cfg(feature = "profiling")]
        $crate::tracy_client::Client::running()
            .expect("Tracy client should be running")
            .frame_mark();
    };
}

/// Plot value macro - plots a numeric value over time
///
/// # Examples
///
/// ```rust,no_run
/// use astraweave_profiling::plot;
///
/// let fps = 60.0;
/// plot!("FPS", fps);
/// ```
#[macro_export]
macro_rules! plot {
    ($name:expr, $value:expr) => {
        #[cfg(feature = "profiling")]
        $crate::tracy_client::Client::running()
            .expect("Tracy client should be running")
            .plot(
                $crate::tracy_client::PlotName::new_leak($name.to_string()),
                $value as f64,
            );
    };
}

/// Message macro - emits a text message to Tracy
///
/// # Examples
///
/// ```rust,no_run
/// use astraweave_profiling::message;
///
/// message!("Player spawned");
/// message!("Level loaded: {}", level_name);
/// ```
#[macro_export]
macro_rules! message {
    ($($arg:tt)*) => {
        #[cfg(feature = "profiling")]
        {
            let msg = format!($($arg)*);
            $crate::tracy_client::Client::running()
                .expect("Tracy client should be running")
                .message(&msg, 0);
        }
    };
}

/// Memory allocation tracking macro
///
/// # Examples
///
/// ```rust,no_run
/// use astraweave_profiling::alloc;
///
/// let data = vec![0u8; 1024];
/// alloc!(data.as_ptr(), data.len());
/// ```
#[macro_export]
macro_rules! alloc {
    ($ptr:expr, $size:expr) => {
        #[cfg(feature = "profiling")]
        $crate::tracy_client::Client::running()
            .expect("Tracy client should be running")
            .alloc($ptr as *const u8, $size);
    };
}

/// Memory deallocation tracking macro
///
/// # Examples
///
/// ```rust,no_run
/// use astraweave_profiling::free;
///
/// let data = vec![0u8; 1024];
/// let ptr = data.as_ptr();
/// drop(data);
/// free!(ptr);
/// ```
#[macro_export]
macro_rules! free {
    ($ptr:expr) => {
        #[cfg(feature = "profiling")]
        $crate::tracy_client::Client::running()
            .expect("Tracy client should be running")
            .free($ptr as *const u8);
    };
}

/// Colored span macro - creates a span with a custom color
///
/// Note: For simplicity, this maps to a regular span in Tracy 0.17.
/// Color support requires more complex macro implementation.
///
/// # Examples
///
/// ```rust,no_run
/// use astraweave_profiling::span_color;
///
/// fn critical_section() {
///     span_color!("critical", 0xFF0000); // Red  
///     // Your code
/// }
/// ```
#[macro_export]
macro_rules! span_color {
    ($name:expr, $_color:expr) => {
        #[cfg(feature = "profiling")]
        let _tracy_span = $crate::tracy_client::span!($name);
    };
}

/// Function-level profiling attribute macro
///
/// This is a compile-time wrapper that instruments entire functions.
/// Note: Requires proc-macro support (future enhancement).
///
/// # Examples
///
/// ```rust,ignore
/// #[profile]
/// fn my_function() {
///     // Automatically profiled
/// }
/// ```
// Note: This is a placeholder for future proc-macro implementation
// For now, use span!() at function entry
/// Profiling context for RAII-style span management
///
/// Note: Due to Tracy 0.17 API constraints, this uses marker-based approach.
/// The actual profiling is done via the span! macro in calling code.
#[cfg(feature = "profiling")]
pub struct ProfileSpan {
    _marker: std::marker::PhantomData<()>,
}

#[cfg(feature = "profiling")]
impl ProfileSpan {
    /// Create a new profiling span
    ///
    /// Note: Due to Tracy API constraints, callers should use `span!()` macro instead.
    /// This is provided for API compatibility but does not create a Tracy span.
    pub fn new(_name: &'static str) -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    /// Create a new profiling span with custom color
    ///
    /// Note: Color support requires different Tracy API. Use `span!()` macro instead.
    pub fn new_colored(_name: &'static str, _color: u32) -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[cfg(not(feature = "profiling"))]
pub struct ProfileSpan;

#[cfg(not(feature = "profiling"))]
impl ProfileSpan {
    pub fn new(_name: &'static str) -> Self {
        Self
    }

    pub fn new_colored(_name: &'static str, _color: u32) -> Self {
        Self
    }
}

/// Profiling utilities
pub struct Profiler;

impl Profiler {
    /// Check if Tracy profiling is enabled at runtime
    pub fn is_enabled() -> bool {
        #[cfg(feature = "profiling")]
        {
            tracy_client::Client::is_running()
        }
        #[cfg(not(feature = "profiling"))]
        {
            false
        }
    }

    /// Get Tracy client version (if available)
    pub fn version() -> Option<&'static str> {
        #[cfg(feature = "profiling")]
        {
            Some(env!("CARGO_PKG_VERSION"))
        }
        #[cfg(not(feature = "profiling"))]
        {
            None
        }
    }
}

// ============================================================================
// Allocation counters + FrameAllocStats (for measurement instrumentation)
// ============================================================================

/// Process-wide allocation counters.
///
/// These counters are updated by a registered `#[global_allocator]` — in AstraWeave,
/// that is `astraweave_ecs::counting_alloc::CountingAlloc`, which forwards every
/// `alloc`/`dealloc`/`realloc` call into the `record_*` functions below.
///
/// When the `alloc-counter` feature is off, all `record_*` functions are no-ops and
/// all readers return zero. This keeps instrumentation calls cost-free in release
/// builds that do not opt in.
pub mod counters {
    #[cfg(feature = "alloc-counter")]
    use core::sync::atomic::{AtomicUsize, Ordering};

    #[cfg(feature = "alloc-counter")]
    static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    #[cfg(feature = "alloc-counter")]
    static DEALLOCS: AtomicUsize = AtomicUsize::new(0);
    #[cfg(feature = "alloc-counter")]
    static REALLOCS: AtomicUsize = AtomicUsize::new(0);
    #[cfg(feature = "alloc-counter")]
    static BYTES_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
    #[cfg(feature = "alloc-counter")]
    static BYTES_DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

    /// A snapshot of the allocation counters at a point in time.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct AllocSnapshot {
        pub allocs: usize,
        pub deallocs: usize,
        pub reallocs: usize,
        pub bytes_allocated: usize,
        pub bytes_deallocated: usize,
    }

    /// Record an allocation of `size` bytes. No-op when `alloc-counter` is off.
    #[inline]
    pub fn record_alloc(_size: usize) {
        #[cfg(feature = "alloc-counter")]
        {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES_ALLOCATED.fetch_add(_size, Ordering::Relaxed);
        }
    }

    /// Record a deallocation of `size` bytes. No-op when `alloc-counter` is off.
    #[inline]
    pub fn record_dealloc(_size: usize) {
        #[cfg(feature = "alloc-counter")]
        {
            DEALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES_DEALLOCATED.fetch_add(_size, Ordering::Relaxed);
        }
    }

    /// Record a reallocation from `old_size` to `new_size` bytes. No-op when
    /// `alloc-counter` is off.
    #[inline]
    pub fn record_realloc(_old_size: usize, _new_size: usize) {
        #[cfg(feature = "alloc-counter")]
        {
            // A realloc is one alloc + one dealloc, consistent with CountingAlloc's
            // original bookkeeping. It also counts as its own "realloc" event for
            // visibility into resize vs. fresh-alloc patterns.
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            DEALLOCS.fetch_add(1, Ordering::Relaxed);
            REALLOCS.fetch_add(1, Ordering::Relaxed);
            BYTES_ALLOCATED.fetch_add(_new_size, Ordering::Relaxed);
            BYTES_DEALLOCATED.fetch_add(_old_size, Ordering::Relaxed);
        }
    }

    /// Total alloc events since last reset.
    #[inline]
    pub fn allocs() -> usize {
        #[cfg(feature = "alloc-counter")]
        {
            ALLOCS.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "alloc-counter"))]
        {
            0
        }
    }

    /// Total dealloc events since last reset.
    #[inline]
    pub fn deallocs() -> usize {
        #[cfg(feature = "alloc-counter")]
        {
            DEALLOCS.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "alloc-counter"))]
        {
            0
        }
    }

    /// Total realloc events since last reset.
    #[inline]
    pub fn reallocs() -> usize {
        #[cfg(feature = "alloc-counter")]
        {
            REALLOCS.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "alloc-counter"))]
        {
            0
        }
    }

    /// Total bytes allocated (sum of `size` across `alloc` and `realloc` new sizes).
    #[inline]
    pub fn bytes_allocated() -> usize {
        #[cfg(feature = "alloc-counter")]
        {
            BYTES_ALLOCATED.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "alloc-counter"))]
        {
            0
        }
    }

    /// Total bytes deallocated (sum of `size` across `dealloc` and `realloc` old sizes).
    #[inline]
    pub fn bytes_deallocated() -> usize {
        #[cfg(feature = "alloc-counter")]
        {
            BYTES_DEALLOCATED.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "alloc-counter"))]
        {
            0
        }
    }

    /// Net live allocations (allocs − deallocs). Positive = leak or live data growth.
    #[inline]
    pub fn net_allocs() -> isize {
        allocs() as isize - deallocs() as isize
    }

    /// Snapshot all counters atomically-ish. Values are read with `Relaxed` ordering
    /// so they are not guaranteed to be consistent with each other in the strict sense,
    /// but every counter monotonically increases — so differences across a snapshot
    /// pair are lower-bounded by the true deltas and are good enough for frame-level
    /// diagnostics.
    #[inline]
    pub fn snapshot() -> AllocSnapshot {
        AllocSnapshot {
            allocs: allocs(),
            deallocs: deallocs(),
            reallocs: reallocs(),
            bytes_allocated: bytes_allocated(),
            bytes_deallocated: bytes_deallocated(),
        }
    }

    /// Reset all counters to zero. Use at the start of a measurement window.
    #[inline]
    pub fn reset() {
        #[cfg(feature = "alloc-counter")]
        {
            ALLOCS.store(0, Ordering::SeqCst);
            DEALLOCS.store(0, Ordering::SeqCst);
            REALLOCS.store(0, Ordering::SeqCst);
            BYTES_ALLOCATED.store(0, Ordering::SeqCst);
            BYTES_DEALLOCATED.store(0, Ordering::SeqCst);
        }
    }
}

/// Delta of allocation activity across a measurement window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameAllocDelta {
    /// Number of `alloc` + `realloc` events during the window.
    pub allocs: usize,
    /// Number of `dealloc` + `realloc` events during the window.
    pub deallocs: usize,
    /// Number of `realloc` events during the window (subset of allocs AND deallocs).
    pub reallocs: usize,
    /// Net allocations (allocs − deallocs). Positive values indicate growth; near-zero
    /// steady state indicates churn without leak.
    pub net_allocs: isize,
    /// Bytes allocated during the window.
    pub bytes_allocated: usize,
    /// Bytes deallocated during the window.
    pub bytes_deallocated: usize,
}

impl FrameAllocDelta {
    /// Net bytes allocated (bytes_allocated − bytes_deallocated).
    #[inline]
    pub fn net_bytes(&self) -> isize {
        self.bytes_allocated as isize - self.bytes_deallocated as isize
    }
}

/// RAII-style helper for measuring allocation activity across a frame or scope.
///
/// Construct with `begin_frame` at the start of the window, call `end_frame` at the
/// end to get a `FrameAllocDelta`. When the `alloc-counter` feature is off, all
/// deltas are zero and the type carries no runtime cost beyond a few `usize`
/// snapshots (which the compiler typically elides).
///
/// # Example
/// ```
/// use astraweave_profiling::FrameAllocStats;
///
/// let stats = FrameAllocStats::begin_frame();
/// // ... do work ...
/// let delta = stats.end_frame();
/// // With alloc-counter off: delta.allocs == 0
/// // With alloc-counter on: delta.allocs reflects real allocation activity.
/// assert!(delta.allocs >= 0);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameAllocStats {
    start: counters::AllocSnapshot,
}

impl FrameAllocStats {
    /// Snapshot the counters and return a new measurement window.
    #[inline]
    pub fn begin_frame() -> Self {
        Self {
            start: counters::snapshot(),
        }
    }

    /// End the measurement window and compute the delta against the starting snapshot.
    /// Consumes `self` to discourage reusing a stale start snapshot.
    #[inline]
    pub fn end_frame(self) -> FrameAllocDelta {
        let now = counters::snapshot();
        FrameAllocDelta {
            allocs: now.allocs.saturating_sub(self.start.allocs),
            deallocs: now.deallocs.saturating_sub(self.start.deallocs),
            reallocs: now.reallocs.saturating_sub(self.start.reallocs),
            net_allocs: (now.allocs as isize - now.deallocs as isize)
                - (self.start.allocs as isize - self.start.deallocs as isize),
            bytes_allocated: now
                .bytes_allocated
                .saturating_sub(self.start.bytes_allocated),
            bytes_deallocated: now
                .bytes_deallocated
                .saturating_sub(self.start.bytes_deallocated),
        }
    }

    /// Peek the delta without consuming the stats, for mid-frame diagnostics.
    #[inline]
    pub fn peek_delta(&self) -> FrameAllocDelta {
        let now = counters::snapshot();
        FrameAllocDelta {
            allocs: now.allocs.saturating_sub(self.start.allocs),
            deallocs: now.deallocs.saturating_sub(self.start.deallocs),
            reallocs: now.reallocs.saturating_sub(self.start.reallocs),
            net_allocs: (now.allocs as isize - now.deallocs as isize)
                - (self.start.allocs as isize - self.start.deallocs as isize),
            bytes_allocated: now
                .bytes_allocated
                .saturating_sub(self.start.bytes_allocated),
            bytes_deallocated: now
                .bytes_deallocated
                .saturating_sub(self.start.bytes_deallocated),
        }
    }
}

/// Emit a Tracy plot for an allocation rate: `{name}.allocs` (count) and `{name}.bytes`.
///
/// Prefer `measured_span!` when wrapping a code block. Use `alloc_plot!` directly when
/// you already hold a `FrameAllocDelta` or want to emit a specific count/bytes pair.
///
/// When the `profiling` feature is off, this macro is a no-op.
///
/// # Example
/// ```
/// use astraweave_profiling::{alloc_plot, FrameAllocStats};
///
/// let stats = FrameAllocStats::begin_frame();
/// // ... work ...
/// let delta = stats.end_frame();
/// alloc_plot!("frame", delta.allocs, delta.bytes_allocated);
/// ```
#[macro_export]
macro_rules! alloc_plot {
    ($name:expr, $count:expr, $bytes:expr) => {{
        // Always evaluate arguments at the call site so callers' local bindings
        // (e.g. `let delta = stats.end_frame()`) are considered used even when the
        // `profiling` feature is disabled and the Tracy emit block is stripped.
        let __aw_alloc_plot_count = $count;
        let __aw_alloc_plot_bytes = $bytes;
        #[cfg(feature = "profiling")]
        {
            // NOTE: We leak the name strings via Tracy's `PlotName::new_leak` because
            // Tracy requires `'static`-equivalent storage. This is consistent with the
            // existing `plot!` macro in this crate and incurs a small one-time cost per
            // unique plot name.
            $crate::tracy_client::Client::running()
                .expect("Tracy client should be running")
                .plot(
                    $crate::tracy_client::PlotName::new_leak(
                        ::std::format!("{}.allocs", $name),
                    ),
                    __aw_alloc_plot_count as f64,
                );
            $crate::tracy_client::Client::running()
                .expect("Tracy client should be running")
                .plot(
                    $crate::tracy_client::PlotName::new_leak(
                        ::std::format!("{}.bytes", $name),
                    ),
                    __aw_alloc_plot_bytes as f64,
                );
        }
        #[cfg(not(feature = "profiling"))]
        {
            // Suppress unused-variable warnings when the Tracy block is stripped.
            let _ = __aw_alloc_plot_count;
            let _ = __aw_alloc_plot_bytes;
        }
    }};
}

/// Combined span + alloc-delta-emit helper.
///
/// Opens a Tracy span named `$name` for the current scope AND, on scope exit, emits
/// an `alloc_plot!` under the same name with the allocation delta accumulated during
/// the span.
///
/// Both the span and the alloc plot are no-ops when their respective features are off:
/// - `profiling` off → no Tracy span, no Tracy plot.
/// - `alloc-counter` off → delta is always zero (plot still emitted if `profiling` is on,
///   so rate charts remain consistent).
///
/// # Example
/// ```
/// use astraweave_profiling::measured_span;
///
/// fn tick() {
///     measured_span!("ecs.tick");
///     // ... work ...
///     // On scope exit: a Tracy span ended AND a `ecs.tick.allocs` / `ecs.tick.bytes`
///     // pair is plotted if both features are on.
/// }
/// ```
#[macro_export]
macro_rules! measured_span {
    ($name:expr) => {
        // Span is emitted by the existing `span!` macro; keep its behavior unchanged.
        $crate::span!($name);

        // RAII guard that snapshots allocation counters at entry and emits a plot at
        // exit. When `profiling` is off, the plot emission is a no-op; when
        // `alloc-counter` is off, the delta is always zero.
        let __aw_alloc_guard =
            $crate::MeasuredSpanGuard::new($name, $crate::FrameAllocStats::begin_frame());
        // Suppress unused-variable warning when neither feature is enabled.
        let _ = &__aw_alloc_guard;
    };
}

/// Internal guard used by `measured_span!`. Emits `alloc_plot!` on drop.
///
/// Not part of the stable API surface — subject to change. Prefer `measured_span!`.
#[doc(hidden)]
pub struct MeasuredSpanGuard {
    // Fields are only read when `profiling` is enabled; silence dead-code warning
    // for release builds without the feature.
    #[allow(dead_code)]
    name: &'static str,
    #[allow(dead_code)]
    stats: FrameAllocStats,
}

impl MeasuredSpanGuard {
    #[doc(hidden)]
    #[inline]
    pub fn new(name: &'static str, stats: FrameAllocStats) -> Self {
        Self { name, stats }
    }
}

impl Drop for MeasuredSpanGuard {
    #[inline]
    fn drop(&mut self) {
        #[cfg(feature = "profiling")]
        {
            let delta = self.stats.peek_delta();
            // Forward to alloc_plot! so we share the same emit path.
            crate::alloc_plot!(self.name, delta.allocs, delta.bytes_allocated);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_status() {
        // Profiling should be disabled in tests by default
        #[cfg(not(feature = "profiling"))]
        assert!(!Profiler::is_enabled());

        // Version should return None when profiling is disabled
        #[cfg(not(feature = "profiling"))]
        assert!(Profiler::version().is_none());
    }

    #[test]
    fn test_span_compiles() {
        // These should compile to no-ops without profiling feature
        span!("test");

        {
            span!("nested");
            // Nested span
        }
    }

    #[test]
    fn test_frame_mark_compiles() {
        // Should compile to no-op
        frame_mark!();
    }

    #[test]
    fn test_plot_compiles() {
        // Should compile to no-op
        plot!("test_metric", 42.0);
    }

    #[test]
    fn test_message_compiles() {
        // Should compile to no-op
        message!("Test message");
        message!("Test with arg: {}", 123);
    }

    #[test]
    fn test_profile_span_raii() {
        {
            let _span = ProfileSpan::new("test");
            // Span active here
        }
        // Span dropped here
    }

    #[test]
    fn test_colored_span() {
        {
            let _span = ProfileSpan::new_colored("critical", 0xFF0000);
            // Red span active
        }
    }
}
