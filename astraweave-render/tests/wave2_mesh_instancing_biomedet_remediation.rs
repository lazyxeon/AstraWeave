//! Wave 2 proactive remediation – mesh.rs, instancing.rs, biome_detector.rs
//! Targets: compute_tangents math, AABB boundary, Instance transforms,
//! InstanceManager stats, InstancePatternBuilder geometry, BiomeDetector edge cases.

use astraweave_render::biome_detector::{BiomeDetector, BiomeDetectorConfig};
use astraweave_render::instancing::{
    Instance, InstanceBatch, InstanceManager, InstancePatternBuilder, InstanceRaw,
};
use astraweave_render::mesh::{compute_tangents, CpuMesh, MeshVertex};
use astraweave_terrain::climate::{ClimateConfig, ClimateMap};
use astraweave_terrain::BiomeType;
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use std::f32::consts::PI;

fn test_climate() -> ClimateMap {
    ClimateMap::new(&ClimateConfig::default(), 42)
}

// ─────────────────────────── mesh.rs ───────────────────────────

#[test]
fn mesh_vertex_new_roundtrip() {
    let v = MeshVertex::new(
        Vec3::new(1.0, -2.0, 3.5),
        Vec3::new(0.0, 0.0, -1.0),
        Vec4::new(0.5, 0.5, 0.0, -1.0),
        Vec2::new(0.25, 0.75),
    );
    assert_eq!(v.position, [1.0, -2.0, 3.5]);
    assert_eq!(v.normal, [0.0, 0.0, -1.0]);
    assert_eq!(v.tangent, [0.5, 0.5, 0.0, -1.0]);
    assert_eq!(v.uv, [0.25, 0.75]);
}

#[test]
fn mesh_vertex_from_arrays_roundtrip() {
    let v = MeshVertex::from_arrays(
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0],
    );
    assert_eq!(v.position[0], -1.0);
    assert_eq!(v.normal[1], 1.0);
    assert_eq!(v.tangent[3], 1.0);
    assert_eq!(v.uv[1], 1.0);
}

#[test]
fn mesh_vertex_size_is_48_bytes() {
    assert_eq!(std::mem::size_of::<MeshVertex>(), 48);
}

#[test]
fn mesh_vertex_pod_zeroed() {
    let v: MeshVertex = bytemuck::Zeroable::zeroed();
    assert_eq!(v.position, [0.0, 0.0, 0.0]);
    assert_eq!(v.normal, [0.0, 0.0, 0.0]);
    assert_eq!(v.tangent, [0.0, 0.0, 0.0, 0.0]);
    assert_eq!(v.uv, [0.0, 0.0]);
}

#[test]
fn cpu_mesh_aabb_empty_is_none() {
    assert!(CpuMesh::default().aabb().is_none());
}

#[test]
fn cpu_mesh_aabb_single_vertex() {
    let mut m = CpuMesh::default();
    m.vertices.push(MeshVertex::from_arrays(
        [5.0, -3.0, 7.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 0.0],
    ));
    let (min, max) = m.aabb().unwrap();
    assert_eq!(min, Vec3::new(5.0, -3.0, 7.0));
    assert_eq!(max, Vec3::new(5.0, -3.0, 7.0));
}

#[test]
fn cpu_mesh_aabb_negative_coords() {
    let mut m = CpuMesh::default();
    for p in [[-10.0, -20.0, -30.0], [10.0, 20.0, 30.0]] {
        m.vertices.push(MeshVertex::from_arrays(
            p,
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ));
    }
    let (min, max) = m.aabb().unwrap();
    assert_eq!(min, Vec3::new(-10.0, -20.0, -30.0));
    assert_eq!(max, Vec3::new(10.0, 20.0, 30.0));
}

#[test]
fn cpu_mesh_aabb_all_same_position() {
    let mut m = CpuMesh::default();
    for _ in 0..5 {
        m.vertices.push(MeshVertex::from_arrays(
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ));
    }
    let (min, max) = m.aabb().unwrap();
    assert_eq!(min, max);
    assert_eq!(min, Vec3::new(1.0, 1.0, 1.0));
}

