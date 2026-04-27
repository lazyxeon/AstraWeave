//! Terrain chunk management and streaming

use crate::biome_lookup::BiomeId;
use crate::{BiomeType, Heightmap};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a terrain chunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId {
    pub x: i32,
    pub z: i32,
}

impl ChunkId {
    /// Create a new chunk ID
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Convert world position to chunk ID
    pub fn from_world_pos(world_pos: Vec3, chunk_size: f32) -> Self {
        let chunk_x = (world_pos.x / chunk_size).floor() as i32;
        let chunk_z = (world_pos.z / chunk_size).floor() as i32;
        Self::new(chunk_x, chunk_z)
    }

    /// Get the world position of the chunk's origin (bottom-left corner)
    pub fn to_world_pos(self, chunk_size: f32) -> Vec3 {
        Vec3::new(self.x as f32 * chunk_size, 0.0, self.z as f32 * chunk_size)
    }

    /// Get the center world position of the chunk
    pub fn to_center_pos(self, chunk_size: f32) -> Vec3 {
        let origin = self.to_world_pos(chunk_size);
        origin + Vec3::new(chunk_size * 0.5, 0.0, chunk_size * 0.5)
    }

    /// Get all chunk IDs within a given radius
    pub fn get_chunks_in_radius(center: Vec3, radius: u32, chunk_size: f32) -> Vec<ChunkId> {
        let center_chunk = ChunkId::from_world_pos(center, chunk_size);
        let mut chunks = Vec::new();

        let radius = radius as i32;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                chunks.push(ChunkId::new(center_chunk.x + dx, center_chunk.z + dz));
            }
        }

        chunks
    }

    /// Calculate distance to another chunk (in chunk units)
    pub fn distance_to(self, other: ChunkId) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dz = (self.z - other.z) as f32;
        (dx * dx + dz * dz).sqrt()
    }
}

/// A single terrain chunk containing heightmap and biome data
#[derive(Debug, Clone)]
pub struct TerrainChunk {
    id: ChunkId,
    heightmap: Heightmap,
    biome_map: Vec<BiomeType>,
    /// Phase 1.6-F.3-phase-1: per-vertex 8-slot biome weights populated from the
    /// pre-erosion heightmap. `None` for chunks constructed through the legacy
    /// `generate_chunk` path that doesn't know the ClimateBias — those callers
    /// continue to compute biome_weights on-the-fly from the post-erosion
    /// heightmap. `Some(Vec<[f32; 8]>)` for chunks from
    /// `generate_chunk_with_climate`, with entries stored in row-major
    /// (z, x) order matching heightmap indexing.
    ///
    /// The §2.5 biome-weight-stability-under-erosion invariant: weights are
    /// computed from pre-erosion Y values. Simple CA erosion (phase 1) barely
    /// moves heights so the distinction is imperceptible; phase 2's
    /// AdvancedErosionSimulator will make the invariant meaningful.
    biome_weights: Option<Vec<[f32; 8]>>,
    /// Phase 1.6-F.4.B.3.D.3b: per-vertex `BiomeId` from the climate-field
    /// architecture. `None` for chunks constructed through legacy paths
    /// (`new` or `new_with_biome_weights`); `Some(Vec<BiomeId>)` for chunks
    /// from the new `WorldGenerator::generate_chunk_with_climate` path that
    /// computes per-vertex biome IDs via `lookup_biome` from D.2.
    ///
    /// Stored in row-major (z, x) order matching heightmap indexing.
    /// Computed from PRE-erosion heights per the §2.5 invariant
    /// (biome assignment uses authorial intent, not post-erosion shape).
    biome_ids: Option<Vec<BiomeId>>,
    mesh_dirty: bool,
}

impl TerrainChunk {
    /// Create a new terrain chunk. Legacy constructor — leaves `biome_weights`
    /// at `None`. Phase 1.6-F.3-phase-1 callers that want pre-erosion
    /// biome_weights use `new_with_biome_weights`.
    pub fn new(id: ChunkId, heightmap: Heightmap, biome_map: Vec<BiomeType>) -> Self {
        Self {
            id,
            heightmap,
            biome_map,
            biome_weights: None,
            biome_ids: None,
            mesh_dirty: true,
        }
    }

