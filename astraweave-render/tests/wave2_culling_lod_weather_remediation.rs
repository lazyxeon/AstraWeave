//! Wave 2 Render Sweep — Culling, LOD, Weather System Proactive Remediation
//!
//! Targets: culling.rs (142 mutants), lod_generator.rs (168 mutants),
//! weather_system.rs (162 mutants — integration tests complementing existing unit tests)

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::culling::{
    batch_visible_instances, build_indirect_commands_cpu, cpu_frustum_cull, BatchId, DrawBatch,
    DrawIndirectCommand, FrustumPlanes, InstanceAABB,
};
use astraweave_render::effects::WeatherKind;
use astraweave_render::lod_generator::{LODConfig, LODGenerator, SimplificationMesh};
use astraweave_render::weather_system::{BiomeWeatherMap, BiomeWindProfile, WeatherTransition};
use astraweave_terrain::biome::BiomeType;
use glam::{Mat4, Vec3};

// ═════════════════════════════════════════════════════════════════════════
// InstanceAABB
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn aabb_new_stores_center_and_extent() {
    let aabb = InstanceAABB::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.5, 0.5, 0.5), 42);
    assert_eq!(aabb.center, [1.0, 2.0, 3.0]);
    assert_eq!(aabb.extent, [0.5, 0.5, 0.5]);
    assert_eq!(aabb.instance_index, 42);
}

#[test]
fn aabb_from_identity_transform() {
    let transform = Mat4::IDENTITY;
    let local_min = Vec3::new(-1.0, -1.0, -1.0);
    let local_max = Vec3::new(1.0, 1.0, 1.0);
    let aabb = InstanceAABB::from_transform(&transform, local_min, local_max, 0);

    // With identity transform, center should be (0,0,0) and extent (1,1,1)
    assert!((aabb.center[0] - 0.0).abs() < 0.01);
    assert!((aabb.center[1] - 0.0).abs() < 0.01);
    assert!((aabb.center[2] - 0.0).abs() < 0.01);
    assert!((aabb.extent[0] - 1.0).abs() < 0.01);
    assert!((aabb.extent[1] - 1.0).abs() < 0.01);
    assert!((aabb.extent[2] - 1.0).abs() < 0.01);
}

#[test]
fn aabb_from_translation_transform() {
    let transform = Mat4::from_translation(Vec3::new(5.0, 10.0, 15.0));
    let local_min = Vec3::new(-1.0, -1.0, -1.0);
    let local_max = Vec3::new(1.0, 1.0, 1.0);
    let aabb = InstanceAABB::from_transform(&transform, local_min, local_max, 7);

    // Center should be translated
    assert!((aabb.center[0] - 5.0).abs() < 0.01);
    assert!((aabb.center[1] - 10.0).abs() < 0.01);
    assert!((aabb.center[2] - 15.0).abs() < 0.01);
    // Extent should remain the same
    assert!((aabb.extent[0] - 1.0).abs() < 0.01);
    assert!((aabb.extent[1] - 1.0).abs() < 0.01);
    assert!((aabb.extent[2] - 1.0).abs() < 0.01);
    assert_eq!(aabb.instance_index, 7);
}

#[test]
fn aabb_from_scale_transform() {
    let transform = Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
    let local_min = Vec3::new(-1.0, -1.0, -1.0);
    let local_max = Vec3::new(1.0, 1.0, 1.0);
    let aabb = InstanceAABB::from_transform(&transform, local_min, local_max, 0);

    assert!(
        (aabb.extent[0] - 2.0).abs() < 0.01,
        "Scaled extent X should be 2.0, got {}",
        aabb.extent[0]
    );
    assert!(
        (aabb.extent[1] - 3.0).abs() < 0.01,
        "Scaled extent Y should be 3.0, got {}",
        aabb.extent[1]
    );
    assert!(
        (aabb.extent[2] - 4.0).abs() < 0.01,
        "Scaled extent Z should be 4.0, got {}",
        aabb.extent[2]
    );
}

// ═════════════════════════════════════════════════════════════════════════
// FrustumPlanes
// ═════════════════════════════════════════════════════════════════════════

