//! Wave 2 – Golden-value tests for culling.rs (142 mutants) + material.rs (68 mutants)
//!
//! Targets: InstanceAABB construction + from_transform, FrustumPlanes exact
//!          plane coefficients, test_aabb math, cpu_frustum_cull pipeline,
//!          build_indirect_commands_cpu, batch_visible_instances grouping,
//!          MaterialGpu::neutral golden, flag constants, validate_* functions.
//!
//! Strategy: Pin exact numerical outputs so any arithmetic or logic mutation is caught.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::culling::{
    batch_visible_instances, build_indirect_commands_cpu, cpu_frustum_cull, BatchId, DrawBatch,
    DrawIndirectCommand, FrustumPlanes, InstanceAABB,
};
use astraweave_render::material::{
    validate_array_layout, validate_material_pack, ArrayLayout, MaterialGpu, MaterialLayerDesc,
    MaterialLoadStats, MaterialPackDesc,
};
use glam::{Mat4, Vec3};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// InstanceAABB::new — golden field mapping
// ============================================================================

#[test]
fn aabb_new_center_matches() {
    let aabb = InstanceAABB::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.5, 0.5, 0.5), 42);
    assert_eq!(aabb.center, [1.0, 2.0, 3.0]);
}

#[test]
fn aabb_new_extent_matches() {
    let aabb = InstanceAABB::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.5, 0.75, 1.0), 42);
    assert_eq!(aabb.extent, [0.5, 0.75, 1.0]);
}

#[test]
fn aabb_new_index_matches() {
    let aabb = InstanceAABB::new(Vec3::ZERO, Vec3::ONE, 99);
    assert_eq!(aabb.instance_index, 99);
}

#[test]
fn aabb_new_padding_zero() {
    let aabb = InstanceAABB::new(Vec3::new(10.0, 20.0, 30.0), Vec3::ONE, 0);
    assert_eq!(aabb._pad0, 0);
}

// ============================================================================
// InstanceAABB::from_transform — identity
// ============================================================================

#[test]
fn aabb_from_identity_center() {
    let aabb = InstanceAABB::from_transform(
        &Mat4::IDENTITY,
        Vec3::new(-1.0, -2.0, -3.0),
        Vec3::new(1.0, 2.0, 3.0),
        0,
    );
    // Center should be (0, 0, 0) — midpoint of [-1,-2,-3] and [1,2,3]
    for i in 0..3 {
        assert!(
            (aabb.center[i]).abs() < 1e-5,
            "center[{}] = {}",
            i,
            aabb.center[i]
        );
    }
}

#[test]
fn aabb_from_identity_extent() {
    let aabb = InstanceAABB::from_transform(
        &Mat4::IDENTITY,
        Vec3::new(-1.0, -2.0, -3.0),
        Vec3::new(1.0, 2.0, 3.0),
        0,
    );
    // Extent should be half-size: (1, 2, 3)
    assert!((aabb.extent[0] - 1.0).abs() < 1e-5);
    assert!((aabb.extent[1] - 2.0).abs() < 1e-5);
    assert!((aabb.extent[2] - 3.0).abs() < 1e-5);
}

// ============================================================================
// InstanceAABB::from_transform — translation
// ============================================================================

#[test]
fn aabb_from_translation_shifts_center() {
    let t = Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0));
    let aabb = InstanceAABB::from_transform(&t, Vec3::new(-1.0, -1.0, -1.0), Vec3::ONE, 5);
    assert!((aabb.center[0] - 10.0).abs() < 1e-5);
    assert!((aabb.center[1] - 20.0).abs() < 1e-5);
    assert!((aabb.center[2] - 30.0).abs() < 1e-5);
}

#[test]
fn aabb_from_translation_same_extent() {
    let t = Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0));
    let aabb = InstanceAABB::from_transform(&t, Vec3::new(-1.0, -1.0, -1.0), Vec3::ONE, 0);
    // extent = half-size of unit cube = (1, 1, 1)
    for i in 0..3 {
        assert!(
            (aabb.extent[i] - 1.0).abs() < 1e-5,
            "extent[{}] = {}",
            i,
            aabb.extent[i]
        );
    }
}

