//! Wave 2 Mutation Remediation Tests — culling, animation, vertex_compression,
//! lod_generator, transparency, gpu_memory, water, hdri_catalog, biome_detector, ibl
//!
//! Pins exact numeric constants and exercises pure-computation functions
//! that don't require GPU access.

use astraweave_render::animation::{
    skin_vertex_cpu, AnimationState, JointPalette, Transform, MAX_JOINTS,
};
use astraweave_render::biome_detector::BiomeDetectorConfig;
use astraweave_render::culling::{
    cpu_frustum_cull, BatchId, DrawIndirectCommand, FrustumPlanes, InstanceAABB,
};
use astraweave_render::gpu_memory::{GpuMemoryBudget, MemoryCategory};
use astraweave_render::hdri_catalog::DayPeriod;
use astraweave_render::lod_generator::LODConfig;
use astraweave_render::transparency::{create_blend_state, BlendMode, TransparencyManager};
use astraweave_render::vertex_compression::{
    CompressedVertex, HalfFloatEncoder, OctahedralEncoder,
};
use astraweave_render::water::WaterUniforms;
use glam::{Mat4, Quat, Vec3};

// ═══════════════════════════════════════════════════════════════════════
// Culling — DrawIndirectCommand defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn draw_indirect_command_default_all_zero() {
    let c = DrawIndirectCommand::default();
    assert_eq!(c.vertex_count, 0);
    assert_eq!(c.instance_count, 0);
    assert_eq!(c.first_vertex, 0);
    assert_eq!(c.first_instance, 0);
}
#[test]
fn draw_indirect_command_new() {
    let c = DrawIndirectCommand::new(36, 1, 0, 0);
    assert_eq!(c.vertex_count, 36);
    assert_eq!(c.instance_count, 1);
}
#[test]
fn draw_indirect_command_size() {
    assert_eq!(std::mem::size_of::<DrawIndirectCommand>(), 16);
}

// ═══════════════════════════════════════════════════════════════════════
// Culling — InstanceAABB
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn instance_aabb_size() {
    assert_eq!(std::mem::size_of::<InstanceAABB>(), 32);
}
#[test]
fn instance_aabb_new_preserves_fields() {
    let a = InstanceAABB::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0), 42);
    assert_eq!(a.instance_index, 42);
}

// ═══════════════════════════════════════════════════════════════════════
// Culling — FrustumPlanes and cpu_frustum_cull
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn frustum_planes_size() {
    assert_eq!(std::mem::size_of::<FrustumPlanes>(), 96);
}
#[test]
fn cpu_frustum_cull_empty_instances() {
    let vp = Mat4::perspective_rh(1.0, 1.0, 0.1, 100.0)
        * Mat4::look_to_rh(Vec3::ZERO, -Vec3::Z, Vec3::Y);
    let frustum = FrustumPlanes::from_view_proj(&vp);
    let instances: Vec<InstanceAABB> = vec![];
    let visible = cpu_frustum_cull(&instances, &frustum);
    assert!(visible.is_empty());
}
#[test]
fn cpu_frustum_cull_origin_box_visible() {
    let vp = Mat4::perspective_rh(1.0, 1.0, 0.1, 100.0)
        * Mat4::look_to_rh(Vec3::new(0.0, 0.0, 5.0), -Vec3::Z, Vec3::Y);
    let frustum = FrustumPlanes::from_view_proj(&vp);
    let instances = vec![InstanceAABB::new(Vec3::ZERO, Vec3::ONE, 0)];
    let visible = cpu_frustum_cull(&instances, &frustum);
    assert!(visible.contains(&0), "Origin box should be visible");
}
#[test]
fn cpu_frustum_cull_far_box_culled() {
    let vp =
        Mat4::perspective_rh(1.0, 1.0, 0.1, 10.0) * Mat4::look_to_rh(Vec3::ZERO, -Vec3::Z, Vec3::Y);
    let frustum = FrustumPlanes::from_view_proj(&vp);
    let instances = vec![InstanceAABB::new(Vec3::new(0.0, 0.0, -100.0), Vec3::ONE, 0)];
    let visible = cpu_frustum_cull(&instances, &frustum);
    assert!(!visible.contains(&0), "Far box should be culled");
}

