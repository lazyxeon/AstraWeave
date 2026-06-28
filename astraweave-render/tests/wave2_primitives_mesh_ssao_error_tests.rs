//! Wave 2 Mutation Remediation — Primitives, Mesh, SSAO, Error Types,
//! Terrain Materials, Biome Audio, and MSAA tests.
//!
//! Targets remaining high-mutant-count CPU-testable APIs.

use astraweave_render::biome_audio::{BiomeAmbientMap, DEFAULT_AMBIENT_CROSSFADE};
use astraweave_render::debug_quad::create_screen_quad;
use astraweave_render::effects::WeatherKind;
use astraweave_render::error::{RenderError, RenderResult};
use astraweave_render::mesh::{compute_tangents, CpuMesh, MeshVertex};
use astraweave_render::msaa::MsaaMode;
use astraweave_render::primitives;
#[cfg(feature = "ssao")]
use astraweave_render::ssao::{SsaoConfig, SsaoKernel, SsaoQuality};
use astraweave_render::terrain_material::{
    TerrainLayerDesc, TerrainLayerGpu, TerrainMaterialDesc, TerrainMaterialGpu,
};
use astraweave_terrain::biome::BiomeType;
use glam::{Vec2, Vec3, Vec4};
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════
// Primitives
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn cube_24_vertices() {
    let (verts, _) = primitives::cube();
    assert_eq!(verts.len(), 24); // 6 faces × 4 verts
}

#[test]
fn cube_36_indices() {
    let (_, indices) = primitives::cube();
    assert_eq!(indices.len(), 36); // 6 faces × 2 tri × 3 indices
}

#[test]
fn cube_indices_valid() {
    let (verts, indices) = primitives::cube();
    for &i in &indices {
        assert!((i as usize) < verts.len());
    }
}

#[test]
fn cube_normals_unit_length() {
    let (verts, _) = primitives::cube();
    for v in &verts {
        let len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
        assert!((len - 1.0).abs() < 1e-5, "Normal not unit: {len}");
    }
}

#[test]
fn cube_face_normals_consistent() {
    let (verts, _) = primitives::cube();
    for face in 0..6 {
        let base = face * 4;
        let n = verts[base].normal;
        for i in 1..4 {
            assert_eq!(
                verts[base + i].normal,
                n,
                "Face {face} vertex {i} normal mismatch"
            );
        }
    }
}

#[test]
fn cube_uvs_in_unit_range() {
    let (verts, _) = primitives::cube();
    for v in &verts {
        assert!(v.uv[0] >= 0.0 && v.uv[0] <= 1.0);
        assert!(v.uv[1] >= 0.0 && v.uv[1] <= 1.0);
    }
}

#[test]
fn plane_4_vertices() {
    let (verts, _) = primitives::plane();
    assert_eq!(verts.len(), 4);
}

#[test]
fn plane_6_indices() {
    let (_, indices) = primitives::plane();
    assert_eq!(indices.len(), 6); // 2 triangles
}

#[test]
fn plane_normal_points_up() {
    let (verts, _) = primitives::plane();
    for v in &verts {
        assert_eq!(v.normal, [0.0, 1.0, 0.0]);
    }
}

#[test]
fn plane_y_coordinate_zero() {
    let (verts, _) = primitives::plane();
    for v in &verts {
        assert_eq!(v.position[1], 0.0);
    }
}

#[test]
fn sphere_vertex_count() {
    let (verts, _) = primitives::sphere(8, 8, 1.0);
    // (stacks+1) * (slices+1) = 9*9 = 81
    assert_eq!(verts.len(), 81);
}

#[test]
fn sphere_index_count() {
    let (_, indices) = primitives::sphere(8, 8, 1.0);
    // stacks * slices * 6 = 8*8*6 = 384
    assert_eq!(indices.len(), 384);
}

#[test]
fn sphere_indices_valid() {
    let (verts, indices) = primitives::sphere(8, 8, 1.0);
    for &i in &indices {
        assert!((i as usize) < verts.len());
    }
}

#[test]
fn sphere_radius_correct() {
    let radius = 5.0;
    let (verts, _) = primitives::sphere(8, 8, radius);
    for v in &verts {
        let r = (v.position[0].powi(2) + v.position[1].powi(2) + v.position[2].powi(2)).sqrt();
        assert!((r - radius).abs() < 0.01, "Vertex at wrong radius: {r}");
    }
}

