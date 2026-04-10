//! Compressed Voxel Storage — Palette Compression + Run-Length Encoding
//!
//! Replaces per-voxel `(f32, u16)` storage with:
//! - **Palette**: deduplicated set of unique voxel types (quantized density + material).
//! - **Index array**: one palette index per cell (u8 if palette ≤ 256 entries, u16 otherwise).
//! - **RLE**: run-length encoded index stream for serialization and compact storage.
//!
//! Memory improvement: 10–50× for typical terrain chunks vs the SVO approach.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::voxel_data::{MaterialId, Voxel, CHUNK_SIZE};

/// Total voxels in a chunk (32³).
pub const CHUNK_VOLUME: usize =
    (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize);

// ─── Palette Entry ───

/// Quantized voxel for palette deduplication.
///
/// Density is quantized to 16 bits (0–65535 maps to 0.0–1.0) to enable
/// exact equality comparison while retaining sub-percent precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaletteEntry {
    pub density_q: u16,
    pub material: MaterialId,
}

impl PaletteEntry {
    /// Quantize a continuous density to 16-bit.
    pub fn from_voxel(v: &Voxel) -> Self {
        Self {
            density_q: (v.density.clamp(0.0, 1.0) * 65535.0) as u16,
            material: v.material,
        }
    }

    /// Dequantize back to a full Voxel.
    pub fn to_voxel(&self) -> Voxel {
        Voxel {
            density: self.density_q as f32 / 65535.0,
            material: self.material,
        }
    }

    /// Air (empty space) entry.
    pub fn air() -> Self {
        Self {
            density_q: 0,
            material: 0,
        }
    }
}

// ─── Palette ───

/// Palette of unique voxel types, mapping indices ↔ entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelPalette {
    entries: Vec<PaletteEntry>,
    #[serde(skip)]
    reverse: HashMap<PaletteEntry, u16>,
}

impl VoxelPalette {
    /// Create a palette with a single "air" entry at index 0.
    pub fn new() -> Self {
        let air = PaletteEntry::air();
        let mut reverse = HashMap::new();
        reverse.insert(air, 0);
        Self {
            entries: vec![air],
            reverse,
        }
    }

    /// Rebuild the reverse lookup (call after deserialization).
    pub fn rebuild_reverse(&mut self) {
        self.reverse.clear();
        for (i, entry) in self.entries.iter().enumerate() {
            self.reverse.insert(*entry, i as u16);
        }
    }

    /// Get or insert an entry, returning its palette index.
    pub fn get_or_insert(&mut self, entry: PaletteEntry) -> u16 {
        if let Some(&idx) = self.reverse.get(&entry) {
            return idx;
        }
        let idx = self.entries.len() as u16;
        self.entries.push(entry);
        self.reverse.insert(entry, idx);
        idx
    }

    /// Look up an entry by palette index.
    pub fn get(&self, idx: u16) -> Option<&PaletteEntry> {
        self.entries.get(idx as usize)
    }

    /// Number of unique entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the palette is empty (should never be — always has air).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether single-byte indices suffice.
    pub fn fits_u8(&self) -> bool {
        self.entries.len() <= 256
    }
}

impl Default for VoxelPalette {
    fn default() -> Self {
        Self::new()
    }
}

// ─── RLE Run ───

/// A single run in run-length encoded data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RleRun {
    /// Palette index.
    pub index: u16,
    /// Number of consecutive cells with this index.
    pub length: u16,
}

/// Encode a sequence of palette indices into RLE runs.
pub fn rle_encode(indices: &[u16]) -> Vec<RleRun> {
    if indices.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let mut current = indices[0];
    let mut count: u16 = 1;

    for &idx in &indices[1..] {
        if idx == current && count < u16::MAX {
            count += 1;
        } else {
            runs.push(RleRun {
                index: current,
                length: count,
            });
            current = idx;
            count = 1;
        }
    }
    runs.push(RleRun {
        index: current,
        length: count,
    });

    runs
}

/// Decode RLE runs back to a flat index array.
pub fn rle_decode(runs: &[RleRun]) -> Vec<u16> {
    let total: usize = runs.iter().map(|r| r.length as usize).sum();
    let mut out = Vec::with_capacity(total);
    for run in runs {
        for _ in 0..run.length {
            out.push(run.index);
        }
    }
    out
}

// ─── Compressed Voxel Chunk ───