    /// Phase 1.6-F.3-phase-1: construct a chunk with pre-erosion biome_weights
    /// already computed. Used by `WorldGenerator::generate_chunk_with_climate`.
    pub fn new_with_biome_weights(
        id: ChunkId,
        heightmap: Heightmap,
        biome_map: Vec<BiomeType>,
        biome_weights: Vec<[f32; 8]>,
    ) -> Self {
        Self {
            id,
            heightmap,
            biome_map,
            biome_weights: Some(biome_weights),
            biome_ids: None,
            mesh_dirty: true,
        }
    }

    /// Phase 1.6-F.4.B.3.D.3b: construct a chunk with both legacy 8-slot
    /// biome_weights (Phase 1.5 splat-rule path) and the new per-vertex
    /// `BiomeId` array (climate-field architecture). The two coexist
    /// during the D.3 transition; D.5+ may consolidate.
    ///
    /// Both arrays are computed from PRE-erosion heights per the §2.5
    /// authorial-intent invariant. `biome_ids` is row-major `(z, x)`
    /// ordering matching `biome_weights` and the heightmap.
    pub fn new_with_climate_field(
        id: ChunkId,
        heightmap: Heightmap,
        biome_map: Vec<BiomeType>,
        biome_weights: Vec<[f32; 8]>,
        biome_ids: Vec<BiomeId>,
    ) -> Self {
        Self {
            id,
            heightmap,
            biome_map,
            biome_weights: Some(biome_weights),
            biome_ids: Some(biome_ids),
            mesh_dirty: true,
        }
    }

    /// Phase 1.6-F.3-phase-1: get the per-vertex biome_weights if populated.
    /// Returns `None` for legacy-constructed chunks.
    pub fn biome_weights(&self) -> Option<&[[f32; 8]]> {
        self.biome_weights.as_deref()
    }

    /// Phase 1.6-F.4.B.3.D.3b: get the per-vertex `BiomeId` array if populated.
    /// Returns `None` for chunks constructed through legacy paths that don't
    /// run the climate-field per-vertex biome lookup.
    pub fn biome_ids(&self) -> Option<&[BiomeId]> {
        self.biome_ids.as_deref()
    }

    /// Get the chunk ID
    pub fn id(&self) -> ChunkId {
        self.id
    }

    /// Get the heightmap
    pub fn heightmap(&self) -> &Heightmap {
        &self.heightmap
    }

    /// Get mutable access to the heightmap (for sculpting brushes)
    pub fn heightmap_mut(&mut self) -> &mut Heightmap {
        self.mesh_dirty = true;
        &mut self.heightmap
    }

    /// Get the biome map
    pub fn biome_map(&self) -> &[BiomeType] {
        &self.biome_map
    }

    /// Get mutable access to the biome map (for paint brushes)
    pub fn biome_map_mut(&mut self) -> &mut [BiomeType] {
        self.mesh_dirty = true;
        &mut self.biome_map
    }

    /// Check if the mesh needs to be regenerated
    pub fn is_mesh_dirty(&self) -> bool {
        self.mesh_dirty
    }

    /// Mark the mesh as clean (after regeneration)
    pub fn mark_mesh_clean(&mut self) {
        self.mesh_dirty = false;
    }

    /// Apply hydraulic erosion to the chunk
    pub fn apply_erosion(&mut self, strength: f32) -> anyhow::Result<()> {
        self.heightmap.apply_hydraulic_erosion(strength)?;
        self.mesh_dirty = true;
        Ok(())
    }

    /// Get the height at a world position within this chunk
    pub fn get_height_at_world_pos(&self, world_pos: Vec3, chunk_size: f32) -> Option<f32> {
        let chunk_origin = self.id.to_world_pos(chunk_size);
        let local_pos = world_pos - chunk_origin;

        // Check if position is within chunk bounds
        if local_pos.x < 0.0
            || local_pos.x >= chunk_size
            || local_pos.z < 0.0
            || local_pos.z >= chunk_size
        {
            return None;
        }

        // Convert to heightmap coordinates
        let resolution = self.heightmap.resolution() as f32;
        let u = (local_pos.x / chunk_size) * (resolution - 1.0);
        let v = (local_pos.z / chunk_size) * (resolution - 1.0);

        Some(self.heightmap.sample_bilinear(u, v))
    }