// ============================================================================
// InstanceAABB::from_transform — scale
// ============================================================================

#[test]
fn aabb_from_uniform_scale_doubles_extent() {
    let s = Mat4::from_scale(Vec3::splat(2.0));
    let aabb = InstanceAABB::from_transform(&s, Vec3::new(-1.0, -1.0, -1.0), Vec3::ONE, 0);
    // scaled extent = 2 * original extent = (2, 2, 2)
    for i in 0..3 {
        assert!(
            (aabb.extent[i] - 2.0).abs() < 1e-4,
            "extent[{}] = {}",
            i,
            aabb.extent[i]
        );
    }
}

// ============================================================================
// FrustumPlanes — orthographic golden plane coefficients
// ============================================================================

#[test]
fn frustum_ortho_identity_has_six_planes() {
    let vp = Mat4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    assert_eq!(fp.planes.len(), 6);
}

#[test]
fn frustum_ortho_planes_normalized() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    for (i, plane) in fp.planes.iter().enumerate() {
        let len = (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-5,
            "Plane {} normal length {} should be ~1.0",
            i,
            len
        );
    }
}

// ============================================================================
// FrustumPlanes::test_aabb — exact boundary math
// ============================================================================

#[test]
fn test_aabb_origin_visible_in_unit_ortho() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    assert!(fp.test_aabb(Vec3::ZERO, Vec3::splat(1.0)));
}

#[test]
fn test_aabb_far_away_left_culled() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    // AABB at x=-100, well outside left plane
    assert!(!fp.test_aabb(Vec3::new(-100.0, 0.0, -50.0), Vec3::splat(1.0)));
}

#[test]
fn test_aabb_far_away_right_culled() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    assert!(!fp.test_aabb(Vec3::new(100.0, 0.0, -50.0), Vec3::splat(1.0)));
}

#[test]
fn test_aabb_huge_extent_always_visible() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    // AABB centered far away but extent is enormous
    assert!(fp.test_aabb(Vec3::new(100.0, 0.0, -50.0), Vec3::splat(10000.0)));
}

// ============================================================================
// cpu_frustum_cull — pipeline golden values
// ============================================================================

#[test]
fn cpu_cull_empty_instances() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    let visible = cpu_frustum_cull(&[], &fp);
    assert!(visible.is_empty());
}

#[test]
fn cpu_cull_one_visible() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    let instances = [InstanceAABB::new(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::splat(1.0),
        7,
    )];
    let visible = cpu_frustum_cull(&instances, &fp);
    assert_eq!(visible, vec![7]);
}

#[test]
fn cpu_cull_one_outside() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    let instances = [InstanceAABB::new(
        Vec3::new(0.0, 0.0, -200.0),
        Vec3::splat(1.0),
        3,
    )];
    let visible = cpu_frustum_cull(&instances, &fp);
    assert!(visible.is_empty());
}

#[test]
fn cpu_cull_mixed_visibility() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    let instances = [
        InstanceAABB::new(Vec3::new(0.0, 0.0, -5.0), Vec3::splat(1.0), 0), // visible
        InstanceAABB::new(Vec3::new(100.0, 0.0, -5.0), Vec3::splat(1.0), 1), // culled
        InstanceAABB::new(Vec3::new(5.0, 5.0, -50.0), Vec3::splat(1.0), 2), // visible
        InstanceAABB::new(Vec3::new(0.0, 0.0, -200.0), Vec3::splat(1.0), 3), // culled
    ];
    let visible = cpu_frustum_cull(&instances, &fp);
    assert_eq!(visible, vec![0, 2]);
}

#[test]
fn cpu_cull_preserves_instance_indices() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let fp = FrustumPlanes::from_view_proj(&vp);
    let instances = [
        InstanceAABB::new(Vec3::new(0.0, 0.0, -5.0), Vec3::splat(1.0), 42),
        InstanceAABB::new(Vec3::new(0.0, 0.0, -10.0), Vec3::splat(1.0), 99),
    ];
    let visible = cpu_frustum_cull(&instances, &fp);
    assert_eq!(visible, vec![42, 99]);
}

