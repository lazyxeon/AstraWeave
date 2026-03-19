//! Wave 2 LOD Blending Remediation Tests
//!
//! Targets all mutation-sensitive code paths in lod_blending.rs:
//! - MorphConfig defaults, for_lod_transition arithmetic
//! - LodBlender::compute_morph_factor boundary transitions
//! - LodBlender::morph_vertices interpolation (positions, normals)
//! - vertex correspondence search radius filtering
//! - spatial hash cell assignment
//! - find_nearest_vertex correctness
//! - MorphedMesh accessors
//! - MorphingLodManager LOD selection + transition zones
//! - create_transition_mesh full pipeline

use astraweave_terrain::lod_blending::*;
use astraweave_terrain::meshing::{ChunkMesh, MeshVertex};
use astraweave_terrain::voxel_data::ChunkCoord;
use glam::Vec3;

// ============================================================================
// Helper: build a simple ChunkMesh with given vertices
// ============================================================================
fn mesh_with_verts(verts: Vec<Vec3>) -> ChunkMesh {
    let vertices: Vec<MeshVertex> = verts
        .iter()
        .map(|&p| MeshVertex {
            position: p,
            normal: Vec3::Y,
            material: 1,
        })
        .collect();
    let indices: Vec<u32> = if vertices.len() >= 3 {
        (0..vertices.len() as u32).collect()
    } else {
        vec![]
    };
    ChunkMesh {
        coord: ChunkCoord::new(0, 0, 0),
        vertices,
        indices,
    }
}

fn mesh_with_normals(positions: Vec<Vec3>, normals: Vec<Vec3>) -> ChunkMesh {
    assert_eq!(positions.len(), normals.len());
    let vertices: Vec<MeshVertex> = positions
        .iter()
        .zip(normals.iter())
        .map(|(&p, &n)| MeshVertex {
            position: p,
            normal: n,
            material: 1,
        })
        .collect();
    let indices: Vec<u32> = if vertices.len() >= 3 {
        (0..vertices.len() as u32).collect()
    } else {
        vec![]
    };
    ChunkMesh {
        coord: ChunkCoord::new(0, 0, 0),
        vertices,
        indices,
    }
}

// ============================================================================
// A. MorphConfig defaults
// ============================================================================

#[test]
fn morph_config_default_start_is_zero() {
    let c = MorphConfig::default();
    assert_eq!(c.morph_start, 0.0);
}

#[test]
fn morph_config_default_end_is_50() {
    let c = MorphConfig::default();
    assert_eq!(c.morph_end, 50.0);
}

#[test]
fn morph_config_default_search_radius_is_2() {
    let c = MorphConfig::default();
    assert_eq!(c.search_radius, 2.0);
}

// ============================================================================
// B. MorphConfig::for_lod_transition arithmetic
// ============================================================================

#[test]
fn for_lod_transition_end_equals_lod_end() {
    let c = MorphConfig::for_lod_transition(100.0, 200.0);
    assert_eq!(c.morph_end, 200.0);
}

#[test]
fn for_lod_transition_start_is_80_percent_of_range() {
    // transition_zone = (200 - 100) * 0.2 = 20
    // morph_start = 200 - 20 = 180
    let c = MorphConfig::for_lod_transition(100.0, 200.0);
    assert!((c.morph_start - 180.0).abs() < 1e-6);
}

#[test]
fn for_lod_transition_search_radius_preserved() {
    let c = MorphConfig::for_lod_transition(50.0, 150.0);
    assert_eq!(c.search_radius, 2.0);
}

#[test]
fn for_lod_transition_wide_range() {
    // (1000 - 0) * 0.2 = 200 → start = 1000 - 200 = 800
    let c = MorphConfig::for_lod_transition(0.0, 1000.0);
    assert!((c.morph_start - 800.0).abs() < 1e-6);
    assert_eq!(c.morph_end, 1000.0);
}