    /// Get the biome at a world position within this chunk
    pub fn get_biome_at_world_pos(&self, world_pos: Vec3, chunk_size: f32) -> Option<BiomeType> {
        let chunk_origin = self.id.to_world_pos(chunk_size);
        let local_pos = world_pos - chunk_origin;

        // Check if position is within chunk bounds
        if local_pos.x < 0.0
            || local_pos.x >= chunk_size
            || local_pos.z < 0.0
            || local_pos.z >= chunk_size
        {
            return None;
        }

        // Convert to biome map coordinates
        let resolution = self.heightmap.resolution() as f32;
        let u = (local_pos.x / chunk_size) * (resolution - 1.0);
        let v = (local_pos.z / chunk_size) * (resolution - 1.0);

        let x = u.round() as usize;
        let z = v.round() as usize;
        let index = z * self.heightmap.resolution() as usize + x;

        self.biome_map.get(index).copied()
    }
}

/// Phase 1.6-F.3-phase-4.B: force exact C0 continuity at shared chunk
/// boundaries by averaging edge-vertex heights across adjacent chunks.
///
/// After world-coord droplet seeding (phase 3), shared-edge divergence
/// is typically ≤ 1 WU but a tail of outliers spikes to ~12 WU (the
/// expected state-dependent residual from droplets entering overlap
/// regions with different prior heightmap states). This function
/// runs after all chunks are generated, iterates every boundary
/// vertex, and sets it to the average of its values across all chunks
/// that contain it.
///
/// Corner vertices are shared by up to 4 chunks; edge vertices by 2.
/// The averaging handles both cases uniformly. Boundary vertices that
/// appear in only one chunk (at the radius boundary where the
/// neighbor is not loaded) are left unchanged.
///
/// Does not modify `biome_weights` — phase-3 established (and phase-4.A
/// re-verified) that biome_weights at shared edges already match
/// byte-for-byte via Shape A's pre-erosion invariant.
///
/// Does not modify normals — normals are recomputed downstream at
/// mesh-assembly time from the heights, so the updated heights
/// naturally produce continuous normals at boundaries.
///
/// Runs in O(N_chunks × chunk_edge_length) — trivial overhead relative
/// to erosion.
pub fn smooth_shared_vertices(chunks: &mut HashMap<ChunkId, TerrainChunk>) {
    if chunks.is_empty() {
        return;
    }

    // Assume all chunks have the same heightmap resolution (enforced by
    // WorldGenerator::generate_chunk[_with_climate]).
    let dim = chunks.values().next().unwrap().heightmap().resolution();
    if dim < 2 {
        return;
    }
    let max = dim - 1;
    let step = max as i64; // world-vertex index step per chunk

    // Pass 1: accumulate (sum, count) per world-vertex key for all
    // boundary vertices.
    let mut acc: HashMap<(i64, i64), (f32, u32)> = HashMap::new();
    for (chunk_id, chunk) in chunks.iter() {
        let heights = chunk.heightmap();
        for z in 0..dim {
            for x in 0..dim {
                let on_boundary = x == 0 || x == max || z == 0 || z == max;
                if !on_boundary {
                    continue;
                }
                let key = (
                    chunk_id.x as i64 * step + x as i64,
                    chunk_id.z as i64 * step + z as i64,
                );
                let y = heights.get_height(x, z);
                let entry = acc.entry(key).or_insert((0.0, 0));
                entry.0 += y;
                entry.1 += 1;
            }
        }
    }

    // Pass 2: write average back. Only vertices with count >= 2 had
    // neighbors that needed reconciling; count == 1 means this boundary
    // vertex is at the edge of the loaded region — no neighbor to match.
    for (chunk_id, chunk) in chunks.iter_mut() {
        let heightmap = chunk.heightmap_mut();
        for z in 0..dim {
            for x in 0..dim {
                let on_boundary = x == 0 || x == max || z == 0 || z == max;
                if !on_boundary {
                    continue;
                }
                let key = (
                    chunk_id.x as i64 * step + x as i64,
                    chunk_id.z as i64 * step + z as i64,
                );
                if let Some(&(sum, count)) = acc.get(&key) {
                    if count >= 2 {
                        let avg = sum / count as f32;
                        heightmap.set_height(x, z, avg);
                    }
                }
            }
        }
    }
}

