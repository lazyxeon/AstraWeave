//! Batch 5: Material validation, post-processing configs, and texture streaming
//! Mutation-resistant integration tests targeting:
//!   - MaterialGpu (neutral, flags, size, Pod/Zeroable)
//!   - MaterialLayerDesc, MaterialPackDesc, ArrayLayout (defaults)
//!   - validate_material_pack (all 6 error branches + happy paths)
//!   - validate_array_layout (duplicate indices, gaps, happy path)
//!   - MaterialLoadStats::concise_summary (format token verification)
//!   - (BloomConfig placeholder tests removed — superseded by the real PBR `bloom::BloomConfig`)
//!   - TaaConfig, MotionBlurConfig, DofConfig, ColorGradingConfig (defaults)
//!   - TextureStreamingManager (budget, request, stats, eviction)

use std::collections::HashMap;
use std::path::PathBuf;

use astraweave_render::{
    ColorGradingConfig, DofConfig, MotionBlurConfig, TaaConfig,
};
use astraweave_render::material::{
    validate_array_layout, validate_material_pack, ArrayLayout, MaterialGpu, MaterialLayerDesc,
    MaterialLoadStats, MaterialPackDesc,
};
use astraweave_render::texture_streaming::TextureStreamingManager;

// ═══════════════════════════════════════════════════════════════════════════════
//  MaterialGpu
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn material_gpu_neutral_layer_0_texture_indices() {
    let m = MaterialGpu::neutral(0);
    assert_eq!(m.texture_indices, [0, 0, 0, 0]);
}

#[test]
fn material_gpu_neutral_layer_5_texture_indices() {
    let m = MaterialGpu::neutral(5);
    // All three main indices = layer_idx, fourth = 0
    assert_eq!(m.texture_indices[0], 5);
    assert_eq!(m.texture_indices[1], 5);
    assert_eq!(m.texture_indices[2], 5);
    assert_eq!(m.texture_indices[3], 0);
}

#[test]
fn material_gpu_neutral_layer_large_texture_indices() {
    let m = MaterialGpu::neutral(999);
    assert_eq!(m.texture_indices, [999, 999, 999, 0]);
}

#[test]
fn material_gpu_neutral_tiling_triplanar() {
    let m = MaterialGpu::neutral(0);
    let eps = 1e-6;
    assert!(
        (m.tiling_triplanar[0] - 1.0).abs() < eps,
        "u_tile should be 1.0"
    );
    assert!(
        (m.tiling_triplanar[1] - 1.0).abs() < eps,
        "v_tile should be 1.0"
    );
    assert!(
        (m.tiling_triplanar[2] - 16.0).abs() < eps,
        "triplanar_scale should be 16.0"
    );
    assert!(
        (m.tiling_triplanar[3] - 0.0).abs() < eps,
        "unused should be 0.0"
    );
}

#[test]
fn material_gpu_neutral_factors() {
    let m = MaterialGpu::neutral(0);
    let eps = 1e-6;
    assert!((m.factors[0] - 0.0).abs() < eps, "metallic should be 0.0");
    assert!((m.factors[1] - 0.5).abs() < eps, "roughness should be 0.5");
    assert!((m.factors[2] - 1.0).abs() < eps, "ao should be 1.0");
    assert!((m.factors[3] - 1.0).abs() < eps, "alpha should be 1.0");
}

#[test]
fn material_gpu_neutral_flags_zero() {
    let m = MaterialGpu::neutral(0);
    assert_eq!(m.flags, 0, "neutral material should have no flags");
}

#[test]
fn material_gpu_neutral_padding_zero() {
    let m = MaterialGpu::neutral(0);
    assert_eq!(m._padding, [0, 0, 0]);
}

#[test]
fn material_gpu_flag_has_albedo() {
    assert_eq!(MaterialGpu::FLAG_HAS_ALBEDO, 1);
}

#[test]
fn material_gpu_flag_has_normal() {
    assert_eq!(MaterialGpu::FLAG_HAS_NORMAL, 2);
}

#[test]
fn material_gpu_flag_has_orm() {
    assert_eq!(MaterialGpu::FLAG_HAS_ORM, 4);
}

#[test]
fn material_gpu_flag_triplanar() {
    assert_eq!(MaterialGpu::FLAG_TRIPLANAR, 8);
}

