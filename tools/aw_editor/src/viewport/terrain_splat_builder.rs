//! Pure-CPU builder that rasterises per-vertex material weights into the
//! eight RGBA8 splat maps consumed by `TerrainMaterialManager` (Phase 2.2 / T6).
//!
//! Given a rectangular grid of [`TerrainVertex`] samples (`width` columns ×
//! `height` rows, row-major), produce
//! [`astraweave_render::NUM_SPLAT_MAPS`] = 8 RGBA8 textures of dims
//! `width × height`. Channel mapping for splat `i`:
//!
//! * `splats[i]` channel R..A → layer weights `(i*4 + 0)..(i*4 + 3)`.
//!
//! For example, `splats[0]` carries layers 0..3 and `splats[7]` carries
//! layers 28..31. This matches `pbr_terrain.wgsl` and `pbr_terrain_forward.wgsl`
//! after Real-Fix.D 2026-05-08, which bumped from 2 splat textures (8 channels)
//! to 8 splat textures (32 channels) per Andrew-gate decision (h) Option D-2
//! (canonical material library).
//!
//! Real-Fix.C 2026-05-08: switched read source from per-vertex
//! `biome_weights_0/1` to `material_ids/material_weights` (resolved §7.7
//! sibling-attribute drift trap at texture-data layer).
//!
//! Real-Fix.D 2026-05-08: bumped from 8-layer splat (2 RGBA8 maps) to 32-layer
//! splat (8 RGBA8 maps); identity now derives from `MaterialLibrary::len()`
//! (= 32). Resolves §7.7 wrapped-component capacity-boundary trap at
//! UI/renderer cross-component capacity coordination layer.
//!
//! The function is intentionally free of GPU or allocator coupling so it can
//! be covered by fast CPU unit tests and reused by the headless scatter/LOD
//! bake pipeline.

use anyhow::{Context, Result};

use astraweave_render::{MAX_TERRAIN_LAYERS, NUM_SPLAT_MAPS};

use super::types::TerrainVertex;

/// The eight RGBA8 splat maps for a single chunk (Real-Fix.D 2026-05-08).
///
/// `splats.len() == NUM_SPLAT_MAPS == 8`. Each `splats[i]` is the same
/// `width × height × 4` byte buffer encoding 4 layer weights (channels
/// R..A → layers `i*4+0..i*4+3`).
#[derive(Debug, Clone)]
pub struct ChunkSplatMaps {
    pub splats: Vec<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

impl ChunkSplatMaps {
    /// Total bytes in each splat texture (all maps are the same size).
    pub fn bytes_per_map(&self) -> usize {
        (self.width as usize) * (self.height as usize) * 4
    }
}

/// Convert a grid of [`TerrainVertex`] samples into RGBA8 splat maps.
///
/// `vertices.len()` must equal `width * height`. Layout is row-major: the
/// vertex at column `x`, row `y` lives at `vertices[y * width + x]`.
///
/// Real-Fix.D 2026-05-08: produces [`NUM_SPLAT_MAPS`] = 8 splat textures
/// covering 32 canonical material layers (was 2 textures × 8 layers).
pub fn build_chunk_splat_maps(
    vertices: &[TerrainVertex],
    width: u32,
    height: u32,
) -> Result<ChunkSplatMaps> {
    if width == 0 || height == 0 {
        anyhow::bail!("splat map dims must be non-zero, got {width}x{height}");
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .context("width × height overflowed usize")?;
    if vertices.len() != expected {
        anyhow::bail!(
            "vertex count mismatch: got {}, expected {} for {}x{} grid",
            vertices.len(),
            expected,
            width,
            height
        );
    }

    let texel_count = expected;
    let mut splats: Vec<Vec<u8>> = (0..NUM_SPLAT_MAPS)
        .map(|_| Vec::with_capacity(texel_count * 4))
        .collect();

    // Real-Fix.D 2026-05-08: reconstruct the 32-channel splat from the
    // sparse 4-pair (material_id, weight) representation. Each splat-channel
    // index corresponds to a GPU layer ID (0..MAX_TERRAIN_LAYERS=32);
    // accumulate the weight contribution of any vertex slot whose material_id
    // matches. material_ids out of range (>= MAX_TERRAIN_LAYERS) are dropped.
    let max_layers = MAX_TERRAIN_LAYERS as usize;
    for v in vertices {
        let mut channels = vec![0.0f32; max_layers];
        for i in 0..4 {
            let layer = v.material_ids[i] as i32;
            if layer >= 0 && (layer as usize) < max_layers {
                channels[layer as usize] += v.material_weights[i];
            }
        }
        for splat_idx in 0..NUM_SPLAT_MAPS {
            let base = splat_idx * 4;
            for ch in 0..4 {
                splats[splat_idx].push(encode_weight(channels[base + ch]));
            }
        }
    }

    Ok(ChunkSplatMaps {
        splats,
        width,
        height,
    })
}

#[inline]
fn encode_weight(w: f32) -> u8 {
    // NaN → 0 (matches "no contribution" semantics from the GOAP/biome mixer).
    if !w.is_finite() {
        return 0;
    }
    let clamped = w.clamp(0.0, 1.0);
    (clamped * 255.0 + 0.5) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: construct a vertex with the given material slots.
    fn vertex_with(ids: [f32; 4], weights: [f32; 4]) -> TerrainVertex {
        TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: ids,
            material_weights: weights,
        }
    }

    #[test]
    fn rejects_zero_dims() {
        assert!(build_chunk_splat_maps(&[], 0, 1).is_err());
        assert!(build_chunk_splat_maps(&[], 1, 0).is_err());
    }

    #[test]
    fn rejects_vertex_count_mismatch() {
        let v = vertex_with([0.0; 4], [1.0, 0.0, 0.0, 0.0]);
        let err = build_chunk_splat_maps(&[v], 2, 2).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("vertex count mismatch"), "got: {msg}");
    }

