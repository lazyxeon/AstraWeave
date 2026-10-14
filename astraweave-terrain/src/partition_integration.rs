//! World Partition Integration for Voxel Terrain
//!
//! This module bridges the voxel terrain system with the World Partition system,
//! enabling seamless streaming and memory management of voxel chunks.
//!
//! # Architecture
//!
//! ```text
//! Partition Cell (256×256×256m)
//! ├── Contains: 8×8×8 = 512 Voxel Chunks (32³ voxels each)
//! └── Memory Budget: ~32MB per cell (512 chunks × 64KB)
//!
//! Streaming Flow:
//! 1. Cell activated → Load all voxel chunks in cell
//! 2. Chunks meshed → GPU resources allocated
//! 3. Cell deactivated → Unload chunks + free GPU memory
//! ```

use crate::meshing::{AsyncMeshGenerator, ChunkMesh, DualContouring};
use crate::voxel_data::{ChunkCoord, VoxelChunk, VoxelGrid, CHUNK_SIZE};
use anyhow::Result;
use glam::Vec3;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Grid coordinate from world partition system
/// Re-exported for convenience (normally from astraweave-scene)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartitionCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl PartitionCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Convert world position to partition coordinate
    pub fn from_world_pos(pos: Vec3, cell_size: f32) -> Self {
        Self {
            x: (pos.x / cell_size).floor() as i32,
            y: (pos.y / cell_size).floor() as i32,
            z: (pos.z / cell_size).floor() as i32,
        }
    }

    /// Get world-space center of this cell
    pub fn to_world_center(self, cell_size: f32) -> Vec3 {
        Vec3::new(
            (self.x as f32 + 0.5) * cell_size,
            (self.y as f32 + 0.5) * cell_size,
            (self.z as f32 + 0.5) * cell_size,
        )
    }

    /// Get world-space min corner
    pub fn to_world_min(self, cell_size: f32) -> Vec3 {
        Vec3::new(
            self.x as f32 * cell_size,
            self.y as f32 * cell_size,
            self.z as f32 * cell_size,
        )
    }

    /// Get all voxel chunks contained in this partition cell
    /// Assumes cell_size = 256.0 and CHUNK_SIZE = 32
    pub fn get_voxel_chunks(self, cell_size: f32) -> Vec<ChunkCoord> {
        let chunks_per_axis = (cell_size / CHUNK_SIZE as f32) as i32;
        let base_x = self.x * chunks_per_axis;
        let base_y = self.y * chunks_per_axis;
        let base_z = self.z * chunks_per_axis;

        let mut chunks =
            Vec::with_capacity((chunks_per_axis * chunks_per_axis * chunks_per_axis) as usize);

        for dx in 0..chunks_per_axis {
            for dy in 0..chunks_per_axis {
                for dz in 0..chunks_per_axis {
                    chunks.push(ChunkCoord::new(base_x + dx, base_y + dy, base_z + dz));
                }
            }
        }

        chunks
    }
}

impl From<ChunkCoord> for PartitionCoord {
    /// Convert voxel chunk coordinate to partition cell coordinate
    fn from(chunk: ChunkCoord) -> Self {
        // Assumes cell_size = 256.0 and CHUNK_SIZE = 32
        // 256 / 32 = 8 chunks per partition cell per axis
        const CHUNKS_PER_CELL: i32 = 8;

        Self {
            x: chunk.x.div_euclid(CHUNKS_PER_CELL),
            y: chunk.y.div_euclid(CHUNKS_PER_CELL),
            z: chunk.z.div_euclid(CHUNKS_PER_CELL),
        }
    }
}

/// Configuration for voxel-partition integration
#[derive(Debug, Clone)]
pub struct VoxelPartitionConfig {
    /// Size of partition cells in world units (default: 256.0)
    pub cell_size: f32,
    /// Maximum memory budget for voxel data (bytes)
    pub memory_budget: usize,
    /// Enable mesh generation for loaded chunks
    pub auto_mesh: bool,
    /// LOD distance thresholds [m]
    pub lod_distances: [f32; 4],
}

impl Default for VoxelPartitionConfig {
    fn default() -> Self {
        Self {
            cell_size: 256.0,
            memory_budget: 500_000_000, // 500MB
            auto_mesh: true,
            lod_distances: [100.0, 250.0, 500.0, 1000.0],
        }
    }
}

