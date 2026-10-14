//! Terrain Persistence Module
//!
//! This module provides save/load functionality for modified terrain regions.
//! It supports:
//! - Individual chunk serialization with compression
//! - Dirty chunk tracking for incremental saves
//! - Region-based loading for efficient streaming
//! - Versioned save format for forward compatibility

use crate::{ChunkCoord, VoxelChunk};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// Current save format version for forward compatibility
pub const TERRAIN_SAVE_VERSION: u32 = 1;

/// Configuration for terrain persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainPersistenceConfig {
    /// Base directory for terrain saves
    pub save_directory: PathBuf,
    /// Use compression for saves (reduces size but slower)
    pub use_compression: bool,
    /// Maximum chunks to save per batch (for incremental saves)
    pub batch_size: usize,
    /// Auto-save interval in seconds (0 = disabled)
    pub auto_save_interval_seconds: f32,
}

impl Default for TerrainPersistenceConfig {
    fn default() -> Self {
        Self {
            save_directory: PathBuf::from("terrain_saves"),
            use_compression: true,
            batch_size: 32,
            auto_save_interval_seconds: 60.0,
        }
    }
}

/// Header for terrain save files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainSaveHeader {
    /// Save format version
    pub version: u32,
    /// World seed (for regeneration)
    pub world_seed: u64,
    /// Number of modified chunks in this save
    pub chunk_count: u32,
    /// Timestamp when save was created
    pub timestamp: u64,
    /// Optional save name/description
    pub description: Option<String>,
}

impl TerrainSaveHeader {
    /// Create a new save header
    pub fn new(world_seed: u64, chunk_count: u32) -> Self {
        Self {
            version: TERRAIN_SAVE_VERSION,
            world_seed,
            chunk_count,
            // Tick 0 until the caller passes a sim tick via `at_tick`.
            // Wall-clock seconds must not enter a save that replay will hash.
            timestamp: 0,
            description: None,
        }
    }

    /// Header whose timestamp is a simulation tick, not `SystemTime`.
    pub fn at_tick(mut self, tick: u64) -> Self {
        self.timestamp = tick;
        self
    }

