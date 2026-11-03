//! Week 2 Day 4: Performance Benchmarks for astraweave-nav
//!
//! Comprehensive benchmark suite measuring:
//! 1. Baking performance (100, 1k, 10k triangles)
//! 2. Pathfinding performance (short, medium, long paths)
//! 3. Throughput (queries/second at different scales)
//! 4. Memory characteristics
//! 5. Stress-test latency (`stress_latency`): the `find_path` calls whose
//!    debug wall-clock asserts moved out of `src/stress_tests.rs` (#575).
//!    Bake timings for those tests stay on `bake_100_triangles`,
//!    `bake_1k_triangles`, and `bake_10k_triangles`.
//!
//! Target: Establish production performance baselines
//! Duration: 0.5 hours

// =============================================================================
// MISSION-CRITICAL CORRECTNESS ASSERTIONS
// =============================================================================
// Navigation benchmarks validate CORRECTNESS of pathfinding systems.
// Assertions verify:
//   1. Triangle Validity: All triangles have 3 distinct, finite vertices
//   2. NavMesh Construction: Baked mesh has valid polygon count
//   3. Path Validity: Found paths have start near request, end near goal
//   4. Path Connectivity: Path waypoints are connected (no teleportation)
//   5. Path Length: Path distance is at least straight-line distance
// =============================================================================

use astraweave_nav::{
    bridge_and_ford, nav_change_from_kind, NavChangeKind, NavMesh, Triangle, TILE_COMMITS_PER_TICK,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use glam::Vec3;

/// CORRECTNESS: Validate triangle has finite, distinct vertices
#[inline]
fn assert_triangle_valid(tri: &Triangle, context: &str) {
    // All vertices must be finite
    assert!(
        tri.a.x.is_finite() && tri.a.y.is_finite() && tri.a.z.is_finite(),
        "[CORRECTNESS FAILURE] {}: triangle vertex A non-finite {:?}",
        context,
        tri.a
    );
    assert!(
        tri.b.x.is_finite() && tri.b.y.is_finite() && tri.b.z.is_finite(),
        "[CORRECTNESS FAILURE] {}: triangle vertex B non-finite {:?}",
        context,
        tri.b
    );
    assert!(
        tri.c.x.is_finite() && tri.c.y.is_finite() && tri.c.z.is_finite(),
        "[CORRECTNESS FAILURE] {}: triangle vertex C non-finite {:?}",
        context,
        tri.c
    );
    // Vertices should not be coincident (degenerate triangle)
    let ab = (tri.b - tri.a).length();
    let bc = (tri.c - tri.b).length();
    let ca = (tri.a - tri.c).length();
    assert!(
        ab > 0.0001 && bc > 0.0001 && ca > 0.0001,
        "[CORRECTNESS FAILURE] {}: degenerate triangle (edges: {}, {}, {})",
        context,
        ab,
        bc,
        ca
    );
}

/// CORRECTNESS: Validate path is geometrically valid
/// Note: find_path returns empty Vec when no path found
#[inline]
fn assert_path_valid(path: &[Vec3], start: Vec3, goal: Vec3, context: &str) {
    if !path.is_empty() {
        // Path should have at least 2 points (start and end)
        assert!(
            path.len() >= 2,
            "[CORRECTNESS FAILURE] {}: path has < 2 waypoints ({})",
            context,
            path.len()
        );
        // First waypoint should be near start
        let start_dist = (path[0] - start).length();
        assert!(
            start_dist < 5.0,
            "[CORRECTNESS FAILURE] {}: path start too far from request ({} units)",
            context,
            start_dist
        );
        // Last waypoint should be near goal
        let goal_dist = (path[path.len() - 1] - goal).length();
        assert!(
            goal_dist < 5.0,
            "[CORRECTNESS FAILURE] {}: path end too far from goal ({} units)",
            context,
            goal_dist
        );
        // All waypoints should be finite
        for (i, wp) in path.iter().enumerate() {
            assert!(
                wp.x.is_finite() && wp.y.is_finite() && wp.z.is_finite(),
                "[CORRECTNESS FAILURE] {}: waypoint {} non-finite {:?}",
                context,
                i,
                wp
            );
        }
    }
    // Note: empty path is valid (no path found on disconnected regions)
}

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
            let tri1 = Triangle {
                a: Vec3::new(x0, 0.0, z0),
                b: Vec3::new(x0, 0.0, z1),
                c: Vec3::new(x1, 0.0, z0),
            };
            // CORRECTNESS: Validate triangle on creation
            assert_triangle_valid(&tri1, "create_grid_navmesh/tri1");
            tris.push(tri1);

            // Triangle 2: top-right half of quad
            let tri2 = Triangle {
                a: Vec3::new(x1, 0.0, z0),
                b: Vec3::new(x0, 0.0, z1),
                c: Vec3::new(x1, 0.0, z1),
            };
            assert_triangle_valid(&tri2, "create_grid_navmesh/tri2");
            tris.push(tri2);
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
        let tri1 = Triangle {
            a: Vec3::new(x, 0.0, 0.0),
            b: Vec3::new(x, 0.0, 1.0),
            c: Vec3::new(x + 2.0, 0.0, 0.0),
        };
        assert_triangle_valid(&tri1, "create_linear_strip/tri1");
        tris.push(tri1);

        let tri2 = Triangle {
            a: Vec3::new(x + 2.0, 0.0, 0.0),
            b: Vec3::new(x, 0.0, 1.0),
            c: Vec3::new(x + 2.0, 0.0, 1.0),
        };
        assert_triangle_valid(&tri2, "create_linear_strip/tri2");
        tris.push(tri2);
    }
    tris
}

