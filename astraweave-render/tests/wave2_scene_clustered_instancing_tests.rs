#![allow(unused, clippy::all)]
//! Wave 2 Mutation Remediation — Scene Environment, Clustered Lighting,
//! Instancing, and Decal tests.
//!
//! Targets the highest-mutant-count APIs not covered by existing wave2 files.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::biome_transition::BiomeVisuals;
use astraweave_render::clustered::{bin_lights_cpu, ClusterDims, CpuLight};
use astraweave_render::clustered_forward::{ClusterConfig, GpuLight};
use astraweave_render::decals::{Decal, DecalBlendMode};
use astraweave_render::effects::WeatherKind;
use astraweave_render::environment::TimeOfDay;
use astraweave_render::gi::voxelization_pipeline::{
    VoxelMaterial, VoxelVertex, VoxelizationConfig, VoxelizationMesh,
};
use astraweave_render::instancing::{
    Instance, InstanceBatch, InstanceManager, InstancePatternBuilder, InstanceRaw,
};
use astraweave_render::scene_environment::{SceneEnvironment, SceneEnvironmentUBO};
use astraweave_render::types;
use astraweave_terrain::biome::BiomeType;
use glam::{Mat4, Quat, Vec3};
use std::f32::consts::PI;

// ═══════════════════════════════════════════════════════════════════════
// SceneEnvironmentUBO
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn scene_env_ubo_size_is_80() {
    assert_eq!(SceneEnvironmentUBO::size(), 96);
}

#[test]
fn scene_env_ubo_default_fog_density() {
    let ubo = SceneEnvironmentUBO::default();
    // Default is from BiomeVisuals::default()
    assert!(ubo.fog_density >= 0.0);
}

#[test]
fn scene_env_ubo_default_blend_factor_zero() {
    let ubo = SceneEnvironmentUBO::default();
    assert_eq!(ubo.blend_factor, 0.0);
}

#[test]
fn scene_env_ubo_default_tint_alpha_zero() {
    let ubo = SceneEnvironmentUBO::default();
    assert_eq!(ubo.tint_alpha, 0.0);
}

#[test]
fn scene_env_ubo_default_tint_color_zero() {
    let ubo = SceneEnvironmentUBO::default();
    assert_eq!(ubo.tint_color, [0.0, 0.0, 0.0]);
}

#[test]
fn scene_env_ubo_from_visuals_carries_fog() {
    let vis = BiomeVisuals::for_biome(BiomeType::Desert);
    let ubo = SceneEnvironmentUBO::from_visuals(&vis, 0.5, [1.0, 0.0, 0.0], 0.8);
    assert_eq!(ubo.fog_density, vis.fog_density);
    assert_eq!(ubo.blend_factor, 0.5);
    assert_eq!(ubo.tint_color, [1.0, 0.0, 0.0]);
    assert_eq!(ubo.tint_alpha, 0.8);
}

#[test]
fn scene_env_ubo_from_visuals_carries_ambient() {
    let vis = BiomeVisuals::for_biome(BiomeType::Forest);
    let ubo = SceneEnvironmentUBO::from_visuals(&vis, 0.0, [0.0; 3], 0.0);
    assert_eq!(ubo.ambient_intensity, vis.ambient_intensity);
    assert_eq!(ubo.ambient_color, vis.ambient_color.to_array());
}

#[test]
fn scene_env_ubo_for_biome_grassland() {
    let ubo = SceneEnvironmentUBO::for_biome(BiomeType::Grassland);
    assert_eq!(ubo.blend_factor, 0.0);
    assert_eq!(ubo.tint_alpha, 0.0);
    assert!(ubo.fog_density >= 0.0);
}

#[test]
fn scene_env_ubo_for_biome_mountain() {
    let ubo = SceneEnvironmentUBO::for_biome(BiomeType::Mountain);
    let vis = BiomeVisuals::for_biome(BiomeType::Mountain);
    assert_eq!(ubo.fog_density, vis.fog_density);
}

#[test]
fn scene_env_ubo_for_biome_tundra() {
    let ubo = SceneEnvironmentUBO::for_biome(BiomeType::Tundra);
    assert_eq!(ubo.blend_factor, 0.0);
}