#[test]
fn material_gpu_flags_are_powers_of_two_no_overlap() {
    let flags = [
        MaterialGpu::FLAG_HAS_ALBEDO,
        MaterialGpu::FLAG_HAS_NORMAL,
        MaterialGpu::FLAG_HAS_ORM,
        MaterialGpu::FLAG_TRIPLANAR,
    ];
    // Each flag must be a power of 2
    for &f in &flags {
        assert!(f.is_power_of_two(), "flag {} not power of 2", f);
    }
    // No two flags should overlap
    for i in 0..flags.len() {
        for j in (i + 1)..flags.len() {
            assert_eq!(
                flags[i] & flags[j],
                0,
                "flags {} and {} overlap",
                flags[i],
                flags[j]
            );
        }
    }
}

#[test]
fn material_gpu_flags_combined_or() {
    let all = MaterialGpu::FLAG_HAS_ALBEDO
        | MaterialGpu::FLAG_HAS_NORMAL
        | MaterialGpu::FLAG_HAS_ORM
        | MaterialGpu::FLAG_TRIPLANAR;
    assert_eq!(all, 0b1111);
    assert_eq!(all, 15);
}

#[test]
fn material_gpu_size_64_bytes() {
    assert_eq!(
        std::mem::size_of::<MaterialGpu>(),
        64,
        "MaterialGpu must be 64 bytes for GPU buffer alignment"
    );
}

#[test]
fn material_gpu_bytemuck_pod_zeroable() {
    // Verify Pod and Zeroable traits via bytemuck round-trip
    let m = MaterialGpu::neutral(3);
    let bytes = bytemuck::bytes_of(&m);
    assert_eq!(bytes.len(), 64);
    let back: &MaterialGpu = bytemuck::from_bytes(bytes);
    assert_eq!(back.texture_indices, m.texture_indices);
    assert_eq!(back.flags, m.flags);
}