/// Unit-square strip from `stress_tests::create_linear_strip`.
///
/// `count` is the triangle count. `create_linear_strip` above uses 2-unit
/// quads and does not match the stress tests.
fn create_unit_strip(count: usize) -> Vec<Triangle> {
    let pairs = count.div_ceil(2);
    let mut tris = Vec::with_capacity(pairs * 2);
    for i in 0..pairs {
        let x0 = i as f32;
        let x1 = (i + 1) as f32;
        let tri1 = Triangle {
            a: Vec3::new(x0, 0.0, 0.0),
            b: Vec3::new(x0, 0.0, 1.0),
            c: Vec3::new(x1, 0.0, 0.0),
        };
        assert_triangle_valid(&tri1, "create_unit_strip/tri1");
        tris.push(tri1);

        let tri2 = Triangle {
            a: Vec3::new(x1, 0.0, 0.0),
            b: Vec3::new(x0, 0.0, 1.0),
            c: Vec3::new(x1, 0.0, 1.0),
        };
        assert_triangle_valid(&tri2, "create_unit_strip/tri2");
        tris.push(tri2);
    }
    tris.truncate(count);
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
            // CORRECTNESS: Baked mesh should have triangles
            assert!(
                !nm.tris.is_empty(),
                "[CORRECTNESS FAILURE] bake_100_triangles: NavMesh has 0 triangles"
            );
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
            assert!(
                !nm.tris.is_empty(),
                "[CORRECTNESS FAILURE] bake_1k_triangles: NavMesh has 0 triangles"
            );
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
            assert!(
                !nm.tris.is_empty(),
                "[CORRECTNESS FAILURE] bake_10k_triangles: NavMesh has 0 triangles"
            );
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
            // CORRECTNESS: Validate path if found
            assert_path_valid(&path, start, goal, "pathfind_short");
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
            assert_path_valid(&path, start, goal, "pathfind_medium");
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
            // CORRECTNESS: Validate long path
            assert_path_valid(&path, start, goal, "pathfind_long");
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
    // CORRECTNESS: Validate baked mesh
    assert!(
        !nm.tris.is_empty(),
        "[CORRECTNESS FAILURE] throughput_100: NavMesh has 0 triangles"
    );

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(5.5, 0.0, 2.5);

    let mut group = c.benchmark_group("throughput_100_triangles");
    group.throughput(Throughput::Elements(1)); // 1 query per iteration

    group.bench_function("queries_per_second", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            // CORRECTNESS: Validate path in throughput test
            assert_path_valid(&path, start, goal, "throughput_100");
            black_box(path)
        })
    });

    group.finish();
}

