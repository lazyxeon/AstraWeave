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
        if !count_this_alloc(size) {
            return INNER.alloc(layout);
        }
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
        if !count_this_alloc(size) {
            return INNER.alloc_zeroed(layout);
        }
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
        if !count_this_alloc(new_size) {
            return INNER.realloc(ptr, layout, new_size);
        }
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

/// One `draw_into` (or any other measured region) grouped by call stack.
///
/// `total` is the number of counted allocations that were recorded. It matches
/// `allocs()` for that region when every counted allocation fit in the site
/// table. Allocations made while a backtrace is being captured are not counted
/// and not recorded, so turning the trace on does not move the floor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllocTrace {
    pub total: usize,
    pub sites: usize,
    pub dropped: usize,
    pub report: String,
}

/// Start recording call stacks for counted allocations.
///
/// The table is fixed and reserved before the flag is set, so recording a site
/// does not allocate. Call [`end_alloc_trace`] on the same thread when the
/// region ends. Nested or re-entered allocations (the capture itself) are
/// ignored until the outer allocation returns.
pub fn begin_alloc_trace() {
    #[cfg(unix)]
    {
        trace::begin();
    }
}

/// Stop recording and return the sites, heaviest first.
///
/// Symbol names are resolved here, after the flag is clear, so formatting
/// does not show up in the counts.
pub fn end_alloc_trace() -> AllocTrace {
    #[cfg(unix)]
    {
        trace::end()
    }
    #[cfg(not(unix))]
    {
        AllocTrace {
            total: 0,
            sites: 0,
            dropped: 0,
            report: "alloc-sites unsupported\n".to_string(),
        }
    }
}

/// `false` means this call is the tracer capturing its own stack. Those
/// bytes are not part of the measured region.
#[inline]
fn count_this_alloc(size: usize) -> bool {
    #[cfg(unix)]
    {
        if trace::TRACE.load(Ordering::Relaxed) {
            return trace::record_or_reentered(size);
        }
    }
    let _ = size;
    true
}

#[cfg(unix)]
mod trace {
    use super::AllocTrace;
    use std::cell::Cell;
    use std::cell::UnsafeCell;
    use std::collections::HashMap;
    use std::ffi::{c_char, c_void};
    use std::sync::atomic::{AtomicBool, Ordering};

    const MAX_FRAMES: usize = 24;
    const MAX_SITES: usize = 512;

    pub(super) static TRACE: AtomicBool = AtomicBool::new(false);

    thread_local! {
        static IN_TRACE: Cell<bool> = const { Cell::new(false) };
    }

    #[derive(Clone, Copy)]
    struct Site {
        frames: [usize; MAX_FRAMES],
        nframes: u8,
        count: u32,
        bytes: u64,
    }

    impl Site {
        const EMPTY: Self = Self {
            frames: [0; MAX_FRAMES],
            nframes: 0,
            count: 0,
            bytes: 0,
        };
    }

    struct Table {
        sites: [Site; MAX_SITES],
        used: usize,
        dropped: u32,
    }

    impl Table {
        const EMPTY: Self = Self {
            sites: [Site::EMPTY; MAX_SITES],
            used: 0,
            dropped: 0,
        };

        fn clear(&mut self) {
            self.used = 0;
            self.dropped = 0;
        }
    }

    struct TableCell(UnsafeCell<Table>);

    // SAFETY: `with_table` is the only reader and writer, and it holds `LOCK`
    // across the whole access. The cell is never aliased without that lock.
    unsafe impl Sync for TableCell {}

    static LOCK: AtomicBool = AtomicBool::new(false);
    static TABLE: TableCell = TableCell(UnsafeCell::new(Table::EMPTY));

    struct Unlock;

    impl Drop for Unlock {
        fn drop(&mut self) {
            LOCK.store(false, Ordering::Release);
        }
    }

    fn with_table<R>(body: impl FnOnce(&mut Table) -> R) -> R {
        while LOCK
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        let _unlock = Unlock;
        // SAFETY: `LOCK` is held, so this thread is the only one in the table.
        body(unsafe { &mut *TABLE.0.get() })
    }