/// Manages loading, unloading, and caching of terrain chunks
#[derive(Debug)]
pub struct ChunkManager {
    chunks: HashMap<ChunkId, TerrainChunk>,
    chunk_size: f32,
    #[allow(dead_code)]
    heightmap_resolution: u32, // currently unused
    max_loaded_chunks: usize,
}

impl ChunkManager {
    /// Create a new chunk manager
    pub fn new(chunk_size: f32, heightmap_resolution: u32) -> Self {
        Self {
            chunks: HashMap::new(),
            chunk_size,
            heightmap_resolution,
            max_loaded_chunks: 256, // Limit memory usage
        }
    }

    /// Add a chunk to the manager
    pub fn add_chunk(&mut self, chunk: TerrainChunk) {
        // If we're at capacity, remove the oldest chunk
        if self.chunks.len() >= self.max_loaded_chunks {
            // Simple LRU: remove a random chunk (in production, use proper LRU)
            if let Some(&chunk_id) = self.chunks.keys().next() {
                self.chunks.remove(&chunk_id);
            }
        }

        self.chunks.insert(chunk.id(), chunk);
    }

    /// Get a chunk by ID
    pub fn get_chunk(&self, chunk_id: ChunkId) -> Option<&TerrainChunk> {
        self.chunks.get(&chunk_id)
    }

    /// Get a mutable chunk by ID
    pub fn get_chunk_mut(&mut self, chunk_id: ChunkId) -> Option<&mut TerrainChunk> {
        self.chunks.get_mut(&chunk_id)
    }

    /// Check if a chunk is loaded
    pub fn has_chunk(&self, chunk_id: ChunkId) -> bool {
        self.chunks.contains_key(&chunk_id)
    }

    /// Get all chunk IDs within a radius of a center position
    pub fn get_chunks_in_radius(&self, center: Vec3, radius: u32) -> Vec<ChunkId> {
        ChunkId::get_chunks_in_radius(center, radius, self.chunk_size)
    }

    /// Unload chunks that are too far from the center
    pub fn unload_distant_chunks(&mut self, center: Vec3, max_radius: u32) {
        let center_chunk = ChunkId::from_world_pos(center, self.chunk_size);
        let max_distance = max_radius as f32;

        let to_remove: Vec<ChunkId> = self
            .chunks
            .keys()
            .filter(|&&chunk_id| chunk_id.distance_to(center_chunk) > max_distance)
            .copied()
            .collect();

        for chunk_id in to_remove {
            self.chunks.remove(&chunk_id);
        }
    }

    /// Get the height at a world position by finding the appropriate chunk
    pub fn get_height_at_world_pos(&self, world_pos: Vec3) -> Option<f32> {
        let chunk_id = ChunkId::from_world_pos(world_pos, self.chunk_size);
        self.get_chunk(chunk_id)?
            .get_height_at_world_pos(world_pos, self.chunk_size)
    }

    /// Get the biome at a world position by finding the appropriate chunk
    pub fn get_biome_at_world_pos(&self, world_pos: Vec3) -> Option<BiomeType> {
        let chunk_id = ChunkId::from_world_pos(world_pos, self.chunk_size);
        self.get_chunk(chunk_id)?
            .get_biome_at_world_pos(world_pos, self.chunk_size)
    }

    /// Get all loaded chunk IDs
    pub fn loaded_chunks(&self) -> Vec<ChunkId> {
        self.chunks.keys().copied().collect()
    }

