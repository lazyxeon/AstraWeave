// Week 2 Day 2: Stress Tests for astraweave-nav
// Tests large navmeshes, complex graphs, and long paths for correctness.
// Latency is measured by `benches/navmesh_benchmarks.rs` (group `stress_latency`,
// plus the existing bake benches). A hard wall-clock assert here flakes: debug
// `cargo test --lib` on the shared macOS runner measured 12.2 ms for
// `test_long_path_100_hops` and failed `< 10` (run 40879883808, job 123468260052).

use super::*;

/// Catastrophic ceiling. Not a performance SLA.
///
/// `ci.yml` sets `AW_PERF_ASSERTS=1` on the pull-request nextest partitions
/// (Linux `ci-pr-linux-archive` and Windows `ci-pr-archive`). Each ceiling is
/// about 10× the old debug bound: a hang fails, and the 12.2 ms sample that
/// failed `< 10` (run 40879883808) stays under 100 ms. Criterion records the
/// same calls in release.
fn assert_perf_ceiling(elapsed: std::time::Duration, ceiling_ms: u128, what: &str) {
    if std::env::var_os("AW_PERF_ASSERTS").is_none() {
        return;
    }
    assert!(
        elapsed.as_millis() < ceiling_ms,
        "{what} exceeded {ceiling_ms} ms catastrophic ceiling (debug build, parallel tests, shared runners; not a performance SLA), took {elapsed:?}"
    );
}

// About 10× the former debug SLAs. See `assert_perf_ceiling`.
const PERF_BAKE_100_MS: u128 = 1_000;
const PERF_PATH_100_TRI_MS: u128 = 100;
const PERF_BAKE_1K_MS: u128 = 20_000;
const PERF_PATH_1K_MS: u128 = 1_000;
const PERF_BAKE_10K_MS: u128 = 100_000;
const PERF_PATH_10K_MS: u128 = 1_000;
const PERF_PATH_50_HOP_MS: u128 = 500;
const PERF_PATH_100_HOP_MS: u128 = 100;
const PERF_QUERIES_100_MS: u128 = 10_000;

// ===== Helper Functions for Stress Test Setup =====

/// Create a grid of triangles (2 triangles per grid cell, forming quads)
/// Grid size: width × height cells = (width × height × 2) triangles
fn create_grid_navmesh(width: usize, height: usize) -> Vec<Triangle> {
    let mut tris = Vec::with_capacity(width * height * 2);

    for y in 0..height {
        for x in 0..width {
            let x0 = x as f32;
            let y0 = y as f32;
            let x1 = (x + 1) as f32;
            let y1 = (y + 1) as f32;

            // First triangle (lower-left)
            tris.push(Triangle {
                a: Vec3::new(x0, 0.0, y0),
                b: Vec3::new(x0, 0.0, y1),
                c: Vec3::new(x1, 0.0, y0),
            });

            // Second triangle (upper-right)
            tris.push(Triangle {
                a: Vec3::new(x1, 0.0, y0),
                b: Vec3::new(x0, 0.0, y1),
                c: Vec3::new(x1, 0.0, y1),
            });
        }
    }

    tris
}

/// Create a linear strip of triangles (each sharing an edge with the next)
/// Creates pairs of triangles forming squares along X axis for proper connectivity
fn create_linear_strip(count: usize) -> Vec<Triangle> {
    let pairs = count.div_ceil(2);
    let mut tris = Vec::with_capacity(pairs * 2);

    for i in 0..pairs {
        let x0 = i as f32;
        let x1 = (i + 1) as f32;

        // First triangle of the pair (lower-left)
        tris.push(Triangle {
            a: Vec3::new(x0, 0.0, 0.0),
            b: Vec3::new(x0, 0.0, 1.0),
            c: Vec3::new(x1, 0.0, 0.0),
        });

        // Second triangle of the pair (upper-right)
        tris.push(Triangle {
            a: Vec3::new(x1, 0.0, 0.0),
            b: Vec3::new(x0, 0.0, 1.0),
            c: Vec3::new(x1, 0.0, 1.0),
        });
    }

    // Trim to exact count requested
    tris.truncate(count);
    tris
}

// ===== Large Navmesh Tests =====

#[test]
fn test_large_navmesh_100_triangles_baking() {
    // 10×5 grid = 100 triangles
    let tris = create_grid_navmesh(10, 5);
    assert_eq!(tris.len(), 100);

    let start = std::time::Instant::now();
    let nav = NavMesh::bake(&tris, 0.5, 60.0);
    let elapsed = start.elapsed();

    assert_eq!(
        nav.tris.len(),
        100,
        "All triangles should be flat and included"
    );
    // `bake_100_triangles` in navmesh_benchmarks.rs.
    assert_perf_ceiling(elapsed, PERF_BAKE_100_MS, "Baking 100 triangles");
}