    pub(super) fn begin() {
        // Warm the thread-local before the flag is set. Its first access must
        // not be counted as a frame allocation.
        let _ = IN_TRACE.try_with(|_| {});
        with_table(Table::clear);
        TRACE.store(true, Ordering::SeqCst);
    }

    pub(super) fn end() -> AllocTrace {
        let (sites, dropped) = {
            // Clear the flag while holding the lock so an in-flight recorder
            // either finishes its insert or observes the flag and skips.
            with_table(|table| {
                TRACE.store(false, Ordering::SeqCst);
                let taken = table.sites[..table.used].to_vec();
                let dropped = table.dropped;
                table.clear();
                (taken, dropped)
            })
        };
        format_sites(&sites, dropped)
    }

    /// Record this stack, or return `false` when already inside a capture.
    pub(super) fn record_or_reentered(size: usize) -> bool {
        let reentered = IN_TRACE.with(|flag| {
            if flag.get() {
                return true;
            }
            flag.set(true);
            false
        });
        if reentered {
            return false;
        }
        record(size);
        IN_TRACE.with(|flag| flag.set(false));
        true
    }

    fn record(size: usize) {
        let mut raw = [std::ptr::null_mut::<c_void>(); MAX_FRAMES];
        // SAFETY: `raw` holds `MAX_FRAMES` pointers and that length is the
        // second argument. `backtrace` writes at most that many frames.
        let n = unsafe { backtrace(raw.as_mut_ptr(), MAX_FRAMES as i32) };
        let n = usize::try_from(n).unwrap_or(0).min(MAX_FRAMES);
        let mut frames = [0usize; MAX_FRAMES];
        for (slot, ptr) in frames.iter_mut().zip(raw.iter()).take(n) {
            *slot = *ptr as usize;
        }
        let nframes = u8::try_from(n).unwrap_or(u8::MAX);
        with_table(|table| {
            if !TRACE.load(Ordering::Relaxed) {
                return;
            }
            for site in table.sites.iter_mut().take(table.used) {
                if site.nframes == nframes && site.frames[..n] == frames[..n] {
                    site.count = site.count.saturating_add(1);
                    site.bytes = site.bytes.saturating_add(size as u64);
                    return;
                }
            }
            if table.used >= MAX_SITES {
                table.dropped = table.dropped.saturating_add(1);
                return;
            }
            table.sites[table.used] = Site {
                frames,
                nframes,
                count: 1,
                bytes: size as u64,
            };
            table.used += 1;
        });
    }