fn make_ortho_frustum() -> FrustumPlanes {
    // Simple orthographic-like view-proj for testing
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
    FrustumPlanes::from_view_proj(&(proj * view))
}

#[test]
fn frustum_from_view_proj_origin_visible() {
    let frustum = make_ortho_frustum();
    // Origin should be visible from camera at z=5 looking at origin
    let visible = frustum.test_aabb(Vec3::ZERO, Vec3::splat(0.5));
    assert!(visible, "Origin should be visible from camera");
}

#[test]
fn frustum_far_object_not_visible() {
    let frustum = make_ortho_frustum();
    // Object very far behind camera at z=500 should NOT be visible
    let visible = frustum.test_aabb(Vec3::new(0.0, 0.0, 500.0), Vec3::splat(0.5));
    assert!(!visible, "Object far behind camera should not be visible");
}

#[test]
fn frustum_object_at_side_not_visible() {
    let frustum = make_ortho_frustum();
    // Object far to the right should not be visible (perspective frustum at FRAC_PI_4 = 45°)
    let visible = frustum.test_aabb(Vec3::new(1000.0, 0.0, 0.0), Vec3::splat(0.5));
    assert!(!visible, "Object far to right should not be visible");
}

#[test]
fn frustum_large_aabb_near_camera_visible() {
    let frustum = make_ortho_frustum();
    // A huge AABB centered near origin should be visible
    let visible = frustum.test_aabb(Vec3::ZERO, Vec3::splat(50.0));
    assert!(visible, "Large AABB near camera should be visible");
}

// ═════════════════════════════════════════════════════════════════════════
// cpu_frustum_cull
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cull_empty_instances_returns_empty() {
    let frustum = make_ortho_frustum();
    let result = cpu_frustum_cull(&[], &frustum);
    assert!(result.is_empty());
}

#[test]
fn cull_visible_instances_returned() {
    let frustum = make_ortho_frustum();
    let instances = vec![
        InstanceAABB::new(Vec3::ZERO, Vec3::splat(1.0), 0), // visible
        InstanceAABB::new(Vec3::new(0.0, 0.0, 500.0), Vec3::splat(0.5), 1), // behind camera
        InstanceAABB::new(Vec3::new(1.0, 0.0, 0.0), Vec3::splat(1.0), 2), // visible
    ];
    let visible = cpu_frustum_cull(&instances, &frustum);
    assert!(visible.contains(&0), "Instance at origin should be visible");
    assert!(
        !visible.contains(&1),
        "Instance behind camera should not be visible"
    );
    assert!(
        visible.contains(&2),
        "Instance slightly to right should be visible"
    );
}

#[test]
fn cull_preserves_instance_indices() {
    let frustum = make_ortho_frustum();
    let instances = vec![
        InstanceAABB::new(Vec3::ZERO, Vec3::splat(1.0), 42),
        InstanceAABB::new(Vec3::ZERO, Vec3::splat(1.0), 99),
    ];
    let visible = cpu_frustum_cull(&instances, &frustum);
    assert!(visible.contains(&42));
    assert!(visible.contains(&99));
}

// ═════════════════════════════════════════════════════════════════════════
// DrawBatch & build_indirect_commands_cpu
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn draw_batch_new_empty() {
    let batch = DrawBatch::new(BatchId::new(1, 2), 100, 0);
    assert_eq!(batch.instance_count(), 0);
    assert_eq!(batch.vertex_count, 100);
    assert_eq!(batch.first_vertex, 0);
}

#[test]
fn draw_batch_add_instances() {
    let mut batch = DrawBatch::new(BatchId::new(1, 2), 100, 0);
    batch.add_instance(5);
    batch.add_instance(10);
    batch.add_instance(15);
    assert_eq!(batch.instance_count(), 3);
}

#[test]
fn build_indirect_commands_empty() {
    let cmds = build_indirect_commands_cpu(&[]);
    assert!(cmds.is_empty());
}

#[test]
fn build_indirect_commands_single_batch() {
    let mut batch = DrawBatch::new(BatchId::new(0, 0), 36, 0);
    batch.add_instance(0);
    batch.add_instance(1);
    batch.add_instance(2);

    let cmds = build_indirect_commands_cpu(&[batch]);
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].vertex_count, 36);
    assert_eq!(cmds[0].instance_count, 3);
    assert_eq!(cmds[0].first_vertex, 0);
}

