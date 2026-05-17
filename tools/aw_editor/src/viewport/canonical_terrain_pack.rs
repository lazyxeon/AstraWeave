//! Canonical biome pack loader for the editor's 32-layer terrain pipeline.
//!
//! P.2 sibling loader (Editor-Engine Render Parity campaign). Reads the same
//! on-disk schema that `astraweave_render::MaterialManager::load_pack_from_toml`
//! reads — `materials.toml` + `arrays.toml` under `assets/materials/<biome>/` —
//! but produces **CPU byte slices** instead of GPU texture arrays. The bytes
//! flow into `Renderer::set_terrain_materials` via `EngineRenderAdapter`,
//! which is the editor's terrain-forward consumer.
//!
//! The runtime's `MaterialManager` parses the same TOML schema but uploads to
//! `MaterialGpuArrays` (a separate bind group used by glTF model rendering).
//! The editor's terrain renders via `TerrainMaterialManager` (32-layer splat),
//! whose `set_material` consumes CPU bytes. The two pipelines are parallel,
//! not chained. Reusing the on-disk schema keeps both paths sourcing the same
//! authored content — the parity contract.
//!
//! **Schema sync risk**: MaterialManager's TOML deserializer types are private
//! local types inside `load_pack_from_toml` (astraweave-render/src/material.rs:
//! 419-449). This module redeclares an equivalent shape — they MUST be kept
//! in lockstep. Future sub-phase candidate: elevate to public canonical types
//! in `astraweave-render`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::imageops::FilterType;
use serde::Deserialize;

/// Albedo array resolution expected by `TerrainMaterialManager::set_material`
/// at default config (matches `terrain_biome_placeholder::PLACEHOLDER_ALBEDO_RES`).
pub const CANONICAL_ALBEDO_RES: u32 = 1024;

/// Normal / ORM array resolution expected by `TerrainMaterialManager::set_material`
/// at default config (matches `terrain_biome_placeholder::PLACEHOLDER_AUX_RES`).
pub const CANONICAL_AUX_RES: u32 = 512;

/// Single layer's resized CPU bytes ready for upload via
/// `TerrainMaterialManager::set_material`.
pub struct CanonicalLayerBytes {
    /// `CANONICAL_ALBEDO_RES × CANONICAL_ALBEDO_RES × 4` bytes (RGBA8).
    pub albedo: Option<Vec<u8>>,
    /// `CANONICAL_AUX_RES × CANONICAL_AUX_RES × 4` bytes (RGBA8 tangent-space normal).
    pub normal: Option<Vec<u8>>,
    /// `CANONICAL_AUX_RES × CANONICAL_AUX_RES × 4` bytes (metallic/roughness/AO RGBA8).
    pub mra: Option<Vec<u8>>,
    /// TOML-declared tiling (UV scale per layer). Default `[1.0, 1.0]`.
    pub uv_scale: [f32; 2],
}

/// Result of loading a canonical biome pack from disk.
///
/// `layers` is ordered by `arrays.toml` array index — layer 0 in the editor's
/// 32-layer splat sits at `layers[0]`, etc. The number of active entries
/// (`active_layer_count`) is the maximum array index + 1 across the arrays.toml
/// mapping.
pub struct CanonicalTerrainPack {
    pub biome_name: String,
    pub layers: Vec<CanonicalLayerBytes>,
    pub active_layer_count: u32,
}

// ─── On-disk schema (private; mirrors MaterialManager's private structs) ────

#[derive(Deserialize)]
struct MaterialsDoc {
    biome: BiomeHeader,
    #[serde(default)]
    layer: Vec<MaterialLayerToml>,
}

#[derive(Deserialize)]
struct BiomeHeader {
    name: String,
}

#[derive(Deserialize, Default)]
struct MaterialLayerToml {
    key: String,
    albedo: Option<String>,
    normal: Option<String>,
    mra: Option<String>,
    #[serde(default = "default_tiling")]
    tiling: [f32; 2],
}

fn default_tiling() -> [f32; 2] {
    [1.0, 1.0]
}

#[derive(Deserialize)]
struct ArraysDoc {
    layers: std::collections::HashMap<String, u32>,
}