#[test]
fn material_gpu_zeroable() {
    let z: MaterialGpu = bytemuck::Zeroable::zeroed();
    assert_eq!(z.texture_indices, [0; 4]);
    assert_eq!(z.flags, 0);
    assert_eq!(z._padding, [0; 3]);
    assert_eq!(z.factors, [0.0; 4]);
    assert_eq!(z.tiling_triplanar, [0.0; 4]);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  MaterialLayerDesc defaults
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn material_layer_desc_default_key_empty() {
    let d = MaterialLayerDesc::default();
    assert!(d.key.is_empty());
}

#[test]
fn material_layer_desc_default_tiling() {
    let d = MaterialLayerDesc::default();
    assert_eq!(d.tiling, [1.0, 1.0]);
}

#[test]
fn material_layer_desc_default_triplanar_scale() {
    let d = MaterialLayerDesc::default();
    let eps = 1e-6;
    assert!((d.triplanar_scale - 16.0).abs() < eps);
}

#[test]
fn material_layer_desc_default_all_textures_none() {
    let d = MaterialLayerDesc::default();
    assert!(d.albedo.is_none());
    assert!(d.normal.is_none());
    assert!(d.mra.is_none());
    assert!(d.metallic.is_none());
    assert!(d.roughness.is_none());
    assert!(d.ao.is_none());
}

#[test]
fn material_layer_desc_default_atlas_none() {
    let d = MaterialLayerDesc::default();
    assert!(d.atlas.is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  MaterialPackDesc defaults
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn material_pack_desc_default_empty() {
    let p = MaterialPackDesc::default();
    assert!(p.biome.is_empty());
    assert!(p.layers.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  ArrayLayout defaults
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn array_layout_default_empty() {
    let a = ArrayLayout::default();
    assert!(a.layer_indices.is_empty());
    assert_eq!(a.count, 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  validate_material_pack — all branches
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn validate_pack_empty_biome_fails() {
    let pack = MaterialPackDesc {
        biome: String::new(),
        layers: vec![],
    };
    let r = validate_material_pack(&pack);
    assert!(r.is_err());
    let msg = r.unwrap_err().to_string();
    assert!(msg.contains("Biome"), "error should mention biome: {}", msg);
}

#[test]
fn validate_pack_empty_layer_key_fails() {
    let pack = MaterialPackDesc {
        biome: "forest".into(),
        layers: vec![MaterialLayerDesc {
            key: String::new(),
            albedo: Some(PathBuf::from("a.png")),
            ..Default::default()
        }],
    };
    let r = validate_material_pack(&pack);
    assert!(r.is_err());
    let msg = r.unwrap_err().to_string();
    assert!(msg.contains("key"), "error should mention key: {}", msg);
}

#[test]
fn validate_pack_duplicate_keys_fails() {
    let pack = MaterialPackDesc {
        biome: "desert".into(),
        layers: vec![
            MaterialLayerDesc {
                key: "sand".into(),
                albedo: Some(PathBuf::from("sand.png")),
                ..Default::default()
            },
            MaterialLayerDesc {
                key: "sand".into(),
                albedo: Some(PathBuf::from("sand2.png")),
                ..Default::default()
            },
        ],
    };
    let r = validate_material_pack(&pack);
    assert!(r.is_err());
    let msg = r.unwrap_err().to_string();
    assert!(
        msg.contains("Duplicate") || msg.contains("duplicate"),
        "error should mention duplicate: {}",
        msg
    );
}

#[test]
fn validate_pack_negative_tiling_u_fails() {
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![MaterialLayerDesc {
            key: "grass".into(),
            tiling: [-1.0, 2.0],
            albedo: Some(PathBuf::from("g.png")),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_negative_tiling_v_fails() {
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![MaterialLayerDesc {
            key: "grass".into(),
            tiling: [2.0, -0.5],
            albedo: Some(PathBuf::from("g.png")),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_zero_tiling_fails() {
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![MaterialLayerDesc {
            key: "grass".into(),
            tiling: [0.0, 1.0],
            albedo: Some(PathBuf::from("g.png")),
            ..Default::default()
        }],
    };
    assert!(
        validate_material_pack(&pack).is_err(),
        "tiling=0.0 should fail (check is <= 0.0)"
    );
}

#[test]
fn validate_pack_zero_triplanar_fails() {
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![MaterialLayerDesc {
            key: "grass".into(),
            triplanar_scale: 0.0,
            albedo: Some(PathBuf::from("g.png")),
            ..Default::default()
        }],
    };
    assert!(
        validate_material_pack(&pack).is_err(),
        "triplanar_scale=0.0 should fail"
    );
}

#[test]
fn validate_pack_negative_triplanar_fails() {
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![MaterialLayerDesc {
            key: "grass".into(),
            triplanar_scale: -5.0,
            albedo: Some(PathBuf::from("g.png")),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_valid_single_layer() {
    let pack = MaterialPackDesc {
        biome: "forest".into(),
        layers: vec![MaterialLayerDesc {
            key: "grass".into(),
            albedo: Some(PathBuf::from("grass.png")),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_ok());
}

#[test]
fn validate_pack_valid_multiple_unique_keys() {
    let pack = MaterialPackDesc {
        biome: "mountain".into(),
        layers: vec![
            MaterialLayerDesc {
                key: "rock".into(),
                albedo: Some(PathBuf::from("rock.png")),
                ..Default::default()
            },
            MaterialLayerDesc {
                key: "snow".into(),
                albedo: Some(PathBuf::from("snow.png")),
                tiling: [4.0, 4.0],
                triplanar_scale: 32.0,
                ..Default::default()
            },
            MaterialLayerDesc {
                key: "gravel".into(),
                normal: Some(PathBuf::from("gravel_n.png")),
                ..Default::default()
            },
        ],
    };
    assert!(validate_material_pack(&pack).is_ok());
}

#[test]
fn validate_pack_epsilon_tiling_passes() {
    // Just above zero should pass
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![MaterialLayerDesc {
            key: "grass".into(),
            tiling: [0.001, 0.001],
            albedo: Some(PathBuf::from("g.png")),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_ok());
}

#[test]
fn validate_pack_large_tiling_passes() {
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![MaterialLayerDesc {
            key: "grass".into(),
            tiling: [1000.0, 1000.0],
            triplanar_scale: 999.0,
            albedo: Some(PathBuf::from("g.png")),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_ok());
}

#[test]
fn validate_pack_no_layers_but_valid_biome_passes() {
    // Empty layers should be OK — just no layers
    let pack = MaterialPackDesc {
        biome: "empty_biome".into(),
        layers: vec![],
    };
    assert!(validate_material_pack(&pack).is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  validate_array_layout
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn validate_layout_empty_count_zero_passes() {
    let layout = ArrayLayout {
        layer_indices: HashMap::new(),
        count: 0,
    };
    assert!(validate_array_layout(&layout).is_ok());
}

#[test]
fn validate_layout_valid_sequential_indices() {
    let mut layout = ArrayLayout {
        layer_indices: HashMap::new(),
        count: 3,
    };
    layout.layer_indices.insert("a".into(), 0);
    layout.layer_indices.insert("b".into(), 1);
    layout.layer_indices.insert("c".into(), 2);
    assert!(validate_array_layout(&layout).is_ok());
}

#[test]
fn validate_layout_duplicate_indices_fails() {
    let mut layout = ArrayLayout {
        layer_indices: HashMap::new(),
        count: 3,
    };
    layout.layer_indices.insert("grass".into(), 0);
    layout.layer_indices.insert("dirt".into(), 0); // duplicate index!
    assert!(validate_array_layout(&layout).is_err());
}

#[test]
fn validate_layout_duplicate_index_error_message() {
    let mut layout = ArrayLayout {
        layer_indices: HashMap::new(),
        count: 3,
    };
    layout.layer_indices.insert("a".into(), 2);
    layout.layer_indices.insert("b".into(), 2);
    let err = validate_array_layout(&layout).unwrap_err().to_string();
    assert!(
        err.contains("Duplicate") || err.contains("duplicate"),
        "should mention duplicate: {}",
        err
    );
    assert!(
        err.contains("2"),
        "should mention the duplicate index: {}",
        err
    );
}

#[test]
fn validate_layout_single_entry_passes() {
    let mut layout = ArrayLayout {
        layer_indices: HashMap::new(),
        count: 1,
    };
    layout.layer_indices.insert("sole".into(), 0);
    assert!(validate_array_layout(&layout).is_ok());
}

#[test]
fn validate_layout_non_sequential_unique_passes() {
    // Indices 0, 5, 10 with count=11 — gaps are warned, not errors
    let mut layout = ArrayLayout {
        layer_indices: HashMap::new(),
        count: 11,
    };
    layout.layer_indices.insert("a".into(), 0);
    layout.layer_indices.insert("b".into(), 5);
    layout.layer_indices.insert("c".into(), 10);
    assert!(validate_array_layout(&layout).is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  MaterialLoadStats::concise_summary
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn concise_summary_contains_biome() {
    let stats = MaterialLoadStats {
        biome: "tundra".into(),
        ..Default::default()
    };
    let s = stats.concise_summary();
    assert!(s.contains("biome=tundra"));
}

#[test]
fn concise_summary_contains_layers_total() {
    let stats = MaterialLoadStats {
        biome: "x".into(),
        layers_total: 7,
        ..Default::default()
    };
    let s = stats.concise_summary();
    assert!(s.contains("layers=7"));
}

#[test]
fn concise_summary_contains_albedo_loaded_substituted() {
    let stats = MaterialLoadStats {
        biome: "x".into(),
        albedo_loaded: 3,
        albedo_substituted: 2,
        ..Default::default()
    };
    let s = stats.concise_summary();
    assert!(s.contains("albedo L/S=3/2"), "got: {}", s);
}

#[test]
fn concise_summary_contains_normal_loaded_substituted() {
    let stats = MaterialLoadStats {
        biome: "x".into(),
        normal_loaded: 4,
        normal_substituted: 1,
        ..Default::default()
    };
    let s = stats.concise_summary();
    assert!(s.contains("normal L/S=4/1"), "got: {}", s);
}

#[test]
fn concise_summary_contains_mra_loaded_packed_substituted() {
    let stats = MaterialLoadStats {
        biome: "x".into(),
        mra_loaded: 2,
        mra_packed: 1,
        mra_substituted: 3,
        ..Default::default()
    };
    let s = stats.concise_summary();
    assert!(s.contains("mra L+P/S=2+1/3"), "got: {}", s);
}

#[test]
fn concise_summary_gpu_memory_formatted() {
    let stats = MaterialLoadStats {
        biome: "x".into(),
        gpu_memory_bytes: 10 * 1024 * 1024, // 10 MiB
        ..Default::default()
    };
    let s = stats.concise_summary();
    assert!(s.contains("gpu=10.00 MiB"), "got: {}", s);
}

#[test]
fn concise_summary_gpu_memory_fractional() {
    let stats = MaterialLoadStats {
        biome: "x".into(),
        gpu_memory_bytes: 1_572_864, // 1.5 MiB
        ..Default::default()
    };
    let s = stats.concise_summary();
    assert!(s.contains("gpu=1.50 MiB"), "got: {}", s);
}

#[test]
fn concise_summary_starts_with_materials_tag() {
    let stats = MaterialLoadStats {
        biome: "x".into(),
        ..Default::default()
    };
    let s = stats.concise_summary();
    assert!(
        s.starts_with("[materials]"),
        "should start with [materials] tag, got: {}",
        s
    );
}

#[test]
fn concise_summary_all_fields_in_order() {
    let stats = MaterialLoadStats {
        biome: "forest".into(),
        layers_total: 5,
        albedo_loaded: 3,
        albedo_substituted: 2,
        normal_loaded: 4,
        normal_substituted: 1,
        mra_loaded: 2,
        mra_packed: 1,
        mra_substituted: 2,
        gpu_memory_bytes: 1024 * 1024 * 10,
    };
    let s = stats.concise_summary();
    // Verify ordering: biome before layers before albedo before normal before mra before gpu
    let pos_biome = s.find("biome=").unwrap();
    let pos_layers = s.find("layers=").unwrap();
    let pos_albedo = s.find("albedo").unwrap();
    let pos_normal = s.find("normal").unwrap();
    let pos_mra = s.find("mra").unwrap();
    let pos_gpu = s.find("gpu=").unwrap();
    assert!(pos_biome < pos_layers, "biome before layers");
    assert!(pos_layers < pos_albedo, "layers before albedo");
    assert!(pos_albedo < pos_normal, "albedo before normal");
    assert!(pos_normal < pos_mra, "normal before mra");
    assert!(pos_mra < pos_gpu, "mra before gpu");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  TaaConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn taa_config_default_enabled() {
    let c = TaaConfig::default();
    assert!(c.enabled, "TAA should be enabled by default");
}

#[test]
fn taa_config_default_blend_factor() {
    let c = TaaConfig::default();
    assert!((c.blend_factor - 0.95).abs() < 1e-6);
}

#[test]
fn taa_config_default_jitter_scale() {
    let c = TaaConfig::default();
    assert!((c.jitter_scale - 1.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  MotionBlurConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn motion_blur_default_disabled() {
    let c = MotionBlurConfig::default();
    assert!(!c.enabled, "motion blur should be disabled by default");
}

#[test]
fn motion_blur_default_sample_count() {
    let c = MotionBlurConfig::default();
    assert_eq!(c.sample_count, 8);
}

#[test]
fn motion_blur_default_strength() {
    let c = MotionBlurConfig::default();
    assert!((c.strength - 1.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  DofConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn dof_config_default_disabled() {
    let c = DofConfig::default();
    assert!(!c.enabled, "DOF should be disabled by default");
}

#[test]
fn dof_config_default_focus_distance() {
    let c = DofConfig::default();
    assert!((c.focus_distance - 10.0).abs() < 1e-6);
}

#[test]
fn dof_config_default_focus_range() {
    let c = DofConfig::default();
    assert!((c.focus_range - 5.0).abs() < 1e-6);
}

#[test]
fn dof_config_default_bokeh_size() {
    let c = DofConfig::default();
    assert!((c.bokeh_size - 2.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  ColorGradingConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn color_grading_default_enabled() {
    let c = ColorGradingConfig::default();
    assert!(c.enabled);
}

#[test]
fn color_grading_default_exposure() {
    let c = ColorGradingConfig::default();
    assert!((c.exposure - 0.0).abs() < 1e-6);
}

#[test]
fn color_grading_default_contrast() {
    let c = ColorGradingConfig::default();
    assert!((c.contrast - 1.0).abs() < 1e-6);
}

#[test]
fn color_grading_default_saturation() {
    let c = ColorGradingConfig::default();
    assert!((c.saturation - 1.0).abs() < 1e-6);
}

#[test]
fn color_grading_default_temperature() {
    let c = ColorGradingConfig::default();
    assert!((c.temperature - 0.0).abs() < 1e-6);
}

#[test]
fn color_grading_default_tint() {
    let c = ColorGradingConfig::default();
    assert!((c.tint - 0.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  TextureStreamingManager — public API
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn streaming_new_budget_conversion_mb_to_bytes() {
    let mgr = TextureStreamingManager::new(64);
    let stats = mgr.get_stats();
    assert_eq!(
        stats.memory_budget_bytes,
        64 * 1024 * 1024,
        "budget should be 64 * 1024 * 1024 bytes"
    );
}

#[test]
fn streaming_new_budget_1mb() {
    let mgr = TextureStreamingManager::new(1);
    let stats = mgr.get_stats();
    assert_eq!(stats.memory_budget_bytes, 1_048_576);
}

#[test]
fn streaming_fresh_stats_zeroed() {
    let mgr = TextureStreamingManager::new(100);
    let stats = mgr.get_stats();
    assert_eq!(stats.loaded_count, 0);
    assert_eq!(stats.pending_count, 0);
    assert_eq!(stats.memory_used_bytes, 0);
    assert!((stats.memory_used_percent - 0.0).abs() < 1e-6);
}

#[test]
fn streaming_request_returns_none_for_new_texture() {
    let mut mgr = TextureStreamingManager::new(64);
    let result = mgr.request_texture("tex_a".into(), 10, 5.0);
    assert!(result.is_none());
}

#[test]
fn streaming_request_increments_pending() {
    let mut mgr = TextureStreamingManager::new(64);
    mgr.request_texture("tex_a".into(), 10, 5.0);
    let stats = mgr.get_stats();
    assert_eq!(stats.pending_count, 1);
}

#[test]
fn streaming_duplicate_request_no_double_pending() {
    let mut mgr = TextureStreamingManager::new(64);
    mgr.request_texture("tex_a".into(), 10, 5.0);
    mgr.request_texture("tex_a".into(), 20, 1.0); // duplicate
    let stats = mgr.get_stats();
    assert_eq!(stats.pending_count, 1, "duplicate should not double-count");
}

#[test]
fn streaming_multiple_unique_requests_counted() {
    let mut mgr = TextureStreamingManager::new(64);
    mgr.request_texture("a".into(), 1, 10.0);
    mgr.request_texture("b".into(), 2, 5.0);
    mgr.request_texture("c".into(), 3, 1.0);
    let stats = mgr.get_stats();
    assert_eq!(stats.pending_count, 3);
}

#[test]
fn streaming_is_resident_false_for_unknown() {
    let mgr = TextureStreamingManager::new(64);
    assert!(!mgr.is_resident(&"nonexistent".into()));
}

#[test]
fn streaming_is_resident_false_for_queued() {
    let mut mgr = TextureStreamingManager::new(64);
    mgr.request_texture("queued".into(), 10, 5.0);
    assert!(!mgr.is_resident(&"queued".into()));
}

#[test]
fn streaming_evict_lru_on_empty_returns_false() {
    let mut mgr = TextureStreamingManager::new(64);
    assert!(!mgr.evict_lru());
}

#[test]
fn streaming_clear_resets_all() {
    let mut mgr = TextureStreamingManager::new(64);
    mgr.request_texture("a".into(), 1, 1.0);
    mgr.request_texture("b".into(), 2, 2.0);
    mgr.clear();
    let stats = mgr.get_stats();
    assert_eq!(stats.loaded_count, 0);
    assert_eq!(stats.pending_count, 0);
    assert_eq!(stats.memory_used_bytes, 0);
}

#[test]
fn streaming_update_residency_sets_position() {
    let mut mgr = TextureStreamingManager::new(64);
    // Just verify it doesn't panic
    mgr.update_residency(glam::Vec3::new(10.0, 20.0, 30.0));
    // No observable side-effect from public API yet
}

#[test]
fn streaming_memory_used_percent_zero_on_fresh() {
    let mgr = TextureStreamingManager::new(256);
    let stats = mgr.get_stats();
    assert!((stats.memory_used_percent - 0.0).abs() < 1e-6);
}

#[test]
fn streaming_stats_budget_scales_with_mb() {
    // Verify the multiplication chain: mb * 1024 * 1024
    for mb in [1, 10, 64, 256, 1024] {
        let mgr = TextureStreamingManager::new(mb);
        let stats = mgr.get_stats();
        assert_eq!(
            stats.memory_budget_bytes,
            mb * 1024 * 1024,
            "budget for {} MB",
            mb
        );
    }
}
