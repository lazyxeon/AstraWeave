//! Wave 2 remediation: material (validation+gpu), biome_material, advanced_post config, error types
//!
//! Targets pure CPU functions with golden values for mutation killing.

use astraweave_render::{
    ColorGradingConfig, DofConfig, MotionBlurConfig, TaaConfig,
};
use astraweave_render::biome_material::{BiomeMaterialConfig, BiomeMaterialSystem};
use astraweave_render::error::{RenderError, RenderResult};
use astraweave_render::hdri_catalog::DayPeriod;
use astraweave_render::material::{
    validate_array_layout, validate_material_pack, ArrayLayout, MaterialGpu, MaterialLayerDesc,
    MaterialLoadStats, MaterialManager, MaterialPackDesc,
};
use astraweave_terrain::biome::BiomeType;
use std::collections::HashMap;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════════
// MaterialGpu
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn material_gpu_size() {
    assert_eq!(std::mem::size_of::<MaterialGpu>(), 64);
}

#[test]
fn material_gpu_flag_values() {
    assert_eq!(MaterialGpu::FLAG_HAS_ALBEDO, 1);
    assert_eq!(MaterialGpu::FLAG_HAS_NORMAL, 2);
    assert_eq!(MaterialGpu::FLAG_HAS_ORM, 4);
    assert_eq!(MaterialGpu::FLAG_TRIPLANAR, 8);
}

#[test]
fn material_gpu_flags_no_overlap() {
    let all = MaterialGpu::FLAG_HAS_ALBEDO
        | MaterialGpu::FLAG_HAS_NORMAL
        | MaterialGpu::FLAG_HAS_ORM
        | MaterialGpu::FLAG_TRIPLANAR;
    // 4 distinct bits should give exactly 15 (0b1111)
    assert_eq!(all, 0b1111);
}

#[test]
fn material_gpu_neutral_golden() {
    let m = MaterialGpu::neutral(7);
    assert_eq!(m.texture_indices, [7, 7, 7, 0]);
    assert_eq!(m.tiling_triplanar, [1.0, 1.0, 16.0, 0.0]);
    assert_eq!(m.factors, [0.0, 0.5, 1.0, 1.0]); // metallic, roughness, ao, alpha
    assert_eq!(m.flags, 0);
}

#[test]
fn material_gpu_neutral_different_indices() {
    let m0 = MaterialGpu::neutral(0);
    let m5 = MaterialGpu::neutral(5);
    assert_eq!(m0.texture_indices[0], 0);
    assert_eq!(m5.texture_indices[0], 5);
    assert_eq!(m5.texture_indices[1], 5);
    assert_eq!(m5.texture_indices[2], 5);
    assert_eq!(m5.texture_indices[3], 0); // unused always 0
}