// ═══════════════════════════════════════════════════════════════════════
// Culling — BatchId ordering
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn batch_id_ordering() {
    let a = BatchId::new(0, 0);
    let b = BatchId::new(0, 1);
    let c = BatchId::new(1, 0);
    assert!(a < b);
    assert!(b < c);
}

// ═══════════════════════════════════════════════════════════════════════
// Animation constants and Transform
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn max_joints_is_256() {
    assert_eq!(MAX_JOINTS, 256);
}
#[test]
fn transform_default_identity() {
    let t = Transform::default();
    assert_eq!(t.translation, Vec3::ZERO);
    assert_eq!(t.rotation, Quat::IDENTITY);
    assert_eq!(t.scale, Vec3::ONE);
}
#[test]
fn transform_to_matrix_identity() {
    let t = Transform::default();
    let m = t.to_matrix();
    assert!((m - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 1e-6));
}
#[test]
fn transform_to_matrix_translation() {
    let t = Transform {
        translation: Vec3::new(1.0, 2.0, 3.0),
        ..Default::default()
    };
    let m = t.to_matrix();
    // Column 3 should contain the translation
    assert!((m.col(3).truncate() - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-6);
}
#[test]
fn transform_lerp_midpoint() {
    let a = Transform {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };
    let b = Transform {
        translation: Vec3::new(2.0, 4.0, 6.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::splat(3.0),
    };
    let mid = a.lerp(&b, 0.5);
    assert!((mid.translation - Vec3::new(1.0, 2.0, 3.0)).length() < 0.01);
    assert!((mid.scale - Vec3::splat(2.0)).length() < 0.01);
}

// ═══════════════════════════════════════════════════════════════════════
// AnimationState
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn animation_state_defaults() {
    let s = AnimationState::default();
    assert_eq!(s.clip_index, 0);
    assert_eq!(s.time, 0.0);
    assert_eq!(s.speed, 1.0);
    assert!(s.looping);
    assert!(!s.playing);
}
#[test]
fn animation_state_update_loops() {
    let mut s = AnimationState {
        playing: true,
        looping: true,
        speed: 1.0,
        time: 0.9,
        ..Default::default()
    };
    s.update(0.2, 1.0); // 0.9 + 0.2 = 1.1 → wraps to 0.1
    assert!(s.time < 0.5, "Should wrap: {}", s.time);
}
#[test]
fn animation_state_update_clamps_non_looping() {
    let mut s = AnimationState {
        playing: true,
        looping: false,
        speed: 1.0,
        time: 0.9,
        ..Default::default()
    };
    s.update(0.2, 1.0);
    assert!((s.time - 1.0).abs() < 0.01);
    assert!(!s.playing); // Should stop at end
}

// ═══════════════════════════════════════════════════════════════════════
// JointPalette
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn joint_palette_default_joint_count() {
    assert_eq!(JointPalette::default().joint_count, 0);
}
#[test]
fn joint_palette_from_matrices_count() {
    let mats = vec![Mat4::IDENTITY, Mat4::IDENTITY, Mat4::IDENTITY];
    let p = JointPalette::from_matrices(&mats);
    assert_eq!(p.joint_count, 3);
}

// ═══════════════════════════════════════════════════════════════════════
// skin_vertex_cpu
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn skin_vertex_identity_no_change() {
    let joints = [Mat4::IDENTITY; 256];
    let (pos, normal) = skin_vertex_cpu(
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(0.0, 1.0, 0.0),
        [0, 0, 0, 0],
        [1.0, 0.0, 0.0, 0.0],
        &joints,
    );
    assert!((pos - Vec3::new(1.0, 2.0, 3.0)).length() < 0.01);
    assert!((normal - Vec3::new(0.0, 1.0, 0.0)).length() < 0.01);
}

// ═══════════════════════════════════════════════════════════════════════
// Vertex compression constants
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn compressed_vertex_standard_size() {
    assert_eq!(CompressedVertex::STANDARD_SIZE, 32);
}
#[test]
fn compressed_vertex_compressed_size() {
    assert_eq!(CompressedVertex::COMPRESSED_SIZE, 20);
}
#[test]
fn compressed_vertex_memory_reduction() {
    assert_eq!(CompressedVertex::MEMORY_REDUCTION, 0.375);
}

// ═══════════════════════════════════════════════════════════════════════
// OctahedralEncoder roundtrip
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn octahedral_encode_decode_roundtrip_x() {
    let n = Vec3::new(1.0, 0.0, 0.0);
    let enc = OctahedralEncoder::encode(n);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec[0] - 1.0).abs() < 0.01);
}
#[test]
fn octahedral_encode_decode_roundtrip_y() {
    let n = Vec3::new(0.0, 1.0, 0.0);
    let enc = OctahedralEncoder::encode(n);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec[1] - 1.0).abs() < 0.01);
}
#[test]
fn octahedral_encode_decode_roundtrip_z() {
    let n = Vec3::new(0.0, 0.0, 1.0);
    let enc = OctahedralEncoder::encode(n);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec[2] - 1.0).abs() < 0.01);
}
#[test]
fn octahedral_encode_decode_roundtrip_negative() {
    let n = Vec3::new(0.0, 0.0, -1.0);
    let enc = OctahedralEncoder::encode(n);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec[2] + 1.0).abs() < 0.01);
}
#[test]
fn octahedral_encoding_error_small() {
    let n = Vec3::new(0.577, 0.577, 0.577);
    let err = OctahedralEncoder::encoding_error(n);
    assert!(err < 0.05, "error too large: {err}");
}