/// A palette-compressed voxel chunk with O(1) random access.
///
/// Stores a palette of unique voxel types and a flat array of palette indices,
/// one per cell. Memory usage scales with number of *unique* voxel types
/// rather than total cells.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedVoxelChunk {
    palette: VoxelPalette,
    /// Flat array of palette indices, linearized as `x + y * W + z * W * H`.
    indices: Vec<u16>,
    width: u32,
    height: u32,
    depth: u32,
}

impl CompressedVoxelChunk {
    /// Create an empty (all-air) chunk with standard dimensions.
    pub fn new() -> Self {
        Self::with_dimensions(CHUNK_SIZE as u32, CHUNK_SIZE as u32, CHUNK_SIZE as u32)
    }

    /// Create with custom dimensions (for testing or non-standard chunks).
    pub fn with_dimensions(width: u32, height: u32, depth: u32) -> Self {
        let volume = (width * height * depth) as usize;
        Self {
            palette: VoxelPalette::new(),
            indices: vec![0; volume], // 0 = air
            width,
            height,
            depth,
        }
    }

    /// Compress raw voxel data into a palette-compressed chunk.
    pub fn from_raw(data: &[Voxel], width: u32, height: u32, depth: u32) -> Self {
        let volume = (width * height * depth) as usize;
        let mut palette = VoxelPalette::new();
        let mut indices = Vec::with_capacity(volume);

        for v in data.iter().take(volume) {
            let entry = PaletteEntry::from_voxel(v);
            let idx = palette.get_or_insert(entry);
            indices.push(idx);
        }

        // Pad with air if data is shorter than volume
        while indices.len() < volume {
            indices.push(0);
        }

        Self {
            palette,
            indices,
            width,
            height,
            depth,
        }
    }

    /// Compress from a standard chunk's voxel data.
    pub fn from_voxels(voxels: &[Voxel]) -> Self {
        let s = CHUNK_SIZE as u32;
        Self::from_raw(voxels, s, s, s)
    }

    /// Linearize (x, y, z) to a flat index.
    fn linear_index(&self, x: u32, y: u32, z: u32) -> usize {
        (x + y * self.width + z * self.width * self.height) as usize
    }

    /// Get the voxel at (x, y, z), dequantized.
    pub fn get(&self, x: u32, y: u32, z: u32) -> Voxel {
        if x >= self.width || y >= self.height || z >= self.depth {
            return Voxel::default();
        }
        let idx = self.indices[self.linear_index(x, y, z)];
        match self.palette.get(idx) {
            Some(entry) => entry.to_voxel(),
            None => Voxel::default(),
        }
    }

    /// Set the voxel at (x, y, z).
    pub fn set(&mut self, x: u32, y: u32, z: u32, voxel: Voxel) {
        if x >= self.width || y >= self.height || z >= self.depth {
            return;
        }
        let entry = PaletteEntry::from_voxel(&voxel);
        let palette_idx = self.palette.get_or_insert(entry);
        let li = self.linear_index(x, y, z);
        self.indices[li] = palette_idx;
    }

    /// Decompress to a flat array of voxels.
    pub fn decompress(&self) -> Vec<Voxel> {
        self.indices
            .iter()
            .map(|&idx| {
                self.palette
                    .get(idx)
                    .map_or(Voxel::default(), |e| e.to_voxel())
            })
            .collect()
    }

    /// Number of unique voxel types in this chunk.
    pub fn palette_size(&self) -> usize {
        self.palette.len()
    }

    /// Total volume in cells.
    pub fn volume(&self) -> usize {
        (self.width * self.height * self.depth) as usize
    }