fn bench_throughput_1k_triangles(c: &mut Criterion) {
    let tris = create_grid_navmesh(32, 16); // 1024 triangles
    let nm = NavMesh::bake(&tris, 0.5, 60.0);
    assert!(
        !nm.tris.is_empty(),
        "[CORRECTNESS FAILURE] throughput_1k: NavMesh has 0 triangles"
    );

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(16.5, 0.0, 8.5);

    let mut group = c.benchmark_group("throughput_1k_triangles");
    group.throughput(Throughput::Elements(1));

    group.bench_function("queries_per_second", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            assert_path_valid(&path, start, goal, "throughput_1k");
            black_box(path)
        })
    });

    group.finish();
}

fn bench_throughput_10k_triangles(c: &mut Criterion) {
    let tris = create_grid_navmesh(100, 50); // 10000 triangles
    let nm = NavMesh::bake(&tris, 0.5, 60.0);
    assert!(
        !nm.tris.is_empty(),
        "[CORRECTNESS FAILURE] throughput_10k: NavMesh has 0 triangles"
    );

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(50.5, 0.0, 25.5);

    let mut group = c.benchmark_group("throughput_10k_triangles");
    group.throughput(Throughput::Elements(1));

    group.bench_function("queries_per_second", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            assert_path_valid(&path, start, goal, "throughput_10k");
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

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, sz| {
            b.iter(|| {
                let nm = NavMesh::bake(black_box(&tris), black_box(0.5), black_box(60.0));
                // CORRECTNESS: Validate baked mesh at all scales
                assert!(
                    !nm.tris.is_empty(),
                    "[CORRECTNESS FAILURE] baking_scaling_{}: NavMesh has 0 triangles",
                    sz
                );
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
        // CORRECTNESS: Validate mesh before pathfinding
        assert!(
            !nm.tris.is_empty(),
            "[CORRECTNESS FAILURE] pathfinding_scaling_{}: NavMesh has 0 triangles",
            size
        );

        let start = Vec3::new(0.5, 0.0, 0.5);
        let goal = Vec3::new((*size as f32) - 0.5, 0.0, (*size as f32) - 0.5);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, sz| {
            b.iter(|| {
                let path = nm.find_path(black_box(start), black_box(goal));
                // CORRECTNESS: Validate paths at all scales
                assert_path_valid(&path, start, goal, &format!("pathfinding_scaling_{}", sz));
                black_box(path)
            })
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark 5: Stress-test latency (#575)
// ============================================================================
// Same calls `src/stress_tests.rs` used to bound with `Instant` on the debug
// `cargo test` path. Correctness stays in those tests. `AW_PERF_ASSERTS` is
// only a catastrophic ceiling, not these measurements.

fn bench_pathfind_stress_100_tri_grid(c: &mut Criterion) {
    let tris = create_grid_navmesh(10, 5);
    assert_eq!(tris.len(), 100, "Expected 100 triangles");
    let nm = NavMesh::bake(&tris, 0.5, 60.0);
    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(9.5, 0.0, 4.5);

    c.bench_function("pathfind_stress_100_tri_grid", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            assert!(
                path.len() >= 2,
                "[CORRECTNESS FAILURE] pathfind_stress_100_tri_grid: empty path"
            );
            assert_path_valid(&path, start, goal, "pathfind_stress_100_tri_grid");
            black_box(path)
        })
    });
}

fn bench_pathfind_stress_1k_diagonal(c: &mut Criterion) {
    let tris = create_grid_navmesh(31, 16);
    assert_eq!(tris.len(), 992, "Expected 992 triangles");
    let nm = NavMesh::bake(&tris, 0.5, 60.0);
    let start = Vec3::new(1.0, 0.0, 1.0);
    let goal = Vec3::new(30.0, 0.0, 15.0);

    c.bench_function("pathfind_stress_1k_diagonal", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            assert!(
                path.len() >= 2,
                "[CORRECTNESS FAILURE] pathfind_stress_1k_diagonal: empty path"
            );
            assert_path_valid(&path, start, goal, "pathfind_stress_1k_diagonal");
            black_box(path)
        })
    });
}

fn bench_pathfind_stress_50_hop(c: &mut Criterion) {
    let tris = create_unit_strip(50);
    assert_eq!(tris.len(), 50, "Expected 50 triangles");
    let nm = NavMesh::bake(&tris, 0.5, 60.0);
    // Goal matches `test_long_path_50_hops`, including the off-mesh x.
    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(49.5, 0.0, 0.5);

    c.bench_function("pathfind_stress_50_hop", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            assert!(
                path.len() >= 2,
                "[CORRECTNESS FAILURE] pathfind_stress_50_hop: empty path"
            );
            assert_path_valid(&path, start, goal, "pathfind_stress_50_hop");
            black_box(path)
        })
    });
}