#[test]
fn build_indirect_commands_multiple_batches() {
    let mut b1 = DrawBatch::new(BatchId::new(0, 0), 36, 0);
    b1.add_instance(0);
    let mut b2 = DrawBatch::new(BatchId::new(1, 0), 24, 36);
    b2.add_instance(1);
    b2.add_instance(2);

    let cmds = build_indirect_commands_cpu(&[b1, b2]);
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0].instance_count, 1);
    assert_eq!(cmds[1].instance_count, 2);
    assert_eq!(cmds[1].first_vertex, 36);
}

#[test]
fn draw_indirect_command_new() {
    let cmd = DrawIndirectCommand::new(100, 5, 200, 300);
    assert_eq!(cmd.vertex_count, 100);
    assert_eq!(cmd.instance_count, 5);
    assert_eq!(cmd.first_vertex, 200);
    assert_eq!(cmd.first_instance, 300);
}

// ═════════════════════════════════════════════════════════════════════════
// batch_visible_instances
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn batch_visible_empty() {
    let batches = batch_visible_instances(&[], |_| BatchId::new(0, 0), |_| (36, 0));
    assert!(batches.is_empty());
}

#[test]
fn batch_visible_groups_by_batch_id() {
    // Instances 0,1 → mesh 0, instance 2 → mesh 1
    let batches = batch_visible_instances(
        &[0, 1, 2],
        |idx| {
            if idx < 2 {
                BatchId::new(0, 0)
            } else {
                BatchId::new(1, 0)
            }
        },
        |batch_id| {
            if batch_id.mesh_id == 0 {
                (36, 0)
            } else {
                (24, 36)
            }
        },
    );

    assert_eq!(batches.len(), 2, "Should have 2 batches");
    // Find the batch with mesh_id=0
    let b0 = batches.iter().find(|b| b.batch_id.mesh_id == 0).unwrap();
    assert_eq!(b0.instance_count(), 2, "Batch 0 should have 2 instances");
    let b1 = batches.iter().find(|b| b.batch_id.mesh_id == 1).unwrap();
    assert_eq!(b1.instance_count(), 1, "Batch 1 should have 1 instance");
}

#[test]
fn batch_id_equality() {
    let a = BatchId::new(1, 2);
    let b = BatchId::new(1, 2);
    let c = BatchId::new(2, 2);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

// ═════════════════════════════════════════════════════════════════════════
// LOD Generator
// ═════════════════════════════════════════════════════════════════════════

fn make_test_mesh() -> SimplificationMesh {
    // Simple 4-vertex quad (2 triangles)
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let normals = vec![Vec3::Z; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = vec![0, 1, 2, 0, 2, 3];
    SimplificationMesh::new(positions, normals, uvs, indices)
}

fn make_grid_mesh(n: usize) -> SimplificationMesh {
    // Create an NxN grid mesh with (N+1)^2 vertices and 2*N*N triangles
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for y in 0..=n {
        for x in 0..=n {
            positions.push(Vec3::new(x as f32, 0.0, y as f32));
            normals.push(Vec3::Y);
            uvs.push([x as f32 / n as f32, y as f32 / n as f32]);
        }
    }

    for y in 0..n {
        for x in 0..n {
            let tl = (y * (n + 1) + x) as u32;
            let tr = tl + 1;
            let bl = tl + (n + 1) as u32;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, tr, bl, tr, br, bl]);
        }
    }

    SimplificationMesh::new(positions, normals, uvs, indices)
}

#[test]
fn simplification_mesh_vertex_count() {
    let mesh = make_test_mesh();
    assert_eq!(mesh.vertex_count(), 4);
}

#[test]
fn simplification_mesh_triangle_count() {
    let mesh = make_test_mesh();
    assert_eq!(mesh.triangle_count(), 2);
}

