//! Pure-CPU builder that rasterises per-vertex material weights into the two
//! RGBA8 splat maps consumed by `TerrainMaterialManager` (Phase 2.2 / T6).
//!
//! Given a rectangular grid of [`TerrainVertex`] samples (`width` columns ×
//! `height` rows, row-major), produce:
//!
//! * `splat_0`: RGBA8 of dims `width × height` where each channel encodes the
//!   total weight of layers 0..3 mapped from the vertex's
//!   `material_ids/material_weights` (clamped to `[0, 1]`).
//! * `splat_1`: RGBA8 of dims `width × height` encoding layers 4..7.
//!
//! Real-Fix.C 2026-05-08: switched read source from per-vertex
//! `biome_weights_0/1` to `material_ids/material_weights` (Option C per
//! Andrew-gate decision). Resolves §7.7 sibling-attribute drift trap at
//! texture-data layer (Round 7 evidence): paint mutates `material_*`; splat
//! builder now reads what paint writes; the two attribute sets unified at
//! the boundary. The 8-channel splat output is reconstructed by mapping
//! `material_ids[i]` (0..7) to the corresponding channel and accumulating
//! `material_weights[i]`.
//!
//! The function is intentionally free of GPU or allocator coupling so it can
//! be covered by fast CPU unit tests and reused by the headless scatter/LOD
//! bake pipeline.

use anyhow::{Context, Result};

use super::types::TerrainVertex;

/// The two RGBA8 splat maps for a single chunk.
#[derive(Debug, Clone)]
pub struct ChunkSplatMaps {
    pub splat_0: Vec<u8>,
    pub splat_1: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl ChunkSplatMaps {
    /// Total bytes in each splat (both maps are the same size).
    pub fn bytes_per_map(&self) -> usize {
        (self.width as usize) * (self.height as usize) * 4
    }
}

/// Convert a grid of [`TerrainVertex`] samples into RGBA8 splat maps.
///
/// `vertices.len()` must equal `width * height`. Layout is row-major: the
/// vertex at column `x`, row `y` lives at `vertices[y * width + x]`.
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
    let mut splat_0 = Vec::with_capacity(texel_count * 4);
    let mut splat_1 = Vec::with_capacity(texel_count * 4);

    // Real-Fix.C 2026-05-08: reconstruct the 8-channel splat from the
    // sparse 4-pair (material_id, weight) representation. Each splat
    // channel index corresponds to a GPU layer ID (0..7); accumulate the
    // weight contribution of any vertex slot whose material_id matches.
    // material_ids out of range (>= 8) are dropped per pbr_terrain.wgsl
    // MAX_TERRAIN_LAYERS=8.
    //
    // [INSTRUMENTATION Round 8 T11.C — Mediator-Brush-Diagnostic-Round-8-Instrumentation.A 2026-05-08]
    // Counts vertices with kept vs dropped material_ids per chunk. Distinguishes
    // H8.1 (capacity mismatch — dropped_count > 0 with non-working IDs) from
    // H8.3 (mapping bug producing out-of-range IDs from in-range UI selections).
    // Throttled to once-per-N invocations to avoid log spam during scrubbing.
    let mut t11c_kept_slots: u32 = 0;
    let mut t11c_dropped_slots: u32 = 0;
    let mut t11c_dropped_ids: [i32; 8] = [-1; 8];
    let mut t11c_dropped_id_count: usize = 0;

    for v in vertices {
        let mut channels = [0.0f32; 8];
        for i in 0..4 {
            let layer = v.material_ids[i] as i32;
            if (0..8).contains(&layer) {
                channels[layer as usize] += v.material_weights[i];
                if v.material_weights[i] > 0.0 {
                    t11c_kept_slots += 1;
                }
            } else if v.material_weights[i] > 0.0 {
                t11c_dropped_slots += 1;
                if t11c_dropped_id_count < t11c_dropped_ids.len() {
                    let already_seen = t11c_dropped_ids[..t11c_dropped_id_count].contains(&layer);
                    if !already_seen {
                        t11c_dropped_ids[t11c_dropped_id_count] = layer;
                        t11c_dropped_id_count += 1;
                    }
                }
            }
        }
        for c in 0..4 {
            splat_0.push(encode_weight(channels[c]));
        }
        for c in 4..8 {
            splat_1.push(encode_weight(channels[c]));
        }
    }

    {
        static R8_C_FRAME: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(0);
        let _r8c_n = R8_C_FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if _r8c_n % 12 == 0 {
            eprintln!(
                "[BRUSH-DBG] splat-build-cap: max_layers=8, vert_count={}, kept_slots={}, dropped_slots={}, dropped_ids_sample={:?}",
                vertices.len(),
                t11c_kept_slots,
                t11c_dropped_slots,
                &t11c_dropped_ids[..t11c_dropped_id_count],
            );
        }
    }

