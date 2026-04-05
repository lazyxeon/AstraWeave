//! Mutation-resistant comprehensive tests for aw_asset_cli.
//!
//! Tests ColorSpace, NormalYConvention, CompressionFormat, BakeConfig defaults,
//! TextureMetadata JSON roundtrip, and infer_config_from_path.

use aw_asset_cli::*;
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════
// ColorSpace enum
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn color_space_srgb_eq() {
    assert_eq!(ColorSpace::Srgb, ColorSpace::Srgb);
}

#[test]
fn color_space_linear_eq() {
    assert_eq!(ColorSpace::Linear, ColorSpace::Linear);
}

#[test]
fn color_space_srgb_ne_linear() {
    assert_ne!(ColorSpace::Srgb, ColorSpace::Linear);
}

#[test]
fn color_space_debug() {
    let s = format!("{:?}", ColorSpace::Srgb);
    assert!(s.contains("Srgb"), "debug: {s}");
}

#[test]
fn color_space_clone() {
    let c = ColorSpace::Linear;
    let c2 = c;
    assert_eq!(c, c2);
}

#[test]
fn color_space_json_roundtrip() {
    let json = serde_json::to_string(&ColorSpace::Srgb).unwrap();
    let back: ColorSpace = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ColorSpace::Srgb);
}

#[test]
fn color_space_linear_json_roundtrip() {
    let json = serde_json::to_string(&ColorSpace::Linear).unwrap();
    let back: ColorSpace = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ColorSpace::Linear);
}

// ═══════════════════════════════════════════════════════════════════════════
// NormalYConvention enum
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn normal_y_opengl_eq() {
    assert_eq!(NormalYConvention::OpenGl, NormalYConvention::OpenGl);
}

#[test]
fn normal_y_directx_eq() {
    assert_eq!(NormalYConvention::DirectX, NormalYConvention::DirectX);
}

#[test]
fn normal_y_ne() {
    assert_ne!(NormalYConvention::OpenGl, NormalYConvention::DirectX);
}

#[test]
fn normal_y_clone() {
    let n = NormalYConvention::OpenGl;
    let n2 = n;
    assert_eq!(n, n2);
}

#[test]
fn normal_y_json_roundtrip() {
    let json = serde_json::to_string(&NormalYConvention::DirectX).unwrap();
    let back: NormalYConvention = serde_json::from_str(&json).unwrap();
    assert_eq!(back, NormalYConvention::DirectX);
}

// ═══════════════════════════════════════════════════════════════════════════
// CompressionFormat enum
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn compression_format_bc1_eq() {
    assert_eq!(CompressionFormat::Bc1, CompressionFormat::Bc1);
}

#[test]
fn compression_format_bc3_eq() {
    assert_eq!(CompressionFormat::Bc3, CompressionFormat::Bc3);
}

#[test]
fn compression_format_bc5_eq() {
    assert_eq!(CompressionFormat::Bc5, CompressionFormat::Bc5);
}

#[test]
fn compression_format_bc7_eq() {
    assert_eq!(CompressionFormat::Bc7, CompressionFormat::Bc7);
}

#[test]
fn compression_format_none_eq() {
    assert_eq!(CompressionFormat::None, CompressionFormat::None);
}

#[test]
fn compression_format_ne() {
    assert_ne!(CompressionFormat::Bc1, CompressionFormat::Bc7);
    assert_ne!(CompressionFormat::None, CompressionFormat::Bc3);
}

#[test]
fn compression_format_clone() {
    let c = CompressionFormat::Bc5;
    let c2 = c;
    assert_eq!(c, c2);
}

#[test]
fn compression_format_json_roundtrip() {
    let json = serde_json::to_string(&CompressionFormat::Bc7).unwrap();
    let back: CompressionFormat = serde_json::from_str(&json).unwrap();
    assert_eq!(back, CompressionFormat::Bc7);
}

// ═══════════════════════════════════════════════════════════════════════════
// BakeConfig defaults
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bake_config_default_color_space() {
    let cfg = BakeConfig::default();
    assert_eq!(cfg.color_space, ColorSpace::Srgb);
}

#[test]
fn bake_config_default_not_normal_map() {
    let cfg = BakeConfig::default();
    assert!(!cfg.is_normal_map);
}

#[test]
fn bake_config_default_normal_y_opengl() {
    let cfg = BakeConfig::default();
    assert_eq!(cfg.normal_y_convention, NormalYConvention::OpenGl);
}

#[test]
fn bake_config_default_compression_bc7() {
    let cfg = BakeConfig::default();
    assert_eq!(cfg.compression, CompressionFormat::Bc7);
}

#[test]
fn bake_config_default_generate_mipmaps() {
    let cfg = BakeConfig::default();
    assert!(cfg.generate_mipmaps);
}

#[test]
fn bake_config_default_output_format_ktx2() {
    let cfg = BakeConfig::default();
    assert_eq!(cfg.output_format, "ktx2");
}

#[test]
fn bake_config_clone() {
    let cfg = BakeConfig::default();
    let cfg2 = cfg.clone();
    assert_eq!(cfg2.color_space, ColorSpace::Srgb);
    assert_eq!(cfg2.compression, CompressionFormat::Bc7);
    assert_eq!(cfg2.output_format, "ktx2");
}