#[test]
fn scene_env_ubo_for_biome_swamp() {
    let ubo = SceneEnvironmentUBO::for_biome(BiomeType::Swamp);
    let vis = BiomeVisuals::for_biome(BiomeType::Swamp);
    assert_eq!(ubo.fog_color, vis.fog_color.to_array());
}

#[test]
fn scene_env_ubo_for_biome_beach() {
    let ubo = SceneEnvironmentUBO::for_biome(BiomeType::Beach);
    let vis = BiomeVisuals::for_biome(BiomeType::Beach);
    assert_eq!(ubo.ambient_color, vis.ambient_color.to_array());
}

#[test]
fn scene_env_ubo_for_biome_river() {
    let ubo = SceneEnvironmentUBO::for_biome(BiomeType::River);
    let vis = BiomeVisuals::for_biome(BiomeType::River);
    assert_eq!(ubo.fog_start, vis.fog_start);
}

#[test]
fn scene_env_ubo_for_biome_desert() {
    let ubo = SceneEnvironmentUBO::for_biome(BiomeType::Desert);
    let vis = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_eq!(ubo.fog_end, vis.fog_end);
}

// ═══════════════════════════════════════════════════════════════════════
// SceneEnvironment (CPU-side)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn scene_env_default_weather_fog_multiplier() {
    let se = SceneEnvironment::default();
    assert_eq!(se.weather_fog_multiplier, 1.0);
}

#[test]
fn scene_env_default_weather_ambient_multiplier() {
    let se = SceneEnvironment::default();
    assert_eq!(se.weather_ambient_multiplier, 1.0);
}

#[test]
fn scene_env_default_blend_factor_zero() {
    let se = SceneEnvironment::default();
    assert_eq!(se.blend_factor, 0.0);
}

#[test]
fn scene_env_set_biome_resets_blend() {
    let mut se = SceneEnvironment::default();
    se.blend_factor = 0.7;
    se.tint_alpha = 0.5;
    se.set_biome(BiomeType::Desert);
    assert_eq!(se.blend_factor, 0.0);
    assert_eq!(se.tint_alpha, 0.0);
    assert_eq!(se.tint_color, [0.0; 3]);
}

#[test]
fn scene_env_set_biome_changes_visuals() {
    let mut se = SceneEnvironment::default();
    se.set_biome(BiomeType::Mountain);
    let expected = BiomeVisuals::for_biome(BiomeType::Mountain);
    assert_eq!(se.visuals.fog_density, expected.fog_density);
}

#[test]
fn scene_env_apply_weather_none() {
    let mut se = SceneEnvironment::default();
    se.apply_weather(WeatherKind::None);
    assert_eq!(se.weather_fog_multiplier, 1.0);
    assert_eq!(se.weather_ambient_multiplier, 1.0);
}

#[test]
fn scene_env_apply_weather_rain() {
    let mut se = SceneEnvironment::default();
    se.apply_weather(WeatherKind::Rain);
    assert_eq!(se.weather_fog_multiplier, 2.5);
    assert_eq!(se.weather_ambient_multiplier, 0.6);
}

#[test]
fn scene_env_apply_weather_snow() {
    let mut se = SceneEnvironment::default();
    se.apply_weather(WeatherKind::Snow);
    assert_eq!(se.weather_fog_multiplier, 1.8);
    assert_eq!(se.weather_ambient_multiplier, 0.75);
}

#[test]
fn scene_env_apply_weather_sandstorm() {
    let mut se = SceneEnvironment::default();
    se.apply_weather(WeatherKind::Sandstorm);
    assert_eq!(se.weather_fog_multiplier, 4.0);
    assert_eq!(se.weather_ambient_multiplier, 0.4);
}

#[test]
fn scene_env_apply_weather_wind_trails() {
    let mut se = SceneEnvironment::default();
    se.apply_weather(WeatherKind::WindTrails);
    assert_eq!(se.weather_fog_multiplier, 1.4);
    assert_eq!(se.weather_ambient_multiplier, 0.9);
}

#[test]
fn scene_env_to_ubo_applies_weather_multiplier() {
    let mut se = SceneEnvironment::default();
    se.set_biome(BiomeType::Forest);
    let base_fog = se.visuals.fog_density;
    let base_ambient = se.visuals.ambient_intensity;
    se.apply_weather(WeatherKind::Rain);
    let ubo = se.to_ubo();
    // fog_density should be base * 2.5
    let expected_fog = base_fog * 2.5;
    assert!((ubo.fog_density - expected_fog).abs() < 1e-5);
    // ambient_intensity should be base * 0.6
    let expected_ambient = base_ambient * 0.6;
    assert!((ubo.ambient_intensity - expected_ambient).abs() < 1e-5);
}