// ─── Loader ─────────────────────────────────────────────────────────────────

/// Load a canonical biome pack from disk.
///
/// `biome_dir` must contain `materials.toml` and `arrays.toml`. Texture paths
/// inside `materials.toml` are resolved relative to `biome_dir` (matching
/// MaterialManager's resolution rule at material.rs:514).
///
/// Returns a `CanonicalTerrainPack` whose `layers` vec is sized to
/// `max(arrays_toml.values) + 1`. Layers that are present in `materials.toml`
/// but absent from `arrays.toml` are skipped (consistent with MaterialManager's
/// behavior at material.rs:505-510). Layers in `arrays.toml` but absent from
/// `materials.toml` produce `CanonicalLayerBytes` with all-`None` channels —
/// `set_material` will substitute grey/flat-normal/neutral-ORM defaults.
pub fn load_canonical_terrain_pack(biome_dir: &Path) -> Result<CanonicalTerrainPack> {
    let materials_toml_path = biome_dir.join("materials.toml");
    let arrays_toml_path = biome_dir.join("arrays.toml");

    let mats_src = std::fs::read_to_string(&materials_toml_path)
        .with_context(|| format!("read {}", materials_toml_path.display()))?;
    let mats_doc: MaterialsDoc = toml::from_str(&mats_src)
        .with_context(|| format!("parse {}", materials_toml_path.display()))?;
    if mats_doc.biome.name.is_empty() {
        anyhow::bail!(
            "biome name empty in {}",
            materials_toml_path.display()
        );
    }

    let arrays_src = std::fs::read_to_string(&arrays_toml_path)
        .with_context(|| format!("read {}", arrays_toml_path.display()))?;
    let arrays_doc: ArraysDoc = toml::from_str(&arrays_src)
        .with_context(|| format!("parse {}", arrays_toml_path.display()))?;

    let max_index = arrays_doc.layers.values().copied().max().unwrap_or(0);
    let layer_count = max_index + 1;
    let mut layers: Vec<CanonicalLayerBytes> = (0..layer_count)
        .map(|_| CanonicalLayerBytes {
            albedo: None,
            normal: None,
            mra: None,
            uv_scale: [1.0, 1.0],
        })
        .collect();

    for layer in mats_doc.layer {
        let Some(&idx) = arrays_doc.layers.get(&layer.key) else {
            log::warn!(
                "[canonical-terrain-pack] layer key '{}' present in materials.toml \
                 but missing from arrays.toml — skipping",
                layer.key
            );
            continue;
        };
        let slot = &mut layers[idx as usize];
        slot.uv_scale = layer.tiling;
        slot.albedo = layer
            .albedo
            .as_deref()
            .and_then(|p| load_albedo_bytes(&biome_dir.join(p)).ok());
        slot.normal = layer
            .normal
            .as_deref()
            .and_then(|p| load_aux_bytes(&biome_dir.join(p)).ok());
        slot.mra = layer
            .mra
            .as_deref()
            .and_then(|p| load_aux_bytes(&biome_dir.join(p)).ok());
    }

    Ok(CanonicalTerrainPack {
        biome_name: mats_doc.biome.name,
        layers,
        active_layer_count: layer_count,
    })
}

/// Read an albedo PNG and resize to `CANONICAL_ALBEDO_RES × CANONICAL_ALBEDO_RES`
/// RGBA8. The source is interpreted as already-sRGB-encoded (matches `image`
/// crate default + the on-disk grassland PNGs at 2048² sRGB).
fn load_albedo_bytes(path: &Path) -> Result<Vec<u8>> {
    let rgba = image::open(path)
        .with_context(|| format!("image::open {}", path.display()))?
        .to_rgba8();
    let resized = if rgba.dimensions() == (CANONICAL_ALBEDO_RES, CANONICAL_ALBEDO_RES) {
        rgba
    } else {
        image::imageops::resize(
            &rgba,
            CANONICAL_ALBEDO_RES,
            CANONICAL_ALBEDO_RES,
            FilterType::Triangle,
        )
    };
    Ok(resized.into_raw())
}