    /// Get the total number of loaded chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Set the maximum number of loaded chunks
    pub fn set_max_loaded_chunks(&mut self, max_chunks: usize) {
        self.max_loaded_chunks = max_chunks;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HeightmapConfig;

    #[test]
    fn test_chunk_id_conversion() {
        let world_pos = Vec3::new(100.0, 0.0, 200.0);
        let chunk_size = 256.0;

        let chunk_id = ChunkId::from_world_pos(world_pos, chunk_size);
        let back_to_world = chunk_id.to_world_pos(chunk_size);

        assert_eq!(chunk_id, ChunkId::new(0, 0));
        assert_eq!(back_to_world, Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_chunk_radius() {
        let center = Vec3::new(128.0, 0.0, 128.0);
        let chunks = ChunkId::get_chunks_in_radius(center, 1, 256.0);

        assert_eq!(chunks.len(), 9); // 3x3 grid
        assert!(chunks.contains(&ChunkId::new(0, 0)));
        assert!(chunks.contains(&ChunkId::new(-1, -1)));
        assert!(chunks.contains(&ChunkId::new(1, 1)));
    }

    #[test]
    fn test_chunk_manager() {
        let mut manager = ChunkManager::new(256.0, 64);

        let chunk_id = ChunkId::new(0, 0);
        let heightmap = Heightmap::new(HeightmapConfig::default()).unwrap();
        let biome_map = vec![BiomeType::Grassland; 64 * 64];
        let chunk = TerrainChunk::new(chunk_id, heightmap, biome_map);

        manager.add_chunk(chunk);

        assert!(manager.has_chunk(chunk_id));
        assert_eq!(manager.chunk_count(), 1);
    }

    // Additional ChunkId tests
    #[test]
    fn test_chunk_id_new() {
        let id = ChunkId::new(5, -3);
        assert_eq!(id.x, 5);
        assert_eq!(id.z, -3);
    }

    #[test]
    fn test_chunk_id_from_world_pos_negative() {
        let world_pos = Vec3::new(-100.0, 50.0, -200.0);
        let chunk_size = 256.0;

        let chunk_id = ChunkId::from_world_pos(world_pos, chunk_size);
        // -100/256 = -0.39... -> floor = -1
        // -200/256 = -0.78... -> floor = -1
        assert_eq!(chunk_id, ChunkId::new(-1, -1));
    }

    #[test]
    fn test_chunk_id_from_world_pos_exact_boundary() {
        let world_pos = Vec3::new(256.0, 0.0, 512.0);
        let chunk_size = 256.0;

        let chunk_id = ChunkId::from_world_pos(world_pos, chunk_size);
        // 256/256 = 1 exactly
        // 512/256 = 2 exactly
        assert_eq!(chunk_id, ChunkId::new(1, 2));
    }

    #[test]
    fn test_chunk_id_to_center_pos() {
        let chunk_id = ChunkId::new(0, 0);
        let chunk_size = 256.0;
        let center = chunk_id.to_center_pos(chunk_size);

        assert_eq!(center, Vec3::new(128.0, 0.0, 128.0));
    }

    #[test]
    fn test_chunk_id_distance_to_same() {
        let id1 = ChunkId::new(5, 5);
        let id2 = ChunkId::new(5, 5);

        assert_eq!(id1.distance_to(id2), 0.0);
    }

    #[test]
    fn test_chunk_id_distance_to_diagonal() {
        let id1 = ChunkId::new(0, 0);
        let id2 = ChunkId::new(3, 4);

        // distance = sqrt(9 + 16) = 5
        assert_eq!(id1.distance_to(id2), 5.0);
    }

    #[test]
    fn test_chunk_id_clone_and_copy() {
        let id = ChunkId::new(10, 20);
        let cloned = id;
        let copied = id; // Copy trait

        assert_eq!(id, cloned);
        assert_eq!(id, copied);
    }

    #[test]
    fn test_chunk_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();

        set.insert(ChunkId::new(0, 0));
        set.insert(ChunkId::new(1, 1));
        set.insert(ChunkId::new(0, 0)); // duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_get_chunks_in_radius_zero() {
        let center = Vec3::new(128.0, 0.0, 128.0);
        let chunks = ChunkId::get_chunks_in_radius(center, 0, 256.0);

        assert_eq!(chunks.len(), 1); // Just the center chunk
    }

    #[test]
    fn test_get_chunks_in_radius_two() {
        let center = Vec3::new(128.0, 0.0, 128.0);
        let chunks = ChunkId::get_chunks_in_radius(center, 2, 256.0);

        assert_eq!(chunks.len(), 25); // 5x5 grid
    }

    // TerrainChunk tests
    fn create_test_chunk(id: ChunkId) -> TerrainChunk {
        let heightmap = Heightmap::new(HeightmapConfig::default()).unwrap();
        let resolution = heightmap.resolution() as usize;
        let biome_map = vec![BiomeType::Grassland; resolution * resolution];
        TerrainChunk::new(id, heightmap, biome_map)
    }

    #[test]
    fn test_terrain_chunk_id() {
        let chunk = create_test_chunk(ChunkId::new(3, 7));
        assert_eq!(chunk.id(), ChunkId::new(3, 7));
    }

    #[test]
    fn test_terrain_chunk_mesh_dirty() {
        let mut chunk = create_test_chunk(ChunkId::new(0, 0));

        // Initially dirty
        assert!(chunk.is_mesh_dirty());

        // Mark clean
        chunk.mark_mesh_clean();
        assert!(!chunk.is_mesh_dirty());
    }

    #[test]
    fn test_terrain_chunk_heightmap_access() {
        let chunk = create_test_chunk(ChunkId::new(0, 0));
        let heightmap = chunk.heightmap();

        assert!(heightmap.resolution() > 0);
    }

    #[test]
    fn test_terrain_chunk_biome_map_access() {
        let chunk = create_test_chunk(ChunkId::new(0, 0));
        let biome_map = chunk.biome_map();

        assert!(!biome_map.is_empty());
        assert_eq!(biome_map[0], BiomeType::Grassland);
    }

    #[test]
    fn test_terrain_chunk_get_height_at_world_pos_valid() {
        let chunk = create_test_chunk(ChunkId::new(0, 0));
        let chunk_size = 256.0;

        // Position within chunk
        let world_pos = Vec3::new(128.0, 0.0, 128.0);
        let height = chunk.get_height_at_world_pos(world_pos, chunk_size);

        assert!(height.is_some());
    }

    #[test]
    fn test_terrain_chunk_get_height_at_world_pos_outside() {
        let chunk = create_test_chunk(ChunkId::new(0, 0));
        let chunk_size = 256.0;

        // Position outside chunk (negative x)
        let world_pos = Vec3::new(-10.0, 0.0, 128.0);
        let height = chunk.get_height_at_world_pos(world_pos, chunk_size);

        assert!(height.is_none());
    }

    #[test]
    fn test_terrain_chunk_get_height_at_world_pos_outside_z() {
        let chunk = create_test_chunk(ChunkId::new(0, 0));
        let chunk_size = 256.0;

        // Position outside chunk (z >= chunk_size)
        let world_pos = Vec3::new(128.0, 0.0, 300.0);
        let height = chunk.get_height_at_world_pos(world_pos, chunk_size);

        assert!(height.is_none());
    }

    #[test]
    fn test_terrain_chunk_get_biome_at_world_pos_valid() {
        let chunk = create_test_chunk(ChunkId::new(0, 0));
        let chunk_size = 256.0;

        // Position within chunk
        let world_pos = Vec3::new(128.0, 0.0, 128.0);
        let biome = chunk.get_biome_at_world_pos(world_pos, chunk_size);

        assert_eq!(biome, Some(BiomeType::Grassland));
    }

    #[test]
    fn test_terrain_chunk_get_biome_at_world_pos_outside() {
        let chunk = create_test_chunk(ChunkId::new(0, 0));
        let chunk_size = 256.0;

        // Position outside chunk
        let world_pos = Vec3::new(-10.0, 0.0, -10.0);
        let biome = chunk.get_biome_at_world_pos(world_pos, chunk_size);

        assert!(biome.is_none());
    }

    #[test]
    fn test_terrain_chunk_apply_erosion() {
        let mut chunk = create_test_chunk(ChunkId::new(0, 0));

        // Mark clean first
        chunk.mark_mesh_clean();
        assert!(!chunk.is_mesh_dirty());

        // Apply erosion
        let result = chunk.apply_erosion(0.1);
        assert!(result.is_ok());

        // Should mark mesh dirty
        assert!(chunk.is_mesh_dirty());
    }

    // ChunkManager tests
    #[test]
    fn test_chunk_manager_new() {
        let manager = ChunkManager::new(256.0, 64);
        assert_eq!(manager.chunk_count(), 0);
    }

    #[test]
    fn test_chunk_manager_add_and_get() {
        let mut manager = ChunkManager::new(256.0, 64);
        let chunk = create_test_chunk(ChunkId::new(5, 5));

        manager.add_chunk(chunk);

        let retrieved = manager.get_chunk(ChunkId::new(5, 5));
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), ChunkId::new(5, 5));
    }