    /// Create header with description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Result of a terrain save operation
#[derive(Debug, Clone)]
pub struct TerrainSaveResult {
    /// Path where save was written
    pub path: PathBuf,
    /// Number of chunks saved
    pub chunks_saved: u32,
    /// Size of save file in bytes
    pub file_size: u64,
    /// Time taken to save in milliseconds
    pub save_time_ms: u64,
}

/// Result of a terrain load operation
#[derive(Debug, Clone)]
pub struct TerrainLoadResult {
    /// Header information from the save
    pub header: TerrainSaveHeader,
    /// Number of chunks loaded
    pub chunks_loaded: u32,
    /// Time taken to load in milliseconds
    pub load_time_ms: u64,
}

/// Terrain persistence manager
#[derive(Debug)]
pub struct TerrainPersistence {
    /// Configuration
    config: TerrainPersistenceConfig,
    /// Tracking dirty (modified) chunks that need saving
    dirty_chunks: HashSet<ChunkCoord>,
    /// Last auto-save timestamp
    last_auto_save: std::time::Instant,
    /// Statistics
    stats: PersistenceStats,
}

/// Statistics for terrain persistence operations
#[derive(Debug, Clone, Default)]
pub struct PersistenceStats {
    /// Total chunks saved this session
    pub total_chunks_saved: u64,
    /// Total chunks loaded this session
    pub total_chunks_loaded: u64,
    /// Total bytes written this session
    pub total_bytes_written: u64,
    /// Total bytes read this session
    pub total_bytes_read: u64,
    /// Number of save operations
    pub save_count: u32,
    /// Number of load operations
    pub load_count: u32,
}

impl TerrainPersistence {
    /// Create a new terrain persistence manager
    pub fn new(config: TerrainPersistenceConfig) -> Self {
        Self {
            config,
            dirty_chunks: HashSet::new(),
            last_auto_save: std::time::Instant::now(),
            stats: PersistenceStats::default(),
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(TerrainPersistenceConfig::default())
    }

    /// Mark a chunk as modified (needs saving)
    pub fn mark_dirty(&mut self, coord: ChunkCoord) {
        self.dirty_chunks.insert(coord);
    }

    /// Mark multiple chunks as modified
    pub fn mark_dirty_batch(&mut self, coords: impl IntoIterator<Item = ChunkCoord>) {
        self.dirty_chunks.extend(coords);
    }

    /// Check if a chunk is marked as dirty
    pub fn is_dirty(&self, coord: &ChunkCoord) -> bool {
        self.dirty_chunks.contains(coord)
    }

    /// Get number of dirty chunks
    pub fn dirty_count(&self) -> usize {
        self.dirty_chunks.len()
    }

    /// Clear dirty tracking (after successful save)
    pub fn clear_dirty(&mut self) {
        self.dirty_chunks.clear();
    }

    /// Check if auto-save should trigger
    pub fn should_auto_save(&self) -> bool {
        if self.config.auto_save_interval_seconds <= 0.0 {
            return false;
        }
        if self.dirty_chunks.is_empty() {
            return false;
        }
        self.last_auto_save.elapsed().as_secs_f32() >= self.config.auto_save_interval_seconds
    }

    /// Save modified chunks to a file
    ///
    /// # Arguments
    /// * `chunks` - HashMap of chunk coordinates to chunk data
    /// * `world_seed` - The world seed for regeneration
    /// * `save_name` - Optional name for the save file
    pub fn save_chunks(
        &mut self,
        chunks: &HashMap<ChunkCoord, VoxelChunk>,
        world_seed: u64,
        save_name: Option<&str>,
    ) -> anyhow::Result<TerrainSaveResult> {
        let start = std::time::Instant::now();

        // Ensure save directory exists
        fs::create_dir_all(&self.config.save_directory)?;

        // Collect dirty chunks to save
        let chunks_to_save: Vec<_> = self
            .dirty_chunks
            .iter()
            .filter_map(|coord| chunks.get(coord).map(|chunk| (*coord, chunk.clone())))
            .collect();

        let chunk_count = chunks_to_save.len() as u32;

        // Filename is a function of the seed and chunk count, not the wall clock.
        let filename = save_name
            .map(|s| format!("{}.terrain", s))
            .unwrap_or_else(|| format!("terrain_{world_seed}_{chunk_count}.terrain"));
        let path = self.config.save_directory.join(&filename);

        // Create header
        let header = TerrainSaveHeader::new(world_seed, chunk_count);

        // Serialize data
        let save_data = TerrainSaveData {
            header: header.clone(),
            chunks: chunks_to_save,
        };

        // Write to file
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        if self.config.use_compression {
            // Serialize to buffer first, then compress
            let serialized = bincode::serialize(&save_data)?;
            let compressed = miniz_oxide::deflate::compress_to_vec(&serialized, 6);
            writer.write_all(&compressed)?;
        } else {
            bincode::serialize_into(&mut writer, &save_data)?;
        }

        writer.flush()?;

        // Get file size
        let file_size = fs::metadata(&path)?.len();

        // Update stats
        self.stats.total_chunks_saved += chunk_count as u64;
        self.stats.total_bytes_written += file_size;
        self.stats.save_count += 1;

        // Clear dirty tracking for saved chunks
        self.dirty_chunks.clear();
        self.last_auto_save = std::time::Instant::now();

        Ok(TerrainSaveResult {
            path,
            chunks_saved: chunk_count,
            file_size,
            save_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Load chunks from a save file
    ///
    /// # Arguments
    /// * `path` - Path to the save file
    ///
    /// # Returns
    /// Tuple of (load result, loaded chunks)
    pub fn load_chunks(
        &mut self,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<(TerrainLoadResult, HashMap<ChunkCoord, VoxelChunk>)> {
        let start = std::time::Instant::now();
        let path = path.as_ref();

        let file = File::open(path)?;
        let file_size = file.metadata()?.len();
        let mut reader = BufReader::new(file);

        let save_data: TerrainSaveData = if self.config.use_compression {
            // Read all data and decompress
            let mut compressed = Vec::new();
            reader.read_to_end(&mut compressed)?;
            let decompressed = miniz_oxide::inflate::decompress_to_vec(&compressed)
                .map_err(|e| anyhow::anyhow!("Decompression failed: {:?}", e))?;
            bincode::deserialize(&decompressed)?
        } else {
            bincode::deserialize_from(&mut reader)?
        };

        // Version check
        if save_data.header.version > TERRAIN_SAVE_VERSION {
            anyhow::bail!(
                "Save file version {} is newer than supported version {}",
                save_data.header.version,
                TERRAIN_SAVE_VERSION
            );
        }

        let chunk_count = save_data.chunks.len() as u32;

        // Convert to HashMap
        let chunks: HashMap<ChunkCoord, VoxelChunk> = save_data.chunks.into_iter().collect();

        // Update stats
        self.stats.total_chunks_loaded += chunk_count as u64;
        self.stats.total_bytes_read += file_size;
        self.stats.load_count += 1;

        Ok((
            TerrainLoadResult {
                header: save_data.header,
                chunks_loaded: chunk_count,
                load_time_ms: start.elapsed().as_millis() as u64,
            },
            chunks,
        ))
    }

    /// List available save files in the save directory
    pub fn list_saves(&self) -> anyhow::Result<Vec<TerrainSaveInfo>> {
        let mut saves = Vec::new();

        if !self.config.save_directory.exists() {
            return Ok(saves);
        }

        for entry in fs::read_dir(&self.config.save_directory)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "terrain") {
                if let Ok(info) = self.read_save_info(&path) {
                    saves.push(info);
                }
            }
        }

        // Sort by timestamp (newest first)
        saves.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(saves)
    }

    /// Read save file info without loading all chunks
    fn read_save_info(&self, path: &Path) -> anyhow::Result<TerrainSaveInfo> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let mut reader = BufReader::new(file);

        // For compressed files, we need to decompress first
        // For efficiency, we could store header separately, but for now we read the whole file
        let header: TerrainSaveHeader = if self.config.use_compression {
            let mut compressed = Vec::new();
            reader.read_to_end(&mut compressed)?;
            let decompressed = miniz_oxide::inflate::decompress_to_vec(&compressed)
                .map_err(|e| anyhow::anyhow!("Decompression failed: {:?}", e))?;
            let save_data: TerrainSaveData = bincode::deserialize(&decompressed)?;
            save_data.header
        } else {
            let save_data: TerrainSaveData = bincode::deserialize_from(&mut reader)?;
            save_data.header
        };

        Ok(TerrainSaveInfo {
            path: path.to_path_buf(),
            name: path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            version: header.version,
            chunk_count: header.chunk_count,
            timestamp: header.timestamp,
            file_size: metadata.len(),
            description: header.description,
        })
    }

    /// Delete a save file
    pub fn delete_save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        fs::remove_file(path)?;
        Ok(())
    }

    /// Get persistence statistics
    pub fn stats(&self) -> &PersistenceStats {
        &self.stats
    }

    /// Get configuration
    pub fn config(&self) -> &TerrainPersistenceConfig {
        &self.config
    }

    /// Get dirty chunks iterator
    pub fn dirty_chunks(&self) -> impl Iterator<Item = &ChunkCoord> {
        self.dirty_chunks.iter()
    }
}

/// Information about a save file
#[derive(Debug, Clone)]
pub struct TerrainSaveInfo {
    /// Path to the save file
    pub path: PathBuf,
    /// Save name (filename without extension)
    pub name: String,
    /// Save format version
    pub version: u32,
    /// Number of chunks in save
    pub chunk_count: u32,
    /// Timestamp when save was created
    pub timestamp: u64,
    /// File size in bytes
    pub file_size: u64,
    /// Optional description
    pub description: Option<String>,
}

/// Internal save data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TerrainSaveData {
    header: TerrainSaveHeader,
    chunks: Vec<(ChunkCoord, VoxelChunk)>,
}

/// Save chunks around a player position (region-based)
pub fn get_chunks_in_region(center: Vec3, radius: f32) -> Vec<ChunkCoord> {
    let chunk_size = crate::CHUNK_SIZE as f32;
    let chunk_radius = (radius / chunk_size).ceil() as i32;
    let center_coord = ChunkCoord::from_world_pos(center);

    let mut coords = Vec::new();
    for x in -chunk_radius..=chunk_radius {
        for y in -chunk_radius..=chunk_radius {
            for z in -chunk_radius..=chunk_radius {
                coords.push(ChunkCoord::new(
                    center_coord.x + x,
                    center_coord.y + y,
                    center_coord.z + z,
                ));
            }
        }
    }
    coords
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_chunk(coord: ChunkCoord) -> VoxelChunk {
        let mut chunk = VoxelChunk::new(coord);
        // Add some test data
        chunk.set_voxel(glam::IVec3::new(0, 0, 0), crate::Voxel::new(1.0, 1));
        chunk
    }

    #[test]
    fn test_persistence_config_default() {
        let config = TerrainPersistenceConfig::default();
        assert!(config.use_compression);
        assert_eq!(config.batch_size, 32);
    }

    #[test]
    fn test_mark_dirty() {
        let mut persistence = TerrainPersistence::default_config();
        let coord = ChunkCoord::new(0, 0, 0);

        assert!(!persistence.is_dirty(&coord));
        persistence.mark_dirty(coord);
        assert!(persistence.is_dirty(&coord));
        assert_eq!(persistence.dirty_count(), 1);
    }

    #[test]
    fn test_save_header_creation() {
        let header = TerrainSaveHeader::new(12345, 10);
        assert_eq!(header.version, TERRAIN_SAVE_VERSION);
        assert_eq!(header.world_seed, 12345);
        assert_eq!(header.chunk_count, 10);
        assert_eq!(header.timestamp, 0);
    }

    #[test]
    fn test_save_and_load_chunks() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;

        let config = TerrainPersistenceConfig {
            save_directory: temp_dir.path().to_path_buf(),
            use_compression: false, // Simpler for testing
            ..Default::default()
        };

        let mut persistence = TerrainPersistence::new(config);

        // Create test chunks
        let coord1 = ChunkCoord::new(0, 0, 0);
        let coord2 = ChunkCoord::new(1, 0, 0);
        let chunk1 = create_test_chunk(coord1);
        let chunk2 = create_test_chunk(coord2);

        let mut chunks = HashMap::new();
        chunks.insert(coord1, chunk1);
        chunks.insert(coord2, chunk2);

        // Mark as dirty
        persistence.mark_dirty(coord1);
        persistence.mark_dirty(coord2);

        // Save
        let save_result = persistence.save_chunks(&chunks, 12345, Some("test_save"))?;
        assert_eq!(save_result.chunks_saved, 2);
        assert!(save_result.file_size > 0);

        // Load
        let (load_result, loaded_chunks) = persistence.load_chunks(&save_result.path)?;
        assert_eq!(load_result.chunks_loaded, 2);
        assert!(loaded_chunks.contains_key(&coord1));
        assert!(loaded_chunks.contains_key(&coord2));

        Ok(())
    }