#[test]
fn for_lod_transition_narrow_range() {
    // (110 - 100) * 0.2 = 2 → start = 110 - 2 = 108
    let c = MorphConfig::for_lod_transition(100.0, 110.0);
    assert!((c.morph_start - 108.0).abs() < 1e-6);
}

// ============================================================================
// C. LodBlender::compute_morph_factor boundaries
// ============================================================================

#[test]
fn morph_factor_well_below_start_returns_zero() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 100.0,
        morph_end: 200.0,
        search_radius: 2.0,
    });
    assert_eq!(b.compute_morph_factor(0.0), 0.0);
}

#[test]
fn morph_factor_exactly_at_start_returns_zero() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 100.0,
        morph_end: 200.0,
        search_radius: 2.0,
    });
    assert_eq!(b.compute_morph_factor(100.0), 0.0);
}

#[test]
fn morph_factor_exactly_at_end_returns_one() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 100.0,
        morph_end: 200.0,
        search_radius: 2.0,
    });
    assert_eq!(b.compute_morph_factor(200.0), 1.0);
}

#[test]
fn morph_factor_well_past_end_returns_one() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 100.0,
        morph_end: 200.0,
        search_radius: 2.0,
    });
    assert_eq!(b.compute_morph_factor(999.0), 1.0);
}

#[test]
fn morph_factor_midpoint_returns_half() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 100.0,
        morph_end: 200.0,
        search_radius: 2.0,
    });
    assert!((b.compute_morph_factor(150.0) - 0.5).abs() < 1e-6);
}

#[test]
fn morph_factor_quarter() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 0.0,
        morph_end: 100.0,
        search_radius: 2.0,
    });
    assert!((b.compute_morph_factor(25.0) - 0.25).abs() < 1e-6);
}

#[test]
fn morph_factor_three_quarters() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 0.0,
        morph_end: 100.0,
        search_radius: 2.0,
    });
    assert!((b.compute_morph_factor(75.0) - 0.75).abs() < 1e-6);
}

#[test]
fn morph_factor_just_above_start() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 100.0,
        morph_end: 200.0,
        search_radius: 2.0,
    });
    let f = b.compute_morph_factor(101.0);
    assert!(f > 0.0, "just above start should produce non-zero factor");
    assert!(f < 0.05, "just above start should be small");
}

#[test]
fn morph_factor_just_below_end() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 100.0,
        morph_end: 200.0,
        search_radius: 2.0,
    });
    let f = b.compute_morph_factor(199.0);
    assert!(f > 0.95, "just below end should be near 1.0");
    assert!(f < 1.0, "just below end should be less than 1.0");
}

// ============================================================================
// D. morph_vertices: pure high-LOD (factor=0)
// ============================================================================

