//! Week 2 Day 4: Performance Benchmarks for astraweave-nav
//!
//! Comprehensive benchmark suite measuring:
//! 1. Baking performance (100, 1k, 10k triangles)
//! 2. Pathfinding performance (short, medium, long paths)
//! 3. Throughput (queries/second at different scales)
//! 4. Memory characteristics
//!
//! Target: Establish production performance baselines
//! Duration: 0.5 hours

use astraweave_nav::{NavMesh, Triangle};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use glam::Vec3;
use std::hint::black_box;

// ============================================================================
// Helper Functions (reusing existing test patterns)
// ============================================================================

/// Create grid-based navmesh (flat horizontal triangles, correct winding)
fn create_grid_navmesh(width: usize, depth: usize) -> Vec<Triangle> {
    let mut tris = Vec::new();
    for z in 0..depth {
        for x in 0..width {
            let x0 = x as f32;
            let z0 = z as f32;
            let x1 = (x + 1) as f32;
            let z1 = (z + 1) as f32;

            // Counter-clockwise winding from +Y view (upward normals)
            // Triangle 1: bottom-left half of quad
            tris.push(Triangle {
                a: Vec3::new(x0, 0.0, z0),
                b: Vec3::new(x0, 0.0, z1),
                c: Vec3::new(x1, 0.0, z0),
            });

            // Triangle 2: top-right half of quad
            tris.push(Triangle {
                a: Vec3::new(x1, 0.0, z0),
                b: Vec3::new(x0, 0.0, z1),
                c: Vec3::new(x1, 0.0, z1),
            });
        }
    }
    tris
}

/// Create linear strip navmesh (for long paths)
fn create_linear_strip(length: usize) -> Vec<Triangle> {
    let mut tris = Vec::new();
    for i in 0..length {
        let x = i as f32 * 2.0;
        // Counter-clockwise winding
        tris.push(Triangle {
            a: Vec3::new(x, 0.0, 0.0),
            b: Vec3::new(x, 0.0, 1.0),
            c: Vec3::new(x + 2.0, 0.0, 0.0),
        });
        tris.push(Triangle {
            a: Vec3::new(x + 2.0, 0.0, 0.0),
            b: Vec3::new(x, 0.0, 1.0),
            c: Vec3::new(x + 2.0, 0.0, 1.0),
        });
    }
    tris
}

// ============================================================================
// Benchmark 1: Baking Performance
// ============================================================================

fn bench_baking_100_triangles(c: &mut Criterion) {
    let tris = create_grid_navmesh(10, 5); // 10*5 = 50 quads = 100 triangles
    assert_eq!(tris.len(), 100, "Expected 100 triangles");

    c.bench_function("bake_100_triangles", |b| {
        b.iter(|| {
            let nm = NavMesh::bake(black_box(&tris), black_box(0.5), black_box(60.0));
            black_box(nm)
        })
    });
}

fn bench_baking_1k_triangles(c: &mut Criterion) {
    let tris = create_grid_navmesh(32, 16); // 32*16 = 512 quads = 1024 triangles
    assert_eq!(tris.len(), 1024, "Expected 1024 triangles");

    c.bench_function("bake_1k_triangles", |b| {
        b.iter(|| {
            let nm = NavMesh::bake(black_box(&tris), black_box(0.5), black_box(60.0));
            black_box(nm)
        })
    });
}

fn bench_baking_10k_triangles(c: &mut Criterion) {
    let tris = create_grid_navmesh(100, 50); // 100*50 = 5000 quads = 10000 triangles
    assert_eq!(tris.len(), 10000, "Expected 10000 triangles");

    c.bench_function("bake_10k_triangles", |b| {
        b.iter(|| {
            let nm = NavMesh::bake(black_box(&tris), black_box(0.5), black_box(60.0));
            black_box(nm)
        })
    });
}

// ============================================================================
// Benchmark 2: Pathfinding Performance
// ============================================================================