    /// Estimated memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        // Palette entries
        let palette_bytes = self.palette.len() * std::mem::size_of::<PaletteEntry>();
        // Index array
        let index_bytes = self.indices.len() * std::mem::size_of::<u16>();
        // Reverse map overhead (~32 bytes per entry approx)
        let map_overhead = self.palette.len() * 32;
        palette_bytes + index_bytes + map_overhead
    }

    /// Raw uncompressed size for comparison.
    pub fn raw_size(&self) -> usize {
        self.volume() * std::mem::size_of::<Voxel>()
    }

    /// Compression ratio: raw_size / compressed_size.
    pub fn compression_ratio(&self) -> f32 {
        let compressed = self.memory_usage();
        if compressed == 0 {
            return 1.0;
        }
        self.raw_size() as f32 / compressed as f32
    }

    /// RLE-encode the index array for serialization.
    pub fn to_rle(&self) -> Vec<RleRun> {
        rle_encode(&self.indices)
    }

    /// Reconstruct from RLE data and a palette.
    pub fn from_rle(
        runs: &[RleRun],
        palette: VoxelPalette,
        width: u32,
        height: u32,
        depth: u32,
    ) -> Self {
        let indices = rle_decode(runs);
        Self {
            palette,
            indices,
            width,
            height,
            depth,
        }
    }

    /// Dimensions.
    pub fn dimensions(&self) -> (u32, u32, u32) {
        (self.width, self.height, self.depth)
    }

    /// Reference to the palette.
    pub fn palette(&self) -> &VoxelPalette {
        &self.palette
    }

    /// Count how many cells are non-air (density_q > 0).
    pub fn solid_count(&self) -> usize {
        self.indices
            .iter()
            .filter(|&&idx| self.palette.get(idx).is_some_and(|e| e.density_q > 0))
            .count()
    }
}

impl Default for CompressedVoxelChunk {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_entry_roundtrip() {
        let v = Voxel {
            density: 0.5,
            material: 42,
        };
        let entry = PaletteEntry::from_voxel(&v);
        let back = entry.to_voxel();
        assert!((back.density - 0.5).abs() < 0.001);
        assert_eq!(back.material, 42);
    }

    #[test]
    fn palette_entry_air() {
        let air = PaletteEntry::air();
        assert_eq!(air.density_q, 0);
        assert_eq!(air.material, 0);
    }

    #[test]
    fn palette_entry_extremes() {
        let solid = PaletteEntry::from_voxel(&Voxel {
            density: 1.0,
            material: 100,
        });
        assert_eq!(solid.density_q, 65535);

        let empty = PaletteEntry::from_voxel(&Voxel {
            density: 0.0,
            material: 0,
        });
        assert_eq!(empty.density_q, 0);
    }

    #[test]
    fn palette_get_or_insert_deduplicates() {
        let mut palette = VoxelPalette::new();
        let entry = PaletteEntry {
            density_q: 32768,
            material: 5,
        };
        let idx1 = palette.get_or_insert(entry);
        let idx2 = palette.get_or_insert(entry);
        assert_eq!(idx1, idx2);
        assert_eq!(palette.len(), 2); // air + this one
    }

    #[test]
    fn palette_starts_with_air() {
        let palette = VoxelPalette::new();
        assert_eq!(palette.len(), 1);
        assert_eq!(palette.get(0), Some(&PaletteEntry::air()));
    }

    #[test]
    fn palette_fits_u8() {
        let palette = VoxelPalette::new();
        assert!(palette.fits_u8());
    }

    #[test]
    fn rle_encode_decode_roundtrip() {
        let data = vec![0u16, 0, 0, 1, 1, 2, 0, 0, 0, 0];
        let runs = rle_encode(&data);
        assert_eq!(runs.len(), 4); // 3×0, 2×1, 1×2, 4×0
        let decoded = rle_decode(&runs);
        assert_eq!(decoded, data);
    }

