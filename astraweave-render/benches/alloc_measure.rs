// SPDX-License-Identifier: MIT
//! Allocation-count regression bench for render hot paths that are exercisable
//! without a live wgpu `Device`.
//!
//! Scope:
//! - `bin_lights_cpu` — pure CPU, trivially benchable. Audit §2.3 #3 flagged
//!   this as a per-frame suspect with 3-4 `Vec<u32>` allocations per call.
//!
//! Out of scope here (require full renderer):
//! - `render.submit` — needs a wgpu surface; captured via `profiling_demo`.
//! - `render.visible_instances` — needs a constructed `Renderer`.
//! - Both are captured in Tracy when running `profiling_demo --features profiling,alloc-counter`.
//!   See `docs/audits/allocation_measurement_plan_<date>.md`.
//!
//! # How to run
//!
//! ```bash
//! cargo bench -p astraweave-render --features alloc-counter --bench alloc_measure
//! ```

// Local `System`-forwarding counting allocator (see physics bench for the same pattern).
#[cfg(feature = "alloc-counter")]
mod bench_alloc {
    use std::alloc::{GlobalAlloc, Layout, System};

    pub struct BenchAlloc;

    unsafe impl GlobalAlloc for BenchAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            astraweave_profiling::counters::record_alloc(layout.size());
            System.alloc(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            astraweave_profiling::counters::record_dealloc(layout.size());
            System.dealloc(ptr, layout)
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            astraweave_profiling::counters::record_alloc(layout.size());
            System.alloc_zeroed(layout)
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            astraweave_profiling::counters::record_realloc(layout.size(), new_size);
            System.realloc(ptr, layout, new_size)
        }
    }
}

#[cfg(feature = "alloc-counter")]
#[global_allocator]
static ALLOC: bench_alloc::BenchAlloc = bench_alloc::BenchAlloc;

use astraweave_render::clustered::{bin_lights_cpu, ClusterDims, CpuLight};
use criterion::{criterion_group, criterion_main, Criterion};
use glam::Vec3;
use std::hint::black_box;

/// Placeholder — raise if the first real capture exceeds this.
const MAX_ALLOCS_BIN_LIGHTS: usize = 1_000;

fn make_lights(count: usize) -> Vec<CpuLight> {
    (0..count)
        .map(|i| CpuLight {
            pos: Vec3::new(
                (i as f32 * 0.37).sin() * 30.0,
                2.0 + (i as f32 * 0.11).cos() * 5.0,
                (i as f32 * 0.52).cos() * 30.0,
            ),
            radius: 8.0 + (i as f32 * 0.23).sin() * 3.0,
        })
        .collect()
}

fn bench_bin_lights(c: &mut Criterion) {
    let dims = ClusterDims { x: 16, y: 8, z: 24 };
    let screen = (1920u32, 1080u32);
    let (near, far, fov) = (0.1_f32, 100.0_f32, std::f32::consts::FRAC_PI_3);

    let mut group = c.benchmark_group("render.bin_lights_cpu");
    for light_count in [32_usize, 128, 512] {
        let lights = make_lights(light_count);
        group.bench_function(format!("lights_{}", light_count), |b| {
            b.iter(|| {
                let r = bin_lights_cpu(&lights, dims, screen, near, far, fov);
                black_box(r);
            });
        });
    }
    group.finish();

    #[cfg(feature = "alloc-counter")]
    {
        use astraweave_profiling::FrameAllocStats;
        let lights = make_lights(128);
        let stats = FrameAllocStats::begin_frame();
        let out = bin_lights_cpu(&lights, dims, screen, near, far, fov);
        let delta = stats.end_frame();
        black_box(out);
        println!(
            "[alloc-measure] render.bin_lights_cpu (lights=128): allocs={} bytes={} reallocs={} (threshold={})",
            delta.allocs, delta.bytes_allocated, delta.reallocs, MAX_ALLOCS_BIN_LIGHTS
        );
        assert!(
            delta.allocs <= MAX_ALLOCS_BIN_LIGHTS,
            "render.bin_lights_cpu regression: {} allocs > threshold {}",
            delta.allocs,
            MAX_ALLOCS_BIN_LIGHTS
        );
    }
}

criterion_group!(benches, bench_bin_lights);
criterion_main!(benches);