    #[test]
    fn produces_eight_splat_maps() {
        let v = vertex_with([0.0, 1.0, 4.0, 7.0], [1.0, 0.5, 0.75, 1.0]);
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        assert_eq!(maps.splats.len(), NUM_SPLAT_MAPS);
        assert_eq!(maps.splats.len(), 8);
        for splat in &maps.splats {
            assert_eq!(splat.len(), maps.bytes_per_map());
        }
    }

    #[test]
    fn encodes_low_layer_weights_in_splat0() {
        // Layers 0..3 with weights 1.0, 0.5, 0.75, 1.0 → splats[0] = [255,128,191,255].
        let v = TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: [0.0, 1.0, 2.0, 3.0],
            material_weights: [1.0, 0.5, 0.75, 1.0],
        };
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        assert_eq!(maps.splats[0], vec![255, 128, 191, 255]);
        // splats[1..7] all zeros for this vertex.
        for splat in &maps.splats[1..] {
            assert_eq!(splat, &vec![0, 0, 0, 0]);
        }
    }

    #[test]
    fn encodes_high_layer_weights_in_higher_splats() {
        // Real-Fix.D primary criterion: layer 21 (tree_leaves) lands in
        // splats[5] channel B (21 = 5*4 + 1; B is index 2; let me recompute:
        // 21 / 4 = 5, 21 % 4 = 1 → splats[5] channel G).
        let v = vertex_with([21.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]);
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        // splats[5] channel G = layer 21.
        assert_eq!(maps.splats[5], vec![0, 255, 0, 0]);
        // All other splats should be zero.
        for (i, splat) in maps.splats.iter().enumerate() {
            if i == 5 {
                continue;
            }
            assert_eq!(splat, &vec![0, 0, 0, 0], "splat[{i}] should be empty");
        }
    }

    #[test]
    fn clamps_out_of_range_weights() {
        let v = TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: [0.0, 1.0, 2.0, 3.0],
            material_weights: [-0.5, 1.5, f32::NAN, f32::INFINITY],
        };
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        // -0.5 clamps to 0; 1.5 clamps to 1.0 (255); NaN → 0; +inf → 0.
        assert_eq!(maps.splats[0], vec![0, 255, 0, 0]);
    }

    #[test]
    fn row_major_layout_matches_input_order() {
        let left = TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: [0.0, 0.0, 0.0, 0.0],
            material_weights: [1.0, 0.0, 0.0, 0.0],
        };
        let right = TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: [1.0, 0.0, 0.0, 0.0],
            material_weights: [1.0, 0.0, 0.0, 0.0],
        };
        let maps = build_chunk_splat_maps(&[left, right], 2, 1).unwrap();
        assert_eq!(&maps.splats[0][0..4], &[255, 0, 0, 0]);
        assert_eq!(&maps.splats[0][4..8], &[0, 255, 0, 0]);
    }

    #[test]
    fn dominant_weight_preserved_through_encoding() {
        let v = TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: [0.0, 1.0, 2.0, 3.0],
            material_weights: [0.1, 0.2, 0.6, 0.1],
        };
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        let max_byte = *maps.splats[0].iter().max().unwrap();
        assert_eq!(max_byte, maps.splats[0][2], "layer 2 should be dominant");
    }

    #[test]
    fn out_of_range_material_ids_dropped() {
        // Real-Fix.D 2026-05-08: material_ids outside 0..MAX_TERRAIN_LAYERS
        // (= 32) are dropped. Layer 32 and -1 fall outside; layer 21 survives
        // (now in-range, was previously dropped at the 8-layer cap).
        let v = TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: [0.0, 32.0, 21.0, -1.0],
            material_weights: [1.0, 1.0, 1.0, 1.0],
        };
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        // Layer 0 → splats[0] R = 255; layer 21 → splats[5] G = 255; rest 0.
        assert_eq!(maps.splats[0], vec![255, 0, 0, 0]);
        assert_eq!(maps.splats[5], vec![0, 255, 0, 0]);
        // Layer 32 and -1 both dropped → no contributions in any splat.
        for (i, splat) in maps.splats.iter().enumerate() {
            if i == 0 || i == 5 {
                continue;
            }
            assert_eq!(splat, &vec![0, 0, 0, 0], "splat[{i}] should be empty");
        }
    }

    #[test]
    fn large_grid_exact_size() {
        let v = vertex_with([0.0, 1.0, 2.0, 3.0], [0.25, 0.25, 0.25, 0.25]);
        let verts = vec![v; 64 * 64];
        let maps = build_chunk_splat_maps(&verts, 64, 64).unwrap();
        assert_eq!(maps.bytes_per_map(), 64 * 64 * 4);
        for splat in &maps.splats {
            assert_eq!(splat.len(), maps.bytes_per_map());
        }
    }

    #[test]
    fn all_named_layers_paint() {
        // Real-Fix.D primary criterion: every named material ID must produce
        // a non-zero contribution in the splat output.
        // 2026-05-15 Real-Fix.D follow-up: `default` removed; named count 22 -> 21.
        for layer_id in 0..21u32 {
            let v = vertex_with([layer_id as f32, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]);
            let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
            let splat_idx = (layer_id / 4) as usize;
            let channel = (layer_id % 4) as usize;
            assert_eq!(
                maps.splats[splat_idx][channel], 255,
                "layer {layer_id} should paint into splats[{splat_idx}][{channel}]"
            );
        }
    }
}