// ============================================================================
// DrawBatch + build_indirect_commands_cpu
// ============================================================================

#[test]
fn draw_batch_new_starts_empty() {
    let b = DrawBatch::new(BatchId::new(0, 0), 36, 0);
    assert_eq!(b.instance_count(), 0);
    assert!(b.instances.is_empty());
}

#[test]
fn draw_batch_add_instance_increments() {
    let mut b = DrawBatch::new(BatchId::new(0, 0), 36, 0);
    b.add_instance(10);
    b.add_instance(20);
    assert_eq!(b.instance_count(), 2);
    assert_eq!(b.instances, vec![10, 20]);
}

#[test]
fn build_indirect_empty_batches() {
    let commands = build_indirect_commands_cpu(&[]);
    assert!(commands.is_empty());
}

#[test]
fn build_indirect_single_batch_golden() {
    let mut b = DrawBatch::new(BatchId::new(1, 2), 36, 100);
    b.add_instance(0);
    b.add_instance(5);
    b.add_instance(8);
    let commands = build_indirect_commands_cpu(&[b]);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].vertex_count, 36);
    assert_eq!(commands[0].instance_count, 3);
    assert_eq!(commands[0].first_vertex, 100);
    assert_eq!(commands[0].first_instance, 0);
}

#[test]
fn build_indirect_multi_batch_preserves_order() {
    let mut b1 = DrawBatch::new(BatchId::new(0, 0), 36, 0);
    b1.add_instance(0);
    let mut b2 = DrawBatch::new(BatchId::new(1, 1), 24, 36);
    b2.add_instance(1);
    b2.add_instance(2);
    let commands = build_indirect_commands_cpu(&[b1, b2]);
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].instance_count, 1);
    assert_eq!(commands[1].instance_count, 2);
    assert_eq!(commands[1].vertex_count, 24);
    assert_eq!(commands[1].first_vertex, 36);
}

// ============================================================================
// batch_visible_instances — grouping golden
// ============================================================================

