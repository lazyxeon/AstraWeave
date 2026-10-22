#![allow(unused, clippy::all)]
//! Wave 2 Mutation Remediation — Extended Coverage Tests
//!
//! Targets functions with zero / inline-only test coverage from wave2:
//! AnimationState play/pause/stop/restart, AnimationClip::sample,
//! TransparencyManager sorting/filtering, DrawBatch build, VertexCompressor,
//! GpuMemoryBudget snapshot/set, LODGenerator, BiomeDetector, DayPeriod::all.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::animation::{
    compute_joint_matrices, AnimationChannel, AnimationClip, AnimationState, ChannelData,
    Interpolation, Joint, JointPalette, Skeleton, Transform,
};
use astraweave_render::biome_detector::{BiomeDetector, BiomeDetectorConfig};
use astraweave_render::culling::{
    batch_visible_instances, build_indirect_commands_cpu, BatchId, DrawBatch, FrustumPlanes,
    InstanceAABB,
};
use astraweave_render::gpu_memory::{CategoryBudget, GpuMemoryBudget, MemoryCategory};
use astraweave_render::hdri_catalog::DayPeriod;
use astraweave_render::lod_generator::{LODConfig, LODGenerator, SimplificationMesh};
use astraweave_render::transparency::{BlendMode, TransparencyManager};
use astraweave_render::vertex_compression::{CompressedVertex, VertexCompressor};
use astraweave_terrain::biome::BiomeType;
use glam::{Mat4, Quat, Vec2, Vec3};

// ═══════════════════════════════════════════════════════════════════════
// AnimationState — play/pause/stop/restart
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn anim_state_play_sets_playing() {
    let mut s = AnimationState::default();
    assert!(!s.playing);
    s.play();
    assert!(s.playing);
}
#[test]
fn anim_state_pause_clears_playing() {
    let mut s = AnimationState::default();
    s.play();
    s.pause();
    assert!(!s.playing);
}
#[test]
fn anim_state_stop_resets_time() {
    let mut s = AnimationState {
        playing: true,
        time: 0.5,
        ..Default::default()
    };
    s.stop();
    assert!(!s.playing);
    assert_eq!(s.time, 0.0);
}
#[test]
fn anim_state_restart_resets_and_plays() {
    let mut s = AnimationState {
        playing: false,
        time: 0.5,
        ..Default::default()
    };
    s.restart();
    assert!(s.playing);
    assert_eq!(s.time, 0.0);
}
#[test]
fn anim_state_update_not_playing_noop() {
    let mut s = AnimationState::default(); // playing=false
    s.time = 0.5;
    s.update(1.0, 2.0);
    assert_eq!(s.time, 0.5); // unchanged
}
#[test]
fn anim_state_looping_wraps() {
    let mut s = AnimationState {
        playing: true,
        looping: true,
        speed: 1.0,
        time: 0.0,
        ..Default::default()
    };
    s.update(2.5, 1.0); // 2.5 mod 1.0 = 0.5
    assert!(s.time >= 0.0 && s.time < 1.0);
    assert!(s.playing);
}
#[test]
fn anim_state_non_looping_clamps_and_stops() {
    let mut s = AnimationState {
        playing: true,
        looping: false,
        speed: 1.0,
        time: 0.0,
        ..Default::default()
    };
    s.update(5.0, 1.0);
    assert!((s.time - 1.0).abs() < 0.01);
    assert!(!s.playing);
}

// ═══════════════════════════════════════════════════════════════════════
// AnimationClip::sample (requires Skeleton + channels)
// ═══════════════════════════════════════════════════════════════════════
fn make_test_skeleton() -> Skeleton {
    Skeleton {
        joints: vec![
            Joint {
                name: "root".into(),
                parent_index: None,
                local_transform: Transform::default(),
                inverse_bind_matrix: Mat4::IDENTITY,
            },
            Joint {
                name: "child".into(),
                parent_index: Some(0),
                local_transform: Transform {
                    translation: Vec3::new(0.0, 1.0, 0.0),
                    ..Default::default()
                },
                inverse_bind_matrix: Mat4::IDENTITY,
            },
        ],
        root_indices: vec![0],
    }
}

#[test]
fn anim_clip_sample_identity_returns_default_transforms() {
    let skeleton = make_test_skeleton();
    let clip = AnimationClip {
        name: "idle".into(),
        duration: 1.0,
        channels: vec![],
    };
    let result = clip.sample(0.0, &skeleton);
    assert_eq!(result.len(), 2);
    // With no channels, should return skeleton's local transforms
    assert_eq!(result[0].translation, Vec3::ZERO);
    assert!((result[1].translation - Vec3::new(0.0, 1.0, 0.0)).length() < 0.01);
}

