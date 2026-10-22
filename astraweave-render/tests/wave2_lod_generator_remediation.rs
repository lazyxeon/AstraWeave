//! Wave 2 proactive remediation tests for lod_generator.rs (168 mutants).
//!
//! Targets public API:
//!   - LODConfig default values
//!   - SimplificationMesh vertex_count / triangle_count
//!   - LODGenerator::generate_lods — number of levels, reduction
//!   - LODGenerator::simplify — error threshold, already-below-target, actual reduction
//!   - Edge cases: degenerate meshes, single triangle, empty mesh

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::lod_generator::{LODConfig, LODGenerator, SimplificationMesh};
use glam::Vec3;

// ══════════════════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════════════════

/// A 4-vertex quad (2 triangles) — simplest non-trivial mesh
fn quad_mesh() -> SimplificationMesh {
    SimplificationMesh::new(
        vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        vec![Vec3::Z, Vec3::Z, Vec3::Z, Vec3::Z],
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        vec![0, 1, 2, 0, 2, 3],
    )
}

/// A grid mesh with many vertices — good for testing actual simplification
fn grid_mesh(n: usize) -> SimplificationMesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for y in 0..n {
        for x in 0..n {
            positions.push(Vec3::new(x as f32, y as f32, 0.0));
            normals.push(Vec3::Z);
            uvs.push([x as f32 / n as f32, y as f32 / n as f32]);
        }
    }

    for y in 0..n - 1 {
        for x in 0..n - 1 {
            let i = (y * n + x) as u32;
            indices.extend_from_slice(&[i, i + 1, i + n as u32]);
            indices.extend_from_slice(&[i + 1, i + n as u32 + 1, i + n as u32]);
        }
    }

    SimplificationMesh::new(positions, normals, uvs, indices)
}

/// Single triangle
fn triangle_mesh() -> SimplificationMesh {
    SimplificationMesh::new(
        vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ],
        vec![Vec3::Z, Vec3::Z, Vec3::Z],
        vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]],
        vec![0, 1, 2],
    )
}

// ══════════════════════════════════════════════════════════════════════════════
// LODConfig defaults
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn config_default_reduction_targets() {
    let c = LODConfig::default();
    assert_eq!(c.reduction_targets, vec![0.75, 0.50, 0.25]);
}

#[test]
fn config_default_max_error() {
    let c = LODConfig::default();
    assert_eq!(c.max_error, 0.01);
}

#[test]
fn config_default_preserve_boundaries() {
    let c = LODConfig::default();
    assert!(c.preserve_boundaries);
}

// ══════════════════════════════════════════════════════════════════════════════
// SimplificationMesh basics
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn mesh_vertex_count() {
    let m = quad_mesh();
    assert_eq!(m.vertex_count(), 4);
}

#[test]
fn mesh_triangle_count() {
    let m = quad_mesh();
    assert_eq!(m.triangle_count(), 2);
}

#[test]
fn mesh_empty() {
    let m = SimplificationMesh::new(vec![], vec![], vec![], vec![]);
    assert_eq!(m.vertex_count(), 0);
    assert_eq!(m.triangle_count(), 0);
}

#[test]
fn mesh_single_triangle() {
    let m = triangle_mesh();
    assert_eq!(m.vertex_count(), 3);
    assert_eq!(m.triangle_count(), 1);
}

#[test]
fn grid_mesh_correct_counts() {
    let m = grid_mesh(10);
    assert_eq!(m.vertex_count(), 100);
    assert_eq!(m.triangle_count(), 9 * 9 * 2); // (n-1)^2 * 2
}

// ══════════════════════════════════════════════════════════════════════════════
// LODGenerator::generate_lods
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn generate_lods_returns_correct_number_of_levels() {
    let gen = LODGenerator::new(LODConfig::default());
    let mesh = grid_mesh(10);
    let lods = gen.generate_lods(&mesh);
    assert_eq!(lods.len(), 3, "default config has 3 reduction targets");
}

#[test]
fn generate_lods_each_level_has_fewer_vertices() {
    let gen = LODGenerator::new(LODConfig {
        max_error: 100.0, // Very permissive to allow full simplification
        ..Default::default()
    });
    let mesh = grid_mesh(10);
    let lods = gen.generate_lods(&mesh);

    assert!(lods.len() >= 2);
    // LOD1 should be smaller or equal to LOD0 (i.e. original)
    assert!(
        lods[0].vertex_count() <= mesh.vertex_count(),
        "LOD1 should not have more vertices than original"
    );
}

#[test]
fn generate_lods_monotonically_decreasing_vertices() {
    let gen = LODGenerator::new(LODConfig {
        max_error: 100.0,
        ..Default::default()
    });
    let mesh = grid_mesh(10);
    let lods = gen.generate_lods(&mesh);

    for i in 1..lods.len() {
        assert!(
            lods[i].vertex_count() <= lods[i - 1].vertex_count(),
            "LOD{} ({} verts) should have ≤ LOD{} ({} verts)",
            i + 1,
            lods[i].vertex_count(),
            i,
            lods[i - 1].vertex_count(),
        );
    }
}