/// Read an aux PNG (normal or MRA) and resize to
/// `CANONICAL_AUX_RES × CANONICAL_AUX_RES` RGBA8.
fn load_aux_bytes(path: &Path) -> Result<Vec<u8>> {
    let rgba = image::open(path)
        .with_context(|| format!("image::open {}", path.display()))?
        .to_rgba8();
    let resized = if rgba.dimensions() == (CANONICAL_AUX_RES, CANONICAL_AUX_RES) {
        rgba
    } else {
        image::imageops::resize(
            &rgba,
            CANONICAL_AUX_RES,
            CANONICAL_AUX_RES,
            FilterType::Triangle,
        )
    };
    Ok(resized.into_raw())
}

/// Build a borrowed `LayerTextures<'a>` vec referencing the pack's CPU bytes.
///
/// The returned slice is ready to pass to `Renderer::set_terrain_materials`.
/// Length matches `pack.active_layer_count`; entries are ordered by array index.
#[cfg(feature = "terrain-splat-arrays")]
pub fn borrow_layer_textures(
    pack: &CanonicalTerrainPack,
) -> Vec<astraweave_render::LayerTextures<'_>> {
    pack.layers
        .iter()
        .map(|layer| astraweave_render::LayerTextures {
            albedo: layer.albedo.as_deref(),
            normal: layer.normal.as_deref(),
            orm: layer.mra.as_deref(),
            height: None,
        })
        .collect()
}

/// Build a `TerrainMaterialGpu` UBO for the pack.
///
/// Each active layer i gets `texture_indices = [i, i, i, i]` — albedo/normal/
/// orm/height all stored at array slice i (matches what the synthetic
/// placeholder path at `engine_adapter.rs:1366` produces). Per-layer
/// `uv_scale` is set from `materials.toml`'s `tiling` field.
#[cfg(feature = "terrain-splat-arrays")]
pub fn build_gpu_material(pack: &CanonicalTerrainPack) -> astraweave_render::TerrainMaterialGpu {
    let mut gpu_material = astraweave_render::TerrainMaterialGpu::default();
    gpu_material.active_layer_count = pack.active_layer_count;
    for (i, layer) in pack
        .layers
        .iter()
        .take(gpu_material.layers.len())
        .enumerate()
    {
        gpu_material.layers[i].texture_indices = [i as u32, i as u32, i as u32, i as u32];
        gpu_material.layers[i].uv_scale = layer.uv_scale;
    }
    gpu_material
}

// Helper paths for the editor's main.rs and harness consumers — keeps the
// `assets/materials/<biome>` convention single-sourced.

/// Build the conventional `assets/materials/<biome>` path used by both the
/// editor (relative to the project root) and the runtime asset pipeline.
pub fn biome_pack_path(materials_root: &Path, biome_name: &str) -> PathBuf {
    materials_root.join(biome_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: the loader parses the canonical grassland biome pack and
    /// returns 5 active layers (matching `assets/materials/grassland/arrays.toml`).
    /// Asset-dependent — skipped if the pack is absent on disk.
    #[test]
    fn loads_grassland_pack_when_present() {
        let materials_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/materials");
        let biome_dir = materials_root.join("grassland");
        if !biome_dir.join("materials.toml").exists() {
            eprintln!(
                "Skipping grassland canonical pack test — {} not present",
                biome_dir.join("materials.toml").display()
            );
            return;
        }
        let pack = load_canonical_terrain_pack(&biome_dir)
            .expect("canonical grassland pack should load");
        assert_eq!(pack.biome_name, "grassland");
        assert_eq!(pack.active_layer_count, 5, "grassland has 5 layers");
        assert_eq!(pack.layers.len(), 5);
        // All 5 layers should have albedo loaded (or absent if files missing).
        // At least one layer (grass at index 0) must have albedo present.
        assert!(
            pack.layers[0].albedo.is_some(),
            "grass layer must have albedo loaded"
        );
        // Albedo bytes must be 1024² × 4.
        if let Some(albedo) = &pack.layers[0].albedo {
            assert_eq!(
                albedo.len(),
                (CANONICAL_ALBEDO_RES * CANONICAL_ALBEDO_RES * 4) as usize
            );
        }
    }
}