fn bench_pathfinding_short_path(c: &mut Criterion) {
    let tris = create_grid_navmesh(10, 10); // 10*10 = 100 quads = 200 triangles
    let nm = NavMesh::bake(&tris, 0.5, 60.0);

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(2.5, 0.0, 0.5); // 2-3 triangles away

    c.bench_function("pathfind_short_2_5_hops", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            black_box(path)
        })
    });
}

fn bench_pathfinding_medium_path(c: &mut Criterion) {
    let tris = create_grid_navmesh(20, 20); // 20*20 = 400 quads = 800 triangles
    let nm = NavMesh::bake(&tris, 0.5, 60.0);

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(10.5, 0.0, 10.5); // ~10-20 triangles away

    c.bench_function("pathfind_medium_10_20_hops", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            black_box(path)
        })
    });
}

fn bench_pathfinding_long_path(c: &mut Criterion) {
    let tris = create_linear_strip(50); // 50 quads = 100 triangles in line
    let nm = NavMesh::bake(&tris, 0.5, 60.0);

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(98.0, 0.0, 0.5); // ~50-100 triangles away

    c.bench_function("pathfind_long_50_100_hops", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            black_box(path)
        })
    });
}

// ============================================================================
// Benchmark 3: Throughput
// ============================================================================

fn bench_throughput_100_triangles(c: &mut Criterion) {
    let tris = create_grid_navmesh(10, 5); // 100 triangles
    let nm = NavMesh::bake(&tris, 0.5, 60.0);

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(5.5, 0.0, 2.5);

    let mut group = c.benchmark_group("throughput_100_triangles");
    group.throughput(Throughput::Elements(1)); // 1 query per iteration

    group.bench_function("queries_per_second", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            black_box(path)
        })
    });

    group.finish();
}

fn bench_throughput_1k_triangles(c: &mut Criterion) {
    let tris = create_grid_navmesh(32, 16); // 1024 triangles
    let nm = NavMesh::bake(&tris, 0.5, 60.0);

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(16.5, 0.0, 8.5);

    let mut group = c.benchmark_group("throughput_1k_triangles");
    group.throughput(Throughput::Elements(1));

    group.bench_function("queries_per_second", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            black_box(path)
        })
    });

    group.finish();
}

fn bench_throughput_10k_triangles(c: &mut Criterion) {
    let tris = create_grid_navmesh(100, 50); // 10000 triangles
    let nm = NavMesh::bake(&tris, 0.5, 60.0);

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(50.5, 0.0, 25.5);

    let mut group = c.benchmark_group("throughput_10k_triangles");
    group.throughput(Throughput::Elements(1));

    group.bench_function("queries_per_second", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            black_box(path)
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark 4: Parameterized Scaling
// ============================================================================

fn bench_baking_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("baking_scaling");

    for size in [100, 500, 1000, 2000, 5000, 10000].iter() {
        let side = (*size as f32 / 2.0).sqrt() as usize; // Approximate square grid
        let tris = create_grid_navmesh(side, side);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let nm = NavMesh::bake(black_box(&tris), black_box(0.5), black_box(60.0));
                black_box(nm)
            })
        });
    }

    group.finish();
}

fn bench_pathfinding_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("pathfinding_scaling");

    for size in [10, 20, 50, 100].iter() {
        let tris = create_grid_navmesh(*size, *size);
        let nm = NavMesh::bake(&tris, 0.5, 60.0);

        let start = Vec3::new(0.5, 0.0, 0.5);
        let goal = Vec3::new((*size as f32) - 0.5, 0.0, (*size as f32) - 0.5);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let path = nm.find_path(black_box(start), black_box(goal));
                black_box(path)
            })
        });
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    baking,
    bench_baking_100_triangles,
    bench_baking_1k_triangles,
    bench_baking_10k_triangles,
    bench_baking_scaling,
);

criterion_group!(
    pathfinding,
    bench_pathfinding_short_path,
    bench_pathfinding_medium_path,
    bench_pathfinding_long_path,
    bench_pathfinding_scaling,
);

criterion_group!(
    throughput,
    bench_throughput_100_triangles,
    bench_throughput_1k_triangles,
    bench_throughput_10k_triangles,
);

criterion_main!(baking, pathfinding, throughput);