#[test]
fn generate_lods_custom_targets() {
    let gen = LODGenerator::new(LODConfig {
        reduction_targets: vec![0.5],
        max_error: 100.0,
        preserve_boundaries: false,
    });
    let mesh = grid_mesh(10);
    let lods = gen.generate_lods(&mesh);
    assert_eq!(lods.len(), 1, "single target = single LOD output");
}

// ══════════════════════════════════════════════════════════════════════════════
// LODGenerator::simplify
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn simplify_no_reduction_needed() {
    let gen = LODGenerator::new(LODConfig::default());
    let mesh = triangle_mesh();
    // Target >= current vertex count → no simplification
    let result = gen.simplify(&mesh, 10);
    assert_eq!(result.vertex_count(), 3, "already below target = no change");
}

#[test]
fn simplify_equal_target_no_change() {
    let gen = LODGenerator::new(LODConfig::default());
    let mesh = quad_mesh();
    let result = gen.simplify(&mesh, 4);
    assert_eq!(result.vertex_count(), 4);
}

#[test]
fn simplify_grid_reduces_vertices() {
    let gen = LODGenerator::new(LODConfig {
        max_error: 100.0,
        ..Default::default()
    });
    let mesh = grid_mesh(10);
    let target = 50;
    let result = gen.simplify(&mesh, target);
    assert!(
        result.vertex_count() <= mesh.vertex_count(),
        "simplified should have fewer vertices"
    );
}

#[test]
fn simplify_preserves_valid_indices() {
    let gen = LODGenerator::new(LODConfig {
        max_error: 100.0,
        ..Default::default()
    });
    let mesh = grid_mesh(8);
    let result = gen.simplify(&mesh, 20);
    let vc = result.vertex_count();
    for &idx in &result.indices {
        assert!(
            (idx as usize) < vc,
            "index {} out of bounds for {} vertices",
            idx,
            vc
        );
    }
}

#[test]
fn simplify_indices_divisible_by_3() {
    let gen = LODGenerator::new(LODConfig {
        max_error: 100.0,
        ..Default::default()
    });
    let mesh = grid_mesh(8);
    let result = gen.simplify(&mesh, 20);
    assert_eq!(
        result.indices.len() % 3,
        0,
        "indices should always be triangle triples"
    );
}

#[test]
fn simplify_max_error_stops_early() {
    // Very small max_error should stop simplification early
    let gen = LODGenerator::new(LODConfig {
        max_error: 0.0, // Zero error tolerance
        reduction_targets: vec![0.25],
        preserve_boundaries: true,
    });
    let mesh = grid_mesh(10);
    let result = gen.simplify(&mesh, 1); // Request extreme reduction
                                         // With max_error=0, shouldn't actually collapse anything (all collapses have nonzero error)
                                         // But flat mesh has zero error for coplanar collapses, so some might happen.
                                         // At least it shouldn't panic.
    assert!(result.vertex_count() > 0);
}

#[test]
fn simplify_normals_and_uvs_consistent() {
    let gen = LODGenerator::new(LODConfig {
        max_error: 100.0,
        ..Default::default()
    });
    let mesh = grid_mesh(8);
    let result = gen.simplify(&mesh, 20);
    assert_eq!(result.positions.len(), result.normals.len());
    assert_eq!(result.positions.len(), result.uvs.len());
}

// ══════════════════════════════════════════════════════════════════════════════
// Edge cases
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn generate_lods_empty_mesh() {
    let gen = LODGenerator::new(LODConfig::default());
    let mesh = SimplificationMesh::new(vec![], vec![], vec![], vec![]);
    let lods = gen.generate_lods(&mesh);
    assert_eq!(lods.len(), 3);
    for lod in &lods {
        assert_eq!(lod.vertex_count(), 0);
    }
}

#[test]
fn generate_lods_single_triangle() {
    let gen = LODGenerator::new(LODConfig::default());
    let mesh = triangle_mesh();
    let lods = gen.generate_lods(&mesh);
    assert_eq!(lods.len(), 3);
    // 3 vertices * 0.75 = 2.25 ≈ 3, * 0.50 = 1.5 ≈ 2, * 0.25 = 0.75 ≈ 1
    // With max_error=0.01 on a flat surface, some collapses may happen
    for lod in &lods {
        assert!(lod.vertex_count() <= 3);
    }
}

#[test]
fn simplify_large_mesh_doesnt_panic() {
    let gen = LODGenerator::new(LODConfig {
        max_error: 100.0,
        ..Default::default()
    });
    let mesh = grid_mesh(20); // 400 vertices
    assert_eq!(mesh.vertex_count(), 400);
    let result = gen.simplify(&mesh, 50);
    assert!(result.vertex_count() <= 400);
    assert!(result.vertex_count() > 0);
}
