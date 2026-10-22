//! Integration tests for MaterialManager validation functionality
//! These tests run independently of the main crate to avoid renderer.rs compilation issues

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::material::{
    validate_array_layout, validate_material_pack, ArrayLayout, MaterialLayerDesc, MaterialPackDesc,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_validate_material_pack_empty_biome() {
    let pack = MaterialPackDesc {
        biome: String::new(),
        layers: vec![],
    };

    let result = validate_material_pack(&pack);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Biome name cannot be empty"));
}

#[test]
fn test_validate_material_pack_duplicate_keys() {
    let pack = MaterialPackDesc {
        biome: "test".to_string(),
        layers: vec![
            MaterialLayerDesc {
                key: "grass".to_string(),
                albedo: Some(PathBuf::from("grass.png")),
                normal: None,
                mra: None,
                metallic: None,
                roughness: None,
                ao: None,
                tiling: [1.0, 1.0],
                triplanar_scale: 1.0,
                atlas: None,
            },
            MaterialLayerDesc {
                key: "grass".to_string(), // Duplicate!
                albedo: Some(PathBuf::from("grass2.png")),
                normal: None,
                mra: None,
                metallic: None,
                roughness: None,
                ao: None,
                tiling: [1.0, 1.0],
                triplanar_scale: 1.0,
                atlas: None,
            },
        ],
    };

    let result = validate_material_pack(&pack);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Duplicate layer key"));
    assert!(err_msg.contains("grass"));
}

#[test]
fn test_validate_material_pack_invalid_tiling() {
    let pack = MaterialPackDesc {
        biome: "test".to_string(),
        layers: vec![MaterialLayerDesc {
            key: "dirt".to_string(),
            albedo: Some(PathBuf::from("dirt.png")),
            normal: None,
            mra: None,
            metallic: None,
            roughness: None,
            ao: None,
            tiling: [-1.0, 1.0], // Invalid!
            triplanar_scale: 1.0,
            atlas: None,
        }],
    };

    let result = validate_material_pack(&pack);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("tiling values must be positive"));
}

#[test]
fn test_validate_material_pack_invalid_triplanar() {
    let pack = MaterialPackDesc {
        biome: "test".to_string(),
        layers: vec![MaterialLayerDesc {
            key: "stone".to_string(),
            albedo: Some(PathBuf::from("stone.png")),
            normal: None,
            mra: None,
            metallic: None,
            roughness: None,
            ao: None,
            tiling: [1.0, 1.0],
            triplanar_scale: -0.5, // Invalid!
            atlas: None,
        }],
    };

    let result = validate_material_pack(&pack);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("triplanar_scale must be positive"));
}

#[test]
fn test_validate_material_pack_valid() {
    let pack = MaterialPackDesc {
        biome: "temperate".to_string(),
        layers: vec![
            MaterialLayerDesc {
                key: "grass".to_string(),
                albedo: Some(PathBuf::from("grass.png")),
                normal: Some(PathBuf::from("grass_n.png")),
                mra: Some(PathBuf::from("grass_mra.png")),
                metallic: None,
                roughness: None,
                ao: None,
                tiling: [2.0, 2.0],
                triplanar_scale: 1.0,
                atlas: None,
            },
            MaterialLayerDesc {
                key: "dirt".to_string(),
                albedo: Some(PathBuf::from("dirt.png")),
                normal: None,
                mra: None,
                metallic: None,
                roughness: None,
                ao: None,
                tiling: [1.5, 1.5],
                triplanar_scale: 0.8,
                atlas: None,
            },
        ],
    };

    let result = validate_material_pack(&pack);
    assert!(result.is_ok(), "Valid pack should pass validation");
}

#[test]
fn test_validate_array_layout_duplicate_indices() {
    let mut layer_indices = HashMap::new();
    layer_indices.insert("grass".to_string(), 0);
    layer_indices.insert("dirt".to_string(), 0); // Duplicate index!

    let layout = ArrayLayout {
        layer_indices,
        count: 2,
    };

    let result = validate_array_layout(&layout);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Duplicate array index 0"));
}

#[test]
fn test_validate_array_layout_valid() {
    let mut layer_indices = HashMap::new();
    layer_indices.insert("grass".to_string(), 0);
    layer_indices.insert("dirt".to_string(), 1);

    let layout = ArrayLayout {
        layer_indices,
        count: 2,
    };

    let result = validate_array_layout(&layout);
    assert!(result.is_ok(), "Valid layout should pass validation");
}

#[test]
fn test_validate_array_layout_with_gaps() {
    let mut layer_indices = HashMap::new();
    layer_indices.insert("grass".to_string(), 0);
    layer_indices.insert("dirt".to_string(), 5); // Gap: 1-4 missing

    let layout = ArrayLayout {
        layer_indices,
        count: 6,
    };

    // Should succeed with warning (not error)
    let result = validate_array_layout(&layout);
    assert!(
        result.is_ok(),
        "Layout with gaps should pass (with warning)"
    );
}