    #[test]
    fn test_chunk_manager_get_chunk_mut() {
        let mut manager = ChunkManager::new(256.0, 64);
        let chunk = create_test_chunk(ChunkId::new(0, 0));
        manager.add_chunk(chunk);

        let chunk_mut = manager.get_chunk_mut(ChunkId::new(0, 0));
        assert!(chunk_mut.is_some());

        // Modify the chunk
        chunk_mut.unwrap().mark_mesh_clean();

        // Verify modification
        let chunk = manager.get_chunk(ChunkId::new(0, 0)).unwrap();
        assert!(!chunk.is_mesh_dirty());
    }

    #[test]
    fn test_chunk_manager_has_chunk() {
        let mut manager = ChunkManager::new(256.0, 64);
        let chunk = create_test_chunk(ChunkId::new(1, 2));
        manager.add_chunk(chunk);

        assert!(manager.has_chunk(ChunkId::new(1, 2)));
        assert!(!manager.has_chunk(ChunkId::new(9, 9)));
    }

    #[test]
    fn test_chunk_manager_loaded_chunks() {
        let mut manager = ChunkManager::new(256.0, 64);

        for i in 0..3 {
            manager.add_chunk(create_test_chunk(ChunkId::new(i, i)));
        }

        let loaded = manager.loaded_chunks();
        assert_eq!(loaded.len(), 3);
    }