#[test]
fn test_large_navmesh_100_triangles_pathfinding() {
    let tris = create_grid_navmesh(10, 5);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    // Path from bottom-left to top-right corner
    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(9.5, 0.0, 4.5);

    let path_start = std::time::Instant::now();
    let path = nav.find_path(start, goal);
    let elapsed = path_start.elapsed();

    assert!(path.len() >= 2, "Path should exist across grid");
    // `pathfind_stress_100_tri_grid` in navmesh_benchmarks.rs.
    assert_perf_ceiling(elapsed, PERF_PATH_100_TRI_MS, "Pathfinding in 100-tri mesh");

    // Verify path goes from start to goal approximately
    assert!(
        (path[0] - start).length() < 1.0,
        "Path should start near start position"
    );
    assert!(
        (path.last().unwrap() - goal).length() < 1.0,
        "Path should end near goal position"
    );
}

#[test]
fn test_large_navmesh_1000_triangles_baking() {
    // 31×16 grid ≈ 992 triangles (close to 1000)
    let tris = create_grid_navmesh(31, 16);
    assert_eq!(tris.len(), 992);

    let start = std::time::Instant::now();
    let nav = NavMesh::bake(&tris, 0.5, 60.0);
    let elapsed = start.elapsed();

    assert_eq!(nav.tris.len(), 992);
    // `bake_1k_triangles` in navmesh_benchmarks.rs (1024 tris, same bake).
    assert_perf_ceiling(elapsed, PERF_BAKE_1K_MS, "Baking ~1000 triangles");

    // Verify adjacency is built (spot check first triangle has neighbors)
    assert!(
        !nav.tris[0].neighbors.is_empty(),
        "Grid triangles should have neighbors"
    );
}

#[test]
fn test_large_navmesh_1000_triangles_pathfinding() {
    let tris = create_grid_navmesh(31, 16);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    // Long diagonal path
    let start = Vec3::new(1.0, 0.0, 1.0);
    let goal = Vec3::new(30.0, 0.0, 15.0);

    let path_start = std::time::Instant::now();
    let path = nav.find_path(start, goal);
    let elapsed = path_start.elapsed();

    assert!(path.len() >= 2);
    // `pathfind_stress_1k_diagonal` in navmesh_benchmarks.rs.
    assert_perf_ceiling(elapsed, PERF_PATH_1K_MS, "Pathfinding in 1000-tri mesh");
}

#[test]
#[ignore] // Expensive test, run with --ignored flag
fn test_large_navmesh_10000_triangles_stress() {
    // 100×50 grid = 10,000 triangles
    let tris = create_grid_navmesh(100, 50);
    assert_eq!(tris.len(), 10000);

    let bake_start = std::time::Instant::now();
    let nav = NavMesh::bake(&tris, 0.5, 60.0);
    let bake_elapsed = bake_start.elapsed();

    assert_eq!(nav.tris.len(), 10000);
    // `bake_10k_triangles` in navmesh_benchmarks.rs.
    assert_perf_ceiling(bake_elapsed, PERF_BAKE_10K_MS, "Baking 10k triangles");

    // Pathfinding across the entire grid
    let start = Vec3::new(1.0, 0.0, 1.0);
    let goal = Vec3::new(99.0, 0.0, 49.0);

    let path_start = std::time::Instant::now();
    let path = nav.find_path(start, goal);
    let path_elapsed = path_start.elapsed();

    assert!(path.len() >= 2);
    // `pathfind_stress_10k_grid` in navmesh_benchmarks.rs.
    assert_perf_ceiling(path_elapsed, PERF_PATH_10K_MS, "Path across a 10k-tri grid");
}

// ===== Complex Graph Tests =====

#[test]
fn test_dense_connectivity_graph() {
    // Create 5×5 grid (50 triangles) where each triangle has many neighbors
    let tris = create_grid_navmesh(5, 5);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    // In a grid, interior triangles have 3 neighbors, edge triangles have 2, corner triangles have 1
    // Verify at least some triangles have multiple neighbors
    let multi_neighbor_count = nav.tris.iter().filter(|t| t.neighbors.len() >= 2).count();
    assert!(
        multi_neighbor_count >= 20,
        "Most triangles in grid should have 2+ neighbors, found {}",
        multi_neighbor_count
    );

    // Pathfinding should still work efficiently
    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(4.5, 0.0, 4.5);
    let path = nav.find_path(start, goal);
    assert!(path.len() >= 2);
}