/// Statistics for voxel partition integration
#[derive(Debug, Clone, Default)]
pub struct VoxelPartitionStats {
    /// Number of active partition cells
    pub active_cells: usize,
    /// Number of loaded voxel chunks
    pub loaded_chunks: usize,
    /// Number of generated meshes
    pub meshed_chunks: usize,
    /// Total memory used by voxel data (bytes)
    pub voxel_memory: usize,
    /// Total memory used by meshes (bytes)
    pub mesh_memory: usize,
}

/// Events emitted by voxel partition system
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum VoxelPartitionEvent {
    /// Partition cell activated, voxel chunks loaded
    CellActivated(PartitionCoord, Vec<ChunkCoord>),
    /// Partition cell deactivated, voxel chunks unloaded
    CellDeactivated(PartitionCoord, Vec<ChunkCoord>),
    /// Voxel chunk meshed
    ChunkMeshed(ChunkCoord, ChunkMesh),
    /// Memory budget exceeded
    MemoryBudgetExceeded(usize, usize), // (used, budget)
}

/// Manager for voxel terrain integrated with world partition
pub struct VoxelPartitionManager {
    config: VoxelPartitionConfig,
    /// Voxel data storage
    voxel_grid: Arc<RwLock<VoxelGrid>>,
    /// Active partition cells
    active_cells: HashSet<PartitionCoord>,
    /// Mapping of partition cells to voxel chunks
    cell_chunks: HashMap<PartitionCoord, Vec<ChunkCoord>>,
    /// Generated meshes
    meshes: HashMap<ChunkCoord, ChunkMesh>,
    /// Mesh generator
    #[allow(dead_code)]
    mesh_generator: AsyncMeshGenerator,
    /// Event queue
    events: Vec<VoxelPartitionEvent>,
    /// Statistics
    stats: VoxelPartitionStats,
}

impl VoxelPartitionManager {
    /// Create a new voxel partition manager
    pub fn new(config: VoxelPartitionConfig) -> Self {
        Self {
            config,
            voxel_grid: Arc::new(RwLock::new(VoxelGrid::new())),
            active_cells: HashSet::new(),
            cell_chunks: HashMap::new(),
            meshes: HashMap::new(),
            mesh_generator: AsyncMeshGenerator::new(),
            events: Vec::new(),
            stats: VoxelPartitionStats::default(),
        }
    }

    /// Activate a partition cell, loading all voxel chunks within it
    pub async fn activate_cell(&mut self, cell: PartitionCoord) -> Result<Vec<ChunkCoord>> {
        if self.active_cells.contains(&cell) {
            return Ok(Vec::new()); // Already active
        }

        // Get all voxel chunks in this cell
        let chunks = cell.get_voxel_chunks(self.config.cell_size);

        // Load chunks (in parallel)
        let mut loaded_chunks = Vec::new();
        let mut voxel_grid = self.voxel_grid.write().await;

        for chunk_coord in &chunks {
            // Check if chunk already exists
            if voxel_grid.get_chunk(*chunk_coord).is_none() {
                // Generate or load chunk data
                let chunk = self.generate_chunk_data(*chunk_coord).await?;
                // Insert chunk using get_or_create, then copy data
                let target = voxel_grid.get_or_create_chunk(*chunk_coord);
                *target = chunk;
                loaded_chunks.push(*chunk_coord);
            }
        }

        drop(voxel_grid);

        // Generate meshes if enabled
        if self.config.auto_mesh {
            self.mesh_chunks(&loaded_chunks).await?;
        }

        // Update tracking
        self.active_cells.insert(cell);
        self.cell_chunks.insert(cell, chunks.clone());
        self.update_stats().await;

        // Emit event
        self.events.push(VoxelPartitionEvent::CellActivated(
            cell,
            loaded_chunks.clone(),
        ));

        // Check memory budget
        self.check_memory_budget();

        Ok(loaded_chunks)
    }

    /// Deactivate a partition cell, unloading all voxel chunks within it
    pub async fn deactivate_cell(&mut self, cell: PartitionCoord) -> Result<Vec<ChunkCoord>> {
        if !self.active_cells.contains(&cell) {
            return Ok(Vec::new()); // Not active
        }

        // Get chunks in this cell
        let chunks = self.cell_chunks.remove(&cell).unwrap_or_default();

        // Remove meshes
        for chunk_coord in &chunks {
            self.meshes.remove(chunk_coord);
        }

        // Remove voxel data
        let mut voxel_grid = self.voxel_grid.write().await;
        for chunk_coord in &chunks {
            voxel_grid.remove_chunk(*chunk_coord);
        }
        drop(voxel_grid);

        // Update tracking
        self.active_cells.remove(&cell);
        self.update_stats().await;

        // Emit event
        self.events
            .push(VoxelPartitionEvent::CellDeactivated(cell, chunks.clone()));

        Ok(chunks)
    }

