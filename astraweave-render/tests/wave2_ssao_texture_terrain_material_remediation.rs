//! Wave 2 Proactive Remediation — ssao, texture, terrain, material_extended
//!
//! CPU-testable APIs for remaining render modules:
//! - ssao.rs (68 mutants): SsaoQuality, SsaoKernel, SsaoConfig
//! - texture.rs (81 mutants): TextureUsage enum methods
//! - terrain.rs (84 mutants): biome_to_id, vegetation_type_to_id, TerrainRenderer
//! - material_extended.rs (86 mutants): feature flags, presets, TOML to_gpu

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::material_extended::{
    MaterialDefinitionExtended, MaterialGpuExtended, MATERIAL_FLAG_ANISOTROPY,
    MATERIAL_FLAG_CLEARCOAT, MATERIAL_FLAG_SHEEN, MATERIAL_FLAG_SUBSURFACE,
    MATERIAL_FLAG_TRANSMISSION,
};
#[cfg(feature = "ssao")]
use astraweave_render::ssao::{SsaoConfig, SsaoKernel, SsaoQuality};
use astraweave_render::terrain::{generate_terrain_preview, TerrainRenderer};
use astraweave_render::texture::TextureUsage;
use astraweave_terrain::WorldConfig;
use glam::Vec3;