#[test]
fn test_sparse_connectivity_linear_strip() {
    // Linear strip: pairs of triangles forming connected squares
    // Interior triangles have 2-3 neighbors (one within pair, 1-2 to adjacent pairs)
    let tris = create_linear_strip(20);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    assert_eq!(nav.tris.len(), 20);

    // Verify connectivity: linear strip should have triangles with 1-3 neighbors
    // (less than grid which would have 3+ neighbors for most triangles)
    let avg_neighbors: f32 =
        nav.tris.iter().map(|t| t.neighbors.len()).sum::<usize>() as f32 / nav.tris.len() as f32;
    assert!(
        (1.0..=3.0).contains(&avg_neighbors),
        "Linear strip should have 1-3 avg neighbors, found {}",
        avg_neighbors
    );

    // Path should follow the linear strip
    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(9.5, 0.0, 0.5); // Adjusted for 10 pairs = 20 triangles
    let path = nav.find_path(start, goal);
    assert!(path.len() >= 2);
}

#[test]
fn test_hierarchical_disconnected_islands() {
    // Create 3 separate island grids (disconnected from each other)
    let mut tris = Vec::new();

    // Island 1: at origin
    tris.extend(create_grid_navmesh(3, 3));

    // Island 2: offset by (10, 0, 0)
    tris.extend(create_grid_navmesh(3, 3).into_iter().map(|mut t| {
        t.a.x += 10.0;
        t.b.x += 10.0;
        t.c.x += 10.0;
        t
    }));

    // Island 3: offset by (20, 0, 0)
    tris.extend(create_grid_navmesh(3, 3).into_iter().map(|mut t| {
        t.a.x += 20.0;
        t.b.x += 20.0;
        t.c.x += 20.0;
        t
    }));

    let nav = NavMesh::bake(&tris, 0.5, 60.0);
    assert_eq!(nav.tris.len(), 54); // 3 islands × 18 triangles each

    // Path within island 1 should succeed
    let path1 = nav.find_path(Vec3::new(0.5, 0.0, 0.5), Vec3::new(2.5, 0.0, 2.5));
    assert!(path1.len() >= 2, "Path within island should exist");

    // Path between island 1 and island 2 should fail (disconnected)
    let path2 = nav.find_path(Vec3::new(0.5, 0.0, 0.5), Vec3::new(10.5, 0.0, 0.5));
    assert_eq!(
        path2.len(),
        0,
        "Path between disconnected islands should not exist"
    );
}

// ===== Long Path Tests =====

#[test]
fn test_long_path_10_hops() {
    // 10-triangle linear strip
    let tris = create_linear_strip(10);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(9.5, 0.0, 0.5);

    let path = nav.find_path(start, goal);

    // Path should traverse all 10 triangles (start + goal + intermediate centers)
    assert!(path.len() >= 2, "Path should have at least start and goal");
    assert!((path[0] - start).length() < 0.5);
    assert!((path.last().unwrap() - goal).length() < 0.5);
}

#[test]
fn test_long_path_50_hops() {
    // 50-triangle linear strip
    let tris = create_linear_strip(50);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(49.5, 0.0, 0.5);

    let path_start = std::time::Instant::now();
    let path = nav.find_path(start, goal);
    let elapsed = path_start.elapsed();

    assert!(path.len() >= 2);
    // `pathfind_stress_50_hop` in navmesh_benchmarks.rs.
    assert_perf_ceiling(elapsed, PERF_PATH_50_HOP_MS, "50-hop path");
}

#[test]
fn test_long_path_100_hops() {
    // 100-triangle linear strip
    let tris = create_linear_strip(100);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    let start = Vec3::new(0.5, 0.0, 0.5);
    let goal = Vec3::new(99.5, 0.0, 0.5);

    let path_start = std::time::Instant::now();
    let path = nav.find_path(start, goal);
    let elapsed = path_start.elapsed();

    assert!(path.len() >= 2);
    // `pathfind_stress_100_hop` in navmesh_benchmarks.rs. The old `< 10` ms
    // bound is the flake in #575.
    assert_perf_ceiling(elapsed, PERF_PATH_100_HOP_MS, "100-hop path");

    // Verify smoothing worked (path should have fewer waypoints than 100+2)
    assert!(
        path.len() < 102,
        "Smoothing should reduce waypoint count from 102 to fewer, got {}",
        path.len()
    );
}

// ===== Multi-Query Tests =====