#[test]
fn anim_clip_sample_translation_channel() {
    let skeleton = make_test_skeleton();
    let clip = AnimationClip {
        name: "slide".into(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 1.0],
            data: ChannelData::Translation(vec![Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0)]),
            interpolation: Interpolation::Linear,
        }],
    };
    let at_half = clip.sample(0.5, &skeleton);
    assert!((at_half[0].translation.x - 2.5).abs() < 0.1);
}

#[test]
fn anim_clip_sample_step_interpolation() {
    let skeleton = make_test_skeleton();
    let clip = AnimationClip {
        name: "jump".into(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 0.5],
            data: ChannelData::Translation(vec![Vec3::ZERO, Vec3::new(0.0, 3.0, 0.0)]),
            interpolation: Interpolation::Step,
        }],
    };
    // Before second keyframe: should hold first value
    let before = clip.sample(0.25, &skeleton);
    assert!((before[0].translation.y - 0.0).abs() < 0.01);
    // At/after second keyframe: should snap to second value
    let after = clip.sample(0.75, &skeleton);
    assert!((after[0].translation.y - 3.0).abs() < 0.01);
}

// ═══════════════════════════════════════════════════════════════════════
// compute_joint_matrices
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn compute_joint_matrices_identity() {
    let skeleton = make_test_skeleton();
    let local_transforms = vec![
        Transform::default(),
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
    ];
    let result = compute_joint_matrices(&skeleton, &local_transforms);
    assert!(result.is_ok());
    let matrices = result.unwrap();
    assert_eq!(matrices.len(), 2);
    // Root with identity bind → identity
    assert!((matrices[0] - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 0.01));
}

// ═══════════════════════════════════════════════════════════════════════
// Transform::lerp boundary
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn transform_lerp_at_zero_returns_self() {
    let a = Transform {
        translation: Vec3::new(1.0, 2.0, 3.0),
        ..Default::default()
    };
    let b = Transform {
        translation: Vec3::new(10.0, 20.0, 30.0),
        ..Default::default()
    };
    let result = a.lerp(&b, 0.0);
    assert!((result.translation - a.translation).length() < 0.01);
}
#[test]
fn transform_lerp_at_one_returns_other() {
    let a = Transform::default();
    let b = Transform {
        translation: Vec3::new(10.0, 20.0, 30.0),
        scale: Vec3::splat(2.0),
        ..Default::default()
    };
    let result = a.lerp(&b, 1.0);
    assert!((result.translation - b.translation).length() < 0.01);
    assert!((result.scale - b.scale).length() < 0.01);
}

// ═══════════════════════════════════════════════════════════════════════
// TransparencyManager — sorting, filtering, clear
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn transparency_manager_add_increases_count() {
    let mut tm = TransparencyManager::new();
    tm.add_instance(0, Vec3::new(0.0, 0.0, 5.0), BlendMode::Alpha);
    tm.add_instance(1, Vec3::new(0.0, 0.0, 10.0), BlendMode::Alpha);
    assert_eq!(tm.count(), 2);
}
#[test]
fn transparency_manager_clear_resets() {
    let mut tm = TransparencyManager::new();
    tm.add_instance(0, Vec3::ZERO, BlendMode::Alpha);
    tm.clear();
    assert_eq!(tm.count(), 0);
}
#[test]
fn transparency_manager_sorted_back_to_front() {
    let mut tm = TransparencyManager::new();
    tm.add_instance(0, Vec3::new(0.0, 0.0, 2.0), BlendMode::Alpha); // close
    tm.add_instance(1, Vec3::new(0.0, 0.0, 10.0), BlendMode::Alpha); // far
    tm.add_instance(2, Vec3::new(0.0, 0.0, 5.0), BlendMode::Alpha); // mid
    tm.update(Vec3::ZERO);
    let sorted: Vec<u32> = tm.sorted_instances().map(|i| i.instance_index).collect();
    // Back-to-front: furthest first
    assert_eq!(sorted[0], 1); // 10 units away
    assert_eq!(sorted[1], 2); // 5 units
    assert_eq!(sorted[2], 0); // 2 units
}
#[test]
fn transparency_manager_filter_by_blend_mode() {
    let mut tm = TransparencyManager::new();
    tm.add_instance(0, Vec3::ZERO, BlendMode::Alpha);
    tm.add_instance(1, Vec3::ZERO, BlendMode::Additive);
    tm.add_instance(2, Vec3::ZERO, BlendMode::Alpha);
    tm.update(Vec3::ZERO);
    let alpha_count = tm.instances_by_blend_mode(BlendMode::Alpha).count();
    let additive_count = tm.instances_by_blend_mode(BlendMode::Additive).count();
    assert_eq!(alpha_count, 2);
    assert_eq!(additive_count, 1);
}