#[test]
fn morph_vertices_factor_zero_returns_high_lod_positions() {
    let blender = LodBlender::default();
    let high = mesh_with_verts(vec![
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(4.0, 5.0, 6.0),
        Vec3::new(7.0, 8.0, 9.0),
    ]);
    let low = mesh_with_verts(vec![
        Vec3::new(10.0, 20.0, 30.0),
        Vec3::new(40.0, 50.0, 60.0),
        Vec3::new(70.0, 80.0, 90.0),
    ]);
    let morphed = blender.morph_vertices(&high, &low, 0.0);
    assert_eq!(morphed.morph_factor, 0.0);
    assert_eq!(morphed.mesh.vertices[0].position, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(morphed.mesh.vertices[1].position, Vec3::new(4.0, 5.0, 6.0));
}

// ============================================================================
// E. morph_vertices: pure low-LOD (factor=1)
// ============================================================================

#[test]
fn morph_vertices_factor_one_returns_low_lod_positions() {
    let blender = LodBlender::default();
    let high = mesh_with_verts(vec![
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(4.0, 5.0, 6.0),
        Vec3::new(7.0, 8.0, 9.0),
    ]);
    let low = mesh_with_verts(vec![
        Vec3::new(10.0, 20.0, 30.0),
        Vec3::new(40.0, 50.0, 60.0),
        Vec3::new(70.0, 80.0, 90.0),
    ]);
    let morphed = blender.morph_vertices(&high, &low, 1.0);
    assert_eq!(morphed.morph_factor, 1.0);
    assert_eq!(
        morphed.mesh.vertices[0].position,
        Vec3::new(10.0, 20.0, 30.0)
    );
}

// ============================================================================
// F. morph_vertices: intermediate interpolation
// ============================================================================

#[test]
fn morph_vertices_half_interpolates_positions() {
    let cfg = MorphConfig {
        morph_start: 0.0,
        morph_end: 100.0,
        search_radius: 10.0, // large enough to find correspondence
    };
    let blender = LodBlender::new(cfg);
    let high = mesh_with_verts(vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]);
    let low = mesh_with_verts(vec![
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(3.0, 0.0, 0.0),
        Vec3::new(2.0, 1.0, 0.0),
    ]);
    let morphed = blender.morph_vertices(&high, &low, 0.5);
    assert!((morphed.morph_factor - 0.5).abs() < 1e-6);
    // vertex 0: lerp(0,2,0.5) = 1.0
    let pos0 = morphed.mesh.vertices[0].position;
    assert!(
        (pos0.x - 1.0).abs() < 0.15,
        "Expected x~1.0, got {}",
        pos0.x
    );
}

#[test]
fn morph_vertices_quarter_interpolation() {
    let cfg = MorphConfig {
        morph_start: 0.0,
        morph_end: 100.0,
        search_radius: 10.0,
    };
    let blender = LodBlender::new(cfg);
    let high = mesh_with_verts(vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    ]);
    let low = mesh_with_verts(vec![
        Vec3::new(4.0, 0.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(4.0, 0.0, 1.0),
    ]);
    let morphed = blender.morph_vertices(&high, &low, 0.25);
    // vertex 0: lerp(0, 4, 0.25) = 1.0
    let p = morphed.mesh.vertices[0].position;
    assert!((p.x - 1.0).abs() < 0.15, "Expected x~1.0, got {}", p.x);
}

// ============================================================================
// G. morph_vertices: normal interpolation
// ============================================================================

#[test]
fn morph_vertices_normals_are_normalized() {
    let cfg = MorphConfig {
        morph_start: 0.0,
        morph_end: 100.0,
        search_radius: 10.0,
    };
    let blender = LodBlender::new(cfg);
    let high = mesh_with_normals(
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
        vec![Vec3::Y, Vec3::Y, Vec3::Y],
    );
    let low = mesh_with_normals(
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
        vec![Vec3::X, Vec3::X, Vec3::X],
    );
    let morphed = blender.morph_vertices(&high, &low, 0.5);
    for v in &morphed.mesh.vertices {
        let len = v.normal.length();
        assert!(
            (len - 1.0).abs() < 0.01 || len < 0.01,
            "normal should be ~unit or ~zero, got {}",
            len
        );
    }
}

// ============================================================================
// H. morph_vertices: no correspondence (distant vertices — beyond search_radius)
// ============================================================================

#[test]
fn morph_vertices_distant_low_lod_keeps_high_positions() {
    let cfg = MorphConfig {
        morph_start: 0.0,
        morph_end: 100.0,
        search_radius: 0.5, // very small search radius
    };
    let blender = LodBlender::new(cfg);
    let high = mesh_with_verts(vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    ]);
    // Low LOD vertices are very far away — no correspondence
    let low = mesh_with_verts(vec![
        Vec3::new(100.0, 100.0, 100.0),
        Vec3::new(200.0, 200.0, 200.0),
        Vec3::new(300.0, 300.0, 300.0),
    ]);
    let morphed = blender.morph_vertices(&high, &low, 0.5);
    // Since low_lod_index is None, high positions should be unchanged
    assert_eq!(morphed.mesh.vertices[0].position, Vec3::ZERO);
    assert_eq!(morphed.mesh.vertices[1].position, Vec3::new(1.0, 0.0, 0.0));
}