#[test]
fn test_multiple_sequential_queries() {
    let tris = create_grid_navmesh(10, 10);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    // Run 100 pathfinding queries in sequence
    let start_time = std::time::Instant::now();
    for i in 0..100 {
        let x = (i % 10) as f32 + 0.5;
        let z = (i / 10) as f32 + 0.5;
        let start = Vec3::new(0.5, 0.0, 0.5);
        let goal = Vec3::new(x, 0.0, z);
        let path = nav.find_path(start, goal);
        assert!(
            path.len() >= 2 || (x < 1.0 && z < 1.0),
            "Query {} should find path or be same triangle",
            i
        );
    }
    let elapsed = start_time.elapsed();

    // `sequential_100_queries` in navmesh_benchmarks.rs.
    assert_perf_ceiling(elapsed, PERF_QUERIES_100_MS, "100 sequential queries");
}

#[test]
fn test_interleaved_baking_and_pathfinding() {
    // Bake a navmesh, find a path, bake another, find another path
    // Tests state consistency and no memory leaks

    let tris1 = create_grid_navmesh(5, 5);
    let nav1 = NavMesh::bake(&tris1, 0.5, 60.0);
    let path1 = nav1.find_path(Vec3::new(0.5, 0.0, 0.5), Vec3::new(4.5, 0.0, 4.5));
    assert!(path1.len() >= 2);

    let tris2 = create_linear_strip(10);
    let nav2 = NavMesh::bake(&tris2, 0.5, 60.0);
    let path2 = nav2.find_path(Vec3::new(0.5, 0.0, 0.5), Vec3::new(9.5, 0.0, 0.5));
    assert!(path2.len() >= 2);

    // nav1 should still work after nav2 is created
    let path3 = nav1.find_path(Vec3::new(1.5, 0.0, 1.5), Vec3::new(3.5, 0.0, 3.5));
    assert!(
        path3.len() >= 2,
        "Original navmesh should still work after creating new one"
    );
}

#[test]
fn test_memory_consistency_1000_queries() {
    let tris = create_grid_navmesh(10, 10);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    // Run 1000 queries to detect memory leaks or state corruption
    for i in 0..1000 {
        let x = ((i * 7) % 10) as f32 + 0.5; // Pseudo-random positions
        let z = ((i * 13) % 10) as f32 + 0.5;
        let start = Vec3::new(0.5, 0.0, 0.5);
        let goal = Vec3::new(x, 0.0, z);
        let path = nav.find_path(start, goal);

        // Every 100 queries, verify path is still valid
        if i % 100 == 0 {
            assert!(
                path.len() >= 2 || (x < 1.0 && z < 1.0),
                "Query {} should remain consistent",
                i
            );
        }
    }
}

// ===== Edge Case: Zero-Length Path =====

#[test]
fn test_zero_length_path_same_position() {
    let tris = create_grid_navmesh(5, 5);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    // Start and goal are identical
    let pos = Vec3::new(2.5, 0.0, 2.5);
    let path = nav.find_path(pos, pos);

    // Should return a path with start==goal (minimal path)
    assert!(
        path.len() >= 2,
        "Zero-length path should still return start+goal"
    );
    assert!((path[0] - pos).length() < 0.1);
    assert!((path.last().unwrap() - pos).length() < 0.1);
}

// ===== Edge Case: Very Close Start and Goal =====

#[test]
fn test_very_close_start_and_goal() {
    let tris = create_grid_navmesh(10, 10);
    let nav = NavMesh::bake(&tris, 0.5, 60.0);

    // Start and goal 0.1 units apart (both in same triangle)
    let start = Vec3::new(5.0, 0.0, 5.0);
    let goal = Vec3::new(5.1, 0.0, 5.0);

    let path = nav.find_path(start, goal);
    assert!(path.len() >= 2);
    assert!((path[0] - start).length() < 0.1);
    assert!((path.last().unwrap() - goal).length() < 0.1);
}

#[test]
fn test_pathfinding_with_max_step_validation() {
    // Verify max_step parameter is preserved (not used in pathfinding, but should be set)
    let tris = create_grid_navmesh(5, 5);
    let nav = NavMesh::bake(&tris, 0.8, 60.0);

    assert_eq!(
        nav.max_step, 0.8,
        "max_step should be preserved during baking"
    );

    // Path should still work normally (max_step doesn't affect pathfinding in this implementation)
    let path = nav.find_path(Vec3::new(0.5, 0.0, 0.5), Vec3::new(4.5, 0.0, 4.5));
    assert!(path.len() >= 2);
}