#[test]
fn simplify_no_reduction_needed() {
    let config = LODConfig::default();
    let gen = LODGenerator::new(config);
    let mesh = make_test_mesh();
    let simplified = gen.simplify(&mesh, 10); // target > vertex count
    assert_eq!(
        simplified.vertex_count(),
        mesh.vertex_count(),
        "Should not reduce when target >= vertex count"
    );
}

#[test]
fn simplify_reduces_vertex_count() {
    let config = LODConfig {
        max_error: 100.0, // high tolerance
        preserve_boundaries: false,
        reduction_targets: vec![0.5],
    };
    let gen = LODGenerator::new(config);
    let mesh = make_grid_mesh(8); // 81 vertices
    let simplified = gen.simplify(&mesh, 40);
    assert!(simplified.vertex_count() <= 81, "Should reduce vertices");
    // Some vertices should be removed
    assert!(
        simplified.vertex_count() < 81,
        "Grid mesh should have fewer vertices after simplify"
    );
}

#[test]
fn simplify_preserves_valid_indices() {
    let config = LODConfig {
        max_error: 50.0,
        preserve_boundaries: false,
        reduction_targets: vec![0.5],
    };
    let gen = LODGenerator::new(config);
    let mesh = make_grid_mesh(4); // 25 vertices
    let simplified = gen.simplify(&mesh, 12);
    // All indices should be valid
    for &idx in &simplified.indices {
        assert!(
            (idx as usize) < simplified.vertex_count(),
            "Index {} out of bounds for {} vertices",
            idx,
            simplified.vertex_count()
        );
    }
    // indices count should be divisible by 3
    assert_eq!(
        simplified.indices.len() % 3,
        0,
        "Indices should be triangle list"
    );
}

#[test]
fn generate_lods_produces_correct_count() {
    let config = LODConfig {
        reduction_targets: vec![0.75, 0.50, 0.25],
        max_error: 100.0,
        preserve_boundaries: false,
    };
    let gen = LODGenerator::new(config);
    let mesh = make_grid_mesh(8);
    let lods = gen.generate_lods(&mesh);
    assert_eq!(lods.len(), 3, "Should produce 3 LOD levels");
}

#[test]
fn generate_lods_decreasing_vertices() {
    let config = LODConfig {
        reduction_targets: vec![0.75, 0.50, 0.25],
        max_error: 100.0,
        preserve_boundaries: false,
    };
    let gen = LODGenerator::new(config);
    let mesh = make_grid_mesh(8); // 81 vertices
    let lods = gen.generate_lods(&mesh);
    // Each LOD should have fewer vertices than original
    for (i, lod) in lods.iter().enumerate() {
        assert!(
            lod.vertex_count() <= mesh.vertex_count(),
            "LOD{} should have fewer vertices than original",
            i
        );
    }
}

#[test]
fn calculate_reduction_zero_for_same() {
    let config = LODConfig::default();
    let gen = LODGenerator::new(config);
    let mesh = make_test_mesh();
    let reduction = gen.calculate_reduction(&mesh, &mesh);
    assert!(
        (reduction - 0.0).abs() < 0.01,
        "Same mesh should have 0% reduction"
    );
}

#[test]
fn calculate_reduction_correct_value() {
    let config = LODConfig::default();
    let gen = LODGenerator::new(config);
    let original = make_grid_mesh(4); // 25 vertices
                                      // Create a "simplified" mesh with 12 vertices
    let simplified = SimplificationMesh::new(
        vec![Vec3::ZERO; 12],
        vec![Vec3::Y; 12],
        vec![[0.0, 0.0]; 12],
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    );
    let reduction = gen.calculate_reduction(&original, &simplified);
    let expected = 1.0 - (12.0 / 25.0);
    assert!(
        (reduction - expected).abs() < 0.01,
        "Expected {expected}, got {reduction}"
    );
}

#[test]
fn lod_config_default_has_three_levels() {
    let config = LODConfig::default();
    assert_eq!(config.reduction_targets.len(), 3);
    assert!((config.reduction_targets[0] - 0.75).abs() < 0.01);
    assert!((config.reduction_targets[1] - 0.50).abs() < 0.01);
    assert!((config.reduction_targets[2] - 0.25).abs() < 0.01);
}