fn bench_pathfind_stress_100_hop(c: &mut Criterion) {
    let tris = create_unit_strip(100);
    assert_eq!(tris.len(), 100, "Expected 100 triangles");
    let nm = NavMesh::bake(&tris, 0.5, 60.0);
    // Goal matches `test_long_path_100_hops`, including the off-mesh x.
    // That test's `< 10` ms assert is the #575 flake.
    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(99.5, 0.0, 0.5);

    c.bench_function("pathfind_stress_100_hop", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            assert!(
                path.len() >= 2,
                "[CORRECTNESS FAILURE] pathfind_stress_100_hop: empty path"
            );
            assert!(
                path.len() < 102,
                "[CORRECTNESS FAILURE] pathfind_stress_100_hop: smoothing left {} waypoints",
                path.len()
            );
            assert_path_valid(&path, start, goal, "pathfind_stress_100_hop");
            black_box(path)
        })
    });
}

fn bench_pathfind_stress_10k_grid(c: &mut Criterion) {
    let tris = create_grid_navmesh(100, 50);
    assert_eq!(tris.len(), 10000, "Expected 10000 triangles");
    let nm = NavMesh::bake(&tris, 0.5, 60.0);
    let start = Vec3::new(1.0, 0.0, 1.0);
    let goal = Vec3::new(99.0, 0.0, 49.0);

    c.bench_function("pathfind_stress_10k_grid", |b| {
        b.iter(|| {
            let path = nm.find_path(black_box(start), black_box(goal));
            assert!(
                path.len() >= 2,
                "[CORRECTNESS FAILURE] pathfind_stress_10k_grid: empty path"
            );
            assert_path_valid(&path, start, goal, "pathfind_stress_10k_grid");
            black_box(path)
        })
    });
}

fn bench_sequential_100_queries(c: &mut Criterion) {
    let tris = create_grid_navmesh(10, 10);
    let nm = NavMesh::bake(&tris, 0.5, 60.0);

    c.bench_function("sequential_100_queries", |b| {
        b.iter(|| {
            for i in 0..100 {
                let x = (i % 10) as f32 + 0.5;
                let z = (i / 10) as f32 + 0.5;
                let start = Vec3::new(0.5, 0.0, 0.5);
                let goal = Vec3::new(x, 0.0, z);
                let path = nm.find_path(black_box(start), black_box(goal));
                assert!(
                    path.len() >= 2 || (x < 1.0 && z < 1.0),
                    "[CORRECTNESS FAILURE] sequential_100_queries: query {i}"
                );
                black_box(path);
            }
        })
    });
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

criterion_group!(
    stress_latency,
    bench_pathfind_stress_100_tri_grid,
    bench_pathfind_stress_1k_diagonal,
    bench_pathfind_stress_50_hop,
    bench_pathfind_stress_100_hop,
    bench_pathfind_stress_10k_grid,
    bench_sequential_100_queries,
);

fn bench_tile_commits_per_tick(c: &mut Criterion) {
    c.bench_function("tile_commits_per_tick", |b| {
        b.iter(|| {
            let mut tiles = bridge_and_ford();
            for seq in 0..TILE_COMMITS_PER_TICK as u32 {
                tiles.mark(nav_change_from_kind(
                    NavChangeKind::Brush,
                    1,
                    seq,
                    [seq as i32 * 32_000, -1_000, 0],
                    [seq as i32 * 32_000 + 500, 1_000, 500],
                ));
            }
            black_box(tiles.commit_tick())
        });
    });
}

criterion_group!(tile_budget, bench_tile_commits_per_tick);

criterion_main!(baking, pathfinding, throughput, stress_latency, tile_budget);