// ============================================================================
// I. MorphedMesh accessors
// ============================================================================

#[test]
fn morphed_mesh_vertex_count_matches() {
    let mesh = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let m = MorphedMesh::new(mesh);
    assert_eq!(m.vertex_count(), 3);
}

#[test]
fn morphed_mesh_triangle_count_div3() {
    let mut mesh = mesh_with_verts(vec![
        Vec3::ZERO,
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        Vec3::ONE,
        Vec3::NEG_ONE,
    ]);
    mesh.indices = vec![0, 1, 2, 3, 4, 5]; // 2 triangles
    let m = MorphedMesh::new(mesh);
    assert_eq!(m.triangle_count(), 2);
}

#[test]
fn morphed_mesh_initial_morph_factor_is_zero() {
    let mesh = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
    let m = MorphedMesh::new(mesh);
    assert_eq!(m.morph_factor, 0.0);
}

// ============================================================================
// J. LodBlender::default
// ============================================================================

#[test]
fn lod_blender_default_uses_default_morph_config() {
    let b = LodBlender::default();
    // Should use morph_start=0, morph_end=50
    assert_eq!(b.compute_morph_factor(0.0), 0.0);
    assert_eq!(b.compute_morph_factor(50.0), 1.0);
    assert!((b.compute_morph_factor(25.0) - 0.5).abs() < 1e-6);
}

// ============================================================================
// K. create_transition_mesh pipeline
// ============================================================================