// ═══════════════════════════════════════════════════════════════════════
// DrawBatch, build_indirect_commands_cpu, batch_visible_instances
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn draw_batch_new_empty() {
    let b = DrawBatch::new(BatchId::new(0, 0), 36, 0);
    assert_eq!(b.instance_count(), 0);
}
#[test]
fn draw_batch_add_instance_increments() {
    let mut b = DrawBatch::new(BatchId::new(0, 0), 36, 0);
    b.add_instance(0);
    b.add_instance(1);
    assert_eq!(b.instance_count(), 2);
}
#[test]
fn build_indirect_commands_cpu_basic() {
    let mut batch = DrawBatch::new(BatchId::new(0, 0), 36, 0);
    batch.add_instance(0);
    batch.add_instance(1);
    let cmds = build_indirect_commands_cpu(&[batch]);
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].vertex_count, 36);
    assert_eq!(cmds[0].instance_count, 2);
}
#[test]
fn build_indirect_commands_cpu_empty_batches() {
    let cmds = build_indirect_commands_cpu(&[]);
    assert!(cmds.is_empty());
}
#[test]
fn batch_visible_instances_groups_by_batch() {
    // Create visible indices with different batch assignments
    let visible = vec![0u32, 1, 2, 3];
    let batches = batch_visible_instances(
        &visible,
        |idx| BatchId::new(idx / 2, 0), // 0,1 → batch(0,0); 2,3 → batch(1,0)
        |bid| (36, bid.mesh_id * 36),   // (vertex_count, first_vertex)
    );
    assert_eq!(batches.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════
// FrustumPlanes::test_aabb
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn frustum_test_aabb_inside_visible() {
    let vp = Mat4::perspective_rh(1.0, 1.0, 0.1, 100.0)
        * Mat4::look_to_rh(Vec3::new(0.0, 0.0, 5.0), -Vec3::Z, Vec3::Y);
    let frustum = FrustumPlanes::from_view_proj(&vp);
    assert!(frustum.test_aabb(Vec3::ZERO, Vec3::ONE));
}
#[test]
fn frustum_test_aabb_outside_invisible() {
    let vp =
        Mat4::perspective_rh(1.0, 1.0, 0.1, 10.0) * Mat4::look_to_rh(Vec3::ZERO, -Vec3::Z, Vec3::Y);
    let frustum = FrustumPlanes::from_view_proj(&vp);
    assert!(!frustum.test_aabb(Vec3::new(0.0, 0.0, -200.0), Vec3::ONE));
}

// ═══════════════════════════════════════════════════════════════════════
// VertexCompressor
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn vertex_compressor_roundtrip() {
    let pos = Vec3::new(1.0, 2.0, 3.0);
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let uv = Vec2::new(0.5, 0.5);
    let compressed = VertexCompressor::compress(pos, normal, uv);
    let (dec_pos, dec_normal, dec_uv) = VertexCompressor::decompress(&compressed);
    assert!((dec_pos - pos).length() < 0.01);
    assert!((dec_normal - normal).length() < 0.05); // octahedral has ~3.5% error
    assert!((dec_uv - uv).length() < 0.01);
}
#[test]
fn vertex_compressor_batch() {
    let positions = vec![Vec3::ZERO, Vec3::ONE];
    let normals = vec![Vec3::Y, Vec3::X];
    let uvs = vec![Vec2::ZERO, Vec2::ONE];
    let batch = VertexCompressor::compress_batch(&positions, &normals, &uvs);
    assert_eq!(batch.len(), 2);
}
#[test]
fn vertex_compressor_savings() {
    let (std_bytes, comp_bytes, saved, pct) = VertexCompressor::calculate_savings(10000);
    assert_eq!(std_bytes, 320000);
    assert_eq!(comp_bytes, 200000);
    assert_eq!(saved, 120000);
    assert!((pct - 37.5).abs() < 0.1);
}
#[test]
fn vertex_compressor_savings_zero_verts() {
    let (std_bytes, comp_bytes, saved, _pct) = VertexCompressor::calculate_savings(0);
    assert_eq!(std_bytes, 0);
    assert_eq!(comp_bytes, 0);
    assert_eq!(saved, 0);
    // Note: pct may be NaN for zero-vertex case (0/0)
}

// ═══════════════════════════════════════════════════════════════════════
// GPU Memory — CategoryBudget defaults, snapshot, MemoryCategory::all
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn category_budget_default_soft_limit() {
    assert_eq!(CategoryBudget::default().soft_limit, 256 * 1024 * 1024);
}
#[test]
fn category_budget_default_hard_limit() {
    assert_eq!(CategoryBudget::default().hard_limit, 512 * 1024 * 1024);
}
#[test]
fn category_budget_default_current_zero() {
    assert_eq!(CategoryBudget::default().current, 0);
}
#[test]
fn memory_category_all_has_8() {
    assert_eq!(MemoryCategory::all().len(), 8);
}
#[test]
fn memory_category_all_contains_textures() {
    assert!(MemoryCategory::all().contains(&MemoryCategory::Textures));
}
#[test]
fn gpu_memory_snapshot_initial() {
    let b = GpuMemoryBudget::new();
    let snap = b.snapshot();
    assert_eq!(snap.len(), 8); // one per category
    for &(_, current, _hard) in &snap {
        assert_eq!(current, 0);
    }
}
#[test]
fn gpu_memory_set_category_budget() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Textures, 100, 200);
    let snap = b.snapshot();
    let tex = snap
        .iter()
        .find(|(c, _, _)| *c == MemoryCategory::Textures)
        .unwrap();
    assert_eq!(tex.2, 200); // hard_limit
}