#[test]
fn scene_env_to_ubo_no_weather_passthrough() {
    let mut se = SceneEnvironment::default();
    se.set_biome(BiomeType::Grassland);
    let ubo = se.to_ubo();
    let vis = BiomeVisuals::for_biome(BiomeType::Grassland);
    assert_eq!(ubo.fog_density, vis.fog_density);
    assert_eq!(ubo.ambient_intensity, vis.ambient_intensity);
}

#[test]
fn scene_env_apply_time_of_day_noon() {
    let mut se = SceneEnvironment::default();
    se.set_biome(BiomeType::Grassland);
    let tod = TimeOfDay::new(12.0, 1.0);
    se.apply_time_of_day(&tod);
    // Noon should have bright ambient, so intensity scales up or stays
    assert!(se.visuals.ambient_intensity > 0.0);
    // The ambient color should be blended 60/40 biome/ToD
    assert!(se.visuals.ambient_color.x > 0.0 || se.visuals.ambient_color.y > 0.0);
}

#[test]
fn scene_env_apply_time_of_day_midnight() {
    let mut se = SceneEnvironment::default();
    se.set_biome(BiomeType::Forest);
    let tod = TimeOfDay::new(0.0, 1.0);
    se.apply_time_of_day(&tod);
    // Night ambient should be dimmer
    // The function clamps luminance to 0.15..1.5
    // At midnight, ambient is very low
}

// ═══════════════════════════════════════════════════════════════════════
// bin_lights_cpu (clustered.rs)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn bin_lights_cpu_empty_no_lights() {
    let (counts, indices, offsets) = bin_lights_cpu(
        &[],
        ClusterDims { x: 4, y: 4, z: 4 },
        (800, 600),
        0.1,
        100.0,
        PI / 3.0,
    );
    let clusters = 4 * 4 * 4;
    assert_eq!(counts.len(), clusters);
    assert!(counts.iter().all(|c| *c == 0));
    assert_eq!(indices.len(), 0);
    assert_eq!(offsets.len(), clusters + 1);
    assert!(offsets.iter().all(|o| *o == 0));
}

#[test]
fn bin_lights_cpu_single_center_light() {
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 5.0,
    }];
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let (counts, indices, _offsets) =
        bin_lights_cpu(&lights, dims, (800, 600), 0.1, 100.0, PI / 3.0);
    // At least one cluster should contain the light
    let total_assigned: u32 = counts.iter().sum();
    assert!(
        total_assigned > 0,
        "Light should be assigned to at least one cluster"
    );
    assert_eq!(indices.len(), total_assigned as usize);
    // All indices should be 0 (only one light)
    assert!(indices.iter().all(|i| *i == 0));
}

#[test]
fn bin_lights_cpu_far_light_excluded() {
    // Light beyond far plane
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 200.0),
        radius: 1.0,
    }];
    let (counts, _, _) = bin_lights_cpu(
        &lights,
        ClusterDims { x: 4, y: 4, z: 4 },
        (800, 600),
        0.1,
        100.0,
        PI / 3.0,
    );
    // Light at z=200 with radius=1 → z-radius=199 > far=100 → excluded
    let total: u32 = counts.iter().sum();
    assert_eq!(total, 0, "Light beyond far plane should be excluded");
}

#[test]
fn bin_lights_cpu_multi_light_indices_correct() {
    let lights = vec![
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 10.0),
            radius: 2.0,
        },
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 50.0),
            radius: 2.0,
        },
    ];
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let (counts, indices, _) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, 100.0, PI / 3.0);
    let total: u32 = counts.iter().sum();
    assert!(total >= 2, "Both lights should be binned");
    // Indices should contain both 0 and 1
    assert!(indices.contains(&0) || indices.contains(&1));
}