    #[test]
    fn test_chunk_manager_unload_distant_chunks() {
        let mut manager = ChunkManager::new(256.0, 64);

        // Add some nearby and distant chunks
        manager.add_chunk(create_test_chunk(ChunkId::new(0, 0)));
        manager.add_chunk(create_test_chunk(ChunkId::new(1, 0)));
        manager.add_chunk(create_test_chunk(ChunkId::new(10, 10))); // Distant

        assert_eq!(manager.chunk_count(), 3);

        // Unload chunks more than 5 chunk units away from center
        manager.unload_distant_chunks(Vec3::new(128.0, 0.0, 128.0), 5);

        assert_eq!(manager.chunk_count(), 2);
        assert!(manager.has_chunk(ChunkId::new(0, 0)));
        assert!(manager.has_chunk(ChunkId::new(1, 0)));
        assert!(!manager.has_chunk(ChunkId::new(10, 10)));
    }

    #[test]
    fn test_chunk_manager_get_height_at_world_pos() {
        let mut manager = ChunkManager::new(256.0, 64);
        manager.add_chunk(create_test_chunk(ChunkId::new(0, 0)));

        // Valid position in chunk
        let height = manager.get_height_at_world_pos(Vec3::new(128.0, 0.0, 128.0));
        assert!(height.is_some());

        // Position outside loaded chunks
        let height = manager.get_height_at_world_pos(Vec3::new(1000.0, 0.0, 1000.0));
        assert!(height.is_none());
    }

    #[test]
    fn test_chunk_manager_get_biome_at_world_pos() {
        let mut manager = ChunkManager::new(256.0, 64);
        manager.add_chunk(create_test_chunk(ChunkId::new(0, 0)));

        // Valid position in chunk
        let biome = manager.get_biome_at_world_pos(Vec3::new(128.0, 0.0, 128.0));
        assert!(biome.is_some());

        // Position outside loaded chunks
        let biome = manager.get_biome_at_world_pos(Vec3::new(1000.0, 0.0, 1000.0));
        assert!(biome.is_none());
    }

    #[test]
    fn test_chunk_manager_set_max_loaded_chunks() {
        let mut manager = ChunkManager::new(256.0, 64);
        manager.set_max_loaded_chunks(10);

        // Add 15 chunks
        for i in 0..15 {
            manager.add_chunk(create_test_chunk(ChunkId::new(i, 0)));
        }

        // Should be capped at 10 chunks
        assert!(manager.chunk_count() <= 10);
    }

    #[test]
    fn test_chunk_manager_get_chunks_in_radius() {
        let manager = ChunkManager::new(256.0, 64);

        let center = Vec3::new(128.0, 0.0, 128.0);
        let chunks = manager.get_chunks_in_radius(center, 1);

        assert_eq!(chunks.len(), 9); // 3x3 grid
    }

    #[test]
    fn test_terrain_chunk_clone() {
        let chunk = create_test_chunk(ChunkId::new(1, 2));
        let cloned = chunk.clone();

        assert_eq!(chunk.id(), cloned.id());
    }

    #[test]
    fn test_chunk_id_serialization() {
        let id = ChunkId::new(42, -17);
        let serialized = serde_json::to_string(&id).unwrap();
        let deserialized: ChunkId = serde_json::from_str(&serialized).unwrap();

        assert_eq!(id, deserialized);
    }
}