// ═════════════════════════════════════════════════════════════════════════
// WeatherTransition integration tests (public API only)
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn weather_transition_new_defaults() {
    let wt = WeatherTransition::new(5.0);
    assert!(!wt.is_active());
    assert_eq!(wt.current_kind(), WeatherKind::None);
    assert_eq!(wt.from_kind(), WeatherKind::None);
    assert_eq!(wt.to_kind(), WeatherKind::None);
    assert!((wt.duration() - 5.0).abs() < 0.01);
}

#[test]
fn weather_transition_default_trait() {
    let wt = WeatherTransition::default();
    assert!((wt.duration() - 3.0).abs() < 0.01);
}

#[test]
fn weather_transition_crossfade_multipliers() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);

    // At start: fog=1.0 (None), at end fog=2.5 (Rain)
    let (fog0, amb0) = wt.current_multipliers();
    assert!((fog0 - 1.0).abs() < 0.01, "Start fog should be 1.0");
    assert!((amb0 - 1.0).abs() < 0.01, "Start ambient should be 1.0");

    wt.update(2.0); // complete
    let (fog1, amb1) = wt.current_multipliers();
    assert!((fog1 - 2.5).abs() < 0.01, "End fog should be 2.5");
    assert!((amb1 - 0.6).abs() < 0.01, "End ambient should be 0.6");
}

#[test]
fn weather_transition_particle_fades() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::Rain, WeatherKind::Snow);

    let out_start = wt.outgoing_particle_fade();
    let in_start = wt.incoming_particle_fade();
    assert!((out_start - 1.0).abs() < 0.01);
    assert!((in_start - 0.0).abs() < 0.01);

    wt.update(2.0);
    let out_end = wt.outgoing_particle_fade();
    let in_end = wt.incoming_particle_fade();
    assert!((out_end - 0.0).abs() < 0.01);
    assert!((in_end - 1.0).abs() < 0.01);
}

#[test]
fn weather_transition_eased_progress_smooth() {
    let mut wt = WeatherTransition::new(4.0);
    wt.start(WeatherKind::None, WeatherKind::Snow);

    // Smoothstep should be symmetric around t=0.5
    wt.update(2.0); // linear progress = 0.5
    let eased_mid = wt.eased_progress();
    assert!(
        (eased_mid - 0.5).abs() < 0.01,
        "Smoothstep at 0.5 should be 0.5, got {eased_mid}"
    );

    // At low t, eased should be less than linear
    let mut wt2 = WeatherTransition::new(4.0);
    wt2.start(WeatherKind::None, WeatherKind::Snow);
    wt2.update(1.0); // linear = 0.25
    let eased_low = wt2.eased_progress();
    assert!(
        eased_low < 0.25 + 0.01,
        "Smoothstep at 0.25 should be < 0.25"
    );
}

#[test]
fn weather_transition_complete_forces_finish() {
    let mut wt = WeatherTransition::new(10.0);
    wt.start(WeatherKind::None, WeatherKind::Sandstorm);
    wt.update(0.1); // barely started
    assert!(wt.is_active());
    wt.complete();
    assert!(!wt.is_active());
    assert!((wt.progress() - 1.0).abs() < 0.001);
}

#[test]
fn weather_transition_set_duration_clamped() {
    let mut wt = WeatherTransition::new(1.0);
    wt.set_duration(-10.0);
    assert!(
        wt.duration() >= 0.01,
        "Duration should be clamped to min 0.01"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// BiomeWeatherMap integration tests
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn biome_weather_map_weights_forest() {
    let weights = BiomeWeatherMap::weights(BiomeType::Forest);
    assert!(!weights.is_empty());
    let total: f32 = weights.iter().map(|w| w.weight).sum();
    assert!((total - 1.0).abs() < 0.02, "Weights should sum to ~1.0");
}

#[test]
fn biome_weather_map_pick_all_biomes_valid() {
    for &biome in BiomeType::all() {
        for roll in [0.0_f32, 0.25, 0.5, 0.75, 0.99] {
            let kind = BiomeWeatherMap::pick(biome, roll);
            let table = BiomeWeatherMap::weights(biome);
            assert!(
                table.iter().any(|w| w.kind == kind),
                "pick({:?}, {roll}) returned invalid WeatherKind",
                biome
            );
        }
    }
}

#[test]
fn biome_weather_map_probability_sums_to_one() {
    for &biome in BiomeType::all() {
        let kinds = [
            WeatherKind::None,
            WeatherKind::Rain,
            WeatherKind::Snow,
            WeatherKind::Sandstorm,
            WeatherKind::WindTrails,
        ];
        let sum: f32 = kinds
            .iter()
            .map(|&k| BiomeWeatherMap::probability(biome, k))
            .sum();
        assert!(
            (sum - 1.0).abs() < 0.02,
            "{:?} probabilities sum to {sum}",
            biome
        );
    }
}

#[test]
fn biome_weather_map_most_likely_tundra_snow() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Tundra),
        WeatherKind::Snow
    );
}