// ═══════════════════════════════════════════════════════════════════════
// HalfFloatEncoder
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn half_float_roundtrip() {
    let val = 0.5_f32;
    let enc = HalfFloatEncoder::encode(val);
    let dec = HalfFloatEncoder::decode(enc);
    assert!((dec - val).abs() < 0.001);
}
#[test]
fn half_float_encode_zero() {
    let enc = HalfFloatEncoder::encode(0.0);
    let dec = HalfFloatEncoder::decode(enc);
    assert_eq!(dec, 0.0);
}

// ═══════════════════════════════════════════════════════════════════════
// LODConfig defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn lod_config_default_reduction_targets() {
    let c = LODConfig::default();
    assert_eq!(c.reduction_targets, [0.75, 0.50, 0.25]);
}
#[test]
fn lod_config_default_max_error() {
    assert_eq!(LODConfig::default().max_error, 0.01);
}
#[test]
fn lod_config_default_preserve_boundaries() {
    assert!(LODConfig::default().preserve_boundaries);
}

// ═══════════════════════════════════════════════════════════════════════
// Transparency — BlendMode & TransparencyManager
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn transparency_manager_starts_empty() {
    let tm = TransparencyManager::new();
    assert_eq!(tm.count(), 0);
}
#[test]
fn blend_state_alpha() {
    let s = create_blend_state(BlendMode::Alpha);
    assert_eq!(s.color.src_factor, wgpu::BlendFactor::SrcAlpha);
    assert_eq!(s.color.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
}
#[test]
fn blend_state_additive() {
    let s = create_blend_state(BlendMode::Additive);
    assert_eq!(s.color.src_factor, wgpu::BlendFactor::SrcAlpha);
    assert_eq!(s.color.dst_factor, wgpu::BlendFactor::One);
}
#[test]
fn blend_state_multiplicative() {
    let s = create_blend_state(BlendMode::Multiplicative);
    assert_eq!(s.color.src_factor, wgpu::BlendFactor::Zero);
    assert_eq!(s.color.dst_factor, wgpu::BlendFactor::Src);
}

// ═══════════════════════════════════════════════════════════════════════
// GPU Memory
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn gpu_memory_budget_zero_usage_at_start() {
    let b = GpuMemoryBudget::new();
    assert_eq!(b.total_usage(), 0);
}
#[test]
fn gpu_memory_budget_usage_percentage_zero() {
    let b = GpuMemoryBudget::new();
    assert_eq!(b.usage_percentage(), 0.0);
}
#[test]
fn gpu_memory_budget_allocate_deallocate() {
    let b = GpuMemoryBudget::new();
    assert!(b.try_allocate(MemoryCategory::Textures, 1024));
    assert_eq!(b.total_usage(), 1024);
    b.deallocate(MemoryCategory::Textures, 1024);
    assert_eq!(b.total_usage(), 0);
}
#[test]
fn gpu_memory_budget_get_usage() {
    let b = GpuMemoryBudget::new();
    b.try_allocate(MemoryCategory::Textures, 512);
    assert_eq!(b.get_usage(MemoryCategory::Textures), 512);
}
#[test]
fn gpu_memory_budget_with_total_budget() {
    let b = GpuMemoryBudget::with_total_budget(1024 * 1024);
    assert_eq!(b.total_usage(), 0);
}

// ═══════════════════════════════════════════════════════════════════════
// HDRI Catalog — DayPeriod
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn day_period_as_str() {
    assert_eq!(DayPeriod::Day.as_str(), "day");
    assert_eq!(DayPeriod::Morning.as_str(), "morning");
    assert_eq!(DayPeriod::Evening.as_str(), "evening");
    assert_eq!(DayPeriod::Night.as_str(), "night");
}
#[test]
fn day_period_from_str_loose_day() {
    assert_eq!(DayPeriod::from_str_loose("day"), Some(DayPeriod::Day));
}
#[test]
fn day_period_from_str_loose_morning_variants() {
    assert_eq!(
        DayPeriod::from_str_loose("morning"),
        Some(DayPeriod::Morning)
    );
    assert_eq!(
        DayPeriod::from_str_loose("sunrise"),
        Some(DayPeriod::Morning)
    );
    assert_eq!(DayPeriod::from_str_loose("dawn"), Some(DayPeriod::Morning));
}
#[test]
fn day_period_from_str_loose_evening_variants() {
    assert_eq!(
        DayPeriod::from_str_loose("evening"),
        Some(DayPeriod::Evening)
    );
    assert_eq!(
        DayPeriod::from_str_loose("sunset"),
        Some(DayPeriod::Evening)
    );
    assert_eq!(DayPeriod::from_str_loose("dusk"), Some(DayPeriod::Evening));
}
#[test]
fn day_period_from_str_loose_night_variants() {
    assert_eq!(DayPeriod::from_str_loose("night"), Some(DayPeriod::Night));
    assert_eq!(
        DayPeriod::from_str_loose("midnight"),
        Some(DayPeriod::Night)
    );
}
#[test]
fn day_period_from_game_hours() {
    assert_eq!(DayPeriod::from_game_hours(7.0), DayPeriod::Morning);
    assert_eq!(DayPeriod::from_game_hours(12.0), DayPeriod::Day);
    assert_eq!(DayPeriod::from_game_hours(18.0), DayPeriod::Evening);
    assert_eq!(DayPeriod::from_game_hours(23.0), DayPeriod::Night);
    assert_eq!(DayPeriod::from_game_hours(3.0), DayPeriod::Night);
}
#[test]
fn day_period_from_game_hours_boundaries() {
    assert_eq!(DayPeriod::from_game_hours(5.0), DayPeriod::Morning);
    assert_eq!(DayPeriod::from_game_hours(10.0), DayPeriod::Day);
    assert_eq!(DayPeriod::from_game_hours(17.0), DayPeriod::Evening);
    assert_eq!(DayPeriod::from_game_hours(21.0), DayPeriod::Night);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeDetectorConfig
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn biome_detector_config_default_distance() {
    assert_eq!(
        BiomeDetectorConfig::default().sample_distance_threshold,
        2.0
    );
}
#[test]
fn biome_detector_config_default_hysteresis() {
    assert_eq!(BiomeDetectorConfig::default().hysteresis_count, 3);
}

// ═══════════════════════════════════════════════════════════════════════
// IblQuality size methods
// ═══════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════
// WaterUniforms defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn water_uniforms_default_camera_pos() {
    let u = WaterUniforms::default();
    assert_eq!(u.camera_pos, [0.0, 5.0, -10.0]);
}
#[test]
fn water_uniforms_default_time() {
    assert_eq!(WaterUniforms::default().time, 0.0);
}
#[test]
fn water_uniforms_default_water_color_deep() {
    assert_eq!(WaterUniforms::default().water_color_deep, [0.02, 0.08, 0.2]);
}
#[test]
fn water_uniforms_default_water_color_shallow() {
    assert_eq!(
        WaterUniforms::default().water_color_shallow,
        [0.1, 0.4, 0.5]
    );
}
#[test]
fn water_uniforms_default_foam_color() {
    assert_eq!(WaterUniforms::default().foam_color, [0.95, 0.98, 1.0]);
}
#[test]
fn water_uniforms_default_foam_threshold() {
    assert_eq!(WaterUniforms::default().foam_threshold, 0.6);
}