// ═══════════════════════════════════════════════════════════════════════════
// MaterialLayerDesc
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn material_layer_desc_default() {
    let d = MaterialLayerDesc::default();
    assert!(d.key.is_empty());
    assert!(d.albedo.is_none());
    assert!(d.normal.is_none());
    assert!(d.mra.is_none());
    assert_eq!(d.tiling, [1.0, 1.0]);
    assert!((d.triplanar_scale - 16.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════
// MaterialLoadStats::concise_summary
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn material_load_stats_summary_contains_biome() {
    let stats = MaterialLoadStats {
        biome: "forest".into(),
        layers_total: 4,
        albedo_loaded: 3,
        albedo_substituted: 1,
        normal_loaded: 2,
        normal_substituted: 2,
        mra_loaded: 1,
        mra_packed: 1,
        mra_substituted: 2,
        gpu_memory_bytes: 1_048_576, // 1 MiB
    };
    let s = stats.concise_summary();
    assert!(s.contains("biome=forest"), "summary: {s}");
    assert!(s.contains("layers=4"), "summary: {s}");
    assert!(s.contains("albedo L/S=3/1"), "summary: {s}");
    assert!(s.contains("normal L/S=2/2"), "summary: {s}");
    assert!(s.contains("mra L+P/S=1+1/2"), "summary: {s}");
    assert!(s.contains("1.00 MiB"), "summary: {s}");
}

#[test]
fn material_load_stats_summary_zero_gpu() {
    let stats = MaterialLoadStats {
        biome: "desert".into(),
        gpu_memory_bytes: 0,
        ..Default::default()
    };
    let s = stats.concise_summary();
    assert!(s.contains("0.00 MiB"), "summary: {s}");
}

// ═══════════════════════════════════════════════════════════════════════════
// validate_material_pack
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn validate_pack_empty_biome_fails() {
    let pack = MaterialPackDesc {
        biome: "".into(),
        layers: vec![],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_valid_minimal() {
    let pack = MaterialPackDesc {
        biome: "grassland".into(),
        layers: vec![MaterialLayerDesc {
            key: "grass".into(),
            albedo: Some(PathBuf::from("grass.png")),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_ok());
}

#[test]
fn validate_pack_duplicate_keys_fails() {
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![
            MaterialLayerDesc {
                key: "dup".into(),
                albedo: Some(PathBuf::from("a.png")),
                ..Default::default()
            },
            MaterialLayerDesc {
                key: "dup".into(),
                albedo: Some(PathBuf::from("b.png")),
                ..Default::default()
            },
        ],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_empty_key_fails() {
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![MaterialLayerDesc {
            key: "".into(),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_negative_tiling_fails() {
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![MaterialLayerDesc {
            key: "layer".into(),
            tiling: [-1.0, 1.0],
            albedo: Some(PathBuf::from("a.png")),
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
            key: "layer".into(),
            tiling: [0.0, 1.0],
            albedo: Some(PathBuf::from("a.png")),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_negative_triplanar_fails() {
    let pack = MaterialPackDesc {
        biome: "test".into(),
        layers: vec![MaterialLayerDesc {
            key: "layer".into(),
            triplanar_scale: -0.1,
            albedo: Some(PathBuf::from("a.png")),
            ..Default::default()
        }],
    };
    assert!(validate_material_pack(&pack).is_err());
}

#[test]
fn validate_pack_multiple_valid_layers() {
    let pack = MaterialPackDesc {
        biome: "forest".into(),
        layers: vec![
            MaterialLayerDesc {
                key: "moss".into(),
                albedo: Some(PathBuf::from("moss.png")),
                ..Default::default()
            },
            MaterialLayerDesc {
                key: "dirt".into(),
                albedo: Some(PathBuf::from("dirt.png")),
                ..Default::default()
            },
            MaterialLayerDesc {
                key: "rock".into(),
                albedo: Some(PathBuf::from("rock.png")),
                normal: Some(PathBuf::from("rock_n.png")),
                ..Default::default()
            },
        ],
    };
    assert!(validate_material_pack(&pack).is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════
// validate_array_layout
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn validate_layout_empty_ok() {
    let layout = ArrayLayout {
        layer_indices: HashMap::new(),
        count: 0,
    };
    assert!(validate_array_layout(&layout).is_ok());
}

#[test]
fn validate_layout_valid_sequential() {
    let mut indices = HashMap::new();
    indices.insert("grass".into(), 0);
    indices.insert("rock".into(), 1);
    indices.insert("sand".into(), 2);
    let layout = ArrayLayout {
        layer_indices: indices,
        count: 3,
    };
    assert!(validate_array_layout(&layout).is_ok());
}

#[test]
fn validate_layout_duplicate_indices_fails() {
    let mut indices = HashMap::new();
    indices.insert("a".into(), 0);
    indices.insert("b".into(), 0); // duplicate index
    let layout = ArrayLayout {
        layer_indices: indices,
        count: 2,
    };
    assert!(validate_array_layout(&layout).is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// MaterialManager
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn material_manager_new_no_stats() {
    let mgr = MaterialManager::new();
    assert!(mgr.current_stats().is_none());
    assert!(mgr.current_layout().is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// BiomeMaterialSystem
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn biome_material_config_default() {
    let cfg = BiomeMaterialConfig::default();
    assert_eq!(cfg.assets_root, PathBuf::from("assets"));
    assert_eq!(cfg.time_of_day, DayPeriod::Day);
    assert!(!cfg.preload_adjacent);
}

#[test]
fn biome_material_system_new_no_biome() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(sys.current_biome().is_none());
}

#[test]
fn biome_material_system_time_of_day() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert_eq!(sys.time_of_day(), DayPeriod::Day);
}

#[test]
fn biome_material_set_time_of_day_returns_true_on_change() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(sys.set_time_of_day(DayPeriod::Night));
    assert_eq!(sys.time_of_day(), DayPeriod::Night);
}

#[test]
fn biome_material_set_time_of_day_returns_false_same() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(!sys.set_time_of_day(DayPeriod::Day)); // already Day
}

#[test]
fn biome_material_needs_transition() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(sys.needs_transition(BiomeType::Forest));
}

#[test]
fn biome_material_mark_loaded_updates_state() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    sys.mark_loaded(BiomeType::Desert, None);
    assert_eq!(sys.current_biome(), Some(BiomeType::Desert));
    assert!(!sys.needs_transition(BiomeType::Desert));
    assert!(sys.needs_transition(BiomeType::Tundra));
}

#[test]
fn biome_material_dir_for_all_biomes() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    for biome in BiomeType::all() {
        let dir = sys.material_dir_for(*biome);
        let expected = PathBuf::from(format!("assets/materials/{}", biome.as_str()));
        assert_eq!(dir, expected, "biome {:?}", biome);
    }
}

#[test]
fn biome_material_terrain_fallback_dir() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert_eq!(
        sys.terrain_fallback_dir(),
        PathBuf::from("assets/materials/terrain")
    );
}

#[test]
fn biome_material_custom_assets_root() {
    let cfg = BiomeMaterialConfig {
        assets_root: PathBuf::from("my/custom/root"),
        ..Default::default()
    };
    let sys = BiomeMaterialSystem::new(cfg);
    assert_eq!(
        sys.material_dir_for(BiomeType::Grassland),
        PathBuf::from("my/custom/root/materials/grassland")
    );
    assert_eq!(
        sys.terrain_fallback_dir(),
        PathBuf::from("my/custom/root/materials/terrain")
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Advanced Post-Processing Configs
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn taa_config_default() {
    let c = TaaConfig::default();
    assert!(c.enabled);
    assert!((c.blend_factor - 0.95).abs() < 1e-6);
    assert!((c.jitter_scale - 1.0).abs() < 1e-6);
}

#[test]
fn motion_blur_config_default() {
    let c = MotionBlurConfig::default();
    assert!(!c.enabled);
    assert_eq!(c.sample_count, 8);
    assert!((c.strength - 1.0).abs() < 1e-6);
}

#[test]
fn dof_config_default() {
    let c = DofConfig::default();
    assert!(!c.enabled);
    assert!((c.focus_distance - 10.0).abs() < 1e-6);
    assert!((c.focus_range - 5.0).abs() < 1e-6);
    assert!((c.bokeh_size - 2.0).abs() < 1e-6);
}

#[test]
fn color_grading_config_default() {
    let c = ColorGradingConfig::default();
    assert!(c.enabled);
    assert!((c.exposure).abs() < 1e-6);
    assert!((c.contrast - 1.0).abs() < 1e-6);
    assert!((c.saturation - 1.0).abs() < 1e-6);
    assert!((c.temperature).abs() < 1e-6);
    assert!((c.tint).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════
// RenderError Display strings
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn render_error_device_display() {
    let e = RenderError::Device("lost".into());
    assert_eq!(format!("{e}"), "GPU device error: lost");
}

#[test]
fn render_error_shader_display() {
    let e = RenderError::Shader("compile fail".into());
    assert_eq!(format!("{e}"), "shader/pipeline error: compile fail");
}

#[test]
fn render_error_asset_load_display() {
    let e = RenderError::AssetLoad {
        asset: "texture".into(),
        detail: "file not found".into(),
    };
    assert_eq!(format!("{e}"), "failed to load texture: file not found");
}

#[test]
fn render_error_surface_display() {
    let e = RenderError::Surface("timeout".into());
    assert_eq!(format!("{e}"), "surface error: timeout");
}

#[test]
fn render_error_graph_display() {
    let e = RenderError::Graph("cycle detected".into());
    assert_eq!(format!("{e}"), "render graph error: cycle detected");
}

#[test]
fn render_error_material_display() {
    let e = RenderError::Material("missing array".into());
    assert_eq!(format!("{e}"), "material error: missing array");
}

#[test]
fn render_error_post_process_display() {
    let e = RenderError::PostProcess("bad shader".into());
    assert_eq!(format!("{e}"), "post-processing error: bad shader");
}

#[test]
fn render_error_shadow_display() {
    let e = RenderError::Shadow("cascade overflow".into());
    assert_eq!(format!("{e}"), "shadow error: cascade overflow");
}

#[test]
fn render_error_animation_display() {
    let e = RenderError::Animation("bone index OOB".into());
    assert_eq!(format!("{e}"), "animation error: bone index OOB");
}

#[test]
fn render_error_image_display() {
    let e = RenderError::Image("bad JPEG".into());
    assert_eq!(format!("{e}"), "image error: bad JPEG");
}

#[test]
fn render_error_wgpu_display() {
    let e = RenderError::Wgpu("validation failed".into());
    assert_eq!(format!("{e}"), "wgpu error: validation failed");
}

#[test]
fn render_error_io_from_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file.txt");
    let re: RenderError = io_err.into();
    let msg = format!("{re}");
    assert!(msg.contains("file.txt"), "msg: {msg}");
}

#[test]
fn render_result_type_ok() {
    let r: RenderResult<i32> = Ok(42);
    assert_eq!(r.unwrap(), 42);
}

#[test]
fn render_result_type_err() {
    let r: RenderResult<()> = Err(RenderError::Device("test".into()));
    assert!(r.is_err());
}