// ═══════════════════════════════════════════════════════════════════════════
// TextureMetadata JSON roundtrip
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn texture_metadata_json_roundtrip() {
    let meta = TextureMetadata {
        source_path: "textures/grass.png".to_string(),
        output_path: "baked/grass.ktx2".to_string(),
        color_space: ColorSpace::Srgb,
        normal_y_convention: None,
        compression: CompressionFormat::Bc7,
        mip_levels: 9,
        dimensions: (512, 512),
        sha256: "abc123".to_string(),
    };
    let json = serde_json::to_string(&meta).unwrap();
    let back: TextureMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(back.source_path, "textures/grass.png");
    assert_eq!(back.output_path, "baked/grass.ktx2");
    assert_eq!(back.color_space, ColorSpace::Srgb);
    assert!(back.normal_y_convention.is_none());
    assert_eq!(back.compression, CompressionFormat::Bc7);
    assert_eq!(back.mip_levels, 9);
    assert_eq!(back.dimensions, (512, 512));
    assert_eq!(back.sha256, "abc123");
}

#[test]
fn texture_metadata_normal_map_json() {
    let meta = TextureMetadata {
        source_path: "normal.png".into(),
        output_path: "normal.ktx2".into(),
        color_space: ColorSpace::Linear,
        normal_y_convention: Some(NormalYConvention::OpenGl),
        compression: CompressionFormat::Bc5,
        mip_levels: 1,
        dimensions: (1024, 1024),
        sha256: "def456".into(),
    };
    let json = serde_json::to_string(&meta).unwrap();
    let back: TextureMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(back.normal_y_convention, Some(NormalYConvention::OpenGl));
    assert_eq!(back.compression, CompressionFormat::Bc5);
    assert_eq!(back.color_space, ColorSpace::Linear);
}

#[test]
fn texture_metadata_clone() {
    let meta = TextureMetadata {
        source_path: "a".into(),
        output_path: "b".into(),
        color_space: ColorSpace::Srgb,
        normal_y_convention: None,
        compression: CompressionFormat::None,
        mip_levels: 1,
        dimensions: (1, 1),
        sha256: "x".into(),
    };
    let m2 = meta.clone();
    assert_eq!(m2.source_path, "a");
    assert_eq!(m2.compression, CompressionFormat::None);
}

// ═══════════════════════════════════════════════════════════════════════════
// infer_config_from_path
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn infer_albedo_texture() {
    let cfg = aw_asset_cli::texture_baker::infer_config_from_path(Path::new("grass_albedo.png"));
    assert_eq!(cfg.color_space, ColorSpace::Srgb);
    assert!(!cfg.is_normal_map);
    assert_eq!(cfg.compression, CompressionFormat::Bc7);
}

#[test]
fn infer_normal_map() {
    let cfg = aw_asset_cli::texture_baker::infer_config_from_path(Path::new("wall_normal.png"));
    assert_eq!(cfg.color_space, ColorSpace::Linear);
    assert!(cfg.is_normal_map);
    assert_eq!(cfg.compression, CompressionFormat::Bc5);
}

#[test]
fn infer_orm_map() {
    let cfg = aw_asset_cli::texture_baker::infer_config_from_path(Path::new("metal_orm.png"));
    assert_eq!(cfg.color_space, ColorSpace::Linear);
    assert!(!cfg.is_normal_map);
    assert_eq!(cfg.compression, CompressionFormat::Bc7);
}

#[test]
fn infer_roughness_map() {
    let cfg = aw_asset_cli::texture_baker::infer_config_from_path(Path::new("stone_roughness.png"));
    assert_eq!(cfg.color_space, ColorSpace::Linear);
    assert!(!cfg.is_normal_map);
}

#[test]
fn infer_metallic_map() {
    let cfg = aw_asset_cli::texture_baker::infer_config_from_path(Path::new("chrome_metallic.png"));
    assert_eq!(cfg.color_space, ColorSpace::Linear);
}

#[test]
fn infer_ao_map() {
    let cfg = aw_asset_cli::texture_baker::infer_config_from_path(Path::new("base_ao.png"));
    assert_eq!(cfg.color_space, ColorSpace::Linear);
}

#[test]
fn infer_default_is_srgb_bc7() {
    let cfg = aw_asset_cli::texture_baker::infer_config_from_path(Path::new("diffuse_color.png"));
    assert_eq!(cfg.color_space, ColorSpace::Srgb);
    assert_eq!(cfg.compression, CompressionFormat::Bc7);
}

#[test]
fn infer_n_suffix() {
    // Files ending with _n.png should be normal maps
    let cfg = aw_asset_cli::texture_baker::infer_config_from_path(Path::new("wall_n.png"));
    assert!(
        cfg.is_normal_map,
        "file ending _n.png should be detected as normal map"
    );
    assert_eq!(cfg.color_space, ColorSpace::Linear);
}

#[test]
fn infer_mra_suffix() {
    let cfg = aw_asset_cli::texture_baker::infer_config_from_path(Path::new("rock_mra.png"));
    assert_eq!(cfg.color_space, ColorSpace::Linear);
}