#[test]
fn cpu_mesh_aabb_many_vertices_extremes() {
    let mut m = CpuMesh::default();
    // The min/max should be determined by the extremes mixed across vertices
    m.vertices.push(MeshVertex::from_arrays(
        [0.0, 100.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 0.0],
    ));
    m.vertices.push(MeshVertex::from_arrays(
        [100.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 0.0],
    ));
    m.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 100.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 0.0],
    ));
    let (min, max) = m.aabb().unwrap();
    assert_eq!(min, Vec3::ZERO);
    assert_eq!(max, Vec3::new(100.0, 100.0, 100.0));
}

// compute_tangents: XZ plane triangle with Y-up normals → tangent should be roughly +X
#[test]
fn compute_tangents_xz_plane_tangent_direction() {
    let mut mesh = CpuMesh {
        vertices: vec![
            MeshVertex::from_arrays(
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
                [0.0, 0.0],
            ),
            MeshVertex::from_arrays(
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
                [1.0, 0.0],
            ),
            MeshVertex::from_arrays(
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
                [0.0, 1.0],
            ),
        ],
        indices: vec![0, 1, 2],
        albedo_image: None,
        texture_source_hint: None,
    };
    compute_tangents(&mut mesh);
    // Tangent along UV-u direction = +X world direction
    for v in &mesh.vertices {
        let tx = v.tangent[0];
        assert!(tx > 0.9, "tangent X should be ~1, got {tx}");
        let len = (v.tangent[0].powi(2) + v.tangent[1].powi(2) + v.tangent[2].powi(2)).sqrt();
        assert!((len - 1.0).abs() < 0.02, "tangent not unit len: {len}");
    }
}

// compute_tangents: handedness sign
#[test]
fn compute_tangents_handedness_positive() {
    let mut mesh = CpuMesh {
        vertices: vec![
            MeshVertex::from_arrays([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 4], [0.0, 0.0]),
            MeshVertex::from_arrays([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 4], [1.0, 0.0]),
            MeshVertex::from_arrays([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0; 4], [0.0, 1.0]),
        ],
        indices: vec![0, 1, 2],
        albedo_image: None,
        texture_source_hint: None,
    };
    compute_tangents(&mut mesh);
    // For standard right-hand UV mapping the w component should be +1 or -1
    for v in &mesh.vertices {
        let w = v.tangent[3];
        assert!(w == 1.0 || w == -1.0, "handedness must be ±1, got {w}");
    }
}

// compute_tangents: flipped UV winding flips handedness
#[test]
fn compute_tangents_flipped_uv_flips_handedness() {
    // Standard winding
    let mut mesh_a = CpuMesh {
        vertices: vec![
            MeshVertex::from_arrays([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 4], [0.0, 0.0]),
            MeshVertex::from_arrays([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 4], [1.0, 0.0]),
            MeshVertex::from_arrays([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0; 4], [0.0, 1.0]),
        ],
        indices: vec![0, 1, 2],
        albedo_image: None,
        texture_source_hint: None,
    };
    // Reversed UV-v
    let mut mesh_b = CpuMesh {
        vertices: vec![
            MeshVertex::from_arrays([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 4], [0.0, 1.0]),
            MeshVertex::from_arrays([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 4], [1.0, 1.0]),
            MeshVertex::from_arrays([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0; 4], [0.0, 0.0]),
        ],
        indices: vec![0, 1, 2],
        albedo_image: None,
        texture_source_hint: None,
    };
    compute_tangents(&mut mesh_a);
    compute_tangents(&mut mesh_b);
    let wa = mesh_a.vertices[0].tangent[3];
    let wb = mesh_b.vertices[0].tangent[3];
    assert_ne!(wa, wb, "Flipped UV should invert handedness");
}

// compute_tangents: non-divisible-by-3 indices → early return, original tangents unchanged
#[test]
fn compute_tangents_incomplete_indices_noop() {
    let original_tangent = [0.42, 0.42, 0.42, 0.42];
    let mut mesh = CpuMesh {
        vertices: vec![
            MeshVertex::from_arrays(
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                original_tangent,
                [0.0, 0.0],
            ),
            MeshVertex::from_arrays(
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                original_tangent,
                [1.0, 0.0],
            ),
        ],
        indices: vec![0, 1], // NOT divisible by 3
        albedo_image: None,
        texture_source_hint: None,
    };
    compute_tangents(&mut mesh);
    // Tangents should be untouched
    assert_eq!(mesh.vertices[0].tangent, original_tangent);
}