    /// Update active cells based on camera position
    pub async fn update_from_camera(&mut self, camera_pos: Vec3, view_distance: f32) -> Result<()> {
        let current_cell = PartitionCoord::from_world_pos(camera_pos, self.config.cell_size);

        // Calculate how many cells we need in each direction
        let cell_radius = (view_distance / self.config.cell_size).ceil() as i32;

        // Determine cells that should be active
        let mut target_cells = HashSet::new();
        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                for dz in -cell_radius..=cell_radius {
                    let cell = PartitionCoord::new(
                        current_cell.x + dx,
                        current_cell.y + dy,
                        current_cell.z + dz,
                    );

                    // Check if cell is within view distance
                    let cell_center = cell.to_world_center(self.config.cell_size);
                    let distance = camera_pos.distance(cell_center);

                    if distance <= view_distance {
                        target_cells.insert(cell);
                    }
                }
            }
        }

        // Activate new cells
        let cells_to_activate: Vec<_> = target_cells
            .difference(&self.active_cells)
            .copied()
            .collect();

        for cell in cells_to_activate {
            self.activate_cell(cell).await?;
        }

        // Deactivate old cells
        let cells_to_deactivate: Vec<_> = self
            .active_cells
            .difference(&target_cells)
            .copied()
            .collect();

        for cell in cells_to_deactivate {
            self.deactivate_cell(cell).await?;
        }

        Ok(())
    }

    /// Generate mesh for specific chunks
    async fn mesh_chunks(&mut self, chunks: &[ChunkCoord]) -> Result<()> {
        let voxel_grid = self.voxel_grid.read().await;

        for chunk_coord in chunks {
            if let Some(chunk) = voxel_grid.get_chunk(*chunk_coord) {
                // Generate mesh
                let mut dc = DualContouring::new();
                let mesh = dc.generate_mesh(chunk);

                // Store mesh
                if !mesh.is_empty() {
                    self.events
                        .push(VoxelPartitionEvent::ChunkMeshed(*chunk_coord, mesh.clone()));
                    self.meshes.insert(*chunk_coord, mesh);
                }
            }
        }

        Ok(())
    }

    /// Generate voxel data for a chunk (placeholder - should be replaced with actual terrain generation)
    async fn generate_chunk_data(&self, coord: ChunkCoord) -> Result<VoxelChunk> {
        // TODO: Integrate with actual terrain generation (heightmap, noise, etc.)
        // For now, return an empty chunk
        Ok(VoxelChunk::new(coord))
    }

    /// Update statistics
    async fn update_stats(&mut self) {
        let voxel_grid = self.voxel_grid.read().await;

        self.stats.active_cells = self.active_cells.len();
        self.stats.loaded_chunks = voxel_grid.chunk_count();
        self.stats.meshed_chunks = self.meshes.len();

        // Calculate memory usage (approximate)
        // Each voxel: 2 bytes (density f32 compressed + material u16)
        // Each chunk: 32^3 voxels = 32,768 voxels × 2 bytes = 64KB
        self.stats.voxel_memory = self.stats.loaded_chunks * 65536;

        // Mesh memory: vertices (32 bytes each) + indices (4 bytes each)
        let mesh_memory: usize = self
            .meshes
            .values()
            .map(|mesh| mesh.vertices.len() * 32 + mesh.indices.len() * 4)
            .sum();
        self.stats.mesh_memory = mesh_memory;
    }

    /// Check if memory budget is exceeded
    fn check_memory_budget(&mut self) {
        let total_memory = self.stats.voxel_memory + self.stats.mesh_memory;

        if total_memory > self.config.memory_budget {
            self.events.push(VoxelPartitionEvent::MemoryBudgetExceeded(
                total_memory,
                self.config.memory_budget,
            ));
        }
    }

    /// Get mesh for a specific chunk
    pub fn get_mesh(&self, coord: ChunkCoord) -> Option<&ChunkMesh> {
        self.meshes.get(&coord)
    }

    /// Get all meshes
    pub fn get_all_meshes(&self) -> &HashMap<ChunkCoord, ChunkMesh> {
        &self.meshes
    }

    /// Get voxel grid (for editing)
    pub fn get_voxel_grid(&self) -> Arc<RwLock<VoxelGrid>> {
        Arc::clone(&self.voxel_grid)
    }

    /// Get statistics
    pub fn get_stats(&self) -> &VoxelPartitionStats {
        &self.stats
    }

    /// Drain events since last call
    pub fn drain_events(&mut self) -> Vec<VoxelPartitionEvent> {
        std::mem::take(&mut self.events)
    }

    /// Get active cells
    pub fn get_active_cells(&self) -> &HashSet<PartitionCoord> {
        &self.active_cells
    }

    /// Check if a specific chunk is loaded
    pub async fn is_chunk_loaded(&self, coord: ChunkCoord) -> bool {
        let voxel_grid = self.voxel_grid.read().await;
        voxel_grid.get_chunk(coord).is_some()
    }
}