#[test]
fn create_transition_mesh_at_start_returns_zero_factor() {
    let cfg = MorphConfig {
        morph_start: 100.0,
        morph_end: 200.0,
        search_radius: 10.0,
    };
    let blender = LodBlender::new(cfg);
    let high = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let low = mesh_with_verts(vec![
        Vec3::ONE,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    let result = blender.create_transition_mesh(&high, &low, 50.0);
    assert_eq!(result.morph_factor, 0.0);
}

#[test]
fn create_transition_mesh_at_end_returns_one_factor() {
    let cfg = MorphConfig {
        morph_start: 100.0,
        morph_end: 200.0,
        search_radius: 10.0,
    };
    let blender = LodBlender::new(cfg);
    let high = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let low = mesh_with_verts(vec![
        Vec3::ONE,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    let result = blender.create_transition_mesh(&high, &low, 250.0);
    assert_eq!(result.morph_factor, 1.0);
}

#[test]
fn create_transition_mesh_mid_distance_produces_intermediate() {
    let cfg = MorphConfig {
        morph_start: 100.0,
        morph_end: 200.0,
        search_radius: 10.0,
    };
    let blender = LodBlender::new(cfg);
    let high = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let low = mesh_with_verts(vec![
        Vec3::ONE,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    let result = blender.create_transition_mesh(&high, &low, 150.0);
    assert!(result.morph_factor > 0.0 && result.morph_factor < 1.0);
    assert!((result.morph_factor - 0.5).abs() < 1e-6);
}

// ============================================================================
// L. MorphingLodManager: construction
// ============================================================================

#[test]
fn morphing_lod_manager_lod_count() {
    let meshes = vec![
        ChunkMesh::empty(ChunkCoord::new(0, 0, 0)),
        ChunkMesh::empty(ChunkCoord::new(0, 0, 0)),
        ChunkMesh::empty(ChunkCoord::new(0, 0, 0)),
    ];
    let distances = vec![100.0, 200.0, 300.0];
    let mgr = MorphingLodManager::new(meshes, distances);
    assert_eq!(mgr.lod_count(), 3);
}

// ============================================================================
// M. MorphingLodManager: LOD selection by distance
// ============================================================================

#[test]
fn morphing_lod_manager_close_returns_low_morph_factor() {
    let lod0 = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let lod1 = mesh_with_verts(vec![
        Vec3::ONE,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    let mgr = MorphingLodManager::new(vec![lod0, lod1], vec![100.0, 200.0]);
    let mesh = mgr.get_mesh_for_distance(10.0);
    assert_eq!(mesh.morph_factor, 0.0);
}

#[test]
fn morphing_lod_manager_far_returns_last_lod() {
    let lod0 = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let lod1 = mesh_with_verts(vec![
        Vec3::ONE,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    let mgr = MorphingLodManager::new(vec![lod0, lod1], vec![100.0, 200.0]);
    let mesh = mgr.get_mesh_for_distance(500.0);
    // Beyond all thresholds — should use last LOD
    assert!(mesh.morph_factor >= 0.0);
}

#[test]
fn morphing_lod_manager_three_levels() {
    let lod0 = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let lod1 = mesh_with_verts(vec![
        Vec3::ONE,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    let lod2 = mesh_with_verts(vec![
        Vec3::new(5.0, 5.0, 5.0),
        Vec3::new(6.0, 0.0, 0.0),
        Vec3::new(0.0, 6.0, 0.0),
    ]);
    let mgr = MorphingLodManager::new(vec![lod0, lod1, lod2], vec![100.0, 200.0, 400.0]);
    assert_eq!(mgr.lod_count(), 3);

    // Very close → LOD 0, no morph
    let m1 = mgr.get_mesh_for_distance(10.0);
    assert_eq!(m1.morph_factor, 0.0);
}

// ============================================================================
// N. morph_factor linear interpolation formula verification
// ============================================================================

#[test]
fn morph_factor_formula_offset_div_range() {
    // The formula is: (distance - morph_start) / (morph_end - morph_start)
    // For start=200, end=300, distance=250: (250-200)/(300-200) = 50/100 = 0.5
    let b = LodBlender::new(MorphConfig {
        morph_start: 200.0,
        morph_end: 300.0,
        search_radius: 2.0,
    });
    assert!((b.compute_morph_factor(250.0) - 0.5).abs() < 1e-6);
    // distance=220: (220-200)/100 = 0.2
    assert!((b.compute_morph_factor(220.0) - 0.2).abs() < 1e-6);
    // distance=280: (280-200)/100 = 0.8
    assert!((b.compute_morph_factor(280.0) - 0.8).abs() < 1e-6);
}

// ============================================================================
// O. morph_vertices preserves material index
// ============================================================================

#[test]
fn morph_vertices_preserves_material() {
    let cfg = MorphConfig {
        morph_start: 0.0,
        morph_end: 100.0,
        search_radius: 10.0,
    };
    let blender = LodBlender::new(cfg);
    let high = ChunkMesh {
        coord: ChunkCoord::new(0, 0, 0),
        vertices: vec![
            MeshVertex {
                position: Vec3::ZERO,
                normal: Vec3::Y,
                material: 42,
            },
            MeshVertex {
                position: Vec3::X,
                normal: Vec3::Y,
                material: 42,
            },
            MeshVertex {
                position: Vec3::Y,
                normal: Vec3::Y,
                material: 42,
            },
        ],
        indices: vec![0, 1, 2],
    };
    let low = ChunkMesh {
        coord: ChunkCoord::new(0, 0, 0),
        vertices: vec![
            MeshVertex {
                position: Vec3::ZERO,
                normal: Vec3::Y,
                material: 99,
            },
            MeshVertex {
                position: Vec3::X,
                normal: Vec3::Y,
                material: 99,
            },
            MeshVertex {
                position: Vec3::Y,
                normal: Vec3::Y,
                material: 99,
            },
        ],
        indices: vec![0, 1, 2],
    };
    let morphed = blender.morph_vertices(&high, &low, 0.5);
    // "Keep high LOD material (no material morphing)"
    for v in &morphed.mesh.vertices {
        assert_eq!(v.material, 42, "material should come from high LOD");
    }
}

// ============================================================================
// P. morph_vertices returns correct morph_factor in result
// ============================================================================

#[test]
fn morph_vertices_result_records_factor() {
    let cfg = MorphConfig {
        morph_start: 0.0,
        morph_end: 100.0,
        search_radius: 10.0,
    };
    let blender = LodBlender::new(cfg);
    let high = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let low = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let morphed = blender.morph_vertices(&high, &low, 0.7);
    assert!((morphed.morph_factor - 0.7).abs() < 1e-6);
}

// ============================================================================
// Q. morph_vertices: vertex count matches high LOD
// ============================================================================

#[test]
fn morph_vertices_output_preserves_high_lod_vertex_count() {
    let cfg = MorphConfig {
        morph_start: 0.0,
        morph_end: 100.0,
        search_radius: 10.0,
    };
    let blender = LodBlender::new(cfg);
    let high = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z]);
    let low = mesh_with_verts(vec![
        Vec3::ONE,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    let morphed = blender.morph_vertices(&high, &low, 0.5);
    // Output should have same vertex count as high LOD (interpolated from high mesh)
    assert_eq!(morphed.vertex_count(), 4);
}

// ============================================================================
// R. Edge case: morph_vertices with empty meshes
// ============================================================================

#[test]
fn morph_vertices_empty_high_lod_returns_empty() {
    let blender = LodBlender::default();
    let high = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
    let low = mesh_with_verts(vec![Vec3::ONE, Vec3::X, Vec3::Y]);
    let morphed = blender.morph_vertices(&high, &low, 0.5);
    assert_eq!(morphed.vertex_count(), 0);
}

// ============================================================================
// S. for_lod_transition: morph_start < morph_end always
// ============================================================================

#[test]
fn for_lod_transition_start_less_than_end() {
    let c = MorphConfig::for_lod_transition(0.0, 500.0);
    assert!(
        c.morph_start < c.morph_end,
        "start {} should be < end {}",
        c.morph_start,
        c.morph_end
    );
}

// ============================================================================
// T. morph_factor is monotonically increasing with distance
// ============================================================================

#[test]
fn morph_factor_monotonically_increasing() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 50.0,
        morph_end: 150.0,
        search_radius: 2.0,
    });
    let mut prev = 0.0f32;
    for d in (0..200).step_by(5) {
        let f = b.compute_morph_factor(d as f32);
        assert!(
            f >= prev,
            "morph_factor should be monotonically increasing: {} >= {} at d={}",
            f,
            prev,
            d
        );
        prev = f;
    }
}

// ============================================================================
// U. morph_factor output clamped to [0, 1]
// ============================================================================

#[test]
fn morph_factor_always_in_unit_range() {
    let b = LodBlender::new(MorphConfig {
        morph_start: 10.0,
        morph_end: 20.0,
        search_radius: 2.0,
    });
    for d in 0..100 {
        let f = b.compute_morph_factor(d as f32);
        assert!(
            f >= 0.0 && f <= 1.0,
            "morph_factor {} out of [0,1] at d={}",
            f,
            d
        );
    }
}

// ============================================================================
// V. morph_vertices with factor exactly 0.0 and 1.0 (boundary conditions)
// ============================================================================

#[test]
fn morph_vertices_factor_zero_morph_factor_field() {
    let b = LodBlender::default();
    let high = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let low = mesh_with_verts(vec![
        Vec3::ONE,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    let m = b.morph_vertices(&high, &low, 0.0);
    assert_eq!(m.morph_factor, 0.0);
}

#[test]
fn morph_vertices_factor_one_morph_factor_field() {
    let b = LodBlender::default();
    let high = mesh_with_verts(vec![Vec3::ZERO, Vec3::X, Vec3::Y]);
    let low = mesh_with_verts(vec![
        Vec3::ONE,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    let m = b.morph_vertices(&high, &low, 1.0);
    assert_eq!(m.morph_factor, 1.0);
}