// compute_tangents: degenerate colocated UVs produce finite tangents
#[test]
fn compute_tangents_degenerate_uv_finite() {
    let mut mesh = CpuMesh {
        vertices: vec![
            MeshVertex::from_arrays([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 4], [0.5, 0.5]),
            MeshVertex::from_arrays([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 4], [0.5, 0.5]),
            MeshVertex::from_arrays([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0; 4], [0.5, 0.5]),
        ],
        indices: vec![0, 1, 2],
        albedo_image: None,
        texture_source_hint: None,
    };
    compute_tangents(&mut mesh);
    for v in &mesh.vertices {
        for &c in &v.tangent {
            assert!(
                c.is_finite(),
                "degenerate UV should still produce finite tangent"
            );
        }
    }
}

// compute_tangents: two-triangle quad produces consistent tangents
#[test]
fn compute_tangents_quad_consistent() {
    let mut mesh = CpuMesh {
        vertices: vec![
            MeshVertex::from_arrays([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 4], [0.0, 0.0]),
            MeshVertex::from_arrays([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 4], [1.0, 0.0]),
            MeshVertex::from_arrays([1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0; 4], [1.0, 1.0]),
            MeshVertex::from_arrays([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0; 4], [0.0, 1.0]),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        albedo_image: None,
        texture_source_hint: None,
    };
    compute_tangents(&mut mesh);
    // All tangents for this flat quad should point roughly +X
    for v in &mesh.vertices {
        assert!(v.tangent[0] > 0.9, "tangent x should be ~1.0");
    }
}

// compute_tangents: empty mesh is no-op
#[test]
fn compute_tangents_empty_mesh() {
    let mut mesh = CpuMesh::default();
    compute_tangents(&mut mesh);
    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
}

// ─────────────────────────── instancing.rs ───────────────────────────

#[test]
fn instance_raw_size_is_64() {
    assert_eq!(std::mem::size_of::<InstanceRaw>(), 64);
}

#[test]
fn instance_raw_from_identity() {
    let raw = InstanceRaw::from_transform(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
    // Identity: diagonal 1s
    for i in 0..4 {
        for j in 0..4 {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!(
                (raw.model[i][j] - expected).abs() < 1e-6,
                "model[{i}][{j}] = {} expected {expected}",
                raw.model[i][j]
            );
        }
    }
}

#[test]
fn instance_raw_from_transform_translation() {
    let raw = InstanceRaw::from_transform(Vec3::new(10.0, 20.0, 30.0), Quat::IDENTITY, Vec3::ONE);
    // Column-major: translation is column 3
    assert!((raw.model[3][0] - 10.0).abs() < 1e-6);
    assert!((raw.model[3][1] - 20.0).abs() < 1e-6);
    assert!((raw.model[3][2] - 30.0).abs() < 1e-6);
    assert!((raw.model[3][3] - 1.0).abs() < 1e-6);
}

#[test]
fn instance_raw_from_transform_scale() {
    let raw = InstanceRaw::from_transform(Vec3::ZERO, Quat::IDENTITY, Vec3::new(2.0, 3.0, 4.0));
    assert!((raw.model[0][0] - 2.0).abs() < 1e-6);
    assert!((raw.model[1][1] - 3.0).abs() < 1e-6);
    assert!((raw.model[2][2] - 4.0).abs() < 1e-6);
}

#[test]
fn instance_raw_from_matrix_roundtrip() {
    let mat = Mat4::from_scale_rotation_translation(
        Vec3::new(2.0, 2.0, 2.0),
        Quat::from_rotation_y(PI / 4.0),
        Vec3::new(5.0, 6.0, 7.0),
    );
    let raw = InstanceRaw::from_matrix(mat);
    let reconstructed = Mat4::from_cols_array_2d(&raw.model);
    let diff = (mat - reconstructed).abs_diff_eq(Mat4::ZERO, 1e-6);
    assert!(diff, "from_matrix roundtrip failed");
}

#[test]
fn instance_identity_values() {
    let inst = Instance::identity();
    assert_eq!(inst.position, Vec3::ZERO);
    assert_eq!(inst.rotation, Quat::IDENTITY);
    assert_eq!(inst.scale, Vec3::ONE);
}

#[test]
fn instance_new_stores_params() {
    let pos = Vec3::new(1.0, 2.0, 3.0);
    let rot = Quat::from_rotation_z(0.5);
    let scl = Vec3::splat(0.5);
    let inst = Instance::new(pos, rot, scl);
    assert_eq!(inst.position, pos);
    assert_eq!(inst.scale, scl);
    let q_diff = inst.rotation.dot(rot);
    assert!((q_diff - 1.0).abs() < 1e-6);
}

#[test]
fn instance_to_raw_matches_from_transform() {
    let inst = Instance::new(
        Vec3::new(3.0, 4.0, 5.0),
        Quat::from_rotation_y(1.0),
        Vec3::new(0.5, 0.5, 0.5),
    );
    let raw_a = inst.to_raw();
    let raw_b = InstanceRaw::from_transform(inst.position, inst.rotation, inst.scale);
    for i in 0..4 {
        for j in 0..4 {
            assert!(
                (raw_a.model[i][j] - raw_b.model[i][j]).abs() < 1e-6,
                "to_raw and from_transform disagree at [{i}][{j}]"
            );
        }
    }
}

#[test]
fn instance_raw_desc_layout() {
    let layout = InstanceRaw::desc();
    assert_eq!(layout.array_stride, 64); // 4x4 f32 matrix
    assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
    assert_eq!(layout.attributes.len(), 4); // 4 columns
}

#[test]
fn instance_raw_desc_shader_locations() {
    let layout = InstanceRaw::desc();
    assert_eq!(layout.attributes[0].shader_location, 5);
    assert_eq!(layout.attributes[1].shader_location, 6);
    assert_eq!(layout.attributes[2].shader_location, 7);
    assert_eq!(layout.attributes[3].shader_location, 8);
}

#[test]
fn instance_batch_new_empty() {
    let batch = InstanceBatch::new(99);
    assert_eq!(batch.mesh_id, 99);
    assert_eq!(batch.instance_count(), 0);
    assert!(batch.buffer.is_none());
}

#[test]
fn instance_batch_add_increments_count() {
    let mut batch = InstanceBatch::new(1);
    batch.add_instance(Instance::identity());
    assert_eq!(batch.instance_count(), 1);
    batch.add_instance(Instance::identity());
    assert_eq!(batch.instance_count(), 2);
}

#[test]
fn instance_batch_clear() {
    let mut batch = InstanceBatch::new(1);
    batch.add_instance(Instance::identity());
    batch.add_instance(Instance::identity());
    batch.clear();
    assert_eq!(batch.instance_count(), 0);
}

#[test]
fn instance_manager_default() {
    let mgr = InstanceManager::default();
    assert_eq!(mgr.total_instances(), 0);
    assert_eq!(mgr.batch_count(), 0);
    assert_eq!(mgr.draw_calls_saved(), 0);
}

#[test]
fn instance_manager_add_single() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(42, Instance::identity());
    assert_eq!(mgr.total_instances(), 1);
    assert_eq!(mgr.batch_count(), 1);
    assert!(mgr.get_batch(42).is_some());
    assert!(mgr.get_batch(99).is_none());
}

#[test]
fn instance_manager_add_multiple_meshes() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(1, Instance::identity());
    mgr.add_instance(2, Instance::identity());
    mgr.add_instance(1, Instance::identity());
    assert_eq!(mgr.total_instances(), 3);
    assert_eq!(mgr.batch_count(), 2);
    assert_eq!(mgr.get_batch(1).unwrap().instance_count(), 2);
    assert_eq!(mgr.get_batch(2).unwrap().instance_count(), 1);
}