// ═══════════════════════════════════════════════════════════════════════
// LODGenerator
// ═══════════════════════════════════════════════════════════════════════
fn make_cube_mesh() -> SimplificationMesh {
    // Simple cube: 8 vertices, 12 triangles
    let positions = vec![
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
    ];
    let normals = vec![Vec3::Y; 8];
    let uvs = vec![[0.0_f32, 0.0]; 8];
    let indices = vec![
        0, 1, 2, 2, 3, 0, // front
        1, 5, 6, 6, 2, 1, // right
        5, 4, 7, 7, 6, 5, // back
        4, 0, 3, 3, 7, 4, // left
        3, 2, 6, 6, 7, 3, // top
        4, 5, 1, 1, 0, 4, // bottom
    ];
    SimplificationMesh::new(positions, normals, uvs, indices)
}

#[test]
fn simplification_mesh_vertex_count() {
    let mesh = make_cube_mesh();
    assert_eq!(mesh.vertex_count(), 8);
}
#[test]
fn simplification_mesh_triangle_count() {
    let mesh = make_cube_mesh();
    assert_eq!(mesh.triangle_count(), 12);
}
#[test]
fn lod_generator_calculate_reduction_identical() {
    let config = LODConfig::default();
    let gen = LODGenerator::new(config);
    let mesh = make_cube_mesh();
    let reduction = gen.calculate_reduction(&mesh, &mesh);
    assert_eq!(reduction, 0.0);
}
#[test]
fn lod_generator_generate_lods_count() {
    let config = LODConfig::default(); // 3 targets
    let gen = LODGenerator::new(config);
    let mesh = make_cube_mesh();
    let lods = gen.generate_lods(&mesh);
    assert_eq!(lods.len(), 3);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeDetector
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn biome_detector_initial_state() {
    let d = BiomeDetector::new(BiomeDetectorConfig::default());
    assert!(d.current_biome().is_none());
    assert_eq!(d.transition_count(), 0);
}
#[test]
fn biome_detector_set_biome() {
    let mut d = BiomeDetector::new(BiomeDetectorConfig::default());
    d.set_biome(BiomeType::Forest);
    assert_eq!(d.current_biome(), Some(BiomeType::Forest));
}
#[test]
fn biome_detector_reset_clears() {
    let mut d = BiomeDetector::new(BiomeDetectorConfig::default());
    d.set_biome(BiomeType::Desert);
    d.reset();
    assert!(d.current_biome().is_none());
}
#[test]
fn biome_detector_classify_scored_desert() {
    // High temp, low moisture → Desert
    let biome = BiomeDetector::classify_scored(5.0, 0.9, 0.1);
    assert_eq!(biome, BiomeType::Desert);
}
#[test]
fn biome_detector_classify_scored_tundra() {
    // Low temp, low moisture → Tundra
    let biome = BiomeDetector::classify_scored(5.0, 0.1, 0.3);
    assert_eq!(biome, BiomeType::Tundra);
}

// ═══════════════════════════════════════════════════════════════════════
// DayPeriod::all
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn day_period_all_has_4() {
    assert_eq!(DayPeriod::all().len(), 4);
}
#[test]
fn day_period_all_contains_expected() {
    let all = DayPeriod::all();
    assert!(all.contains(&DayPeriod::Day));
    assert!(all.contains(&DayPeriod::Morning));
    assert!(all.contains(&DayPeriod::Evening));
    assert!(all.contains(&DayPeriod::Night));
}
#[test]
fn day_period_from_game_hours_wraps() {
    // 25.0 mod 24 = 1.0 → Night (before 5.0)
    assert_eq!(DayPeriod::from_game_hours(25.0), DayPeriod::Night);
}
#[test]
fn day_period_from_game_hours_negative() {
    // -1.0 rem_euclid 24 = 23.0 → Night
    assert_eq!(DayPeriod::from_game_hours(-1.0), DayPeriod::Night);
}