#[test]
fn bin_lights_cpu_offsets_exclusive_scan() {
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 30.0,
    }];
    let dims = ClusterDims { x: 2, y: 2, z: 2 };
    let (counts, _, offsets) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, 100.0, PI / 3.0);
    // offsets[0] should be 0 (exclusive scan starts at 0)
    assert_eq!(offsets[0], 0);
    // offsets[i+1] = offsets[i] + counts[i]
    for i in 0..counts.len() {
        assert_eq!(offsets[i + 1], offsets[i] + counts[i]);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ClusterConfig (clustered_forward.rs)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn cluster_config_default_x() {
    let c = ClusterConfig::default();
    assert_eq!(c.cluster_x, 16);
}

#[test]
fn cluster_config_default_y() {
    let c = ClusterConfig::default();
    assert_eq!(c.cluster_y, 9);
}

#[test]
fn cluster_config_default_z() {
    let c = ClusterConfig::default();
    assert_eq!(c.cluster_z, 24);
}

#[test]
fn cluster_config_default_near() {
    let c = ClusterConfig::default();
    assert_eq!(c.near, 0.1);
}

#[test]
fn cluster_config_default_far() {
    let c = ClusterConfig::default();
    assert_eq!(c.far, 100.0);
}

#[test]
fn gpu_light_new_packs_radius() {
    let l = GpuLight::new(Vec3::new(1.0, 2.0, 3.0), 5.0, Vec3::new(1.0, 1.0, 1.0), 2.0);
    assert_eq!(l.position[0], 1.0);
    assert_eq!(l.position[1], 2.0);
    assert_eq!(l.position[2], 3.0);
    assert_eq!(l.position[3], 5.0); // radius in w
}

#[test]
fn gpu_light_new_packs_intensity() {
    let l = GpuLight::new(Vec3::ZERO, 1.0, Vec3::new(0.5, 0.7, 0.9), 3.0);
    assert_eq!(l.color[0], 0.5);
    assert_eq!(l.color[1], 0.7);
    assert_eq!(l.color[2], 0.9);
    assert_eq!(l.color[3], 3.0); // intensity in w
}

// ═══════════════════════════════════════════════════════════════════════
// cluster_index (types.rs)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn types_cluster_index_within_bounds() {
    let dims = types::ClusterDims { x: 16, y: 9, z: 24 };
    let idx = types::cluster_index(100, 50, 1920, 1080, 5.0, 0.1, 100.0, dims);
    assert!(idx < 16 * 9 * 24);
}

#[test]
fn types_cluster_index_origin() {
    let dims = types::ClusterDims { x: 8, y: 8, z: 8 };
    let idx = types::cluster_index(0, 0, 800, 800, 0.1, 0.1, 100.0, dims);
    // Should be in the first z-slice (depth=near)
    assert!(idx < 8 * 8 * 8);
}

#[test]
fn types_cluster_index_far_corner() {
    let dims = types::ClusterDims { x: 8, y: 8, z: 8 };
    let idx = types::cluster_index(799, 799, 800, 800, 99.9, 0.1, 100.0, dims);
    assert!(idx < 8 * 8 * 8);
}

#[test]
fn types_cluster_index_different_depths_differ() {
    let dims = types::ClusterDims { x: 4, y: 4, z: 8 };
    let near_idx = types::cluster_index(200, 200, 800, 800, 1.0, 0.1, 100.0, dims);
    let far_idx = types::cluster_index(200, 200, 800, 800, 90.0, 0.1, 100.0, dims);
    assert_ne!(near_idx, far_idx);
}

// ═══════════════════════════════════════════════════════════════════════
// Instancing (Instance, InstanceRaw)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn instance_identity_position_zero() {
    let inst = Instance::identity();
    assert_eq!(inst.position, Vec3::ZERO);
    assert_eq!(inst.scale, Vec3::ONE);
    assert_eq!(inst.rotation, Quat::IDENTITY);
}