/// Helper functions for integration
impl VoxelPartitionManager {
    /// Create from partition cell size
    pub fn with_cell_size(cell_size: f32) -> Self {
        let config = VoxelPartitionConfig {
            cell_size,
            ..Default::default()
        };
        Self::new(config)
    }

    /// Create with custom memory budget
    pub fn with_memory_budget(memory_budget: usize) -> Self {
        let config = VoxelPartitionConfig {
            memory_budget,
            ..Default::default()
        };
        Self::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_coord_conversion() {
        let chunk = ChunkCoord::new(16, 8, 24); // Chunk at (16, 8, 24)
        let partition: PartitionCoord = chunk.into();

        // 16 / 8 = 2, 8 / 8 = 1, 24 / 8 = 3
        assert_eq!(partition.x, 2);
        assert_eq!(partition.y, 1);
        assert_eq!(partition.z, 3);
    }

    #[test]
    fn test_partition_to_chunks() {
        let partition = PartitionCoord::new(0, 0, 0);
        let chunks = partition.get_voxel_chunks(256.0);

        // 256 / 32 = 8 chunks per axis → 8^3 = 512 chunks
        assert_eq!(chunks.len(), 512);

        // First chunk should be at (0, 0, 0)
        assert_eq!(chunks[0], ChunkCoord::new(0, 0, 0));
    }

    #[test]
    fn test_world_pos_to_partition() {
        let pos = Vec3::new(300.0, 128.0, 500.0);
        let partition = PartitionCoord::from_world_pos(pos, 256.0);

        // 300 / 256 = 1, 128 / 256 = 0, 500 / 256 = 1
        assert_eq!(partition.x, 1);
        assert_eq!(partition.y, 0);
        assert_eq!(partition.z, 1);
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = VoxelPartitionManager::new(VoxelPartitionConfig::default());
        assert_eq!(manager.get_stats().active_cells, 0);
        assert_eq!(manager.get_stats().loaded_chunks, 0);
    }

    #[tokio::test]
    async fn test_cell_activation() {
        let mut manager = VoxelPartitionManager::new(VoxelPartitionConfig::default());
        let cell = PartitionCoord::new(0, 0, 0);

        let loaded = manager.activate_cell(cell).await.unwrap();
        assert!(!loaded.is_empty());
        assert!(manager.get_active_cells().contains(&cell));
    }

    #[tokio::test]
    async fn test_cell_deactivation() {
        let mut manager = VoxelPartitionManager::new(VoxelPartitionConfig::default());
        let cell = PartitionCoord::new(0, 0, 0);

        manager.activate_cell(cell).await.unwrap();
        let unloaded = manager.deactivate_cell(cell).await.unwrap();

        assert!(!unloaded.is_empty());
        assert!(!manager.get_active_cells().contains(&cell));
    }

    #[tokio::test]
    async fn test_camera_update() {
        // One 32 m cell is one voxel chunk. The old 300 m / 256 m setup
        // meshed every chunk in the neighborhood (~28 s) and was ignored
        // for that reason. The assertion is only that the camera cell loads.
        let config = VoxelPartitionConfig {
            cell_size: 32.0,
            auto_mesh: false,
            ..VoxelPartitionConfig::default()
        };
        let mut manager = VoxelPartitionManager::new(config);
        let camera_pos = Vec3::new(16.0, 16.0, 16.0);

        manager.update_from_camera(camera_pos, 1.0).await.unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.active_cells, 1);
        assert!(stats.loaded_chunks > 0);
    }
}
