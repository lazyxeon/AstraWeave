//! SDK Benchmarks
//!
//! Measures performance of C ABI layer and FFI operations:
//! - FFI call overhead (C ABI boundary crossing)
//! - World creation and destruction via C API
//! - JSON serialization for C callbacks
//! - String marshalling (Rust ↔ C)
//! - Version query operations
//!
//! Performance targets:
//! - FFI call overhead: <50 ns (minimal boundary cost)
//! - World operations: <1 µs (fast enough for real-time)
//! - JSON serialization: <10 µs per snapshot
//! - String marshalling: <100 ns (efficient data transfer)

use astraweave_sdk::{
    aw_version, aw_version_string, aw_world_create, aw_world_destroy, aw_world_snapshot_json,
    aw_world_tick,
};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

// ============================================================================
// Benchmark 1: Version Query Operations
// ============================================================================

fn bench_version_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_operations");

    // Benchmark: FFI version struct query
    group.bench_function("aw_version", |b| {
        b.iter(|| {
            let version = aw_version();
            black_box(version)
        })
    });

    // Benchmark: Version string size query (buffer size detection)
    group.bench_function("aw_version_string_size", |b| {
        b.iter(|| {
            let size = unsafe { aw_version_string(std::ptr::null_mut(), 0) };
            black_box(size)
        })
    });

    // Benchmark: Full version string copy
    group.bench_function("aw_version_string_copy", |b| {
        let mut buffer = vec![0u8; 32];
        b.iter(|| {
            let size = unsafe { aw_version_string(buffer.as_mut_ptr(), buffer.len()) };
            black_box(size);
            black_box(buffer[0])
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark 2: World Lifecycle
// ============================================================================

fn bench_world_lifecycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_lifecycle");

    // Benchmark: Full world lifecycle (create + destroy)
    group.bench_function("world_create_destroy", |b| {
        b.iter(|| {
            let world = aw_world_create();
            aw_world_destroy(world);
            black_box(world)
        })
    });

    // Benchmark: World creation overhead only
    group.bench_function("world_create_only", |b| {
        b.iter(|| {
            let world = aw_world_create();
            black_box(world);
            aw_world_destroy(world);
        })
    });

    // Benchmark: World destruction overhead
    #[allow(clippy::redundant_closure)]
    group.bench_function("world_destroy", |b| {
        b.iter_batched(
            || aw_world_create(),
            |world| {
                aw_world_destroy(world);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Benchmark 3: World Tick (Runtime Update)
// ============================================================================

fn bench_world_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_tick");

    // Benchmark: World tick (baseline tick overhead)
    group.bench_function("tick_world", |b| {
        let world = aw_world_create();

        b.iter(|| {
            aw_world_tick(world, 0.016);
        });

        aw_world_destroy(world);
    });

    // Benchmark: Multiple ticks (simulate frame sequence)
    group.bench_function("tick_10_frames", |b| {
        let world = aw_world_create();

        b.iter(|| {
            for _ in 0..10 {
                aw_world_tick(world, 0.016);
            }
        });

        aw_world_destroy(world);
    });

    group.finish();
}

// ============================================================================
// Benchmark 4: JSON Serialization
// ============================================================================

fn bench_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialization");

    // Benchmark: Snapshot size query
    group.bench_function("snapshot_size_query", |b| {
        let world = aw_world_create();

        b.iter(|| {
            let size = aw_world_snapshot_json(world, std::ptr::null_mut(), 0);
            black_box(size)
        });

        aw_world_destroy(world);
    });

    // Benchmark: Full snapshot JSON copy
    group.bench_function("snapshot_json_copy", |b| {
        let world = aw_world_create();
        let mut buffer = vec![0u8; 4096];

        b.iter(|| {
            let size = aw_world_snapshot_json(world, buffer.as_mut_ptr(), buffer.len());
            black_box(size);
            black_box(buffer[0])
        });

        aw_world_destroy(world);
    });

    // Benchmark: Snapshot after tick (evolved state)
    group.bench_function("snapshot_after_tick", |b| {
        let world = aw_world_create();
        let mut buffer = vec![0u8; 4096];

        b.iter(|| {
            aw_world_tick(world, 0.016);
            let size = aw_world_snapshot_json(world, buffer.as_mut_ptr(), buffer.len());
            black_box(size)
        });

        aw_world_destroy(world);
    });

    group.finish();
}

// ============================================================================
// Benchmark 5: String Marshalling
// ============================================================================

fn bench_string_marshalling(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_marshalling");

    // Benchmark: CString creation (Rust → C string)
    group.bench_function("cstring_creation", |b| {
        b.iter(|| {
            let s = std::ffi::CString::new("test_string").unwrap();
            black_box(s.as_ptr())
        })
    });

    // Benchmark: CString with formatting
    group.bench_function("cstring_with_format", |b| {
        b.iter(|| {
            let s = std::ffi::CString::new(format!("Entity_{}", 123)).unwrap();
            black_box(s.as_ptr())
        })
    });

    // Benchmark: String from C buffer (C → Rust string)
    group.bench_function("string_from_c_buffer", |b| {
        let c_str = std::ffi::CString::new("test_string").unwrap();
        let ptr = c_str.as_ptr();

        b.iter(|| {
            let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
            black_box(s.to_string_lossy())
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark 6: FFI Overhead
// ============================================================================

fn bench_ffi_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_overhead");

    // Benchmark: Minimal FFI call (version query as baseline)
    group.bench_function("minimal_ffi_call", |b| {
        b.iter(|| {
            let v = aw_version();
            black_box(v)
        })
    });

    // Benchmark: FFI with pointer argument
    group.bench_function("ffi_with_ptr_arg", |b| {
        b.iter(|| {
            let size = unsafe { aw_version_string(std::ptr::null_mut(), 0) };
            black_box(size)
        })
    });

    // Benchmark: FFI with data marshalling
    group.bench_function("ffi_with_marshalling", |b| {
        let mut buffer = vec![0u8; 32];
        b.iter(|| {
            let size = unsafe { aw_version_string(buffer.as_mut_ptr(), buffer.len()) };
            black_box(size);
            black_box(buffer[0])
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_version_operations,
    bench_world_lifecycle,
    bench_world_tick,
    bench_json_serialization,
    bench_string_marshalling,
    bench_ffi_overhead,
);
criterion_main!(benches);