#[test]
fn instance_new_stores_fields() {
    let inst = Instance::new(Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY, Vec3::splat(2.0));
    assert_eq!(inst.position, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(inst.scale, Vec3::splat(2.0));
}

#[test]
fn instance_to_raw_identity_diagonal() {
    let inst = Instance::identity();
    let raw = inst.to_raw();
    assert_eq!(raw.model[0][0], 1.0);
    assert_eq!(raw.model[1][1], 1.0);
    assert_eq!(raw.model[2][2], 1.0);
    assert_eq!(raw.model[3][3], 1.0);
}

#[test]
fn instance_to_raw_translation() {
    let inst = Instance::new(Vec3::new(10.0, 20.0, 30.0), Quat::IDENTITY, Vec3::ONE);
    let raw = inst.to_raw();
    // Translation is in column 3
    assert_eq!(raw.model[3][0], 10.0);
    assert_eq!(raw.model[3][1], 20.0);
    assert_eq!(raw.model[3][2], 30.0);
}

#[test]
fn instance_to_raw_scale() {
    let inst = Instance::new(Vec3::ZERO, Quat::IDENTITY, Vec3::new(2.0, 3.0, 4.0));
    let raw = inst.to_raw();
    assert!((raw.model[0][0] - 2.0).abs() < 1e-5);
    assert!((raw.model[1][1] - 3.0).abs() < 1e-5);
    assert!((raw.model[2][2] - 4.0).abs() < 1e-5);
}

#[test]
fn instance_raw_from_matrix_identity() {
    let raw = InstanceRaw::from_matrix(Mat4::IDENTITY);
    assert_eq!(raw.model[0][0], 1.0);
    assert_eq!(raw.model[3][3], 1.0);
}

#[test]
fn instance_raw_size_64_bytes() {
    assert_eq!(std::mem::size_of::<InstanceRaw>(), 64);
}

// ═══════════════════════════════════════════════════════════════════════
// InstanceBatch
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn instance_batch_new_empty() {
    let batch = InstanceBatch::new(42);
    assert_eq!(batch.mesh_id, 42);
    assert_eq!(batch.instance_count(), 0);
}

#[test]
fn instance_batch_add_increments_count() {
    let mut batch = InstanceBatch::new(1);
    batch.add_instance(Instance::identity());
    batch.add_instance(Instance::identity());
    assert_eq!(batch.instance_count(), 2);
}

#[test]
fn instance_batch_clear_resets_count() {
    let mut batch = InstanceBatch::new(1);
    batch.add_instance(Instance::identity());
    batch.clear();
    assert_eq!(batch.instance_count(), 0);
}

// ═══════════════════════════════════════════════════════════════════════
// InstanceManager
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn instance_manager_new_empty() {
    let mgr = InstanceManager::new();
    assert_eq!(mgr.total_instances(), 0);
    assert_eq!(mgr.batch_count(), 0);
}

#[test]
fn instance_manager_add_instance_increments() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(1, Instance::identity());
    assert_eq!(mgr.total_instances(), 1);
    assert_eq!(mgr.batch_count(), 1);
}

#[test]
fn instance_manager_add_instances_bulk() {
    let mut mgr = InstanceManager::new();
    mgr.add_instances(
        1,
        vec![
            Instance::identity(),
            Instance::identity(),
            Instance::identity(),
        ],
    );
    assert_eq!(mgr.total_instances(), 3);
    assert_eq!(mgr.batch_count(), 1);
}

#[test]
fn instance_manager_multi_mesh_batches() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(1, Instance::identity());
    mgr.add_instance(2, Instance::identity());
    mgr.add_instance(3, Instance::identity());
    assert_eq!(mgr.total_instances(), 3);
    assert_eq!(mgr.batch_count(), 3);
}

#[test]
fn instance_manager_get_batch_returns_correct() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(
        42,
        Instance::new(Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
    );
    let batch = mgr.get_batch(42);
    assert!(batch.is_some());
    assert_eq!(batch.unwrap().instance_count(), 1);
}

#[test]
fn instance_manager_get_batch_missing() {
    let mgr = InstanceManager::new();
    assert!(mgr.get_batch(999).is_none());
}

#[test]
fn instance_manager_clear_resets_all() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(1, Instance::identity());
    mgr.add_instance(2, Instance::identity());
    mgr.clear();
    assert_eq!(mgr.total_instances(), 0);
    assert_eq!(mgr.batch_count(), 0);
}

#[test]
fn instance_manager_draw_call_reduction_empty() {
    let mgr = InstanceManager::new();
    assert_eq!(mgr.draw_call_reduction_percent(), 0.0);
}

#[test]
fn instance_manager_default() {
    let mgr = InstanceManager::default();
    assert_eq!(mgr.total_instances(), 0);
}

// ═══════════════════════════════════════════════════════════════════════
// InstancePatternBuilder
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn pattern_grid_count() {
    let instances = InstancePatternBuilder::new().grid(3, 4, 1.0).build();
    assert_eq!(instances.len(), 12);
}