#[test]
fn biome_weather_map_most_likely_desert_none() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Desert),
        WeatherKind::None
    );
}

#[test]
fn biome_weather_map_most_likely_swamp_rain() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Swamp),
        WeatherKind::Rain
    );
}

#[test]
fn biome_weather_map_pick_desert_sandstorm_range() {
    // Desert: 60% None, 25% Sandstorm → roll in [0.60, 0.85) should be Sandstorm
    let kind = BiomeWeatherMap::pick(BiomeType::Desert, 0.75);
    assert_eq!(kind, WeatherKind::Sandstorm);
}

// ═════════════════════════════════════════════════════════════════════════
// BiomeWindProfile integration tests
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn wind_profile_all_biomes_positive_strength() {
    for &biome in BiomeType::all() {
        let profile = BiomeWindProfile::for_biome(biome);
        assert!(
            profile.base_strength >= 0.0,
            "{:?} has negative base_strength",
            biome
        );
    }
}

#[test]
fn wind_profile_mountain_strongest() {
    let mountain = BiomeWindProfile::for_biome(BiomeType::Mountain);
    for &biome in BiomeType::all() {
        let other = BiomeWindProfile::for_biome(biome);
        assert!(
            mountain.base_strength >= other.base_strength,
            "Mountain ({}) should be >= {:?} ({})",
            mountain.base_strength,
            biome,
            other.base_strength
        );
    }
}

#[test]
fn wind_profile_effective_strength_non_negative() {
    for &biome in BiomeType::all() {
        let profile = BiomeWindProfile::for_biome(biome);
        for i in 0..20 {
            let s = profile.effective_strength(i as f32 * 0.5);
            assert!(
                s >= 0.0,
                "{:?} at t={} has negative strength {}",
                biome,
                i,
                s
            );
        }
    }
}

#[test]
fn wind_profile_effective_direction_normalized() {
    for &biome in BiomeType::all() {
        let profile = BiomeWindProfile::for_biome(biome);
        for t in [0.0_f32, 1.0, 5.0, 10.0, 20.0] {
            let dir = profile.effective_direction(t);
            let len = dir.length();
            assert!(
                (len - 1.0).abs() < 0.02,
                "{:?} direction not normalized at t={}: len={}",
                biome,
                t,
                len
            );
        }
    }
}

#[test]
fn wind_profile_gusty_varies_strength() {
    let mountain = BiomeWindProfile::for_biome(BiomeType::Mountain);
    assert!(mountain.gusty);
    let mut min_s = f32::MAX;
    let mut max_s = f32::MIN;
    for i in 0..100 {
        let s = mountain.effective_strength(i as f32 * 0.1);
        min_s = min_s.min(s);
        max_s = max_s.max(s);
    }
    assert!(
        max_s > min_s + 0.1,
        "Gusty wind should vary strength: min={min_s}, max={max_s}"
    );
}

#[test]
fn wind_profile_calm_constant_direction() {
    let swamp = BiomeWindProfile::for_biome(BiomeType::Swamp);
    assert!(!swamp.gusty);
    let d0 = swamp.effective_direction(0.0);
    let d1 = swamp.effective_direction(100.0);
    assert!(
        (d0 - d1).length() < 0.001,
        "Non-gusty biome should have stable direction"
    );
}