#[test]
fn sphere_min_stacks_clamped() {
    // stacks=1 should be clamped to 3
    let (verts, _) = primitives::sphere(1, 8, 1.0);
    // Should use stacks=3: (3+1)*(8+1) = 36
    assert_eq!(verts.len(), 36);
}

#[test]
fn sphere_normals_approximately_unit() {
    let (verts, _) = primitives::sphere(8, 8, 1.0);
    for v in &verts {
        let len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
        // Poles might have degenerate normals, allow small tolerance
        if len > 0.01 {
            assert!((len - 1.0).abs() < 0.05, "Normal not unit: {len}");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// CpuMesh and MeshVertex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mesh_vertex_new() {
    let v = MeshVertex::new(
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec4::new(1.0, 0.0, 0.0, 1.0),
        Vec2::new(0.5, 0.5),
    );
    assert_eq!(v.position, [1.0, 2.0, 3.0]);
    assert_eq!(v.normal, [0.0, 1.0, 0.0]);
    assert_eq!(v.tangent, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(v.uv, [0.5, 0.5]);
}

#[test]
fn mesh_vertex_from_arrays() {
    let v = MeshVertex::from_arrays(
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0],
    );
    assert_eq!(v.position, [1.0, 0.0, 0.0]);
    assert_eq!(v.uv, [0.0, 1.0]);
}

#[test]
fn cpu_mesh_aabb_empty() {
    let mesh = CpuMesh::default();
    assert!(mesh.aabb().is_none());
}

#[test]
fn cpu_mesh_aabb_single_vertex() {
    let mesh = CpuMesh {
        vertices: vec![MeshVertex::new(
            Vec3::new(5.0, 10.0, -3.0),
            Vec3::Y,
            Vec4::X,
            Vec2::ZERO,
        )],
        indices: vec![],
        ..Default::default()
    };
    let (min, max) = mesh.aabb().unwrap();
    assert_eq!(min, Vec3::new(5.0, 10.0, -3.0));
    assert_eq!(max, Vec3::new(5.0, 10.0, -3.0));
}

#[test]
fn cpu_mesh_aabb_multi_vertex() {
    let mesh = CpuMesh {
        vertices: vec![
            MeshVertex::new(Vec3::new(-1.0, 0.0, 0.0), Vec3::Y, Vec4::X, Vec2::ZERO),
            MeshVertex::new(Vec3::new(1.0, 2.0, 3.0), Vec3::Y, Vec4::X, Vec2::ZERO),
            MeshVertex::new(Vec3::new(0.0, -5.0, 1.0), Vec3::Y, Vec4::X, Vec2::ZERO),
        ],
        indices: vec![0, 1, 2],
        ..Default::default()
    };
    let (min, max) = mesh.aabb().unwrap();
    assert_eq!(min, Vec3::new(-1.0, -5.0, 0.0));
    assert_eq!(max, Vec3::new(1.0, 2.0, 3.0));
}

#[test]
fn compute_tangents_triangle() {
    let mut mesh = CpuMesh {
        vertices: vec![
            MeshVertex::new(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::Y,
                Vec4::ZERO,
                Vec2::new(0.0, 0.0),
            ),
            MeshVertex::new(
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::Y,
                Vec4::ZERO,
                Vec2::new(1.0, 0.0),
            ),
            MeshVertex::new(
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::Y,
                Vec4::ZERO,
                Vec2::new(0.0, 1.0),
            ),
        ],
        indices: vec![0, 1, 2],
        ..Default::default()
    };
    compute_tangents(&mut mesh);
    // After computing tangents, they should be non-zero
    for v in &mesh.vertices {
        let len = (v.tangent[0].powi(2) + v.tangent[1].powi(2) + v.tangent[2].powi(2)).sqrt();
        assert!(len > 0.5, "Tangent should be computed: {len}");
    }
}

#[test]
fn compute_tangents_non_divisible_by_3_is_noop() {
    let mut mesh = CpuMesh {
        vertices: vec![MeshVertex::new(Vec3::ZERO, Vec3::Y, Vec4::ZERO, Vec2::ZERO)],
        indices: vec![0, 1], // Not divisible by 3
        ..Default::default()
    };
    compute_tangents(&mut mesh);
    // Should be a no-op (early return)
    assert_eq!(mesh.vertices[0].tangent, [0.0, 0.0, 0.0, 0.0]);
}

// ═══════════════════════════════════════════════════════════════════════
// TerrainMaterialDesc and presets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn terrain_layer_gpu_default_blend_sharpness() {
    let l = TerrainLayerGpu::default();
    assert_eq!(l.blend_sharpness, 0.5);
}

#[test]
fn terrain_layer_gpu_default_triplanar_power() {
    let l = TerrainLayerGpu::default();
    assert_eq!(l.triplanar_power, 4.0);
}

#[test]
fn terrain_layer_gpu_default_uv_scale() {
    let l = TerrainLayerGpu::default();
    assert_eq!(l.uv_scale, [1.0, 1.0]);
}

#[test]
fn terrain_layer_gpu_default_material_factors() {
    let l = TerrainLayerGpu::default();
    assert_eq!(l.material_factors, [0.0, 0.5]); // metallic=0, roughness=0.5
}

#[test]
fn terrain_material_gpu_default_triplanar_enabled() {
    let m = TerrainMaterialGpu::default();
    assert_eq!(m.triplanar_enabled, 1);
}

#[test]
fn terrain_material_gpu_default_normal_blend_rnm() {
    let m = TerrainMaterialGpu::default();
    assert_eq!(m.normal_blend_method, 1); // RNM
}

#[test]
fn terrain_material_gpu_default_slope_threshold() {
    let m = TerrainMaterialGpu::default();
    assert_eq!(m.triplanar_slope_threshold, 45.0);
}

#[test]
fn terrain_material_gpu_default_height_blend() {
    let m = TerrainMaterialGpu::default();
    assert_eq!(m.height_blend_enabled, 1);
}

#[test]
fn terrain_material_desc_grassland_name() {
    let g = TerrainMaterialDesc::grassland();
    assert_eq!(g.name, "grassland_terrain");
    assert_eq!(g.biome, "grassland");
}

#[test]
fn terrain_material_desc_grassland_layers() {
    let g = TerrainMaterialDesc::grassland();
    assert_eq!(g.layers.len(), 4);
    assert_eq!(g.layers[0].name, "grass");
    assert_eq!(g.layers[1].name, "dirt");
    assert_eq!(g.layers[2].name, "rock");
    assert_eq!(g.layers[3].name, "sparse_grass");
}

#[test]
fn terrain_material_desc_grassland_values() {
    let g = TerrainMaterialDesc::grassland();
    assert_eq!(g.splat_uv_scale, 0.5);
    assert!(g.triplanar_enabled);
    assert_eq!(g.triplanar_slope_threshold, 35.0);
    assert_eq!(g.normal_blend_method, "rnm");
    assert!(g.height_blend_enabled);
}

#[test]
fn terrain_material_desc_desert_name() {
    let d = TerrainMaterialDesc::desert();
    assert_eq!(d.name, "desert_terrain");
    assert_eq!(d.biome, "desert");
}

#[test]
fn terrain_material_desc_desert_layers() {
    let d = TerrainMaterialDesc::desert();
    assert_eq!(d.layers.len(), 4);
    assert_eq!(d.layers[0].name, "sand");
    assert_eq!(d.layers[3].name, "cracked_ground");
}

#[test]
fn terrain_material_desc_desert_splat_scale() {
    let d = TerrainMaterialDesc::desert();
    assert_eq!(d.splat_uv_scale, 0.4);
    assert_eq!(d.triplanar_slope_threshold, 40.0);
}

#[test]
fn terrain_material_desc_forest_name() {
    let f = TerrainMaterialDesc::forest();
    assert_eq!(f.name, "forest_terrain");
    assert_eq!(f.biome, "forest");
}

#[test]
fn terrain_material_desc_forest_layers() {
    let f = TerrainMaterialDesc::forest();
    assert_eq!(f.layers.len(), 4);
    assert_eq!(f.layers[0].name, "moss");
    assert_eq!(f.layers[3].name, "leaf_litter");
}

#[test]
fn terrain_material_desc_forest_values() {
    let f = TerrainMaterialDesc::forest();
    assert_eq!(f.splat_uv_scale, 0.6);
    assert_eq!(f.triplanar_slope_threshold, 30.0);
}

#[test]
fn normal_blend_to_gpu_rnm() {
    let mut m = TerrainMaterialDesc::default();
    m.normal_blend_method = "rnm".to_string();
    assert_eq!(m.normal_blend_to_gpu(), 1);
}

#[test]
fn normal_blend_to_gpu_linear() {
    let mut m = TerrainMaterialDesc::default();
    m.normal_blend_method = "linear".to_string();
    assert_eq!(m.normal_blend_to_gpu(), 0);
}

#[test]
fn normal_blend_to_gpu_udn() {
    let mut m = TerrainMaterialDesc::default();
    m.normal_blend_method = "udn".to_string();
    assert_eq!(m.normal_blend_to_gpu(), 2);
}

#[test]
fn normal_blend_to_gpu_unknown_defaults_rnm() {
    let mut m = TerrainMaterialDesc::default();
    m.normal_blend_method = "garbage".to_string();
    assert_eq!(m.normal_blend_to_gpu(), 1);
}

#[test]
fn terrain_material_to_gpu_resolves_textures() {
    let desc = TerrainMaterialDesc::grassland();
    let resolver = |path: &PathBuf| -> u32 {
        let name = path.file_stem().unwrap().to_str().unwrap();
        match name {
            "grass_albedo" => 10,
            "grass_normal" => 11,
            "grassland_splat" => 100,
            _ => 0,
        }
    };
    let gpu = desc.to_gpu(&resolver);
    assert_eq!(gpu.splat_map_index_0, 100);
    assert_eq!(gpu.layers[0].texture_indices[0], 10); // grass albedo
    assert_eq!(gpu.layers[0].texture_indices[1], 11); // grass normal
}

#[test]
fn terrain_material_to_gpu_triplanar_flag() {
    let desc = TerrainMaterialDesc::grassland();
    let resolver = |_: &PathBuf| -> u32 { 0 };
    let gpu = desc.to_gpu(&resolver);
    assert_eq!(gpu.triplanar_enabled, 1);
    assert_eq!(gpu.normal_blend_method, 1); // rnm
}

#[test]
fn terrain_layer_desc_default() {
    let l = TerrainLayerDesc::default();
    assert_eq!(l.uv_scale, [1.0, 1.0]);
    assert_eq!(l.blend_sharpness, 0.5);
    assert_eq!(l.triplanar_power, 4.0);
    assert_eq!(l.metallic, 0.0);
    assert_eq!(l.roughness, 0.5);
    assert!(l.albedo.is_none());
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeAmbientMap
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn biome_ambient_map_default_has_8() {
    let map = BiomeAmbientMap::default();
    assert_eq!(map.len(), 8);
    assert!(!map.is_empty());
}

#[test]
fn biome_ambient_map_default_crossfade() {
    assert_eq!(DEFAULT_AMBIENT_CROSSFADE, 3.0);
    let map = BiomeAmbientMap::default();
    assert_eq!(map.crossfade_sec(), 3.0);
}

#[test]
fn biome_ambient_map_empty() {
    let map = BiomeAmbientMap::empty();
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
    assert_eq!(map.crossfade_sec(), 3.0);
}

#[test]
fn biome_ambient_map_get_forest() {
    let map = BiomeAmbientMap::default();
    let path = map.get(BiomeType::Forest).unwrap();
    assert!(path.starts_with("assets/audio/ambient/"));
    assert!(path.ends_with(".ogg"));
}

#[test]
fn biome_ambient_map_all_biomes_have_tracks() {
    let map = BiomeAmbientMap::default();
    for b in [
        BiomeType::Forest,
        BiomeType::Desert,
        BiomeType::Grassland,
        BiomeType::Mountain,
        BiomeType::Tundra,
        BiomeType::Swamp,
        BiomeType::Beach,
        BiomeType::River,
    ] {
        assert!(map.get(b).is_some(), "Missing track for {:?}", b);
    }
}

#[test]
fn biome_ambient_map_set_override() {
    let mut map = BiomeAmbientMap::new();
    map.set(BiomeType::Forest, "custom/night_forest.wav");
    assert_eq!(
        map.get(BiomeType::Forest).unwrap(),
        "custom/night_forest.wav"
    );
}

#[test]
fn biome_ambient_map_remove() {
    let mut map = BiomeAmbientMap::new();
    map.remove(BiomeType::Desert);
    assert!(map.get(BiomeType::Desert).is_none());
    assert_eq!(map.len(), 7);
}

#[test]
fn biome_ambient_map_set_crossfade() {
    let mut map = BiomeAmbientMap::new();
    map.set_crossfade_sec(5.0);
    assert_eq!(map.crossfade_sec(), 5.0);
}

#[test]
fn biome_ambient_map_crossfade_clamps_min() {
    let mut map = BiomeAmbientMap::new();
    map.set_crossfade_sec(0.001);
    assert!((map.crossfade_sec() - 0.01).abs() < 1e-5);
}

#[test]
fn biome_ambient_map_crossfade_clamps_negative() {
    let mut map = BiomeAmbientMap::new();
    map.set_crossfade_sec(-5.0);
    assert!((map.crossfade_sec() - 0.01).abs() < 1e-5);
}

// ═══════════════════════════════════════════════════════════════════════
// MsaaMode
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn msaa_mode_default_is_x4() {
    let mode = MsaaMode::default();
    assert_eq!(mode, MsaaMode::X4);
}

#[test]
fn msaa_mode_off_sample_count() {
    assert_eq!(MsaaMode::Off.sample_count(), 1);
}

#[test]
fn msaa_mode_x2_sample_count() {
    assert_eq!(MsaaMode::X2.sample_count(), 2);
}

#[test]
fn msaa_mode_x4_sample_count() {
    assert_eq!(MsaaMode::X4.sample_count(), 4);
}

#[test]
fn msaa_mode_x8_sample_count() {
    assert_eq!(MsaaMode::X8.sample_count(), 8);
}

#[test]
fn msaa_mode_off_not_enabled() {
    assert!(!MsaaMode::Off.is_enabled());
}

#[test]
fn msaa_mode_x2_enabled() {
    assert!(MsaaMode::X2.is_enabled());
}

#[test]
fn msaa_mode_x4_enabled() {
    assert!(MsaaMode::X4.is_enabled());
}

#[test]
fn msaa_mode_x8_enabled() {
    assert!(MsaaMode::X8.is_enabled());
}

// ═══════════════════════════════════════════════════════════════════════
// SsaoQuality and SsaoConfig (feature-gated: ssao)
// ═══════════════════════════════════════════════════════════════════════

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_low_samples() {
    assert_eq!(SsaoQuality::Low.sample_count(), 8);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_medium_samples() {
    assert_eq!(SsaoQuality::Medium.sample_count(), 16);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_high_samples() {
    assert_eq!(SsaoQuality::High.sample_count(), 32);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_ultra_samples() {
    assert_eq!(SsaoQuality::Ultra.sample_count(), 64);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_low_radius() {
    assert_eq!(SsaoQuality::Low.radius(), 0.5);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_medium_radius() {
    assert_eq!(SsaoQuality::Medium.radius(), 1.0);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_high_radius() {
    assert_eq!(SsaoQuality::High.radius(), 1.5);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_ultra_radius() {
    assert_eq!(SsaoQuality::Ultra.radius(), 2.0);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_low_blur() {
    assert_eq!(SsaoQuality::Low.blur_kernel_size(), 0);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_medium_blur() {
    assert_eq!(SsaoQuality::Medium.blur_kernel_size(), 3);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_high_blur() {
    assert_eq!(SsaoQuality::High.blur_kernel_size(), 5);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_ultra_blur() {
    assert_eq!(SsaoQuality::Ultra.blur_kernel_size(), 7);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_quality() {
    let c = SsaoConfig::default();
    assert_eq!(c.quality, SsaoQuality::Medium);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_radius() {
    let c = SsaoConfig::default();
    assert_eq!(c.radius, 1.0);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_bias() {
    let c = SsaoConfig::default();
    assert_eq!(c.bias, 0.025);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_intensity() {
    let c = SsaoConfig::default();
    assert_eq!(c.intensity, 1.0);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_enabled() {
    let c = SsaoConfig::default();
    assert!(c.enabled);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_generate_samples_in_hemisphere() {
    let kernel = SsaoKernel::generate(16);
    // All samples should have z >= 0 (hemisphere above surface)
    for i in 0..16 {
        assert!(kernel.samples[i][2] >= 0.0, "Sample {i} z should be >= 0");
    }
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_generate_nonzero() {
    let kernel = SsaoKernel::generate(8);
    // At least some samples should be non-zero
    let any_nonzero = (0..8).any(|i| {
        kernel.samples[i][0].abs() > 1e-6
            || kernel.samples[i][1].abs() > 1e-6
            || kernel.samples[i][2].abs() > 1e-6
    });
    assert!(any_nonzero, "Kernel should have non-zero samples");
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_generate_samples_within_unit() {
    let kernel = SsaoKernel::generate(32);
    for i in 0..32 {
        let len = (kernel.samples[i][0].powi(2)
            + kernel.samples[i][1].powi(2)
            + kernel.samples[i][2].powi(2))
        .sqrt();
        assert!(len <= 1.01, "Sample {i} exceeds unit hemisphere: {len}");
    }
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_zero_count_no_panic() {
    let kernel = SsaoKernel::generate(0);
    // Should not panic, all zeros
    assert_eq!(kernel.samples[0], [0.0; 4]);
}

// ═══════════════════════════════════════════════════════════════════════
// RenderError
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn render_error_device_display() {
    let e = RenderError::Device("adapter lost".into());
    assert_eq!(format!("{e}"), "GPU device error: adapter lost");
}

#[test]
fn render_error_shader_display() {
    let e = RenderError::Shader("compile failed".into());
    assert_eq!(format!("{e}"), "shader/pipeline error: compile failed");
}

#[test]
fn render_error_asset_load_display() {
    let e = RenderError::AssetLoad {
        asset: "texture".into(),
        detail: "not found".into(),
    };
    assert_eq!(format!("{e}"), "failed to load texture: not found");
}

#[test]
fn render_error_surface_display() {
    let e = RenderError::Surface("lost".into());
    assert_eq!(format!("{e}"), "surface error: lost");
}

#[test]
fn render_error_graph_display() {
    let e = RenderError::Graph("cycle".into());
    assert_eq!(format!("{e}"), "render graph error: cycle");
}

#[test]
fn render_error_material_display() {
    let e = RenderError::Material("missing array".into());
    assert_eq!(format!("{e}"), "material error: missing array");
}

#[test]
fn render_error_post_process_display() {
    let e = RenderError::PostProcess("bloom".into());
    assert_eq!(format!("{e}"), "post-processing error: bloom");
}

#[test]
fn render_error_shadow_display() {
    let e = RenderError::Shadow("cascade overflow".into());
    assert_eq!(format!("{e}"), "shadow error: cascade overflow");
}

#[test]
fn render_error_animation_display() {
    let e = RenderError::Animation("missing clip".into());
    assert_eq!(format!("{e}"), "animation error: missing clip");
}

#[test]
fn render_error_image_display() {
    let e = RenderError::Image("corrupt png".into());
    assert_eq!(format!("{e}"), "image error: corrupt png");
}

#[test]
fn render_error_wgpu_display() {
    let e = RenderError::Wgpu("validation".into());
    assert_eq!(format!("{e}"), "wgpu error: validation");
}

#[test]
fn render_error_io_from_std() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
    let err: RenderError = io_err.into();
    let msg = format!("{err}");
    assert!(msg.contains("gone"), "Should contain io error: {msg}");
}

#[test]
fn render_result_ok() {
    let r: RenderResult<i32> = Ok(42);
    assert_eq!(r.unwrap(), 42);
}

// ═══════════════════════════════════════════════════════════════════════
// WeatherKind enum values
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn weather_kind_none_ne_rain() {
    assert_ne!(WeatherKind::None, WeatherKind::Rain);
}

#[test]
fn weather_kind_snow_ne_sandstorm() {
    assert_ne!(WeatherKind::Snow, WeatherKind::Sandstorm);
}

#[test]
fn weather_kind_wind_trails_ne_none() {
    assert_ne!(WeatherKind::WindTrails, WeatherKind::None);
}

#[test]
fn weather_kind_equality() {
    assert_eq!(WeatherKind::Rain, WeatherKind::Rain);
    assert_eq!(WeatherKind::Snow, WeatherKind::Snow);
    assert_eq!(WeatherKind::Sandstorm, WeatherKind::Sandstorm);
}

// ═══════════════════════════════════════════════════════════════════════
// DebugQuad
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn debug_quad_creates_6_vertices() {
    let verts = create_screen_quad();
    assert_eq!(verts.len(), 6); // 2 triangles for fullscreen quad
}

#[test]
fn debug_quad_uvs_in_range() {
    let verts = create_screen_quad();
    for v in &verts {
        assert!(v.uv[0] >= 0.0 && v.uv[0] <= 1.0, "U out of range");
        assert!(v.uv[1] >= 0.0 && v.uv[1] <= 1.0, "V out of range");
    }
}