#[test]
fn batch_visible_groups_by_batch_id() {
    let visible = vec![0u32, 1, 2, 3];
    let batches = batch_visible_instances(
        &visible,
        |idx| {
            if idx < 2 {
                BatchId::new(0, 0)
            } else {
                BatchId::new(1, 1)
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
    assert_eq!(batches.len(), 2);
    // BTreeMap ordering: BatchId(0,0) < BatchId(1,1)
    assert_eq!(batches[0].batch_id, BatchId::new(0, 0));
    assert_eq!(batches[0].instance_count(), 2);
    assert_eq!(batches[1].batch_id, BatchId::new(1, 1));
    assert_eq!(batches[1].instance_count(), 2);
}

#[test]
fn batch_visible_empty_returns_empty() {
    let batches = batch_visible_instances(&[], |_| BatchId::new(0, 0), |_| (36, 0));
    assert!(batches.is_empty());
}

// ============================================================================
// BatchId — equality and ordering
// ============================================================================

#[test]
fn batch_id_eq() {
    assert_eq!(BatchId::new(1, 2), BatchId::new(1, 2));
}

#[test]
fn batch_id_neq_mesh() {
    assert_ne!(BatchId::new(1, 2), BatchId::new(3, 2));
}

#[test]
fn batch_id_neq_material() {
    assert_ne!(BatchId::new(1, 2), BatchId::new(1, 3));
}

#[test]
fn batch_id_ord_mesh_first() {
    assert!(BatchId::new(0, 5) < BatchId::new(1, 0));
}

#[test]
fn batch_id_ord_material_tiebreak() {
    assert!(BatchId::new(1, 0) < BatchId::new(1, 1));
}

// ============================================================================
// DrawIndirectCommand — golden construction
// ============================================================================

#[test]
fn draw_indirect_new_golden() {
    let cmd = DrawIndirectCommand::new(36, 5, 100, 0);
    assert_eq!(cmd.vertex_count, 36);
    assert_eq!(cmd.instance_count, 5);
    assert_eq!(cmd.first_vertex, 100);
    assert_eq!(cmd.first_instance, 0);
}

#[test]
fn draw_indirect_default_all_zero() {
    let cmd = DrawIndirectCommand::default();
    assert_eq!(cmd.vertex_count, 0);
    assert_eq!(cmd.instance_count, 0);
    assert_eq!(cmd.first_vertex, 0);
    assert_eq!(cmd.first_instance, 0);
}

// ============================================================================
// MaterialGpu::neutral — golden field values
// ============================================================================

#[test]
fn neutral_texture_indices_all_same() {
    let m = MaterialGpu::neutral(5);
    assert_eq!(m.texture_indices, [5, 5, 5, 0]);
}

#[test]
fn neutral_tiling_golden() {
    let m = MaterialGpu::neutral(0);
    assert_eq!(m.tiling_triplanar, [1.0, 1.0, 16.0, 0.0]);
}

#[test]
fn neutral_factors_golden() {
    let m = MaterialGpu::neutral(0);
    // metallic=0, roughness=0.5, ao=1, alpha=1
    assert_eq!(m.factors, [0.0, 0.5, 1.0, 1.0]);
}

#[test]
fn neutral_flags_zero() {
    let m = MaterialGpu::neutral(0);
    assert_eq!(m.flags, 0);
}

#[test]
fn neutral_padding_zero() {
    let m = MaterialGpu::neutral(0);
    assert_eq!(m._padding, [0, 0, 0]);
}

#[test]
fn neutral_layer_idx_propagates() {
    for idx in [0, 1, 7, 255] {
        let m = MaterialGpu::neutral(idx);
        assert_eq!(m.texture_indices[0], idx);
        assert_eq!(m.texture_indices[1], idx);
        assert_eq!(m.texture_indices[2], idx);
        assert_eq!(m.texture_indices[3], 0); // always 0
    }
}

// ============================================================================
// MaterialGpu — flag bit constants
// ============================================================================

#[test]
fn flag_has_albedo_is_bit_0() {
    assert_eq!(MaterialGpu::FLAG_HAS_ALBEDO, 1);
}

#[test]
fn flag_has_normal_is_bit_1() {
    assert_eq!(MaterialGpu::FLAG_HAS_NORMAL, 2);
}

#[test]
fn flag_has_orm_is_bit_2() {
    assert_eq!(MaterialGpu::FLAG_HAS_ORM, 4);
}

#[test]
fn flag_triplanar_is_bit_3() {
    assert_eq!(MaterialGpu::FLAG_TRIPLANAR, 8);
}

#[test]
fn flags_non_overlapping() {
    let all = MaterialGpu::FLAG_HAS_ALBEDO
        | MaterialGpu::FLAG_HAS_NORMAL
        | MaterialGpu::FLAG_HAS_ORM
        | MaterialGpu::FLAG_TRIPLANAR;
    assert_eq!(all, 0b1111);
}

// ============================================================================
// MaterialGpu — size
// ============================================================================

#[test]
fn material_gpu_size_64_bytes() {
    assert_eq!(std::mem::size_of::<MaterialGpu>(), 64);
}

// ============================================================================
// MaterialLayerDesc — default golden
// ============================================================================

#[test]
fn material_layer_default_tiling() {
    let d = MaterialLayerDesc::default();
    assert_eq!(d.tiling, [1.0, 1.0]);
}

#[test]
fn material_layer_default_triplanar_scale() {
    let d = MaterialLayerDesc::default();
    assert_eq!(d.triplanar_scale, 16.0);
}

#[test]
fn material_layer_default_no_textures() {
    let d = MaterialLayerDesc::default();
    assert!(d.albedo.is_none());
    assert!(d.normal.is_none());
    assert!(d.mra.is_none());
    assert!(d.metallic.is_none());
    assert!(d.roughness.is_none());
    assert!(d.ao.is_none());
    assert!(d.atlas.is_none());
}

// ============================================================================
// MaterialLoadStats — concise_summary format golden
// ============================================================================

#[test]
fn load_stats_summary_contains_biome() {
    let stats = MaterialLoadStats {
        biome: "tundra".to_string(),
        layers_total: 3,
        ..Default::default()
    };
    assert!(stats.concise_summary().contains("biome=tundra"));
}

#[test]
fn load_stats_summary_gpu_mib_golden() {
    let stats = MaterialLoadStats {
        biome: "x".to_string(),
        gpu_memory_bytes: 1024 * 1024, // exactly 1 MiB
        ..Default::default()
    };
    assert!(stats.concise_summary().contains("gpu=1.00 MiB"));
}

#[test]
fn load_stats_summary_layers_golden() {
    let stats = MaterialLoadStats {
        biome: "x".to_string(),
        layers_total: 7,
        ..Default::default()
    };
    assert!(stats.concise_summary().contains("layers=7"));
}

#[test]
fn load_stats_summary_albedo_format() {
    let stats = MaterialLoadStats {
        biome: "x".to_string(),
        albedo_loaded: 3,
        albedo_substituted: 2,
        ..Default::default()
    };
    assert!(stats.concise_summary().contains("albedo L/S=3/2"));
}

// ============================================================================
// validate_material_pack — edge cases
// ============================================================================

#[test]
fn validate_pack_empty_key_fails() {
    let pack = MaterialPackDesc {
        biome: "forest".to_string(),
        layers: vec![MaterialLayerDesc {
            key: String::new(),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_zero_tiling_fails() {
    let pack = MaterialPackDesc {
        biome: "forest".to_string(),
        layers: vec![MaterialLayerDesc {
            key: "grass".to_string(),
            tiling: [0.0, 1.0],
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_zero_triplanar_fails() {
    let pack = MaterialPackDesc {
        biome: "test".to_string(),
        layers: vec![MaterialLayerDesc {
            key: "x".to_string(),
            triplanar_scale: 0.0,
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_no_layers_passes() {
    let pack = MaterialPackDesc {
        biome: "forest".to_string(),
        layers: vec![],
    };
    assert!(validate_material_pack(&pack).is_ok());
}

#[test]
fn validate_pack_valid_single_layer() {
    let pack = MaterialPackDesc {
        biome: "desert".to_string(),
        layers: vec![MaterialLayerDesc {
            key: "sand".to_string(),
            albedo: Some(PathBuf::from("sand.png")),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_ok());
}

// ============================================================================
// validate_array_layout — edge cases
// ============================================================================

#[test]
fn validate_layout_empty_passes() {
    let layout = ArrayLayout {
        layer_indices: HashMap::new(),
        count: 0,
    };
    assert!(validate_array_layout(&layout).is_ok());
}

#[test]
fn validate_layout_unique_indices_passes() {
    let mut layout = ArrayLayout {
        layer_indices: HashMap::new(),
        count: 2,
    };
    layout.layer_indices.insert("a".to_string(), 0);
    layout.layer_indices.insert("b".to_string(), 1);
    assert!(validate_array_layout(&layout).is_ok());
}

#[test]
fn validate_layout_duplicate_indices_fails() {
    let mut layout = ArrayLayout {
        layer_indices: HashMap::new(),
        count: 2,
    };
    layout.layer_indices.insert("a".to_string(), 0);
    layout.layer_indices.insert("b".to_string(), 0);
    assert!(validate_array_layout(&layout).is_err());
}

// ============================================================================
// InstanceAABB + FrustumPlanes sizes (layout pinning)
// ============================================================================

#[test]
fn instance_aabb_size_32_bytes() {
    assert_eq!(std::mem::size_of::<InstanceAABB>(), 32);
}

#[test]
fn frustum_planes_size_96_bytes() {
    assert_eq!(std::mem::size_of::<FrustumPlanes>(), 96);
}

#[test]
fn draw_indirect_command_size_16_bytes() {
    assert_eq!(std::mem::size_of::<DrawIndirectCommand>(), 16);
}