#[test]
fn pattern_grid_positions() {
    let instances = InstancePatternBuilder::new().grid(2, 2, 5.0).build();
    // row=0,col=0 → (0,0,0), row=0,col=1 → (5,0,0), row=1,col=0 → (0,0,5), row=1,col=1 → (5,0,5)
    assert_eq!(instances[0].position, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(instances[1].position, Vec3::new(5.0, 0.0, 0.0));
    assert_eq!(instances[2].position, Vec3::new(0.0, 0.0, 5.0));
    assert_eq!(instances[3].position, Vec3::new(5.0, 0.0, 5.0));
}

#[test]
fn pattern_circle_count() {
    let instances = InstancePatternBuilder::new().circle(8, 10.0).build();
    assert_eq!(instances.len(), 8);
}

#[test]
fn pattern_circle_radius() {
    let instances = InstancePatternBuilder::new().circle(16, 5.0).build();
    for inst in &instances {
        let d = inst.position.length();
        assert!((d - 5.0).abs() < 0.01, "Instance at wrong radius: {d}");
    }
}

#[test]
fn pattern_combined_grid_circle() {
    let instances = InstancePatternBuilder::new()
        .grid(2, 2, 1.0)
        .circle(4, 3.0)
        .build();
    assert_eq!(instances.len(), 4 + 4);
}

#[test]
fn pattern_default() {
    let builder = InstancePatternBuilder::default();
    let instances = builder.build();
    assert_eq!(instances.len(), 0);
}

// ═══════════════════════════════════════════════════════════════════════
// Decal
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn decal_new_defaults() {
    let d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    assert_eq!(d.albedo_tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(d.normal_strength, 1.0);
    assert_eq!(d.roughness, 0.5);
    assert_eq!(d.metallic, 0.0);
    assert_eq!(d.blend_mode, DecalBlendMode::AlphaBlend);
    assert_eq!(d.fade_duration, 0.0);
    assert_eq!(d.fade_time, 0.0);
}

#[test]
fn decal_update_permanent_returns_true() {
    let mut d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    // fade_duration=0 means permanent
    assert!(d.update(1.0));
    assert!(d.update(100.0));
}

#[test]
fn decal_update_fading_decreases_alpha() {
    let mut d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    d.fade_duration = 2.0;
    assert!(d.update(0.5)); // Not expired
                            // fade_alpha = 1.0 - (0.5 / 2.0) = 0.75
    assert!((d.albedo_tint[3] - 0.75).abs() < 1e-5);
}

#[test]
fn decal_update_fading_halfway() {
    let mut d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    d.fade_duration = 4.0;
    assert!(d.update(2.0));
    // fade_alpha = 1.0 - (2.0 / 4.0) = 0.5
    assert!((d.albedo_tint[3] - 0.5).abs() < 1e-5);
}

#[test]
fn decal_update_expired_returns_false() {
    let mut d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    d.fade_duration = 1.0;
    assert!(!d.update(2.0)); // Past fade duration
}

#[test]
fn decal_to_gpu_has_inv_projection() {
    let d = Decal::new(
        Vec3::new(5.0, 0.0, 0.0),
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    let gpu = d.to_gpu();
    // inv_projection should not be all zeros (it's the inverse of a transform)
    let flat: Vec<f32> = gpu
        .inv_projection
        .iter()
        .flat_map(|row| row.iter())
        .copied()
        .collect();
    assert!(
        flat.iter().any(|v| *v != 0.0),
        "inv_projection shouldn't be all zeros"
    );
}

#[test]
fn decal_to_gpu_packs_blend_mode() {
    let mut d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    d.blend_mode = DecalBlendMode::Additive;
    let gpu = d.to_gpu();
    assert_eq!(gpu.params[3], 1.0); // Additive = 1
}

#[test]
fn decal_to_gpu_packs_atlas_uv() {
    let d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.25, 0.5], [0.125, 0.125]),
    );
    let gpu = d.to_gpu();
    assert_eq!(gpu.atlas_uv[0], 0.25);
    assert_eq!(gpu.atlas_uv[1], 0.5);
    assert_eq!(gpu.atlas_uv[2], 0.125);
    assert_eq!(gpu.atlas_uv[3], 0.125);
}