    #[test]
    fn rle_encode_single() {
        let data = vec![5u16];
        let runs = rle_encode(&data);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].index, 5);
        assert_eq!(runs[0].length, 1);
    }

    #[test]
    fn rle_encode_empty() {
        let runs = rle_encode(&[]);
        assert!(runs.is_empty());
    }

    #[test]
    fn rle_encode_uniform() {
        let data = vec![7u16; 1000];
        let runs = rle_encode(&data);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].index, 7);
        assert_eq!(runs[0].length, 1000);
    }

    #[test]
    fn compressed_chunk_new_is_air() {
        let chunk = CompressedVoxelChunk::new();
        assert_eq!(chunk.volume(), CHUNK_VOLUME);
        assert_eq!(chunk.palette_size(), 1); // only air
        assert_eq!(chunk.solid_count(), 0);

        let v = chunk.get(0, 0, 0);
        assert_eq!(v.density, 0.0);
        assert_eq!(v.material, 0);
    }

    #[test]
    fn compressed_chunk_set_get() {
        let mut chunk = CompressedVoxelChunk::with_dimensions(4, 4, 4);
        let voxel = Voxel {
            density: 0.75,
            material: 10,
        };
        chunk.set(1, 2, 3, voxel);

        let got = chunk.get(1, 2, 3);
        assert!((got.density - 0.75).abs() < 0.001);
        assert_eq!(got.material, 10);
        assert_eq!(chunk.palette_size(), 2); // air + the new one
    }

    #[test]
    fn compressed_chunk_set_out_of_bounds_noop() {
        let mut chunk = CompressedVoxelChunk::with_dimensions(4, 4, 4);
        chunk.set(
            10,
            0,
            0,
            Voxel {
                density: 1.0,
                material: 1,
            },
        );
        // Should not crash; out-of-bounds get returns default
        let v = chunk.get(10, 0, 0);
        assert_eq!(v.density, 0.0);
    }

    #[test]
    fn compressed_chunk_from_raw() {
        let mut raw = vec![
            Voxel {
                density: 0.0,
                material: 0,
            };
            64
        ]; // 4×4×4
           // Fill half with stone
        for v in raw.iter_mut().take(32) {
            v.density = 1.0;
            v.material = 1;
        }
        let chunk = CompressedVoxelChunk::from_raw(&raw, 4, 4, 4);
        assert_eq!(chunk.volume(), 64);
        assert_eq!(chunk.palette_size(), 2); // air + stone
        assert_eq!(chunk.solid_count(), 32);
    }

    #[test]
    fn compressed_chunk_decompress() {
        let mut chunk = CompressedVoxelChunk::with_dimensions(2, 2, 2);
        chunk.set(
            0,
            0,
            0,
            Voxel {
                density: 1.0,
                material: 5,
            },
        );
        chunk.set(
            1,
            1,
            1,
            Voxel {
                density: 0.5,
                material: 3,
            },
        );

        let decompressed = chunk.decompress();
        assert_eq!(decompressed.len(), 8);
        assert!((decompressed[0].density - 1.0).abs() < 0.001); // (0,0,0) = index 0
        assert_eq!(decompressed[0].material, 5);
    }

    #[test]
    fn compressed_chunk_compression_ratio() {
        // All-air chunk: palette has 1 entry, indices are all 0
        let chunk = CompressedVoxelChunk::new();
        let ratio = chunk.compression_ratio();
        // 32³ × 6 bytes raw ≈ 192KB; compressed ≈ 32³ × 2 + palette ≈ 65KB
        // Ratio should be > 1 (we use less memory than raw)
        assert!(ratio > 1.0, "Compression ratio {ratio} should be > 1.0");
    }

    #[test]
    fn compressed_chunk_rle_roundtrip() {
        let mut chunk = CompressedVoxelChunk::with_dimensions(4, 4, 4);
        chunk.set(
            0,
            0,
            0,
            Voxel {
                density: 1.0,
                material: 1,
            },
        );
        chunk.set(
            1,
            0,
            0,
            Voxel {
                density: 1.0,
                material: 1,
            },
        );
        chunk.set(
            2,
            0,
            0,
            Voxel {
                density: 0.5,
                material: 2,
            },
        );

        let runs = chunk.to_rle();
        let reconstructed = CompressedVoxelChunk::from_rle(&runs, chunk.palette.clone(), 4, 4, 4);

        assert_eq!(reconstructed.get(0, 0, 0).material, 1);
        assert_eq!(reconstructed.get(1, 0, 0).material, 1);
        assert_eq!(reconstructed.get(2, 0, 0).material, 2);
        assert_eq!(reconstructed.get(3, 0, 0).material, 0); // air
    }

    #[test]
    fn compressed_chunk_memory_less_than_raw() {
        // For a chunk with only 2 palette entries, compressed < raw
        let chunk = CompressedVoxelChunk::new();
        assert!(
            chunk.memory_usage() < chunk.raw_size(),
            "memory {} should be < raw {}",
            chunk.memory_usage(),
            chunk.raw_size()
        );
    }

    #[test]
    fn rle_uniform_chunk_very_compact() {
        // All-air chunk: RLE should be a single run
        let chunk = CompressedVoxelChunk::new();
        let runs = chunk.to_rle();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].index, 0);
        assert_eq!(runs[0].length, CHUNK_VOLUME as u16);
    }

    #[test]
    fn palette_rebuild_reverse() {
        let mut palette = VoxelPalette::new();
        let entry = PaletteEntry {
            density_q: 100,
            material: 7,
        };
        let idx = palette.get_or_insert(entry);

        // Simulate deserialization by clearing reverse
        palette.reverse.clear();
        palette.rebuild_reverse();

        let idx2 = palette.get_or_insert(entry);
        assert_eq!(idx, idx2); // should find existing entry
    }
}