    #[test]
    fn test_list_saves() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;

        let config = TerrainPersistenceConfig {
            save_directory: temp_dir.path().to_path_buf(),
            use_compression: false,
            ..Default::default()
        };

        let mut persistence = TerrainPersistence::new(config.clone());

        // Create and save chunks
        let coord = ChunkCoord::new(0, 0, 0);
        let chunk = create_test_chunk(coord);
        let mut chunks = HashMap::new();
        chunks.insert(coord, chunk);

        persistence.mark_dirty(coord);
        persistence.save_chunks(&chunks, 12345, Some("save1"))?;

        persistence.mark_dirty(coord);
        persistence.save_chunks(&chunks, 12345, Some("save2"))?;

        // List saves
        let saves = persistence.list_saves()?;
        assert_eq!(saves.len(), 2);

        Ok(())
    }

    #[test]
    fn test_auto_save_trigger() {
        let config = TerrainPersistenceConfig {
            auto_save_interval_seconds: 0.0, // Disabled
            ..Default::default()
        };

        let mut persistence = TerrainPersistence::new(config);
        persistence.mark_dirty(ChunkCoord::new(0, 0, 0));

        // Should not trigger when disabled
        assert!(!persistence.should_auto_save());
    }

    #[test]
    fn test_clear_dirty() {
        let mut persistence = TerrainPersistence::default_config();

        persistence.mark_dirty(ChunkCoord::new(0, 0, 0));
        persistence.mark_dirty(ChunkCoord::new(1, 0, 0));
        assert_eq!(persistence.dirty_count(), 2);

        persistence.clear_dirty();
        assert_eq!(persistence.dirty_count(), 0);
    }

    #[test]
    fn test_get_chunks_in_region() {
        let center = Vec3::new(0.0, 0.0, 0.0);
        let coords = get_chunks_in_region(center, 32.0);
        assert!(!coords.is_empty());
    }

    #[test]
    fn test_persistence_stats() {
        let persistence = TerrainPersistence::default_config();
        let stats = persistence.stats();
        assert_eq!(stats.total_chunks_saved, 0);
        assert_eq!(stats.save_count, 0);
    }
}
