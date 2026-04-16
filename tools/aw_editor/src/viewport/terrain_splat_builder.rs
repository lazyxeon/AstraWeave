//! Pure-CPU builder that rasterises per-vertex biome weights into the two
//! RGBA8 splat maps consumed by `TerrainMaterialManager` (Phase 2.2 / T6).
//!
//! Given a rectangular grid of [`TerrainVertex`] samples (`width` columns ×
//! `height` rows, row-major), produce:
//!
//! * `splat_0`: RGBA8 of dims `width × height` where each channel encodes
//!   `biome_weights_0[c] * 255` (clamped to `[0, 1]`).
//! * `splat_1`: RGBA8 of dims `width × height` encoding `biome_weights_1`.
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

    for v in vertices {
        for &w in &v.biome_weights_0 {
            splat_0.push(encode_weight(w));
        }
        for &w in &v.biome_weights_1 {
            splat_1.push(encode_weight(w));
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

    fn make_vertex(w0: [f32; 4], w1: [f32; 4]) -> TerrainVertex {
        TerrainVertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            biome_weights_0: w0,
            biome_weights_1: w1,
            material_ids: [0.0; 4],
            material_weights: [1.0, 0.0, 0.0, 0.0],
        }
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
        let v = make_vertex([1.0, 0.5, 0.25, 0.0], [0.75, 0.0, 0.0, 1.0]);
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        assert_eq!(maps.width, 1);
        assert_eq!(maps.height, 1);
        assert_eq!(maps.bytes_per_map(), 4);
        // splat_0: 1.0 → 255, 0.5 → 128, 0.25 → 64, 0.0 → 0
        assert_eq!(maps.splat_0, vec![255, 128, 64, 0]);
        // splat_1: 0.75 → 191, 0.0 → 0, 0.0 → 0, 1.0 → 255
        assert_eq!(maps.splat_1, vec![191, 0, 0, 255]);
    }

    #[test]
    fn clamps_out_of_range_weights() {
        let v = make_vertex([-0.5, 1.5, f32::NAN, f32::INFINITY], [0.0; 4]);
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        // -0.5 clamps to 0; 1.5 clamps to 1.0 (255); NaN → 0; +inf → 0 (not finite).
        assert_eq!(maps.splat_0, vec![0, 255, 0, 0]);
    }

    #[test]
    fn row_major_layout_matches_input_order() {
        // Build a 2×1 grid with distinguishable weights per column.
        let left = make_vertex([1.0, 0.0, 0.0, 0.0], [0.0; 4]);
        let right = make_vertex([0.0, 1.0, 0.0, 0.0], [0.0; 4]);
        let maps = build_chunk_splat_maps(&[left, right], 2, 1).unwrap();
        // Texel (0,0) = left: [255,0,0,0]; texel (1,0) = right: [0,255,0,0]
        assert_eq!(&maps.splat_0[0..4], &[255, 0, 0, 0]);
        assert_eq!(&maps.splat_0[4..8], &[0, 255, 0, 0]);
    }

    #[test]
    fn dominant_weight_preserved_through_encoding() {
        // Ensure that the dominant biome (matching to_engine_vertex) survives
        // the round-trip to RGBA8 — i.e. the channel with the max weight ends
        // up with the max byte value.
        let v = make_vertex([0.1, 0.2, 0.6, 0.1], [0.0, 0.0, 0.0, 0.0]);
        let maps = build_chunk_splat_maps(&[v], 1, 1).unwrap();
        let max_byte = *maps.splat_0.iter().max().unwrap();
        assert_eq!(max_byte, maps.splat_0[2], "channel 2 should be dominant");
    }

    #[test]
    fn large_grid_exact_size() {
        let verts = vec![make_vertex([0.25; 4], [0.25; 4]); 64 * 64];
        let maps = build_chunk_splat_maps(&verts, 64, 64).unwrap();
        assert_eq!(maps.bytes_per_map(), 64 * 64 * 4);
        assert_eq!(maps.splat_0.len(), maps.bytes_per_map());
        assert_eq!(maps.splat_1.len(), maps.bytes_per_map());
        // 0.25 → 64 (rounding: 0.25*255 + 0.5 = 64.25 → 64)
        assert!(maps.splat_0.iter().all(|&b| b == 64));
    }
}