#[test]
fn instance_manager_add_instances_bulk() {
    let mut mgr = InstanceManager::new();
    let batch = vec![
        Instance::identity(),
        Instance::identity(),
        Instance::identity(),
    ];
    mgr.add_instances(10, batch);
    assert_eq!(mgr.total_instances(), 3);
    assert_eq!(mgr.get_batch(10).unwrap().instance_count(), 3);
}

#[test]
fn instance_manager_clear() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(1, Instance::identity());
    mgr.add_instance(2, Instance::identity());
    mgr.clear();
    assert_eq!(mgr.total_instances(), 0);
    assert_eq!(mgr.batch_count(), 0);
    assert_eq!(mgr.draw_calls_saved(), 0);
}

#[test]
fn instance_manager_draw_call_reduction_empty() {
    let mgr = InstanceManager::new();
    assert_eq!(mgr.draw_call_reduction_percent(), 0.0);
}

#[test]
fn pattern_builder_empty() {
    let instances = InstancePatternBuilder::new().build();
    assert!(instances.is_empty());
}

#[test]
fn pattern_builder_grid_count() {
    let instances = InstancePatternBuilder::new().grid(4, 5, 1.0).build();
    assert_eq!(instances.len(), 20); // 4 rows * 5 cols
}

#[test]
fn pattern_builder_grid_positions() {
    let instances = InstancePatternBuilder::new().grid(2, 3, 10.0).build();
    // row0: (0,0,0), (10,0,0), (20,0,0)
    // row1: (0,0,10), (10,0,10), (20,0,10)
    assert!((instances[0].position - Vec3::new(0.0, 0.0, 0.0)).length() < 1e-6);
    assert!((instances[1].position - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-6);
    assert!((instances[2].position - Vec3::new(20.0, 0.0, 0.0)).length() < 1e-6);
    assert!((instances[3].position - Vec3::new(0.0, 0.0, 10.0)).length() < 1e-6);
}

#[test]
fn pattern_builder_grid_all_y_zero() {
    let instances = InstancePatternBuilder::new().grid(3, 3, 5.0).build();
    for inst in &instances {
        assert_eq!(inst.position.y, 0.0);
    }
}

#[test]
fn pattern_builder_circle_count() {
    let instances = InstancePatternBuilder::new().circle(12, 5.0).build();
    assert_eq!(instances.len(), 12);
}

#[test]
fn pattern_builder_circle_radius() {
    let r = 7.5;
    let instances = InstancePatternBuilder::new().circle(16, r).build();
    for inst in &instances {
        let dist = inst.position.length();
        assert!((dist - r).abs() < 0.01, "distance {dist} should be ~{r}");
    }
}

#[test]
fn pattern_builder_circle_all_y_zero() {
    let instances = InstancePatternBuilder::new().circle(8, 10.0).build();
    for inst in &instances {
        assert_eq!(inst.position.y, 0.0);
    }
}

#[test]
fn pattern_builder_grid_then_circle_chained() {
    let instances = InstancePatternBuilder::new()
        .grid(2, 2, 1.0)
        .circle(4, 5.0)
        .build();
    assert_eq!(instances.len(), 8); // 4 grid + 4 circle
}

// ─────────────────────────── biome_detector.rs ───────────────────────────

#[test]
fn biome_detector_default_config() {
    let cfg = BiomeDetectorConfig::default();
    assert!(cfg.sample_distance_threshold > 0.0);
    assert!(cfg.hysteresis_count > 0);
}

#[test]
fn biome_detector_initial_state() {
    let det = BiomeDetector::new(BiomeDetectorConfig::default());
    assert!(det.current_biome().is_none());
    assert_eq!(det.transition_count(), 0);
}

#[test]
fn biome_detector_first_update_sets_biome() {
    let climate = test_climate();
    let mut det = BiomeDetector::new(BiomeDetectorConfig {
        sample_distance_threshold: 0.0,
        hysteresis_count: 3,
    });
    let t = det.update(&climate, 0.0, 0.0, 50.0);
    assert!(t.is_some(), "First update should produce transition");
    assert!(det.current_biome().is_some());
    assert_eq!(det.transition_count(), 1);
}

#[test]
fn biome_detector_stationary_no_extra_transitions() {
    let climate = test_climate();
    let mut det = BiomeDetector::new(BiomeDetectorConfig {
        sample_distance_threshold: 0.0,
        hysteresis_count: 3,
    });
    let _ = det.update(&climate, 0.0, 0.0, 50.0); // first
                                                  // Same position many times
    for _ in 0..20 {
        let t = det.update(&climate, 0.0, 0.0, 50.0);
        assert!(t.is_none(), "Same position = same biome, no transition");
    }
    assert_eq!(det.transition_count(), 1);
}

#[test]
fn biome_detector_distance_threshold_gates_sampling() {
    let climate = test_climate();
    let cfg = BiomeDetectorConfig {
        sample_distance_threshold: 100.0, // very large
        hysteresis_count: 1,
    };
    let mut det = BiomeDetector::new(cfg);
    let _ = det.update(&climate, 0.0, 0.0, 50.0);
    let first = det.current_biome();
    // Move slightly — under threshold
    let t = det.update(&climate, 1.0, 1.0, 50.0);
    assert!(t.is_none(), "Movement under threshold returns None");
    assert_eq!(det.current_biome(), first);
}

#[test]
fn biome_detector_classify_scored_desert() {
    // Hot temp + low moisture → Desert (temperature and moisture are 0-1 range)
    let biome = BiomeDetector::classify_scored(5.0, 0.9, 0.1);
    assert_eq!(biome, BiomeType::Desert);
}

#[test]
fn biome_detector_classify_scored_tundra() {
    // Cold → Tundra
    let biome = BiomeDetector::classify_scored(5.0, 0.1, 0.3);
    assert_eq!(biome, BiomeType::Tundra);
}

#[test]
fn biome_detector_classify_scored_mild() {
    // Mild + medium moisture → Grassland or Forest
    let biome = BiomeDetector::classify_scored(25.0, 0.5, 0.5);
    assert!(
        biome == BiomeType::Grassland || biome == BiomeType::Forest,
        "Expected grassland or forest, got {:?}",
        biome
    );
}

#[test]
fn biome_detector_set_biome_override() {
    let mut det = BiomeDetector::new(BiomeDetectorConfig::default());
    det.set_biome(BiomeType::Beach);
    assert_eq!(det.current_biome(), Some(BiomeType::Beach));
}

#[test]
fn biome_detector_set_biome_replaces_previous() {
    let mut det = BiomeDetector::new(BiomeDetectorConfig::default());
    det.set_biome(BiomeType::Mountain);
    assert_eq!(det.current_biome(), Some(BiomeType::Mountain));
    det.set_biome(BiomeType::Swamp);
    assert_eq!(det.current_biome(), Some(BiomeType::Swamp));
}

#[test]
fn biome_detector_reset_clears_biome() {
    let climate = test_climate();
    let mut det = BiomeDetector::new(BiomeDetectorConfig {
        sample_distance_threshold: 0.0,
        hysteresis_count: 1,
    });
    let _ = det.update(&climate, 0.0, 0.0, 10.0);
    assert!(det.current_biome().is_some());
    det.reset();
    assert!(det.current_biome().is_none());
}

#[test]
fn biome_detector_reset_allows_reinitialize() {
    let climate = test_climate();
    let mut det = BiomeDetector::new(BiomeDetectorConfig {
        sample_distance_threshold: 0.0,
        hysteresis_count: 1,
    });
    let _ = det.update(&climate, 0.0, 0.0, 10.0);
    det.reset();
    // After reset, next update should act as first sample
    let t = det.update(&climate, 500.0, 500.0, 10.0);
    assert!(
        t.is_some(),
        "After reset, first update should trigger transition"
    );
}

#[test]
fn biome_detector_classify_scored_deterministic() {
    // Same inputs should always give same output
    let b1 = BiomeDetector::classify_scored(50.0, 0.5, 0.5);
    let b2 = BiomeDetector::classify_scored(50.0, 0.5, 0.5);
    assert_eq!(b1, b2);
}

#[test]
fn biome_detector_classify_scored_extremes() {
    // Very high temperature + very low moisture → Desert
    let hot_dry = BiomeDetector::classify_scored(0.0, 1.0, 0.0);
    assert_eq!(hot_dry, BiomeType::Desert);
    // Very low temp + very low moisture
    let cold_dry = BiomeDetector::classify_scored(0.0, 0.0, 0.0);
    assert_eq!(cold_dry, BiomeType::Tundra);
}

#[test]
fn biome_detector_transition_info_has_correct_coords() {
    let climate = test_climate();
    let mut det = BiomeDetector::new(BiomeDetectorConfig {
        sample_distance_threshold: 0.0,
        hysteresis_count: 1,
    });
    let t = det
        .update(&climate, 42.5, 99.0, 33.0)
        .expect("first update");
    assert_eq!(t.x, 42.5);
    assert_eq!(t.z, 99.0);
    assert_eq!(t.height, 33.0);
    assert!(t.old_biome.is_none(), "first transition has no old biome");
}

#[test]
fn biome_detector_transition_count_starts_zero() {
    let det = BiomeDetector::new(BiomeDetectorConfig::default());
    assert_eq!(det.transition_count(), 0);
}