// ═══════════════════════════════════════════════════════════════════════════════
// SsaoQuality — sample_count, radius, blur_kernel_size per level
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_sample_counts() {
    assert_eq!(SsaoQuality::Low.sample_count(), 8);
    assert_eq!(SsaoQuality::Medium.sample_count(), 16);
    assert_eq!(SsaoQuality::High.sample_count(), 32);
    assert_eq!(SsaoQuality::Ultra.sample_count(), 64);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_radius_increasing() {
    let r_low = SsaoQuality::Low.radius();
    let r_med = SsaoQuality::Medium.radius();
    let r_high = SsaoQuality::High.radius();
    let r_ultra = SsaoQuality::Ultra.radius();
    assert!(r_low < r_med);
    assert!(r_med < r_high);
    assert!(r_high < r_ultra);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_exact_radius_values() {
    assert!((SsaoQuality::Low.radius() - 0.5).abs() < 1e-6);
    assert!((SsaoQuality::Medium.radius() - 1.0).abs() < 1e-6);
    assert!((SsaoQuality::High.radius() - 1.5).abs() < 1e-6);
    assert!((SsaoQuality::Ultra.radius() - 2.0).abs() < 1e-6);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_blur_kernel_parity() {
    // Low has no blur (0), others have odd kernel sizes
    assert_eq!(SsaoQuality::Low.blur_kernel_size(), 0);
    assert_eq!(SsaoQuality::Medium.blur_kernel_size(), 3);
    assert_eq!(SsaoQuality::High.blur_kernel_size(), 5);
    assert_eq!(SsaoQuality::Ultra.blur_kernel_size(), 7);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_blur_sizes_increasing() {
    let sizes: Vec<u32> = [
        SsaoQuality::Low,
        SsaoQuality::Medium,
        SsaoQuality::High,
        SsaoQuality::Ultra,
    ]
    .iter()
    .map(|q| q.blur_kernel_size())
    .collect();
    for pair in sizes.windows(2) {
        assert!(
            pair[1] > pair[0],
            "Blur sizes not increasing: {} -> {}",
            pair[0],
            pair[1]
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SsaoConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_enabled() {
    let config = SsaoConfig::default();
    assert!(config.enabled);
    assert_eq!(config.quality, SsaoQuality::Medium);
    assert!((config.radius - 1.0).abs() < 1e-6);
    assert!((config.bias - 0.025).abs() < 1e-6);
    assert!((config.intensity - 1.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SsaoKernel — hemisphere samples
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_samples_in_hemisphere() {
    for n in [8, 16, 32, 64] {
        let kernel = SsaoKernel::generate(n);
        for i in 0..n as usize {
            let s = kernel.samples[i];
            assert!(s[2] >= 0.0, "Sample {} z < 0 for n={}", i, n);
            let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
            assert!(
                len <= 1.0 + 1e-5,
                "Sample {} len {} > 1 for n={}",
                i,
                len,
                n
            );
            assert!(len > 0.0, "Sample {} zero-length for n={}", i, n);
        }
    }
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_unused_samples_zeroed() {
    let kernel = SsaoKernel::generate(16);
    for i in 16..64 {
        assert_eq!(
            kernel.samples[i],
            [0.0, 0.0, 0.0, 0.0],
            "Unused sample {} not zeroed",
            i
        );
    }
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_4th_component_zero() {
    let kernel = SsaoKernel::generate(32);
    for i in 0..32 {
        assert_eq!(kernel.samples[i][3], 0.0, "Sample {} w != 0", i);
    }
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_scale_increases_with_index() {
    let kernel = SsaoKernel::generate(32);
    let len_first = {
        let s = kernel.samples[0];
        (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
    };
    let len_last = {
        let s = kernel.samples[31];
        (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
    };
    assert!(
        len_last > len_first,
        "Last {} should be farther than first {}",
        len_last,
        len_first
    );
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_different_sample_counts_different() {
    let k8 = SsaoKernel::generate(8);
    let k16 = SsaoKernel::generate(16);
    // Sample 0 should be the same (same formula)
    // But sample 8 in k16 should be non-zero while k8[8] is zero
    assert_eq!(k8.samples[8], [0.0, 0.0, 0.0, 0.0]);
    assert_ne!(k16.samples[8], [0.0, 0.0, 0.0, 0.0]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TextureUsage — format, needs_mipmaps, description
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn texture_usage_albedo_srgb() {
    assert_eq!(
        TextureUsage::Albedo.format(),
        wgpu::TextureFormat::Rgba8UnormSrgb
    );
}

#[test]
fn texture_usage_emissive_srgb() {
    assert_eq!(
        TextureUsage::Emissive.format(),
        wgpu::TextureFormat::Rgba8UnormSrgb
    );
}

#[test]
fn texture_usage_normal_linear() {
    assert_eq!(
        TextureUsage::Normal.format(),
        wgpu::TextureFormat::Rgba8Unorm
    );
}

#[test]
fn texture_usage_mra_linear() {
    assert_eq!(TextureUsage::MRA.format(), wgpu::TextureFormat::Rgba8Unorm);
}

#[test]
fn texture_usage_height_linear() {
    assert_eq!(
        TextureUsage::Height.format(),
        wgpu::TextureFormat::Rgba8Unorm
    );
}

#[test]
fn texture_usage_mipmaps_albedo_emissive_mra() {
    assert!(TextureUsage::Albedo.needs_mipmaps());
    assert!(TextureUsage::Emissive.needs_mipmaps());
    assert!(TextureUsage::MRA.needs_mipmaps());
}

#[test]
fn texture_usage_no_mipmaps_normal_height() {
    assert!(!TextureUsage::Normal.needs_mipmaps());
    assert!(!TextureUsage::Height.needs_mipmaps());
}

#[test]
fn texture_usage_descriptions_nonempty() {
    let usages = [
        TextureUsage::Albedo,
        TextureUsage::Normal,
        TextureUsage::MRA,
        TextureUsage::Emissive,
        TextureUsage::Height,
    ];
    for u in &usages {
        let desc = u.description();
        assert!(!desc.is_empty(), "{:?} has empty description", u);
    }
}

#[test]
fn texture_usage_descriptions_unique() {
    let descs: Vec<&str> = [
        TextureUsage::Albedo,
        TextureUsage::Normal,
        TextureUsage::MRA,
        TextureUsage::Emissive,
        TextureUsage::Height,
    ]
    .iter()
    .map(|u| u.description())
    .collect();
    for i in 0..descs.len() {
        for j in (i + 1)..descs.len() {
            assert_ne!(
                descs[i], descs[j],
                "Descriptions {} and {} are identical",
                i, j
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TerrainRenderer — biome_to_id, vegetation, mesh generation
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn terrain_renderer_default_chunk_size() {
    let config = WorldConfig::default();
    let renderer = TerrainRenderer::new(config);
    // Access chunk_size indirectly via world_generator
    assert!(renderer.world_generator().config().chunk_size > 0.0);
}

#[test]
fn terrain_renderer_generates_mesh() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let chunk_id = astraweave_terrain::ChunkId::new(0, 0);
    let mesh = renderer.get_or_generate_chunk_mesh(chunk_id).unwrap();
    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.indices.is_empty());
    assert_eq!(mesh.chunk_id, chunk_id);
}

#[test]
fn terrain_renderer_caches_meshes() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let chunk_id = astraweave_terrain::ChunkId::new(1, 1);
    let _mesh1 = renderer.get_or_generate_chunk_mesh(chunk_id).unwrap();
    // Second call should hit cache
    let mesh2 = renderer.get_or_generate_chunk_mesh(chunk_id).unwrap();
    assert_eq!(mesh2.chunk_id, chunk_id);
}

#[test]
fn terrain_renderer_get_loaded_mesh_after_generation() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let chunk_id = astraweave_terrain::ChunkId::new(2, 2);
    assert!(renderer.get_loaded_mesh(chunk_id).is_none());
    let _mesh = renderer.get_or_generate_chunk_mesh(chunk_id).unwrap();
    assert!(renderer.get_loaded_mesh(chunk_id).is_some());
}

#[test]
fn terrain_renderer_chunks_in_radius() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let center = Vec3::new(128.0, 0.0, 128.0);
    let chunks = renderer.get_chunks_in_radius(center, 1).unwrap();
    assert!(!chunks.is_empty());
    for id in &chunks {
        assert!(renderer.get_loaded_mesh(*id).is_some());
    }
}

#[test]
fn terrain_preview_generates_correct_size() {
    let config = WorldConfig::default();
    let center = Vec3::new(0.0, 0.0, 0.0);
    let preview = generate_terrain_preview(&config, center, 16).unwrap();
    assert_eq!(preview.len(), 16 * 16);
}

#[test]
fn terrain_preview_values_finite() {
    let config = WorldConfig::default();
    let preview = generate_terrain_preview(&config, Vec3::ZERO, 8).unwrap();
    for &h in &preview {
        assert!(h.is_finite(), "Height should be finite, got {}", h);
    }
}

#[test]
fn terrain_mesh_vertices_have_valid_normals() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let chunk_id = astraweave_terrain::ChunkId::new(0, 0);
    let mesh = renderer.get_or_generate_chunk_mesh(chunk_id).unwrap();
    for v in &mesh.vertices {
        let n = Vec3::from_array(v.normal);
        let len = n.length();
        assert!(len > 0.5 && len < 1.5, "Normal length {} out of range", len);
    }
}

#[test]
fn terrain_mesh_indices_valid() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let chunk_id = astraweave_terrain::ChunkId::new(0, 0);
    let mesh = renderer.get_or_generate_chunk_mesh(chunk_id).unwrap();
    let vc = mesh.vertices.len() as u32;
    for &idx in &mesh.indices {
        assert!(idx < vc, "Index {} >= vertex count {}", idx, vc);
    }
}

#[test]
fn terrain_mesh_indices_multiple_of_three() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let mesh = renderer
        .get_or_generate_chunk_mesh(astraweave_terrain::ChunkId::new(0, 0))
        .unwrap();
    assert_eq!(
        mesh.indices.len() % 3,
        0,
        "Triangle mesh should have indices divisible by 3"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// MaterialGpuExtended — feature flags, presets, defaults
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn material_default_size_256_bytes() {
    assert_eq!(std::mem::size_of::<MaterialGpuExtended>(), 256);
    assert_eq!(std::mem::align_of::<MaterialGpuExtended>(), 16);
}

#[test]
fn material_default_no_features() {
    let mat = MaterialGpuExtended::default();
    assert!(!mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(!mat.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert!(!mat.has_feature(MATERIAL_FLAG_SUBSURFACE));
    assert!(!mat.has_feature(MATERIAL_FLAG_SHEEN));
    assert!(!mat.has_feature(MATERIAL_FLAG_TRANSMISSION));
}

#[test]
fn material_default_base_color_white() {
    let mat = MaterialGpuExtended::default();
    assert_eq!(mat.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn material_default_roughness_half() {
    let mat = MaterialGpuExtended::default();
    assert_eq!(mat.roughness_factor, 0.5);
    assert_eq!(mat.metallic_factor, 0.0);
}

#[test]
fn material_default_ior_1_5() {
    let mat = MaterialGpuExtended::default();
    assert_eq!(mat.ior, 1.5);
}

#[test]
fn material_car_paint_has_clearcoat() {
    let mat = MaterialGpuExtended::car_paint(Vec3::new(0.8, 0.0, 0.0), 0.9, 0.3);
    assert!(mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(!mat.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert_eq!(mat.clearcoat_strength, 1.0);
    assert_eq!(mat.metallic_factor, 0.9);
    assert_eq!(mat.roughness_factor, 0.3);
    assert_eq!(mat.base_color_factor[0], 0.8);
}

#[test]
fn material_brushed_metal_has_anisotropy() {
    let mat = MaterialGpuExtended::brushed_metal(Vec3::splat(0.9), 0.4, 0.8, 0.5);
    assert!(mat.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert_eq!(mat.metallic_factor, 1.0);
    assert_eq!(mat.anisotropy_strength, 0.8);
    assert_eq!(mat.anisotropy_rotation, 0.5);
}

#[test]
fn material_skin_has_subsurface() {
    let mat = MaterialGpuExtended::skin(
        Vec3::new(0.95, 0.8, 0.7),
        Vec3::new(0.9, 0.3, 0.3),
        1.5,
        0.7,
    );
    assert!(mat.has_feature(MATERIAL_FLAG_SUBSURFACE));
    assert_eq!(mat.metallic_factor, 0.0);
    assert_eq!(mat.subsurface_scale, 0.7);
    assert_eq!(mat.subsurface_radius, 1.5);
    assert_eq!(mat.subsurface_color[0], 0.9);
}

#[test]
fn material_velvet_has_sheen() {
    let mat = MaterialGpuExtended::velvet(Vec3::new(0.5, 0.0, 0.1), Vec3::new(1.0, 0.9, 0.8), 0.3);
    assert!(mat.has_feature(MATERIAL_FLAG_SHEEN));
    assert_eq!(mat.sheen_roughness, 0.3);
    assert_eq!(mat.sheen_color[0], 1.0);
    assert_eq!(mat.sheen_color[1], 0.9);
}

#[test]
fn material_glass_has_transmission() {
    let mat =
        MaterialGpuExtended::glass(Vec3::ONE, 0.05, 0.95, 1.5, Vec3::new(0.9, 1.0, 0.9), 10.0);
    assert!(mat.has_feature(MATERIAL_FLAG_TRANSMISSION));
    assert_eq!(mat.transmission_factor, 0.95);
    assert_eq!(mat.ior, 1.5);
    assert_eq!(mat.attenuation_distance, 10.0);
}

#[test]
fn material_enable_disable_features() {
    let mut mat = MaterialGpuExtended::default();
    mat.enable_feature(MATERIAL_FLAG_CLEARCOAT);
    mat.enable_feature(MATERIAL_FLAG_SHEEN);
    assert!(mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(mat.has_feature(MATERIAL_FLAG_SHEEN));

    mat.disable_feature(MATERIAL_FLAG_CLEARCOAT);
    assert!(!mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(mat.has_feature(MATERIAL_FLAG_SHEEN));
}

#[test]
fn material_multiple_features_independent() {
    let mut mat = MaterialGpuExtended::default();
    mat.enable_feature(
        MATERIAL_FLAG_CLEARCOAT | MATERIAL_FLAG_ANISOTROPY | MATERIAL_FLAG_SUBSURFACE,
    );
    assert!(mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(mat.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert!(mat.has_feature(MATERIAL_FLAG_SUBSURFACE));
    assert!(!mat.has_feature(MATERIAL_FLAG_SHEEN));
}

#[test]
fn material_flags_are_distinct_powers_of_two() {
    let flags = [
        MATERIAL_FLAG_CLEARCOAT,
        MATERIAL_FLAG_ANISOTROPY,
        MATERIAL_FLAG_SUBSURFACE,
        MATERIAL_FLAG_SHEEN,
        MATERIAL_FLAG_TRANSMISSION,
    ];
    for i in 0..flags.len() {
        assert!(
            flags[i].is_power_of_two(),
            "Flag {} is not power of two",
            flags[i]
        );
        for j in (i + 1)..flags.len() {
            assert_ne!(flags[i], flags[j], "Flags {} and {} are identical", i, j);
        }
    }
}

#[test]
fn material_toml_to_gpu_sets_flags_automatically() {
    let def = MaterialDefinitionExtended {
        name: "test".to_string(),
        albedo: None,
        normal: None,
        orm: None,
        base_color_factor: [1.0; 4],
        metallic_factor: 0.5,
        roughness_factor: 0.5,
        occlusion_strength: 1.0,
        emissive_factor: [0.0; 3],
        clearcoat_strength: 0.8, // > 0 → CLEARCOAT flag
        clearcoat_roughness: 0.03,
        clearcoat_normal: None,
        anisotropy_strength: 0.5, // > 0.001 → ANISOTROPY flag
        anisotropy_rotation: 0.0,
        subsurface_color: [1.0; 3],
        subsurface_scale: 0.3, // > 0 → SUBSURFACE flag
        subsurface_radius: 1.0,
        thickness_map: None,
        sheen_color: [0.5, 0.5, 0.5], // max > 0 → SHEEN flag
        sheen_roughness: 0.5,
        transmission_factor: 0.9, // > 0 → TRANSMISSION flag
        ior: 1.5,
        attenuation_color: [1.0; 3],
        attenuation_distance: 1.0,
    };
    let gpu = def.to_gpu(0, 1, 2, 3, 4);
    assert!(gpu.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(gpu.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert!(gpu.has_feature(MATERIAL_FLAG_SUBSURFACE));
    assert!(gpu.has_feature(MATERIAL_FLAG_SHEEN));
    assert!(gpu.has_feature(MATERIAL_FLAG_TRANSMISSION));
}

#[test]
fn material_toml_to_gpu_no_flags_when_zeroed() {
    let def = MaterialDefinitionExtended {
        name: "zero".to_string(),
        albedo: None,
        normal: None,
        orm: None,
        base_color_factor: [1.0; 4],
        metallic_factor: 0.0,
        roughness_factor: 0.5,
        occlusion_strength: 1.0,
        emissive_factor: [0.0; 3],
        clearcoat_strength: 0.0,
        clearcoat_roughness: 0.03,
        clearcoat_normal: None,
        anisotropy_strength: 0.0,
        anisotropy_rotation: 0.0,
        subsurface_color: [1.0; 3],
        subsurface_scale: 0.0,
        subsurface_radius: 1.0,
        thickness_map: None,
        sheen_color: [0.0, 0.0, 0.0],
        sheen_roughness: 0.5,
        transmission_factor: 0.0,
        ior: 1.5,
        attenuation_color: [1.0; 3],
        attenuation_distance: 1.0,
    };
    let gpu = def.to_gpu(0, 0, 0, 0, 0);
    assert_eq!(gpu.flags, 0);
}

#[test]
fn material_toml_preserves_texture_indices() {
    let def = MaterialDefinitionExtended {
        name: "indices_test".to_string(),
        albedo: None,
        normal: None,
        orm: None,
        base_color_factor: [1.0; 4],
        metallic_factor: 0.0,
        roughness_factor: 0.5,
        occlusion_strength: 1.0,
        emissive_factor: [0.0; 3],
        clearcoat_strength: 0.0,
        clearcoat_roughness: 0.03,
        clearcoat_normal: None,
        anisotropy_strength: 0.0,
        anisotropy_rotation: 0.0,
        subsurface_color: [1.0; 3],
        subsurface_scale: 0.0,
        subsurface_radius: 1.0,
        thickness_map: None,
        sheen_color: [0.0; 3],
        sheen_roughness: 0.5,
        transmission_factor: 0.0,
        ior: 1.5,
        attenuation_color: [1.0; 3],
        attenuation_distance: 1.0,
    };
    let gpu = def.to_gpu(10, 20, 30, 40, 50);
    assert_eq!(gpu.albedo_index, 10);
    assert_eq!(gpu.normal_index, 20);
    assert_eq!(gpu.orm_index, 30);
    assert_eq!(gpu.clearcoat_normal_index, 40);
    assert_eq!(gpu.thickness_index, 50);
}
