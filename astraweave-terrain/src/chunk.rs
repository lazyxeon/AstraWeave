//! Terrain chunk management and streaming

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
    mesh_dirty: bool,
}

impl TerrainChunk {
    /// Create a new terrain chunk
    pub fn new(id: ChunkId, heightmap: Heightmap, biome_map: Vec<BiomeType>) -> Self {
        Self {
            id,
            heightmap,
            biome_map,
            mesh_dirty: true,
        }
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