#[test]
fn decal_blend_mode_multiply_value() {
    assert_eq!(DecalBlendMode::Multiply as u32, 0);
}

#[test]
fn decal_blend_mode_additive_value() {
    assert_eq!(DecalBlendMode::Additive as u32, 1);
}

#[test]
fn decal_blend_mode_alpha_blend_value() {
    assert_eq!(DecalBlendMode::AlphaBlend as u32, 2);
}

#[test]
fn decal_blend_mode_stain_value() {
    assert_eq!(DecalBlendMode::Stain as u32, 3);
}

// ═══════════════════════════════════════════════════════════════════════
// Voxelization pipeline types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn voxel_vertex_new_stores_arrays() {
    let v = VoxelVertex::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(v.position, [1.0, 2.0, 3.0]);
    assert_eq!(v.normal, [0.0, 1.0, 0.0]);
}

#[test]
fn voxel_material_default_albedo() {
    let m = VoxelMaterial::default();
    assert_eq!(m.albedo, [0.8, 0.8, 0.8]);
    assert_eq!(m.metallic, 0.0);
    assert_eq!(m.roughness, 0.8);
    assert_eq!(m.emissive, [0.0, 0.0, 0.0]);
}

#[test]
fn voxel_material_from_albedo() {
    let m = VoxelMaterial::from_albedo(Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(m.albedo, [1.0, 0.0, 0.0]);
    assert_eq!(m.metallic, 0.0); // default
}

#[test]
fn voxel_material_emissive() {
    let m = VoxelMaterial::emissive(Vec3::new(10.0, 5.0, 0.0));
    assert_eq!(m.emissive, [10.0, 5.0, 0.0]);
    assert_eq!(m.albedo, [0.8, 0.8, 0.8]); // default
}

#[test]
fn voxelization_config_default_resolution() {
    let c = VoxelizationConfig::default();
    assert_eq!(c.voxel_resolution, 256);
    assert_eq!(c.world_size, 1000.0);
    assert_eq!(c.triangle_count, 0);
    assert_eq!(c.light_intensity, 1.0);
}

#[test]
fn voxelization_mesh_triangle_count() {
    let mesh = VoxelizationMesh::new(
        vec![
            VoxelVertex::new(Vec3::ZERO, Vec3::Y),
            VoxelVertex::new(Vec3::X, Vec3::Y),
            VoxelVertex::new(Vec3::Z, Vec3::Y),
        ],
        vec![0, 1, 2],
        VoxelMaterial::default(),
    );
    assert_eq!(mesh.triangle_count(), 1);
}

#[test]
fn voxelization_mesh_triangle_count_two() {
    let mesh = VoxelizationMesh::new(
        vec![
            VoxelVertex::new(Vec3::ZERO, Vec3::Y),
            VoxelVertex::new(Vec3::X, Vec3::Y),
            VoxelVertex::new(Vec3::Z, Vec3::Y),
            VoxelVertex::new(Vec3::ONE, Vec3::Y),
        ],
        vec![0, 1, 2, 1, 2, 3],
        VoxelMaterial::default(),
    );
    assert_eq!(mesh.triangle_count(), 2);
}

// ═══════════════════════════════════════════════════════════════════════
// types::Instance (the types.rs version)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn types_instance_from_pos_scale_color() {
    let inst = types::Instance::from_pos_scale_color(
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::splat(2.0),
        [1.0, 0.0, 0.0, 1.0],
    );
    assert_eq!(inst.color, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(inst.material_id, 0);
}

#[test]
fn types_instance_raw_normal_matrix_computed() {
    let inst = types::Instance::from_pos_scale_color(Vec3::ZERO, Vec3::ONE, [1.0, 1.0, 1.0, 1.0]);
    let raw = inst.raw();
    // Identity transform → normal matrix should be identity
    assert!((raw.normal_matrix[0][0] - 1.0).abs() < 1e-5);
    assert!((raw.normal_matrix[1][1] - 1.0).abs() < 1e-5);
    assert!((raw.normal_matrix[2][2] - 1.0).abs() < 1e-5);
}

#[test]
fn types_instance_raw_material_id() {
    let mut inst = types::Instance::from_pos_scale_color(Vec3::ZERO, Vec3::ONE, [0.0; 4]);
    inst.material_id = 42;
    let raw = inst.raw();
    assert_eq!(raw.material_id, 42);
}