    fn format_sites(sites: &[Site], dropped: u32) -> AllocTrace {
        let mut pcs = Vec::new();
        for site in sites {
            pcs.extend_from_slice(&site.frames[..site.nframes as usize]);
        }
        let names = symbolize(&pcs);
        let mut grouped: Vec<(u32, u64, Vec<String>)> = Vec::new();
        for site in sites {
            let frames = render_frames(&site.frames[..site.nframes as usize], &names);
            if let Some(existing) = grouped.iter_mut().find(|(_, _, got)| got == &frames) {
                existing.0 = existing.0.saturating_add(site.count);
                existing.1 = existing.1.saturating_add(site.bytes);
            } else {
                grouped.push((site.count, site.bytes, frames));
            }
        }
        grouped.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(&b.2)));
        let total = grouped.iter().fold(0usize, |sum, (count, _, _)| {
            sum.saturating_add(*count as usize)
        }) + dropped as usize;
        let mut report = format!(
            "alloc-sites total={total} sites={} dropped={dropped}\n",
            grouped.len()
        );
        for (count, bytes, frames) in &grouped {
            report.push_str(&format!("{count}\t{bytes}\n"));
            for frame in frames {
                report.push('\t');
                report.push_str(frame);
                report.push('\n');
            }
        }
        AllocTrace {
            total,
            sites: grouped.len(),
            dropped: dropped as usize,
            report,
        }
    }

    fn render_frames(pcs: &[usize], names: &HashMap<usize, String>) -> Vec<String> {
        if pcs.is_empty() {
            return vec!["<no frames>".to_string()];
        }
        let mut kept = Vec::new();
        for pc in pcs {
            let line = names
                .get(pc)
                .cloned()
                .unwrap_or_else(|| format!("0x{pc:x}"));
            if is_tracer_frame(&line) {
                continue;
            }
            kept.push(line);
        }
        if kept.is_empty() {
            kept.push("<tracer only>".to_string());
        }
        kept
    }

    fn is_tracer_frame(line: &str) -> bool {
        // Keep `std::backtrace::Backtrace::capture` and
        // `wgpu_core::snatch::trace::LockTrace::enter`. Those are the sites
        // this trace exists to show. Only the recorder and the alloc shim drop.
        line.contains("counting_alloc")
            || line.contains("CountingAlloc")
            || line.contains("_Unwind_")
            || line.contains("__rust_alloc")
            || line.contains("alloc::alloc::")
    }

    /// Resolve program-counter offsets with `addr2line` after tracing has
    /// stopped. One process parses the debug info for every unique frame.
    fn symbolize(pcs: &[usize]) -> HashMap<usize, String> {
        let mut map = HashMap::new();
        let mut order = Vec::new();
        let mut offsets = Vec::new();
        for &pc in pcs {
            if map.contains_key(&pc) {
                continue;
            }
            let offset = file_offset(pc);
            map.insert(pc, format!("0x{offset:x}"));
            order.push(pc);
            offsets.push(offset);
        }
        if order.is_empty() {
            return map;
        }
        let Ok(exe) = std::env::current_exe() else {
            return map;
        };
        let mut cmd = std::process::Command::new("addr2line");
        cmd.arg("-e").arg(exe).arg("-f").arg("-C");
        for offset in &offsets {
            cmd.arg(format!("0x{offset:x}"));
        }
        let Ok(output) = cmd.output() else {
            return map;
        };
        if !output.status.success() {
            return map;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = text.lines().collect();
        for (index, pc) in order.iter().enumerate() {
            let name = lines.get(index * 2).copied().unwrap_or("");
            let loc = lines.get(index * 2 + 1).copied().unwrap_or("");
            if name.is_empty() || name == "??" {
                continue;
            }
            map.insert(*pc, short_loc(name, loc));
        }
        map
    }

    fn short_loc(name: &str, loc: &str) -> String {
        let file = loc.rsplit(['/', '\\']).next().unwrap_or(loc);
        if file.is_empty() || file.starts_with('?') {
            name.to_string()
        } else {
            format!("{name} ({file})")
        }
    }

    fn file_offset(pc: usize) -> usize {
        let mut info = DlInfo {
            dli_fname: std::ptr::null(),
            dli_fbase: std::ptr::null_mut(),
            dli_sname: std::ptr::null(),
            dli_saddr: std::ptr::null_mut(),
        };
        // SAFETY: `info` is a writable `Dl_info`. `dladdr` only reads `pc` as
        // an address and fills `info`. A zero return leaves the offset raw.
        let found = unsafe { dladdr(pc as *const c_void, &mut info) };
        if found == 0 || info.dli_fbase.is_null() {
            return pc;
        }
        pc.wrapping_sub(info.dli_fbase as usize)
    }

    #[repr(C)]
    struct DlInfo {
        dli_fname: *const c_char,
        dli_fbase: *mut c_void,
        dli_sname: *const c_char,
        dli_saddr: *mut c_void,
    }

    extern "C" {
        fn backtrace(buffer: *mut *mut c_void, size: i32) -> i32;
        fn dladdr(addr: *const c_void, info: *mut DlInfo) -> i32;
    }
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

    #[test]
    fn alloc_trace_is_quiet_until_a_frame_records() {
        begin_alloc_trace();
        let trace = end_alloc_trace();
        assert!(
            trace
                .report
                .starts_with("alloc-sites total=0 sites=0 dropped=0\n")
                || trace.report == "alloc-sites unsupported\n",
            "{}",
            trace.report
        );
        assert_eq!(trace.total, 0);
    }
}