    Ok(ChunkSplatMaps {
        splat_0,
        splat_1,
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

    /// Real-Fix.C 2026-05-08: helper builds a vertex from an 8-slot dense
    /// weight array using the same top-4 sparse encoding the biome generator
    /// produces. Mirrors `terrain_integration::TerrainState::biome_weights_8_to_material_slots`
    /// so splat builder tests retain the prior 8-channel testing semantics.
    fn make_vertex(w0: [f32; 4], w1: [f32; 4]) -> TerrainVertex {
        let weights_8 = [w0[0], w0[1], w0[2], w0[3], w1[0], w1[1], w1[2], w1[3]];
        let (ids, ws) = top4_from_8(weights_8);
        TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: ids,
            material_weights: ws,
        }
    }

    /// Standalone replica of biome_weights_8_to_material_slots without
    /// total-zero fallback (so test inputs of all-zero produce all-zero
    /// splat instead of the safe-default sand layer).
    fn top4_from_8(weights_8: [f32; 8]) -> ([f32; 4], [f32; 4]) {
        let mut entries: [(f32, f32); 8] = [
            (0.0, weights_8[0]),
            (1.0, weights_8[1]),
            (2.0, weights_8[2]),
            (3.0, weights_8[3]),
            (4.0, weights_8[4]),
            (5.0, weights_8[5]),
            (6.0, weights_8[6]),
            (7.0, weights_8[7]),
        ];
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut top4: [(f32, f32); 4] = [entries[0], entries[1], entries[2], entries[3]];
        top4.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut ids = [0.0f32; 4];
        let mut ws = [0.0f32; 4];
        for i in 0..4 {
            ids[i] = top4[i].0;
            ws[i] = top4[i].1;
        }
        (ids, ws)
    }

    #[test]
    fn rejects_zero_dims() {
        assert!(build_chunk_splat_maps(&[], 0, 1).is_err());
        assert!(build_chunk_splat_maps(&[], 1, 0).is_err());
    }

    #[test]
    fn rejects_vertex_count_mismatch() {
        let v = make_vertex([1.0, 0.0, 0.0, 0.0], [0.0; 4]);
        let err = build_chunk_splat_maps(&[v], 2, 2).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("vertex count mismatch"), "got: {msg}");
    }

    #[test]
    fn encodes_single_vertex_grid() {
        // Test direct material_* construction since top-4-from-8 sparsifies
        // and may not preserve all 4 channel weights when there are >4
        // non-zero slots. Build the vertex directly: layers 0..3 with
        // weights 1.0, 0.5, 0.25, 0.0 in splat_0; layers 4 and 7 with
        // weights 0.75 and 1.0 in splat_1.
        let v = TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: [0.0, 1.0, 4.0, 7.0],
            material_weights: [1.0, 0.5, 0.75, 1.0],
        };
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        assert_eq!(maps.width, 1);
        assert_eq!(maps.height, 1);
        assert_eq!(maps.bytes_per_map(), 4);
        // splat_0: layer 0 = 1.0 (255), layer 1 = 0.5 (128), layer 2 = 0 (0), layer 3 = 0 (0)
        assert_eq!(maps.splat_0, vec![255, 128, 0, 0]);
        // splat_1: layer 4 = 0.75 (191), layer 5 = 0, layer 6 = 0, layer 7 = 1.0 (255)
        assert_eq!(maps.splat_1, vec![191, 0, 0, 255]);
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
        assert_eq!(maps.splat_0, vec![0, 255, 0, 0]);
    }

    #[test]
    fn row_major_layout_matches_input_order() {
        // Build a 2×1 grid with distinguishable layers per column.
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
        // Texel (0,0) = left: layer 0 = 1.0 → [255,0,0,0]
        // Texel (1,0) = right: layer 1 = 1.0 → [0,255,0,0]
        assert_eq!(&maps.splat_0[0..4], &[255, 0, 0, 0]);
        assert_eq!(&maps.splat_0[4..8], &[0, 255, 0, 0]);
    }

    #[test]
    fn dominant_weight_preserved_through_encoding() {
        // Ensure dominant material survives the round-trip to RGBA8.
        let v = TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: [0.0, 1.0, 2.0, 3.0],
            material_weights: [0.1, 0.2, 0.6, 0.1],
        };
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        let max_byte = *maps.splat_0.iter().max().unwrap();
        assert_eq!(max_byte, maps.splat_0[2], "layer 2 should be dominant");
    }

    #[test]
    fn out_of_range_material_ids_dropped() {
        // Real-Fix.C: material_ids outside 0..8 are dropped per
        // pbr_terrain.wgsl MAX_TERRAIN_LAYERS=8.
        let v = TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            material_ids: [0.0, 9.0, 21.0, -1.0],
            material_weights: [1.0, 1.0, 1.0, 1.0],
        };
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        // Only layer 0 survives (1.0 → 255); slots with material_id 9, 21, -1 dropped.
        assert_eq!(maps.splat_0, vec![255, 0, 0, 0]);
        assert_eq!(maps.splat_1, vec![0, 0, 0, 0]);
    }

    #[test]
    fn large_grid_exact_size() {
        // 8-channel uniform 0.125 weights → top-4 sparsifies → channels 0-3 get 0.25 each.
        // After biome_weights_8_to_material_slots normalization, top4 weights sum to 1.0,
        // so each of 4 selected layers gets 0.25 weight.
        let verts = vec![make_vertex([0.125; 4], [0.125; 4]); 64 * 64];
        let maps = build_chunk_splat_maps(&verts, 64, 64).unwrap();
        assert_eq!(maps.bytes_per_map(), 64 * 64 * 4);
        assert_eq!(maps.splat_0.len(), maps.bytes_per_map());
        assert_eq!(maps.splat_1.len(), maps.bytes_per_map());
    }
}
